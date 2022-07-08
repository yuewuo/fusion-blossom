//! Serial Dual Module
//! 
//! A serial implementation of the dual module. This is the very basic fusion blossom algorithm that aims at debugging and as a ground truth
//! where traditional matching is too time consuming because of their |E| = O(|V|^2) scaling.
//! 
//! This implementation supports fast clear: optimized for a small number of syndrome and small cluster coverage, the ``clear growth'' operator
//! can be executed in O(1) time, at the cost of dynamic check and dynamic reset. This also increases cache coherency, because a global clear
//! operation is unfriendly to cache.
//!

use super::util::*;
use crate::derivative::Derivative;
use std::sync::Arc;
use crate::parking_lot::RwLock;
use super::dual_module::*;
use super::visualize::*;


pub struct DualModuleSerial {
    /// all vertices including virtual ones
    pub vertices: Vec<VertexPtr>,
    /// nodes internal information
    pub nodes: Vec<Option<DualNodeInternalPtr>>,
    /// keep edges, which can also be accessed in [`Self::vertices`]
    pub edges: Vec<EdgePtr>,
    /// current timestamp
    pub active_timestamp: usize,
    /// bias of vertex index, useful when partitioning the decoding graph into multiple [`DualModuleSerial`]
    pub vertex_index_bias: usize,

    // TODO: maintain an active list to optimize for average cases: most errors have already been matched, and we only need to work on a few remained
}

pub type DualNodeInternalPtr = Arc<RwLock<DualNodeInternal>>;

/// internal information of the dual node, added to the [`DualNode`]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNodeInternal {
    /// the pointer to the origin [`DualNode`]
    pub origin: DualNodePtr,
    /// local index, to find myself in [`DualModuleSerial::nodes`]
    index: NodeIndex,
    /// dual variable of this node
    pub dual_variable: Weight,
    /// edges on the boundary of this node, (`is_left`, `edge`)
    pub boundary: Vec<(bool, EdgePtr)>,
}

