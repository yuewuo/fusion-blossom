//! Serial Dual Parallel
//! 
//! A parallel implementation of the dual module, leveraging the serial version
//! 
//! While it assumes single machine (using async runtime of Rust), the design targets distributed version
//! that can spawn on different machines efficiently. The distributed version can be build based on this 
//! parallel version.
//! 
//! Notes:
//! 
//! According to https://tokio.rs/tokio/tutorial, tokio is not good for parallel computation. It suggests
//! using https://docs.rs/rayon/latest/rayon/. 
//!

use super::util::*;
use std::sync::{Arc, Weak};
use super::dual_module::*;
use super::dual_module_serial::*;
use crate::parking_lot::RwLock;
use super::serde_json;
use serde::{Serialize, Deserialize};
use super::futures::future::join_all;
use crate::futures::executor::ThreadPool;
// use crate::futures::FutureExt;  // .boxed()


pub struct DualModuleParallel {
    /// initializer, used for customized division
    pub initializer: SolverInitializer,
    /// the basic wrapped serial modules at the beginning, afterwards the fused units are appended after them
    pub units: Vec<DualModuleParallelUnitPtr>,
    /// the mapping from vertices to units: serial unit (holding real vertices) as well as parallel units (holding interfacing vertices);
    /// used for loading syndrome to the holding units
    pub vertex_to_unit: Vec<usize>,
    /// configuration
    pub config: DualModuleParallelConfig,
    /// thread pool used to execute async functions
    pub thread_pool: ThreadPool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualModuleParallelConfig {
    /// enable async execution of dual operations
    #[serde(default = "dual_module_parallel_default_configs::thread_pool_size")]
    pub thread_pool_size: usize,
    /// the number of divided serial modules; these are the units that that are preserved after it's cleared
    #[serde(default = "dual_module_parallel_default_configs::division")]
    pub division: usize,
}

pub mod dual_module_parallel_default_configs {
    pub fn thread_pool_size() -> usize { 0 }  // by default to the number of CPU cores
    pub fn division() -> usize { 1 }  // by default no division: a single unit contains all
}

pub struct DualModuleParallelUnit {
    /// fused module is not accessible globally: it must be accessed from its parent
    pub is_fused: bool,
    /// whether it's active or not; some units are "placeholder" units that are not active until they actually fuse their children
    pub is_active: bool,
    /// `Some(_)` only if this parallel dual module is a simple wrapper of a serial dual module
    pub wrapped_module: Option<DualModuleSerialPtr>,
    /// left and right children dual modules
    pub children: Option<(DualModuleParallelUnitWeak, DualModuleParallelUnitWeak)>,
    /// parent dual module
    pub parent: Option<DualModuleParallelUnitWeak>,
    /// interfacing nodes between the left and right
    pub nodes: Vec<Option<DualNodeInternalPtr>>,
    /// interface ids (each dual module may have multiple interfaces, e.g. in case A-B, B-C, C-D, D-A,
    /// if ABC is in the same module, D is in another module, then there are two interfaces C-D, D-A between modules ABC and D)
    pub interfaces: Vec<Weak<Interface>>,
}

create_ptr_types!(DualModuleParallelUnit, DualModuleParallelUnitPtr, DualModuleParallelUnitWeak);

impl DualModuleParallel {

    /// recommended way to create a new instance, given a customized configuration
    pub fn new_config(initializer: &SolverInitializer, config: DualModuleParallelConfig) -> Self {
        let mut thread_pool_builder = ThreadPool::builder();
        if config.thread_pool_size != 0 {
            thread_pool_builder.pool_size(config.thread_pool_size);
        }
        let thread_pool = thread_pool_builder.create().expect("creating thread pool failed");
        assert!(config.division > 0, "0 division forbidden");
        if config.division == 1 {  // no division
            // let dual_module = 
        } else {  // exist division
            unimplemented!()
        }
        Self {
            initializer: initializer.clone(),
            units: vec![],
            vertex_to_unit: vec![],
            config: config,
            thread_pool: thread_pool,
        }
    }

}

impl DualModuleImpl for DualModuleParallel {

    /// initialize the dual module, which is supposed to be reused for multiple decoding tasks with the same structure
    fn new(initializer: &SolverInitializer) -> Self {
        let config: DualModuleParallelConfig = serde_json::from_value(json!({})).unwrap();
        Self::new_config(initializer, config)
    }

    /// clear all growth and existing dual nodes
    fn clear(&mut self) {
        for (unit_idx, unit_ptr) in self.units.iter().enumerate() {
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                self.thread_pool.spawn_ok(async move {
                    unit_ptr.clear().await
                });
            }
            let mut unit = unit_ptr.write();
            unit.is_fused = false;  // everything is not fused at the beginning
            unit.is_active = unit_idx < self.config.division;  // only divided serial modules are active at the beginning
        }
    }

