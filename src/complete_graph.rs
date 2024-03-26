use super::dual_module::EdgeWeightModifier;
use super::util::*;
use crate::priority_queue::PriorityQueue;
use crate::rayon::prelude::*;
use std::collections::BTreeMap;

/// build complete graph out of skeleton graph using Dijkstra's algorithm
#[derive(Debug, Clone)]
pub struct CompleteGraph {
    /// number of vertices
    pub vertex_num: VertexNum,
    /// the vertices to run Dijkstra's algorithm
    pub vertices: Vec<CompleteGraphVertex>,
    /// timestamp to invalidate all vertices without iterating them; only invalidating all vertices individually when active_timestamp is usize::MAX
    active_timestamp: FastClearTimestamp,
    /// remember the edges that's modified by erasures
    pub edge_modifier: EdgeWeightModifier,
    /// original edge weights
    pub weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>,
}

#[derive(Debug, Clone)]
pub struct CompleteGraphVertex {
    /// all skeleton graph edges connected to this vertex
    pub edges: BTreeMap<VertexIndex, Weight>,
    /// timestamp for Dijkstra's algorithm
    timestamp: FastClearTimestamp,
}

impl CompleteGraph {
    /// create complete graph given skeleton graph
    #[allow(clippy::unnecessary_cast)]
    pub fn new(vertex_num: VertexNum, weighted_edges: &[(VertexIndex, VertexIndex, Weight)]) -> Self {
        let mut vertices: Vec<CompleteGraphVertex> = (0..vertex_num)
            .map(|_| CompleteGraphVertex {
                edges: BTreeMap::new(),
                timestamp: 0,
            })
            .collect();
        for &(i, j, weight) in weighted_edges.iter() {
            vertices[i as usize].edges.insert(j, weight);
            vertices[j as usize].edges.insert(i, weight);
        }
        Self {
            vertex_num,
            vertices,
            active_timestamp: 0,
            edge_modifier: EdgeWeightModifier::new(),
            weighted_edges: weighted_edges.to_owned(),
        }
    }

    /// reset any temporary changes like erasure edges
    #[allow(clippy::unnecessary_cast)]
    pub fn reset(&mut self) {
        // recover erasure edges
        while self.edge_modifier.has_modified_edges() {
            let (edge_index, original_weight) = self.edge_modifier.pop_modified_edge();
            let (vertex_idx_1, vertex_idx_2, _) = &self.weighted_edges[edge_index as usize];
            let vertex_1 = &mut self.vertices[*vertex_idx_1 as usize];
            vertex_1.edges.insert(*vertex_idx_2, original_weight);
            let vertex_2 = &mut self.vertices[*vertex_idx_2 as usize];
            vertex_2.edges.insert(*vertex_idx_1, original_weight);
            self.weighted_edges[edge_index as usize] = (*vertex_idx_1, *vertex_idx_2, original_weight);
        }
    }

    #[allow(clippy::unnecessary_cast)]
    fn load_edge_modifier(&mut self, edge_modifier: &[(EdgeIndex, Weight)]) {
        assert!(
            !self.edge_modifier.has_modified_edges(),
            "the current erasure modifier is not clean, probably forget to clean the state?"
        );
        for (edge_index, target_weight) in edge_modifier.iter() {
            let (vertex_idx_1, vertex_idx_2, original_weight) = &self.weighted_edges[*edge_index as usize];
            let vertex_1 = &mut self.vertices[*vertex_idx_1 as usize];
            vertex_1.edges.insert(*vertex_idx_2, *target_weight);
            let vertex_2 = &mut self.vertices[*vertex_idx_2 as usize];
            vertex_2.edges.insert(*vertex_idx_1, *target_weight);
            self.edge_modifier.push_modified_edge(*edge_index, *original_weight);
            self.weighted_edges[*edge_index as usize] = (*vertex_idx_1, *vertex_idx_2, *target_weight);
        }
    }

    /// temporarily set some edges to 0 weight, and when it resets, those edges will be reverted back to the original weight
    pub fn load_erasures(&mut self, erasures: &[EdgeIndex]) {
        let edge_modifier: Vec<_> = erasures.iter().map(|edge_index| (*edge_index, 0)).collect();
        self.load_edge_modifier(&edge_modifier);
    }

    pub fn load_dynamic_weights(&mut self, dynamic_weights: &[(EdgeIndex, Weight)]) {
        let edge_modifier = dynamic_weights.to_vec();
        self.load_edge_modifier(&edge_modifier);
    }

    /// invalidate Dijkstra's algorithm state from previous call
    #[allow(clippy::unnecessary_cast)]
    pub fn invalidate_previous_dijkstra(&mut self) -> usize {
        if self.active_timestamp == FastClearTimestamp::MAX {
            // rarely happens
            self.active_timestamp = 0;
            for i in 0..self.vertex_num {
                self.vertices[i as usize].timestamp = 0; // refresh all timestamps to avoid conflicts
            }
        }
        self.active_timestamp += 1; // implicitly invalidate all vertices
        self.active_timestamp
    }

