//! Minimum-Weight Perfect Matching Solver
//! 
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//! 

use super::util::*;
use super::dual_module::{DualModuleInterface, DualModuleImpl};
use super::primal_module::{PrimalModuleImpl, SubGraphBuilder, PerfectMatching};
use super::dual_module_serial::DualModuleSerial;
use super::primal_module_serial::PrimalModuleSerial;
use super::visualize::*;
use crate::derivative::Derivative;


/// a serial solver
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SolverSerial {
    initializer: SolverInitializer,
    /// a serial implementation of the primal module
    #[derivative(Debug="ignore")]
    primal_module: PrimalModuleSerial,
    /// a serial implementation of the dual module
    #[derivative(Debug="ignore")]
    dual_module: DualModuleSerial,
    /// the interface between the primal and dual module
    interface: DualModuleInterface,
    /// subgraph builder for easier integration with decoder
    subgraph_builder: SubGraphBuilder,
}

impl Clone for SolverSerial {
    fn clone(&self) -> Self {
        Self::new(&self.initializer)  // create independent instances of the solver
    }
}

impl SolverSerial {

    /// create a new decoder
    pub fn new(initializer: &SolverInitializer) -> Self {
        let mut dual_module = DualModuleSerial::new(initializer);
        let primal_module = PrimalModuleSerial::new(initializer);
        let interface = DualModuleInterface::new(&vec![], &mut dual_module);  // initialize with empty syndrome
        let subgraph_builder = SubGraphBuilder::new(initializer);
        Self {
            initializer: initializer.clone(),
            primal_module: primal_module,
            dual_module: dual_module,
            interface: interface,
            subgraph_builder: subgraph_builder,
        }
    }

    pub fn solve_perfect_matching(&mut self, syndrome_vertices: &Vec<usize>, visualizer: Option<&mut Visualizer>) -> PerfectMatching {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface = self.primal_module.solve_visualizer(syndrome_vertices, &mut self.dual_module, visualizer);
        self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module)
    }

    /// solve subgraph directly
    pub fn solve_subgraph(&mut self, syndrome_vertices: &Vec<usize>) -> Vec<EdgeIndex> {
        self.solve_subgraph_visualizer(syndrome_vertices, None)
    }

    pub fn solve_subgraph_visualizer(&mut self, syndrome_vertices: &Vec<usize>, visualizer: Option<&mut Visualizer>) -> Vec<EdgeIndex> {
        let perfect_matching = self.solve_perfect_matching(syndrome_vertices, visualizer);
        self.subgraph_builder.clear();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        self.subgraph_builder.get_subgraph()
    }

    /// solve the minimum weight perfect matching (legacy API, the same output as the blossom V library)
    pub fn solve_legacy(&mut self, syndrome_vertices: &Vec<usize>) -> Vec<usize> {
        self.solve_legacy_visualizer(syndrome_vertices, None)
    }

    pub fn solve_legacy_visualizer(&mut self, syndrome_vertices: &Vec<usize>, visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        self.primal_module.clear();
        self.dual_module.clear();
        self.interface = self.primal_module.solve_visualizer(syndrome_vertices, &mut self.dual_module, visualizer);
        let perfect_matching = self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module);
        perfect_matching.legacy_get_mwpm_result(&syndrome_vertices)
    }

}

// static functions, not recommended because it doesn't reuse the data structure of dual module
impl SolverSerial {

    pub fn mwpm_solve(initializer: &SolverInitializer, syndrome_nodes: &Vec<usize>) -> Vec<usize> {
        Self::mwpm_solve_visualizer(initializer, syndrome_nodes, None)
    }

    pub fn mwpm_solve_visualizer(initializer: &SolverInitializer, syndrome_nodes: &Vec<usize>, visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        let mut solver = Self::new(initializer);
        solver.solve_legacy_visualizer(syndrome_nodes, visualizer)
    }

}

impl FusionVisualizer for SolverSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}
