//! Serial Primal Module
//! 
//! A serial implementation of the primal module. This is the very basic fusion blossom algorithm that aims at debugging and as a ground truth
//! where traditional matching is too time consuming because of their |E| = O(|V|^2) scaling.
//!

use super::util::*;
use crate::derivative::Derivative;
use super::primal_module::*;
use super::visualize::*;
use super::dual_module::*;


pub struct PrimalModuleSerial {
    /// nodes internal information
    pub nodes: Vec<Option<PrimalNodeInternalPtr>>,
    /// debug mode: only resolve one conflict each time
    pub debug_resolve_only_one: bool,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AlternatingTreeNode {
    /// the root of an alternating tree
    pub root: PrimalNodeInternalWeak,
    /// the parent in the alternating tree, note that root doesn't have a parent; together with a child of blossom that touches parent, used to create blossom and expand perfect matching
    pub parent: Option<(PrimalNodeInternalWeak, DualNodeWeak)>,
    /// the children in the alternating tree, note that odd depth can only have exactly one children; together with a child of blossom that touches each child node in the tree, used to create blossom and expand perfect matching
    pub children: Vec<(PrimalNodeInternalWeak, DualNodeWeak)>,
    /// the depth in the alternating tree, root has 0 depth
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchTarget {
    Peer(PrimalNodeInternalWeak),
    VirtualVertex(VertexIndex),
}

/// internal information of the primal node, added to the [`DualNode`]; note that primal nodes and dual nodes
/// always have one-to-one correspondence
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PrimalNodeInternal {
    /// the pointer to the origin [`DualNode`]
    pub origin: DualNodeWeak,
    /// local index, to find myself in [`DualModuleSerial::nodes`]
    pub index: NodeIndex,
    /// alternating tree information if applicable
    pub tree_node: Option<AlternatingTreeNode>,
    /// temporary match with another node, (target, touching_grandson)
    pub temporary_match: Option<(MatchTarget, DualNodeWeak)>,
}

pub type PrimalNodeInternalPtr = ArcRwLock<PrimalNodeInternal>;
pub type PrimalNodeInternalWeak = WeakRwLock<PrimalNodeInternal>;

impl std::fmt::Debug for PrimalNodeInternalPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let primal_node_internal = self.read_recursive();
        write!(f, "{}", primal_node_internal.index)
    }
}

impl std::fmt::Debug for PrimalNodeInternalWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl PrimalNodeInternal {

    /// check if in the cache, this node is a free node
    pub fn is_free(&self) -> bool {
        debug_assert!({
            let origin_ptr = self.origin.upgrade_force();
            let node = origin_ptr.read_recursive();
            node.parent_blossom.is_none()
        }, "do not call this function to a internal node, consider call PrimalModuleSerial::get_outer_node");
        if self.tree_node.is_some() { return false }  // this node belongs to an alternating tree
        if self.temporary_match.is_some() { return false }  // already temporarily matched
        true
    }

    /// modify the depth and root of a sub-tree using DFS
    pub fn change_sub_tree_root(&mut self, depth: usize, root: PrimalNodeInternalPtr) {
        let tree_node = self.tree_node.as_mut().unwrap();
        tree_node.depth = depth;
        tree_node.root = root.downgrade();
        for (child_weak, _) in tree_node.children.iter() {
            let child_ptr = child_weak.upgrade_force();
            let mut child = child_ptr.write();
            child.change_sub_tree_root(depth + 1, root.clone());
        }
    }

}

impl PrimalModuleImpl for PrimalModuleSerial {

    fn new(_initializer: &SolverInitializer) -> Self {
        Self {
            nodes: vec![],
            debug_resolve_only_one: false,
        }
    }

    fn clear(&mut self) {
        self.nodes.clear();
    }
    
    fn load(&mut self, interface: &DualModuleInterface) {
        debug_assert!(self.nodes.is_empty(), "loading to the same primal module without clear");
        for (index, node) in interface.nodes.iter().enumerate() {
            assert!(node.is_some(), "must load a fresh dual module interface, found empty node");
            let node_ptr = node.as_ref().unwrap();
            let node = node_ptr.read_recursive();
            assert!(matches!(node.class, DualNodeClass::SyndromeVertex{ .. }), "must load a fresh dual module interface, found a blossom");
            assert_eq!(node.index, index, "must load a fresh dual module interface, found index out of order");
            let primal_node_internal = PrimalNodeInternal {
                origin: node_ptr.downgrade(),
                index: index,
                tree_node: None,
                temporary_match: None,
            };
            self.nodes.push(Some(PrimalNodeInternalPtr::new(primal_node_internal)));
        }
    }

