//! Minimum-Weight Perfect Matching Solver
//! 
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//! 

use super::util::*;
use super::dual_module::{DualModuleInterfacePtr, DualModuleImpl};
use super::primal_module::{PrimalModuleImpl, SubGraphBuilder, PerfectMatching, VisualizeSubgraph};
use super::dual_module_serial::DualModuleSerial;
use super::primal_module_serial::PrimalModuleSerialPtr;
use super::dual_module_parallel::*;
use super::example_codes::*;
use super::primal_module_parallel::*;
use super::visualize::*;
use crate::derivative::Derivative;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufWriter;
use super::pointers::*;
#[cfg(feature="python_binding")]
use pyo3::prelude::*;


/// a serial solver
#[derive(Derivative)]
#[derivative(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct LegacySolverSerial {
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
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
    pub fn solve_legacy(&mut self, syndrome_pattern: &SyndromePattern) -> Vec<VertexIndex> {
        self.solve_legacy_visualizer(syndrome_pattern, None)
    }

    pub fn solve_legacy_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) -> Vec<VertexIndex> {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface_ptr.clear();
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
        let perfect_matching = self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module);
        perfect_matching.legacy_get_mwpm_result(syndrome_pattern.defect_vertices.clone())
    }

}

// static functions, not recommended because it doesn't reuse the data structure of dual module
impl LegacySolverSerial {

    pub fn mwpm_solve(initializer: &SolverInitializer, syndrome_pattern: &SyndromePattern) -> Vec<VertexIndex> {
        Self::mwpm_solve_visualizer(initializer, syndrome_pattern, None)
    }

