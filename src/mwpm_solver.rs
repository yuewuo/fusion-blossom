//! Minimum-Weight Perfect Matching Solver
//! 
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//! 

use super::util::*;
use super::dual_module::{DualModuleInterfacePtr, DualModuleImpl};
use super::primal_module::{PrimalModuleImpl, SubGraphBuilder, PerfectMatching};
use super::dual_module_serial::DualModuleSerial;
use super::primal_module_serial::PrimalModuleSerialPtr;
use super::dual_module_parallel::*;
use super::example::*;
use super::primal_module_parallel::*;
use super::visualize::*;
use crate::derivative::Derivative;
use std::fs::File;
use std::io::prelude::*;
use std::sync::Arc;


/// a serial solver
#[derive(Derivative)]
#[derivative(Debug)]
pub struct LegacySolverSerial {
    initializer: SolverInitializer,
    /// a serial implementation of the primal module
    #[derivative(Debug="ignore")]
    primal_module: PrimalModuleSerialPtr,
    /// a serial implementation of the dual module
    #[derivative(Debug="ignore")]
    dual_module: DualModuleSerial,
    /// the interface between the primal and dual module
    interface_ptr: DualModuleInterfacePtr,
    /// subgraph builder for easier integration with decoder
    subgraph_builder: SubGraphBuilder,
}

impl Clone for LegacySolverSerial {
    fn clone(&self) -> Self {
        Self::new(&self.initializer)  // create independent instances of the solver
    }
}

impl LegacySolverSerial {

    /// create a new decoder
    pub fn new(initializer: &SolverInitializer) -> Self {
        let dual_module = DualModuleSerial::new_empty(initializer);
        let primal_module = PrimalModuleSerialPtr::new_empty(initializer);
        let interface_ptr = DualModuleInterfacePtr::new_empty();
        let subgraph_builder = SubGraphBuilder::new(initializer);
        Self {
            initializer: initializer.clone(),
            primal_module,
            dual_module,
            interface_ptr,
            subgraph_builder,
        }
    }

    pub fn solve_perfect_matching(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
        self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module)
    }

    /// solve subgraph directly
    pub fn solve_subgraph(&mut self, syndrome_pattern: &SyndromePattern) -> Vec<EdgeIndex> {
        self.solve_subgraph_visualizer(syndrome_pattern, None)
    }

    pub fn solve_subgraph_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.solve_perfect_matching(syndrome_pattern, visualizer);
        self.subgraph_builder.clear();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        self.subgraph_builder.get_subgraph()
    }

    /// solve the minimum weight perfect matching (legacy API, the same output as the blossom V library)
    pub fn solve_legacy(&mut self, syndrome_pattern: &SyndromePattern) -> Vec<usize> {
        self.solve_legacy_visualizer(syndrome_pattern, None)
    }

    pub fn solve_legacy_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
        let perfect_matching = self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module);
        perfect_matching.legacy_get_mwpm_result(&syndrome_pattern.syndrome_vertices)
    }

}

// static functions, not recommended because it doesn't reuse the data structure of dual module
impl LegacySolverSerial {

    pub fn mwpm_solve(initializer: &SolverInitializer, syndrome_pattern: &SyndromePattern) -> Vec<usize> {
        Self::mwpm_solve_visualizer(initializer, syndrome_pattern, None)
    }

    pub fn mwpm_solve_visualizer(initializer: &SolverInitializer, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        let mut solver = Self::new(initializer);
        solver.solve_legacy_visualizer(syndrome_pattern, visualizer)
    }

}

impl FusionVisualizer for SolverSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}

pub trait PrimalDualSolver {
    fn clear(&mut self);
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>);
    fn perfect_matching(&mut self) -> PerfectMatching;
    fn sum_dual_variables(&self) -> Weight;
    fn generate_profiler_report(&self) -> serde_json::Value;
}

pub struct SolverSerial {
    dual_module: DualModuleSerial,
    primal_module: PrimalModuleSerialPtr,
    interface_ptr: DualModuleInterfacePtr,
}

impl SolverSerial {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleSerial::new_empty(initializer),
            primal_module: PrimalModuleSerialPtr::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
        }
    }
}

