//! Example Decoding
//! 
//! This module contains several abstract decoding graph and it's randomized simulator utilities.
//! This helps to debug, but it doesn't corresponds to real error model, nor it's capable of simulating circuit-level noise model.
//! For complex error model and simulator functionality, please see https://github.com/yuewuo/QEC-Playground
//! 
//! 
//! 

use super::visualize::*;
use super::util::*;
use std::collections::HashMap;
use crate::serde_json;


/// Vertex corresponds to a stabilizer measurement bit
pub struct CodeVertex {
    /// position helps to visualize
    position: VisualizePosition,
    /// neighbor edges helps to set find individual edge
    neighbor_edges: Vec<usize>,
    /// virtual vertex won't report measurement results
    is_virtual: bool,
}

/// Edge flips the measurement result of two vertices
pub struct CodeEdge {
    /// the two vertices incident to this edge
    vertices: (usize, usize),
    /// probability of flipping the results of these two vertices; do not set p to 0 to remove edge: if desired, create a new code type
    p: f64,
    /// the integer weight of this edge
    half_weight: Weight,
}

impl CodeEdge {
    pub fn new(a: usize, b: usize) -> Self {
        Self {
            vertices: (a, b),
            p: 0.,
            half_weight: 0,
        }
    }
}

/// default function for computing (pre-scaled) weight from probability
pub fn weight_of_p(p: f64) -> f64 {
    assert!(p >= 0. && p <= 0.5, "p must be a reasonable value between 0 and 50%");
    ((1. - p) / p).ln()
}

pub trait ExampleCode {

    /// get mutable references to vertices and edges
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>);
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>);

    /// generic method that automatically computes integer weights from probabilities,
    /// scales such that the maximum integer weight is 10000 and the minimum is 1
    fn compute_weights(&mut self, max_half_weight: Weight) {
        let (_vertices, edges) = self.vertices_edges();
        let mut max_weight = 0.;
        for edge in edges.iter() {
            let weight = weight_of_p(edge.p);
            if weight > max_weight {
                max_weight = weight;
            }
        }
        assert!(max_weight > 0., "max weight is not expected to be 0.");
        // scale all weights but set the smallest to 1
        for edge in edges.iter_mut() {
            let weight = weight_of_p(edge.p);
            let half_weight: Weight = ((max_half_weight as f64) * weight / max_weight).round() as Weight;
            edge.half_weight = if half_weight == 0 { 1 } else { half_weight };  // weight is required to be even
        }
    }

    /// sanity check to avoid duplicate edges that are hard to debug
    fn sanity_check(&mut self) -> Result<(), String> {
        let (vertices, edges) = self.vertices_edges();
        // check the graph is reasonable
        if vertices.len() == 0 || edges.len() == 0 {
            return Err(format!("empty graph"));
        }
        // check duplicated edges
        let mut existing_edges = HashMap::<(usize, usize), usize>::with_capacity(edges.len() * 2);
        for (idx, edge) in edges.iter().enumerate() {
            let (v1, v2) = edge.vertices;
            let unique_edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
            if existing_edges.contains_key(&unique_edge) {
                let previous_idx = existing_edges[&unique_edge];
                return Err(format!("duplicate edge {} and {} with incident vertices {} and {}", previous_idx, idx, v1, v2));
            }
            existing_edges.insert(unique_edge, idx);
        }
        // check duplicated referenced edge from each vertex
        for (vertex_idx, vertex) in vertices.iter().enumerate() {
            let mut existing_edges = HashMap::<usize, ()>::new();
            if vertex.neighbor_edges.len() == 0 {
                return Err(format!("vertex {} do not have any neighbor edges", vertex_idx));
            }
            for edge_idx in vertex.neighbor_edges.iter() {
                if existing_edges.contains_key(edge_idx) {
                    return Err(format!("duplicate referred edge {} from vertex {}", edge_idx, vertex_idx));
                }
                existing_edges.insert(*edge_idx, ());
            }
        }
        Ok(())
    }

    /// set probability of all edges; user can set individual probabilities
    fn set_probability(&mut self, p: f64) {
        let (_vertices, edges) = self.vertices_edges();
        for edge in edges.iter_mut() {
            edge.p = p;
        }
    }

    /// automatically create vertices given edges
    fn fill_vertices(&mut self, vertex_num: usize) {
        let (vertices, edges) = self.vertices_edges();
        vertices.clear();
        vertices.reserve(vertex_num);
        for i in 0..vertex_num {
            vertices.push(CodeVertex {
                position: VisualizePosition::new(0., 0., 0.),
                neighbor_edges: Vec::new(),
                is_virtual: false,
            });
        }
        for (edge_idx, edge) in edges.iter().enumerate() {
            let vertex_1 = &mut vertices[edge.vertices.0];
            vertex_1.neighbor_edges.push(edge_idx);
            let vertex_2 = &mut vertices[edge.vertices.1];
            vertex_2.neighbor_edges.push(edge_idx);
        }
    }

    /// gather all positions of vertices
    fn get_positions(&mut self) -> Vec<VisualizePosition> {
        let (vertices, _edges) = self.vertices_edges();
        let mut positions = Vec::with_capacity(vertices.len());
        for vertex in vertices.iter() {
            positions.push(vertex.position.clone());
        }
        positions
    }

}

