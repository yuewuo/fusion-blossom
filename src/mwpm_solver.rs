//! Minimum-Weight Perfect Matching Solver
//! 
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//! 

use super::util::*;
use super::dual_module::{DualModuleInterface, DualModuleImpl};
use super::primal_module::{PrimalModuleImpl, SubGraphBuilder};
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

    pub fn load_syndrome(&mut self, syndrome_vertices: &Vec<usize>) {
        self.interface = DualModuleInterface::new(syndrome_vertices, &mut self.dual_module);
        self.primal_module.load(&self.interface);
    }

    pub fn load_syndrome_and_solve(&mut self, syndrome_vertices: &Vec<usize>) {
        self.load_syndrome(syndrome_vertices);
        // grow until end
        let mut group_max_update_length = self.dual_module.compute_maximum_update_length();
        while !group_max_update_length.is_empty() {
            // println!("group_max_update_length: {:?}", group_max_update_length);
            if let Some(length) = group_max_update_length.get_none_zero_growth() {
                self.interface.grow(length, &mut self.dual_module);
            } else {
                self.primal_module.resolve(group_max_update_length, &mut self.interface, &mut self.dual_module);
            }
            group_max_update_length = self.dual_module.compute_maximum_update_length();
        }
    }

    /// solve subgraph directly
    pub fn solve_subgraph(&mut self, syndrome_vertices: &Vec<usize>) -> Vec<EdgeIndex> {
        self.load_syndrome_and_solve(syndrome_vertices);
        let perfect_matching = self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module);
        self.subgraph_builder.clear();
        self.subgraph_builder.load_perfect_matching(&perfect_matching);
        self.subgraph_builder.get_subgraph()
    }

    pub fn solve(&mut self, syndrome_vertices: &Vec<usize>) -> Vec<usize> {
        self.load_syndrome_and_solve(syndrome_vertices);
        self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module).legacy_get_mwpm_result(&syndrome_vertices)
    }

    // utilities to call this solver
    pub fn solve_mwpm_visualizer(initializer: &SolverInitializer, syndrome_vertices: &Vec<usize>, mut visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        let mut solver = Self::new(initializer);
        solver.load_syndrome(syndrome_vertices);
        if let Some(ref mut visualizer) = visualizer { visualizer.snapshot(format!("start"), &solver).unwrap(); }
        unimplemented!()
    }

    pub fn solve_mwpm(initializer: &SolverInitializer, syndrome_nodes: &Vec<usize>) -> Vec<usize> {
        Self::solve_mwpm_visualizer(initializer, syndrome_nodes, None)
    }

}

impl FusionVisualizer for SolverSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}
