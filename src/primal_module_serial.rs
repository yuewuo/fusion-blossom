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
    pub temporary_match: Option<PrimalNodeInternalPtr>,
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
                        primal_node_internal_1.temporary_match = Some(primal_node_internal_ptr_2.clone());
                        primal_node_internal_2.temporary_match = Some(primal_node_internal_ptr_1.clone());
                        // update dual module interface
                        interface.set_grow_state(primal_node_internal_1.origin.clone(), DualNodeGrowState::Stay, dual_module);
                        interface.set_grow_state(primal_node_internal_2.origin.clone(), DualNodeGrowState::Stay, dual_module);
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
                        let leaf_node_internal_ptr = matched_node_internal.temporary_match.as_ref().unwrap().clone();
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
                        leaf_node_internal.tree_node = Some(AlternatingTreeNode {
                            root: free_node_internal_ptr.clone(),
                            parent: Some(matched_node_internal_ptr.clone()),
                            children: vec![],
                            depth: 2,
                        });
                        // update dual module interface
                        interface.set_grow_state(free_node_internal.origin.clone(), DualNodeGrowState::Grow, dual_module);
                        interface.set_grow_state(matched_node_internal.origin.clone(), DualNodeGrowState::Shrink, dual_module);
                        interface.set_grow_state(leaf_node_internal.origin.clone(), DualNodeGrowState::Grow, dual_module);
                        continue
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
                            // TODO: handle other tree structure
                            
                            continue
                        }
                    }
                    unimplemented!()
                },
                MaxUpdateLength::TouchingVirtual(node_ptr, virtual_vertex_index) => {
                    unimplemented!()
                },
                MaxUpdateLength::BlossomNeedExpand(node_ptr) => {
                    // TODO: we need to break the while loop here because expanding a blossom will lead to ambiguous conflicts
                    // blossom breaking is assumed to be very rare given our multiple-tree approach, so don't need to optimize for it
                    unimplemented!()
                },
                MaxUpdateLength::VertexShrinkStop(node_ptr) => {
                    if current_conflict_index == 1 {
                        // if this happens, then debug the sorting of conflict events and also check alternating tree: a vertex should never be a floating "-" node
                        unreachable!("VertexShrinkStop conflict cannot be solved by primal module, and should be sorted to the last of the heap")
                    }
                    // just skip and wait for the next round to resolve it, if it's not being resolved already
                    continue
                }
                _ => unreachable!("should not resolve these issues")
            }
        }
    }

}

impl FusionVisualizer for PrimalModuleSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        unimplemented!()
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
        primal_module.load(&interface);  // load syndrome and connect to the dual module interface
        visualizer.snapshot(format!("syndrome"), &dual_module).unwrap();
        dual_module.grow(half_weight);
        visualizer.snapshot(format!("grow"), &dual_module).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        dual_module.grow(half_weight);
        visualizer.snapshot(format!("alternating tree grow"), &dual_module).unwrap();
        // cannot grow anymore, resolve conflicts
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
        // conflicts resolved, grow again
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        dual_module.grow(2 * half_weight);
        visualizer.snapshot(format!("blossom grow"), &dual_module).unwrap();
    }

}
