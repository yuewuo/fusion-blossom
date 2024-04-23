//! Primal Module
//!
//! Generics for primal modules, defining the necessary interfaces for a primal module
//!

#![cfg_attr(feature = "unsafe_pointer", allow(dropping_references))]
use super::complete_graph::*;
use super::dual_module::*;
use super::pointers::*;
use super::util::*;
use super::visualize::*;
use crate::derivative::Derivative;
#[cfg(feature = "python_binding")]
use pyo3::prelude::*;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Derivative)]
#[derivative(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct IntermediateMatching {
    /// matched pairs; note that each pair will only appear once. (node_1, touching_1), (node_2, touching_2)
    pub peer_matchings: Vec<((DualNodePtr, DualNodeWeak), (DualNodePtr, DualNodeWeak))>,
    /// those nodes matched to the boundary. ((node, touching), virtual_vertex)
    pub virtual_matchings: Vec<((DualNodePtr, DualNodeWeak), VertexIndex)>,
}

#[derive(Derivative)]
#[derivative(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct PerfectMatching {
    /// matched pairs; note that each pair will only appear once. (defect_node_1, defect_node_2)
    pub peer_matchings: Vec<(DualNodePtr, DualNodePtr)>,
    /// those nodes matched to the boundary. (syndrome node, virtual_vertex)
    pub virtual_matchings: Vec<(DualNodePtr, VertexIndex)>,
}

/// common trait that must be implemented for each implementation of primal module
pub trait PrimalModuleImpl {
    /// create a primal module given the dual module
    fn new_empty(solver_initializer: &SolverInitializer) -> Self;

    /// clear all states; however this method is not necessarily called when load a new decoding problem, so you need to call it yourself
    fn clear(&mut self);

    fn load_defect_dual_node(&mut self, dual_node_ptr: &DualNodePtr);

    /// load a single syndrome and update the dual module and the interface
    fn load_defect<D: DualModuleImpl>(
        &mut self,
        defect_vertex: VertexIndex,
        interface_ptr: &DualModuleInterfacePtr,
        dual_module: &mut D,
    ) {
        interface_ptr.create_defect_node(defect_vertex, dual_module);
        let interface = interface_ptr.read_recursive();
        let index = interface.nodes_length - 1;
        self.load_defect_dual_node(
            interface.nodes[index]
                .as_ref()
                .expect("must load a fresh dual module interface, found empty node"),
        )
    }

    /// load a new decoding problem given dual interface: note that all nodes MUST be syndrome node
    #[allow(clippy::unnecessary_cast)]
    fn load(&mut self, interface_ptr: &DualModuleInterfacePtr) {
        let interface = interface_ptr.read_recursive();
        debug_assert!(interface.parent.is_none(), "cannot load an interface that is already fused");
        debug_assert!(
            interface.children.is_none(),
            "please customize load function if interface is fused"
        );
        for index in 0..interface.nodes_length as NodeIndex {
            let node = &interface.nodes[index as usize];
            debug_assert!(node.is_some(), "must load a fresh dual module interface, found empty node");
            let node_ptr = node.as_ref().unwrap();
            let node = node_ptr.read_recursive();
            debug_assert!(
                matches!(node.class, DualNodeClass::DefectVertex { .. }),
                "must load a fresh dual module interface, found a blossom"
            );
            debug_assert_eq!(
                node.index, index,
                "must load a fresh dual module interface, found index out of order"
            );
            self.load_defect_dual_node(node_ptr);
        }
    }

    /// analyze the reason why dual module cannot further grow, update primal data structure (alternating tree, temporary matches, etc)
    /// and then tell dual module what to do to resolve these conflicts;
    /// note that this function doesn't necessarily resolve all the conflicts, but can return early if some major change is made.
    /// when implementing this function, it's recommended that you resolve as many conflicts as possible.
    fn resolve<D: DualModuleImpl>(
        &mut self,
        group_max_update_length: GroupMaxUpdateLength,
        interface: &DualModuleInterfacePtr,
        dual_module: &mut D,
    );

