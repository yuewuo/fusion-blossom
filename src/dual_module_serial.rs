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
use std::sync::{Arc, Weak};
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
    pub active_timestamp: FastClearTimestamp,
    /// bias of vertex index, useful when partitioning the decoding graph into multiple [`DualModuleSerial`]
    pub vertex_index_bias: usize,

    // TODO: maintain an active list to optimize for average cases: most syndrome vertices have already been matched, and we only need to work on a few remained
}

pub struct DualNodeInternalPtr { ptr: Arc<RwLock<DualNodeInternal>>, }
pub struct DualNodeInternalWeak { ptr: Weak<RwLock<DualNodeInternal>>, }

impl DualNodeInternalPtr { pub fn downgrade(&self) -> DualNodeInternalWeak { DualNodeInternalWeak { ptr: Arc::downgrade(&self.ptr) } } }
impl DualNodeInternalWeak { pub fn upgrade_force(&self) -> DualNodeInternalPtr { DualNodeInternalPtr { ptr: self.ptr.upgrade().unwrap() } } }

impl Clone for DualNodeInternalPtr {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl RwLockPtr<DualNodeInternal> for DualNodeInternalPtr {
    fn new_ptr(ptr: Arc<RwLock<DualNodeInternal>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: DualNodeInternal) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<DualNodeInternal>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<DualNodeInternal>> { &mut self.ptr }
}

impl PartialEq for DualNodeInternalPtr {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl std::fmt::Debug for DualNodeInternalPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let dual_node_internal = self.read_recursive();
        write!(f, "{}", dual_node_internal.index)
    }
}

impl Clone for DualNodeInternalWeak {
    fn clone(&self) -> Self {
       Self { ptr: self.ptr.clone() }
    }
}

impl std::fmt::Debug for DualNodeInternalWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl PartialEq for DualNodeInternalWeak {
    fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
}

/// internal information of the dual node, added to the [`DualNode`]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNodeInternal {
    /// the pointer to the origin [`DualNode`]
    pub origin: DualNodeWeak,
    /// local index, to find myself in [`DualModuleSerial::nodes`]
    index: NodeIndex,
    /// dual variable of this node
    pub dual_variable: Weight,
    /// edges on the boundary of this node, (`is_left`, `edge`)
    pub boundary: Vec<(bool, EdgeWeak)>,
    /// over-grown vertices on the boundary of this node, this is to solve a bug where all surrounding edges are fully grown
    /// so all edges are deleted from the boundary... this will lose track of the real boundary when shrinking back
    pub overgrown_stack: Vec<(VertexWeak, Weight)>,
}

pub struct VertexPtr { ptr: Arc<RwLock<Vertex>> }
pub struct VertexWeak { ptr: Weak<RwLock<Vertex>> }

impl VertexPtr { pub fn downgrade(&self) -> VertexWeak { VertexWeak { ptr: Arc::downgrade(&self.ptr) } } }
impl VertexWeak { pub fn upgrade_force(&self) -> VertexPtr { VertexPtr { ptr: self.ptr.upgrade().unwrap() } } }

impl Clone for VertexPtr {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl FastClearRwLockPtr<Vertex> for VertexPtr {
    fn new_ptr(ptr: Arc<RwLock<Vertex>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: Vertex) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<Vertex>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<Vertex>> { &mut self.ptr }
}

impl PartialEq for VertexPtr {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl std::fmt::Debug for VertexPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let vertex = self.read_recursive_force();
        write!(f, "{}", vertex.vertex_index)
    }
}

impl Clone for VertexWeak {
    fn clone(&self) -> Self {
       Self { ptr: self.ptr.clone() }
    }
}

impl std::fmt::Debug for VertexWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl PartialEq for VertexWeak {
    fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
}

pub struct EdgePtr { ptr: Arc<RwLock<Edge>> }
pub struct EdgeWeak { ptr: Weak<RwLock<Edge>> }

impl EdgePtr { pub fn downgrade(&self) -> EdgeWeak { EdgeWeak { ptr: Arc::downgrade(&self.ptr) } } }
impl EdgeWeak { pub fn upgrade_force(&self) -> EdgePtr { EdgePtr { ptr: self.ptr.upgrade().unwrap() } } }

impl Clone for EdgePtr {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl FastClearRwLockPtr<Edge> for EdgePtr {
    fn new_ptr(ptr: Arc<RwLock<Edge>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: Edge) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<Edge>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<Edge>> { &mut self.ptr }
}

impl PartialEq for EdgePtr {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl std::fmt::Debug for EdgePtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let edge = self.read_recursive_force();
        write!(f, "{}", edge.index)
    }
}

impl Clone for EdgeWeak {
    fn clone(&self) -> Self {
       Self { ptr: self.ptr.clone() }
    }
}

impl std::fmt::Debug for EdgeWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl PartialEq for EdgeWeak {
    fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Vertex {
    /// the index of this vertex in the decoding graph, not necessary the index in [`DualModuleSerial::vertices`] if it's partitioned
    pub vertex_index: VertexIndex,
    /// if a vertex is virtual, then it can be matched any times
    pub is_virtual: bool,
    /// if a vertex is syndrome, then [`Vertex::propagated_dual_node`] always corresponds to that root
    pub is_syndrome: bool,
    /// all neighbor edges, in surface code this should be constant number of edges
    #[derivative(Debug="ignore")]
    pub edges: Vec<EdgeWeak>,
    /// propagated dual node
    pub propagated_dual_node: Option<DualNodeInternalWeak>,
    /// propagated grandson node: must be a syndrome node
    pub propagated_grandson_dual_node: Option<DualNodeInternalWeak>,
    /// for fast clear
    pub timestamp: FastClearTimestamp,
}

#[derive(Derivative)]
#[derivative(Debug)]
pub struct Edge {
    /// local index, to find myself in [`DualModuleSerial::edges`]
    index: EdgeIndex,
    /// total weight of this edge
    pub weight: Weight,
    /// left vertex (always with smaller index for consistency)
    #[derivative(Debug="ignore")]
    pub left: VertexWeak,
    /// right vertex (always with larger index for consistency)
    #[derivative(Debug="ignore")]
    pub right: VertexWeak,
    /// growth from the left point
    pub left_growth: Weight,
    /// growth from the right point
    pub right_growth: Weight,
    /// left active tree node (if applicable)
    pub left_dual_node: Option<DualNodeInternalWeak>,
    /// left grandson node: must be a syndrome node
    pub left_grandson_dual_node: Option<DualNodeInternalWeak>,
    /// right active tree node (if applicable)
    pub right_dual_node: Option<DualNodeInternalWeak>,
    /// left grandson node: must be a syndrome node
    pub right_grandson_dual_node: Option<DualNodeInternalWeak>,
    /// for fast clear
    pub timestamp: FastClearTimestamp,
}

impl DualModuleImpl for DualModuleSerial {

