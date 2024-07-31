//! Minimum-Weight Perfect Matching Solver
//!
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//!

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;

use nonzero::nonzero as nz;
#[cfg(feature = "python_binding")]
use pyo3::prelude::*;

use crate::blossom_v;
use crate::complete_graph::*;
use crate::derivative::Derivative;
use crate::dual_module::*;

use super::dual_module::{DualModuleImpl, DualModuleInterfacePtr};
use super::dual_module_parallel::*;
use super::dual_module_serial::DualModuleSerial;
use super::pointers::*;
use super::primal_module::{PerfectMatching, PrimalModuleImpl, SubGraphBuilder, VisualizeSubgraph};
use super::primal_module_parallel::*;
use super::primal_module_serial::PrimalModuleSerialPtr;
use super::util::*;
use super::visualize::*;

/// a serial solver
#[derive(Derivative)]
#[derivative(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct LegacySolverSerial {
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    initializer: SolverInitializer,
    /// a serial implementation of the primal module
    #[derivative(Debug = "ignore")]
    primal_module: PrimalModuleSerialPtr,
    /// a serial implementation of the dual module
    #[derivative(Debug = "ignore")]
    dual_module: DualModuleSerial,
    /// the interface between the primal and dual module
    interface_ptr: DualModuleInterfacePtr,
    /// subgraph builder for easier integration with decoder
    subgraph_builder: SubGraphBuilder,
}

impl Clone for LegacySolverSerial {
    fn clone(&self) -> Self {
        Self::new(&self.initializer) // create independent instances of the solver
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

    pub fn solve_perfect_matching(
        &mut self,
        syndrome_pattern: &SyndromePattern,
        visualizer: Option<&mut Visualizer>,
    ) -> PerfectMatching {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
        self.primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module)
    }

    /// solve subgraph directly
    pub fn solve_subgraph(&mut self, syndrome_pattern: &SyndromePattern) -> Vec<EdgeIndex> {
        self.solve_subgraph_visualizer(syndrome_pattern, None)
    }

    pub fn solve_subgraph_visualizer(
        &mut self,
        syndrome_pattern: &SyndromePattern,
        visualizer: Option<&mut Visualizer>,
    ) -> Vec<EdgeIndex> {
        let perfect_matching = self.solve_perfect_matching(syndrome_pattern, visualizer);
        self.subgraph_builder.clear();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        self.subgraph_builder.get_subgraph()
    }

    /// solve the minimum weight perfect matching (legacy API, the same output as the blossom V library)
    pub fn solve_legacy(&mut self, syndrome_pattern: &SyndromePattern) -> Vec<VertexIndex> {
        self.solve_legacy_visualizer(syndrome_pattern, None)
    }

    pub fn solve_legacy_visualizer(
        &mut self,
        syndrome_pattern: &SyndromePattern,
        visualizer: Option<&mut Visualizer>,
    ) -> Vec<VertexIndex> {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        perfect_matching.legacy_get_mwpm_result(syndrome_pattern.defect_vertices.clone())
    }
}

// static functions, not recommended because it doesn't reuse the data structure of dual module
impl LegacySolverSerial {
    pub fn mwpm_solve(initializer: &SolverInitializer, syndrome_pattern: &SyndromePattern) -> Vec<VertexIndex> {
        Self::mwpm_solve_visualizer(initializer, syndrome_pattern, None)
    }

    pub fn mwpm_solve_visualizer(
        initializer: &SolverInitializer,
        syndrome_pattern: &SyndromePattern,
        visualizer: Option<&mut Visualizer>,
    ) -> Vec<VertexIndex> {
        let mut solver = Self::new(initializer);
        solver.solve_legacy_visualizer(syndrome_pattern, visualizer)
    }
}