    /// return a matching that can possibly include blossom nodes: this does not affect dual module
    fn intermediate_matching<D: DualModuleImpl>(
        &mut self,
        interface: &DualModuleInterfacePtr,
        dual_module: &mut D,
    ) -> IntermediateMatching;

    /// break down the blossoms to find the final matching; this function will take more time on the dual module
    fn perfect_matching<D: DualModuleImpl>(
        &mut self,
        interface: &DualModuleInterfacePtr,
        dual_module: &mut D,
    ) -> PerfectMatching {
        let intermediate_matching = self.intermediate_matching(interface, dual_module);
        intermediate_matching.get_perfect_matching()
    }

    fn solve<D: DualModuleImpl>(
        &mut self,
        interface: &DualModuleInterfacePtr,
        syndrome_pattern: &SyndromePattern,
        dual_module: &mut D,
    ) {
        self.solve_step_callback(interface, syndrome_pattern, dual_module, |_, _, _, _| {})
    }

    fn solve_visualizer<D: DualModuleImpl + FusionVisualizer>(
        &mut self,
        interface: &DualModuleInterfacePtr,
        syndrome_pattern: &SyndromePattern,
        dual_module: &mut D,
        visualizer: Option<&mut Visualizer>,
    ) where
        Self: FusionVisualizer + Sized,
    {
        if let Some(visualizer) = visualizer {
            self.solve_step_callback(
                interface,
                syndrome_pattern,
                dual_module,
                |interface, dual_module, primal_module, group_max_update_length| {
                    #[cfg(test)]
                    println!("group_max_update_length: {:?}", group_max_update_length);
                    if let Some(length) = group_max_update_length.get_none_zero_growth() {
                        visualizer
                            .snapshot_combined(format!("grow {length}"), vec![interface, dual_module, primal_module])
                            .unwrap();
                    } else {
                        let first_conflict = format!("{:?}", group_max_update_length.peek().unwrap());
                        visualizer
                            .snapshot_combined(
                                format!("resolve {first_conflict}"),
                                vec![interface, dual_module, primal_module],
                            )
                            .unwrap();
                    };
                },
            );
            visualizer
                .snapshot_combined("solved".to_string(), vec![interface, dual_module, self])
                .unwrap();
        } else {
            self.solve(interface, syndrome_pattern, dual_module);
        }
    }

    fn solve_step_callback<D: DualModuleImpl, F>(
        &mut self,
        interface: &DualModuleInterfacePtr,
        syndrome_pattern: &SyndromePattern,
        dual_module: &mut D,
        callback: F,
    ) where
        F: FnMut(&DualModuleInterfacePtr, &mut D, &mut Self, &GroupMaxUpdateLength),
    {
        interface.load(syndrome_pattern, dual_module);
        self.load(interface);
        self.solve_step_callback_interface_loaded(interface, dual_module, callback);
    }

    fn solve_step_callback_interface_loaded<D: DualModuleImpl, F>(
        &mut self,
        interface: &DualModuleInterfacePtr,
        dual_module: &mut D,
        mut callback: F,
    ) where
        F: FnMut(&DualModuleInterfacePtr, &mut D, &mut Self, &GroupMaxUpdateLength),
    {
        let mut group_max_update_length = dual_module.compute_maximum_update_length();
        while !group_max_update_length.is_empty() {
            callback(interface, dual_module, self, &group_max_update_length);
            if let Some(length) = group_max_update_length.get_none_zero_growth() {
                interface.grow(length, dual_module);
            } else {
                self.resolve(group_max_update_length, interface, dual_module);
            }
            group_max_update_length = dual_module.compute_maximum_update_length();
        }
    }

    /// performance profiler report
    fn generate_profiler_report(&self) -> serde_json::Value {
        json!({})
    }
}

impl Default for IntermediateMatching {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl IntermediateMatching {
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new() -> Self {
        Self {
            peer_matchings: vec![],
            virtual_matchings: vec![],
        }
    }

    pub fn append(&mut self, other: &mut Self) {
        self.peer_matchings.append(&mut other.peer_matchings);
        self.virtual_matchings.append(&mut other.virtual_matchings);
    }

