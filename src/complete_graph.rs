use super::util::*;
use crate::priority_queue::PriorityQueue;
use std::collections::BTreeMap;
use super::dual_module::EdgeWeightModifier;


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
    pub fn new(vertex_num: VertexNum, weighted_edges: &[(VertexIndex, VertexIndex, Weight)]) -> Self {
        let mut vertices: Vec<CompleteGraphVertex> = (0..vertex_num).map(|_| CompleteGraphVertex { edges: BTreeMap::new(), timestamp: 0, }).collect();
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
    pub fn reset(&mut self) {
        // recover erasure edges
        while self.edge_modifier.has_modified_edges() {
            let (edge_index, original_weight) = self.edge_modifier.pop_modified_edge();
            let (vertex_idx_1, vertex_idx_2, _) = &self.weighted_edges[edge_index as usize];
            let vertex_1 = &mut self.vertices[*vertex_idx_1 as usize];
            assert_eq!(vertex_1.edges.insert(*vertex_idx_2, original_weight), Some(0), "previous weight should be 0");
            let vertex_2 = &mut self.vertices[*vertex_idx_2 as usize];
            assert_eq!(vertex_2.edges.insert(*vertex_idx_1, original_weight), Some(0), "previous weight should be 0");
            self.weighted_edges[edge_index as usize] = (*vertex_idx_1, *vertex_idx_2, original_weight);
        }
    }

    fn load_edge_modifier(&mut self, edge_modifier: &[(EdgeIndex, Weight)]) {
        assert!(!self.edge_modifier.has_modified_edges(), "the current erasure modifier is not clean, probably forget to clean the state?");
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

    /// invalidate Dijkstra's algorithm state from previous call
    pub fn invalidate_previous_dijkstra(&mut self) -> usize {
        if self.active_timestamp == FastClearTimestamp::MAX {  // rarely happens
            self.active_timestamp = 0;
            for i in 0..self.vertex_num {
                self.vertices[i as usize].timestamp = 0;  // refresh all timestamps to avoid conflicts
            }
        }
        self.active_timestamp += 1;  // implicitly invalidate all vertices
        self.active_timestamp
    }

    /// get all complete graph edges from the specific vertex, but will terminate if `terminate` vertex is found
    pub fn all_edges_with_terminate(&mut self, vertex: VertexIndex, terminate: VertexIndex) -> BTreeMap<VertexIndex, (VertexIndex, Weight)> {
        let active_timestamp = self.invalidate_previous_dijkstra();
        let mut pq = PriorityQueue::<EdgeIndex, PriorityElement>::new();
        pq.push(vertex, PriorityElement::new(0, vertex));
        let mut computed_edges = BTreeMap::<VertexIndex, (VertexIndex, Weight)>::new();  // { peer: (previous, weight) }
        loop {  // until no more elements
            if pq.is_empty() {
                break
            }
            let (target, PriorityElement { weight, previous }) = pq.pop().unwrap();
            // eprintln!("target: {}, weight: {}, next: {}", target, weight, next);
            debug_assert!({
                !computed_edges.contains_key(&target)  // this entry shouldn't have been set
            });
            // update entry
            self.vertices[target as usize].timestamp = active_timestamp;  // mark as visited
            if target != vertex {
                computed_edges.insert(target, (previous, weight));
                if target == terminate {
                    break  // early terminate
                }
            }
            // add its neighbors to priority queue
            for (&neighbor, &neighbor_weight) in self.vertices[target as usize].edges.iter() {
                let edge_weight = weight + neighbor_weight;
                if let Some(PriorityElement { weight: existing_weight, previous: existing_previous }) = pq.get_priority(&neighbor) {
                    // update the priority if weight is smaller or weight is equal but distance is smaller
                    // this is necessary if the graph has weight-0 edges, which could lead to cycles in the graph and cause deadlock
                    let mut update = &edge_weight < existing_weight;
                    if &edge_weight == existing_weight {
                        let distance = if neighbor > previous { neighbor - previous } else { previous - neighbor };
                        let existing_distance = if &neighbor > existing_previous { neighbor - existing_previous } else { existing_previous - neighbor };
                        // prevent loop by enforcing strong non-descending
                        if distance < existing_distance || (distance == existing_distance && &previous < existing_previous) {
                            update = true;
                        }
                    }
                    if update {
                        pq.change_priority(&neighbor, PriorityElement::new(edge_weight, target));
                    }
                } else {  // insert new entry only if neighbor has not been visited
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
                break
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
        other.weight.partial_cmp(&self.weight)  // reverse `self` and `other` to prioritize smaller weight
    }
}

impl std::cmp::Ord for PriorityElement {
    #[inline]
    fn cmp(&self, other: &PriorityElement) -> std::cmp::Ordering {
        other.weight.cmp(&self.weight)  // reverse `self` and `other` to prioritize smaller weight
    }
}

impl PriorityElement {
    pub fn new(weight: Weight, previous: VertexIndex) -> Self {
        Self {
            weight,
            previous,
        }
    }
}