    /// initialize the dual module, which is supposed to be reused for multiple decoding tasks with the same structure
    fn new(vertex_num: usize, weighted_edges: &Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: &Vec<VertexIndex>) -> Self {
        let active_timestamp = 0;
        // create vertices
        let vertices: Vec<VertexPtr> = (0..vertex_num).map(|vertex_index| VertexPtr::new(Vertex {
            vertex_index: vertex_index,
            is_virtual: false,
            is_syndrome: false,
            edges: Vec::new(),
            propagated_dual_node: None,
            propagated_grandson_dual_node: None,
            timestamp: active_timestamp,
        })).collect();
        // set virtual vertices
        for &virtual_vertex in virtual_vertices.iter() {
            let mut vertex = vertices[virtual_vertex].write(active_timestamp);
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
            let edge_ptr = EdgePtr::new(Edge {
                index: edges.len(),
                weight: weight,
                left: vertices[left].downgrade(),
                right: vertices[right].downgrade(),
                left_growth: 0,
                right_growth: 0,
                left_dual_node: None,
                left_grandson_dual_node: None,
                right_dual_node: None,
                right_grandson_dual_node: None,
                timestamp: 0,
            });
            for (a, b) in [(i, j), (j, i)] {
                let mut vertex = vertices[a].write(active_timestamp);
                debug_assert!({  // O(N^2) sanity check, debug mode only (actually this bug is not critical, only the shorter edge will take effect)
                    let mut no_duplicate = true;
                    for edge_weak in vertex.edges.iter() {
                        let edge_ptr = edge_weak.upgrade_force();
                        let edge = edge_ptr.read_recursive(active_timestamp);
                        if edge.left == vertices[b].downgrade() || edge.right == vertices[b].downgrade() {
                            no_duplicate = false;
                            eprintln!("duplicated edge between {} and {} with weight w1 = {} and w2 = {}, consider merge them into a single edge", i, j, weight, edge.weight);
                            break
                        }
                    }
                    no_duplicate
                });
                vertex.edges.push(edge_ptr.downgrade());
            }
            edges.push(edge_ptr);
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
        self.clear_graph();
        self.nodes.clear();
    }

    /// add a new dual node from dual module root
    fn add_dual_node(&mut self, dual_node_ptr: &DualNodePtr) {
        let active_timestamp = self.active_timestamp;
        let mut node = dual_node_ptr.write();
        assert!(node.internal.is_none(), "dual node has already been created, do not call twice");
        let node_internal_ptr = DualNodeInternalPtr::new(DualNodeInternal {
            origin: dual_node_ptr.downgrade(),
            index: self.nodes.len(),
            dual_variable: 0,
            boundary: Vec::new(),
            overgrown_stack: Vec::new(),
        });
        {
            let boundary = &mut node_internal_ptr.write().boundary;
            match &node.class {
                DualNodeClass::Blossom { nodes_circle } => {
                    // copy all the boundary edges and modify edge belongings
                    for dual_node_weak in nodes_circle.iter() {
                        let dual_node_ptr = dual_node_weak.upgrade_force();
                        self.prepare_dual_node_growth(&dual_node_ptr, false);  // prepare all nodes in shrinking mode for consistency
                        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
                        let dual_node_internal = dual_node_internal_ptr.read_recursive();
                        for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
                            let edge_ptr = edge_weak.upgrade_force();
                            boundary.push((*is_left, edge_weak.clone()));
                            let mut edge = edge_ptr.write(active_timestamp);
                            assert!(if *is_left { edge.left_dual_node.is_some() } else { edge.right_dual_node.is_some() }, "dual node of edge should be some");
                            debug_assert!(if *is_left { edge.left_dual_node == Some(dual_node_internal_ptr.downgrade())
                                } else { edge.right_dual_node == Some(dual_node_internal_ptr.downgrade()) }, "edge belonging");
                            if *is_left {
                                edge.left_dual_node = Some(node_internal_ptr.downgrade());
                            } else {
                                edge.right_dual_node = Some(node_internal_ptr.downgrade());
                            }
                        }
                    }
                },
                DualNodeClass::SyndromeVertex { syndrome_index } => {
                    assert!(*syndrome_index >= self.vertex_index_bias, "syndrome not belonging to this dual module");
                    let vertex_idx = syndrome_index - self.vertex_index_bias;
                    assert!(vertex_idx < self.vertices.len(), "syndrome not belonging to this dual module");
                    let vertex_ptr = &self.vertices[vertex_idx];
                    vertex_ptr.dynamic_clear(active_timestamp);
                    let mut vertex = vertex_ptr.write(active_timestamp);
                    vertex.propagated_dual_node = Some(node_internal_ptr.downgrade());
                    vertex.propagated_grandson_dual_node = Some(node_internal_ptr.downgrade());
                    vertex.is_syndrome = true;
                    for edge_weak in vertex.edges.iter() {
                        let edge_ptr = edge_weak.upgrade_force();
                        edge_ptr.dynamic_clear(active_timestamp);
                        let mut edge = edge_ptr.write(active_timestamp);
                        let is_left = vertex_ptr.downgrade() == edge.left;
                        assert!(if is_left { edge.left_dual_node.is_none() } else { edge.right_dual_node.is_none() }, "dual node of edge should be none");
                        if is_left {
                            edge.left_dual_node = Some(node_internal_ptr.downgrade());
                            edge.left_grandson_dual_node = Some(node_internal_ptr.downgrade());
                        } else {
                            edge.right_dual_node = Some(node_internal_ptr.downgrade());
                            edge.right_grandson_dual_node = Some(node_internal_ptr.downgrade());
                        }
                        boundary.push((is_left, edge_weak.clone()));
                    }
                },
            }
        }
        node.internal = Some(self.nodes.len());
        self.nodes.push(Some(node_internal_ptr));
    }

    fn remove_blossom(&mut self, dual_node_ptr: DualNodePtr) {
        let active_timestamp = self.active_timestamp;
        self.prepare_dual_node_growth(&dual_node_ptr, false);  // prepare the blossom into shrinking
        let node = dual_node_ptr.read_recursive();
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        assert_eq!(dual_node_internal.dual_variable, 0, "only blossom with dual variable = 0 can be safely removed");
        assert!(dual_node_internal.overgrown_stack.len() == 0, "removing a blossom with non-empty overgrown stack forbidden");
        let node_idx = dual_node_internal.index;
        assert!(self.nodes[node_idx].is_some(), "blossom may have already been removed, do not call twice");
        assert!(self.nodes[node_idx].as_ref().unwrap() == &dual_node_internal_ptr, "the blossom doesn't belong to this DualModuleInterface");
        self.nodes[node_idx] = None;  // simply remove this blossom node
        // recover edge belongings
        for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
            let edge_ptr = edge_weak.upgrade_force();
            let mut edge = edge_ptr.write(active_timestamp);
            assert!(if *is_left { edge.left_dual_node.is_some() } else { edge.right_dual_node.is_some() }, "dual node of edge should be some");
            if *is_left {
                edge.left_dual_node = None;
            } else {
                edge.right_dual_node = None;
            }
        }
        if let DualNodeClass::Blossom{ nodes_circle } = &node.class {
            for circle_dual_node_weak in nodes_circle.iter() {
                let circle_dual_node_ptr = circle_dual_node_weak.upgrade_force();
                let circle_dual_node_internal_ptr = self.get_dual_node_internal_ptr(&circle_dual_node_ptr);
                let circle_dual_node_internal = circle_dual_node_internal_ptr.read_recursive();
                for (is_left, edge_weak) in circle_dual_node_internal.boundary.iter() {
                    let edge_ptr = edge_weak.upgrade_force();
                    let mut edge = edge_ptr.write(active_timestamp);
                    assert!(if *is_left { edge.left_dual_node.is_none() } else { edge.right_dual_node.is_none() }, "dual node of edge should be none");
                    if *is_left {
                        edge.left_dual_node = Some(circle_dual_node_internal_ptr.downgrade());
                    } else {
                        edge.right_dual_node = Some(circle_dual_node_internal_ptr.downgrade());
                    }
                }
            }
        } else {
            unreachable!()
        }
    }

    fn set_grow_state(&mut self, _dual_node_ptr: &DualNodePtr, _grow_state: DualNodeGrowState) {
        // do nothing, we don't record grow state here
    }

