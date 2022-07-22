//! Minimum-Weight Perfect Matching Solver
//! 
//! This module includes some common usage of primal and dual modules to solve MWPM problem.
//! Note that you can call different primal and dual modules, even interchangeably, by following the examples in this file
//! 

use super::util::*;
use std::sync::Arc;
use super::dual_module::{DualModuleInterface, DualModuleImpl};
use super::primal_module::{PrimalModuleImpl};
use super::dual_module_serial::DualModuleSerial;
use super::primal_module_serial::PrimalModuleSerial;
use super::visualize::*;
use crate::derivative::Derivative;


#[derive(Debug)]
pub struct SolverInitializer {
    pub vertex_num: usize,
    pub weighted_edges: Vec<(usize, usize, Weight)>,
    pub virtual_vertices: Vec<usize>,
}

/// a serial solver
#[derive(Derivative)]
#[derivative(Debug)]
pub struct SolverSerial {
    initializer: Arc<SolverInitializer>,
    /// a serial implementation of the primal module
    #[derivative(Debug="ignore")]
    primal_module: PrimalModuleSerial,
    /// a serial implementation of the dual module
    #[derivative(Debug="ignore")]
    dual_module: DualModuleSerial,
    /// the interface between the primal and dual module
    interface: DualModuleInterface,
}

impl Clone for SolverSerial {
    fn clone(&self) -> Self {
        Self::from_initializer(&self.initializer)  // create independent instances of the solver
    }
}

impl SolverSerial {

    /// create a new decoder
    pub fn new(vertex_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_vertices: &Vec<usize>) -> Self {
        let initializer = Arc::new(SolverInitializer {
            vertex_num: vertex_num,
            weighted_edges: weighted_edges.clone(),
            virtual_vertices: virtual_vertices.clone(),
        });
        Self::from_initializer(&initializer)
    }

    pub fn from_initializer(initializer: &Arc<SolverInitializer>) -> Self {
        let mut dual_module = DualModuleSerial::new(initializer.vertex_num, &initializer.weighted_edges, &initializer.virtual_vertices);
        let primal_module = PrimalModuleSerial::new(initializer.vertex_num, &initializer.weighted_edges, &initializer.virtual_vertices);
        let interface = DualModuleInterface::new(&vec![], &mut dual_module);  // initialize with empty syndrome
        Self {
            initializer: Arc::clone(&initializer),
            primal_module: primal_module,
            dual_module: dual_module,
            interface: interface,
        }
    }

    pub fn load_syndrome(&mut self, syndrome_vertices: &Vec<usize>) {
        self.interface = DualModuleInterface::new(syndrome_vertices, &mut self.dual_module);
        self.primal_module.load(&self.interface);
    }

    pub fn solve(&mut self, syndrome_vertices: &Vec<usize>) -> Vec<usize> {
        self.load_syndrome(syndrome_vertices);
        // grow until end
        let mut group_max_update_length = self.dual_module.compute_maximum_update_length();
        while !group_max_update_length.is_empty() {
            // println!("group_max_update_length: {:?}", group_max_update_length);
            if let Some(length) = group_max_update_length.get_none_zero_growth() {
                self.interface.grow(length, &mut self.dual_module);
                // visualizer.as_mut().map(|v| v.snapshot_combined(format!("grow {length}"), vec![&interface, &dual_module, &primal_module]).unwrap());
            } else {
                // let first_conflict = format!("{:?}", group_max_update_length.get_conflicts().peek().unwrap());
                self.primal_module.resolve(group_max_update_length, &mut self.interface, &mut self.dual_module);
                // visualizer.as_mut().map(|v| v.snapshot_combined(format!("resolve {first_conflict}"), vec![&interface, &dual_module, &primal_module]).unwrap());
            }
            group_max_update_length = self.dual_module.compute_maximum_update_length();
        }
        self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module).legacy_get_mwpm_result(&syndrome_vertices)
    }

    // utilities to call this solver
    pub fn solve_mwpm_visualizer(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_vertices: &Vec<usize>
            , syndrome_vertices: &Vec<usize>, mut visualizer: Option<&mut Visualizer>) -> Vec<usize> {
        let mut solver = Self::new(node_num, weighted_edges, virtual_vertices);
        solver.load_syndrome(syndrome_vertices);
        if let Some(ref mut visualizer) = visualizer { visualizer.snapshot(format!("start"), &solver).unwrap(); }
        unimplemented!()
    }

    pub fn solve_mwpm(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_vertices: &Vec<usize>, syndrome_nodes: &Vec<usize>) -> Vec<usize> {
        Self::solve_mwpm_visualizer(node_num, weighted_edges, virtual_vertices, syndrome_nodes, None)
    }

}

impl FusionVisualizer for SolverSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut value = self.primal_module.snapshot(abbrev);
        snapshot_combine_values(&mut value, self.dual_module.snapshot(abbrev), abbrev);
        value
    }
}
