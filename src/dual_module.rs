//! Dual Module
//! 
//! Generics for dual modules, defining the necessary interfaces for a dual module
//!

use super::util::*;
use std::sync::Arc;
use crate::derivative::Derivative;
use crate::parking_lot::RwLock;


/// A dual node is either a blossom or a vertex
#[derive(Derivative)]
#[derivative(Debug)]
pub enum DualNodeClass {
    Blossom {
        nodes_circle: Vec<NodeIndex>,
    },
    SyndromeVertex {
        syndrome_index: VertexIndex,
    },
}

/// Three possible states: Grow (+1), Stay (+0), Shrink (-1)
#[derive(Derivative)]
#[derivative(Debug)]
pub enum DualNodeGrowState {
    Grow,
    Stay,
    Shrink,
}

/// gives the maximum absolute length to grow, if not possible, give the reason
#[derive(Derivative)]
#[derivative(Debug)]
pub enum MaximumUpdateLength {
    /// non-zero maximum update length
    NonZeroGrow(Weight),
    /// unimplemented length, only used during development, should be removed later
    Unimplemented,
}

/// A dual node corresponds to either a vertex or a blossom (on which the dual variables are defined)
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNode {
    /// the index of this dual node, helps to locate internal details of this dual node
    pub index: NodeIndex,
    /// the implementation internal node, providing the index of it
    pub internal: Option<usize>,
    /// the class of this dual node
    pub class: DualNodeClass,
    /// whether it grows, stays or shrinks
    pub grow_state: DualNodeGrowState,
}

/// the shared pointer of [`DualNode`]
pub type DualNodePtr = Arc<RwLock<DualNode>>;

/// a sharable array of dual nodes, supporting dynamic partitioning;
/// note that a node can be destructed and we do not reuse its index, leaving a blank space
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualModuleRoot {
    /// all the dual node that can be used to control a concrete dual module implementation
    pub nodes: Vec<Option<DualNodePtr>>,
}

/// common trait that must be implemented for each implementation of dual module
pub trait DualModuleImpl {

    /// create a new dual module
    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self;

    /// clear all growth and existing dual nodes, prepared for the next decoding
    fn clear(&mut self);

    /// create corresponding dual node, note that [`DualNode.internal`] must be None, i.e. each dual node must be created exactly once
    fn create_dual_node(&mut self, node_array: &DualModuleRoot, node: DualNodePtr);

    #[inline(always)]
    /// helper function to specifically create a vertex node
    fn create_vertex_node(&mut self, node_array: &DualModuleRoot, node: DualNodePtr) {
        debug_assert!({
            let node = node.read_recursive();
            matches!(node.class, DualNodeClass::SyndromeVertex{ .. })
        }, "node class mismatch");
        self.create_dual_node(node_array, node)
    }

    #[inline(always)]
    /// helper function to specifically create a blossom node
    fn create_blossom(&mut self, node_array: &DualModuleRoot, node: DualNodePtr) {
        debug_assert!({
            let node = node.read_recursive();
            matches!(node.class, DualNodeClass::Blossom{ .. })
        }, "node class mismatch");
        self.create_dual_node(node_array, node)
    }

    /// expand a blossom
    fn expand_blossom(&mut self, node: DualNodePtr);

    /// An optional function that helps to break down the implementation of [`DualModuleImpl::compute_maximum_update_length`]
    /// check the maximum length to grow (shrink) specific dual node, if length is 0, give the reason of why it cannot further grow (shrink).
    /// if `is_grow` is false, return `length` <= 0, in any case |`length`| is maximized so that at least one edge becomes fully grown or fully not-grown.
    /// if `simultaneous_update` is true, also check for the peer node according to [`DualNode::grow_state`].
    fn compute_maximum_update_length_dual_node(&mut self, _dual_node_ptr: &DualNodePtr, _is_grow: bool, _simultaneous_update: bool) -> MaximumUpdateLength {
        panic!("this dual module implementation doesn't support this function, please use another dual module")
    }

    /// check the maximum length to grow (shrink) for all nodes
    fn compute_maximum_grow_length(&mut self) -> MaximumUpdateLength;

    /// An optional function that can manipulate individual dual node, not necessarily supported by all implementations
    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        panic!("this dual module implementation doesn't support this function, please use another dual module")
    }

    /// grow a specific length globally, length must be positive.
    /// note that reversing the process is possible, but not recommended: to do that, reverse the state of each dual node, Grow->Shrink, Shrink->Grow
    fn grow(&mut self, length: Weight);

}

impl DualModuleRoot {

    pub fn new(syndrome: &Vec<VertexIndex>, dual_module_impl: &mut impl DualModuleImpl) -> Self {
        let mut array = Self {
            nodes: Vec::new(),
        };
        for vertex_idx in syndrome.iter() {
            let node_ptr = array.new_syndrome_vertex(*vertex_idx);
            dual_module_impl.create_vertex_node(&array, node_ptr);
        }
        array
    }

    /// create a dual node corresponding to a syndrome vertex
    pub fn new_syndrome_vertex(&mut self, vertex_idx: VertexIndex) -> DualNodePtr {
        let node_idx = self.nodes.len();
        let node_ptr = Arc::new(RwLock::new(DualNode {
            index: node_idx,
            internal: None,
            class: DualNodeClass::SyndromeVertex {
                syndrome_index: vertex_idx,
            },
            grow_state: DualNodeGrowState::Grow,
        }));
        self.nodes.push(Some(Arc::clone(&node_ptr)));
        node_ptr
    }

}