    fn compute_maximum_update_length_dual_node(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool, simultaneous_update: bool) -> MaxUpdateLength {
        let active_timestamp = self.active_timestamp;
        if !simultaneous_update {
            // when `simultaneous_update` is set, it's assumed that all nodes are prepared to grow or shrink
            // this is because if we dynamically prepare them, it would be inefficient
            self.prepare_dual_node_growth(dual_node_ptr, is_grow);
        }
        let mut max_length_abs = Weight::MAX;
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        if !is_grow {
            if dual_node_internal.dual_variable == 0 {
                let dual_node = dual_node_ptr.read_recursive();
                match dual_node.class {
                    DualNodeClass::Blossom { .. } => { return MaxUpdateLength::BlossomNeedExpand(dual_node_ptr.downgrade()) }
                    DualNodeClass::SyndromeVertex { .. } => { return MaxUpdateLength::VertexShrinkStop(dual_node_ptr.downgrade()) }
                }
            }
            if dual_node_internal.overgrown_stack.len() > 0 {
                let last_index = dual_node_internal.overgrown_stack.len() - 1;
                let (_, overgrown) = &dual_node_internal.overgrown_stack[last_index];
                max_length_abs = std::cmp::min(max_length_abs, *overgrown);
            }
            max_length_abs = std::cmp::min(max_length_abs, dual_node_internal.dual_variable);
        }
        for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
            let edge_ptr = edge_weak.upgrade_force();
            let is_left = *is_left;
            let edge = edge_ptr.read_recursive(active_timestamp);
            if is_grow {
                // first check if both side belongs to the same tree node, if so, no constraint on this edge
                let peer_dual_node_internal_ptr: Option<DualNodeInternalPtr> = if is_left {
                    edge.right_dual_node.as_ref().map(|ptr| ptr.upgrade_force())
                } else {
                    edge.left_dual_node.as_ref().map(|ptr| ptr.upgrade_force())
                };
                match peer_dual_node_internal_ptr {
                    Some(peer_dual_node_internal_ptr) => {
                        if peer_dual_node_internal_ptr == dual_node_internal_ptr {
                            continue
                        } else {
                            let peer_dual_node_internal = peer_dual_node_internal_ptr.read_recursive();
                            let peer_dual_node_ptr = peer_dual_node_internal.origin.upgrade_force();
                            let peer_dual_node = peer_dual_node_ptr.read_recursive();
                            let remaining_length = edge.weight - edge.left_growth - edge.right_growth;
                            let local_max_length_abs = match peer_dual_node.grow_state {
                                DualNodeGrowState::Grow => {
                                    assert!(remaining_length % 2 == 0, "there is odd gap between two growing nodes, please make sure all weights are even numbers");
                                    remaining_length / 2
                                },
                                DualNodeGrowState::Shrink => {
                                    // special case: if peer is a syndrome vertex and it's dual variable is already 0, 
                                    // then we need to determine if some other growing nodes are conflicting with me
                                    if remaining_length == 0 && matches!(peer_dual_node.class, DualNodeClass::SyndromeVertex{ .. }) && peer_dual_node_internal.dual_variable == 0 {
                                        for (peer_is_left, peer_edge_weak) in peer_dual_node_internal.boundary.iter() {
                                            let peer_edge_ptr = peer_edge_weak.upgrade_force();
                                            let peer_edge = peer_edge_ptr.read_recursive(active_timestamp);
                                            if peer_edge.left_growth + peer_edge.right_growth == peer_edge.weight {
                                                let peer_is_left = *peer_is_left;
                                                let far_peer_dual_node_internal_ptr: Option<DualNodeInternalPtr> = if peer_is_left {
                                                    peer_edge.right_dual_node.as_ref().map(|ptr| ptr.upgrade_force())
                                                } else {
                                                    peer_edge.left_dual_node.as_ref().map(|ptr| ptr.upgrade_force())
                                                };
                                                match far_peer_dual_node_internal_ptr {
                                                    Some(far_peer_dual_node_internal_ptr) => {
                                                        let far_peer_dual_node_internal = far_peer_dual_node_internal_ptr.read_recursive();
                                                        let far_peer_dual_node_ptr = far_peer_dual_node_internal.origin.upgrade_force();
                                                        if &far_peer_dual_node_ptr != dual_node_ptr {
                                                            let far_peer_dual_node = far_peer_dual_node_ptr.read_recursive();
                                                            match far_peer_dual_node.grow_state {
                                                                DualNodeGrowState::Grow => {
                                                                    let far_peer_grandson_ptr = if peer_is_left {
                                                                        peer_edge.right_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                                                    } else {
                                                                        peer_edge.left_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                                                    };
                                                                    let grandson_ptr = if is_left {
                                                                        edge.left_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                                                    } else {
                                                                        edge.right_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                                                    };
                                                                    return MaxUpdateLength::Conflicting(
                                                                        (far_peer_dual_node_ptr.downgrade(), far_peer_grandson_ptr), 
                                                                        (dual_node_ptr.downgrade(), grandson_ptr)
                                                                    );
                                                                },
                                                                _ => { }
                                                            }
                                                        }
                                                    },
                                                    None => { }
                                                }
                                            }
                                        }
                                    }
                                    continue
                                },
                                DualNodeGrowState::Stay => { remaining_length }
                            };
                            if local_max_length_abs == 0 {
                                let peer_grandson_ptr = if is_left {
                                    edge.right_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                } else {
                                    edge.left_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                };
                                let grandson_ptr = if is_left {
                                    edge.left_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                } else {
                                    edge.right_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                };
                                return MaxUpdateLength::Conflicting(
                                    (peer_dual_node_ptr.downgrade(), peer_grandson_ptr), 
                                    (dual_node_ptr.downgrade(), grandson_ptr)
                                );
                            }
                            max_length_abs = std::cmp::min(max_length_abs, local_max_length_abs);
                        }
                    },
                    None => {
                        let local_max_length_abs = edge.weight - edge.left_growth - edge.right_growth;
                        if local_max_length_abs == 0 {
                            // check if peer is virtual node
                            let peer_vertex_ptr: VertexPtr = if is_left {
                                edge.right.upgrade_force()
                            } else {
                                edge.left.upgrade_force()
                            };
                            let peer_vertex = peer_vertex_ptr.read_recursive(active_timestamp);
                            if peer_vertex.is_virtual {
                                let grandson_ptr = if is_left {
                                    edge.left_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                } else {
                                    edge.right_grandson_dual_node.as_ref().map(|ptr| ptr.upgrade_force()).unwrap().read_recursive().origin.clone()
                                };
                                return MaxUpdateLength::TouchingVirtual((dual_node_ptr.downgrade(), grandson_ptr), peer_vertex.vertex_index);
                            } else {
                                println!("edge: {edge_ptr:?}, peer_vertex_ptr: {peer_vertex_ptr:?}");
                                unreachable!("this edge should've been removed from boundary because it's already fully grown, and it's peer vertex is not virtual")
                            }
                        }
                        max_length_abs = std::cmp::min(max_length_abs, local_max_length_abs);
                    },
                }
            } else {
                if is_left {
                    if edge.left_growth == 0 {
                        unreachable!()
                    }
                    max_length_abs = std::cmp::min(max_length_abs, edge.left_growth);
                } else {
                    if edge.right_growth == 0 {
                        unreachable!()
                    }
                    max_length_abs = std::cmp::min(max_length_abs, edge.right_growth);
                }
            }
        }
        MaxUpdateLength::NonZeroGrow(max_length_abs)
    }

    fn compute_maximum_update_length(&mut self) -> GroupMaxUpdateLength {
        // first prepare all nodes for individual grow or shrink; Stay nodes will be prepared to shrink in order to minimize effect on others
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                match self.nodes[i].as_ref() {
                    Some(internal_dual_node_ptr) => {
                        let dual_node_internal = internal_dual_node_ptr.read_recursive();
                        dual_node_internal.origin.upgrade_force()
                    },
                    _ => { continue }
                }
            };
            let dual_node = dual_node_ptr.read_recursive();
            match dual_node.grow_state {
                DualNodeGrowState::Grow => { },
                DualNodeGrowState::Shrink => { self.prepare_dual_node_growth(&dual_node_ptr, false); },
                DualNodeGrowState::Stay => { },  // do not touch, Stay nodes might have become a part of a blossom, so it's not safe to change the boundary
            };
        }
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                match self.nodes[i].as_ref() {
                    Some(internal_dual_node_ptr) => {
                        let dual_node_internal = internal_dual_node_ptr.read_recursive();
                        dual_node_internal.origin.upgrade_force()
                    },
                    _ => { continue }
                }
            };
            let dual_node = dual_node_ptr.read_recursive();
            match dual_node.grow_state {
                DualNodeGrowState::Grow => { self.prepare_dual_node_growth(&dual_node_ptr, true); },
                DualNodeGrowState::Shrink => { },
                DualNodeGrowState::Stay => { },  // do not touch, Stay nodes might have become a part of a blossom, so it's not safe to change the boundary
            };
        }
        let mut group_max_update_length = GroupMaxUpdateLength::new();
        for i in 0..self.nodes.len() {
            let dual_node_ptr = {
                match self.nodes[i].as_ref() {
                    Some(internal_dual_node_ptr) => {
                        let dual_node_internal = internal_dual_node_ptr.read_recursive();
                        dual_node_internal.origin.upgrade_force()
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
            let max_update_length = self.compute_maximum_update_length_dual_node(&dual_node_ptr, is_grow, true);
            group_max_update_length.add(max_update_length);
        }
        group_max_update_length
    }

    fn grow_dual_node(&mut self, dual_node_ptr: &DualNodePtr, length: Weight) {
        let active_timestamp = self.active_timestamp;
        if length == 0 {
            eprintln!("[warning] calling `grow_dual_node` with zero length, nothing to do");
            return
        }
        self.prepare_dual_node_growth(dual_node_ptr, length > 0);
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        {
            // update node dual variable and do sanity check
            let mut dual_node_internal = dual_node_internal_ptr.write();
            dual_node_internal.dual_variable += length;
            assert!(dual_node_internal.dual_variable >= 0, "shrinking to negative dual variable is forbidden");
            // update over-grown vertices
            if dual_node_internal.overgrown_stack.len() > 0 {
                let last_index = dual_node_internal.overgrown_stack.len() - 1;
                let (_, overgrown) = &mut dual_node_internal.overgrown_stack[last_index];
                if length < 0 { assert!( *overgrown >= -length, "overgrown vertex cannot shrink so much"); }
                *overgrown += length;
            }
        }
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
            let edge_ptr = edge_weak.upgrade_force();
            let is_left = *is_left;
            let (growth, weight) = {  // minimize writer lock acquisition
                let mut edge = edge_ptr.write(active_timestamp);
                if is_left {
                    edge.left_growth += length;
                    assert!(edge.left_growth >= 0, "negative growth forbidden");
                } else {
                    edge.right_growth += length;
                    assert!(edge.right_growth >= 0, "negative growth forbidden");
                }
                (edge.left_growth + edge.right_growth, edge.weight)
            };
            let edge = edge_ptr.read_recursive(active_timestamp);
            if growth > weight {
                // first check for if both side belongs to the same dual node, if so, it's ok
                let dual_node_internal_ptr_2: &Option<DualNodeInternalWeak> = if is_left {
                    &edge.right_dual_node
                } else {
                    &edge.left_dual_node
                };
                if dual_node_internal_ptr_2.is_none() || dual_node_internal_ptr_2.as_ref().unwrap() != &dual_node_internal_ptr.downgrade() {
                    let left_ptr = edge.left.upgrade_force();
                    let right_ptr = edge.right.upgrade_force();
                    panic!("over-grown edge ({},{}): {}/{}", left_ptr.read_recursive(active_timestamp).vertex_index
                        , right_ptr.read_recursive(active_timestamp).vertex_index, growth, weight);
                }
            } else if growth < 0 {
                let left_ptr = edge.left.upgrade_force();
                let right_ptr = edge.right.upgrade_force();
                panic!("under-grown edge ({},{}): {}/{}", left_ptr.read_recursive(active_timestamp).vertex_index
                    , right_ptr.read_recursive(active_timestamp).vertex_index, growth, weight);
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
                    dual_node_internal.origin.upgrade_force()
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
                    dual_node_internal.origin.upgrade_force()
                } else { continue }
            };
            let dual_node = dual_node_ptr.read_recursive();
            if matches!(dual_node.grow_state, DualNodeGrowState::Grow) {
                self.grow_dual_node(&dual_node_ptr, length);
            }
        }
    }

    fn peek_touching_child(&mut self, blossom_ptr: &DualNodePtr, dual_node_ptr: &DualNodePtr) -> DualNodePtr {
        let active_timestamp = self.active_timestamp;
        debug_assert!({
            let blossom_internal_ptr = self.get_dual_node_internal_ptr(&blossom_ptr);
            let blossom_internal = blossom_internal_ptr.read_recursive();
            blossom_internal.dual_variable == 0
        }, "only asking blossom with 0 dual variable is valid");
        self.prepare_dual_node_growth(blossom_ptr, false);  // must first prepare shrink ones
        self.prepare_dual_node_growth(dual_node_ptr, true);
        let blossom = blossom_ptr.read_recursive();
        match &blossom.class {
            DualNodeClass::Blossom { nodes_circle } => {
                for child_node_weak in nodes_circle.iter() {
                    let child_node_ptr = child_node_weak.upgrade_force();
                    let child_node_internal_ptr = self.get_dual_node_internal_ptr(&child_node_ptr);
                    let child_node_internal = child_node_internal_ptr.read_recursive();
                    // iterate over the boundary of dual node to check which is touching
                    for (is_left, edge_weak) in child_node_internal.boundary.iter() {
                        let edge_ptr = edge_weak.upgrade_force();
                        let is_left = *is_left;
                        let edge = edge_ptr.read_recursive(active_timestamp);
                        if edge.left_growth + edge.right_growth < edge.weight {
                            continue  // this edge is not fully grown, skip
                        }
                        let peer_dual_node_internal_ptr: Option<DualNodeInternalPtr> = if is_left {
                            edge.right_dual_node.as_ref().map(|ptr| ptr.upgrade_force())
                        } else {
                            edge.left_dual_node.as_ref().map(|ptr| ptr.upgrade_force())
                        };
                        match peer_dual_node_internal_ptr {
                            Some(peer_dual_node_internal_ptr) => {
                                let peer_dual_node_internal = peer_dual_node_internal_ptr.read_recursive();
                                let peer_dual_node_ptr = peer_dual_node_internal.origin.upgrade_force();
                                if &peer_dual_node_ptr == dual_node_ptr {
                                    return child_node_ptr.clone()
                                }
                            }, _ => { }
                        }
                    }
                }
            }, _ => { panic!("{blossom_ptr:?} is not a blossom"); }
        }
        unreachable!("cannot find a touching child between node {:?} and blossom {:?}", dual_node_ptr, blossom_ptr);
    }

    fn peek_touching_descendant(&mut self, blossom_ptr: &DualNodePtr, dual_node_ptr: &DualNodePtr) -> DualNodePtr {
        unimplemented!()
    }

    fn peek_touching_descendant_virtual(&mut self, blossom_ptr: &DualNodePtr, virtual_vertex: VertexIndex) -> DualNodePtr {
        unimplemented!()
    }

}

