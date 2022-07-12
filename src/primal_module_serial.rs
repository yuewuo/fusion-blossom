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
    /// cached interface that can be more up-to-date than the dual node interface
    dual_node_cache: DualNode,
}

impl PrimalNodeInternal {

    /// check if in the cache, this node is a free node
    pub fn is_free(&self) -> bool {
        assert!(self.dual_node_cache.parent_blossom.is_none(), "do not call this function to a internal node, consider call PrimalModuleSerial::get_outer_node");
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
                dual_node_cache: node.clone(),
            };
            self.nodes.push(Some(PrimalNodeInternalPtr::new(primal_node_internal)));
        }
    }

    fn resolve(&mut self, mut group_max_update_length: GroupMaxUpdateLength) -> PrimalInstructionVec {
        debug_assert!(!group_max_update_length.is_empty() && group_max_update_length.get_none_zero_growth().is_none());
        let conflicts = group_max_update_length.get_conflicts();
        let mut current_conflict_index = 0;
        let mut resolve_instructions = vec![];
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
                        resolve_instructions.push(PrimalInstruction::UpdateGrowState(node_ptr_1.clone(), DualNodeGrowState::Stay));
                        resolve_instructions.push(PrimalInstruction::UpdateGrowState(node_ptr_2.clone(), DualNodeGrowState::Stay));
                        continue
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
        resolve_instructions
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
        if let Some(parent_dual_node_ptr) = &node.dual_node_cache.parent_blossom {
            let parent_primal_node_internal_ptr = self.get_primal_node_internal_ptr(parent_dual_node_ptr);
            self.get_outer_node(parent_primal_node_internal_ptr)
        } else {
            primal_node_internal_ptr.clone()
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
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        let resolve_instructions = primal_module.resolve(group_max_update_length);
        println!("resolve_instructions: {:?}", resolve_instructions);
        interface.execute_resolve_instructions(&mut dual_module, resolve_instructions);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        println!("group_max_update_length: {:?}", group_max_update_length);
        // let resolve_instructions = primal_module.resolve(group_max_update_length);
        // println!("resolve_instructions: {:?}", resolve_instructions);
    }

}
