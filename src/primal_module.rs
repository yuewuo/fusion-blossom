//! Primal Module
//! 
//! Generics for primal modules, defining the necessary interfaces for a primal module
//!

use super::util::*;
use super::dual_module::*;
use crate::derivative::Derivative;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use super::complete_graph::*;


#[derive(Derivative)]
#[derivative(Debug)]
pub struct IntermediateMatching {
    /// matched pairs; note that each pair will only appear once. (node_1, touching_1), (node_2, touching_2)
    pub peer_matchings: Vec<((DualNodePtr, DualNodeWeak), (DualNodePtr, DualNodeWeak))>,
    /// those nodes matched to the boundary. ((node, touching), virtual_vertex)
    pub virtual_matchings: Vec<((DualNodePtr, DualNodeWeak), VertexIndex)>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PerfectMatching {
    /// matched pairs; note that each pair will only appear once. (syndrome_node_1, syndrome_node_2)
    pub peer_matchings: Vec<(DualNodePtr, DualNodePtr)>,
    /// those nodes matched to the boundary. (syndrome node, virtual_vertex)
    pub virtual_matchings: Vec<(DualNodePtr, VertexIndex)>,
}

/// common trait that must be implemented for each implementation of primal module
pub trait PrimalModuleImpl {

    /// create a primal module given the dual module
    fn new<D: DualModuleImpl>(dual_module: &D) -> Self;

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
    fn intermediate_matching<D: DualModuleImpl>(&mut self, interface: &mut DualModuleInterface, dual_module: &mut D) -> IntermediateMatching;

    /// break down the blossoms to find the final matching; this function will take more time on the dual module
    fn perfect_matching<D: DualModuleImpl>(&mut self, interface: &mut DualModuleInterface, dual_module: &mut D) -> PerfectMatching {
        let intermediate_matching = self.intermediate_matching(interface, dual_module);
        intermediate_matching.get_perfect_matching()
    }

}

impl IntermediateMatching {

    pub fn new() -> Self {
        Self {
            peer_matchings: vec![],
            virtual_matchings: vec![],
        }
    }

    /// expand the intermediate matching into a perfect matching with only syndrome nodes
    pub fn get_perfect_matching(&self) -> PerfectMatching {
        let mut perfect_matching = PerfectMatching::new();
        // handle peer matchings
        for ((dual_node_ptr_1, touching_weak_1), (dual_node_ptr_2, touching_weak_2)) in self.peer_matchings.iter() {
            let touching_ptr_1 = touching_weak_1.upgrade_force();
            let touching_ptr_2 = touching_weak_2.upgrade_force();
            perfect_matching.peer_matchings.extend(Self::expand_peer_matching(dual_node_ptr_1, &touching_ptr_1, dual_node_ptr_2, &touching_ptr_2));
        }
        // handle virtual matchings
        for ((dual_node_ptr, touching_weak), virtual_vertex) in self.virtual_matchings.iter() {
            let touching_ptr = touching_weak.upgrade_force();
            perfect_matching.peer_matchings.extend(Self::expand_blossom(dual_node_ptr, &touching_ptr));
            perfect_matching.virtual_matchings.push((touching_ptr, *virtual_vertex));
        }
        perfect_matching
    }

    /// break down a single matched pair to find the perfect matching
    pub fn expand_peer_matching(dual_node_ptr_1: &DualNodePtr, touching_ptr_1: &DualNodePtr, dual_node_ptr_2: &DualNodePtr
            , touching_ptr_2: &DualNodePtr) -> Vec<(DualNodePtr, DualNodePtr)> {
        // println!("expand_peer_matching ({:?}, {:?}), ({:?}, {:?}) {{", dual_node_ptr_1, touching_ptr_1, dual_node_ptr_2, touching_ptr_2);
        let mut perfect_matching = vec![];
        perfect_matching.extend(Self::expand_blossom(dual_node_ptr_1, touching_ptr_1));
        perfect_matching.extend(Self::expand_blossom(dual_node_ptr_2, touching_ptr_2));
        perfect_matching.push((touching_ptr_1.clone(), touching_ptr_2.clone()));
        // println!("}},");
        perfect_matching
    }