/*
Implementing fast clear operations
*/

impl FastClear for Edge {

    fn hard_clear(&mut self) {
        self.left_growth = 0;
        self.right_growth = 0;
        self.left_dual_node = None;
        self.left_grandson_dual_node = None;
        self.right_dual_node = None;
        self.right_grandson_dual_node = None;
    }

    fn get_timestamp(&self) -> FastClearTimestamp { self.timestamp }
    fn set_timestamp(&mut self, timestamp: FastClearTimestamp) { self.timestamp = timestamp; }

}

impl FastClear for Vertex {

    fn hard_clear(&mut self) {
        self.is_syndrome = false;
        self.propagated_dual_node = None;
        self.propagated_grandson_dual_node = None;
    }

    fn get_timestamp(&self) -> FastClearTimestamp { self.timestamp }
    fn set_timestamp(&mut self, timestamp: FastClearTimestamp) { self.timestamp = timestamp; }

}

impl DualModuleSerial {

    /// hard clear all growth (manual call not recommended due to performance drawback)
    pub fn hard_clear_graph(&mut self) {
        for edge in self.edges.iter() {
            let mut edge = edge.write_force();
            edge.hard_clear();
            edge.timestamp = 0;
        }
        for vertex in self.vertices.iter() {
            let mut vertex = vertex.write_force();
            vertex.hard_clear();
            vertex.timestamp = 0;
        }
        self.active_timestamp = 0;
    }