    /// get all complete graph edges from the specific vertex, but will terminate if `terminate` vertex is found
    #[allow(clippy::unnecessary_cast)]
    pub fn all_edges_with_terminate(
        &mut self,
        vertex: VertexIndex,
        terminate: VertexIndex,
    ) -> BTreeMap<VertexIndex, (VertexIndex, Weight)> {
        let active_timestamp = self.invalidate_previous_dijkstra();
        let mut pq = PriorityQueue::<EdgeIndex, PriorityElement>::new();
        pq.push(vertex, PriorityElement::new(0, vertex));
        let mut computed_edges = BTreeMap::<VertexIndex, (VertexIndex, Weight)>::new(); // { peer: (previous, weight) }
        loop {
            // until no more elements
            if pq.is_empty() {
                break;
            }
            let (target, PriorityElement { weight, previous }) = pq.pop().unwrap();
            // eprintln!("target: {}, weight: {}, next: {}", target, weight, next);
            debug_assert!({
                !computed_edges.contains_key(&target) // this entry shouldn't have been set
            });
            // update entry
            self.vertices[target as usize].timestamp = active_timestamp; // mark as visited
            if target != vertex {
                computed_edges.insert(target, (previous, weight));
                if target == terminate {
                    break; // early terminate
                }
            }
            // add its neighbors to priority queue
            for (&neighbor, &neighbor_weight) in self.vertices[target as usize].edges.iter() {
                let edge_weight = weight + neighbor_weight;
                if let Some(PriorityElement {
                    weight: existing_weight,
                    previous: existing_previous,
                }) = pq.get_priority(&neighbor)
                {
                    // update the priority if weight is smaller or weight is equal but distance is smaller
                    // this is necessary if the graph has weight-0 edges, which could lead to cycles in the graph and cause deadlock
                    let mut update = &edge_weight < existing_weight;
                    if &edge_weight == existing_weight {
                        let distance = if neighbor > previous {
                            neighbor - previous
                        } else {
                            previous - neighbor
                        };
                        let existing_distance = if &neighbor > existing_previous {
                            neighbor - existing_previous
                        } else {
                            existing_previous - neighbor
                        };
                        // prevent loop by enforcing strong non-descending
                        if distance < existing_distance || (distance == existing_distance && &previous < existing_previous) {
                            update = true;
                        }
                    }
                    if update {
                        pq.change_priority(&neighbor, PriorityElement::new(edge_weight, target));
                    }
                } else {
                    // insert new entry only if neighbor has not been visited
                    if self.vertices[neighbor as usize].timestamp != active_timestamp {
                        pq.push(neighbor, PriorityElement::new(edge_weight, target));
                    }
                }
            }
        }
        // println!("[debug] computed_edges: {:?}", computed_edges);
        computed_edges
    }

    /// get all complete graph edges from the specific vertex
    pub fn all_edges(&mut self, vertex: VertexIndex) -> BTreeMap<VertexIndex, (VertexIndex, Weight)> {
        self.all_edges_with_terminate(vertex, VertexIndex::MAX)
    }

    /// get minimum-weight path between any two vertices `a` and `b`, in the order `a -> path[0].0 -> path[1].0 -> .... -> path[-1].0` and it's guaranteed that path[-1].0 = b
    pub fn get_path(&mut self, a: VertexIndex, b: VertexIndex) -> (Vec<(VertexIndex, Weight)>, Weight) {
        assert_ne!(a, b, "cannot get path between the same vertex");
        let edges = self.all_edges_with_terminate(a, b);
        // println!("edges: {:?}", edges);
        let mut vertex = b;
        let mut path = Vec::new();
        loop {
            if vertex == a {
                break;
            }
            let &(previous, weight) = &edges[&vertex];
            path.push((vertex, weight));
            if path.len() > 1 {
                let previous_index = path.len() - 2;
                path[previous_index].1 -= weight;
            }
            vertex = previous;
        }
        path.reverse();
        (path, edges[&b].1)
    }
}

#[derive(Clone)]
pub struct PrebuiltCompleteGraph {
    /// number of vertices
    pub vertex_num: VertexNum,
    /// all edge weights, if set to Weight::MAX then this edge does not exist
    pub edges: Vec<BTreeMap<VertexIndex, Weight>>,
    /// the virtual boundary weight
    pub virtual_boundary_weight: Vec<Option<(VertexIndex, Weight)>>,
}

