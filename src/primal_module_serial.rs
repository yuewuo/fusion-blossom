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
}

pub struct PrimalNodeInternalPtr { ptr: Arc<RwLock<PrimalNodeInternal>>, }

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

/// internal information of the primal node, added to the [`DualNode`]; note that primal nodes and dual nodes
/// always have one-to-one correspondence
#[derive(Derivative)]
#[derivative(Debug)]
pub struct PrimalNodeInternal {
    /// the pointer to the origin [`DualNode`]
    pub origin: DualNodePtr,
    /// local index, to find myself in [`DualModuleSerial::nodes`]
    index: NodeIndex,
}

impl PrimalModuleImpl for PrimalModuleSerial {

    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self {
        unimplemented!()
    }

    fn clear(&mut self) {
        unimplemented!()
    }

    fn update(&mut self, max_update_length: &MaxUpdateLength) -> PrimalInstructionVec {
        unimplemented!()
    }

}

impl FusionVisualizer for PrimalModuleSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        unimplemented!()
    }
}
