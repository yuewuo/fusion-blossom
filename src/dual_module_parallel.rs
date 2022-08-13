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
use crate::futures::executor::{block_on, ThreadPool};
use crate::futures::task::SpawnExt;
use super::visualize::*;


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
        let mut units = vec![];
        let mut vertex_to_unit = Vec::with_capacity(initializer.vertex_num);
        if config.division == 1 {  // no division
            let dual_module = DualModuleSerial::new(&initializer);
            let dual_module_ptr = DualModuleSerialPtr::new(dual_module);
            let unit = DualModuleParallelUnitPtr::new_wrapper(dual_module_ptr);
            units.push(unit);
            for _ in 0..initializer.vertex_num {
                vertex_to_unit.push(0);  // all vertices belongs to the only unit
            }
        } else {  // exist division
            unimplemented!()
        }
        Self {
            initializer: initializer.clone(),
            units: units,
            vertex_to_unit: vertex_to_unit,
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
                block_on(self.thread_pool.spawn_with_handle(async move {
                    unit_ptr.clear().await
                }).unwrap());
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
                })
            }
        }
        if !async_tasks.is_empty() {
            block_on(self.thread_pool.spawn_with_handle(async { join_all(async_tasks).await; }).unwrap());
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
            block_on(self.thread_pool.spawn_with_handle(async { join_all(async_tasks).await; }).unwrap());
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
            block_on(self.thread_pool.spawn_with_handle(async { join_all(async_tasks).await; }).unwrap());
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
        let group_max_update_length = if async_tasks.is_empty() {
            GroupMaxUpdateLength::new()
        } else {
            block_on(self.thread_pool.spawn_with_handle(async move {
                let mut group_max_update_length = GroupMaxUpdateLength::new();
                let results = join_all(async_tasks).await;
                for max_update_length in results.into_iter() {
                    group_max_update_length.add(max_update_length);
                }
                group_max_update_length
            }).unwrap())
        };
        match group_max_update_length {
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
        let group_max_update_length = if async_tasks.is_empty() {
            GroupMaxUpdateLength::new()
        } else {
            block_on(self.thread_pool.spawn_with_handle(async move {
                let mut group_max_update_length = GroupMaxUpdateLength::new();
                let results = join_all(async_tasks).await;
                for local_group_max_update_length in results.into_iter() {
                    group_max_update_length.extend(local_group_max_update_length);
                }
                group_max_update_length
            }).unwrap())
        };
        group_max_update_length
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
            block_on(self.thread_pool.spawn_with_handle(async { join_all(async_tasks).await; }).unwrap());
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
            block_on(self.thread_pool.spawn_with_handle(async { join_all(async_tasks).await; }).unwrap());
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
            block_on(self.thread_pool.spawn_with_handle(async { join_all(async_tasks).await; }).unwrap());
        }
    }

}


/*
Implementing visualization functions
*/

impl FusionVisualizer for DualModuleParallel {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // do the sanity check first before taking snapshot
        // self.sanity_check().unwrap();
        if self.config.division == 1 {
            self.units[0].read_recursive().wrapped_module.as_ref().unwrap().read_recursive().snapshot(abbrev)
        } else {
            unimplemented!();
        }
    }
}

/// We cannot implement async function because a RwLockWriteGuard implements !Send
impl DualModuleParallelUnitPtr {

    /// create a simple wrapper over a serial dual module
    pub fn new_wrapper(dual_module_ptr: DualModuleSerialPtr) -> Self {
        Self::new(DualModuleParallelUnit {
            is_active: true,
            is_fused: false,
            wrapped_module: Some(dual_module_ptr),
            children: None,
            parent: None,
            interfaces: vec![],
            nodes: vec![],
        })
    }

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


#[cfg(test)]
pub mod tests {
    use super::*;
    use super::super::example::*;
    use super::super::primal_module::*;
    use super::super::primal_module_serial::*;

    pub fn dual_module_parallel_basic_standard_syndrome_optional_viz(d: usize, visualize_filename: Option<String>, syndrome_vertices: Vec<VertexIndex>, final_dual: Weight)
            -> (DualModuleInterface, PrimalModuleSerial, DualModuleParallel) {
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
        let mut dual_module = DualModuleParallel::new(&initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerial::new(&initializer);
        primal_module.debug_resolve_only_one = true;  // to enable debug mode
        // try to work on a simple syndrome
        code.set_syndrome(&syndrome_vertices);
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        interface.debug_print_actions = true;
        primal_module.load(&interface);  // load syndrome and connect to the dual module interface
        visualizer.as_mut().map(|v| v.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module, &primal_module]).unwrap());
        // grow until end
        let mut group_max_update_length = dual_module.compute_maximum_update_length();
        while !group_max_update_length.is_empty() {
            println!("group_max_update_length: {:?}", group_max_update_length);
            if let Some(length) = group_max_update_length.get_none_zero_growth() {
                interface.grow(length, &mut dual_module);
                visualizer.as_mut().map(|v| v.snapshot_combined(format!("grow {length}"), vec![&interface, &dual_module, &primal_module]).unwrap());
            } else {
                let first_conflict = format!("{:?}", group_max_update_length.get_conflicts().peek().unwrap());
                primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
                visualizer.as_mut().map(|v| v.snapshot_combined(format!("resolve {first_conflict}"), vec![&interface, &dual_module, &primal_module]).unwrap());
            }
            group_max_update_length = dual_module.compute_maximum_update_length();
        }
        assert_eq!(interface.sum_dual_variables, final_dual * 2 * half_weight, "unexpected final dual variable sum");
        (interface, primal_module, dual_module)
    }

    pub fn dual_module_parallel_standard_syndrome(d: usize, visualize_filename: String, syndrome_vertices: Vec<VertexIndex>, final_dual: Weight)
            -> (DualModuleInterface, PrimalModuleSerial, DualModuleParallel) {
        dual_module_parallel_basic_standard_syndrome_optional_viz(d, Some(visualize_filename), syndrome_vertices, final_dual)
    }

    /// test a simple case
    #[test]
    fn dual_module_parallel_basic_1() {  // cargo test dual_module_parallel_basic_1 -- --nocapture
        let visualize_filename = format!("dual_module_parallel_basic_1.json");
        let syndrome_vertices = vec![39, 52, 63, 90, 100];
        dual_module_parallel_standard_syndrome(11, visualize_filename, syndrome_vertices, 9);
    }

}