    fn resolve<D: DualModuleImpl>(&mut self, mut group_max_update_length: GroupMaxUpdateLength, interface: &mut DualModuleInterface, dual_module: &mut D) {
        debug_assert!(!group_max_update_length.is_empty() && group_max_update_length.get_none_zero_growth().is_none());
        let mut current_conflict_index = 0;
        while let Some(conflict) = group_max_update_length.pop() {
            current_conflict_index += 1;
            if self.debug_resolve_only_one && current_conflict_index > 1 {  // debug mode
                break
            }
            // println!("conflict: {conflict:?}");
            match conflict {
                MaxUpdateLength::Conflicting((node_ptr_1, touching_ptr_1), (node_ptr_2, touching_ptr_2)) => {
                    assert!(node_ptr_1 != node_ptr_2, "one cannot conflict with itself, double check to avoid deadlock");
                    if self.get_primal_node_internal_ptr_option(&node_ptr_1).is_none() { continue }  // ignore out-of-date event
                    if self.get_primal_node_internal_ptr_option(&node_ptr_2).is_none() { continue }  // ignore out-of-date event
                    // always use outer node in case it's already wrapped into a blossom
                    let primal_node_internal_ptr_1 = self.get_outer_node(self.get_primal_node_internal_ptr(&node_ptr_1));
                    let primal_node_internal_ptr_2 = self.get_outer_node(self.get_primal_node_internal_ptr(&node_ptr_2));
                    if primal_node_internal_ptr_1 == primal_node_internal_ptr_2 {
                        assert!(current_conflict_index != 1, "the first conflict cannot be ignored, otherwise may cause hidden infinite loop");
                        continue  // this is no longer a conflict because both of them belongs to a single blossom
                    }
                    let mut primal_node_internal_1 = primal_node_internal_ptr_1.write();
                    let mut primal_node_internal_2 = primal_node_internal_ptr_2.write();
                    let grow_state_1 = primal_node_internal_1.origin.upgrade_force().read_recursive().grow_state;
                    let grow_state_2 = primal_node_internal_2.origin.upgrade_force().read_recursive().grow_state;
                    if !grow_state_1.is_against(&grow_state_2) {
                        assert!(current_conflict_index != 1, "the first conflict cannot be ignored, otherwise may cause hidden infinite loop");
                        continue  // this is no longer a conflict
                    }
                    // this is the most probable case, so put it in the front
                    let (free_1, free_2) = (primal_node_internal_1.is_free(), primal_node_internal_2.is_free());
                    if free_1 && free_2 {
                        // simply match them temporarily
                        primal_node_internal_1.temporary_match = Some((MatchTarget::Peer(primal_node_internal_ptr_2.downgrade()), touching_ptr_1.downgrade()));
                        primal_node_internal_2.temporary_match = Some((MatchTarget::Peer(primal_node_internal_ptr_1.downgrade()), touching_ptr_2.downgrade()));
                        // update dual module interface
                        interface.set_grow_state(&primal_node_internal_1.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                        interface.set_grow_state(&primal_node_internal_2.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                        continue
                    }
                    // second probable case: single node touches a temporary matched pair and become an alternating tree
                    if (free_1 && primal_node_internal_2.temporary_match.is_some()) || (free_2 && primal_node_internal_1.temporary_match.is_some()) {
                        let (free_node_internal_ptr, free_touching_ptr, mut free_node_internal, matched_node_internal_ptr, matched_touching_ptr, mut matched_node_internal) = if free_1 {
                            (primal_node_internal_ptr_1.clone(), touching_ptr_1.clone(), primal_node_internal_1, primal_node_internal_ptr_2.clone(), touching_ptr_2.clone(), primal_node_internal_2)
                        } else {
                            (primal_node_internal_ptr_2.clone(), touching_ptr_2.clone(), primal_node_internal_2, primal_node_internal_ptr_1.clone(), touching_ptr_1.clone(), primal_node_internal_1)
                        };
                        // creating an alternating tree: free node becomes the root, matched node becomes child
                        let (match_target, matched_touching_grandson) = matched_node_internal.temporary_match.as_ref().unwrap().clone();
                        match &match_target {
                            MatchTarget::Peer(leaf_node_internal_weak) => {
                                let leaf_node_internal_ptr = leaf_node_internal_weak.upgrade_force();
                                let mut leaf_node_internal = leaf_node_internal_ptr.write();
                                free_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: free_node_internal_ptr.downgrade(),
                                    parent: None,
                                    children: vec![(matched_node_internal_ptr.downgrade(), free_touching_ptr.downgrade())],
                                    depth: 0,
                                });
                                matched_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: free_node_internal_ptr.downgrade(),
                                    parent: Some((free_node_internal_ptr.downgrade(), matched_touching_ptr.downgrade())),
                                    children: vec![(leaf_node_internal_weak.clone(), matched_touching_grandson)],
                                    depth: 1,
                                });
                                matched_node_internal.temporary_match = None;
                                leaf_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: free_node_internal_ptr.downgrade(),
                                    parent: Some((matched_node_internal_ptr.downgrade(), leaf_node_internal.temporary_match.as_ref().unwrap().1.clone())),
                                    children: vec![],
                                    depth: 2,
                                });
                                leaf_node_internal.temporary_match = None;
                                // update dual module interface
                                interface.set_grow_state(&free_node_internal.origin.upgrade_force(), DualNodeGrowState::Grow, dual_module);
                                interface.set_grow_state(&matched_node_internal.origin.upgrade_force(), DualNodeGrowState::Shrink, dual_module);
                                interface.set_grow_state(&leaf_node_internal.origin.upgrade_force(), DualNodeGrowState::Grow, dual_module);
                                continue
                            },
                            MatchTarget::VirtualVertex(_) => {
                                // virtual boundary doesn't have to be matched, so in this case simply match these two nodes together
                                free_node_internal.temporary_match = Some((MatchTarget::Peer(matched_node_internal_ptr.downgrade()), free_touching_ptr.downgrade()));
                                matched_node_internal.temporary_match = Some((MatchTarget::Peer(free_node_internal_ptr.downgrade()), matched_touching_ptr.downgrade()));
                                // update dual module interface
                                interface.set_grow_state(&free_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                                interface.set_grow_state(&matched_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                                continue
                            }
                        }
                    }
                    // third probable case: tree touches single vertex
                    if (free_1 && primal_node_internal_2.tree_node.is_some()) || (primal_node_internal_1.tree_node.is_some() && free_2) {
                        let (tree_node_internal_ptr, tree_touching_ptr, tree_node_internal, free_node_internal_ptr, free_touching_ptr, mut free_node_internal) = 
                            if primal_node_internal_1.tree_node.is_some() {
                                (primal_node_internal_ptr_1.clone(), touching_ptr_1.clone(), primal_node_internal_1, primal_node_internal_ptr_2.clone(), touching_ptr_2.clone(), primal_node_internal_2)
                            } else {
                                (primal_node_internal_ptr_2.clone(), touching_ptr_2.clone(), primal_node_internal_2, primal_node_internal_ptr_1.clone(), touching_ptr_1.clone(), primal_node_internal_1)
                            };
                        free_node_internal.temporary_match = Some((MatchTarget::Peer(tree_node_internal_ptr.downgrade()), free_touching_ptr.downgrade()));
                        interface.set_grow_state(&free_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                        drop(tree_node_internal);  // unlock
                        self.augment_tree_given_matched(tree_node_internal_ptr, free_node_internal_ptr, tree_touching_ptr.downgrade(), interface, dual_module);
                        continue
                    }
                    // fourth probable case: tree touches matched pair
                    if (primal_node_internal_1.tree_node.is_some() && primal_node_internal_2.temporary_match.is_some())
                            || (primal_node_internal_1.temporary_match.is_some() && primal_node_internal_2.tree_node.is_some()) {
                        let (tree_node_internal_ptr, tree_touching_ptr, mut tree_node_internal, matched_node_internal_ptr, matched_touching_ptr, mut matched_node_internal) = 
                            if primal_node_internal_1.tree_node.is_some() {
                                (primal_node_internal_ptr_1.clone(), touching_ptr_1.clone(), primal_node_internal_1, primal_node_internal_ptr_2.clone(), touching_ptr_2.clone(), primal_node_internal_2)
                            } else {
                                (primal_node_internal_ptr_2.clone(), touching_ptr_2.clone(), primal_node_internal_2, primal_node_internal_ptr_1.clone(), touching_ptr_1.clone(), primal_node_internal_1)
                            };
                        let match_target = matched_node_internal.temporary_match.as_ref().unwrap().0.clone();
                        match &match_target {
                            MatchTarget::Peer(leaf_node_internal_weak) => {
                                let leaf_node_internal_ptr = leaf_node_internal_weak.upgrade_force();
                                let tree_node = tree_node_internal.tree_node.as_mut().unwrap();
                                assert!(tree_node.depth % 2 == 0, "conflicting one must be + node");
                                // simply add this matched pair to the children
                                tree_node.children.push((matched_node_internal_ptr.downgrade(), tree_touching_ptr.downgrade()));
                                // link children to parent
                                matched_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: tree_node.root.clone(),
                                    parent: Some((tree_node_internal_ptr.downgrade(), matched_touching_ptr.downgrade())),
                                    children: vec![(leaf_node_internal_weak.clone(), matched_node_internal.temporary_match.as_ref().unwrap().1.clone())],
                                    depth: tree_node.depth + 1,
                                });
                                matched_node_internal.temporary_match = None;
                                let mut leaf_node_internal = leaf_node_internal_ptr.write();
                                leaf_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: tree_node.root.clone(),
                                    parent: Some((matched_node_internal_ptr.downgrade(), leaf_node_internal.temporary_match.as_ref().unwrap().1.clone())),
                                    children: vec![],
                                    depth: tree_node.depth + 2,
                                });
                                leaf_node_internal.temporary_match = None;
                                // update dual module interface
                                interface.set_grow_state(&matched_node_internal.origin.upgrade_force(), DualNodeGrowState::Shrink, dual_module);
                                interface.set_grow_state(&leaf_node_internal.origin.upgrade_force(), DualNodeGrowState::Grow, dual_module);
                                continue
                            },
                            MatchTarget::VirtualVertex(_) => {
                                // virtual boundary doesn't have to be matched, so in this case remove it and augment the tree
                                matched_node_internal.temporary_match = Some((MatchTarget::Peer(tree_node_internal_ptr.downgrade()), matched_touching_ptr.downgrade()));
                                drop(matched_node_internal);  // unlock
                                drop(tree_node_internal);  // unlock
                                self.augment_tree_given_matched(tree_node_internal_ptr, matched_node_internal_ptr, tree_touching_ptr.downgrade(), interface, dual_module);
                                continue
                            }
                        }
                    }
                    // much less probable case: two trees touch and both are augmented
                    if primal_node_internal_1.tree_node.is_some() && primal_node_internal_2.tree_node.is_some() {
                        let root_1 = primal_node_internal_1.tree_node.as_ref().unwrap().root.clone();
                        let root_2 = primal_node_internal_2.tree_node.as_ref().unwrap().root.clone();
                        // form a blossom inside an alternating tree
                        if root_1 == root_2 {
                            // drop writer lock to allow reader locks
                            drop(primal_node_internal_1);
                            drop(primal_node_internal_2);
                            // find LCA of two nodes, two paths are from child to parent
                            let (lca_ptr, path_1, path_2) = self.find_lowest_common_ancestor(primal_node_internal_ptr_1.clone(), primal_node_internal_ptr_2.clone());
                            let nodes_circle = {
                                let mut nodes_circle: Vec<DualNodePtr> = path_1.iter().map(|ptr| ptr.read_recursive().origin.upgrade_force()).collect();
                                nodes_circle.push(lca_ptr.read_recursive().origin.upgrade_force());
                                for i in (0..path_2.len()).rev() { nodes_circle.push(path_2[i].read_recursive().origin.upgrade_force()); }
                                nodes_circle
                            };
                            // build `touching_children`
                            let touching_children = {
                                let mut touching_children = Vec::<(DualNodeWeak, DualNodeWeak)>::new();
                                if !path_1.is_empty() {
                                    for (idx, ptr) in path_1.iter().enumerate() {
                                        let node = ptr.read_recursive();
                                        let tree_node = node.tree_node.as_ref().unwrap();
                                        let left_touching_ptr = if idx == 0 {
                                            touching_ptr_1.downgrade()
                                        } else {
                                            let last_ptr = path_1[idx-1].downgrade();
                                            let idx = tree_node.children.iter().position(|(ptr, _)| ptr == &last_ptr).expect("should find child");
                                            tree_node.children[idx].1.clone()
                                        };
                                        let right_touching_ptr = tree_node.parent.as_ref().unwrap().1.clone();
                                        touching_children.push((left_touching_ptr, right_touching_ptr));
                                    }
                                }
                                { // the lca
                                    let node = lca_ptr.read_recursive();
                                    let tree_node = node.tree_node.as_ref().unwrap();
                                    let left_touching_ptr = if path_1.is_empty() {
                                        touching_ptr_1.downgrade()
                                    } else {
                                        let left_ptr = path_1[path_1.len() - 1].downgrade();
                                        let left_idx = tree_node.children.iter().position(|(ptr, _)| ptr == &left_ptr).expect("should find child");
                                        tree_node.children[left_idx].1.clone()
                                    };
                                    let right_touching_ptr = if path_2.is_empty() {
                                        touching_ptr_2.downgrade()
                                    } else {
                                        let right_ptr = path_2[path_2.len() - 1].downgrade();
                                        let right_idx = tree_node.children.iter().position(|(ptr, _)| ptr == &right_ptr).expect("should find child");
                                        tree_node.children[right_idx].1.clone()
                                    };
                                    touching_children.push((left_touching_ptr, right_touching_ptr));
                                }
                                if !path_2.is_empty() {
                                    for (idx, ptr) in path_2.iter().enumerate().rev() {
                                        let node = ptr.read_recursive();
                                        let tree_node = node.tree_node.as_ref().unwrap();
                                        let left_touching_ptr = tree_node.parent.as_ref().unwrap().1.clone();
                                        let right_touching_ptr = if idx == 0 {
                                            touching_ptr_2.downgrade()
                                        } else {
                                            let last_ptr = path_2[idx-1].downgrade();
                                            let idx = tree_node.children.iter().position(|(ptr, _)| ptr == &last_ptr).expect("should find child");
                                            tree_node.children[idx].1.clone()
                                        };
                                        touching_children.push((left_touching_ptr, right_touching_ptr));
                                    }
                                }
                                touching_children
                            };
                            let blossom_node_ptr = interface.create_blossom(nodes_circle, touching_children, dual_module);
                            let primal_node_internal_blossom_ptr = PrimalNodeInternalPtr::new(PrimalNodeInternal {
                                origin: blossom_node_ptr.downgrade(),
                                index: self.nodes.len(),
                                tree_node: None,
                                temporary_match: None,
                            });
                            self.nodes.push(Some(primal_node_internal_blossom_ptr.clone()));
                            // handle other part of the tree structure
                            let mut children = vec![];
                            for path in [&path_1, &path_2] {
                                if path.len() > 0 {
                                    let mut last_ptr = path[0].downgrade();
                                    for (height, ptr) in path.iter().enumerate() {
                                        let mut node = ptr.write();
                                        if height == 0 {
                                            let tree_node = node.tree_node.as_ref().unwrap();
                                            for (child_ptr, child_touching_ptr) in tree_node.children.iter() {
                                                children.push((child_ptr.clone(), child_touching_ptr.clone()));
                                            }
                                        } else {
                                            if height % 2 == 0 {
                                                let tree_node = node.tree_node.as_ref().unwrap();
                                                for (child_ptr, child_touching_ptr) in tree_node.children.iter() {
                                                    if child_ptr != &last_ptr {  // not in the blossom circle
                                                        children.push((child_ptr.clone(), child_touching_ptr.clone()));
                                                    }
                                                }
                                            }
                                        }
                                        node.tree_node = None;  // this path is going to be part of the blossom, no longer in the tree
                                        last_ptr = ptr.downgrade();
                                    }
                                }
                            }
                            let mut lca = lca_ptr.write();
                            let lca_tree_node = lca.tree_node.as_ref().unwrap();
                            {  // add children of lca_ptr
                                for (child_ptr, child_touching_ptr) in lca_tree_node.children.iter() {
                                    if path_1.len() > 0 && &path_1[path_1.len() - 1].downgrade() == child_ptr { continue }
                                    if path_2.len() > 0 && &path_2[path_2.len() - 1].downgrade() == child_ptr { continue }
                                    children.push((child_ptr.clone(), child_touching_ptr.clone()));
                                }
                            }
                            if lca_tree_node.parent.is_some() || children.len() > 0 {
                                let mut primal_node_internal_blossom = primal_node_internal_blossom_ptr.write();
                                let new_tree_root = if lca_tree_node.depth == 0 { primal_node_internal_blossom_ptr.clone() } else { lca_tree_node.root.upgrade_force() };
                                let tree_node = AlternatingTreeNode {
                                    root: new_tree_root.downgrade(),
                                    parent: lca_tree_node.parent.clone(),
                                    children: children,
                                    depth: lca_tree_node.depth,
                                };
                                if lca_tree_node.parent.is_some() {
                                    let (parent_weak, _) = lca_tree_node.parent.as_ref().unwrap();
                                    let parent_ptr = parent_weak.upgrade_force();
                                    let mut parent = parent_ptr.write();
                                    let parent_tree_node = parent.tree_node.as_mut().unwrap();
                                    debug_assert!(parent_tree_node.children.len() == 1, "lca's parent should be a - node with only one child");
                                    let touching_ptr = parent_tree_node.children[0].1.clone();  // the touching grandson is not changed when forming blossom
                                    parent_tree_node.children.clear();
                                    parent_tree_node.children.push((primal_node_internal_blossom_ptr.downgrade(), touching_ptr));
                                }
                                if tree_node.children.len() > 0 {
                                    // connect this blossom to the new alternating tree
                                    for (child_weak, _) in tree_node.children.iter() {
                                        let child_ptr = child_weak.upgrade_force();
                                        let mut child = child_ptr.write();
                                        let child_tree_node = child.tree_node.as_mut().unwrap();
                                        debug_assert!(child_tree_node.parent.is_some(), "child should have a parent");
                                        let touching_ptr = child_tree_node.parent.as_ref().unwrap().1.clone();  // the touching grandson is not changed when forming blossom
                                        child_tree_node.parent = Some((primal_node_internal_blossom_ptr.downgrade(), touching_ptr));
                                    }
                                    primal_node_internal_blossom.tree_node = Some(tree_node);
                                    primal_node_internal_blossom.change_sub_tree_root(lca_tree_node.depth, new_tree_root);
                                } else {
                                    primal_node_internal_blossom.tree_node = Some(tree_node);
                                }
                            }
                            lca.tree_node = None;
                            continue
                        } else {
                            drop(primal_node_internal_1);  // unlock
                            drop(primal_node_internal_2);  // unlock
                            self.augment_tree_given_matched(primal_node_internal_ptr_1.clone(), primal_node_internal_ptr_2.clone(), touching_ptr_1.downgrade(), interface, dual_module);
                            self.augment_tree_given_matched(primal_node_internal_ptr_2.clone(), primal_node_internal_ptr_1.clone(), touching_ptr_2.downgrade(), interface, dual_module);
                            continue
                        }
                    }
                    unreachable!()
                },
                MaxUpdateLength::TouchingVirtual((node_ptr, touching_ptr), virtual_vertex_index) => {
                    if self.get_primal_node_internal_ptr_option(&node_ptr).is_none() { continue }  // ignore out-of-date event
                    let primal_node_internal_ptr = self.get_outer_node(self.get_primal_node_internal_ptr(&node_ptr));
                    let mut primal_node_internal = primal_node_internal_ptr.write();
                    let grow_state = primal_node_internal.origin.upgrade_force().read_recursive().grow_state;
                    if grow_state != DualNodeGrowState::Grow {
                        assert!(current_conflict_index != 1, "the first conflict cannot be ignored, otherwise may cause hidden infinite loop");
                        continue  // this is no longer a conflict
                    }
                    // this is the most probable case, so put it in the front
                    if primal_node_internal.is_free() {
                        primal_node_internal.temporary_match = Some((MatchTarget::VirtualVertex(virtual_vertex_index), touching_ptr.downgrade()));
                        interface.set_grow_state(&primal_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                        continue
                    }
                    // tree touching virtual boundary will just augment the whole tree
                    if primal_node_internal.tree_node.is_some() {
                        drop(primal_node_internal);
                        self.augment_tree_given_virtual_vertex(primal_node_internal_ptr, virtual_vertex_index, touching_ptr.downgrade(), interface, dual_module);
                        continue
                    }
                    unreachable!()
                },
                MaxUpdateLength::BlossomNeedExpand(node_ptr) => {
                    if self.get_primal_node_internal_ptr_option(&node_ptr).is_none() { continue }  // ignore out-of-date event
                    // blossom breaking is assumed to be very rare given our multiple-tree approach, so don't need to optimize for it
                    // first, isolate this blossom from its alternating tree
                    let primal_node_internal_ptr = self.get_primal_node_internal_ptr(&node_ptr);
                    let outer_primal_node_internal_ptr = self.get_outer_node(primal_node_internal_ptr.clone());
                    if outer_primal_node_internal_ptr != primal_node_internal_ptr {
                        // this blossom is now wrapped into another blossom, so we don't need to expand it anymore
                        assert!(current_conflict_index != 1, "the first conflict cannot be ignored, otherwise may cause hidden infinite loop");
                        continue
                    }
                    let primal_node_internal = primal_node_internal_ptr.read_recursive();
                    let grow_state = primal_node_internal.origin.upgrade_force().read_recursive().grow_state;
                    if grow_state != DualNodeGrowState::Shrink {
                        assert!(current_conflict_index != 1, "the first conflict cannot be ignored, otherwise may cause hidden infinite loop");
                        continue  // this is no longer a conflict
                    }
                    // copy the nodes circle
                    let (nodes_circle, touching_children) = {
                        let blossom = node_ptr.read_recursive();
                        match &blossom.class {
                            DualNodeClass::Blossom{ nodes_circle, touching_children } => (nodes_circle.clone(), touching_children.clone()),
                            _ => unreachable!("the expanding node is not a blossom")
                        }
                    };
                    // remove it from nodes
                    assert_eq!(self.nodes[primal_node_internal.index], Some(primal_node_internal_ptr.clone()), "index wrong");
                    self.nodes[primal_node_internal.index] = None;
                    assert!(primal_node_internal.tree_node.is_some(), "expanding blossom must belong to an alternating tree");
                    let tree_node = primal_node_internal.tree_node.as_ref().unwrap();
                    assert!(tree_node.depth % 2 == 1, "expanding blossom must a '-' node in an alternating tree");
                    let (parent_ptr, parent_touching_ptr, parent_touching_child_ptr) = {  // remove it from it's parent's tree
                        let (parent_weak, parent_touching_child_ptr) = &tree_node.parent.as_ref().unwrap();
                        let parent_ptr = parent_weak.upgrade_force();
                        let mut parent = parent_ptr.write();
                        let parent_tree_node = parent.tree_node.as_mut().unwrap();
                        let idx = parent_tree_node.children.iter().position(|ptr| ptr.0 == primal_node_internal_ptr.downgrade()).expect("should find");
                        let parent_touching_ptr = parent_tree_node.children[idx].1.clone();
                        parent_tree_node.children.remove(idx);
                        parent_tree_node.children.retain(|ptr| ptr.0 != primal_node_internal_ptr.downgrade());
                        (parent_ptr.clone(), parent_touching_ptr, parent_touching_child_ptr.upgrade_force().get_secondary_ancestor_blossom().downgrade())
                    };
                    let (child_ptr, child_touching_ptr, child_touching_child_ptr) = {  // make children independent trees
                        debug_assert!(tree_node.children.len() == 1, "a - node must have exactly ONE child");
                        let child_weak = &tree_node.children[0].0;
                        let child_touching_child_ptr = tree_node.children[0].1.upgrade_force().get_secondary_ancestor_blossom().downgrade();
                        let child_ptr = child_weak.upgrade_force();
                        let child = child_ptr.read_recursive();
                        let child_tree_node = child.tree_node.as_ref().unwrap();
                        // find which blossom-child is touching this child
                        (child_ptr.clone(), child_tree_node.parent.as_ref().unwrap().1.clone(), child_touching_child_ptr)
                    };
                    interface.expand_blossom(node_ptr, dual_module);
                    // now we need to re-connect all the expanded nodes, by analyzing the relationship of nodes_circle, parent_touching_ptr and child_touching_ptr
                    let parent_touching_index = nodes_circle.iter().position(|ptr| ptr == &parent_touching_child_ptr).expect("touching node should be in the blossom circle");
                    let child_touching_index = nodes_circle.iter().position(|ptr| ptr == &child_touching_child_ptr).expect("touching node should be in the blossom circle");
                    let mut is_tree_sequence_ascending = true;
                    let (match_sequence, tree_sequence) = {  // tree sequence is from parent to child
                        let mut match_sequence = Vec::new();
                        let mut tree_sequence = Vec::new();
                        if parent_touching_index == child_touching_index {
                            tree_sequence.push(parent_touching_index);
                            for i in parent_touching_index+1 .. nodes_circle.len() { match_sequence.push(i); }
                            for i in 0 .. parent_touching_index { match_sequence.push(i); }
                        } else if parent_touching_index > child_touching_index {
                            if (parent_touching_index - child_touching_index) % 2 == 0 {  // [... c <----- p ...]
                                for i in (child_touching_index .. parent_touching_index+1).rev() { tree_sequence.push(i); }
                                is_tree_sequence_ascending = false;
                                for i in parent_touching_index+1 .. nodes_circle.len() { match_sequence.push(i); }
                                for i in 0 .. child_touching_index { match_sequence.push(i); }
                            } else {  // [--> c ...... p ---]
                                for i in parent_touching_index .. nodes_circle.len() { tree_sequence.push(i); }
                                for i in 0 .. child_touching_index+1 { tree_sequence.push(i); }
                                for i in child_touching_index+1 .. parent_touching_index { match_sequence.push(i); }
                            }
                        } else {  // parent_touching_index < child_touching_index
                            if (child_touching_index - parent_touching_index) % 2 == 0 {  // [... p -----> c ...]
                                for i in parent_touching_index .. child_touching_index+1 { tree_sequence.push(i); }
                                for i in child_touching_index+1 .. nodes_circle.len() { match_sequence.push(i); }
                                for i in 0 .. parent_touching_index { match_sequence.push(i); }
                            } else {  // [--- p ...... c <--]
                                for i in (0 .. parent_touching_index+1).rev() { tree_sequence.push(i); }
                                for i in (child_touching_index .. nodes_circle.len()).rev() { tree_sequence.push(i); }
                                is_tree_sequence_ascending = false;
                                for i in parent_touching_index+1 .. child_touching_index { match_sequence.push(i); }
                            }
                        }
                        (match_sequence, tree_sequence)
                    };
                    debug_assert!(match_sequence.len() % 2 == 0 && tree_sequence.len() % 2 == 1, "parity of sequence wrong");
                    // match the nodes in the match sequence
                    for i in (0..match_sequence.len()).step_by(2) {
                        let primal_node_internal_ptr_1 = self.get_primal_node_internal_ptr(&nodes_circle[match_sequence[i]].upgrade_force());
                        let primal_node_internal_ptr_2 = self.get_primal_node_internal_ptr(&nodes_circle[match_sequence[i+1]].upgrade_force());
                        debug_assert!((match_sequence[i] + 1) % nodes_circle.len() == match_sequence[i+1], "match sequence should be ascending");
                        let touching_ptr_1 = touching_children[match_sequence[i]].1.clone();  // assuming ascending match sequence
                        let touching_ptr_2 = touching_children[match_sequence[i+1]].0.clone();  // assuming ascending match sequence
                        let mut primal_node_internal_1 = primal_node_internal_ptr_1.write();
                        let mut primal_node_internal_2 = primal_node_internal_ptr_2.write();
                        primal_node_internal_1.temporary_match = Some((MatchTarget::Peer(primal_node_internal_ptr_2.downgrade()), touching_ptr_1));
                        primal_node_internal_2.temporary_match = Some((MatchTarget::Peer(primal_node_internal_ptr_1.downgrade()), touching_ptr_2));
                        interface.set_grow_state(&primal_node_internal_1.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                        interface.set_grow_state(&primal_node_internal_2.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                    }
                    // connect the nodes in the tree sequence to the alternating tree, note that the tree sequence is from parent to child
                    for (idx, current_i) in tree_sequence.iter().enumerate() {
                        debug_assert!({
                            if idx + 1 < tree_sequence.len() {
                                if is_tree_sequence_ascending { (tree_sequence[idx] + 1) % nodes_circle.len() == tree_sequence[idx+1] }
                                else { (tree_sequence[idx+1] + 1) % nodes_circle.len() == tree_sequence[idx] }
                            } else { true }
                        }, "tree sequence orientation must be consistent");
                        let current_parent_ptr = if idx == 0 { parent_ptr.clone() } else { self.get_primal_node_internal_ptr(&nodes_circle[tree_sequence[idx - 1]].upgrade_force()) };
                        let current_parent_touching_ptr = if idx == 0 {
                            tree_node.parent.as_ref().unwrap().1.clone()
                        } else {
                            if is_tree_sequence_ascending { touching_children[*current_i].0.clone() } else { touching_children[*current_i].1.clone() }
                        };
                        let current_child_ptr = if idx == tree_sequence.len() - 1 { child_ptr.clone() }
                            else { self.get_primal_node_internal_ptr(&nodes_circle[tree_sequence[idx + 1]].upgrade_force()) };
                        let current_child_touching_ptr = if idx == tree_sequence.len() - 1 {
                            tree_node.children[0].1.clone()
                        } else {
                            if is_tree_sequence_ascending { touching_children[*current_i].1.clone() } else { touching_children[*current_i].0.clone() }
                        };
                        let current_ptr = self.get_primal_node_internal_ptr(&nodes_circle[*current_i].upgrade_force());
                        let mut current = current_ptr.write();
                        current.tree_node = Some(AlternatingTreeNode {
                            root: tree_node.root.clone(),
                            parent: Some((current_parent_ptr.downgrade(), current_parent_touching_ptr)),
                            children: vec![(current_child_ptr.downgrade(), current_child_touching_ptr)],
                            depth: tree_node.depth + idx,
                        });
                        interface.set_grow_state(&current.origin.upgrade_force(), if idx % 2 == 0 { DualNodeGrowState::Shrink } else { DualNodeGrowState::Grow }, dual_module);
                    }
                    {  // connect parent
                        let mut parent = parent_ptr.write();
                        let parent_tree_node = parent.tree_node.as_mut().unwrap();
                        let child_ptr = self.get_primal_node_internal_ptr(&nodes_circle[tree_sequence[0]].upgrade_force());
                        parent_tree_node.children.push((child_ptr.downgrade(), parent_touching_ptr));
                    }
                    {  // connect child and fix the depth information of the child
                        let mut child = child_ptr.write();
                        let child_tree_node = child.tree_node.as_mut().unwrap();
                        let parent_ptr = self.get_primal_node_internal_ptr(&nodes_circle[tree_sequence[tree_sequence.len()-1]].upgrade_force());
                        child_tree_node.parent = Some((parent_ptr.downgrade(), child_touching_ptr));
                        child.change_sub_tree_root(tree_node.depth + tree_sequence.len(), tree_node.root.upgrade_force());
                    }
                },
                MaxUpdateLength::VertexShrinkStop(_node_ptr) => {
                    if current_conflict_index == 1 {
                        // if this happens, then debug the sorting of conflict events and also check alternating tree: a vertex should never be a floating "-" node
                        unreachable!("VertexShrinkStop conflict cannot be solved by primal module, and should be sorted to the last of the heap")
                    }
                    // just skip and wait for the next round to resolve it, if it's not being resolved already
                }
                _ => unreachable!("should not resolve these issues")
            }
        }
    }

    fn intermediate_matching<D: DualModuleImpl>(&mut self, _interface: &mut DualModuleInterface, _dual_module: &mut D) -> IntermediateMatching {
        let mut immediate_matching = IntermediateMatching::new();
        for i in 0..self.nodes.len() {
            let primal_node_internal_ptr = self.nodes[i].clone();
            match primal_node_internal_ptr {
                Some(primal_node_internal_ptr) => {
                    let primal_node_internal = primal_node_internal_ptr.read_recursive();
                    assert!(primal_node_internal.tree_node.is_none(), "cannot compute perfect matching with active alternating tree");
                    let origin_ptr = primal_node_internal.origin.upgrade_force();
                    let interface_node = origin_ptr.read_recursive();
                    if interface_node.parent_blossom.is_some() {
                        assert_eq!(primal_node_internal.temporary_match, None, "blossom internal nodes should not be matched");
                        continue  // do not handle this blossom at this level
                    }
                    if let Some((match_target, match_touching_ptr)) = primal_node_internal.temporary_match.as_ref() {
                        match match_target {
                            MatchTarget::Peer(peer_internal_weak) => {
                                let peer_internal_ptr = peer_internal_weak.upgrade_force();
                                let peer_internal = peer_internal_ptr.read_recursive();
                                if primal_node_internal.index < peer_internal.index {  // to avoid duplicate matched pairs
                                    let peer_touching_ptr = peer_internal.temporary_match.as_ref().unwrap().1.clone();
                                    immediate_matching.peer_matchings.push((
                                        (primal_node_internal.origin.upgrade_force(), match_touching_ptr.clone()), 
                                        (peer_internal.origin.upgrade_force(), peer_touching_ptr)
                                    ));
                                }
                            },
                            MatchTarget::VirtualVertex(virtual_vertex) => {
                                immediate_matching.virtual_matchings.push((
                                    (primal_node_internal.origin.upgrade_force(), match_touching_ptr.clone())
                                    , *virtual_vertex
                                ));
                            },
                        }
                    } else {
                        panic!("cannot compute final matching with unmatched outer node {:?}", primal_node_internal_ptr);
                    }
                }, None => { }
            }
        }
        immediate_matching
    }

}