impl PrimalDualSolver for SolverSerial {
    fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching(&mut self) -> PerfectMatching { self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module) }
    fn sum_dual_variables(&self) -> Weight { self.interface_ptr.read_recursive().sum_dual_variables }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverDualParallel {
    dual_module: DualModuleParallel<DualModuleSerial>,
    primal_module: PrimalModuleSerialPtr,
    interface_ptr: DualModuleInterfacePtr,
}

impl SolverDualParallel {
    pub fn new(initializer: &SolverInitializer, partition_info: &Arc<PartitionInfo>) -> Self {
        let config = DualModuleParallelConfig::default();
        Self {
            dual_module: DualModuleParallel::new_config(initializer, Arc::clone(partition_info), config),
            primal_module: PrimalModuleSerialPtr::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
        }
    }
}

impl PrimalDualSolver for SolverDualParallel {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
        self.interface_ptr.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        self.dual_module.static_fuse_all();
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching(&mut self) -> PerfectMatching { self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module) }
    fn sum_dual_variables(&self) -> Weight { self.interface_ptr.read_recursive().sum_dual_variables }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverParallel {
    dual_module: DualModuleParallel<DualModuleSerial>,
    primal_module: PrimalModuleParallel,
}

impl SolverParallel {
    pub fn new(initializer: &SolverInitializer, partition_info: &Arc<PartitionInfo>) -> Self {
        let dual_config = DualModuleParallelConfig::default();
        let primal_config = PrimalModuleParallelConfig::default();
        Self {
            dual_module: DualModuleParallel::new_config(initializer, Arc::clone(partition_info), dual_config),
            primal_module: PrimalModuleParallel::new_config(initializer, Arc::clone(partition_info), primal_config),
        }
    }
}

impl PrimalDualSolver for SolverParallel {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        self.primal_module.parallel_solve_visualizer(syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching(&mut self) -> PerfectMatching {
        let useless_interface_ptr = DualModuleInterfacePtr::new_empty();  // don't actually use it
        self.primal_module.perfect_matching(&useless_interface_ptr, &mut self.dual_module)
    }
    fn sum_dual_variables(&self) -> Weight {
        let last_unit = self.primal_module.units.last().unwrap().write();  // use the interface in the last unit
        let sum_dual_variables = last_unit.interface_ptr.read_recursive().sum_dual_variables;
        sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

pub struct SolverErrorPatternLogger {
    file: File,
}

impl SolverErrorPatternLogger {
    pub fn new(initializer: &SolverInitializer, code: &dyn ExampleCode, mut config: serde_json::Value) -> Self {
        let mut filename = "tmp/syndrome_patterns.txt".to_string();
        let config = config.as_object_mut().expect("config must be JSON object");
        if let Some(value) = config.remove("filename") {
            filename = value.as_str().expect("filename string").to_string();
        }
        if !config.is_empty() { panic!("unknown config keys: {:?}", config.keys().collect::<Vec<&String>>()); }
        let mut file = File::create(filename).unwrap();
        file.write_all(b"Syndrome Pattern v1.0   <initializer> <positions> <syndrome_pattern>*\n").unwrap();
        serde_json::to_writer(&file, &initializer).unwrap();  // large object write to file directly
        file.write_all(b"\n").unwrap();
        serde_json::to_writer(&file, &code.get_positions()).unwrap();
        file.write_all(b"\n").unwrap();
        Self {
            file,
        }
    }
}

impl PrimalDualSolver for SolverErrorPatternLogger {
    fn clear(&mut self) { }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, _visualizer: Option<&mut Visualizer>) {
        self.file.write_all(serde_json::to_string(&serde_json::json!(syndrome_pattern)).unwrap().as_bytes()).unwrap();
        self.file.write_all(b"\n").unwrap();
    }
    fn perfect_matching(&mut self) -> PerfectMatching {
        panic!("error pattern logger do not actually solve the problem, please use Verifier::None by `--verifier none`")
    }
    fn sum_dual_variables(&self) -> Weight { panic!("error pattern logger do not actually solve the problem") }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}
