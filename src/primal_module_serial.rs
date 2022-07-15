//! Serial Primal Module
//! 
//! A serial implementation of the primal module. This is the very basic fusion blossom algorithm that aims at debugging and as a ground truth
//! where traditional matching is too time consuming because of their |E| = O(|V|^2) scaling.
//!

use super::util::*;
use crate::derivative::Derivative;
use std::sync::Arc;
use crate::parking_lot::RwLock;
use super::primal_module::*;
use super::visualize::*;
use super::dual_module::*;


pub struct PrimalModuleSerial {
    /// nodes internal information
    pub nodes: Vec<Option<PrimalNodeInternalPtr>>,
    /// debug mode: only resolve one conflict each time
    pub debug_resolve_only_one: bool,
}

pub struct PrimalNodeInternalPtr { ptr: Arc<RwLock<PrimalNodeInternal>>, }

impl Clone for PrimalNodeInternalPtr {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl RwLockPtr<PrimalNodeInternal> for PrimalNodeInternalPtr {
    fn new_ptr(ptr: Arc<RwLock<PrimalNodeInternal>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: PrimalNodeInternal) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<PrimalNodeInternal>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<PrimalNodeInternal>> { &mut self.ptr }
}

impl PartialEq for PrimalNodeInternalPtr {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl std::fmt::Debug for PrimalNodeInternalPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let primal_node_internal = self.read_recursive();
        write!(f, "{}", primal_node_internal.index)
    }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct AlternatingTreeNode {
    /// the root of an alternating tree
    pub root: PrimalNodeInternalPtr,
    /// the parent in the alternating tree, note that root doesn't have a parent
    pub parent: Option<PrimalNodeInternalPtr>,
    /// the children in the alternating tree, note that odd depth can only have exactly one children
    pub children: Vec<PrimalNodeInternalPtr>,
    /// the depth in the alternating tree, root has 0 depth
    pub depth: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MatchTarget {
    Peer(PrimalNodeInternalPtr),
    VirtualVertex(VertexIndex),
}

/// internal information of the primal node, added to the [`DualNode`]; note that primal nodes and dual nodes
/// always have one-to-one correspondence
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PrimalNodeInternal {
    /// the pointer to the origin [`DualNode`]
    pub origin: DualNodePtr,
    /// local index, to find myself in [`DualModuleSerial::nodes`]
    index: NodeIndex,
    /// alternating tree information if applicable
    pub tree_node: Option<AlternatingTreeNode>,
    /// temporary match with another node
    pub temporary_match: Option<MatchTarget>,
}

impl PrimalNodeInternal {

    /// check if in the cache, this node is a free node
    pub fn is_free(&self) -> bool {
        debug_assert!({
            let node = self.origin.read_recursive();
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
        tree_node.root = root.clone();
        for child_ptr in tree_node.children.iter() {
            let mut child = child_ptr.write();
            child.change_sub_tree_root(depth + 1, root.clone());
        }
    }

}

impl PrimalNodeInternalPtr {

    /// make this the new root of an independet alternating tree
    pub fn make_independent_tree(&self) {
        let mut node = self.write();
        let tree_node = node.tree_node.as_mut().unwrap();
        assert!(tree_node.depth % 2 == 0, "only '+' node can make independent tree");
        node.change_sub_tree_root(0, self.clone());
    }

}

impl PrimalModuleImpl for PrimalModuleSerial {

    fn new(_vertex_num: usize, _weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, _virtual_vertices: &Vec<VertexIndex>) -> Self {
        Self {
            nodes: vec![],
            debug_resolve_only_one: false,
        }
    }

    fn clear(&mut self) {
        self.nodes.clear();
    }
    
    fn load(&mut self, interface: &DualModuleInterface) {
        self.clear();
        for (index, node) in interface.nodes.iter().enumerate() {
            assert!(node.is_some(), "must load a fresh dual module interface, found empty node");
            let node_ptr = node.as_ref().unwrap();
            let node = node_ptr.read_recursive();
            assert!(matches!(node.class, DualNodeClass::SyndromeVertex{ .. }), "must load a fresh dual module interface, found a blossom");
            assert_eq!(node.index, index, "must load a fresh dual module interface, found index out of order");
            let primal_node_internal = PrimalNodeInternal {
                origin: node_ptr.clone(),
                index: index,
                tree_node: None,
                temporary_match: None,
            };
            self.nodes.push(Some(PrimalNodeInternalPtr::new(primal_node_internal)));
        }
    }

    fn resolve<D: DualModuleImpl>(&mut self, mut group_max_update_length: GroupMaxUpdateLength, interface: &mut DualModuleInterface, dual_module: &mut D) {
        debug_assert!(!group_max_update_length.is_empty() && group_max_update_length.get_none_zero_growth().is_none());
        let conflicts = group_max_update_length.get_conflicts();
        let mut current_conflict_index = 0;
        while let Some(conflict) = conflicts.pop() {
            current_conflict_index += 1;
            if self.debug_resolve_only_one && current_conflict_index > 1 {  // debug mode
                break
            }
            match conflict {
                MaxUpdateLength::Conflicting(node_ptr_1, node_ptr_2) => {
                    assert!(node_ptr_1 != node_ptr_2, "one cannot conflict with itself, double check to avoid deadlock");
                    // always use outer node in case it's already wrapped into a blossom
                    let primal_node_internal_ptr_1 = self.get_outer_node(self.get_primal_node_internal_ptr(&node_ptr_1));
                    let primal_node_internal_ptr_2 = self.get_outer_node(self.get_primal_node_internal_ptr(&node_ptr_2));
                    let mut primal_node_internal_1 = primal_node_internal_ptr_1.write();
                    let mut primal_node_internal_2 = primal_node_internal_ptr_2.write();
                    // this is the most probable case, so put it in the front
                    let (free_1, free_2) = (primal_node_internal_1.is_free(), primal_node_internal_2.is_free());
                    if free_1 && free_2 {
                        // simply match them temporarily
                        primal_node_internal_1.temporary_match = Some(MatchTarget::Peer(primal_node_internal_ptr_2.clone()));
                        primal_node_internal_2.temporary_match = Some(MatchTarget::Peer(primal_node_internal_ptr_1.clone()));
                        // update dual module interface
                        interface.set_grow_state(&primal_node_internal_1.origin, DualNodeGrowState::Stay, dual_module);
                        interface.set_grow_state(&primal_node_internal_2.origin, DualNodeGrowState::Stay, dual_module);
                        continue
                    }
                    // second probable case: single node touches a temporary matched pair and become an alternating tree
                    if (free_1 && primal_node_internal_2.temporary_match.is_some()) || (free_2 && primal_node_internal_1.temporary_match.is_some()) {
                        let (free_node_internal_ptr, mut free_node_internal, matched_node_internal_ptr, mut matched_node_internal) = if free_1 {
                            (primal_node_internal_ptr_1.clone(), primal_node_internal_1, primal_node_internal_ptr_2.clone(), primal_node_internal_2)
                        } else {
                            (primal_node_internal_ptr_2.clone(), primal_node_internal_2, primal_node_internal_ptr_1.clone(), primal_node_internal_1)
                        };
                        // creating an alternating tree: free node becomes the root, matched node becomes child
                        let match_target: MatchTarget = matched_node_internal.temporary_match.as_ref().unwrap().clone();
                        match &match_target {
                            MatchTarget::Peer(leaf_node_internal_ptr) => {
                                let mut leaf_node_internal = leaf_node_internal_ptr.write();
                                free_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: free_node_internal_ptr.clone(),
                                    parent: None,
                                    children: vec![matched_node_internal_ptr.clone()],
                                    depth: 0,
                                });
                                matched_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: free_node_internal_ptr.clone(),
                                    parent: Some(free_node_internal_ptr.clone()),
                                    children: vec![leaf_node_internal_ptr.clone()],
                                    depth: 1,
                                });
                                matched_node_internal.temporary_match = None;
                                leaf_node_internal.tree_node = Some(AlternatingTreeNode {
                                    root: free_node_internal_ptr.clone(),
                                    parent: Some(matched_node_internal_ptr.clone()),
                                    children: vec![],
                                    depth: 2,
                                });
                                leaf_node_internal.temporary_match = None;
                                // update dual module interface
                                interface.set_grow_state(&free_node_internal.origin, DualNodeGrowState::Grow, dual_module);
                                interface.set_grow_state(&matched_node_internal.origin, DualNodeGrowState::Shrink, dual_module);
                                interface.set_grow_state(&leaf_node_internal.origin, DualNodeGrowState::Grow, dual_module);
                                continue
                            },
                            MatchTarget::VirtualVertex(_) => {
                                // virtual boundary doesn't have to be matched, so in this case simply match these two nodes together
                                free_node_internal.temporary_match = Some(MatchTarget::Peer(matched_node_internal_ptr.clone()));
                                matched_node_internal.temporary_match = Some(MatchTarget::Peer(free_node_internal_ptr.clone()));
                                // update dual module interface
                                interface.set_grow_state(&free_node_internal.origin, DualNodeGrowState::Stay, dual_module);
                                interface.set_grow_state(&matched_node_internal.origin, DualNodeGrowState::Stay, dual_module);
                                continue
                            }
                        }
                    }
                    if primal_node_internal_1.tree_node.is_some() && primal_node_internal_2.tree_node.is_some() {
                        let root_1 = primal_node_internal_1.tree_node.as_ref().unwrap().root.clone();
                        let root_2 = primal_node_internal_2.tree_node.as_ref().unwrap().root.clone();
                        // form a blossom inside an alternating tree
                        if root_1 == root_2 {
                            // drop writer lock to allow reader locks
                            drop(primal_node_internal_1);
                            drop(primal_node_internal_2);
                            // find LCA of two nodes
                            let (lca_ptr, path_1, path_2) = self.find_lowest_common_ancestor(primal_node_internal_ptr_1.clone(), primal_node_internal_ptr_2.clone());
                            let nodes_circle = {
                                let mut nodes_circle: Vec<DualNodePtr> = path_1.iter().map(|ptr| ptr.read_recursive().origin.clone()).collect();
                                nodes_circle.push(lca_ptr.read_recursive().origin.clone());
                                for i in (0..path_2.len()).rev() { nodes_circle.push(path_2[i].read_recursive().origin.clone()); }
                                nodes_circle
                            };
                            let blossom_node_ptr = interface.create_blossom(nodes_circle, dual_module);
                            let primal_node_internal_blossom_ptr = PrimalNodeInternalPtr::new(PrimalNodeInternal {
                                origin: blossom_node_ptr.clone(),
                                index: self.nodes.len(),
                                tree_node: None,
                                temporary_match: None,
                            });
                            self.nodes.push(Some(primal_node_internal_blossom_ptr.clone()));
                            // handle other part of the tree structure
                            let mut children = vec![];
                            for path in [&path_1, &path_2] {
                                if path.len() > 0 {
                                    let mut last_ptr = path[0].clone();
                                    for (height, ptr) in path.iter().enumerate() {
                                        let mut node = ptr.write();
                                        if height > 1 {
                                            if height % 2 == 0 {
                                                let tree_node = node.tree_node.as_mut().unwrap();
                                                for child_ptr in tree_node.children.iter() {
                                                    if child_ptr != &last_ptr {  // not in the blossom circle
                                                        children.push(child_ptr.clone());
                                                    }
                                                }
                                            }
                                        }
                                        node.tree_node = None;  // this path is going to be part of the blossom, no longer in the tree
                                        last_ptr = ptr.clone();
                                    }
                                }
                            }
                            let mut lca = lca_ptr.write();
                            let lca_tree_node = lca.tree_node.as_ref().unwrap();
                            if lca_tree_node.parent.is_some() && children.len() > 0 {
                                // connect this blossom to the new alternating tree
                                for child_ptr in children.iter() {
                                    let mut child = child_ptr.write();
                                    let child_tree_node = child.tree_node.as_mut().unwrap();
                                    assert!(child_tree_node.parent.is_some(), "child should be a '-' node");
                                    child_tree_node.parent = Some(primal_node_internal_blossom_ptr.clone());
                                }
                                let mut primal_node_internal_blossom = primal_node_internal_blossom_ptr.write();
                                let new_tree_root = if lca_tree_node.depth == 0 { primal_node_internal_blossom_ptr.clone() } else { lca_tree_node.root.clone() };
                                primal_node_internal_blossom.tree_node = Some(AlternatingTreeNode {
                                    root: new_tree_root.clone(),
                                    parent: if lca_tree_node.depth == 0 { None } else { lca_tree_node.parent.clone() },
                                    children: children,
                                    depth: lca_tree_node.depth,
                                });
                                primal_node_internal_blossom.change_sub_tree_root(lca_tree_node.depth, new_tree_root);
                            }
                            lca.tree_node = None;
                            continue
                        }
                    }
                    unimplemented!()
                },
                MaxUpdateLength::TouchingVirtual(node_ptr, virtual_vertex_index) => {
                    let primal_node_internal_ptr = self.get_outer_node(self.get_primal_node_internal_ptr(&node_ptr));
                    let mut primal_node_internal = primal_node_internal_ptr.write();
                    // this is the most probable case, so put it in the front
                    if primal_node_internal.is_free() {
                        primal_node_internal.temporary_match = Some(MatchTarget::VirtualVertex(virtual_vertex_index));
                        interface.set_grow_state(&primal_node_internal.origin, DualNodeGrowState::Stay, dual_module);
                        continue
                    }
                    unimplemented!()
                },
                MaxUpdateLength::BlossomNeedExpand(node_ptr) => {
                    // do we need to break the while loop here because expanding a blossom will lead to ambiguous conflicts?
                    // blossom breaking is assumed to be very rare given our multiple-tree approach, so don't need to optimize for it
                    // for now, I assume it won't cause problem
                    // first, isolate this blossom from it's alternating tree
                    let primal_node_internal_ptr = self.get_primal_node_internal_ptr(&node_ptr);
                    let outer_primal_node_internal_ptr = self.get_outer_node(primal_node_internal_ptr.clone());
                    if outer_primal_node_internal_ptr != primal_node_internal_ptr {
                        // this blossom is now wrapped into another blossom, so we don't need to expand it anymore
                        continue
                    }
                    let primal_node_internal = primal_node_internal_ptr.read_recursive();
                    // remove it from nodes
                    assert_eq!(self.nodes[primal_node_internal.index], Some(primal_node_internal_ptr.clone()), "index wrong");
                    self.nodes[primal_node_internal.index] = None;
                    assert!(primal_node_internal.tree_node.is_some(), "expanding blossom must belong to an alternating tree");
                    let tree_node = primal_node_internal.tree_node.as_ref().unwrap();
                    assert!(tree_node.depth % 2 == 1, "expanding blossom must a '-' node in an alternating tree");
                    {  // remove it from it's parent's tree
                        let parent_ptr = tree_node.parent.as_ref().unwrap();
                        let mut parent = parent_ptr.write();
                        let parent_tree_node = parent.tree_node.as_mut().unwrap();
                        parent_tree_node.children.retain(|ptr| ptr != &primal_node_internal_ptr);
                        if parent_tree_node.children.len() == 0 && parent_tree_node.depth == 0 {
                            parent.tree_node = None;  // no longer an alternating tree
                        }
                    }
                    {  // make children independent trees
                        for child_ptr in tree_node.children.iter() {
                            child_ptr.make_independent_tree();
                            let mut child = child_ptr.write();
                            let tree_node = child.tree_node.as_ref().unwrap();
                            if tree_node.children.len() == 0 && tree_node.depth == 0 {
                                child.tree_node = None;
                            }
                        }
                    }
                    interface.expand_blossom(node_ptr, dual_module);
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
                    if abbrev { "m" } else { "temporary_match" }: primal_node.temporary_match.as_ref().map(|match_target| {
                        match match_target {
                            MatchTarget::Peer(peer_ptr) => json!({ if abbrev { "p" } else { "peer" }: peer_ptr.read_recursive().index }),
                            MatchTarget::VirtualVertex(vertex_idx) => json!({ if abbrev { "v" } else { "virtual_vertex" }: vertex_idx }),
                        }
                    }),
                    if abbrev { "t" } else { "tree_node" }: primal_node.tree_node.as_ref().map(|tree_node| {
                        json!({
                            if abbrev { "r" } else { "root" }: tree_node.root.read_recursive().index,
                            if abbrev { "p" } else { "parent" }: tree_node.parent.as_ref().map(|ptr| ptr.read_recursive().index),
                            if abbrev { "c" } else { "children" }: tree_node.children.iter().map(|ptr| ptr.read_recursive().index).collect::<Vec<NodeIndex>>(),
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

    pub fn get_primal_node_internal_ptr(&self, dual_node_ptr: &DualNodePtr) -> PrimalNodeInternalPtr {
        let dual_node = dual_node_ptr.read_recursive();
        let primal_node_internal_ptr = self.nodes[dual_node.index].as_ref().expect("internal primal node must exists");
        debug_assert!(dual_node_ptr == &primal_node_internal_ptr.read_recursive().origin, "dual node and primal internal node must corresponds to each other");
        primal_node_internal_ptr.clone()
    }

    /// get the outer node in the most up-to-date cache
    pub fn get_outer_node(&self, primal_node_internal_ptr: PrimalNodeInternalPtr) -> PrimalNodeInternalPtr {
        let node = primal_node_internal_ptr.read_recursive();
        let interface_node = node.origin.read_recursive();
        if let Some(parent_dual_node_ptr) = &interface_node.parent_blossom {
            let parent_primal_node_internal_ptr = self.get_primal_node_internal_ptr(parent_dual_node_ptr);
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
                primal_node_internal_ptr_1 = tree_node.parent.as_ref().unwrap().clone();
            }
        } else if depth_2 > depth_1 {
            loop {
                let ptr = primal_node_internal_ptr_2.clone();
                let primal_node_internal = ptr.read_recursive();
                let tree_node = primal_node_internal.tree_node.as_ref().unwrap();
                if tree_node.depth == depth_1 { break }
                path_2.push(primal_node_internal_ptr_2.clone());
                primal_node_internal_ptr_2 = tree_node.parent.as_ref().unwrap().clone();
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
            primal_node_internal_ptr_1 = tree_node_1.parent.as_ref().unwrap().clone();
            primal_node_internal_ptr_2 = tree_node_2.parent.as_ref().unwrap().clone();
        }
    }

    /// do a sanity check of it's tree structure and internal state
    pub fn sanity_check(&self) -> Result<(), String> {
        for (index, primal_module_internal_ptr) in self.nodes.iter().enumerate() {
            match primal_module_internal_ptr {
                Some(primal_module_internal_ptr) => {
                    let primal_module_internal = primal_module_internal_ptr.read_recursive();
                    if primal_module_internal.index != index { return Err(format!("primal node index wrong: expected {}, actual {}", index, primal_module_internal.index)) }
                    let origin_node = primal_module_internal.origin.read_recursive();
                    if origin_node.index != primal_module_internal.index { return Err(format!("origin index wrong: expected {}, actual {}", index, origin_node.index)) }
                    if primal_module_internal.temporary_match.is_some() && primal_module_internal.tree_node.is_some() {
                        return Err(format!("{} temporary match and tree node cannot both exists", index))
                    }
                    if let Some(match_target) = primal_module_internal.temporary_match.as_ref() {
                        match match_target {
                            MatchTarget::Peer(peer_ptr) => {
                                let peer = peer_ptr.read_recursive();
                                if let Some(peer_match_target) = peer.temporary_match.as_ref() {
                                    if peer_match_target != &MatchTarget::Peer(primal_module_internal_ptr.clone()) {
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
                        for child_ptr in tree_node.children.iter() {
                            let child = child_ptr.read_recursive();
                            if let Some(child_tree_node) = child.tree_node.as_ref() {
                                if child_tree_node.parent.as_ref() != Some(&primal_module_internal_ptr) {
                                    return Err(format!("{}'s child {} has a different parent, link broken", index, child.index))
                                }
                            } else { return Err(format!("{}'s child {} doesn't belong to any tree, link broken", index, child.index)) }
                            // check if child is still tracked, i.e. inside self.nodes
                            if child.index >= self.nodes.len() || self.nodes[child.index].is_none() {
                                return Err(format!("child's index {} is not in the interface", child.index))
                            }
                            let tracked_child_ptr = self.nodes[child.index].as_ref().unwrap();
                            if tracked_child_ptr != child_ptr {
                                return Err(format!("the tracked ptr of child {} is not what's being pointed", child.index))
                            }
                        }
                        // then check if I'm my parent's child
                        if let Some(parent_ptr) = tree_node.parent.as_ref() {
                            let parent = parent_ptr.read_recursive();
                            if let Some(parent_tree_node) = parent.tree_node.as_ref() {
                                let mut found_match_count = 0;
                                for node_ptr in parent_tree_node.children.iter() {
                                    if node_ptr == primal_module_internal_ptr {
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
                            if tracked_parent_ptr != parent_ptr {
                                return Err(format!("the tracked ptr of child {} is not what's being pointed", parent.index))
                            }
                        } else {
                            if &tree_node.root != primal_module_internal_ptr {
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
                                if let Some(current_parent_ptr) = current_tree_node.parent.as_ref() {
                                    let current_parent_ptr = current_parent_ptr.clone();
                                    drop(current);
                                    current_ptr = current_parent_ptr;
                                    current_up += 1;
                                } else {
                                    // confirm this is root and then break the loop
                                    if &current_tree_node.root != &current_ptr {
                                        return Err(format!("current {} is not the root of the tree, yet it has no parent", current.index))
                                    }
                                    break
                                }
                            } else { return Err(format!("climbing up from {} to {} but it doesn't belong to a tree anymore", index, current.index)) }
                        }
                        if current_up != tree_node.depth {
                            return Err(format!("{} is marked with depth {} but the real depth is {}", index, tree_node.depth, current_up))
                        }
                        if current_ptr != tree_node.root {
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
mod tests {
    use super::*;
    use super::super::example::*;
    use super::super::dual_module_serial::*;

    #[test]
    fn primal_module_serial_basic_1() {  // cargo test primal_module_serial_basic_1 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_1.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.vertices[18].is_syndrome = true;
        code.vertices[26].is_syndrome = true;
        code.vertices[34].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        interface.debug_print_actions = true;
        primal_module.load(&interface);  // load syndrome and connect to the dual module interface
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module, &primal_module]).unwrap();
        interface.grow(half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("alternating tree grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 6 * half_weight);
        visualizer.snapshot_combined(format!("end"), vec![&interface, &dual_module, &primal_module]).unwrap();
    }

    #[test]
    fn primal_module_serial_basic_2() {  // cargo test primal_module_serial_basic_2 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_2.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.vertices[16].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        interface.debug_print_actions = true;
        primal_module.load(&interface);  // load syndrome and connect to the dual module interface
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module, &primal_module]).unwrap();
        interface.grow(2 * half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        assert!(group_max_update_length.is_empty(), "no more things to solve");
        visualizer.snapshot_combined(format!("end"), vec![&interface, &dual_module, &primal_module]).unwrap();
    }

    #[test]
    fn primal_module_serial_basic_3() {  // cargo test primal_module_serial_basic_3 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_3.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.vertices[16].is_syndrome = true;
        code.vertices[26].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        interface.debug_print_actions = true;
        primal_module.load(&interface);  // load syndrome and connect to the dual module interface
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module, &primal_module]).unwrap();
        interface.grow(2 * half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.is_empty(), "no more things to solve");
        visualizer.snapshot_combined(format!("end"), vec![&interface, &dual_module, &primal_module]).unwrap();
    }

    #[test]
    fn primal_module_serial_basic_4() {  // cargo test primal_module_serial_basic_4 -- --nocapture
        let visualize_filename = format!("primal_module_serial_basic_4.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(11, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.vertices[16].is_syndrome = true;
        code.vertices[52].is_syndrome = true;
        code.vertices[65].is_syndrome = true;
        code.vertices[76].is_syndrome = true;
        code.vertices[112].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        interface.debug_print_actions = true;
        primal_module.load(&interface);  // load syndrome and connect to the dual module interface
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module, &primal_module]).unwrap();
        interface.grow(2 * half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module, &primal_module]).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        visualizer.snapshot_combined(format!("resolve one"), vec![&interface, &dual_module, &primal_module]).unwrap();
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        // primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        // let group_max_update_length = dual_module.compute_maximum_update_length();
        // println!("group_max_update_length: {:?}", group_max_update_length);
    }

}