pub trait PrimalDualSolver {
    fn clear(&mut self);
    fn reset_profiler(&mut self) {} // only if profiler records some information that needs to be cleared, e.g. vec![]
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>);
    fn solve(&mut self, syndrome_pattern: &SyndromePattern) {
        self.solve_visualizer(syndrome_pattern, None)
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching;
    fn perfect_matching(&mut self) -> PerfectMatching {
        self.perfect_matching_visualizer(None)
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex>;
    fn subgraph(&mut self) -> Vec<EdgeIndex> {
        self.subgraph_visualizer(None)
    }
    fn sum_dual_variables(&self) -> Weight;
    fn generate_profiler_report(&self) -> serde_json::Value;
    #[allow(clippy::unnecessary_cast)]
    fn stim_integration_predict_bit_packed_data(
        &mut self,
        in_file: String,
        out_file: String,
        edge_masks: &[usize],
        num_shots: usize,
        num_dets: usize,
        num_obs: usize,
    ) {
        let mut in_reader = std::io::BufReader::new(File::open(in_file).expect("in_file not found"));
        let mut out_writer = std::io::BufWriter::new(File::create(out_file).expect("out_file not found"));
        let num_det_bytes = (num_dets + 7) / 8; // ceil
        let mut dets_bit_packed = vec![0; num_det_bytes];
        assert!(num_obs <= 64, "too many observables");
        let prediction_bytes = (num_obs + 7) / 8; // ceil
        for _ in 0..num_shots {
            in_reader.read_exact(&mut dets_bit_packed).expect("read success");
            let mut defect_vertices = vec![];
            for (i, &byte) in dets_bit_packed.iter().enumerate() {
                if byte == 0 {
                    continue;
                }
                for j in 0..8 {
                    if byte & (1 << j) != 0 {
                        // little endian
                        defect_vertices.push((i * 8 + j) as VertexIndex);
                    }
                }
            }
            let syndrome_pattern = SyndromePattern::new_vertices(defect_vertices);
            self.solve(&syndrome_pattern);
            let subgraph = self.subgraph();
            let mut prediction = 0;
            for edge_index in subgraph {
                prediction ^= edge_masks[edge_index as usize];
            }
            for j in 0..prediction_bytes {
                let byte = ((prediction >> (j * 8)) & 0x0FF) as u8;
                out_writer.write_all(&[byte]).unwrap();
            }
            self.clear();
        }
    }
}

#[cfg(feature = "python_binding")]
macro_rules! bind_trait_primal_dual_solver {
    ($struct_name:ident) => {
        #[pymethods]
        impl $struct_name {
            #[pyo3(name = "clear")]
            fn trait_clear(&mut self) {
                self.clear()
            }
            #[pyo3(name = "solve_visualizer")]
            fn trait_solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
                self.solve_visualizer(syndrome_pattern, visualizer)
            }
            #[pyo3(name = "solve")] // in Python, `solve` and `solve_visualizer` is the same because it can take optional parameter
            fn trait_solve(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
                self.solve_visualizer(syndrome_pattern, visualizer)
            }
            #[pyo3(name = "perfect_matching_visualizer")]
            fn trait_perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
                self.perfect_matching_visualizer(visualizer)
            }
            #[pyo3(name = "perfect_matching")] // in Python, `perfect_matching` and `perfect_matching_visualizer` is the same because it can take optional parameter
            fn trait_perfect_matching(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
                self.perfect_matching_visualizer(visualizer)
            }
            #[pyo3(name = "subgraph_visualizer")]
            fn trait_subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
                self.subgraph_visualizer(visualizer)
            }
            #[pyo3(name = "subgraph")] // in Python, `subgraph` and `subgraph_visualizer` is the same because it can take optional parameter
            fn trait_subgraph(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
                self.subgraph_visualizer(visualizer)
            }
            #[pyo3(name = "sum_dual_variables")]
            fn trait_sum_dual_variables(&self) -> Weight {
                self.sum_dual_variables()
            }
            #[pyo3(name = "generate_profiler_report")]
            fn trait_generate_profiler_report(&self) -> PyObject {
                json_to_pyobject(self.generate_profiler_report())
            }
            #[pyo3(name = "stim_integration_predict_bit_packed_data")]
            fn trait_stim_integration_predict_bit_packed_data(
                &mut self,
                in_file: String,
                out_file: String,
                edge_masks: Vec<usize>,
                num_shots: usize,
                num_dets: usize,
                num_obs: usize,
            ) {
                self.stim_integration_predict_bit_packed_data(in_file, out_file, &edge_masks, num_shots, num_dets, num_obs)
            }
        }
    };
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SolverSerial {
    pub dual_module: DualModuleSerial,
    pub primal_module: PrimalModuleSerialPtr,
    pub interface_ptr: DualModuleInterfacePtr,
    pub subgraph_builder: SubGraphBuilder,
}

