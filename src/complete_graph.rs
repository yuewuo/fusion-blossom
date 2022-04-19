use super::util::*;
use crate::priority_queue::PriorityQueue;
use std::collections::BTreeMap;

/// build complete graph out of skeleton graph using Dijkstra's algorithm
#[derive(Debug, Clone)]
pub struct CompleteGraph {
    /// number of nodes
    pub node_num: usize,
    /// the nodes to run Dijkstra's algorithm
    pub nodes: Vec<CompleteGraphNode>,
    /// timestamp to invalidate all nodes without iterating them; only invalidating all nodes individually when active_timestamp is usize::MAX
    active_timestamp: usize,
}

#[derive(Debug, Clone)]
pub struct CompleteGraphNode {
    /// all skeleton graph edges connected to this node
    pub edges: BTreeMap<usize, Weight>,
    /// timestamp for Dijkstra's algorithm
    timestamp: usize,
}

impl CompleteGraph {
    /// create complete graph given skeleton graph
    pub fn new(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>) -> Self {
        let mut nodes: Vec<CompleteGraphNode> = (0..node_num).map(|_| CompleteGraphNode { edges: BTreeMap::new(), timestamp: 0, }).collect();
        for &(i, j, weight) in weighted_edges.iter() {
            nodes[i].edges.insert(j, weight);
            nodes[j].edges.insert(i, weight);
        }
        Self {
            node_num: node_num,
            nodes: nodes,
            active_timestamp: 0,
        }
    }

    /// invalidate Dijkstra's algorithm state from previous call
    pub fn invalidate_previous_dijkstra(&mut self) -> usize {
        if self.active_timestamp == usize::MAX {  // rarely happens
            self.active_timestamp = 0;
            for i in 0..self.node_num {
                self.nodes[i].timestamp = 0;  // refresh all timestamps to avoid conflicts
            }
        }
        self.active_timestamp += 1;  // implicitly invalidate all nodes
        self.active_timestamp
    }

    /// get all complete graph edges from the specific node, but will terminate if `terminate` node is found
    pub fn all_edges_with_terminate(&mut self, node: usize, terminate: usize) -> BTreeMap<usize, (usize, Weight)> {
        let active_timestamp = self.invalidate_previous_dijkstra();
        let mut pq = PriorityQueue::<usize, PriorityElement>::new();
        pq.push(node, PriorityElement::new(0, node));
        let mut computed_edges = BTreeMap::<usize, (usize, Weight)>::new();  // { peer: (previous, weight) }
        loop {  // until no more elements
            if pq.len() == 0 {
                break
            }
            let (target, PriorityElement { weight, previous }) = pq.pop().unwrap();
            // eprintln!("target: {}, weight: {}, next: {}", target, weight, next);
            debug_assert!({
                !computed_edges.contains_key(&target)  // this entry shouldn't have been set
            });
            // update entry
            self.nodes[target].timestamp = active_timestamp;  // mark as visited
            if target != node {
                computed_edges.insert(target, (previous, weight));
                if target == terminate {
                    break  // early terminate
                }
            }
            // add its neighbors to priority queue
            for (&neighbor, &neighbor_weight) in self.nodes[target].edges.iter() {
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
                    if self.nodes[neighbor].timestamp != active_timestamp {
                        pq.push(neighbor, PriorityElement::new(edge_weight, target));
                    }
                }
            }
        }
        // println!("[debug] computed_edges: {:?}", computed_edges);
        computed_edges
    }

    /// get all complete graph edges from the specific node
    pub fn all_edges(&mut self, node: usize) -> BTreeMap<usize, (usize, Weight)> {
        self.all_edges_with_terminate(node, usize::MAX)
    }

    /// get minimum-weight path between any two nodes `a` and `b`, in the order `a -> path[0].0 -> path[1].0 -> .... -> path[-1].0` and it's guaranteed that path[-1].0 = b
    pub fn get_path(&mut self, a: usize, b: usize) -> (Vec<(usize, Weight)>, Weight) {
        assert_ne!(a, b, "cannot get path between the same node");
        let edges = self.all_edges_with_terminate(a, b);
        // println!("edges: {:?}", edges);
        let mut node = b;
        let mut path = Vec::new();
        loop {
            if node == a {
                break
            }
            let &(previous, weight) = &edges[&node];
            path.push((node, weight));
            if path.len() > 1 {
                let previous_index = path.len() - 2;
                path[previous_index].1 -= weight;
            }
            node = previous;
        }
        path.reverse();
        (path, edges[&b].1)
    }
}

#[derive(Eq, Debug)]
pub struct PriorityElement {
    pub weight: Weight,
    pub previous: usize,
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
    pub fn new(weight: Weight, previous: usize) -> Self {
        Self {
            weight: weight,
            previous: previous,
        }
    }
}