pub type VertexPtr = Arc<RwLock<Vertex>>;
pub type EdgePtr = Arc<RwLock<Edge>>;

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Vertex {
    /// the index of this vertex in the decoding graph, not necessary the index in [`DualModule::vertices`] if it's partitioned
    pub index: VertexIndex,
    /// if a vertex is virtual, then it can be matched any times
    pub is_virtual: bool,
    /// all neighbor edges, in surface code this should be constant number of edges
    #[derivative(Debug="ignore")]
    pub edges: Vec<EdgePtr>,
    /// propagated dual node
    pub propagated_dual_node: Option<DualNodeInternalPtr>,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Edge {
    /// total weight of this edge
    pub weight: Weight,
    /// left vertex (always with smaller index for consistency)
    #[derivative(Debug="ignore")]
    pub left: VertexPtr,
    /// right vertex (always with larger index for consistency)
    #[derivative(Debug="ignore")]
    pub right: VertexPtr,
    /// growth from the left point
    pub left_growth: Weight,
    /// growth from the right point
    pub right_growth: Weight,
    /// left active tree node (if applicable)
    #[derivative(Debug="ignore")]
    pub left_dual_node: Option<DualNodeInternalPtr>,
    /// right active tree node (if applicable)
    #[derivative(Debug="ignore")]
    pub right_dual_node: Option<DualNodeInternalPtr>,
    /// for fast clear
    pub timestamp: usize,
}

impl DualModuleImpl for DualModuleSerial {

    /// initialize the dual module, which is supposed to be reused for multiple decoding tasks with the same structure
    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self {
        // create vertices
        let vertices: Vec<VertexPtr> = (0..vertex_num).map(|vertex_index| Arc::new(RwLock::new(Vertex {
            index: vertex_index,
            is_virtual: false,
            edges: Vec::new(),
            propagated_dual_node: None,
        }))).collect();
        // set virtual vertices
        for &virtual_vertex in virtual_vertices.iter() {
            let mut vertex = vertices[virtual_vertex].write();
            vertex.is_virtual = true;
        }
        // set edges
        let mut edges = Vec::<EdgePtr>::new();
        for &(i, j, weight) in weighted_edges.iter() {
            assert_ne!(i, j, "invalid edge from and to the same vertex {}", i);
            assert!(i < vertex_num, "edge ({}, {}) connected to an invalid vertex {}", i, j, i);
            assert!(j < vertex_num, "edge ({}, {}) connected to an invalid vertex {}", i, j, j);
            let left = usize::min(i, j);
            let right = usize::max(i, j);
            let edge = Arc::new(RwLock::new(Edge {
                weight: weight,
                left: Arc::clone(&vertices[left]),
                right: Arc::clone(&vertices[right]),
                left_growth: 0,
                right_growth: 0,
                left_dual_node: None,
                right_dual_node: None,
                timestamp: 0,
            }));
            for (a, b) in [(i, j), (j, i)] {
                let mut vertex = vertices[a].write();
                debug_assert!({  // O(N^2) sanity check, debug mode only (actually this bug is not critical, only the shorter edge will take effect)
                    let mut no_duplicate = true;
                    for edge in vertex.edges.iter() {
                        let edge = edge.read_recursive();
                        if Arc::ptr_eq(&edge.left, &vertices[b]) || Arc::ptr_eq(&edge.right, &vertices[b]) {
                            no_duplicate = false;
                            eprintln!("duplicated edge between {} and {} with weight w1 = {} and w2 = {}, consider merge them into a single edge", i, j, weight, edge.weight);
                            break
                        }
                    }
                    no_duplicate
                });
                vertex.edges.push(Arc::clone(&edge));
            }
            edges.push(edge);
        }
        Self {
            vertices: vertices,
            nodes: Vec::new(),
            edges: edges,
            active_timestamp: 0,
            vertex_index_bias: 0,
        }
    }

    /// clear all growth and existing dual nodes
    fn clear(&mut self) {
        self.clear_growth();
        self.nodes.clear();
    }

    /// add a new dual node from dual module root
    fn add_dual_node(&mut self, dual_node_ptr: DualNodePtr) {
        let mut node = dual_node_ptr.write();
        assert!(node.internal.is_none(), "dual node has already been created, do not call twice");
        let node_internal_ptr = Arc::new(RwLock::new(DualNodeInternal {
            origin: Arc::clone(&dual_node_ptr),
            dual_variable: 0,
            boundary: Vec::new(),
            index: self.nodes.len(),
        }));
        {
            let boundary = &mut node_internal_ptr.write().boundary;
            match &node.class {
                DualNodeClass::Blossom { nodes_circle } => {
                    // copy all the boundary edges and modify edge belongings
                    for dual_node_ptr in nodes_circle.iter() {
                        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
                        let dual_node_internal = dual_node_internal_ptr.read_recursive();
                        for (is_left, edge_ptr) in dual_node_internal.boundary.iter() {
                            boundary.push((*is_left, Arc::clone(edge_ptr)));
                            let mut edge = edge_ptr.write();
                            assert!(if *is_left { edge.left_dual_node.is_some() } else { edge.right_dual_node.is_some() }, "dual node of edge should be some");
                            if *is_left {
                                edge.left_dual_node = Some(Arc::clone(&node_internal_ptr));
                            } else {
                                edge.right_dual_node = Some(Arc::clone(&node_internal_ptr));
                            }
                        }
                    }
                },
                DualNodeClass::SyndromeVertex { syndrome_index } => {
                    assert!(*syndrome_index >= self.vertex_index_bias, "syndrome not belonging to this dual module");
                    let vertex_idx = syndrome_index - self.vertex_index_bias;
                    assert!(vertex_idx < self.vertices.len(), "syndrome not belonging to this dual module");
                    let vertex_ptr = &self.vertices[vertex_idx];
                    let vertex = vertex_ptr.read_recursive();
                    for edge_ptr in vertex.edges.iter() {
                        let mut edge = edge_ptr.write();
                        let is_left = Arc::ptr_eq(vertex_ptr, &edge.left);
                        assert!(if is_left { edge.left_dual_node.is_none() } else { edge.right_dual_node.is_none() }, "dual node of edge should be none");
                        if is_left {
                            edge.left_dual_node = Some(Arc::clone(&node_internal_ptr));
                        } else {
                            edge.right_dual_node = Some(Arc::clone(&node_internal_ptr));
                        }
                        boundary.push((is_left, Arc::clone(edge_ptr)));
                    }
                },
            }
        }
        node.internal = Some(self.nodes.len());
        self.nodes.push(Some(node_internal_ptr));
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        self.prepare_dual_node_growth(&dual_node_ptr, false);  // prepare the blossom into shrinking
        let node = dual_node_ptr.read_recursive();
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        assert_eq!(dual_node_internal.dual_variable, 0, "only blossom with dual variable = 0 can be safely removed");
        let node_idx = dual_node_internal.index;
        assert!(self.nodes[node_idx].is_some(), "blossom may have already been removed, do not call twice");
        assert!(Arc::ptr_eq(self.nodes[node_idx].as_ref().unwrap(), &dual_node_internal_ptr), "the blossom doesn't belong to this DualModuleRoot");
        self.nodes[node_idx] = None;  // simply remove this blossom node
        // recover edge belongings
        for (is_left, edge_ptr) in dual_node_internal.boundary.iter() {
            let mut edge = edge_ptr.write();
            assert!(if *is_left { edge.left_dual_node.is_some() } else { edge.right_dual_node.is_some() }, "dual node of edge should be some");
            if *is_left {
                edge.left_dual_node = None;
            } else {
                edge.right_dual_node = None;
            }
        }
        if let DualNodeClass::Blossom{ nodes_circle } = &node.class {
            for circle_dual_node_ptr in nodes_circle.iter() {
                let circle_dual_node_internal_ptr = self.get_dual_node_internal_ptr(&circle_dual_node_ptr);
                let circle_dual_node_internal = circle_dual_node_internal_ptr.read_recursive();
                for (is_left, edge_ptr) in circle_dual_node_internal.boundary.iter() {
                    let mut edge = edge_ptr.write();
                    assert!(if *is_left { edge.left_dual_node.is_none() } else { edge.right_dual_node.is_none() }, "dual node of edge should be none");
                    if *is_left {
                        edge.left_dual_node = Some(Arc::clone(&circle_dual_node_internal_ptr));
                    } else {
                        edge.right_dual_node = Some(Arc::clone(&circle_dual_node_internal_ptr));
                    }
                }
            }
        } else {
            unreachable!()
        }
    }

    fn compute_maximum_update_length_dual_node(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        if !simultaneous_update {
            // when `simultaneous_update` is set, it's assumed that all nodes are prepared to grow or shrink
            // this is because if we dynamically prepare them, it would be inefficient
            self.prepare_dual_node_growth(dual_node_ptr, is_grow);
        }
        let mut max_length_abs = Weight::MAX;
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        for (is_left, edge_ptr) in dual_node_internal.boundary.iter() {
            let is_left = *is_left;
            let edge = edge_ptr.read_recursive();
            if is_grow {
                // first check if both side belongs to the same tree node, if so, no constraint on this edge
                let peer_dual_node_internal_ptr: Option<DualNodeInternalPtr> = if is_left {
                    edge.right_dual_node.as_ref().map(|ptr| Arc::clone(ptr))
                } else {
                    edge.left_dual_node.as_ref().map(|ptr| Arc::clone(ptr))
                };
                match peer_dual_node_internal_ptr {
                    Some(peer_dual_node_internal_ptr) => {
                        if Arc::ptr_eq(&peer_dual_node_internal_ptr, &dual_node_internal_ptr) {
                            continue
                        } else {
                            let peer_dual_node_internal = peer_dual_node_internal_ptr.read_recursive();
                            let peer_dual_node_ptr = &peer_dual_node_internal.origin;
                            let peer_dual_node = peer_dual_node_ptr.read_recursive();
                            let remaining_length = edge.weight - edge.left_growth - edge.right_growth;
                            let local_max_length_abs = match peer_dual_node.grow_state {
                                DualNodeGrowState::Grow => {
                                    assert!(remaining_length % 2 == 0, "there is odd gap between two growing nodes, please make sure all weights are even numbers");
                                    remaining_length / 2
                                },
                                DualNodeGrowState::Shrink => { continue },  // shrinking node will never cause 
                                DualNodeGrowState::Stay => { remaining_length }
                            };
                            if local_max_length_abs == 0 {
                                return MaxUpdateLength::Conflicting(Arc::clone(&peer_dual_node_ptr), Arc::clone(&dual_node_ptr));
                            }
                            max_length_abs = std::cmp::min(max_length_abs, local_max_length_abs);
                        }
                    },
                    None => {
                        let local_max_length_abs = edge.weight - edge.left_growth - edge.right_growth;
                        if local_max_length_abs == 0 {
                            return MaxUpdateLength::Unimplemented;
                        }
                        max_length_abs = std::cmp::min(max_length_abs, local_max_length_abs);
                    },
                }
            } else {
                if is_left {
                    if edge.left_growth == 0 {  // TODO: check blossom non-negative
                        return MaxUpdateLength::Unimplemented;
                    }
                    max_length_abs = std::cmp::min(max_length_abs, edge.left_growth);
                } else {
                    if edge.right_growth == 0 {
                        return MaxUpdateLength::Unimplemented;
                    }
                    max_length_abs = std::cmp::min(max_length_abs, edge.right_growth);
                }
            }
        }
        MaxUpdateLength::NonZeroGrow(max_length_abs)
    }

    fn compute_maximum_update_length(&mut self) -> MaxUpdateLength {
        // first prepare all nodes for individual grow or shrink; Stay nodes will be prepared to shrink in order to minimize effect on others
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                match self.nodes[i].as_ref() {
                    Some(internal_dual_node_ptr) => {
                        let dual_node_internal = internal_dual_node_ptr.read_recursive();
                        Arc::clone(&dual_node_internal.origin)
                    },
                    _ => { continue }
                }
            };
            let dual_node = dual_node_ptr.read_recursive();
            match dual_node.grow_state {
                DualNodeGrowState::Grow => { self.prepare_dual_node_growth(&dual_node_ptr, true); },
                DualNodeGrowState::Shrink | DualNodeGrowState::Stay => { self.prepare_dual_node_growth(&dual_node_ptr, false); },
            };
        }
        let mut max_update_length = MaxUpdateLength::NoMoreNodes;
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                match self.nodes[i].as_ref() {
                    Some(internal_dual_node_ptr) => {
                        let dual_node_internal = internal_dual_node_ptr.read_recursive();
                        Arc::clone(&dual_node_internal.origin)
                    },
                    _ => { continue }
                }
            };
            let dual_node = dual_node_ptr.read_recursive();
            let is_grow = match dual_node.grow_state {
                DualNodeGrowState::Grow => true,
                DualNodeGrowState::Shrink => false,
                DualNodeGrowState::Stay => { continue }
            };
            let local_max_update_length = self.compute_maximum_update_length_dual_node(&dual_node_ptr, is_grow, true);
            max_update_length = MaxUpdateLength::min(max_update_length, local_max_update_length);
        }
        max_update_length
    }

    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        if length == 0 {
            eprintln!("[warning] calling `grow_dual_node` with zero length, nothing to do");
            return
        }
        self.prepare_dual_node_growth(dual_node_ptr, length > 0);
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        {  // update node dual variable and do sanity check
            let mut dual_node_internal = dual_node_internal_ptr.write();
            dual_node_internal.dual_variable += length;
            assert!(dual_node_internal.dual_variable >= 0, "shrinking to negative dual variable is forbidden");
        }
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        for (is_left, edge_ptr) in dual_node_internal.boundary.iter() {
            let is_left = *is_left;
            let (growth, weight) = {  // minimize writer lock acquisition
                let mut edge = edge_ptr.write();
                if is_left {
                    edge.left_growth += length;
                    assert!(edge.left_growth >= 0, "negative growth forbidden");
                } else {
                    edge.right_growth += length;
                    assert!(edge.right_growth >= 0, "negative growth forbidden");
                }
                (edge.left_growth + edge.right_growth, edge.weight)
            };
            let edge = edge_ptr.read_recursive();
            if growth > weight {
                // first check for if both side belongs to the same dual node, if so, it's ok
                let dual_node_internal_ptr_2: &Option<DualNodeInternalPtr> = if is_left {
                    &edge.right_dual_node
                } else {
                    &edge.left_dual_node
                };
                if dual_node_internal_ptr_2.is_none() || !Arc::ptr_eq(dual_node_internal_ptr_2.as_ref().unwrap(), &dual_node_internal_ptr) {
                    panic!("over-grown edge ({},{}): {}/{}", edge.left.read_recursive().index, edge.right.read_recursive().index, growth, weight);
                }
            } else if growth < 0 {
                panic!("under-grown edge ({},{}): {}/{}", edge.left.read_recursive().index, edge.right.read_recursive().index, growth, weight);
            }
        }
    }

    fn grow(&mut self, length: Weight) {
        assert!(length > 0, "only positive growth is supported");
        // first handle shrinks and then grow, to make sure they don't conflict
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                if let Some(node) = self.nodes[i].as_ref() {
                    let dual_node_internal = node.read_recursive();
                    Arc::clone(&dual_node_internal.origin)
                } else { continue }
            };
            let dual_node = dual_node_ptr.read_recursive();
            if matches!(dual_node.grow_state, DualNodeGrowState::Shrink) {
                self.grow_dual_node(&dual_node_ptr, -length);
            }
        }
        // then grow those needed
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                if let Some(node) = self.nodes[i].as_ref() {
                    let dual_node_internal = node.read_recursive();
                    Arc::clone(&dual_node_internal.origin)
                } else { continue }
            };
            let dual_node = dual_node_ptr.read_recursive();
            if matches!(dual_node.grow_state, DualNodeGrowState::Grow) {
                self.grow_dual_node(&dual_node_ptr, length);
            }
        }
    }

}