impl FusionVisualizer for PrimalModuleSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // do the sanity check first before taking snapshot
        self.sanity_check().unwrap();
        let mut primal_nodes = Vec::<serde_json::Value>::new();
        for primal_node_ptr in self.nodes.iter() {
            if let Some(primal_node_ptr) = &primal_node_ptr {
                let primal_node = primal_node_ptr.read_recursive();
                primal_nodes.push(json!({
                    if abbrev { "m" } else { "temporary_match" }: primal_node.temporary_match.as_ref().map(|(match_target, touching_ptr)| {
                        match match_target {
                            MatchTarget::Peer(peer_weak) => {
                                let peer_ptr = peer_weak.upgrade_force();
                                json!({
                                    if abbrev { "p" } else { "peer" }: peer_ptr.read_recursive().index,
                                    if abbrev { "t" } else { "touching" }: touching_ptr.upgrade_force().read_recursive().index,
                                })
                            },
                            MatchTarget::VirtualVertex(vertex_idx) => json!({
                                if abbrev { "v" } else { "virtual_vertex" }: vertex_idx,
                                if abbrev { "t" } else { "touching" }: touching_ptr.upgrade_force().read_recursive().index,
                            }),
                        }
                    }),
                    if abbrev { "t" } else { "tree_node" }: primal_node.tree_node.as_ref().map(|tree_node| {
                        json!({
                            if abbrev { "r" } else { "root" }: tree_node.root.upgrade_force().read_recursive().index,
                            if abbrev { "p" } else { "parent" }: tree_node.parent.as_ref().map(|(weak, _)| weak.upgrade_force().read_recursive().index),
                            if abbrev { "pt" } else { "parent_touching" }: tree_node.parent.as_ref().map(|(_, weak)| weak.upgrade_force().read_recursive().index),
                            if abbrev { "c" } else { "children" }: tree_node.children.iter().map(|(weak, _)| weak.upgrade_force().read_recursive().index).collect::<Vec<NodeIndex>>(),
                            if abbrev { "ct" } else { "children_touching" }: tree_node.children.iter().map(|(_, weak)| weak.upgrade_force().read_recursive().index).collect::<Vec<NodeIndex>>(),
                            if abbrev { "d" } else { "depth" }: tree_node.depth,
                        })
                    }),
                }));
            } else {
                primal_nodes.push(json!(null));
            }
        }
        json!({
            "primal_nodes": primal_nodes,
        })
    }
}