    /// expand blossom iteratively into matched pairs, note that this will NOT change the structure of the primal module;
    pub fn expand_blossom(blossom_ptr: &DualNodePtr, touching_ptr: &DualNodePtr) -> Vec<(DualNodePtr, DualNodePtr)> {
        // println!("expand_blossom ({:?}, {:?}) {{", blossom_ptr, touching_ptr);
        let mut perfect_matching = vec![];
        let mut child_ptr = touching_ptr.clone();
        while &child_ptr != blossom_ptr {
            let child_weak = child_ptr.downgrade();
            let child = child_ptr.read_recursive();
            if let Some(parent_blossom_weak) = child.parent_blossom.as_ref() {
                let parent_blossom_ptr = parent_blossom_weak.upgrade_force();
                let parent_blossom = parent_blossom_ptr.read_recursive();
                if let DualNodeClass::Blossom{ nodes_circle, touching_children } = &parent_blossom.class {
                    let idx = nodes_circle.iter().position(|ptr| ptr == &child_weak).expect("should find child");
                    debug_assert!(nodes_circle.len() % 2 == 1 && nodes_circle.len() >= 3, "must be a valid blossom");
                    for i in (0..(nodes_circle.len()-1)).step_by(2) {
                        let idx_1 = (idx + i + 1) % nodes_circle.len();
                        let idx_2 = (idx + i + 2) % nodes_circle.len();
                        let dual_node_ptr_1 = nodes_circle[idx_1].upgrade_force();
                        let dual_node_ptr_2 = nodes_circle[idx_2].upgrade_force();
                        let touching_ptr_1 = touching_children[idx_1].1.upgrade_force();  // match to right
                        let touching_ptr_2 = touching_children[idx_2].0.upgrade_force();  // match to left
                        perfect_matching.extend(Self::expand_peer_matching(
                            &dual_node_ptr_1, &touching_ptr_1, &dual_node_ptr_2, &touching_ptr_2
                        ))
                    }
                }
                drop(child);
                child_ptr = parent_blossom_ptr.clone();
            } else { panic!("cannot find parent of {}", child.index) }
        }
        // println!("}},");
        perfect_matching
    }

}

impl PerfectMatching {

    pub fn new() -> Self {
        Self {
            peer_matchings: vec![],
            virtual_matchings: vec![],
        }
    }

    /// this interface is not very optimized, but is compatible with blossom V algorithm's result
    pub fn legacy_get_mwpm_result(&self, syndrome_vertices: &Vec<usize>) -> Vec<usize> {
        let mut peer_matching_maps = BTreeMap::<usize, usize>::new();
        for (ptr_1, ptr_2) in self.peer_matchings.iter() {
            let a_vid = {
                let node = ptr_1.read_recursive();
                if let DualNodeClass::SyndromeVertex{ syndrome_index } = &node.class { *syndrome_index } else { unreachable!("can only be syndrome") }
            };
            let b_vid = {
                let node = ptr_2.read_recursive();
                if let DualNodeClass::SyndromeVertex{ syndrome_index } = &node.class { *syndrome_index } else { unreachable!("can only be syndrome") }
            };
            peer_matching_maps.insert(a_vid, b_vid);
            peer_matching_maps.insert(b_vid, a_vid);
        }
        let mut virtual_matching_maps = BTreeMap::<usize, usize>::new();
        for (ptr, virtual_vertex) in self.virtual_matchings.iter() {
            let a_vid = {
                let node = ptr.read_recursive();
                if let DualNodeClass::SyndromeVertex{ syndrome_index } = &node.class { *syndrome_index } else { unreachable!("can only be syndrome") }
            };
            virtual_matching_maps.insert(a_vid, *virtual_vertex);
        }
        let mut mwpm_result = Vec::with_capacity(syndrome_vertices.len());
        for syndrome_vertex in syndrome_vertices.iter() {
            if let Some(a) = peer_matching_maps.get(&syndrome_vertex) {
                mwpm_result.push(*a);
            } else if let Some(v) = virtual_matching_maps.get(&syndrome_vertex) {
                mwpm_result.push(*v);
            } else { panic!("cannot find syndrome vertex {}", syndrome_vertex) }
        }
        mwpm_result
    }

}

/// build a subgraph based on minimum-weight paths between matched pairs
#[derive(Debug, Clone)]
pub struct SubGraphBuilder {
    /// number of vertices
    pub vertex_num: usize,
    /// mapping from vertex pair to edge index
    vertex_pair_edges: HashMap<(VertexIndex, VertexIndex), EdgeIndex>,
    /// an instance of complete graph to compute minimum-weight path between any pair of vertices
    pub complete_graph: CompleteGraph,
    /// current subgraph, assuming edges are not very much
    pub subgraph: BTreeSet<EdgeIndex>,
}

impl SubGraphBuilder {

