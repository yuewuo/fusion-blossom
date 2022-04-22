//! Single-thread implementation of fusion blossom algorithm
//! 

use super::util::*;
use std::sync::Arc;
use crate::parking_lot::Mutex;  // in single thread implementation, it has "Inline fast path for the uncontended case"
use crate::serde_json;
use super::union_find::*;
use super::visualize::*;

pub struct Node {
    /// the index of this node in [`FusionSingleThread::nodes`]
    pub node_index: usize,
    /// if a node is virtual, then it can be matched any times
    pub is_virtual: bool,
    /// if a node is syndrome
    pub is_syndrome: bool,
    /// all neighbor edges, in surface code this should be constant number of edges, (`peer_node_index`, `edge`)
    pub edges: Vec<Arc<Mutex<Edge>>>,
    /// corresponding tree node if exist (only applies if this is syndrome vertex)
    pub tree_node: Option<Arc<Mutex<TreeNode>>>,
}

pub struct TreeNode {
    /// the index of this tree node in [`FusionSingleThread::tree_nodes`]
    pub tree_node_index: usize,
    /// if set, this node has already fall back to UF decoder cluster which can only grow and never shrink
    pub fallback_union_find: bool,
    /// if this tree node is a single vertex, this is the corresponding syndrome node
    pub syndrome_node: Option<Arc<Mutex<Node>>>,
    /// the odd cycle of tree nodes if it's a blossom; otherwise this will be empty
    pub blossom: Vec<Arc<Mutex<TreeNode>>>,
    /// edges on the boundary of this vertex or blossom's dual cluster, (`is_left`, `edge`)
    pub boundary: Vec<(bool, Arc<Mutex<Edge>>)>,
    /// dual variable of this node
    pub dual_variable: Weight,
}

pub struct Edge {
    /// the index of this edge in [`FusionSingleThread::edges`]
    pub edge_index: usize,
    /// total weight of this edge
    pub weight: Weight,
    /// left node (always with smaller index)
    pub left: Arc<Mutex<Node>>,
    /// right node (always with larger index)
    pub right: Arc<Mutex<Node>>,
    /// growth from the left point
    pub left_growth: Weight,
    /// growth from the right point
    pub right_growth: Weight,
    /// left side stack of tree nodes
    pub left_stack: Vec<Arc<Mutex<TreeNode>>>,
    /// right side stack of tree nodes
    pub right_stack: Vec<Arc<Mutex<TreeNode>>>,
}

pub struct FusionSingleThread {
    /// all nodes including virtual
    pub nodes: Vec<Arc<Mutex<Node>>>,
    /// keep edges, which can also be accessed in [`Self::nodes`]
    pub edges: Vec<Arc<Mutex<Edge>>>,
    /// keep union-find information of different [`TreeNode`], note that it's never called if solving exact MWPM result;
    /// it's only union together when falling back to UF decoder 
    pub union_clusters: UnionFindGeneric<FusionUnionNode>,
    /// alternating tree nodes; can be either a vertex or a blossom
    pub tree_nodes: Vec<Arc<Mutex<TreeNode>>>,
}

impl FusionSingleThread {
    /// create a fusion decoder
    pub fn new(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_nodes: &Vec<usize>) -> Self {
        let nodes: Vec<Arc<Mutex<Node>>> = (0..node_num).map(|node_index| Arc::new(Mutex::new(Node {
            node_index: node_index,
            is_virtual: false,
            is_syndrome: false,
            edges: Vec::new(),
            tree_node: None,
        }))).collect();
        let mut edges = Vec::<Arc<Mutex<Edge>>>::new();
        for &virtual_node in virtual_nodes.iter() {
            let mut node = nodes[virtual_node].lock();
            node.is_virtual = true;
        }
        for &(i, j, weight) in weighted_edges.iter() {
            assert_ne!(i, j, "invalid edge between the same node {}", i);
            let left = usize::min(i, j);
            let right = usize::max(i, j);
            let edge = Arc::new(Mutex::new(Edge {
                edge_index: edges.len(),
                weight: weight,
                left: Arc::clone(&nodes[left]),
                right: Arc::clone(&nodes[right]),
                left_growth: 0,
                right_growth: 0,
                left_stack: Vec::new(),
                right_stack: Vec::new(),
            }));
            edges.push(Arc::clone(&edge));
            for (a, b) in [(i, j), (j, i)] {
                let mut node = nodes[a].lock();
                debug_assert!({  // O(N^2) sanity check, debug mode only
                    let mut no_duplicate = true;
                    for edge in node.edges.iter() {
                        let edge = edge.lock();
                        if Arc::ptr_eq(&edge.left, &nodes[b]) || Arc::ptr_eq(&edge.right, &nodes[b]) {
                            no_duplicate = false;
                            eprintln!("duplicated edge {}-{} with weight {}", i, j, weight);
                            break
                        }
                    }
                    no_duplicate
                });
                node.edges.push(Arc::clone(&edge));
            }
        }
        Self {
            nodes: nodes,
            edges: edges,
            union_clusters: UnionFindGeneric::new(0),
            tree_nodes: Vec::new(),
        }
    }