bind_trait_fusion_visualizer!(SolverSerial);
impl FusionVisualizer for SolverSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
        value
    }
}

#[cfg(feature = "python_binding")]
bind_trait_primal_dual_solver! {SolverSerial}

#[cfg(feature = "python_binding")]
#[pymethods]
impl SolverSerial {
    #[new]
    #[pyo3(signature = (initializer, *, max_tree_size = None))]
    pub fn new_python(initializer: &SolverInitializer, max_tree_size: Option<usize>) -> Self {
        let mut solver = Self::new(initializer);
        if let Some(max_tree_size) = max_tree_size {
            solver.primal_module.write().max_tree_size = max_tree_size;
        }
        solver
    }
}

impl SolverSerial {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleSerial::new_empty(initializer),
            primal_module: PrimalModuleSerialPtr::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverSerial {
    fn clear(&mut self) {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            self.subgraph_builder.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![&self.interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &self.interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SolverDualParallel {
    pub dual_module: DualModuleParallel<DualModuleSerial>,
    pub primal_module: PrimalModuleSerialPtr,
    pub interface_ptr: DualModuleInterfacePtr,
    pub subgraph_builder: SubGraphBuilder,
}

bind_trait_fusion_visualizer!(SolverDualParallel);
impl FusionVisualizer for SolverDualParallel {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        snapshot_combine_values(&mut value, self.interface_ptr.snapshot(abbrev), abbrev);
        value
    }
}

#[cfg(feature = "python_binding")]
bind_trait_primal_dual_solver! {SolverDualParallel}

#[cfg(feature = "python_binding")]
#[pymethods]
impl SolverDualParallel {
    #[new]
    pub fn new_python(
        initializer: &SolverInitializer,
        partition_info: &PartitionInfo,
        primal_dual_config: PyObject,
    ) -> Self {
        let primal_dual_config = pyobject_to_json(primal_dual_config);
        Self::new(initializer, partition_info, primal_dual_config)
    }
}

impl SolverDualParallel {
    pub fn new(
        initializer: &SolverInitializer,
        partition_info: &PartitionInfo,
        primal_dual_config: serde_json::Value,
    ) -> Self {
        let config: DualModuleParallelConfig = serde_json::from_value(primal_dual_config).unwrap();
        Self {
            dual_module: DualModuleParallel::new_config(initializer, partition_info, config),
            primal_module: PrimalModuleSerialPtr::new_empty(initializer),
            interface_ptr: DualModuleInterfacePtr::new_empty(),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverDualParallel {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
        self.interface_ptr.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            assert!(
                syndrome_pattern.dynamic_weights.is_empty(),
                "erasures and dynamic_weights cannot be provided at the same time"
            );
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        if !syndrome_pattern.dynamic_weights.is_empty() {
            self.subgraph_builder.load_dynamic_weights(&syndrome_pattern.dynamic_weights);
        }
        self.dual_module.static_fuse_all();
        self.primal_module
            .solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self
            .primal_module
            .perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![&self.interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &self.interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        self.interface_ptr.read_recursive().sum_dual_variables
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({
            "dual": self.dual_module.generate_profiler_report(),
            "primal": self.primal_module.generate_profiler_report(),
        })
    }
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SolverParallel {
    pub dual_module: DualModuleParallel<DualModuleSerial>,
    pub primal_module: PrimalModuleParallel,
    pub subgraph_builder: SubGraphBuilder,
}

bind_trait_fusion_visualizer!(SolverParallel);
impl FusionVisualizer for SolverParallel {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}

#[cfg(feature = "python_binding")]
bind_trait_primal_dual_solver! {SolverParallel}

#[cfg(feature = "python_binding")]
#[pymethods]
impl SolverParallel {
    #[new]
    pub fn new_python(
        initializer: &SolverInitializer,
        partition_info: &PartitionInfo,
        primal_dual_config: PyObject,
    ) -> Self {
        let primal_dual_config = pyobject_to_json(primal_dual_config);
        Self::new(initializer, partition_info, primal_dual_config)
    }

    #[pyo3(name = "defect_perfect_matching")]
    pub fn defect_perfect_matching(&mut self) -> Vec<(VertexIndex, VertexIndex)> {
        let perfect_matching = self.perfect_matching_visualizer(None);
        let mut defect_matching = vec![];
        // iterate over peer matching
        for (a, b) in perfect_matching.peer_matchings.iter() {
            let node_a = a.read_recursive();
            let vertex_a = if let DualNodeClass::DefectVertex { defect_index } = &node_a.class {
                *defect_index
            } else {
                unreachable!("can only be syndrome")
            };
            let node_b = b.read_recursive();
            let vertex_b = if let DualNodeClass::DefectVertex { defect_index } = &node_b.class {
                *defect_index
            } else {
                unreachable!("can only be syndrome")
            };
            defect_matching.push((vertex_a, vertex_b));
        }
        // iterate over virtual matching
        for (a, virtual_vertex) in perfect_matching.virtual_matchings.iter() {
            let node_a = a.read_recursive();
            let vertex_a = if let DualNodeClass::DefectVertex { defect_index } = &node_a.class {
                *defect_index
            } else {
                unreachable!("can only be syndrome")
            };
            defect_matching.push((vertex_a, *virtual_vertex));
        }
        defect_matching
    }
}

impl SolverParallel {
    pub fn new(
        initializer: &SolverInitializer,
        partition_info: &PartitionInfo,
        mut primal_dual_config: serde_json::Value,
    ) -> Self {
        let primal_dual_config = primal_dual_config.as_object_mut().expect("config must be JSON object");
        let mut dual_config = DualModuleParallelConfig::default();
        let mut primal_config = PrimalModuleParallelConfig::default();
        if let Some(value) = primal_dual_config.remove("dual") {
            dual_config = serde_json::from_value(value).unwrap();
        }
        if let Some(value) = primal_dual_config.remove("primal") {
            primal_config = serde_json::from_value(value).unwrap();
        }
        if !primal_dual_config.is_empty() {
            panic!(
                "unknown primal_dual_config keys: {:?}",
                primal_dual_config.keys().collect::<Vec<&String>>()
            );
        }
        Self {
            dual_module: DualModuleParallel::new_config(initializer, partition_info, dual_config),
            primal_module: PrimalModuleParallel::new_config(initializer, partition_info, primal_config),
            subgraph_builder: SubGraphBuilder::new(initializer),
        }
    }
}

impl PrimalDualSolver for SolverParallel {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        if !syndrome_pattern.erasures.is_empty() {
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        self.primal_module
            .parallel_solve_visualizer(syndrome_pattern, &self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let useless_interface_ptr = DualModuleInterfacePtr::new_empty(); // don't actually use it
        let perfect_matching = self
            .primal_module
            .perfect_matching(&useless_interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            let last_interface_ptr = &self.primal_module.units.last().unwrap().read_recursive().interface_ptr;
            visualizer
                .snapshot_combined(
                    "perfect matching".to_string(),
                    vec![last_interface_ptr, &self.dual_module, &perfect_matching],
                )
                .unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            let last_interface_ptr = &self.primal_module.units.last().unwrap().read_recursive().interface_ptr;
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        last_interface_ptr,
                        &self.dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight {
        let last_unit = self.primal_module.units.last().unwrap().write(); // use the interface in the last unit
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

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SolverErrorPatternLogger {
    pub file: BufWriter<File>,
}

#[cfg(feature = "python_binding")]
bind_trait_primal_dual_solver! {SolverErrorPatternLogger}

impl SolverErrorPatternLogger {
    pub fn new(initializer: &SolverInitializer, positions: &Vec<VisualizePosition>, mut config: serde_json::Value) -> Self {
        let mut filename = "tmp/syndrome_patterns.txt".to_string();
        let config = config.as_object_mut().expect("config must be JSON object");
        if let Some(value) = config.remove("filename") {
            filename = value.as_str().expect("filename string").to_string();
        }
        if !config.is_empty() {
            panic!("unknown config keys: {:?}", config.keys().collect::<Vec<&String>>());
        }
        let file = File::create(filename).unwrap();
        let mut file = BufWriter::new(file);
        file.write_all(b"Syndrome Pattern v1.0   <initializer> <positions> <syndrome_pattern>*\n")
            .unwrap();
        serde_json::to_writer(&mut file, &initializer).unwrap(); // large object write to file directly
        file.write_all(b"\n").unwrap();
        serde_json::to_writer(&mut file, &positions).unwrap();
        file.write_all(b"\n").unwrap();
        Self { file }
    }
}

impl PrimalDualSolver for SolverErrorPatternLogger {
    fn clear(&mut self) {}
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, _visualizer: Option<&mut Visualizer>) {
        self.file
            .write_all(
                serde_json::to_string(&serde_json::json!(syndrome_pattern))
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        self.file.write_all(b"\n").unwrap();
    }
    fn perfect_matching_visualizer(&mut self, _visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        panic!("error pattern logger do not actually solve the problem, please use Verifier::None by `--verifier none`")
    }
    fn subgraph_visualizer(&mut self, _visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        // panic!("error pattern logger do not actually solve the problem, please use Verifier::None by `--verifier none`")
        vec![]
    }
    fn sum_dual_variables(&self) -> Weight {
        panic!("error pattern logger do not actually solve the problem")
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}

/// an exact solver calling blossom V library for benchmarking comparison
#[derive(Clone)]
pub struct SolverBlossomV {
    pub initializer: SolverInitializer,
    pub prebuilt_complete_graph: PrebuiltCompleteGraph,
    pub subgraph_builder: SubGraphBuilder,
    pub matched_pairs: Vec<(VertexIndex, VertexIndex)>,
}

impl SolverBlossomV {
    pub fn new(initializer: &SolverInitializer) -> Self {
        Self {
            initializer: initializer.clone(),
            prebuilt_complete_graph: PrebuiltCompleteGraph::new_threaded(initializer, 0),
            subgraph_builder: SubGraphBuilder::new(initializer),
            matched_pairs: vec![],
        }
    }
}

impl PrimalDualSolver for SolverBlossomV {
    fn clear(&mut self) {
        self.matched_pairs.clear();
        self.subgraph_builder.clear();
    }
    fn solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        assert!(visualizer.is_none(), "not supported");
        assert!(syndrome_pattern.erasures.is_empty(), "doesn't support erasure for now");
        let defect_vertices = &syndrome_pattern.defect_vertices;
        if defect_vertices.is_empty() {
            return;
        }
        let mut mapping_to_defect_vertices: BTreeMap<VertexIndex, usize> = BTreeMap::new();
        for (i, &defect_vertex) in defect_vertices.iter().enumerate() {
            mapping_to_defect_vertices.insert(defect_vertex, i);
        }
        // for each real vertex, add a corresponding virtual vertex to be matched
        let defect_num = defect_vertices.len();
        let legacy_vertex_num = defect_num * 2;
        let mut legacy_weighted_edges = Vec::<(usize, usize, u32)>::new();
        for i in 0..defect_num - 1 {
            for j in i + 1..defect_num {
                if let Some(weight) = self
                    .prebuilt_complete_graph
                    .get_edge_weight(defect_vertices[i], defect_vertices[j])
                {
                    legacy_weighted_edges.push((i, j, weight as u32));
                }
            }
        }
        for (i, &defect_vertex) in defect_vertices.iter().enumerate() {
            if let Some((_, weight)) = self.prebuilt_complete_graph.get_boundary_weight(defect_vertex) {
                // connect this real vertex to it's corresponding virtual vertex
                legacy_weighted_edges.push((i, i + defect_num, weight as u32));
            }
        }
        for i in 0..defect_num - 1 {
            for j in i + 1..defect_num {
                // virtual boundaries are always fully connected with weight 0
                legacy_weighted_edges.push((i + defect_num, j + defect_num, 0));
            }
        }
        // run blossom V to get matchings
        // println!("[debug] legacy_vertex_num: {:?}", legacy_vertex_num);
        // println!("[debug] legacy_weighted_edges: {:?}", legacy_weighted_edges);
        let matchings = blossom_v::safe_minimum_weight_perfect_matching(legacy_vertex_num, &legacy_weighted_edges);
        let mut matched_pairs = Vec::new();
        for i in 0..defect_num {
            let j = matchings[i];
            if j < defect_num {
                // match to a real vertex
                if i < j {
                    // avoid duplicate matched pair
                    matched_pairs.push((defect_vertices[i], defect_vertices[j]));
                }
            } else {
                assert_eq!(
                    j,
                    i + defect_num,
                    "if not matched to another real vertex, it must match to it's corresponding virtual vertex"
                );
                matched_pairs.push((
                    defect_vertices[i],
                    self.prebuilt_complete_graph
                        .get_boundary_weight(defect_vertices[i])
                        .expect("boundary must exist if match to virtual vertex")
                        .0,
                ));
            }
        }
        self.matched_pairs = matched_pairs;
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        assert!(visualizer.is_none(), "not supported");
        let virtual_vertices: BTreeSet<VertexIndex> = self.initializer.virtual_vertices.iter().cloned().collect();
        let mut perfect_matching = PerfectMatching::new();
        let mut counter = 0;
        let interface_ptr = DualModuleInterfacePtr::new_empty();
        let mut create_dual_node = |vertex_index: VertexIndex| {
            counter += 1;
            DualNodePtr::new_value(DualNode {
                index: counter,
                class: DualNodeClass::DefectVertex {
                    defect_index: vertex_index,
                },
                grow_state: DualNodeGrowState::Grow,
                parent_blossom: None,
                dual_variable_cache: (0, 0),
                belonging: interface_ptr.downgrade(),
                defect_size: nz!(1usize),
            })
        };
        for &(vertex_1, vertex_2) in self.matched_pairs.iter() {
            assert!(!virtual_vertices.contains(&vertex_1)); // 1 is not virtual
            if virtual_vertices.contains(&vertex_2) {
                perfect_matching
                    .virtual_matchings
                    .push((create_dual_node(vertex_1), vertex_2));
            } else {
                perfect_matching
                    .peer_matchings
                    .push((create_dual_node(vertex_1), create_dual_node(vertex_2)));
            }
            self.subgraph_builder.add_matching(vertex_1, vertex_2);
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        assert!(visualizer.is_none(), "not supported");
        self.subgraph_builder.clear();
        for &(vertex_1, vertex_2) in self.matched_pairs.iter() {
            self.subgraph_builder.add_matching(vertex_1, vertex_2);
        }
        self.subgraph_builder.subgraph.iter().copied().collect()
    }
    #[allow(clippy::unnecessary_cast)]
    fn sum_dual_variables(&self) -> Weight {
        let mut subgraph_builder = self.subgraph_builder.clone();
        subgraph_builder.clear();
        for &(vertex_1, vertex_2) in self.matched_pairs.iter() {
            subgraph_builder.add_matching(vertex_1, vertex_2);
        }
        let subgraph: Vec<EdgeIndex> = subgraph_builder.subgraph.iter().copied().collect();
        let mut weight = 0;
        for &edge_index in subgraph.iter() {
            weight += self.initializer.weighted_edges[edge_index as usize].2;
        }
        weight
    }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}

#[cfg(feature = "python_binding")]
#[pyfunction]
pub(crate) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<LegacySolverSerial>()?;
    m.add_class::<SolverSerial>()?;
    m.add_class::<SolverDualParallel>()?;
    m.add_class::<SolverParallel>()?;
    m.add_class::<SolverErrorPatternLogger>()?;
    Ok(())
}