impl PrimalModuleSerial {

    pub fn get_primal_node_internal_ptr_option(&self, dual_node_ptr: &DualNodePtr) -> Option<PrimalNodeInternalPtr> {
        let dual_node = dual_node_ptr.read_recursive();
        self.nodes[dual_node.index].as_ref().map(|primal_node_internal_ptr| {
            debug_assert!(dual_node_ptr == &primal_node_internal_ptr.read_recursive().origin.upgrade_force()
                , "dual node and primal internal node must corresponds to each other");
            primal_node_internal_ptr.clone()
        })
    }

    pub fn get_primal_node_internal_ptr(&self, dual_node_ptr: &DualNodePtr) -> PrimalNodeInternalPtr {
        self.get_primal_node_internal_ptr_option(dual_node_ptr).expect("internal primal node must exists")
    }

    /// get the outer node in the most up-to-date cache
    pub fn get_outer_node(&self, primal_node_internal_ptr: PrimalNodeInternalPtr) -> PrimalNodeInternalPtr {
        let node = primal_node_internal_ptr.read_recursive();
        let origin_ptr = node.origin.upgrade_force();
        let interface_node = origin_ptr.read_recursive();
        if let Some(parent_dual_node_weak) = &interface_node.parent_blossom {
            let parent_dual_node_ptr = parent_dual_node_weak.upgrade_force();
            let parent_primal_node_internal_ptr = self.get_primal_node_internal_ptr(&parent_dual_node_ptr);
            self.get_outer_node(parent_primal_node_internal_ptr)
        } else {
            primal_node_internal_ptr.clone()
        }
    }