    /// soft clear all growth
    pub fn clear_graph(&mut self) {
        if self.active_timestamp == FastClearTimestamp::MAX {  // rarely happens
            self.hard_clear_graph();
        }
        self.active_timestamp += 1;  // implicitly clear all edges growth
    }

    fn sanity_check_grandson(&self, propagated_dual_node_weak: &DualNodeInternalWeak, propagated_grandson_dual_node_weak: &DualNodeInternalWeak) -> Result<(), String> {
        let propagated_dual_node_ptr = propagated_dual_node_weak.upgrade_force();
        let propagated_grandson_dual_node_ptr = propagated_grandson_dual_node_weak.upgrade_force();
        let propagated_dual_node = propagated_dual_node_ptr.read_recursive();
        let propagated_grandson_dual_node = propagated_grandson_dual_node_ptr.read_recursive();
        let propagated_node_ptr = propagated_dual_node.origin.upgrade_force();
        let propagated_node = propagated_node_ptr.read_recursive();
        let propagated_grandson_ptr = propagated_grandson_dual_node.origin.upgrade_force();
        let propagated_grandson = propagated_grandson_ptr.read_recursive();
        if matches!(propagated_grandson.class, DualNodeClass::SyndromeVertex{..}) {
            if matches!(propagated_node.class, DualNodeClass::SyndromeVertex{..}) {
                if propagated_dual_node_ptr != propagated_grandson_dual_node_ptr {
                    return Err(format!("syndrome node {:?} must have grandson equal to itself {:?}", propagated_dual_node_ptr, propagated_grandson_dual_node_ptr))
                }
            } else {
                // test if grandson is a real grandson
                drop(propagated_grandson);
                let mut descendant_ptr = propagated_grandson_ptr;
                loop {
                    let descendant = descendant_ptr.read_recursive();
                    if let Some(descendant_parent_ptr) = descendant.parent_blossom.as_ref() {
                        let descendant_parent_ptr = descendant_parent_ptr.upgrade_force();
                        if descendant_parent_ptr == propagated_node_ptr {
                            return Ok(())
                        }
                        drop(descendant);
                        descendant_ptr = descendant_parent_ptr;
                    } else {
                        return Err(format!("grandson check failed"));
                    }
                }
            }
        } else {
            return Err(format!("grandson must be a vertex"))
        }
        Ok(())
    }

    /// do a sanity check of if all the nodes are in consistent state
    pub fn sanity_check(&self) -> Result<(), String> {
        let active_timestamp = self.active_timestamp;
        for vertex_ptr in self.vertices.iter() {
            vertex_ptr.dynamic_clear(active_timestamp);
            let vertex = vertex_ptr.read_recursive(active_timestamp);
            if let Some(propagated_grandson_dual_node) = vertex.propagated_grandson_dual_node.as_ref() {
                if let Some(propagated_dual_node) = vertex.propagated_dual_node.as_ref() {
                    self.sanity_check_grandson(propagated_dual_node, propagated_grandson_dual_node)?;
                } else {
                    return Err(format!("vertex {} has propagated grandson dual node {:?} but missing propagated dual node"
                        , vertex.vertex_index, vertex.propagated_grandson_dual_node))
                }
            }
            if vertex.propagated_dual_node.is_some() && vertex.propagated_grandson_dual_node.is_none() {
                return Err(format!("vertex {} has propagated dual node {:?} but missing grandson", vertex.vertex_index, vertex.propagated_dual_node))
            }
        }
        Ok(())
    }

}

/*
Implementing visualization functions
*/