    pub fn mwpm_solve_visualizer(initializer: &SolverInitializer, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) -> Vec<VertexIndex> {
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
    fn solve(&mut self, syndrome_pattern: &SyndromePattern) { self.solve_visualizer(syndrome_pattern, None) }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching;
    fn perfect_matching(&mut self) -> PerfectMatching { self.perfect_matching_visualizer(None) }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex>;
    fn subgraph(&mut self) -> Vec<EdgeIndex> { self.subgraph_visualizer(None) }
    fn sum_dual_variables(&self) -> Weight;
    fn generate_profiler_report(&self) -> serde_json::Value;
    fn stim_integration_predict_bit_packed_data(&mut self, in_file: String, out_file: String, edge_masks: &[usize], num_shots: usize
            , num_dets: usize, num_obs: usize) {
        let mut in_reader = std::io::BufReader::new(File::open(&in_file).expect("in_file not found"));
        let mut out_writer = std::io::BufWriter::new(File::create(&out_file).expect("out_file not found"));
        let num_det_bytes = (num_dets + 7) / 8;  // ceil
        let mut dets_bit_packed = vec![0; num_det_bytes];
        assert!(num_obs <= 64, "too many observables");
        let prediction_bytes = (num_obs + 7) / 8;  // ceil
        for _ in 0..num_shots {
            in_reader.read_exact(&mut dets_bit_packed).expect("read success");
            let mut defect_vertices = vec![];
            for (i, &byte) in dets_bit_packed.iter().enumerate() {
                if byte == 0 {
                    continue
                }
                for j in 0..8 {
                    if byte & (1 << j) != 0 {  // little endian
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

#[cfg(feature="python_binding")]
macro_rules! bind_trait_primal_dual_solver {
    ($struct_name:ident) => {
        #[pymethods]
        impl $struct_name {
            #[pyo3(name = "clear")]
            fn trait_clear(&mut self) { self.clear() }
            #[pyo3(name = "solve_visualizer")]
            fn trait_solve_visualizer(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
                self.solve_visualizer(syndrome_pattern, visualizer)
            }
            #[pyo3(name = "solve")]  // in Python, `solve` and `solve_visualizer` is the same because it can take optional parameter
            fn trait_solve(&mut self, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
                self.solve_visualizer(syndrome_pattern, visualizer)
            }
            #[pyo3(name = "perfect_matching_visualizer")]
            fn trait_perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
                self.perfect_matching_visualizer(visualizer)
            }
            #[pyo3(name = "perfect_matching")]  // in Python, `perfect_matching` and `perfect_matching_visualizer` is the same because it can take optional parameter
            fn trait_perfect_matching(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
                self.perfect_matching_visualizer(visualizer)
            }
            #[pyo3(name = "subgraph_visualizer")]
            fn trait_subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
                self.subgraph_visualizer(visualizer)
            }
            #[pyo3(name = "subgraph")]  // in Python, `subgraph` and `subgraph_visualizer` is the same because it can take optional parameter
            fn trait_subgraph(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
                self.subgraph_visualizer(visualizer)
            }
            #[pyo3(name = "sum_dual_variables")]
            fn trait_sum_dual_variables(&self) -> Weight { self.sum_dual_variables() }
            #[pyo3(name = "generate_profiler_report")]
            fn trait_generate_profiler_report(&self) -> PyObject { json_to_pyobject(self.generate_profiler_report()) }
            #[pyo3(name = "stim_integration_predict_bit_packed_data")]
            fn trait_stim_integration_predict_bit_packed_data(&mut self, in_file: String, out_file: String, edge_masks: Vec<usize>, num_shots: usize
                    , num_dets: usize, num_obs: usize) {
                self.stim_integration_predict_bit_packed_data(in_file, out_file, &edge_masks, num_shots, num_dets, num_obs)
            }
        }
    };
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SolverSerial {
    dual_module: DualModuleSerial,
    primal_module: PrimalModuleSerialPtr,
    interface_ptr: DualModuleInterfacePtr,
    subgraph_builder: SubGraphBuilder,
}

#[cfg(feature="python_binding")]
bind_trait_primal_dual_solver!{SolverSerial}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl SolverSerial {
    #[cfg_attr(feature = "python_binding", new)]
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
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer.snapshot_combined("perfect matching".to_string(), vec![&self.interface_ptr, &self.dual_module, &perfect_matching]).unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![&self.interface_ptr, &self.dual_module
                , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight { self.interface_ptr.read_recursive().sum_dual_variables }
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
    dual_module: DualModuleParallel<DualModuleSerial>,
    primal_module: PrimalModuleSerialPtr,
    interface_ptr: DualModuleInterfacePtr,
    subgraph_builder: SubGraphBuilder,
}

#[cfg(feature="python_binding")]
bind_trait_primal_dual_solver!{SolverDualParallel}

#[cfg(feature = "python_binding")]
#[pymethods]
impl SolverDualParallel {
    #[new]
    pub fn new_python(initializer: &SolverInitializer, partition_info: &PartitionInfo, primal_dual_config: PyObject) -> Self {
        let primal_dual_config = pyobject_to_json(primal_dual_config);
        Self::new(initializer, partition_info, primal_dual_config)
    }
}

impl SolverDualParallel {
    pub fn new(initializer: &SolverInitializer, partition_info: &PartitionInfo, primal_dual_config: serde_json::Value) -> Self {
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
            self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        }
        self.dual_module.static_fuse_all();
        self.primal_module.solve_visualizer(&self.interface_ptr, syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let perfect_matching = self.primal_module.perfect_matching(&self.interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            visualizer.snapshot_combined("perfect matching".to_string(), vec![&self.interface_ptr, &self.dual_module, &perfect_matching]).unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![&self.interface_ptr, &self.dual_module
                , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
        }
        subgraph
    }
    fn sum_dual_variables(&self) -> Weight { self.interface_ptr.read_recursive().sum_dual_variables }
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
    dual_module: DualModuleParallel<DualModuleSerial>,
    primal_module: PrimalModuleParallel,
    subgraph_builder: SubGraphBuilder,
}

#[cfg(feature="python_binding")]
bind_trait_primal_dual_solver!{SolverParallel}

#[cfg(feature = "python_binding")]
#[pymethods]
impl SolverParallel {
    #[new]
    pub fn new_python(initializer: &SolverInitializer, partition_info: &PartitionInfo, primal_dual_config: PyObject) -> Self {
        let primal_dual_config = pyobject_to_json(primal_dual_config);
        Self::new(initializer, partition_info, primal_dual_config)
    }
}

impl SolverParallel {
    pub fn new(initializer: &SolverInitializer, partition_info: &PartitionInfo, mut primal_dual_config: serde_json::Value) -> Self {
        let primal_dual_config = primal_dual_config.as_object_mut().expect("config must be JSON object");
        let mut dual_config = DualModuleParallelConfig::default();
        let mut primal_config = PrimalModuleParallelConfig::default();
        if let Some(value) = primal_dual_config.remove("dual") {
            dual_config = serde_json::from_value(value).unwrap();
        }
        if let Some(value) = primal_dual_config.remove("primal") {
            primal_config = serde_json::from_value(value).unwrap();
        }
        if !primal_dual_config.is_empty() { panic!("unknown primal_dual_config keys: {:?}", primal_dual_config.keys().collect::<Vec<&String>>()); }
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
        self.primal_module.parallel_solve_visualizer(syndrome_pattern, &mut self.dual_module, visualizer);
    }
    fn perfect_matching_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        let useless_interface_ptr = DualModuleInterfacePtr::new_empty();  // don't actually use it
        let perfect_matching = self.primal_module.perfect_matching(&useless_interface_ptr, &mut self.dual_module);
        if let Some(visualizer) = visualizer {
            let last_interface_ptr = &self.primal_module.units.last().unwrap().read_recursive().interface_ptr;
            visualizer.snapshot_combined("perfect matching".to_string(), vec![last_interface_ptr, &self.dual_module, &perfect_matching]).unwrap();
        }
        perfect_matching
    }
    fn subgraph_visualizer(&mut self, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.perfect_matching();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = self.subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer {
            let last_interface_ptr = &self.primal_module.units.last().unwrap().read_recursive().interface_ptr;
            visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![last_interface_ptr, &self.dual_module
                , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
        }
        subgraph
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

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SolverErrorPatternLogger {
    file: BufWriter<File>,
}

#[cfg(feature="python_binding")]
bind_trait_primal_dual_solver!{SolverErrorPatternLogger}

impl SolverErrorPatternLogger {
    pub fn new(initializer: &SolverInitializer, code: &dyn ExampleCode, mut config: serde_json::Value) -> Self {
        let mut filename = "tmp/syndrome_patterns.txt".to_string();
        let config = config.as_object_mut().expect("config must be JSON object");
        if let Some(value) = config.remove("filename") {
            filename = value.as_str().expect("filename string").to_string();
        }
        if !config.is_empty() { panic!("unknown config keys: {:?}", config.keys().collect::<Vec<&String>>()); }
        let file = File::create(filename).unwrap();
        let mut file = BufWriter::new(file);
        file.write_all(b"Syndrome Pattern v1.0   <initializer> <positions> <syndrome_pattern>*\n").unwrap();
        serde_json::to_writer(&mut file, &initializer).unwrap();  // large object write to file directly
        file.write_all(b"\n").unwrap();
        serde_json::to_writer(&mut file, &code.get_positions()).unwrap();
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
    fn perfect_matching_visualizer(&mut self, _visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        panic!("error pattern logger do not actually solve the problem, please use Verifier::None by `--verifier none`")
    }
    fn subgraph_visualizer(&mut self, _visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        panic!("error pattern logger do not actually solve the problem, please use Verifier::None by `--verifier none`")
    }
    fn sum_dual_variables(&self) -> Weight { panic!("error pattern logger do not actually solve the problem") }
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}

#[cfg(feature="python_binding")]
#[pyfunction]
pub(crate) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<LegacySolverSerial>()?;
    m.add_class::<SolverSerial>()?;
    m.add_class::<SolverDualParallel>()?;
    m.add_class::<SolverParallel>()?;
    m.add_class::<SolverErrorPatternLogger>()?;
    Ok(())
}