impl<T> FusionVisualizer for T where T: ExampleCode {
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let (self_vertices, self_edges) = self.immutable_vertices_edges();
        let mut vertices = Vec::<serde_json::Value>::new();
        for vertex in self_vertices.iter() {
            vertices.push(json!({
                if abbrev { "v" } else { "is_virtual" }: if vertex.is_virtual { 1 } else { 0 },
                if abbrev { "s" } else { "is_syndrome" }: 0,  // TODO: calculate syndrome
                // if abbrev { "s" } else { "is_syndrome" }: if vertex.is_syndrome { 1 } else { 0 },
            }));
        }
        let mut edges = Vec::<serde_json::Value>::new();
        for edge in self_edges.iter() {
            edges.push(json!({
                if abbrev { "w" } else { "weight" }: edge.half_weight * 2,
                if abbrev { "l" } else { "left" }: edge.vertices.0,
                if abbrev { "r" } else { "right" }: edge.vertices.1,
                if abbrev { "lg" } else { "left_growth" }: 0,  // code itself is not capable of calculating growth
                if abbrev { "rg" } else { "right_growth" }: 0,
            }));
        }
        json!({
            "nodes": vertices,  // TODO: update HTML code to use the same language
            "edges": edges,
            "tree_nodes": [],
        })
    }
}

/// perfect quantum repetition code
pub struct CodeCapacityRepetitionCode {
    /// vertices in the code
    vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    edges: Vec<CodeEdge>,
}

impl ExampleCode for CodeCapacityRepetitionCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) { (&mut self.vertices, &mut self.edges) }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) { (&self.vertices, &self.edges) }
}

impl CodeCapacityRepetitionCode {

    pub fn new(d: usize, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    pub fn create_code(d: usize) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let vertex_num = (d - 1) + 2;  // two virtual vertices at left and right
        // create edges
        let mut edges = Vec::new();
        for i in 0..d-1 {
            edges.push(CodeEdge::new(i, i+1));
        }
        edges.push(CodeEdge::new(0, d));  // tje left-most edge
        let mut code = Self {
            vertices: Vec::new(),
            edges: edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        code.vertices[d-1].is_virtual = true;
        code.vertices[d].is_virtual = true;
        let mut positions = Vec::new();
        for i in 0..d {
            positions.push(VisualizePosition::new(0., i as f64, 0.));
        }
        positions.push(VisualizePosition::new(0., -1., 0.));
        for i in 0..vertex_num {
            code.vertices[i].position = positions[i].clone();
        }
        code
    }


}

/// code capacity noise model is a single measurement round with perfect stabilizer measurements;
/// e.g. this is the decoding graph of a CSS surface code (standard one, not rotated one) with X-type stabilizers
pub struct CodeCapacityPlanarCode {
    /// vertices in the code
    vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    edges: Vec<CodeEdge>,
}

impl ExampleCode for CodeCapacityPlanarCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) { (&mut self.vertices, &mut self.edges) }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) { (&self.vertices, &self.edges) }
}

impl CodeCapacityPlanarCode {

    pub fn new(d: usize, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    pub fn create_code(d: usize) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let row_vertex_num = (d-1) + 2;  // two virtual nodes at left and right
        let vertex_num = row_vertex_num * d;  // `d` rows
        // create edges
        let mut edges = Vec::new();
        for row in 0..d {
            let bias = row * row_vertex_num;
            for i in 0..d-1 {
                edges.push(CodeEdge::new(bias + i, bias + i+1));
            }
            edges.push(CodeEdge::new(bias + 0, bias + d));  // left most edge
            if row + 1 < d {
                for i in 0..d-1 {
                    edges.push(CodeEdge::new(bias + i, bias + i + row_vertex_num));
                }
            }
        }
        let mut code = Self {
            vertices: Vec::new(),
            edges: edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        for row in 0..d {
            let bias = row * row_vertex_num;
            code.vertices[bias + d - 1].is_virtual = true;
            code.vertices[bias + d].is_virtual = true;
        }
        let mut positions = Vec::new();
        for row in 0..d {
            let pos_i = row as f64;
            for i in 0..d {
                positions.push(VisualizePosition::new(pos_i, i as f64, 0.));
            }
            positions.push(VisualizePosition::new(pos_i, -1., 0.));
        }
        for i in 0..vertex_num {
            code.vertices[i].position = positions[i].clone();
        }
        code
    }


}


#[cfg(test)]
mod tests {
    use super::*;

    fn visualize_code(code: &mut impl ExampleCode, visualize_filename: String) {
        print_visualize_link(&visualize_filename);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
        visualizer.snapshot(format!("code"), code).unwrap();
    }

    #[test]
    fn example_code_capacity_repetition_code() {  // cargo test example_code_capacity_repetition_code -- --nocapture
        let mut code = CodeCapacityRepetitionCode::new(7, 0.1, 10000);
        code.sanity_check().unwrap();
        visualize_code(&mut code, format!("example_code_capacity_repetition_code.json"));
    }

    #[test]
    fn example_code_capacity_planar_code() {  // cargo test example_code_capacity_planar_code -- --nocapture
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, 10000);
        code.sanity_check().unwrap();
        visualize_code(&mut code, format!("example_code_capacity_planar_code.json"));
    }

}