    pub fn clear_growth(&mut self) {
        // clear all growth
        for edge in self.edges.iter() {
            let mut edge = edge.lock();
            edge.left_growth = 0;
            edge.right_growth = 0;
        }
    }

    // /// create a new blossom
    // pub fn add_tree_node_blossom(&mut self, tree_nodes: Vec<Arc<Mutex<TreeNode>>>) -> Arc<Mutex<TreeNode>> {
    //     // merge the boundaries of these tree nodes
    //     let tree_node = Arc::new(Mutex::new(TreeNode {
    //         tree_node_index: self.tree_nodes.len(),
    //         blossom: vertices,
    //         boundary: boundary,
    //         dual_variable: 0,
    //     }));
    //     self.tree_nodes.push(Arc::clone(&tree_node));
    //     self.union_clusters.insert(FusionUnionNode::default());  // when created, each cluster is on its own
    //     assert_eq!(self.tree_nodes.len(), self.union_clusters.payload.len(), "these two are one-to-one corresponding");
    //     tree_node
    // }

    /// create a new tree node with single syndrome node
    pub fn add_tree_node_vertex(&mut self, syndrome_node: Arc<Mutex<Node>>) -> Arc<Mutex<TreeNode>> {
        // iterate other the edges of this vertex and add them to boundary
        let boundary = {
            let mut boundary = Vec::new();
            let node = syndrome_node.lock();
            if node.tree_node.is_some() {
                eprintln!("add the same syndrome node {} twice as tree node, return the previously added", node.node_index);
                return Arc::clone(node.tree_node.as_ref().unwrap())
            }
            for edge in node.edges.iter() {
                let edge_locked = edge.lock();
                let is_left = Arc::ptr_eq(&syndrome_node, &edge_locked.left);
                boundary.push((is_left, Arc::clone(edge)));
            }
            boundary
        };
        let tree_node = Arc::new(Mutex::new(TreeNode {
            tree_node_index: self.tree_nodes.len(),
            syndrome_node: Some(syndrome_node),
            fallback_union_find: false,
            blossom: Vec::new(),
            boundary: boundary,
            dual_variable: 0,
        }));
        self.tree_nodes.push(Arc::clone(&tree_node));
        self.union_clusters.insert(FusionUnionNode::default());  // when created, each cluster is on its own
        assert_eq!(self.tree_nodes.len(), self.union_clusters.payload.len(), "these two are one-to-one corresponding");
        tree_node
    }

    pub fn load_syndrome(&mut self, syndrome_nodes: &Vec<usize>) {
        // it loads a new syndrome, so it's necessary to clear all growth status
        self.clear_growth();
        // clear all syndrome
        for node in self.nodes.iter() {
            let mut node = node.lock();
            node.is_syndrome = false;
        }
        // set syndromes
        for &syndrome_node in syndrome_nodes.iter() {
            let mut node = self.nodes[syndrome_node].lock();
            assert_eq!(node.is_virtual, false, "virtual node cannot have syndrome");
            node.is_syndrome = true;
        }
    }
}