    pub fn new(initializer: &SolverInitializer) -> Self {
        let mut vertex_pair_edges = HashMap::with_capacity(initializer.weighted_edges.len());
        for (edge_index, (i, j, _)) in initializer.weighted_edges.iter().enumerate() {
            let id = if i < j { (*i, *j) } else { (*j, *i) };
            vertex_pair_edges.insert(id, edge_index);
        }
        Self {
            vertex_num: initializer.vertex_num,
            vertex_pair_edges: vertex_pair_edges,
            complete_graph: CompleteGraph::new(initializer.vertex_num, &initializer.weighted_edges),
            subgraph: BTreeSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.subgraph.clear();
        self.complete_graph.reset();
    }

    /// temporarily set some edges to 0 weight, and when it resets, those edges will be reverted back to the original weight
    pub fn load_erasures(&mut self, erasures: &Vec<EdgeIndex>) {
        self.complete_graph.load_erasures(erasures);
    }

    /// load perfect matching to the subgraph builder
    pub fn load_perfect_matching(&mut self, perfect_matching: &PerfectMatching) {
        for (ptr_1, ptr_2) in perfect_matching.peer_matchings.iter() {
            let a_vid = {
                let node = ptr_1.read_recursive();
                if let DualNodeClass::SyndromeVertex{ syndrome_index } = &node.class { *syndrome_index } else { unreachable!("can only be syndrome") }
            };
            let b_vid = {
                let node = ptr_2.read_recursive();
                if let DualNodeClass::SyndromeVertex{ syndrome_index } = &node.class { *syndrome_index } else { unreachable!("can only be syndrome") }
            };
            self.add_matching(a_vid, b_vid);
        }
        for (ptr, virtual_vertex) in perfect_matching.virtual_matchings.iter() {
            let a_vid = {
                let node = ptr.read_recursive();
                if let DualNodeClass::SyndromeVertex{ syndrome_index } = &node.class { *syndrome_index } else { unreachable!("can only be syndrome") }
            };
            self.add_matching(a_vid, *virtual_vertex);
        }
    }

    /// add a matching, finding the minimum path and XOR them into the subgraph (if adding the same pair twice, they will cancel each other)
    pub fn add_matching(&mut self, vertex_1: VertexIndex, vertex_2: VertexIndex) {
        let (path, _) = self.complete_graph.get_path(vertex_1, vertex_2);
        let mut a = vertex_1;
        for (vertex, _) in path.iter() {
            let b = *vertex;
            let id = if a < b { (a, b) } else { (b, a) };
            let edge_index = *self.vertex_pair_edges.get(&id).expect("edge should exist");
            if self.subgraph.contains(&edge_index) {
                self.subgraph.remove(&edge_index);
            } else {
                self.subgraph.insert(edge_index);
            }
            a = b;
        }
    }

    /// get the total weight of the subgraph
    pub fn total_weight(&self) -> Weight {
        let mut weight = 0;
        for edge_index in self.subgraph.iter() {
            weight += self.complete_graph.weighted_edges[*edge_index].2;
        }
        weight
    }

    /// get subgraph as a vec
    pub fn get_subgraph(&self) -> Vec<EdgeIndex> {
        self.subgraph.iter().map(|x| *x).collect()
    }

}