    /// find the lowest common ancestor (LCA) of two nodes in the alternating tree, return (LCA, path_1, path_2) where path includes leaf but exclude the LCA
    pub fn find_lowest_common_ancestor(&self, mut primal_node_internal_ptr_1: PrimalNodeInternalPtr, mut primal_node_internal_ptr_2: PrimalNodeInternalPtr)
            -> (PrimalNodeInternalPtr, Vec<PrimalNodeInternalPtr>, Vec<PrimalNodeInternalPtr>) {
        let (depth_1, depth_2) = {
            let primal_node_internal_1 = primal_node_internal_ptr_1.read_recursive();
            let primal_node_internal_2 = primal_node_internal_ptr_2.read_recursive();
            let tree_node_1 = primal_node_internal_1.tree_node.as_ref().unwrap();
            let tree_node_2 = primal_node_internal_2.tree_node.as_ref().unwrap();
            assert_eq!(tree_node_1.root, tree_node_2.root, "must belong to the same tree");
            (tree_node_1.depth, tree_node_2.depth)
        };
        let mut path_1 = vec![];
        let mut path_2 = vec![];
        if depth_1 > depth_2 {
            loop {
                let ptr = primal_node_internal_ptr_1.clone();
                let primal_node_internal = ptr.read_recursive();
                let tree_node = primal_node_internal.tree_node.as_ref().unwrap();
                if tree_node.depth == depth_2 { break }
                path_1.push(primal_node_internal_ptr_1.clone());
                primal_node_internal_ptr_1 = tree_node.parent.as_ref().unwrap().0.upgrade_force();
            }
        } else if depth_2 > depth_1 {
            loop {
                let ptr = primal_node_internal_ptr_2.clone();
                let primal_node_internal = ptr.read_recursive();
                let tree_node = primal_node_internal.tree_node.as_ref().unwrap();
                if tree_node.depth == depth_1 { break }
                path_2.push(primal_node_internal_ptr_2.clone());
                primal_node_internal_ptr_2 = tree_node.parent.as_ref().unwrap().0.upgrade_force();
            }
        }
        // now primal_node_internal_ptr_1 and primal_node_internal_ptr_2 has the same depth, compare them until they're equal
        loop {
            if primal_node_internal_ptr_1 == primal_node_internal_ptr_2 {
                return (primal_node_internal_ptr_1, path_1, path_2)
            }
            let ptr_1 = primal_node_internal_ptr_1.clone();
            let ptr_2 = primal_node_internal_ptr_2.clone();
            let primal_node_internal_1 = ptr_1.read_recursive();
            let primal_node_internal_2 = ptr_2.read_recursive();
            let tree_node_1 = primal_node_internal_1.tree_node.as_ref().unwrap();
            let tree_node_2 = primal_node_internal_2.tree_node.as_ref().unwrap();
            path_1.push(primal_node_internal_ptr_1.clone());
            path_2.push(primal_node_internal_ptr_2.clone());
            primal_node_internal_ptr_1 = tree_node_1.parent.as_ref().unwrap().0.upgrade_force();
            primal_node_internal_ptr_2 = tree_node_2.parent.as_ref().unwrap().0.upgrade_force();
        }
    }