impl FusionVisualizer for FusionSingleThread {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let mut nodes = Vec::<serde_json::Value>::new();
        for node in self.nodes.iter() {
            let node = node.lock();
            nodes.push(json!({
                if abbrev { "v" } else { "is_virtual" }: if node.is_virtual { 1 } else { 0 },
                if abbrev { "s" } else { "is_syndrome" }: if node.is_syndrome { 1 } else { 0 },
            }));
        }
        let mut edges = Vec::<serde_json::Value>::new();
        for edge in self.edges.iter() {
            let edge = edge.lock();
            edges.push(json!({
                if abbrev { "w" } else { "weight" }: edge.weight,
                if abbrev { "l" } else { "left" }: edge.left.lock().node_index,
                if abbrev { "r" } else { "right" }: edge.right.lock().node_index,
                if abbrev { "lg" } else { "left_growth" }: edge.left_growth,
                if abbrev { "rg" } else { "right_growth" }: edge.right_growth,
            }));
        }
        json!({
            "nodes": nodes,
            "edges": edges,
        })
    }
}

/// union find is never called if solving exact MWPM result; it's only union together when falling back to UF decoder 
#[derive(Debug, Clone)]
pub struct FusionUnionNode {
    pub set_size: usize,
}

impl UnionNodeTrait for FusionUnionNode {
    #[inline]
    fn union(left: &Self, right: &Self) -> (bool, Self) {
        let result = Self {
            set_size: left.set_size + right.set_size,
        };
        // if left size is larger, choose left (weighted union)
        (left.set_size >= right.set_size, result)
    }
    #[inline]
    fn clear(&mut self) {
        self.set_size = 1;
    }
    #[inline]
    fn default() -> Self {
        Self {
            set_size: 1,
        }
    }
}

pub fn solve_mwpm_visualizer(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_nodes: &Vec<usize>, syndrome_nodes: &Vec<usize>, mut visualizer: Option<&mut Visualizer>) -> Vec<usize> {
    let mut fusion_solver = FusionSingleThread::new(node_num, weighted_edges, virtual_nodes);
    fusion_solver.load_syndrome(syndrome_nodes);
    if let Some(ref mut visualizer) = visualizer { visualizer.snapshot(format!("start"), &fusion_solver).unwrap(); }
    if let Some(ref mut visualizer) = visualizer { visualizer.snapshot(format!("form blossom"), &fusion_solver).unwrap(); }
    if let Some(ref mut visualizer) = visualizer { visualizer.snapshot(format!("end"), &fusion_solver).unwrap(); }
    unimplemented!()
}

pub fn solve_mwpm(node_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_nodes: &Vec<usize>, syndrome_nodes: &Vec<usize>) -> Vec<usize> {
    solve_mwpm_visualizer(node_num, weighted_edges, virtual_nodes, syndrome_nodes, None)
}