    // although not the intended way to use it, we do support these common APIs for compatibility with normal primal modules

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                let dual_node_ptr = dual_node_ptr.clone();
                async_tasks.push(async move {
                    unit_ptr.add_dual_node(&dual_node_ptr).await
                });
            }
        }
        if !async_tasks.is_empty() {
            self.thread_pool.spawn_ok(async { join_all(async_tasks).await; });
        }
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                let dual_node_ptr = dual_node_ptr.clone();
                async_tasks.push(async move {
                    unit_ptr.remove_blossom(dual_node_ptr.clone()).await
                });
            }
        }
        if !async_tasks.is_empty() {
            self.thread_pool.spawn_ok(async { join_all(async_tasks).await; });
        }
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                let dual_node_ptr = dual_node_ptr.clone();
                let grow_state = grow_state.clone();
                async_tasks.push(async move {
                    unit_ptr.set_grow_state(&dual_node_ptr, grow_state).await
                });
            }
        }
        if !async_tasks.is_empty() {
            self.thread_pool.spawn_ok(async { join_all(async_tasks).await; });
        }
    }

    fn compute_maximum_update_length_dual_node(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                let dual_node_ptr = dual_node_ptr.clone();
                async_tasks.push(async move {
                    unit_ptr.compute_maximum_update_length_dual_node(&dual_node_ptr, is_grow, simultaneous_update).await
                });
            }
        }
        let group_max_update_length = Arc::new(RwLock::new(GroupMaxUpdateLength::new()));
        if !async_tasks.is_empty() {
            {  // copy async data
                let group_max_update_length = group_max_update_length.clone();
                self.thread_pool.spawn_ok(async move {
                    let results = join_all(async_tasks).await;
                    let mut group_max_update_length = group_max_update_length.write();
                    for max_update_length in results.into_iter() {
                        group_max_update_length.add(max_update_length);
                    }
                });
            }
        }
        match Arc::try_unwrap(group_max_update_length).unwrap().into_inner() {
            GroupMaxUpdateLength::NonZeroGrow(weight) => MaxUpdateLength::NonZeroGrow(weight),
            GroupMaxUpdateLength::Conflicts(mut conflicts) => conflicts.pop().unwrap(),  // just return the first conflict is fine
        }
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                async_tasks.push(async move {
                    unit_ptr.compute_maximum_update_length().await
                });
            }
        }
        let group_max_update_length = Arc::new(RwLock::new(GroupMaxUpdateLength::new()));
        if !async_tasks.is_empty() {
            {  // copy async data
                let group_max_update_length = group_max_update_length.clone();
                self.thread_pool.spawn_ok(async move {
                    let results = join_all(async_tasks).await;
                    let mut group_max_update_length = group_max_update_length.write();
                    for local_group_max_update_length in results.into_iter() {
                        group_max_update_length.extend(local_group_max_update_length);
                    }
                });
            }
        }
        Arc::try_unwrap(group_max_update_length).unwrap().into_inner()
    }

    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                let dual_node_ptr = dual_node_ptr.clone();
                async_tasks.push(async move {
                    unit_ptr.grow_dual_node(&dual_node_ptr, length).await
                });
            }
        }
        if !async_tasks.is_empty() {
            self.thread_pool.spawn_ok(async { join_all(async_tasks).await; });
        }
    }

    fn grow(&mut self, length: Weight) {
        let mut async_tasks = Vec::new();
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                async_tasks.push(async move {
                    unit_ptr.grow(length).await
                });
            }
        }
        if !async_tasks.is_empty() {
            self.thread_pool.spawn_ok(async { join_all(async_tasks).await; });
        }
    }

    fn load_edge_modifier(&mut self, edge_modifier: &Vec<(EdgeIndex, Weight)>) {
        let mut async_tasks = Vec::new();
        let edge_modifier = Arc::new(edge_modifier.clone());  // share as async data
        for unit_ptr in self.units.iter() {
            if !unit_ptr.write().is_active { continue }
            {  // copy async data
                let unit_ptr = unit_ptr.clone();
                let edge_modifier = edge_modifier.clone();
                async_tasks.push(async move {
                    unit_ptr.load_edge_modifier(&edge_modifier).await
                });
            }
        }
        if !async_tasks.is_empty() {
            self.thread_pool.spawn_ok(async { join_all(async_tasks).await; });
        }
    }

}

/// We cannot implement async function because a RwLockWriteGuard implements !Send
impl DualModuleParallelUnitPtr {

    /// clear all growth and existing dual nodes
    pub async fn clear(&self) {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().clear();
        } else {
            unimplemented!()
        }
    }

    /// add a new dual node from dual module root
    pub async fn add_dual_node(&self, dual_node_ptr: &DualNodePtr) {
        let unit = self.write();
        // TODO: determine whether `dual_node_ptr` has anything to do with the underlying dual module, if not, simply return
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().add_dual_node(dual_node_ptr)
        } else {
            unimplemented!()
        }
    }

    pub async fn remove_blossom(&self, dual_node_ptr: DualNodePtr) {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().remove_blossom(dual_node_ptr)
        } else {
            unimplemented!()
        }
    }

    pub async fn set_grow_state(&self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().set_grow_state(dual_node_ptr, grow_state)
        } else {
            unimplemented!()
        }
    }

    pub async fn compute_maximum_update_length_dual_node(&self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().compute_maximum_update_length_dual_node(dual_node_ptr, is_grow, simultaneous_update)
        } else {
            unimplemented!()
        }
    }

    pub async fn compute_maximum_update_length(&self) -> GroupMaxUpdateLength {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().compute_maximum_update_length()
        } else {
            unimplemented!()
        }
    }

    pub async fn grow_dual_node(&self, dual_node_ptr: &DualNodePtr, length: Weight) {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().grow_dual_node(dual_node_ptr, length)
        } else {
            unimplemented!()
        }
    }

    pub async fn grow(&self, length: Weight) {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().grow(length)
        } else {
            unimplemented!()
        }
    }

    pub async fn load_edge_modifier(&self, edge_modifier: &Vec<(EdgeIndex, Weight)>) {
        let unit = self.write();
        if let Some(dual_module_ptr) = unit.wrapped_module.as_ref() {
            dual_module_ptr.write().load_edge_modifier(edge_modifier)
        } else {
            unimplemented!()
        }
    }

}

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
