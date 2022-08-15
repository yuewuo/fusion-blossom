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
use crate::serde_json;
use serde::{Serialize, Deserialize};
use super::visualize::*;
use crate::rayon::prelude::*;


pub struct DualModuleParallel {
    /// initializer, used for customized partition
    pub initializer: SolverInitializer,
    /// the basic wrapped serial modules at the beginning, afterwards the fused units are appended after them
    pub units: Vec<DualModuleParallelUnitPtr>,
    /// the mapping from vertices to units: serial unit (holding real vertices) as well as parallel units (holding interfacing vertices);
    /// used for loading syndrome to the holding units
    pub vertex_to_unit: Vec<usize>,
    /// configuration
    pub config: DualModuleParallelConfig,
    /// thread pool used to execute async functions in parallel
    pub thread_pool: rayon::ThreadPool,

}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DualModuleParallelConfig {
    /// enable async execution of dual operations
    #[serde(default = "dual_module_parallel_default_configs::thread_pool_size")]
    pub thread_pool_size: usize,
    /// detailed plan of partitioning serial modules: each serial module possesses a list of vertices, including all interface vertices
    #[serde(default = "dual_module_parallel_default_configs::partitions")]
    pub partitions: Vec<Vec<VertexIndex>>,
    /// detailed plan of interfacing vertices
    #[serde(default = "dual_module_parallel_default_configs::interfaces")]
    pub interfaces: Vec<(usize, usize, Vec<VertexIndex>)>,
}

impl DualModuleParallelConfig {

    pub fn partition_sanity_check(&self, initializer: &SolverInitializer) {
        assert!(self.partitions.len() > 0, "at least one partition must exist");
        // first verify that each vertex is only present in a single partition
        let mut vertex_partitioned: Vec<Option<usize>> = (0..initializer.vertex_num).map(|_| None).collect();
        for (partition_index, partition) in self.partitions.iter().enumerate() {
            for vertex_index in partition.iter().cloned() {
                assert!(vertex_index < initializer.vertex_num, "invalid vertex index {} in partitions", vertex_index);
                assert!(vertex_partitioned[vertex_index].is_none(), "duplicate partition of vertex {}", vertex_index);
                vertex_partitioned[vertex_index] = Some(partition_index);
            }
        }
        let mut parents: Vec<Option<usize>> = (0..self.partitions.len() + self.interfaces.len()).map(|_| None).collect();
        for (interface_index, (left_index, right_index, interface)) in self.interfaces.iter().enumerate() {
            let unit_index = interface_index +  self.partitions.len();
            assert!(*left_index < unit_index, "dependency wrong, {} depending on {}", unit_index, left_index);
            assert!(*right_index < unit_index, "dependency wrong, {} depending on {}", unit_index, right_index);
            assert!(parents[*left_index].is_none(), "cannot fuse {} twice", left_index);
            assert!(parents[*right_index].is_none(), "cannot fuse {} twice", right_index);
            parents[*left_index] = Some(unit_index);
            parents[*right_index] = Some(unit_index);
            for vertex_index in interface.iter().cloned() {
                assert!(vertex_index < initializer.vertex_num, "invalid vertex index {} in partitions", vertex_index);
                assert!(vertex_partitioned[vertex_index].is_none(), "duplicate partition of vertex {}", vertex_index);
                vertex_partitioned[vertex_index] = Some(unit_index);
            }
        }
        // check that all nodes except for the last one has been merged
        for unit_index in 0..self.partitions.len() + self.interfaces.len() - 1 {
            assert!(parents[unit_index].is_some(), "found unit {} without being fused", unit_index);
        }
    }

}

impl Default for DualModuleParallelConfig {
    fn default() -> Self { serde_json::from_value(json!({})).unwrap() }
}

