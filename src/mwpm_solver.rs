//! Minimum-Weight Perfect Matching Solver
//! 
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//! 

use super::util::*;
use super::dual_module::{DualModuleInterface, DualModuleImpl};
use super::primal_module::{PrimalModuleImpl};
use super::dual_module_serial::DualModuleSerial;
use super::primal_module_serial::PrimalModuleSerial;
use super::visualize::*;


/// a serial solver
pub struct SolverSerial {
    /// a serial implementation of the primal module
    primal_module: PrimalModuleSerial,
    /// a serial implementation of the dual module
    dual_module: DualModuleSerial,
    /// the interface between the primal and dual module
    interface: DualModuleInterface,
}

impl SolverSerial {

    /// create a 
    pub fn new(vertex_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_nodes: &Vec<usize>) -> Self {
        let mut dual_module = DualModuleSerial::new(vertex_num, weighted_edges, virtual_nodes);
        let primal_module = PrimalModuleSerial::new(vertex_num, weighted_edges, virtual_nodes);
        let interface = DualModuleInterface::new(&vec![], &mut dual_module);  // initialize with empty syndrome
        Self {
            primal_module: primal_module,
            dual_module: dual_module,
            interface: interface,
        }
    }

    pub fn load_syndrome(&mut self, syndrome_vertices: &Vec<usize>) {
        unimplemented!()
    }

    // utilities to call this solver
    pub fn solve_mwpm_visualizer(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_nodes: &Vec<usize>
            , syndrome_vertices: &Vec<usize>, mut visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        let mut solver = Self::new(node_num, weighted_edges, virtual_nodes);
        solver.load_syndrome(syndrome_vertices);
        if let Some(ref mut visualizer) = visualizer { visualizer.snapshot(format!("start"), &solver).unwrap(); }
        unimplemented!()
    }

    pub fn solve_mwpm(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_nodes: &Vec<usize>, syndrome_nodes: &Vec<usize>) -> Vec<usize> {
        Self::solve_mwpm_visualizer(node_num, weighted_edges, virtual_nodes, syndrome_nodes, None)
    }

}

impl FusionVisualizer for SolverSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}