/*
Implementing fast clear operations
*/

impl Edge {

    /// dynamic clear edge
    pub fn dynamic_clear(&mut self, active_timestamp: usize) {
        if self.timestamp != active_timestamp {
            self.left_growth = 0;
            self.right_growth = 0;
            self.timestamp = active_timestamp;
        }
    }

}

impl DualModuleSerial {

    /// hard clear all growth (manual call not recommended)
    pub fn hard_clear_growth(&mut self) {
        for edge in self.edges.iter() {
            let mut edge = edge.write();
            edge.left_growth = 0;
            edge.right_growth = 0;
            edge.timestamp = 0;
        }
        self.active_timestamp = 0;
    }

    /// soft clear all growth
    pub fn clear_growth(&mut self) {
        if self.active_timestamp == usize::MAX {  // rarely happens
            self.hard_clear_growth();
        }
        self.active_timestamp += 1;  // implicitly clear all edges growth
    }

}

/*
Implementing visualization functions
*/

impl FusionVisualizer for DualModuleSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut vertices = Vec::<serde_json::Value>::new();
        for vertex in self.vertices.iter() {
            let vertex = vertex.read_recursive();
            vertices.push(json!({
                if abbrev { "v" } else { "is_virtual" }: if vertex.is_virtual { 1 } else { 0 },
            }));
        }
        let mut edges = Vec::<serde_json::Value>::new();
        for edge in self.edges.iter() {
            let edge = edge.read_recursive();
            edges.push(json!({
                if abbrev { "w" } else { "weight" }: edge.weight,
                if abbrev { "l" } else { "left" }: edge.left.read_recursive().index,
                if abbrev { "r" } else { "right" }: edge.right.read_recursive().index,
                if abbrev { "lg" } else { "left_growth" }: edge.left_growth,
                if abbrev { "rg" } else { "right_growth" }: edge.right_growth,
            }));
        }
        json!({
            "nodes": vertices,  // TODO: update HTML code to use the same language
            "edges": edges,
            "tree_nodes": [],
        })
    }
}