    /// for any - node, match the children by matching them with + node
    pub fn match_subtree<D: DualModuleImpl>(&self, tree_node_internal_ptr: PrimalNodeInternalPtr, interface: &mut DualModuleInterface, dual_module: &mut D) {
        let mut tree_node_internal = tree_node_internal_ptr.write();
        let tree_node = tree_node_internal.tree_node.as_ref().unwrap();
        debug_assert!(tree_node.depth % 2 == 1, "only match - node is possible");
        let child_node_internal_ptr = tree_node.children[0].0.upgrade_force();
        tree_node_internal.temporary_match = Some((MatchTarget::Peer(child_node_internal_ptr.downgrade()), tree_node.children[0].1.clone()));
        interface.set_grow_state(&tree_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
        tree_node_internal.tree_node = None;
        let mut child_node_internal = child_node_internal_ptr.write();
        let child_touching_ptr = child_node_internal.tree_node.as_ref().unwrap().parent.as_ref().unwrap().1.clone();
        child_node_internal.temporary_match = Some((MatchTarget::Peer(tree_node_internal_ptr.downgrade()), child_touching_ptr));
        let child_tree_node = child_node_internal.tree_node.as_ref().unwrap();
        interface.set_grow_state(&child_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
        for (grandson_ptr, _) in child_tree_node.children.iter() {
            self.match_subtree(grandson_ptr.upgrade_force(), interface, dual_module);
        }
        child_node_internal.tree_node = None;
    }

    /// for any + node, match it with another node will augment the whole tree, breaking out into several matched pairs;
    /// `tree_grandson_ptr` is the grandson of tree_node_internal_ptr that touches `match_node_internal_ptr`
    pub fn augment_tree_given_matched<D: DualModuleImpl>(&self, tree_node_internal_ptr: PrimalNodeInternalPtr, match_node_internal_ptr: PrimalNodeInternalPtr
            , tree_touching_ptr: DualNodeWeak, interface: &mut DualModuleInterface, dual_module: &mut D) {
        let mut tree_node_internal = tree_node_internal_ptr.write();
        tree_node_internal.temporary_match = Some((MatchTarget::Peer(match_node_internal_ptr.downgrade()), tree_touching_ptr));
        interface.set_grow_state(&tree_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
        let tree_node = tree_node_internal.tree_node.as_ref().unwrap();
        debug_assert!(tree_node.depth % 2 == 0, "only augment + node is possible");
        for (child_ptr, _) in tree_node.children.iter() {
            if child_ptr != &match_node_internal_ptr.downgrade() {
                self.match_subtree(child_ptr.upgrade_force(), interface, dual_module);
            }
        }
        if tree_node.depth != 0 {  // it's not root, then we need to match parent to grandparent
            let parent_node_internal_weak = tree_node.parent.as_ref().unwrap().0.clone();
            let parent_node_internal_ptr = parent_node_internal_weak.upgrade_force();
            let grandparent_node_internal_ptr = {  // must unlock parent
                let mut parent_node_internal = parent_node_internal_ptr.write();
                let parent_tree_node = parent_node_internal.tree_node.as_ref().unwrap();
                let grandparent_node_internal_weak = parent_tree_node.parent.as_ref().unwrap().0.clone();
                parent_node_internal.temporary_match = Some((MatchTarget::Peer(grandparent_node_internal_weak.clone()), parent_tree_node.parent.as_ref().unwrap().1.clone()));
                parent_node_internal.tree_node = None;
                interface.set_grow_state(&parent_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                grandparent_node_internal_weak.upgrade_force()
            };
            let grandparent_touching_ptr = {
                let grandparent_node_internal = grandparent_node_internal_ptr.read_recursive();
                let grandparent_tree_node = grandparent_node_internal.tree_node.as_ref().unwrap();
                let idx = grandparent_tree_node.children.iter().position(|(ptr, _)| ptr == &parent_node_internal_weak).expect("should find child");
                grandparent_tree_node.children[idx].1.clone()
            };
            self.augment_tree_given_matched(grandparent_node_internal_ptr, parent_node_internal_ptr.clone(), grandparent_touching_ptr, interface, dual_module);
        }
        tree_node_internal.tree_node = None;
    }

    /// for any + node, match it with virtual boundary will augment the whole tree, breaking out into several matched pairs
    pub fn augment_tree_given_virtual_vertex<D: DualModuleImpl>(&self, tree_node_internal_ptr: PrimalNodeInternalPtr, virtual_vertex_index: VertexIndex
            , tree_touching_ptr: DualNodeWeak, interface: &mut DualModuleInterface, dual_module: &mut D) {
        let mut tree_node_internal = tree_node_internal_ptr.write();
        tree_node_internal.temporary_match = Some((MatchTarget::VirtualVertex(virtual_vertex_index), tree_touching_ptr));
        interface.set_grow_state(&tree_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
        let tree_node = tree_node_internal.tree_node.as_ref().unwrap();
        debug_assert!(tree_node.depth % 2 == 0, "only augment + node is possible");
        for (child_ptr, _) in tree_node.children.iter() {
            self.match_subtree(child_ptr.upgrade_force(), interface, dual_module);
        }
        if tree_node.depth != 0 {  // it's not root, then we need to match parent to grandparent
            let parent_node_internal_weak = tree_node.parent.as_ref().unwrap().0.clone();
            let parent_node_internal_ptr = parent_node_internal_weak.upgrade_force();
            let grandparent_node_internal_ptr = {  // must unlock parent
                let mut parent_node_internal = parent_node_internal_ptr.write();
                let parent_tree_node = parent_node_internal.tree_node.as_ref().unwrap();
                let grandparent_node_internal_weak = parent_tree_node.parent.as_ref().unwrap().0.clone();
                parent_node_internal.temporary_match = Some((MatchTarget::Peer(grandparent_node_internal_weak.clone()), parent_tree_node.parent.as_ref().unwrap().1.clone()));
                parent_node_internal.tree_node = None;
                interface.set_grow_state(&parent_node_internal.origin.upgrade_force(), DualNodeGrowState::Stay, dual_module);
                grandparent_node_internal_weak.upgrade_force()
            };
            let grandparent_touching_ptr = {
                let grandparent_node_internal = grandparent_node_internal_ptr.read_recursive();
                let grandparent_tree_node = grandparent_node_internal.tree_node.as_ref().unwrap();
                let idx = grandparent_tree_node.children.iter().position(|(ptr, _)| ptr == &parent_node_internal_weak).expect("should find child");
                grandparent_tree_node.children[idx].1.clone()
            };
            self.augment_tree_given_matched(grandparent_node_internal_ptr, parent_node_internal_ptr.clone(), grandparent_touching_ptr, interface, dual_module);
        }
        tree_node_internal.tree_node = None;
    }

    /// do a sanity check of it's tree structure and internal state
    pub fn sanity_check(&self) -> Result<(), String> {
        for (index, primal_module_internal_ptr) in self.nodes.iter().enumerate() {
            match primal_module_internal_ptr {
                Some(primal_module_internal_ptr) => {
                    let primal_module_internal = primal_module_internal_ptr.read_recursive();
                    if primal_module_internal.index != index { return Err(format!("primal node index wrong: expected {}, actual {}", index, primal_module_internal.index)) }
                    let origin_ptr = primal_module_internal.origin.upgrade_force();
                    let origin_node = origin_ptr.read_recursive();
                    if origin_node.index != primal_module_internal.index { return Err(format!("origin index wrong: expected {}, actual {}", index, origin_node.index)) }
                    if primal_module_internal.temporary_match.is_some() && primal_module_internal.tree_node.is_some() {
                        return Err(format!("{} temporary match and tree node cannot both exists", index))
                    }
                    if origin_node.parent_blossom.is_some() {
                        if primal_module_internal.tree_node.is_some() { return Err(format!("blossom internal node {index} is still in a tree")) }
                        if primal_module_internal.temporary_match.is_some() { return Err(format!("blossom internal node {index} is still matched")) }
                    }
                    if let Some((match_target, _)) = primal_module_internal.temporary_match.as_ref() {
                        if origin_node.grow_state != DualNodeGrowState::Stay { return Err(format!("matched node {index} is not set to Stay")) }
                        match match_target {
                            MatchTarget::Peer(peer_weak) => {
                                let peer_ptr = peer_weak.upgrade_force();
                                let peer = peer_ptr.read_recursive();
                                if let Some((peer_match_target, _)) = peer.temporary_match.as_ref() {
                                    if peer_match_target != &MatchTarget::Peer(primal_module_internal_ptr.downgrade()) {
                                        return Err(format!("match peer {} is not matched with {}, instead it's {:?}", peer.index, index, peer_match_target))
                                    }
                                } else {
                                    return Err(format!("match peer is not marked as matched"))
                                }
                            },
                            MatchTarget::VirtualVertex(_vertex_idx) => { },  // nothing to check
                        }
                    }
                    if let Some(tree_node) = primal_module_internal.tree_node.as_ref() {
                        // first check if every child's parent is myself
                        for (child_weak, _) in tree_node.children.iter() {
                            let child_ptr = child_weak.upgrade_force();
                            let child = child_ptr.read_recursive();
                            if let Some(child_tree_node) = child.tree_node.as_ref() {
                                if child_tree_node.parent.as_ref().map(|x| &x.0) != Some(&primal_module_internal_ptr.downgrade()) {
                                    return Err(format!("{}'s child {} has a different parent, link broken", index, child.index))
                                }
                            } else { return Err(format!("{}'s child {} doesn't belong to any tree, link broken", index, child.index)) }
                            // check if child is still tracked, i.e. inside self.nodes
                            if child.index >= self.nodes.len() || self.nodes[child.index].is_none() {
                                return Err(format!("child's index {} is not in the interface", child.index))
                            }
                            let tracked_child_ptr = self.nodes[child.index].as_ref().unwrap();
                            if tracked_child_ptr != &child_ptr {
                                return Err(format!("the tracked ptr of child {} is not what's being pointed", child.index))
                            }
                        }
                        // then check if I'm my parent's child
                        if let Some((parent_weak, _)) = tree_node.parent.as_ref() {
                            let parent_ptr = parent_weak.upgrade_force();
                            let parent = parent_ptr.read_recursive();
                            if let Some(parent_tree_node) = parent.tree_node.as_ref() {
                                let mut found_match_count = 0;
                                for (node_ptr, _) in parent_tree_node.children.iter() {
                                    if node_ptr == &primal_module_internal_ptr.downgrade() {
                                        found_match_count += 1;
                                    }
                                }
                                if found_match_count != 1 {
                                    return Err(format!("{} is the parent of {} but the child only presents {} times", parent.index, index, found_match_count))
                                }
                            } else { return Err(format!("{}'s parent {} doesn't belong to any tree, link broken", index, parent.index)) }
                            // check if parent is still tracked, i.e. inside self.nodes
                            if parent.index >= self.nodes.len() || self.nodes[parent.index].is_none() {
                                return Err(format!("parent's index {} is not in the interface", parent.index))
                            }
                            let tracked_parent_ptr = self.nodes[parent.index].as_ref().unwrap();
                            if tracked_parent_ptr != &parent_ptr {
                                return Err(format!("the tracked ptr of child {} is not what's being pointed", parent.index))
                            }
                        } else {
                            if tree_node.root != primal_module_internal_ptr.downgrade() {
                                return Err(format!("{} is not the root of the tree, yet it has no parent", index))
                            }
                        }
                        // then check if the root and the depth is correct
                        let mut current_ptr = primal_module_internal_ptr.clone();
                        let mut current_up = 0;
                        loop {
                            let current = current_ptr.read_recursive();
                            // check if current is still tracked, i.e. inside self.nodes
                            if current.index >= self.nodes.len() || self.nodes[current.index].is_none() {
                                return Err(format!("current's index {} is not in the interface", current.index))
                            }
                            let tracked_current_ptr = self.nodes[current.index].as_ref().unwrap();
                            if tracked_current_ptr != &current_ptr {
                                return Err(format!("the tracked ptr of current {} is not what's being pointed", current.index))
                            }
                            // go to parent
                            if let Some(current_tree_node) = current.tree_node.as_ref() {
                                if let Some((current_parent_ptr, _)) = current_tree_node.parent.as_ref() {
                                    let current_parent_ptr = current_parent_ptr.clone();
                                    drop(current);
                                    current_ptr = current_parent_ptr.upgrade_force();
                                    current_up += 1;
                                } else {
                                    // confirm this is root and then break the loop
                                    if &current_tree_node.root != &current_ptr.downgrade() {
                                        return Err(format!("current {} is not the root of the tree, yet it has no parent", current.index))
                                    }
                                    break
                                }
                            } else { return Err(format!("climbing up from {} to {} but it doesn't belong to a tree anymore", index, current.index)) }
                        }
                        if current_up != tree_node.depth {
                            return Err(format!("{} is marked with depth {} but the real depth is {}", index, tree_node.depth, current_up))
                        }
                        if current_ptr.downgrade() != tree_node.root {
                            return Err(format!("{} is marked with root {:?} but the real root is {:?}", index, tree_node.root, current_ptr))
                        }
                    }
                }, _ => { }
            }
        }
        Ok(())
    }

}


#[cfg(test)]
pub mod tests {
    use super::*;
    use super::super::example::*;
    use super::super::dual_module_serial::*;
    use super::super::*;

    pub fn primal_module_serial_basic_standard_syndrome_optional_viz(d: usize, visualize_filename: Option<String>, syndrome_vertices: Vec<VertexIndex>, final_dual: Weight)
            -> (DualModuleInterface, PrimalModuleSerial, DualModuleSerial) {
        println!("{syndrome_vertices:?}");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(d, 0.1, half_weight);
        let mut visualizer = match visualize_filename.as_ref() {
            Some(visualize_filename) => {
                let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
                visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
                print_visualize_link(&visualize_filename);
                Some(visualizer)
            }, None => None
        };
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, visualizer.as_mut());
        assert_eq!(interface.sum_dual_variables, final_dual * 2 * half_weight, "unexpected final dual variable sum");
        (interface, primal_module, dual_module)
    }

    pub fn primal_module_serial_basic_standard_syndrome(d: usize, visualize_filename: String, syndrome_vertices: Vec<VertexIndex>, final_dual: Weight)
            -> (DualModuleInterface, PrimalModuleSerial, DualModuleSerial) {
        primal_module_serial_basic_standard_syndrome_optional_viz(d, Some(visualize_filename), syndrome_vertices, final_dual)
    }

    /// test a simple blossom
    #[test]
    fn primal_module_serial_basic_1() {  // cargo test primal_module_serial_basic_1 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_1.json");
        let syndrome_vertices = vec![18, 26, 34];
        primal_module_serial_basic_standard_syndrome(7, visualize_filename, syndrome_vertices, 4);
    }

    /// test a free node conflict with a virtual boundary
    #[test]
    fn primal_module_serial_basic_2() {  // cargo test primal_module_serial_basic_2 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_2.json");
        let syndrome_vertices = vec![16];
        primal_module_serial_basic_standard_syndrome(7, visualize_filename, syndrome_vertices, 1);
    }

    /// test a free node conflict with a matched node (with virtual boundary)
    #[test]
    fn primal_module_serial_basic_3() {  // cargo test primal_module_serial_basic_3 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_3.json");
        let syndrome_vertices = vec![16, 26];
        primal_module_serial_basic_standard_syndrome(7, visualize_filename, syndrome_vertices, 3);
    }

    /// test blossom shrinking and expanding
    #[test]
    fn primal_module_serial_basic_4() {  // cargo test primal_module_serial_basic_4 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_4.json");
        let syndrome_vertices = vec![16, 52, 65, 76, 112];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 10);
    }

    /// test blossom conflicts with vertex
    #[test]
    fn primal_module_serial_basic_5() {  // cargo test primal_module_serial_basic_5 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_5.json");
        let syndrome_vertices = vec![39, 51, 61, 62, 63, 64, 65, 75, 87, 67];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 6);
    }

    /// test cascaded blossom
    #[test]
    fn primal_module_serial_basic_6() {  // cargo test primal_module_serial_basic_6 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_6.json");
        let syndrome_vertices = vec![39, 51, 61, 62, 63, 64, 65, 75, 87];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 6);
    }

    /// test two alternating trees conflict with each other
    #[test]
    fn primal_module_serial_basic_7() {  // cargo test primal_module_serial_basic_7 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_7.json");
        let syndrome_vertices = vec![37, 61, 63, 66, 68, 44];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 7);
    }