#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use crate::rand_xoshiro::rand_core::SeedableRng;

    #[test]
    fn single_thread_repetition_code_d11() {  // cargo test single_thread_repetition_code_d11 -- --nocapture
        let d = 11;
        let p = 0.2f64;
        let node_num = (d-1) + 2;  // two virtual nodes at left and right
        let weight: Weight = (10000. * ((1. - p).ln() - p.ln())) as Weight;
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for i in 0..d-1 {
                weighted_edges.push((i, i+1, weight));
            }
            weighted_edges.push((0, d, weight));  // left most edge
            weighted_edges
        };
        let virtual_nodes = vec![d-1, d];
        println!("[debug] weighted_edges: {:?}", weighted_edges);
        let mut errors: Vec<bool> = (0..d).map(|_| false).collect();
        let mut measurements: Vec<bool> = (0..d-1).map(|_| false).collect();
        let rounds = 5;
        for round in 0..rounds {
            let mut rng = DeterministicRng::seed_from_u64(round);
            // generate random error
            for i in 0..d-1 { measurements[i] = false; }  // clear measurement errors
            for i in 0..d {
                errors[i] = rng.next_f64() < p;
                if errors[i] {
                    if i > 0 {
                        measurements[i-1] ^= true;  // flip left
                    }
                    if i < d-1 {
                        measurements[i] ^= true;  // flip right
                    }
                }
            }
            // println!("[debug {}] errors: {:?}", round, errors);
            let mut error_nodes = Vec::new();
            for i in 0..d {
                if errors[i] {
                    error_nodes.push(i);
                }
            }
            println!("[debug {}] error_nodes: {:?}", round, error_nodes);
            // generate syndrome
            let mut syndrome_nodes = Vec::new();
            for i in 0..d-1 {
                if measurements[i] {
                    syndrome_nodes.push(i);
                }
            }
            println!("[debug {}] syndrome_nodes: {:?}", round, syndrome_nodes);
            // run ground truth blossom V algorithm
            let blossom_v_matchings = blossom_v_mwpm(node_num, &weighted_edges, &virtual_nodes, &syndrome_nodes);
            let blossom_v_details = detailed_matching(node_num, &weighted_edges, &syndrome_nodes, &blossom_v_matchings);
            let mut blossom_v_weight: Weight = 0;
            for detail in blossom_v_details.iter() {
                blossom_v_weight += detail.weight;
            }
            println!("[debug {}] blossom_v_matchings: {:?}", round, blossom_v_matchings);
            println!("[debug {}] blossom_v_weight: {:?}", round, blossom_v_weight);
            // run single-thread fusion blossom algorithm
            let fusion_matchings = solve_mwpm(node_num, &weighted_edges, &virtual_nodes, &syndrome_nodes);
            let fusion_details = detailed_matching(node_num, &weighted_edges, &syndrome_nodes, &fusion_matchings);
            let mut fusion_weight: Weight = 0;
            for detail in fusion_details.iter() {
                fusion_weight += detail.weight;
            }
            println!("[debug {}] fusion_matchings: {:?}", round, fusion_matchings);
            println!("[debug {}] fusion_weight: {:?}", round, fusion_weight);
            // they must have same total weight
            assert_eq!(blossom_v_weight, fusion_weight, "both should be optimal MWPM, with the same total weight");
        }
    }

    #[test]
    fn single_thread_repetition_code_visualize_d11() {  // cargo test single_thread_repetition_code_visualize_d11 -- --nocapture
        let d = 11;
        let p = 0.2f64;
        let node_num = (d-1) + 2;  // two virtual nodes at left and right
        let weight: Weight = (10000. * ((1. - p).ln() - p.ln())).max(1.) as Weight;
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for i in 0..d-1 {
                weighted_edges.push((i, i+1, weight));
            }
            weighted_edges.push((0, d, weight));  // left most edge
            weighted_edges
        };
        let virtual_nodes = vec![d-1, d];
        println!("[debug] weighted_edges: {:?}", weighted_edges);
        let mut errors: Vec<bool> = (0..d).map(|_| false).collect();
        let mut measurements: Vec<bool> = (0..d-1).map(|_| false).collect();
        // load error
        let error_nodes = vec![2, 10];
        println!("[debug] error_nodes: {:?}", error_nodes);
        for i in 0..d-1 { measurements[i] = false; }  // clear measurement errors
        for &i in error_nodes.iter() {
            errors[i] = true;
            if i > 0 {
                measurements[i-1] ^= true;  // flip left
            }
            if i < d-1 {
                measurements[i] ^= true;  // flip right
            }
        }
        // println!("[debug {}] errors: {:?}", round, errors);
        // generate syndrome
        let mut syndrome_nodes = Vec::new();
        for i in 0..d-1 {
            if measurements[i] {
                syndrome_nodes.push(i);
            }
        }
        println!("[debug] syndrome_nodes: {:?}", syndrome_nodes);
        // run single-thread fusion blossom algorithm
        let visualize_filename = static_visualize_data_filename();
        print_visualize_link(&visualize_filename);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        let mut positions = Vec::new();
        for i in 0..d {
            positions.push(VisualizePosition::new(0., i as f64, 0.));
            // positions.push(VisualizePosition::new(i as f64 * 0.2, i as f64 * 2., i as f64 * 5.));  // debug visualization: strange position
        }
        positions.push(VisualizePosition::new(0., -1., 0.));
        visualizer.set_positions(positions, true);  // automatic center all nodes
        let fusion_matchings = solve_mwpm_visualizer(node_num, &weighted_edges, &virtual_nodes, &syndrome_nodes, Some(&mut visualizer));
        let fusion_details = detailed_matching(node_num, &weighted_edges, &syndrome_nodes, &fusion_matchings);
        let mut fusion_weight: Weight = 0;
        for detail in fusion_details.iter() {
            fusion_weight += detail.weight;
        }
        println!("[debug] fusion_matchings: {:?}", fusion_matchings);
        println!("[debug] fusion_weight: {:?}", fusion_weight);
    }
}