pub mod dual_module_parallel_default_configs {
    use super::super::util::*;
    pub fn thread_pool_size() -> usize { 0 }  // by default to the number of CPU cores
    pub fn partitions() -> Vec<Vec<VertexIndex>> { vec![] }  // by default, this field is optional, and when empty, it will have only 1 partition
    pub fn interfaces() -> Vec<(usize, usize, Vec<VertexIndex>)> { vec![] }  // by default no interface
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
    pub fn new_config(initializer: &SolverInitializer, mut config: DualModuleParallelConfig) -> Self {
        let mut thread_pool_builder = rayon::ThreadPoolBuilder::new();
        if config.thread_pool_size != 0 {
            thread_pool_builder = thread_pool_builder.num_threads(config.thread_pool_size);
        }
        let thread_pool = thread_pool_builder.build().expect("creating thread pool failed");
        if config.partitions.len() == 0 {
            config.partitions = vec![(0..initializer.vertex_num).collect()];
        }
        assert!(config.partitions.len() > 0, "0 partition forbidden");
        let mut units = vec![];
        let mut vertex_to_unit = Vec::with_capacity(initializer.vertex_num);
        if config.partitions.len() == 1 {  // no partition
            let dual_module = DualModuleSerial::new(&initializer);
            let dual_module_ptr = DualModuleSerialPtr::new(dual_module);
            let unit = DualModuleParallelUnitPtr::new_wrapper(dual_module_ptr);
            units.push(unit);
            for _ in 0..initializer.vertex_num {
                vertex_to_unit.push(0);  // all vertices belongs to the only unit
            }
            assert!(config.interfaces.is_empty(), "don't specify `interfaces` if no partition");
        } else {  // multiple partitions, do the initialization in parallel to take advantage of multiple cores
            config.partition_sanity_check(initializer);
            vertex_to_unit = (0..initializer.vertex_num).map(|_| usize::MAX).collect();
            for (partition_index, partition) in config.partitions.iter().enumerate() {
                for vertex_index in partition.iter().cloned() {
                    vertex_to_unit[vertex_index] = partition_index;
                }
            }
            for (interface_index, (_, _, interface)) in config.interfaces.iter().enumerate() {
                let unit_index = interface_index +  config.partitions.len();
                for vertex_index in interface.iter().cloned() {
                    vertex_to_unit[vertex_index] = unit_index;
                }
            }
            let mut partitioned_initializers: Vec<SolverInitializer> = config.partitions.iter().map(|partition| {
                SolverInitializer::new(partition.len(), vec![], vec![])  // note that all fields can be modified later
            }).collect();
            let mut partition_units = vec![];
            let mut interface_units = vec![];
            thread_pool.scope(|s| {
                s.spawn(|_| {
                    (0..config.partitions.len()).into_par_iter().map(|partition_index| {
                        let partition = &config.partitions[partition_index];
                        let dual_module = DualModuleSerial::new(&initializer);
                        println!("partition_index: {partition_index}");
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }).collect_into_vec(&mut partition_units);
                });
                s.spawn(|_| {
                    (0..config.interfaces.len()).into_par_iter().map(|interface_index| {
                        let (left, right, interface) = &config.interfaces[interface_index];
                        println!("interface_index: {interface_index}");
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                    }).collect_into_vec(&mut interface_units);
                });
            });
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
        Self::new_config(initializer, DualModuleParallelConfig::default())
    }

    /// clear all growth and existing dual nodes
    fn clear(&mut self) {
        self.units.par_iter().enumerate().for_each(|(unit_idx, unit_ptr)| {
            let mut unit = unit_ptr.write();
            unit.clear();
            unit.is_fused = false;  // everything is not fused at the beginning
            unit.is_active = unit_idx < self.config.partitions.len();  // only partitioned serial modules are active at the beginning
        });
    }

    // although not the intended way to use it, we do support these common APIs for compatibility with normal primal modules

    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        self.units.par_iter().for_each(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return }
            unit.add_dual_node(&dual_node_ptr);
        });
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        self.units.par_iter().for_each(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return }
            unit.remove_blossom(dual_node_ptr.clone());
        });
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        self.units.par_iter().for_each(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return }
            unit.set_grow_state(&dual_node_ptr, grow_state);
        });
    }

    fn compute_maximum_update_length_dual_node(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        let results: Vec<_> = self.units.par_iter().filter_map(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return None }
            Some(unit.compute_maximum_update_length_dual_node(&dual_node_ptr, is_grow, simultaneous_update))
        }).collect();
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        for max_update_length in results.into_iter() {
            group_max_update_length.add(max_update_length);
        }
        match group_max_update_length {
            GroupMaxUpdateLength::NonZeroGrow(weight) => MaxUpdateLength::NonZeroGrow(weight),
            GroupMaxUpdateLength::Conflicts(mut conflicts) => conflicts.pop().unwrap(),  // just return the first conflict is fine
        }
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        let results: Vec<_> = self.units.par_iter().filter_map(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return None }
            Some(unit.compute_maximum_update_length())
        }).collect();
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        for local_group_max_update_length in results.into_iter() {
            group_max_update_length.extend(local_group_max_update_length);
        }
        group_max_update_length
    }

    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        self.units.par_iter().for_each(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return }
            unit.grow_dual_node(&dual_node_ptr, length);
        });
    }

    fn grow(&mut self, length: Weight) {
        self.units.par_iter().for_each(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return }
            unit.grow(length);
        });
    }

    fn load_edge_modifier(&mut self, edge_modifier: &Vec<(EdgeIndex, Weight)>) {
        self.units.par_iter().for_each(|unit_ptr| {
            let mut unit = unit_ptr.write();
            if !unit.is_active { return }
            unit.load_edge_modifier(edge_modifier);
        });
    }

}