impl PrebuiltCompleteGraph {
    #[allow(clippy::unnecessary_cast)]
    pub fn new_threaded(initializer: &SolverInitializer, thread_pool_size: usize) -> Self {
        let mut thread_pool_builder = rayon::ThreadPoolBuilder::new();
        if thread_pool_size != 0 {
            thread_pool_builder = thread_pool_builder.num_threads(thread_pool_size);
        }
        let thread_pool = thread_pool_builder.build().expect("creating thread pool failed");
        let vertex_num = initializer.vertex_num as usize;
        // first collect virtual vertices and real vertices
        let mut is_virtual = vec![false; vertex_num];
        for &virtual_vertex in initializer.virtual_vertices.iter() {
            is_virtual[virtual_vertex as usize] = true;
        }
        type Result = (BTreeMap<VertexIndex, Weight>, Option<(VertexIndex, Weight)>);
        let mut results: Vec<Result> = vec![];
        thread_pool.scope(|_| {
            (0..vertex_num)
                .into_par_iter()
                .map(|vertex_index| {
                    let mut complete_graph = CompleteGraph::new(initializer.vertex_num, &initializer.weighted_edges);
                    let mut edges = BTreeMap::new();
                    let mut virtual_boundary_weight = None;
                    if !is_virtual[vertex_index] {
                        // only build graph for non-virtual vertices
                        let complete_graph_edges = complete_graph.all_edges(vertex_index as VertexIndex);
                        let mut boundary: Option<(VertexIndex, Weight)> = None;
                        for (&peer, &(_, weight)) in complete_graph_edges.iter() {
                            if !is_virtual[peer as usize] {
                                edges.insert(peer, weight);
                            }
                            if is_virtual[peer as usize] && (boundary.is_none() || weight < boundary.as_ref().unwrap().1) {
                                boundary = Some((peer, weight));
                            }
                        }
                        virtual_boundary_weight = boundary;
                    }
                    (edges, virtual_boundary_weight)
                })
                .collect_into_vec(&mut results);
        });
        // optimization: remove edges in the middle
        type UnzipResult = (Vec<BTreeMap<VertexIndex, Weight>>, Vec<Option<(VertexIndex, Weight)>>);
        let (mut edges, virtual_boundary_weight): UnzipResult = results.into_iter().unzip();
        let mut to_be_removed_vec: Vec<Vec<VertexIndex>> = vec![];
        thread_pool.scope(|_| {
            (0..vertex_num)
                .into_par_iter()
                .map(|vertex_index| {
                    let mut to_be_removed = vec![];
                    if !is_virtual[vertex_index] {
                        for (&peer, &weight) in edges[vertex_index].iter() {
                            let boundary_weight = if let Some((_, weight)) = virtual_boundary_weight[vertex_index as usize] {
                                weight
                            } else {
                                Weight::MAX
                            };
                            let boundary_weight_peer = if let Some((_, weight)) = virtual_boundary_weight[peer as usize] {
                                weight
                            } else {
                                Weight::MAX
                            };
                            if boundary_weight != Weight::MAX
                                && boundary_weight_peer != Weight::MAX
                                && weight > boundary_weight + boundary_weight_peer
                            {
                                to_be_removed.push(peer);
                            }
                        }
                    }
                    to_be_removed
                })
                .collect_into_vec(&mut to_be_removed_vec);
        });
        for vertex_index in 0..vertex_num {
            for peer in to_be_removed_vec[vertex_index].iter() {
                edges[vertex_index].remove(peer);
            }
        }
        Self {
            vertex_num: initializer.vertex_num,
            edges,
            virtual_boundary_weight,
        }
    }

    pub fn new(initializer: &SolverInitializer) -> Self {
        Self::new_threaded(initializer, 1)
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn get_edge_weight(&self, vertex_1: VertexIndex, vertex_2: VertexIndex) -> Option<Weight> {
        self.edges[vertex_1 as usize].get(&vertex_2).cloned()
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn get_boundary_weight(&self, vertex_index: VertexIndex) -> Option<(VertexIndex, Weight)> {
        self.virtual_boundary_weight[vertex_index as usize]
    }
}

#[derive(Eq, Debug)]
pub struct PriorityElement {
    pub weight: Weight,
    pub previous: VertexIndex,
}

impl std::cmp::PartialEq for PriorityElement {
    #[inline]
    fn eq(&self, other: &PriorityElement) -> bool {
        self.weight == other.weight
    }
}

impl std::cmp::PartialOrd for PriorityElement {
    #[inline]
    fn partial_cmp(&self, other: &PriorityElement) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for PriorityElement {
    #[inline]
    fn cmp(&self, other: &PriorityElement) -> std::cmp::Ordering {
        other.weight.cmp(&self.weight) // reverse `self` and `other` to prioritize smaller weight
    }
}

impl PriorityElement {
    pub fn new(weight: Weight, previous: VertexIndex) -> Self {
        Self { weight, previous }
    }
}