    /// expand the intermediate matching into a perfect matching with only syndrome nodes
    pub fn get_perfect_matching(&self) -> PerfectMatching {
        let mut perfect_matching = PerfectMatching::new();
        // handle peer matchings
        for ((dual_node_ptr_1, touching_weak_1), (dual_node_ptr_2, touching_weak_2)) in self.peer_matchings.iter() {
            let touching_ptr_1 = touching_weak_1.upgrade_force();
            let touching_ptr_2 = touching_weak_2.upgrade_force();
            perfect_matching.peer_matchings.extend(Self::expand_peer_matching(
                dual_node_ptr_1,
                &touching_ptr_1,
                dual_node_ptr_2,
                &touching_ptr_2,
            ));
        }
        // handle virtual matchings
        for ((dual_node_ptr, touching_weak), virtual_vertex) in self.virtual_matchings.iter() {
            let touching_ptr = touching_weak.upgrade_force();
            perfect_matching
                .peer_matchings
                .extend(Self::expand_blossom(dual_node_ptr, &touching_ptr));
            perfect_matching.virtual_matchings.push((touching_ptr, *virtual_vertex));
        }
        perfect_matching
    }

    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    #[cfg(feature = "python_binding")]
    #[getter]
    pub fn get_peer_matchings(&self) -> Vec<((NodeIndex, NodeIndex), (NodeIndex, NodeIndex))> {
        self.peer_matchings
            .iter()
            .map(|((a, b), (c, d))| {
                (
                    (a.updated_index(), b.upgrade_force().updated_index()),
                    (c.updated_index(), d.upgrade_force().updated_index()),
                )
            })
            .collect()
    }

    #[cfg(feature = "python_binding")]
    #[getter]
    pub fn get_virtual_matchings(&self) -> Vec<((NodeIndex, NodeIndex), VertexIndex)> {
        self.virtual_matchings
            .iter()
            .map(|((a, b), c)| ((a.updated_index(), b.upgrade_force().updated_index()), *c))
            .collect()
    }
}

impl IntermediateMatching {
    /// break down a single matched pair to find the perfect matching
    pub fn expand_peer_matching(
        dual_node_ptr_1: &DualNodePtr,
        touching_ptr_1: &DualNodePtr,
        dual_node_ptr_2: &DualNodePtr,
        touching_ptr_2: &DualNodePtr,
    ) -> Vec<(DualNodePtr, DualNodePtr)> {
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
                if let DualNodeClass::Blossom {
                    nodes_circle,
                    touching_children,
                } = &parent_blossom.class
                {
                    let idx = nodes_circle
                        .iter()
                        .position(|ptr| ptr == &child_weak)
                        .expect("should find child");
                    debug_assert!(
                        nodes_circle.len() % 2 == 1 && nodes_circle.len() >= 3,
                        "must be a valid blossom"
                    );
                    for i in (0..(nodes_circle.len() - 1)).step_by(2) {
                        let idx_1 = (idx + i + 1) % nodes_circle.len();
                        let idx_2 = (idx + i + 2) % nodes_circle.len();
                        let dual_node_ptr_1 = nodes_circle[idx_1].upgrade_force();
                        let dual_node_ptr_2 = nodes_circle[idx_2].upgrade_force();
                        let touching_ptr_1 = touching_children[idx_1].1.upgrade_force(); // match to right
                        let touching_ptr_2 = touching_children[idx_2].0.upgrade_force(); // match to left
                        perfect_matching.extend(Self::expand_peer_matching(
                            &dual_node_ptr_1,
                            &touching_ptr_1,
                            &dual_node_ptr_2,
                            &touching_ptr_2,
                        ))
                    }
                }
                drop(child);
                child_ptr = parent_blossom_ptr.clone();
            } else {
                panic!("cannot find parent of {}", child.index)
            }
        }
        // println!("}},");
        perfect_matching
    }
}

impl Default for PerfectMatching {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl PerfectMatching {
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new() -> Self {
        Self {
            peer_matchings: vec![],
            virtual_matchings: vec![],
        }
    }