/*
Implementing visualization functions
*/

impl FusionVisualizer for DualModuleParallel {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // do the sanity check first before taking snapshot
        // self.sanity_check().unwrap();
        if self.config.partitions.len() == 1 {
            self.units[0].read_recursive().wrapped_module.as_ref().unwrap().read_recursive().snapshot(abbrev)
        } else {
            unimplemented!();
        }
    }
}

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

}

/// We cannot implement async function because a RwLockWriteGuard implements !Send
impl DualModuleImpl for DualModuleParallelUnit {

    /// clear all growth and existing dual nodes
    fn new(_initializer: &SolverInitializer) -> Self {
        panic!("creating parallel unit directly from initializer is forbidden, use `DualModuleParallel::new` instead");
    }

    /// clear all growth and existing dual nodes
    fn clear(&mut self) {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().clear();
        } else {
            unimplemented!()
        }
    }

    /// add a new dual node from dual module root
    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        // TODO: determine whether `dual_node_ptr` has anything to do with the underlying dual module, if not, simply return
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().add_dual_node(dual_node_ptr)
        } else {
            unimplemented!()
        }
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().remove_blossom(dual_node_ptr)
        } else {
            unimplemented!()
        }
    }

    fn set_grow_state(&mut self, dual_node_ptr: &DualNodePtr, grow_state: DualNodeGrowState) {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().set_grow_state(dual_node_ptr, grow_state)
        } else {
            unimplemented!()
        }
    }

    fn compute_maximum_update_length_dual_node(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().compute_maximum_update_length_dual_node(dual_node_ptr, is_grow, simultaneous_update)
        } else {
            unimplemented!()
        }
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().compute_maximum_update_length()
        } else {
            unimplemented!()
        }
    }

    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().grow_dual_node(dual_node_ptr, length)
        } else {
            unimplemented!()
        }
    }

    fn grow(&mut self, length: Weight) {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
            dual_module_ptr.write().grow(length)
        } else {
            unimplemented!()
        }
    }

    fn load_edge_modifier(&mut self, edge_modifier: &Vec<(EdgeIndex, Weight)>) {
        if let Some(dual_module_ptr) = self.wrapped_module.as_ref() {
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

    pub fn dual_module_parallel_basic_standard_syndrome_optional_viz<F>(d: usize, visualize_filename: Option<String>, syndrome_vertices: Vec<VertexIndex>
            , final_dual: Weight, partition_func: F)
            -> (DualModuleInterface, PrimalModuleSerial, DualModuleParallel) where F: Fn(&SolverInitializer, &mut DualModuleParallelConfig) {
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
        let mut config = DualModuleParallelConfig::default();
        partition_func(&initializer, &mut config);
        let mut dual_module = DualModuleParallel::new_config(&initializer, config);
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

    pub fn dual_module_parallel_standard_syndrome<F>(d: usize, visualize_filename: String, syndrome_vertices: Vec<VertexIndex>
            , final_dual: Weight, partition_func: F)
            -> (DualModuleInterface, PrimalModuleSerial, DualModuleParallel) where F: Fn(&SolverInitializer, &mut DualModuleParallelConfig) {
        dual_module_parallel_basic_standard_syndrome_optional_viz(d, Some(visualize_filename), syndrome_vertices, final_dual, partition_func)
    }

    /// test a simple case
    #[test]
    fn dual_module_parallel_basic_1() {  // cargo test dual_module_parallel_basic_1 -- --nocapture
        let visualize_filename = format!("dual_module_parallel_basic_1.json");
        let syndrome_vertices = vec![39, 52, 63, 90, 100];
        dual_module_parallel_standard_syndrome(11, visualize_filename, syndrome_vertices, 9, |initializer, config| {
            println!("initializer: {initializer:?}");
            println!("config: {config:?}");
        });
    }

    /// split into 2, with no syndrome vertex on the interface
    #[test]
    fn dual_module_parallel_basic_2() {  // cargo test dual_module_parallel_basic_2 -- --nocapture
        let visualize_filename = format!("dual_module_parallel_basic_2.json");
        let syndrome_vertices = vec![39, 52, 63, 90, 100];
        dual_module_parallel_standard_syndrome(11, visualize_filename, syndrome_vertices, 9, |_initializer, config| {
            config.partitions = vec![
                (0..72).collect(),     // unit 0
                (84..121).collect(),   // unit 1
            ];
            config.interfaces = vec![
                (0, 1, (72..84).collect()),  // unit 2, by fusing 0 and 1
            ];
            println!("{config:?}");
        });
    }

}