/*
Implement internal helper functions that maintains the state of dual clusters
*/

impl DualModuleSerial {

    pub fn get_dual_node_internal_ptr(&self, dual_node_ptr: &DualNodePtr) -> DualNodeInternalPtr {
        let dual_node = dual_node_ptr.read_recursive();
        let dual_node_idx = dual_node.internal.as_ref().expect("must first register the dual node using `create_dual_node`");
        let dual_node_internal_ptr = self.nodes[*dual_node_idx].as_ref().expect("internal dual node must exists");
        debug_assert!(Arc::ptr_eq(&dual_node_ptr, &dual_node_internal_ptr.read_recursive().origin), "dual node and dual internal node must corresponds to each other");
        Arc::clone(&dual_node_internal_ptr)
    }

    /// adjust the boundary of each dual node to fit into the need of growing (`length` > 0) or shrinking (`length` < 0)
    pub fn prepare_dual_node_growth(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool) {
        let mut updated_boundary = Vec::<(bool, EdgePtr)>::new();
        let mut propagating_vertices = Vec::<VertexPtr>::new();
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        if is_grow {  // gracefully update the boundary to ease growing
            let dual_node_internal = dual_node_internal_ptr.read_recursive();
            for (is_left, edge_ptr) in dual_node_internal.boundary.iter() {
                let is_left = *is_left;
                let edge = edge_ptr.read_recursive();
                let peer_dual_node: &Option<DualNodeInternalPtr> = if is_left {
                    &edge.right_dual_node
                } else {
                    &edge.left_dual_node
                };
                if edge.left_growth + edge.right_growth == edge.weight && peer_dual_node.is_none() {
                    // need to propagate to a new node
                    let peer_vertex_ptr = if is_left {
                        Arc::clone(&edge.right)
                    } else {
                        Arc::clone(&edge.left)
                    };
                    // to avoid already occupied node being propagated
                    assert!(peer_vertex_ptr.read_recursive().propagated_dual_node.is_none(), "growing into another propagated vertex forbidden");
                    propagating_vertices.push(peer_vertex_ptr);
                } else {  // keep other edges
                    updated_boundary.push((is_left, Arc::clone(edge_ptr)));
                }
            }
            // propagating nodes may be duplicated, but it's easy to check by `propagated_tree_node`
            for node_ptr in propagating_vertices.iter() {
                let mut node = node_ptr.write();
                if node.propagated_dual_node.is_none() {
                    node.propagated_dual_node = Some(Arc::clone(&dual_node_internal_ptr));
                    for edge_ptr in node.edges.iter() {
                        let (is_left, newly_propagated_edge) = {
                            let edge = edge_ptr.read_recursive();
                            let is_left = Arc::ptr_eq(node_ptr, &edge.left);
                            let not_fully_grown = edge.left_growth + edge.right_growth < edge.weight;
                            let newly_propagated_edge = not_fully_grown && if is_left {
                                edge.left_dual_node.is_none()
                            } else {
                                edge.right_dual_node.is_none()
                            };
                            (is_left, newly_propagated_edge)
                        };
                        if newly_propagated_edge {
                            updated_boundary.push((is_left, Arc::clone(edge_ptr)));
                            let mut edge = edge_ptr.write();
                            if is_left {
                                edge.left_dual_node = Some(Arc::clone(&dual_node_internal_ptr));
                            } else {
                                edge.right_dual_node = Some(Arc::clone(&dual_node_internal_ptr));
                            };
                        }
                    }
                }
            }
        } else {  // gracefully update the boundary to ease shrinking
            let dual_node_internal = dual_node_internal_ptr.read_recursive();
            for (is_left, edge_ptr) in dual_node_internal.boundary.iter() {
                let is_left = *is_left;
                let edge = edge_ptr.read_recursive();
                let this_growth = if is_left {
                    edge.left_growth
                } else {
                    edge.right_growth
                };
                if this_growth == 0 {
                    // need to shrink before this vertex
                    let this_node = if is_left {
                        Arc::clone(&edge.left)
                    } else {
                        Arc::clone(&edge.right)
                    };
                    // to avoid already occupied node being propagated
                    assert!(this_node.read_recursive().propagated_dual_node.is_some(), "unexpected shrink into an empty node");
                    propagating_vertices.push(this_node);
                } else {  // keep other edges
                    updated_boundary.push((is_left, Arc::clone(edge_ptr)));
                }
            }
            // propagating nodes may be duplicated, but it's easy to check by `propagated_tree_node`
            for node_ptr in propagating_vertices.iter() {
                let mut node = node_ptr.write();
                if node.propagated_dual_node.is_some() {
                    node.propagated_dual_node = None;
                    for edge_ptr in node.edges.iter() {
                        let (is_left, newly_propagated_edge) = {
                            let edge = edge_ptr.read_recursive();
                            let is_left = Arc::ptr_eq(node_ptr, &edge.left);
                            // fully grown edge is where to shrink
                            let newly_propagated_edge = edge.left_growth + edge.right_growth == edge.weight;
                            (is_left, newly_propagated_edge)
                        };
                        if newly_propagated_edge {
                            updated_boundary.push((!is_left, Arc::clone(edge_ptr)));
                            let edge = edge_ptr.read_recursive();
                            if is_left {
                                assert!(edge.right_dual_node.is_some(), "unexpected shrinking to empty edge");
                                assert!(Arc::ptr_eq(edge.right_dual_node.as_ref().unwrap(), &dual_node_internal_ptr), "shrinking edge should be same tree node");
                            } else {
                                assert!(edge.left_dual_node.is_some(), "unexpected shrinking to empty edge");
                                assert!(Arc::ptr_eq(edge.left_dual_node.as_ref().unwrap(), &dual_node_internal_ptr), "shrinking edge should be same tree node");
                            };
                        } else {
                            let mut edge = edge_ptr.write();
                            if is_left {
                                edge.left_dual_node = None;
                            } else {
                                edge.right_dual_node = None;
                            };
                        }
                    }
                }
            }
        }
        // update the boundary
        let mut dual_node_internal = dual_node_internal_ptr.write();
        std::mem::swap(&mut updated_boundary, &mut dual_node_internal.boundary);
        // println!("{} boundary: {:?}", tree_node.boundary.len(), tree_node.boundary);
        assert!(dual_node_internal.boundary.len() > 0, "the boundary of a dual cluster is never empty");
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::example::*;

    #[test]
    fn dual_module_serial_basics() {  // cargo test dual_module_serial_basics -- --nocapture
        let visualize_filename = format!("dual_module_serial_basics.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[19].is_syndrome = true;
        code.vertices[25].is_syndrome = true;
        let syndrome = code.get_syndrome();
        visualizer.snapshot_combined(format!("syndrome"), vec![&code, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let root = DualModuleRoot::new(&syndrome, &mut dual_module);
        let dual_node_19_ptr = Arc::clone(root.nodes[0].as_ref().unwrap());
        let dual_node_25_ptr = Arc::clone(root.nodes[1].as_ref().unwrap());
        dual_module.grow_dual_node(&dual_node_19_ptr, half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, half_weight);
        visualizer.snapshot_combined(format!("grow to 0.5"), vec![&code, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, half_weight);
        visualizer.snapshot_combined(format!("grow to 1"), vec![&code, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, half_weight);
        visualizer.snapshot_combined(format!("grow to 1.5"), vec![&code, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, -half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, -half_weight);
        visualizer.snapshot_combined(format!("shrink to 1"), vec![&code, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, -half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, -half_weight);
        visualizer.snapshot_combined(format!("shrink to 0.5"), vec![&code, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, -half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, -half_weight);
        visualizer.snapshot_combined(format!("shrink to 0"), vec![&code, &dual_module]).unwrap();
    }

    #[test]
    fn dual_module_serial_blossom_basics() {  // cargo test dual_module_serial_blossom_basics -- --nocapture
        let visualize_filename = format!("dual_module_serial_blossom_basics.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[19].is_syndrome = true;
        code.vertices[26].is_syndrome = true;
        code.vertices[35].is_syndrome = true;
        let syndrome = code.get_syndrome();
        visualizer.snapshot_combined(format!("syndrome"), vec![&code, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let mut root = DualModuleRoot::new(&syndrome, &mut dual_module);
        let dual_node_19_ptr = Arc::clone(root.nodes[0].as_ref().unwrap());
        let dual_node_26_ptr = Arc::clone(root.nodes[1].as_ref().unwrap());
        let dual_node_35_ptr = Arc::clone(root.nodes[2].as_ref().unwrap());
        dual_module.grow(2 * half_weight);
        visualizer.snapshot_combined(format!("before create blossom"), vec![&code, &dual_module]).unwrap();
        let nodes_circle = vec![Arc::clone(&dual_node_19_ptr), Arc::clone(&dual_node_26_ptr), Arc::clone(&dual_node_35_ptr)];
        set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink);
        let dual_node_blossom = root.create_blossom(nodes_circle, &mut dual_module);
        dual_module.grow(half_weight);
        visualizer.snapshot_combined(format!("blossom grow half weight"), vec![&code, &dual_module]).unwrap();
        dual_module.grow(half_weight);
        visualizer.snapshot_combined(format!("blossom grow half weight"), vec![&code, &dual_module]).unwrap();
        dual_module.grow(half_weight);
        visualizer.snapshot_combined(format!("blossom grow half weight"), vec![&code, &dual_module]).unwrap();
        set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink);
        dual_module.grow(half_weight);
        visualizer.snapshot_combined(format!("blossom shrink half weight"), vec![&code, &dual_module]).unwrap();
        dual_module.grow(2 * half_weight);
        visualizer.snapshot_combined(format!("blossom shrink weight"), vec![&code, &dual_module]).unwrap();
        root.expand_blossom(dual_node_blossom, &mut dual_module);
        set_grow_state(&dual_node_19_ptr, DualNodeGrowState::Shrink);
        set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink);
        set_grow_state(&dual_node_35_ptr, DualNodeGrowState::Shrink);
        dual_module.grow(half_weight);
        visualizer.snapshot_combined(format!("individual shrink half weight"), vec![&code, &dual_module]).unwrap();
    }

    #[test]
    fn dual_module_serial_stop_reason_1() {  // cargo test dual_module_serial_stop_reason_1 -- --nocapture
        let visualize_filename = format!("dual_module_serial_stop_reason_1.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[19].is_syndrome = true;
        code.vertices[25].is_syndrome = true;
        let syndrome = code.get_syndrome();
        visualizer.snapshot_combined(format!("syndrome"), vec![&code, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let root = DualModuleRoot::new(&syndrome, &mut dual_module);
        let dual_node_19_ptr = Arc::clone(root.nodes[0].as_ref().unwrap());
        let dual_node_25_ptr = Arc::clone(root.nodes[1].as_ref().unwrap());
        // grow the maximum
        let max_update_length = dual_module.compute_maximum_update_length();
        if let MaxUpdateLength::NonZeroGrow(length) = &max_update_length {
            assert_eq!(*length, 2 * half_weight);
            dual_module.grow(*length);
        } else {
            panic!("unexpected max update length: {:?}", max_update_length)
        }
        visualizer.snapshot_combined(format!("grow"), vec![&code, &dual_module]).unwrap();
        // grow the maximum
        let max_update_length = dual_module.compute_maximum_update_length();
        if let MaxUpdateLength::NonZeroGrow(length) = &max_update_length {
            assert_eq!(*length, half_weight);
            dual_module.grow(*length);
        } else {
            panic!("unexpected max update length: {:?}", max_update_length)
        }
        visualizer.snapshot_combined(format!("grow"), vec![&code, &dual_module]).unwrap();
        // cannot grow anymore, find out the reason
        let max_update_length = dual_module.compute_maximum_update_length();
        println!("max_update_length: {:?}", max_update_length);
        assert!(max_update_length.is_conflicting(&dual_node_19_ptr, &dual_node_25_ptr), "unexpected max update length: {:?}", max_update_length);
    }

}