    /// this interface is not very optimized, but is compatible with blossom V algorithm's result
    pub fn legacy_get_mwpm_result(&self, defect_vertices: Vec<VertexIndex>) -> Vec<DefectIndex> {
        let mut peer_matching_maps = BTreeMap::<VertexIndex, VertexIndex>::new();
        for (ptr_1, ptr_2) in self.peer_matchings.iter() {
            let a_vid = {
                let node = ptr_1.read_recursive();
                if let DualNodeClass::DefectVertex { defect_index } = &node.class {
                    *defect_index
                } else {
                    unreachable!("can only be syndrome")
                }
            };
            let b_vid = {
                let node = ptr_2.read_recursive();
                if let DualNodeClass::DefectVertex { defect_index } = &node.class {
                    *defect_index
                } else {
                    unreachable!("can only be syndrome")
                }
            };
            peer_matching_maps.insert(a_vid, b_vid);
            peer_matching_maps.insert(b_vid, a_vid);
        }
        let mut virtual_matching_maps = BTreeMap::<VertexIndex, VertexIndex>::new();
        for (ptr, virtual_vertex) in self.virtual_matchings.iter() {
            let a_vid = {
                let node = ptr.read_recursive();
                if let DualNodeClass::DefectVertex { defect_index } = &node.class {
                    *defect_index
                } else {
                    unreachable!("can only be syndrome")
                }
            };
            virtual_matching_maps.insert(a_vid, *virtual_vertex);
        }
        let mut mwpm_result = Vec::with_capacity(defect_vertices.len());
        for defect_vertex in defect_vertices.iter() {
            if let Some(a) = peer_matching_maps.get(defect_vertex) {
                mwpm_result.push(*a);
            } else if let Some(v) = virtual_matching_maps.get(defect_vertex) {
                mwpm_result.push(*v);
            } else {
                panic!("cannot find defect vertex {}", defect_vertex)
            }
        }
        mwpm_result
    }

    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    #[cfg(feature = "python_binding")]
    #[getter]
    pub fn get_peer_matchings(&self) -> Vec<(NodeIndex, NodeIndex)> {
        self.peer_matchings
            .iter()
            .map(|(a, b)| (a.updated_index(), b.updated_index()))
            .collect()
    }

    #[cfg(feature = "python_binding")]
    #[getter]
    pub fn get_virtual_matchings(&self) -> Vec<(NodeIndex, VertexIndex)> {
        self.virtual_matchings.iter().map(|(a, b)| (a.updated_index(), *b)).collect()
    }
}

impl FusionVisualizer for PerfectMatching {
    #[allow(clippy::unnecessary_cast)]
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let primal_nodes = if self.peer_matchings.is_empty() && self.virtual_matchings.is_empty() {
            vec![]
        } else {
            let mut maximum_node_index = 0;
            for (ptr_1, ptr_2) in self.peer_matchings.iter() {
                maximum_node_index = std::cmp::max(maximum_node_index, ptr_1.get_ancestor_blossom().read_recursive().index);
                maximum_node_index = std::cmp::max(maximum_node_index, ptr_2.get_ancestor_blossom().read_recursive().index);
            }
            for (ptr, _virtual_vertex) in self.virtual_matchings.iter() {
                maximum_node_index = std::cmp::max(maximum_node_index, ptr.get_ancestor_blossom().read_recursive().index);
            }
            let mut primal_nodes = vec![json!(null); maximum_node_index as usize + 1];
            for (ptr_1, ptr_2) in self.peer_matchings.iter() {
                for (ptr_a, ptr_b) in [(ptr_1, ptr_2), (ptr_2, ptr_1)] {
                    primal_nodes[ptr_a.read_recursive().index as usize] = json!({
                        if abbrev { "m" } else { "temporary_match" }: {
                            if abbrev { "p" } else { "peer" }: ptr_b.read_recursive().index,
                            if abbrev { "t" } else { "touching" }: ptr_a.read_recursive().index,
                        },
                        if abbrev { "t" } else { "tree_node" }: {
                            if abbrev { "r" } else { "root" }: ptr_a.read_recursive().index,
                            if abbrev { "d" } else { "depth" }: 1,
                        },
                    });
                }
            }
            for (ptr, virtual_vertex) in self.virtual_matchings.iter() {
                primal_nodes[ptr.read_recursive().index as usize] = json!({
                    if abbrev { "m" } else { "temporary_match" }: {
                        if abbrev { "v" } else { "virtual_vertex" }: virtual_vertex,
                        if abbrev { "t" } else { "touching" }: ptr.read_recursive().index,
                    },
                    if abbrev { "t" } else { "tree_node" }: {
                        if abbrev { "r" } else { "root" }: ptr.read_recursive().index,
                        if abbrev { "d" } else { "depth" }: 1,
                    },
                });
            }
            primal_nodes
        };
        json!({
            "primal_nodes": primal_nodes,
        })
    }
}

