//! Serial Dual Parallel
//! 
//! A parallel implementation of the dual module, leveraging the serial version
//! 
//! While it assumes single machine (using async runtime of Rust), the design targets distributed version
//! that can spawn on different machines efficiently. The distributed version can be build based on this 
//! parallel version.
//!

use super::util::*;
use std::sync::{Arc, Weak};
use super::dual_module::*;
use super::dual_module_serial::*;
use crate::parking_lot::RwLock;


pub struct DualModuleParallel {
    /// initializer, used for customized division
    pub initializer: SolverInitializer,
    /// the number of divided serial modules; these are the units that that are preserved after it's cleared
    pub division: usize,
    /// the basic wrapped serial modules at the beginning, afterwards the fused units are appended after them
    pub units: Vec<DualModuleParallelUnitPtr>,
    /// the mapping from vertices to units: serial unit (holding real vertices) as well as parallel units (holding interfacing vertices);
    /// used for loading syndrome to the holding units
    pub vertex_to_unit: Vec<usize>,
}

pub struct DualModuleParallelUnit {
    /// `Some(_)` only if this parallel dual module is a simple wrapper of a serial dual module
    pub wrapped_module: Option<DualModuleSerialPtr>,
    /// left dual module
    pub left: DualModuleParallelUnitWeak,
    /// right dual module
    pub right: DualModuleParallelUnitWeak,
    /// interfacing nodes between the left and right
    pub nodes: Vec<Option<DualNodeInternalPtr>>,
    /// interface ids (each dual module may have multiple interfaces, e.g. in case A-B, B-C, C-D, D-A,
    /// if ABC is in the same module, D is in another module, then there are two interfaces C-D, D-A between modules ABC and D)
    pub interfaces: Vec<Weak<Interface>>,
}

impl DualModuleImpl for DualModuleParallel {

    /// initialize the dual module, which is supposed to be reused for multiple decoding tasks with the same structure
    fn new(initializer: &SolverInitializer) -> Self {
        Self {
            initializer: initializer.clone(),
            division: 0,  // invalid construction, need to initialize later
            units: vec![],
            vertex_to_unit: vec![],
        }
    }

    /// clear all growth and existing dual nodes
    fn clear(&mut self) {
        unimplemented!()
    }

    /// add a new dual node from dual module root
    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        unimplemented!()
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        unimplemented!()
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        unimplemented!()
    }

    fn compute_maximum_update_length_dual_node(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        unimplemented!()
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        unimplemented!()
    }

    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        unimplemented!()
    }

    fn grow(&mut self, length: Weight) {
        unimplemented!()
    }

    fn load_edge_modifier(&mut self, edge_modifier: &Vec<(EdgeIndex, Weight)>) {
        unimplemented!()
    }

}

create_ptr_types!(DualModuleParallelUnit, DualModuleParallelUnitPtr, DualModuleParallelUnitWeak);

/// interface consists of several vertices; each vertex exists as a virtual vertex in several different serial dual modules.
/// each virtual vertex exists in at most one interface
pub struct InterfaceData {
    /// the serial dual modules that processes these virtual vertices,
    pub possession_modules: Vec<DualModuleSerialWeak>,
    /// the virtual vertices references in different modules, [idx of serial dual module] [idx of interfacing vertex]
    pub interfacing_vertices: Vec<Vec<VertexWeak>>,
}

/// interface between dual modules, consisting of a list of nodes of virtual nodes that sits on different modules
pub struct Interface {
    /// unique interface id for ease of zero-cost switching
    pub interface_id: usize,
    /// link to interface data
    pub data: Weak<InterfaceData>,
}
