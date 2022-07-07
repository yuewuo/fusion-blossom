//! Dual Module
//! 
//! Generics for dual modules, defining the necessary interfaces for a dual module
//!

use super::util::*;
use std::sync::Arc;
use crate::derivative::Derivative;
use crate::parking_lot::RwLock;
use std::any::Any;


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

/// A dual node corresponds to either a vertex or a blossom (on which the dual variables are defined)
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNode {
    /// the index of this dual node, helps to locate internal details of this dual node
    pub index: NodeIndex,
    /// the implementation internal node if applicable
    pub internal: Option<Arc<RwLock<dyn Any>>>,
    /// the class of this dual node
    pub class: DualNodeClass,
    /// whether it grows, stays or shrinks
    pub grow_state: DualNodeGrowState,
}

/// the shared pointer of [`DualNode`]
pub type DualNodePtr = Arc<RwLock<DualNode>>;

/// a sharable array of dual nodes, supporting dynamic partitioning
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNodeArray {
    pub nodes: Vec<Option<DualNodePtr>>,
}

/// common trait that must be implemented for each implementation of dual module
pub trait DualModule {

    /// create a new dual module
    fn new(vertex_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_vertices: &Vec<usize>) -> Self;

    /// clear all growth and existing dual nodes, prepared for the next decoding
    fn clear(&mut self);

    /// create corresponding dual node, note that [`DualNode.internal`] must be None, i.e. each dual node must be created exactly once
    fn create_dual_node(&mut self, node: DualNodePtr);

    /// helper function to specifically create a vertex node
    fn create_vertex_node(&mut self, node: DualNodePtr) {
        debug_assert!({
            let node = node.read_recursive();
            matches!(node.class, DualNodeClass::SyndromeVertex{ .. })
        }, "node class mismatch");
        self.create_dual_node(node)
    }

    /// helper function to specifically create a blossom node
    fn create_blossom(&mut self, node: DualNodePtr) {
        debug_assert!({
            let node = node.read_recursive();
            matches!(node.class, DualNodeClass::Blossom{ .. })
        }, "node class mismatch");
        self.create_dual_node(node)
    }

    /// expand a blossom
    fn expand_blossom(&mut self, node: DualNodePtr);

}