/// build a subgraph based on minimum-weight paths between matched pairs
#[derive(Debug, Clone)]
pub struct SubGraphBuilder {
    /// number of vertices
    pub vertex_num: VertexNum,
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
            vertex_pair_edges.insert(id, edge_index as EdgeIndex);
        }
        Self {
            vertex_num: initializer.vertex_num,
            vertex_pair_edges,
            complete_graph: CompleteGraph::new(initializer.vertex_num, &initializer.weighted_edges),
            subgraph: BTreeSet::new(),
        }
    }

    pub fn clear(&mut self) {
        self.subgraph.clear();
        self.complete_graph.reset();
    }

    /// temporarily set some edges to 0 weight, and when it resets, those edges will be reverted back to the original weight
    pub fn load_erasures(&mut self, erasures: &[EdgeIndex]) {
        self.complete_graph.load_erasures(erasures);
    }

    pub fn load_dynamic_weights(&mut self, dynamic_weights: &[(EdgeIndex, Weight)]) {
        self.complete_graph.load_dynamic_weights(dynamic_weights);
    }

    /// load perfect matching to the subgraph builder
    pub fn load_perfect_matching(&mut self, perfect_matching: &PerfectMatching) {
        self.subgraph.clear();
        for (ptr_1, ptr_2) in perfect_matching.peer_matchings.iter() {
            let a_vid = {
                let node = ptr_1.read_recursive();
                if let DualNodeClass::DefectVertex { defect_index } = &node.class {
                    *defect_index
                } else {
                    unreachable!("can only be syndrome")
                }
            };
            let b_vid = {
                let node = ptr_2.read_recursive();
                if let DualNodeClass::DefectVertex { defect_index } = &node.class {
                    *defect_index
                } else {
                    unreachable!("can only be syndrome")
                }
            };
            self.add_matching(a_vid, b_vid);
        }
        for (ptr, virtual_vertex) in perfect_matching.virtual_matchings.iter() {
            let a_vid = {
                let node = ptr.read_recursive();
                if let DualNodeClass::DefectVertex { defect_index } = &node.class {
                    *defect_index
                } else {
                    unreachable!("can only be syndrome")
                }
            };
            self.add_matching(a_vid, *virtual_vertex);
        }
    }

    pub fn load_subgraph(&mut self, subgraph: &[EdgeIndex]) {
        self.subgraph.clear();
        self.subgraph.extend(subgraph);
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
    #[allow(clippy::unnecessary_cast)]
    pub fn total_weight(&self) -> Weight {
        let mut weight = 0;
        for edge_index in self.subgraph.iter() {
            weight += self.complete_graph.weighted_edges[*edge_index as usize].2;
        }
        weight
    }

    /// get subgraph as a vec
    pub fn get_subgraph(&self) -> Vec<EdgeIndex> {
        self.subgraph.iter().copied().collect()
    }
}

/// to visualize subgraph
pub struct VisualizeSubgraph<'a> {
    pub subgraph: &'a Vec<EdgeIndex>,
}

impl<'a> VisualizeSubgraph<'a> {
    pub fn new(subgraph: &'a Vec<EdgeIndex>) -> Self {
        Self { subgraph }
    }
}

impl FusionVisualizer for VisualizeSubgraph<'_> {
    fn snapshot(&self, _abbrev: bool) -> serde_json::Value {
        json!({
            "subgraph": self.subgraph,
        })
    }
}

#[cfg(feature = "python_binding")]
#[pyfunction]
pub(crate) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<IntermediateMatching>()?;
    m.add_class::<PerfectMatching>()?;
    Ok(())
}
