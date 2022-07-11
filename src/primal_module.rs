//! Primal Module
//! 
//! Generics for primal modules, defining the necessary interfaces for a primal module
//!

use super::util::*;
use crate::derivative::Derivative;
use super::dual_module::*;


/// generates a series of actions that the dual module needs to execute
#[derive(Derivative, PartialEq)]
#[derivative(Debug)]
pub enum PrimalInstruction {
    /// update dual node grow state
    UpdateGrowState(DualNodePtr, DualNodeGrowState),
    /// create a blossom
    CreateBlossom(Vec<DualNodePtr>),
    /// expand a blossom
    ExpandBlossom(DualNodePtr),
}

pub type PrimalInstructionVec = Vec<PrimalInstruction>;

/// common trait that must be implemented for each implementation of primal module
pub trait PrimalModuleImpl {

    /// create a primal module given the same parameters of the dual module, although not all of them is needed
    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self;

    /// clear all states to prepare for the next decoding task
    fn clear(&mut self);

    /// analyze the reason why dual module cannot further grow, update primal data structure (alternating tree, temporary matches, etc)
    /// and then tell dual module what to do
    fn update(&mut self, max_update_length: &MaxUpdateLength) -> PrimalInstructionVec;

}