impl FusionVisualizer for DualModuleSerial {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        // do the sanity check first before taking snapshot
        self.sanity_check().unwrap();
        let active_timestamp = self.active_timestamp;
        let mut vertices = Vec::<serde_json::Value>::new();
        for vertex_ptr in self.vertices.iter() {
            vertex_ptr.dynamic_clear(active_timestamp);
            let vertex = vertex_ptr.read_recursive(active_timestamp);
            vertices.push(json!({
                if abbrev { "v" } else { "is_virtual" }: if vertex.is_virtual { 1 } else { 0 },
                if abbrev { "s" } else { "is_syndrome" }: if vertex.is_syndrome { 1 } else { 0 },
                if abbrev { "p" } else { "propagated_dual_node" }: vertex.propagated_dual_node.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
                if abbrev { "pg" } else { "propagated_grandson_dual_node" }: vertex.propagated_grandson_dual_node.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
            }));
        }
        let mut edges = Vec::<serde_json::Value>::new();
        for edge_ptr in self.edges.iter() {
            edge_ptr.dynamic_clear(active_timestamp);
            let edge = edge_ptr.read_recursive(active_timestamp);
            edges.push(json!({
                if abbrev { "w" } else { "weight" }: edge.weight,
                if abbrev { "l" } else { "left" }: edge.left.upgrade_force().read_recursive(active_timestamp).vertex_index,
                if abbrev { "r" } else { "right" }: edge.right.upgrade_force().read_recursive(active_timestamp).vertex_index,
                if abbrev { "lg" } else { "left_growth" }: edge.left_growth,
                if abbrev { "rg" } else { "right_growth" }: edge.right_growth,
                if abbrev { "ld" } else { "left_dual_node" }: edge.left_dual_node.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
                if abbrev { "lgd" } else { "left_grandson_dual_node" }: edge.left_grandson_dual_node.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
                if abbrev { "rd" } else { "right_dual_node" }: edge.right_dual_node.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
                if abbrev { "rgd" } else { "right_grandson_dual_node" }: edge.right_grandson_dual_node.as_ref().map(|weak| weak.upgrade_force().read_recursive().index),
            }));
        }
        let mut dual_nodes = Vec::<serde_json::Value>::new();
        for node_ptr in self.nodes.iter() {
            if let Some(node_ptr) = node_ptr.as_ref() {
                let node = node_ptr.read_recursive();
                dual_nodes.push(json!({
                    if abbrev { "b" } else { "boundary" }: node.boundary.iter().map(|(is_left, edge_weak)|
                        (*is_left, edge_weak.upgrade_force().read_recursive(active_timestamp).index)).collect::<Vec<(bool, usize)>>(),
                    if abbrev { "d" } else { "dual_variable" }: node.dual_variable,
                }));
            } else {
                dual_nodes.push(json!(null));
            }
        }
        json!({
            "vertices": vertices,
            "edges": edges,
            "dual_nodes": dual_nodes,
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
        debug_assert!(dual_node_ptr == &dual_node_internal_ptr.read_recursive().origin.upgrade_force(), "dual node and dual internal node must corresponds to each other");
        dual_node_internal_ptr.clone()
    }

    /// adjust the boundary of each dual node to fit into the need of growing (`length` > 0) or shrinking (`length` < 0)
    pub fn prepare_dual_node_growth(&mut self, dual_node_ptr: &DualNodePtr, is_grow: bool) {
        let active_timestamp = self.active_timestamp;
        let mut updated_boundary = Vec::<(bool, EdgeWeak)>::new();
        let mut propagating_vertices = Vec::<(VertexPtr, Option<DualNodeInternalWeak>)>::new();
        let dual_node_internal_ptr = self.get_dual_node_internal_ptr(&dual_node_ptr);
        if is_grow {  // gracefully update the boundary to ease growing
            let dual_node_internal = dual_node_internal_ptr.read_recursive();
            for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
                let edge_ptr = edge_weak.upgrade_force();
                let is_left = *is_left;
                let edge = edge_ptr.read_recursive(active_timestamp);
                let peer_dual_node: &Option<DualNodeInternalWeak> = if is_left {
                    &edge.right_dual_node
                } else {
                    &edge.left_dual_node
                };
                if edge.left_growth + edge.right_growth == edge.weight && peer_dual_node.is_none() {
                    // need to propagate to a new node
                    let peer_vertex_ptr = if is_left {
                        edge.right.upgrade_force()
                    } else {
                        edge.left.upgrade_force()
                    };
                    // to avoid already occupied node being propagated
                    peer_vertex_ptr.dynamic_clear(active_timestamp);
                    let peer_vertex = peer_vertex_ptr.read_recursive(active_timestamp);
                    if peer_vertex.is_virtual {  // virtual node is never propagated, so keep this edge in the boundary
                        updated_boundary.push((is_left, edge_weak.clone()));
                    } else {
                        assert!(peer_vertex.propagated_dual_node.is_none(), "growing into another propagated vertex forbidden");
                        assert!(peer_vertex.propagated_grandson_dual_node.is_none(), "growing into another propagated vertex forbidden");
                        propagating_vertices.push((peer_vertex_ptr.clone(), if is_left { edge.left_grandson_dual_node.clone() } else { edge.right_grandson_dual_node.clone() }));
                        // this edge is dropped, so we need to set both end of this edge to this dual node
                        drop(edge);  // unlock read
                        let mut edge = edge_ptr.write(active_timestamp);
                        if is_left {
                            edge.right_dual_node = Some(dual_node_internal_ptr.downgrade());
                            debug_assert!(edge.left_grandson_dual_node.is_some());
                            edge.right_grandson_dual_node = edge.left_grandson_dual_node.clone();
                        } else {
                            edge.left_dual_node = Some(dual_node_internal_ptr.downgrade());
                            debug_assert!(edge.right_grandson_dual_node.is_some());
                            edge.left_grandson_dual_node = edge.right_grandson_dual_node.clone();
                        }
                    }
                } else {  // keep other edges
                    updated_boundary.push((is_left, edge_weak.clone()));
                }
            }
            drop(dual_node_internal);  // unlock
            // propagating nodes may be duplicated, but it's easy to check by `propagated_dual_node`
            for (vertex_ptr, grandson_dual_node) in propagating_vertices.into_iter() {
                let mut vertex = vertex_ptr.write(active_timestamp);
                if vertex.propagated_dual_node.is_none() {
                    vertex.propagated_dual_node = Some(dual_node_internal_ptr.downgrade());
                    vertex.propagated_grandson_dual_node = grandson_dual_node.clone();
                    let mut count_newly_propagated_edge = 0;
                    for edge_weak in vertex.edges.iter() {
                        let edge_ptr = edge_weak.upgrade_force();
                        let (is_left, newly_propagated_edge) = {
                            edge_ptr.dynamic_clear(active_timestamp);
                            let edge = edge_ptr.read_recursive(active_timestamp);
                            let is_left = vertex_ptr.downgrade() == edge.left;
                            let newly_propagated_edge = if is_left {
                                edge.left_dual_node.is_none()
                            } else {
                                edge.right_dual_node.is_none()
                            };
                            (is_left, newly_propagated_edge)
                        };
                        if newly_propagated_edge {
                            count_newly_propagated_edge += 1;
                            updated_boundary.push((is_left, edge_weak.clone()));
                            let mut edge = edge_ptr.write(active_timestamp);
                            if is_left {
                                edge.left_dual_node = Some(dual_node_internal_ptr.downgrade());
                                edge.left_grandson_dual_node = grandson_dual_node.clone();
                            } else {
                                edge.right_dual_node = Some(dual_node_internal_ptr.downgrade());
                                edge.right_grandson_dual_node = grandson_dual_node.clone();
                            };
                        }
                    }
                    if count_newly_propagated_edge == 0 {
                        let mut dual_node_internal = dual_node_internal_ptr.write();
                        dual_node_internal.overgrown_stack.push((vertex_ptr.downgrade(), 0));
                    }
                }
            }
        } else {  // gracefully update the boundary to ease shrinking
            {
                let mut dual_node_internal = dual_node_internal_ptr.write();
                while dual_node_internal.overgrown_stack.len() > 0 {
                    let last_index = dual_node_internal.overgrown_stack.len() - 1;
                    let (_, overgrown) = &dual_node_internal.overgrown_stack[last_index];
                    if *overgrown == 0 {
                        let (vertex_weak, _) = dual_node_internal.overgrown_stack.pop().unwrap();
                        let vertex_ptr = vertex_weak.upgrade_force();
                        // push the surrounding edges back to the boundary
                        let mut vertex = vertex_ptr.write(active_timestamp);
                        debug_assert!(vertex.propagated_dual_node == Some(dual_node_internal_ptr.downgrade()), "overgrown vertex must be propagated by the original dual node");
                        vertex.propagated_dual_node = None;
                        vertex.propagated_grandson_dual_node = None;
                        for edge_weak in vertex.edges.iter() {
                            let edge_ptr = edge_weak.upgrade_force();
                            let mut edge = edge_ptr.write(active_timestamp);
                            let is_left = vertex_ptr.downgrade() == edge.left;
                            debug_assert!(if is_left {
                                edge.left_dual_node == Some(dual_node_internal_ptr.downgrade()) && edge.left_grandson_dual_node.is_some()
                            } else {
                                edge.right_dual_node == Some(dual_node_internal_ptr.downgrade()) && edge.right_grandson_dual_node.is_some()
                            }, "overgrown vertices must be surrounded by the same propagated dual node");
                            if is_left {
                                edge.left_dual_node = None;
                                edge.left_grandson_dual_node = None;
                            } else {
                                edge.right_dual_node = None;
                                edge.right_grandson_dual_node = None;
                            }
                            updated_boundary.push((!is_left, edge_weak.clone()));  // boundary has the opposite end
                        }
                    } else {
                        break  // found non-negative overgrown in the stack
                    }
                }
            }
            let dual_node_internal = dual_node_internal_ptr.read_recursive();
            for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
                let edge_ptr = edge_weak.upgrade_force();
                let is_left = *is_left;
                let edge = edge_ptr.read_recursive(active_timestamp);
                let this_growth = if is_left {
                    edge.left_growth
                } else {
                    edge.right_growth
                };
                if this_growth == 0 {
                    // need to shrink before this vertex
                    let this_vertex_ptr = if is_left {
                        edge.left.upgrade_force()
                    } else {
                        edge.right.upgrade_force()
                    };
                    // to avoid already occupied node being propagated
                    let this_vertex = this_vertex_ptr.read_recursive(active_timestamp);
                    if this_vertex.is_syndrome {  // never shrink from the syndrome itself
                        updated_boundary.push((is_left, edge_weak.clone()));
                    } else {
                        assert!(this_vertex.propagated_dual_node.is_some(), "unexpected shrink into an empty vertex");
                        propagating_vertices.push((this_vertex_ptr.clone(), None));
                    }
                } else {  // keep other edges
                    updated_boundary.push((is_left, edge_weak.clone()));
                }
            }
            // propagating nodes may be duplicated, but it's easy to check by `propagated_dual_node`
            for (vertex_ptr, _) in propagating_vertices.into_iter() {
                let mut vertex = vertex_ptr.write(active_timestamp);
                if vertex.propagated_dual_node.is_some() {
                    vertex.propagated_dual_node = None;
                    vertex.propagated_grandson_dual_node = None;
                    for edge_weak in vertex.edges.iter() {
                        let edge_ptr = edge_weak.upgrade_force();
                        let (is_left, newly_propagated_edge) = {
                            let edge = edge_ptr.read_recursive(active_timestamp);
                            let is_left = vertex_ptr.downgrade() == edge.left;
                            debug_assert!(if is_left { edge.right != vertex_ptr.downgrade() } else { edge.right == vertex_ptr.downgrade() });
                            // fully grown edge is where to shrink
                            let newly_propagated_edge = edge.left_dual_node == Some(dual_node_internal_ptr.downgrade())
                                && edge.right_dual_node == Some(dual_node_internal_ptr.downgrade())
                                && edge.left_growth + edge.right_growth >= edge.weight;
                            debug_assert!({
                                newly_propagated_edge || {
                                    if is_left { edge.left_growth == 0  } else { edge.right_growth == 0 }
                                }
                            }, "an edge must be either newly propagated or to be removed");
                            (is_left, newly_propagated_edge)
                        };
                        if newly_propagated_edge {
                            updated_boundary.push((!is_left, edge_weak.clone()));
                            let mut edge = edge_ptr.write(active_timestamp);
                            if is_left {
                                assert!(edge.right_dual_node.is_some(), "unexpected shrinking to empty edge");
                                assert!(edge.right_dual_node.as_ref().unwrap() == &dual_node_internal_ptr.downgrade(), "shrinking edge should be same tree node");
                                edge.left_dual_node = None;
                                edge.left_grandson_dual_node = None;
                            } else {
                                assert!(edge.left_dual_node.is_some(), "unexpected shrinking to empty edge");
                                assert!(edge.left_dual_node.as_ref().unwrap() == &dual_node_internal_ptr.downgrade(), "shrinking edge should be same tree node");
                                edge.right_dual_node = None;
                                edge.right_grandson_dual_node = None;
                            };
                        } else {
                            let mut edge = edge_ptr.write(active_timestamp);
                            if is_left {
                                edge.left_dual_node = None;
                                edge.left_grandson_dual_node = None;
                            } else {
                                edge.right_dual_node = None;
                                edge.right_grandson_dual_node = None;
                            }
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
    use super::super::primal_module_serial::tests::*;

    #[allow(dead_code)]
    fn debug_print_dual_node(dual_module: &DualModuleSerial, dual_node_ptr: &DualNodePtr) {
        println!("boundary:");
        let dual_node_internal_ptr = dual_module.get_dual_node_internal_ptr(dual_node_ptr);
        let dual_node_internal = dual_node_internal_ptr.read_recursive();
        for (is_left, edge_weak) in dual_node_internal.boundary.iter() {
            let edge_ptr = edge_weak.upgrade_force();
            let edge = edge_ptr.read_recursive_force();
            println!("    {} {:?}", if *is_left { " left" } else { "right" }, edge);
        }
    }

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
        let interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let dual_node_19_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_25_ptr = interface.nodes[1].as_ref().unwrap().clone();
        dual_module.grow_dual_node(&dual_node_19_ptr, half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, half_weight);
        visualizer.snapshot_combined(format!("grow to 0.5"), vec![&interface, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, half_weight);
        visualizer.snapshot_combined(format!("grow to 1"), vec![&interface, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, half_weight);
        visualizer.snapshot_combined(format!("grow to 1.5"), vec![&interface, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, -half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, -half_weight);
        visualizer.snapshot_combined(format!("shrink to 1"), vec![&interface, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, -half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, -half_weight);
        visualizer.snapshot_combined(format!("shrink to 0.5"), vec![&interface, &dual_module]).unwrap();
        dual_module.grow_dual_node(&dual_node_19_ptr, -half_weight);
        dual_module.grow_dual_node(&dual_node_25_ptr, -half_weight);
        visualizer.snapshot_combined(format!("shrink to 0"), vec![&interface, &dual_module]).unwrap();
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
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let dual_node_19_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_26_ptr = interface.nodes[1].as_ref().unwrap().clone();
        let dual_node_35_ptr = interface.nodes[2].as_ref().unwrap().clone();
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 6 * half_weight);
        visualizer.snapshot_combined(format!("before create blossom"), vec![&interface, &dual_module]).unwrap();
        let nodes_circle = vec![dual_node_19_ptr.clone(), dual_node_26_ptr.clone(), dual_node_35_ptr.clone()];
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        let dual_node_blossom = interface.create_blossom(nodes_circle, &mut dual_module);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 7 * half_weight);
        visualizer.snapshot_combined(format!("blossom grow half weight"), vec![&interface, &dual_module]).unwrap();
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 8 * half_weight);
        visualizer.snapshot_combined(format!("blossom grow half weight"), vec![&interface, &dual_module]).unwrap();
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 9 * half_weight);
        visualizer.snapshot_combined(format!("blossom grow half weight"), vec![&interface, &dual_module]).unwrap();
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 8 * half_weight);
        visualizer.snapshot_combined(format!("blossom shrink half weight"), vec![&interface, &dual_module]).unwrap();
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 6 * half_weight);
        visualizer.snapshot_combined(format!("blossom shrink weight"), vec![&interface, &dual_module]).unwrap();
        interface.expand_blossom(dual_node_blossom, &mut dual_module);
        interface.set_grow_state(&dual_node_19_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.set_grow_state(&dual_node_35_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 3 * half_weight);
        visualizer.snapshot_combined(format!("individual shrink half weight"), vec![&interface, &dual_module]).unwrap();
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
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let dual_node_19_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_25_ptr = interface.nodes[1].as_ref().unwrap().clone();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 4 * half_weight);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module]).unwrap();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 6 * half_weight);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module]).unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_19_ptr, &dual_node_25_ptr), "unexpected: {:?}", group_max_update_length);
    }

    #[test]
    fn dual_module_serial_stop_reason_2() {  // cargo test dual_module_serial_stop_reason_2 -- --nocapture
        let visualize_filename = format!("dual_module_serial_stop_reason_2.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[18].is_syndrome = true;
        code.vertices[26].is_syndrome = true;
        code.vertices[34].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let dual_node_18_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_26_ptr = interface.nodes[1].as_ref().unwrap().clone();
        let dual_node_34_ptr = interface.nodes[2].as_ref().unwrap().clone();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 3 * half_weight);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module]).unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_18_ptr, &dual_node_26_ptr)
            || group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_26_ptr, &dual_node_34_ptr), "unexpected: {:?}", group_max_update_length);
        // first match 18 and 26
        interface.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Stay, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Stay, &mut dual_module);
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_26_ptr, &dual_node_34_ptr)
            , "unexpected: {:?}", group_max_update_length);
        // 34 touches 26, so it will grow the tree by absorbing 18 and 26
        interface.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Grow, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 4 * half_weight);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module]).unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_18_ptr, &dual_node_34_ptr), "unexpected: {:?}", group_max_update_length);
        // for a blossom because 18 and 34 come from the same alternating tree
        let dual_node_blossom = interface.create_blossom(vec![dual_node_18_ptr.clone(), dual_node_26_ptr.clone(), dual_node_34_ptr.clone()], &mut dual_module);
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 6 * half_weight);
        visualizer.snapshot_combined(format!("grow blossom"), vec![&interface, &dual_module]).unwrap();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 8 * half_weight);
        visualizer.snapshot_combined(format!("grow blossom"), vec![&interface, &dual_module]).unwrap();
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().get_touching_virtual() == Some((dual_node_blossom.clone(), 23))
            || group_max_update_length.get_conflicts_immutable().peek().unwrap().get_touching_virtual() == Some((dual_node_blossom.clone(), 39))
            , "unexpected: {:?}", group_max_update_length);
        // blossom touches virtual boundary, so it's matched
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Stay, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.is_empty(), "unexpected: {:?}", group_max_update_length);
        // also test the reverse procedure: shrinking and expanding blossom
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 6 * half_weight);
        visualizer.snapshot_combined(format!("shrink blossom"), vec![&interface, &dual_module]).unwrap();
        // before expand
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 4 * half_weight);
        visualizer.snapshot_combined(format!("shrink blossom"), vec![&interface, &dual_module]).unwrap();
        // cannot shrink anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap() == &MaxUpdateLength::BlossomNeedExpand(dual_node_blossom.downgrade())
            , "unexpected: {:?}", group_max_update_length);
        // expand blossom
        interface.expand_blossom(dual_node_blossom, &mut dual_module);
        // regain access to underlying nodes
        interface.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Grow, &mut dual_module);
        interface.set_grow_state(&dual_node_34_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 2 * half_weight);
        visualizer.snapshot_combined(format!("shrink"), vec![&interface, &dual_module]).unwrap();
    }

    /// this test helps observe bugs of fast clear, by removing snapshot: snapshot will do the clear automatically
    #[test]
    fn dual_module_serial_fast_clear_1() {  // cargo test dual_module_serial_fast_clear_1 -- --nocapture
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, half_weight);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[18].is_syndrome = true;
        code.vertices[26].is_syndrome = true;
        code.vertices[34].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        // create dual nodes and grow them by half length
        let dual_node_18_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_26_ptr = interface.nodes[1].as_ref().unwrap().clone();
        let dual_node_34_ptr = interface.nodes[2].as_ref().unwrap().clone();
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 3 * half_weight);
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_18_ptr, &dual_node_26_ptr)
            || group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_26_ptr, &dual_node_34_ptr), "unexpected: {:?}", group_max_update_length);
        // first match 18 and 26
        interface.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Stay, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Stay, &mut dual_module);
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_26_ptr, &dual_node_34_ptr)
            , "unexpected: {:?}", group_max_update_length);
        // 34 touches 26, so it will grow the tree by absorbing 18 and 26
        interface.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Grow, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 4 * half_weight);
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().is_conflicting(&dual_node_18_ptr, &dual_node_34_ptr), "unexpected: {:?}", group_max_update_length);
        // for a blossom because 18 and 34 come from the same alternating tree
        let dual_node_blossom = interface.create_blossom(vec![dual_node_18_ptr.clone(), dual_node_26_ptr.clone(), dual_node_34_ptr.clone()], &mut dual_module);
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        // grow the maximum
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        // cannot grow anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap().get_touching_virtual() == Some((dual_node_blossom.clone(), 23))
            || group_max_update_length.get_conflicts_immutable().peek().unwrap().get_touching_virtual() == Some((dual_node_blossom.clone(), 39))
            , "unexpected: {:?}", group_max_update_length);
        // blossom touches virtual boundary, so it's matched
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Stay, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.is_empty(), "unexpected: {:?}", group_max_update_length);
        // also test the reverse procedure: shrinking and expanding blossom
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        // before expand
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        // cannot shrink anymore, find out the reason
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert!(group_max_update_length.get_conflicts_immutable().peek().unwrap() == &MaxUpdateLength::BlossomNeedExpand(dual_node_blossom.downgrade())
            , "unexpected: {:?}", group_max_update_length);
        // expand blossom
        interface.expand_blossom(dual_node_blossom, &mut dual_module);
        // regain access to underlying nodes
        interface.set_grow_state(&dual_node_18_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.set_grow_state(&dual_node_26_ptr, DualNodeGrowState::Grow, &mut dual_module);
        interface.set_grow_state(&dual_node_34_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        let group_max_update_length = dual_module.compute_maximum_update_length();
        assert_eq!(group_max_update_length.get_none_zero_growth(), Some(2 * half_weight), "unexpected: {:?}", group_max_update_length);
        interface.grow(2 * half_weight, &mut dual_module);
        assert_eq!(interface.sum_dual_variables, 2 * half_weight);
    }

    #[test]
    fn dual_module_peek_touching_child_1() {  // cargo test dual_module_peek_touching_child_1 -- --nocapture
        let visualize_filename = format!("dual_module_peek_touching_child_1.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(11, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[16].is_syndrome = true;
        code.vertices[52].is_syndrome = true;
        code.vertices[65].is_syndrome = true;
        code.vertices[76].is_syndrome = true;
        code.vertices[112].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        let dual_node_16_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_52_ptr = interface.nodes[1].as_ref().unwrap().clone();
        let dual_node_65_ptr = interface.nodes[2].as_ref().unwrap().clone();
        let dual_node_76_ptr = interface.nodes[3].as_ref().unwrap().clone();
        let dual_node_112_ptr = interface.nodes[4].as_ref().unwrap().clone();
        interface.grow(2 * half_weight, &mut dual_module);
        let nodes_circle = vec![dual_node_52_ptr.clone(), dual_node_65_ptr.clone(), dual_node_76_ptr.clone()];
        interface.set_grow_state(&dual_node_65_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        let dual_node_blossom = interface.create_blossom(nodes_circle, &mut dual_module);
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Stay, &mut dual_module);
        interface.grow(2 * half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("prepared"), vec![&interface, &dual_module]).unwrap();
        // test API
        interface.set_grow_state(&dual_node_blossom, DualNodeGrowState::Shrink, &mut dual_module);
        let touching_child_ptr_1 = dual_module.peek_touching_child(&dual_node_blossom, &dual_node_16_ptr);
        println!("touching_child_ptr_1: {touching_child_ptr_1:?}");
        assert_eq!(touching_child_ptr_1, dual_node_52_ptr);
        let touching_child_ptr_2 = dual_module.peek_touching_child(&dual_node_blossom, &dual_node_112_ptr);
        println!("touching_child_ptr_2: {touching_child_ptr_2:?}");
        assert_eq!(touching_child_ptr_2, dual_node_76_ptr);
    }

    #[test]
    fn dual_module_grow_iterative_1() {  // cargo test dual_module_grow_iterative_1 -- --nocapture
        let visualize_filename = format!("dual_module_grow_iterative_1.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(11, 0.1, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        // try to work on a simple syndrome
        code.vertices[39].is_syndrome = true;
        code.vertices[65].is_syndrome = true;
        code.vertices[87].is_syndrome = true;
        let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        interface.grow_iterative(4 * half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("grow"), vec![&interface, &dual_module]).unwrap();
        assert_eq!(interface.sum_dual_variables, 3 * 4 * half_weight);
        let dual_node_39_ptr = interface.nodes[0].as_ref().unwrap().clone();
        let dual_node_65_ptr = interface.nodes[1].as_ref().unwrap().clone();
        let dual_node_87_ptr = interface.nodes[2].as_ref().unwrap().clone();
        interface.set_grow_state(&dual_node_39_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.set_grow_state(&dual_node_65_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.set_grow_state(&dual_node_87_ptr, DualNodeGrowState::Shrink, &mut dual_module);
        interface.grow_iterative(4 * half_weight, &mut dual_module);
        visualizer.snapshot_combined(format!("shrink"), vec![&interface, &dual_module]).unwrap();
        assert_eq!(interface.sum_dual_variables, 0);
    }

    #[test]
    fn dual_module_debug_1() {  // cargo test dual_module_debug_1 -- --nocapture
        let visualize_filename = format!("dual_module_debug_1.json");
        let syndrome_vertices = vec![6, 7, 17, 18, 21, 27, 28, 42, 43, 49, 51, 52, 54, 55, 61, 63, 65, 67, 76, 78, 80, 86, 103, 110, 113, 114, 116, 122, 125, 127];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 23);
    }

    #[test]
    fn dual_module_debug_2() {  // cargo test dual_module_debug_2 -- --nocapture
        let visualize_filename = format!("dual_module_debug_2.json");
        let syndrome_vertices = vec![5, 12, 16, 19, 21, 38, 42, 43, 49, 56, 61, 67, 72, 73, 74, 75, 76, 88, 89, 92, 93, 99, 105, 112, 117, 120, 124, 129];
        primal_module_serial_basic_standard_syndrome(11, visualize_filename, syndrome_vertices, 22);
    }
    
}
