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
use std::any::Any;
use super::visualize::*;


pub struct DualModuleSerial {
    /// all vertices including virtual ones
    pub vertices: Vec<VertexPtr>,
    /// nodes internal information
    pub nodes: Vec<DualNodeInternalPtr>,
    /// keep edges, which can also be accessed in [`Self::vertices`]
    pub edges: Vec<EdgePtr>,
    /// current timestamp
    pub active_timestamp: usize,
}

pub type DualNodeInternalPtr = Arc<RwLock<DualNodeInternal>>;

/// internal information of the dual node, added to the [`DualNode`]
#[derive(Derivative)]
#[derivative(Debug)]
pub struct DualNodeInternal {
    /// the pointer to the origin [`DualNode`]
    pub origin: DualNodePtr,
    /// dual variable of this node
    pub dual_variable: Weight,
    /// edges on the boundary of this node, (`is_left`, `edge`)
    pub boundary: Vec<(bool, EdgePtr)>,
}

pub type VertexPtr = Arc<RwLock<Vertex>>;
pub type EdgePtr = Arc<RwLock<Edge>>;

#[derive(Derivative)]
#[derivative(Debug)]
struct Vertex {
    /// the index of this vertex in the decoding graph, not necessary the index in [`DualModule::vertices`] if it's partitioned
    pub index: VertexIndex,
    /// if a vertex is virtual, then it can be matched any times
    pub is_virtual: bool,
    /// all neighbor edges, in surface code this should be constant number of edges
    #[derivative(Debug="ignore")]
    pub edges: Vec<EdgePtr>,
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
    /// for fast clear
    pub timestamp: usize,
}

impl DualModule for DualModuleSerial {

    /// initialize the dual module, which is supposed to be reused for multiple decoding tasks with the same structure
    fn new(vertex_num: usize, weighted_edges: &Vec<(usize, usize, Weight)>, virtual_vertices: &Vec<usize>) -> Self {
        // create vertices
        let vertices: Vec<VertexPtr> = (0..vertex_num).map(|vertex_index| Arc::new(RwLock::new(Vertex {
            index: vertex_index,
            is_virtual: false,
            edges: Vec::new(),
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
        }
    }

    /// clear all growth and existing dual nodes
    fn clear(&mut self) {
        self.clear_growth();
        self.nodes.clear();
    }

    /// create a new dual node
    fn create_dual_node(&mut self, node: DualNodePtr) {
        let node_ptr = Arc::clone(&node);
        let mut node = node.write();
        assert!(node.internal.is_none(), "dual node has already been created, do not call twice");
        let boundary = {
            let mut boundary = Vec::<(bool, EdgePtr)>::new();
            match &node.class {
                DualNodeClass::Blossom { nodes_circle } => {

                },
                DualNodeClass::SyndromeVertex { syndrome_index } => {

                },
            }
            boundary
        };
        let node_internal = Arc::new(RwLock::new(DualNodeInternal {
            origin: node_ptr,
            dual_variable: 0,
            boundary: boundary,
        }));
        node.internal = Some(Arc::clone(&node_internal) as Arc<RwLock<dyn Any>>);
        self.nodes.push(node_internal);
    }

    /// expand a blossom
    fn expand_blossom(&mut self, node: DualNodePtr) {

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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::example::*;

    #[test]
    fn dual_module_serial_basics() {  // cargo test dual_module_serial_basics -- --nocapture
        let visualize_filename = format!("dual_module_serial_basics.json");
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, 500);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        print_visualize_link(&visualize_filename);
        // create dual module out of code
        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
        visualizer.snapshot_combined(format!("code"), vec![&code, &dual_module]).unwrap();
    }

}
