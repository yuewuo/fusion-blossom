//! Primal Module
//! 
//! Generics for primal modules, defining the necessary interfaces for a primal module
//!

use super::util::*;
use super::dual_module::*;
use crate::derivative::Derivative;


#[derive(Derivative)]
#[derivative(Debug)]
pub struct PerfectMatching {
    /// matched pairs; note that each pair will only appear once
    pub peer_matchings: Vec<(DualNodePtr, DualNodePtr)>,
    /// those nodes matched to the boundary
    pub virtual_matchings: Vec<(DualNodePtr, VertexIndex)>,
}

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
    fn resolve<D: DualModuleImpl>(&mut self, group_max_update_length: GroupMaxUpdateLength, interface: &mut DualModuleInterface, dual_module: &mut D);

    /// return a matching that can possibly include blossom nodes: this does not affect dual module
    fn intermediate_matching<D: DualModuleImpl>(&mut self, interface: &mut DualModuleInterface, dual_module: &mut D) -> PerfectMatching;

    /// break down the blossoms to find the final matching; this function will take more time on the dual module
    fn final_matching<D: DualModuleImpl>(&mut self, interface: &mut DualModuleInterface, dual_module: &mut D) -> PerfectMatching {
        let intermediate_perfect_matching = self.intermediate_matching(interface, dual_module);
        self.expand_matching(&intermediate_perfect_matching, interface, dual_module)
    }

    /// convert any intermediate matching to final matching with only syndrome nodes
    fn expand_matching<D: DualModuleImpl>(&mut self, intermediate_matching: &PerfectMatching, interface: &mut DualModuleInterface, dual_module: &mut D) -> PerfectMatching {
        let mut perfect_matching = PerfectMatching::new();
        // handle peer matchings
        for (dual_node_ptr_1, dual_node_ptr_2) in intermediate_matching.peer_matchings.iter() {
            perfect_matching.peer_matchings.extend(self.expand_peer_matching(dual_node_ptr_1, dual_node_ptr_2, interface, dual_module));
        }
        // handle virtual matchings
        for (dual_node_ptr, virtual_vertex) in intermediate_matching.virtual_matchings.iter() {
            let interface_node = dual_node_ptr.read_recursive();
            let is_blossom = matches!(interface_node.class, DualNodeClass::Blossom{ .. });
            drop(interface_node);  // unlock
            let grandson = if is_blossom {
                unimplemented!()
                // let grandson = dual_module.peek_touching_grandson_virtual(dual_node_ptr, *virtual_vertex);
                // perfect_matching.peer_matchings.extend(self.expand_blossom(dual_node_ptr, &grandson));
                // grandson
            } else { dual_node_ptr.clone() };
            perfect_matching.virtual_matchings.push((grandson, *virtual_vertex));
        }
        perfect_matching
    }

    /// break down a single matched pair to find the perfect matching
    fn expand_peer_matching<D: DualModuleImpl>(&mut self, dual_node_ptr_1: &DualNodePtr, dual_node_ptr_2: &DualNodePtr, interface: &mut DualModuleInterface, dual_module: &mut D)
            -> Vec<(DualNodePtr, DualNodePtr)> {
        let mut peer_matchings = vec![];
        let interface_node_1 = dual_node_ptr_1.read_recursive();
        let interface_node_2 = dual_node_ptr_2.read_recursive();
        let is_blossom_1 = matches!(interface_node_1.class, DualNodeClass::Blossom{ .. });
        let dual_variable_1 = interface_node_1.get_dual_variable(interface);
        let is_blossom_2 = matches!(interface_node_2.class, DualNodeClass::Blossom{ .. });
        let dual_variable_2 = interface_node_2.get_dual_variable(interface);
        drop(interface_node_1);  // unlock
        drop(interface_node_2);  // unlock
        let (descendant_1, is_blossom_descendant_1) = if is_blossom_1 {
            let descendant = dual_module.peek_touching_descendant(dual_node_ptr_1, dual_node_ptr_2);
            peer_matchings.extend(self.expand_blossom(dual_node_ptr_1, &descendant));
            let is_blossom_descendant = matches!(descendant.read_recursive().class, DualNodeClass::Blossom{ .. });
            (descendant, is_blossom_descendant)
        } else { (dual_node_ptr_1.clone(), false) };
        let (descendant_2, is_blossom_descendant_2) = if is_blossom_2 {
            let descendant = dual_module.peek_touching_descendant(dual_node_ptr_2, dual_node_ptr_1);
            peer_matchings.extend(self.expand_blossom(dual_node_ptr_2, &descendant));
            let is_blossom_descendant = matches!(descendant.read_recursive().class, DualNodeClass::Blossom{ .. });
            (descendant, is_blossom_descendant)
        } else { (dual_node_ptr_2.clone(), false) };
        if !is_blossom_descendant_1 && !is_blossom_descendant_2 {
            peer_matchings.push((descendant_1, descendant_2));
        } else {
            peer_matchings.extend(self.expand_peer_matching(&descendant_1, &descendant_2, interface, dual_module))
        }
        peer_matchings
    }

    /// expand blossom iteratively into matched pairs, note that this will NOT change the structure of the primal module;
    fn expand_blossom(&mut self, blossom_ptr: &DualNodePtr, descendant_ptr: &DualNodePtr) -> Vec<(DualNodePtr, DualNodePtr)> {
        let mut child_ptr = descendant_ptr.clone();  // first find the direct child
        while child_ptr.read_recursive().parent_blossom != Some(blossom_ptr.downgrade()) {
            let parenet_ptr = child_ptr.read_recursive().parent_blossom.as_ref().expect("must be descendant").upgrade_force();
            child_ptr = parenet_ptr;
            
        }
        unimplemented!()
    }

}

impl PerfectMatching {

    pub fn new() -> Self {
        Self {
            peer_matchings: vec![],
            virtual_matchings: vec![],
        }
    }

}
