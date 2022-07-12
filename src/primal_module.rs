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

    /// clear all states; however this method is not necessarily called when load a new decoding problem, so you need to call it yourself
    fn clear(&mut self);

    /// load a new decoding problem given dual interface: note that all 
    fn load(&mut self, interface: &DualModuleInterface);

    /// analyze the reason why dual module cannot further grow, update primal data structure (alternating tree, temporary matches, etc)
    /// and then tell dual module what to do to resolve these conflicts;
    /// note that this function doesn't necessarily resolve all the conflicts, but can return early if some major change is made.
    /// when implementing this function, it's recommended that you resolve as many conflicts as possible.
    fn resolve(&mut self, group_max_update_length: GroupMaxUpdateLength) -> PrimalInstructionVec;

}