    /// test an alternating tree touches a virtual boundary
    #[test]
    fn primal_module_serial_basic_8() {  // cargo test primal_module_serial_basic_8 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_8.json");
        let syndrome_vertices = vec![61, 64, 67];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 5);
    }

    /// test a matched node (with virtual boundary) conflicts with an alternating tree
    #[test]
    fn primal_module_serial_basic_9() {  // cargo test primal_module_serial_basic_9 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_9.json");
        let syndrome_vertices = vec![60, 63, 66, 30];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 6);
    }

    /// test the error pattern in the paper
    #[test]
    fn primal_module_serial_basic_10() {  // cargo test primal_module_serial_basic_10 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_10.json");
        let syndrome_vertices = vec![39, 52, 63, 90, 100];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 9);
    }

    /// debug a case of deadlock after changing the strategy of detecting conflicts around VertexShrinkStop;
    /// reason: forget to check whether conflicting nodes are growing: only growing one should be reported
    #[test]
    fn primal_module_serial_basic_11() {  // cargo test primal_module_serial_basic_11 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_11.json");
        let syndrome_vertices = vec![13, 29, 52, 53, 58, 60, 71, 74, 76, 87, 96, 107, 112, 118, 121, 122, 134, 137, 141, 145, 152, 153, 154, 156, 157, 169, 186, 202, 203, 204, 230, 231];
        primal_module_serial_basic_standard_syndrome(15, visualize_filename, syndrome_vertices, 20);
    }

    /// debug a case where it disagree with blossom V library, mine reports 11866, blossom V reports 12284
    #[test]
    fn primal_module_debug_1() {  // cargo test primal_module_debug_1 -- --nocapture
        let visualize_filename = format!("primal_module_debug_1.json");
        let syndrome_vertices = vec![34, 35, 84, 89, 92, 100, 141, 145, 149, 164, 193, 201, 205, 220, 235, 242, 243, 260, 261, 290, 300, 308, 309, 315, 317];
        let max_half_weight = 500;
        let mut code = CircuitLevelPlanarCode::new(7, 7, 0.01, max_half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        let initializer = code.get_initializer();
        // blossom V ground truth
        let blossom_mwpm_result = blossom_v_mwpm(&initializer, &syndrome_vertices);
        println!("blossom_mwpm_result: {blossom_mwpm_result:?}");
        let blossom_details = detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            println!("    {detail:?}");
            blossom_total_weight += detail.weight;
        }
        // create dual module
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
        let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            println!("    {detail:?}");
            fusion_total_weight += detail.weight;
        }
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        {  // also test subgraph builder
            let mut subgraph_builder = SubGraphBuilder::new(&initializer);
            subgraph_builder.load_perfect_matching(&fusion_mwpm);
            assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        }
        assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
    }

    /// debug a case where it disagree with blossom V library, mine reports 33000, blossom V reports 34000
    #[test]
    fn primal_module_debug_2() {  // cargo test primal_module_debug_2 -- --nocapture
        let visualize_filename = format!("primal_module_debug_2.json");
        let syndrome_vertices = vec![7, 8, 10, 22, 23, 24, 25, 37, 38, 39, 40, 42, 43, 69, 57, 59, 60, 72, 76, 93, 109, 121, 123, 125, 135, 136, 137, 138, 139, 140, 141, 150, 151, 153, 154, 155, 166, 171, 172, 181, 183, 184, 188, 200, 204, 219, 233];
        let max_half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(15, 0.3, max_half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        let initializer = code.get_initializer();
        // blossom V ground truth
        let blossom_mwpm_result = blossom_v_mwpm(&initializer, &syndrome_vertices);
        let blossom_details = detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            println!("    {detail:?}");
            blossom_total_weight += detail.weight;
        }
        // create dual module
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
        let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            println!("    {detail:?}");
            fusion_total_weight += detail.weight;
        }
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        {  // also test subgraph builder
            let mut subgraph_builder = SubGraphBuilder::new(&initializer);
            subgraph_builder.load_perfect_matching(&fusion_mwpm);
            assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        }
        assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
    }

    /// debug a case where it disagree with blossom V library, mine reports 16000, blossom V reports 17000
    #[test]
    fn primal_module_debug_3() {  // cargo test primal_module_debug_3 -- --nocapture
        let visualize_filename = format!("primal_module_debug_3.json");
        let syndrome_vertices = vec![17, 34, 36, 54, 55, 74, 95, 96, 112, 113, 114, 115, 116, 130, 131, 132, 134, 150, 151, 154, 156, 171, 172, 173, 190];
        let max_half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(19, 0.499, max_half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        let initializer = code.get_initializer();
        // blossom V ground truth
        let blossom_mwpm_result = blossom_v_mwpm(&initializer, &syndrome_vertices);
        let blossom_details = detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            println!("    {detail:?}");
            blossom_total_weight += detail.weight;
        }
        // create dual module
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
        let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            println!("    {detail:?}");
            fusion_total_weight += detail.weight;
        }
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        {  // also test subgraph builder
            let mut subgraph_builder = SubGraphBuilder::new(&initializer);
            subgraph_builder.load_perfect_matching(&fusion_mwpm);
            assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        }
        assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
    }

    /// debug a case where it disagree with blossom V library, mine reports 9000, blossom V reports 7000
    #[test]
    fn primal_module_debug_4() {  // cargo test primal_module_debug_4 -- --nocapture
        let visualize_filename = format!("primal_module_debug_4.json");
        let syndrome_vertices = vec![1, 3, 6, 8, 9, 11, 13];
        let max_half_weight = 500;
        let mut code = CodeCapacityRepetitionCode::new(15, 0.499, max_half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        let initializer = code.get_initializer();
        // blossom V ground truth
        let blossom_mwpm_result = blossom_v_mwpm(&initializer, &syndrome_vertices);
        let blossom_details = detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            println!("    {detail:?}");
            blossom_total_weight += detail.weight;
        }
        // create dual module
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
        let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            println!("    {detail:?}");
            fusion_total_weight += detail.weight;
        }
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        {  // also test subgraph builder
            let mut subgraph_builder = SubGraphBuilder::new(&initializer);
            subgraph_builder.load_perfect_matching(&fusion_mwpm);
            assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        }
        assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
    }

    /// debug a case of being stuck after disable the flag `debug_resolve_only_one` for faster speed
    #[test]
    fn primal_module_debug_5() {  // cargo test primal_module_debug_5 -- --nocapture
        let visualize_filename = format!("primal_module_debug_5.json");
        let syndrome_vertices = vec![0, 1, 3, 8, 9];
        let max_half_weight = 500;
        let mut code = CodeCapacityRepetitionCode::new(11, 0.03, max_half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        let initializer = code.get_initializer();
        // blossom V ground truth
        let blossom_mwpm_result = blossom_v_mwpm(&initializer, &syndrome_vertices);
        let blossom_details = detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            println!("    {detail:?}");
            blossom_total_weight += detail.weight;
        }
        // create dual module
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
        let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            println!("    {detail:?}");
            fusion_total_weight += detail.weight;
        }
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        {  // also test subgraph builder
            let mut subgraph_builder = SubGraphBuilder::new(&initializer);
            subgraph_builder.load_perfect_matching(&fusion_mwpm);
            assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        }
        assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
    }

    #[test]
    fn primal_module_serial_perfect_matching_1() {  // cargo test primal_module_serial_perfect_matching_1 -- --nocapture
        let syndrome_vertices = vec![39, 51, 61, 62, 63, 64, 65, 75, 87, 67];
        let (mut interface, mut primal_module, mut dual_module) = primal_module_serial_basic_standard_syndrome_optional_viz(11, None, syndrome_vertices, 6);
        let intermediate_matching = primal_module.intermediate_matching(&mut interface, &mut dual_module);
        println!("intermediate_matching: {intermediate_matching:?}");
        let perfect_matching = primal_module.perfect_matching(&mut interface, &mut dual_module);
        println!("perfect_matching: {perfect_matching:?}");
    }

    /// debug a case of non-zero weight given pure erasure
    #[test]
    fn primal_module_debug_6() {  // cargo test primal_module_debug_6 -- --nocapture
        let visualize_filename = format!("primal_module_debug_6.json");
        let syndrome_vertices = vec![13, 34, 87, 107, 276, 296];
        let erasures = vec![13, 33, 174, 516];
        let max_half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(19, 0., max_half_weight);
        code.set_erasure_probability(0.003);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        let mut initializer = code.get_initializer();
        for edge_index in erasures.iter() {
            let (vertex_idx_1, vertex_idx_2, _) = &initializer.weighted_edges[*edge_index];
            initializer.weighted_edges[*edge_index] = (*vertex_idx_1, *vertex_idx_2, 0);
        }
        // blossom V ground truth
        let blossom_mwpm_result = blossom_v_mwpm(&initializer, &syndrome_vertices);
        let blossom_details = detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            println!("    {detail:?}");
            blossom_total_weight += detail.weight;
        }
        // create dual module
        let mut dual_module = DualModuleSerial::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
        let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            println!("    {detail:?}");
            fusion_total_weight += detail.weight;
        }
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        {  // also test subgraph builder
            let mut subgraph_builder = SubGraphBuilder::new(&initializer);
            subgraph_builder.load_perfect_matching(&fusion_mwpm);
            assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        }
        assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
    }

}
