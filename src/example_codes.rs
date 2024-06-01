//! Example Decoding
//!
//! This module contains several abstract decoding graph and it's randomized simulator utilities.
//! This helps to debug, but it doesn't corresponds to real noise model, nor it's capable of simulating circuit-level noise model.
//! For complex noise model and simulator functionality, please see <https://github.com/yuewuo/QEC-Playground>
//!
//! Note that these examples are not optimized for cache for simplicity.
//! To maximize code efficiency, user should design how to group vertices such that memory speed is constant for arbitrary large code distance.
//!

use super::pointers::*;
use super::util::*;
use super::visualize::*;
use crate::derivative::Derivative;
use crate::rand_xoshiro::rand_core::SeedableRng;
use crate::rayon::prelude::*;
use crate::serde_json;
#[cfg(feature = "python_binding")]
use pyo3::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};

/// Vertex corresponds to a stabilizer measurement bit
#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct CodeVertex {
    /// position helps to visualize
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub position: VisualizePosition,
    /// neighbor edges helps to set find individual edge
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub neighbor_edges: Vec<EdgeIndex>,
    /// virtual vertex won't report measurement results
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub is_virtual: bool,
    /// whether it's a defect, note that virtual nodes should NOT be defects
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub is_defect: bool,
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl CodeVertex {
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// Edge flips the measurement result of two vertices
#[derive(Derivative, Clone)]
#[derivative(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct CodeEdge {
    /// the two vertices incident to this edge
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: (VertexIndex, VertexIndex),
    /// probability of flipping the results of these two vertices; do not set p to 0 to remove edge: if desired, create a new code type
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub p: f64,
    /// probability of having a reported event of error on this edge
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub pe: f64,
    /// the integer weight of this edge
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub half_weight: Weight,
    /// whether this edge is erased
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub is_erasure: bool,
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl CodeEdge {
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new(a: VertexIndex, b: VertexIndex) -> Self {
        Self {
            vertices: (a, b),
            p: 0.,
            pe: 0.,
            half_weight: 0,
            is_erasure: false,
        }
    }
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// default function for computing (pre-scaled) weight from probability
#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn weight_of_p(p: f64) -> f64 {
    assert!((0. ..=0.5).contains(&p), "p must be a reasonable value between 0 and 50%");
    ((1. - p) / p).ln()
}

pub trait ExampleCode {
    /// get mutable references to vertices and edges
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>);
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>);

    /// get the number of vertices
    fn vertex_num(&self) -> VertexNum {
        self.immutable_vertices_edges().0.len() as VertexNum
    }

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
            edge.half_weight = if half_weight == 0 { 1 } else { half_weight }; // weight is required to be even
        }
    }

    /// sanity check to avoid duplicate edges that are hard to debug
    fn sanity_check(&self) -> Result<(), String> {
        let (vertices, edges) = self.immutable_vertices_edges();
        // check the graph is reasonable
        if vertices.is_empty() || edges.is_empty() {
            return Err("empty graph".to_string());
        }
        // check duplicated edges
        let mut existing_edges = HashMap::<(VertexIndex, VertexIndex), EdgeIndex>::with_capacity(edges.len() * 2);
        for (edge_idx, edge) in edges.iter().enumerate() {
            let (v1, v2) = edge.vertices;
            let unique_edge = if v1 < v2 { (v1, v2) } else { (v2, v1) };
            if existing_edges.contains_key(&unique_edge) {
                let previous_idx = existing_edges[&unique_edge];
                return Err(format!(
                    "duplicate edge {} and {} with incident vertices {} and {}",
                    previous_idx, edge_idx, v1, v2
                ));
            }
            existing_edges.insert(unique_edge, edge_idx as EdgeIndex);
        }
        // check duplicated referenced edge from each vertex
        for (vertex_idx, vertex) in vertices.iter().enumerate() {
            let mut existing_edges = HashMap::<EdgeIndex, ()>::new();
            if vertex.neighbor_edges.is_empty() {
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

    /// set erasure probability of all edges; user can set individual probabilities
    fn set_erasure_probability(&mut self, pe: f64) {
        let (_vertices, edges) = self.vertices_edges();
        for edge in edges.iter_mut() {
            edge.pe = pe;
        }
    }

    /// automatically create vertices given edges
    #[allow(clippy::unnecessary_cast)]
    fn fill_vertices(&mut self, vertex_num: VertexNum) {
        let (vertices, edges) = self.vertices_edges();
        vertices.clear();
        vertices.reserve(vertex_num as usize);
        for _ in 0..vertex_num {
            vertices.push(CodeVertex {
                position: VisualizePosition::new(0., 0., 0.),
                neighbor_edges: Vec::new(),
                is_virtual: false,
                is_defect: false,
            });
        }
        for (edge_idx, edge) in edges.iter().enumerate() {
            let vertex_1 = &mut vertices[edge.vertices.0 as usize];
            vertex_1.neighbor_edges.push(edge_idx as EdgeIndex);
            let vertex_2 = &mut vertices[edge.vertices.1 as usize];
            vertex_2.neighbor_edges.push(edge_idx as EdgeIndex);
        }
    }

    /// gather all positions of vertices
    fn get_positions(&self) -> Vec<VisualizePosition> {
        let (vertices, _edges) = self.immutable_vertices_edges();
        let mut positions = Vec::with_capacity(vertices.len());
        for vertex in vertices.iter() {
            positions.push(vertex.position.clone());
        }
        positions
    }

    /// generate standard interface to instantiate Fusion blossom solver
    fn get_initializer(&self) -> SolverInitializer {
        let (vertices, edges) = self.immutable_vertices_edges();
        let vertex_num = vertices.len() as VertexIndex;
        let mut weighted_edges = Vec::with_capacity(edges.len());
        for edge in edges.iter() {
            weighted_edges.push((edge.vertices.0, edge.vertices.1, edge.half_weight * 2));
        }
        let mut virtual_vertices = Vec::new();
        for (vertex_idx, vertex) in vertices.iter().enumerate() {
            if vertex.is_virtual {
                virtual_vertices.push(vertex_idx as VertexIndex);
            }
        }
        SolverInitializer {
            vertex_num,
            weighted_edges,
            virtual_vertices,
        }
    }

    /// set defect vertices (non-trivial measurement result in case of single round of measurement,
    /// or different result from the previous round in case of multiple rounds of measurement)
    #[allow(clippy::unnecessary_cast)]
    fn set_defect_vertices(&mut self, defect_vertices: &[VertexIndex]) {
        let (vertices, _edges) = self.vertices_edges();
        for vertex in vertices.iter_mut() {
            vertex.is_defect = false;
        }
        for vertex_idx in defect_vertices.iter() {
            let vertex = &mut vertices[*vertex_idx as usize];
            vertex.is_defect = true;
        }
    }

    /// set erasure edges
    #[allow(clippy::unnecessary_cast)]
    fn set_erasures(&mut self, erasures: &[EdgeIndex]) {
        let (_vertices, edges) = self.vertices_edges();
        for edge in edges.iter_mut() {
            edge.is_erasure = false;
        }
        for edge_idx in erasures.iter() {
            let edge = &mut edges[*edge_idx as usize];
            edge.is_erasure = true;
        }
    }

    /// set syndrome
    fn set_syndrome(&mut self, syndrome_pattern: &SyndromePattern) {
        self.set_defect_vertices(&syndrome_pattern.defect_vertices);
        self.set_erasures(&syndrome_pattern.erasures);
    }

    /// get current defect vertices
    fn get_defect_vertices(&self) -> Vec<VertexIndex> {
        let (vertices, _edges) = self.immutable_vertices_edges();
        let mut syndrome = Vec::new();
        for (vertex_idx, vertex) in vertices.iter().enumerate() {
            if vertex.is_defect {
                syndrome.push(vertex_idx as VertexIndex);
            }
        }
        syndrome
    }

    /// get current erasure edges
    fn get_erasures(&self) -> Vec<EdgeIndex> {
        let (_vertices, edges) = self.immutable_vertices_edges();
        let mut erasures = Vec::new();
        for (edge_idx, edge) in edges.iter().enumerate() {
            if edge.is_erasure {
                erasures.push(edge_idx as EdgeIndex);
            }
        }
        erasures
    }

    /// get current syndrome
    fn get_syndrome(&self) -> SyndromePattern {
        SyndromePattern::new(self.get_defect_vertices(), self.get_erasures())
    }

    /// generate random errors based on the edge probabilities and a seed for pseudo number generator
    #[allow(clippy::unnecessary_cast)]
    fn generate_random_errors(&mut self, seed: u64) -> SyndromePattern {
        let mut rng = DeterministicRng::seed_from_u64(seed);
        let (vertices, edges) = self.vertices_edges();
        for vertex in vertices.iter_mut() {
            vertex.is_defect = false;
        }
        for edge in edges.iter_mut() {
            let p = if rng.next_f64() < edge.pe {
                edge.is_erasure = true;
                0.5 // when erasure happens, there are 50% chance of error
            } else {
                edge.is_erasure = false;
                edge.p
            };
            if rng.next_f64() < p {
                let (v1, v2) = edge.vertices;
                let vertex_1 = &mut vertices[v1 as usize];
                if !vertex_1.is_virtual {
                    vertex_1.is_defect = !vertex_1.is_defect;
                }
                let vertex_2 = &mut vertices[v2 as usize];
                if !vertex_2.is_virtual {
                    vertex_2.is_defect = !vertex_2.is_defect;
                }
            }
        }
        self.get_syndrome()
    }

    #[allow(clippy::unnecessary_cast)]
    fn generate_errors(&mut self, edge_indices: &[EdgeIndex]) -> SyndromePattern {
        let (vertices, edges) = self.vertices_edges();
        for &edge_index in edge_indices {
            let edge = &mut edges.get_mut(edge_index as usize).unwrap();
            let (v1, v2) = edge.vertices;
            let vertex_1 = &mut vertices[v1 as usize];
            if !vertex_1.is_virtual {
                vertex_1.is_defect = !vertex_1.is_defect;
            }
            let vertex_2 = &mut vertices[v2 as usize];
            if !vertex_2.is_virtual {
                vertex_2.is_defect = !vertex_2.is_defect;
            }
        }
        self.get_syndrome()
    }

    fn clear_errors(&mut self) {
        let (vertices, edges) = self.vertices_edges();
        for vertex in vertices.iter_mut() {
            vertex.is_defect = false;
        }
        for edge in edges.iter_mut() {
            edge.is_erasure = true;
        }
    }

    fn is_virtual(&self, vertex_idx: usize) -> bool {
        let (vertices, _edges) = self.immutable_vertices_edges();
        vertices[vertex_idx].is_virtual
    }

    fn is_defect(&self, vertex_idx: usize) -> bool {
        let (vertices, _edges) = self.immutable_vertices_edges();
        vertices[vertex_idx].is_defect
    }

    /// reorder the vertices such that new vertices (the indices of the old order) is sequential
    #[allow(clippy::unnecessary_cast)]
    fn reorder_vertices(&mut self, sequential_vertices: &[VertexIndex]) {
        let (vertices, edges) = self.vertices_edges();
        assert_eq!(vertices.len(), sequential_vertices.len(), "amount of vertices must be same");
        let old_to_new = build_old_to_new(sequential_vertices);
        // change the vertices numbering
        *vertices = (0..vertices.len())
            .map(|new_index| vertices[sequential_vertices[new_index] as usize].clone())
            .collect();
        for edge in edges.iter_mut() {
            let (old_left, old_right) = edge.vertices;
            edge.vertices = (
                old_to_new[old_left as usize].unwrap(),
                old_to_new[old_right as usize].unwrap(),
            );
        }
    }
}

#[cfg(feature = "python_binding")]
use rand::{thread_rng, Rng};

#[cfg(feature = "python_binding")]
macro_rules! bind_trait_example_code {
    ($struct_name:ident) => {
        #[pymethods]
        impl $struct_name {
            fn __repr__(&self) -> String {
                format!("{:?}", self)
            }
            #[pyo3(name = "vertex_num")]
            fn trait_vertex_num(&self) -> VertexNum {
                self.vertex_num()
            }
            #[pyo3(name = "compute_weights")]
            fn trait_compute_weights(&mut self, max_half_weight: Weight) {
                self.compute_weights(max_half_weight)
            }
            #[pyo3(name = "sanity_check")]
            fn trait_sanity_check(&self) -> Option<String> {
                self.sanity_check().err()
            }
            #[pyo3(name = "set_probability")]
            fn trait_set_probability(&mut self, p: f64) {
                self.set_probability(p)
            }
            #[pyo3(name = "set_erasure_probability")]
            fn trait_set_erasure_probability(&mut self, p: f64) {
                self.set_erasure_probability(p)
            }
            #[pyo3(name = "fill_vertices")]
            fn trait_fill_vertices(&mut self, vertex_num: VertexNum) {
                self.fill_vertices(vertex_num)
            }
            #[pyo3(name = "get_positions")]
            fn trait_get_positions(&self) -> Vec<VisualizePosition> {
                self.get_positions()
            }
            #[pyo3(name = "get_initializer")]
            fn trait_get_initializer(&self) -> SolverInitializer {
                self.get_initializer()
            }
            #[pyo3(name = "set_defect_vertices")]
            fn trait_set_defect_vertices(&mut self, defect_vertices: Vec<VertexIndex>) {
                self.set_defect_vertices(&defect_vertices)
            }
            #[pyo3(name = "set_erasures")]
            fn trait_set_erasures(&mut self, erasures: Vec<EdgeIndex>) {
                self.set_erasures(&erasures)
            }
            #[pyo3(name = "set_syndrome")]
            fn trait_set_syndrome(&mut self, syndrome_pattern: &SyndromePattern) {
                self.set_syndrome(syndrome_pattern)
            }
            #[pyo3(name = "get_defect_vertices")]
            fn trait_get_defect_vertices(&self) -> Vec<VertexIndex> {
                self.get_defect_vertices()
            }
            #[pyo3(name = "get_erasures")]
            fn trait_get_erasures(&self) -> Vec<EdgeIndex> {
                self.get_erasures()
            }
            #[pyo3(name = "get_syndrome")]
            fn trait_get_syndrome(&self) -> SyndromePattern {
                self.get_syndrome()
            }
            #[pyo3(name = "generate_random_errors", signature = (seed=thread_rng().gen()))]
            fn trait_generate_random_errors(&mut self, seed: u64) -> SyndromePattern {
                self.generate_random_errors(seed)
            }
            #[pyo3(name = "generate_errors")]
            fn trait_generate_errors(&mut self, edge_indices: Vec<EdgeIndex>) -> SyndromePattern {
                self.generate_errors(&edge_indices)
            }
            #[pyo3(name = "clear_errors")]
            fn trait_clear_errors(&mut self) {
                self.clear_errors()
            }
            #[pyo3(name = "is_virtual")]
            fn trait_is_virtual(&mut self, vertex_idx: usize) -> bool {
                self.is_virtual(vertex_idx)
            }
            #[pyo3(name = "is_defect")]
            fn trait_is_defect(&mut self, vertex_idx: usize) -> bool {
                self.is_defect(vertex_idx)
            }
            #[pyo3(name = "reorder_vertices")]
            fn trait_reorder_vertices(&mut self, sequential_vertices: Vec<VertexIndex>) {
                self.reorder_vertices(&sequential_vertices)
            }
            #[pyo3(name = "snapshot", signature = (abbrev=true))]
            fn trait_snapshot(&mut self, abbrev: bool) -> PyObject {
                json_to_pyobject(self.snapshot(abbrev))
            }
        }
    };
}

impl<T> FusionVisualizer for T
where
    T: ExampleCode,
{
    fn snapshot(&self, abbrev: bool) -> serde_json::Value {
        let (self_vertices, self_edges) = self.immutable_vertices_edges();
        let mut vertices = Vec::<serde_json::Value>::new();
        for vertex in self_vertices.iter() {
            vertices.push(json!({
                if abbrev { "v" } else { "is_virtual" }: i32::from(vertex.is_virtual),
                if abbrev { "s" } else { "is_defect" }: i32::from(vertex.is_defect),
            }));
        }
        let mut edges = Vec::<serde_json::Value>::new();
        for edge in self_edges.iter() {
            edges.push(json!({
                if abbrev { "w" } else { "weight" }: edge.half_weight * 2,
                if abbrev { "l" } else { "left" }: edge.vertices.0,
                if abbrev { "r" } else { "right" }: edge.vertices.1,
                // code itself is not capable of calculating growth
            }));
        }
        json!({
            "vertices": vertices,  // TODO: update HTML code to use the same language
            "edges": edges,
        })
    }
}

/// perfect quantum repetition code
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct CodeCapacityRepetitionCode {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for CodeCapacityRepetitionCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {CodeCapacityRepetitionCode}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl CodeCapacityRepetitionCode {
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, p, max_half_weight = 500)))]
    pub fn new(d: VertexNum, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[allow(clippy::unnecessary_cast)]
    pub fn create_code(d: VertexNum) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let vertex_num = (d - 1) + 2; // two virtual vertices at left and right
                                      // create edges
        let mut edges = Vec::new();
        for i in 0..d - 1 {
            edges.push(CodeEdge::new(i, i + 1));
        }
        edges.push(CodeEdge::new(0, d)); // tje left-most edge
        let mut code = Self {
            vertices: Vec::new(),
            edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        code.vertices[d as usize - 1].is_virtual = true;
        code.vertices[d as usize].is_virtual = true;
        let mut positions = Vec::new();
        for i in 0..d {
            positions.push(VisualizePosition::new(0., i as f64, 0.));
        }
        positions.push(VisualizePosition::new(0., -1., 0.));
        for (i, position) in positions.into_iter().enumerate() {
            code.vertices[i].position = position;
        }
        code
    }
}

/// code capacity noise model is a single measurement round with perfect stabilizer measurements;
/// e.g. this is the decoding graph of a CSS surface code (standard one, not rotated one) with X-type stabilizers
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct CodeCapacityPlanarCode {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for CodeCapacityPlanarCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {CodeCapacityPlanarCode}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl CodeCapacityPlanarCode {
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, p, max_half_weight = 500)))]
    pub fn new(d: VertexNum, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[allow(clippy::unnecessary_cast)]
    pub fn create_code(d: VertexNum) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let row_vertex_num = (d - 1) + 2; // two virtual nodes at left and right
        let vertex_num = row_vertex_num * d; // `d` rows
                                             // create edges
        let mut edges = Vec::new();
        for row in 0..d {
            let bias = row * row_vertex_num;
            for i in 0..d - 1 {
                edges.push(CodeEdge::new(bias + i, bias + i + 1));
            }
            edges.push(CodeEdge::new(bias, bias + d)); // left most edge
            if row + 1 < d {
                for i in 0..d - 1 {
                    edges.push(CodeEdge::new(bias + i, bias + i + row_vertex_num));
                }
            }
        }
        let mut code = Self {
            vertices: Vec::new(),
            edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        for row in 0..d {
            let bias = row * row_vertex_num;
            code.vertices[(bias + d - 1) as usize].is_virtual = true;
            code.vertices[(bias + d) as usize].is_virtual = true;
        }
        let mut positions = Vec::new();
        for row in 0..d {
            let pos_i = row as f64;
            for i in 0..d {
                positions.push(VisualizePosition::new(pos_i, i as f64, 0.));
            }
            positions.push(VisualizePosition::new(pos_i, -1., 0.));
        }
        for (i, position) in positions.into_iter().enumerate() {
            code.vertices[i].position = position;
        }
        code
    }
}

/// phenomenological noise model is multiple measurement rounds adding only measurement errors
/// e.g. this is the decoding graph of a CSS surface code (standard one, not rotated one) with X-type stabilizers
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct PhenomenologicalPlanarCode {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for PhenomenologicalPlanarCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {PhenomenologicalPlanarCode}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl PhenomenologicalPlanarCode {
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, noisy_measurements, p, max_half_weight = 500)))]
    pub fn new(d: VertexNum, noisy_measurements: VertexNum, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d, noisy_measurements);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[allow(clippy::unnecessary_cast)]
    pub fn create_code(d: VertexNum, noisy_measurements: VertexNum) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let row_vertex_num = (d - 1) + 2; // two virtual nodes at left and right
        let t_vertex_num = row_vertex_num * d; // `d` rows
        let td = noisy_measurements + 1; // a perfect measurement round is capped at the end
        let vertex_num = t_vertex_num * td; // `td` layers
                                            // create edges
        let mut edges = Vec::new();
        for t in 0..td {
            let t_bias = t * t_vertex_num;
            for row in 0..d {
                let bias = t_bias + row * row_vertex_num;
                for i in 0..d - 1 {
                    edges.push(CodeEdge::new(bias + i, bias + i + 1));
                }
                edges.push(CodeEdge::new(bias, bias + d)); // left most edge
                if row + 1 < d {
                    for i in 0..d - 1 {
                        edges.push(CodeEdge::new(bias + i, bias + i + row_vertex_num));
                    }
                }
            }
            // inter-layer connection
            if t + 1 < td {
                for row in 0..d {
                    let bias = t_bias + row * row_vertex_num;
                    for i in 0..d - 1 {
                        edges.push(CodeEdge::new(bias + i, bias + i + t_vertex_num));
                    }
                }
            }
        }
        let mut code = Self {
            vertices: Vec::new(),
            edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        for t in 0..td {
            let t_bias = t * t_vertex_num;
            for row in 0..d {
                let bias = t_bias + row * row_vertex_num;
                code.vertices[(bias + d - 1) as usize].is_virtual = true;
                code.vertices[(bias + d) as usize].is_virtual = true;
            }
        }
        let mut positions = Vec::new();
        for t in 0..td {
            let pos_t = t as f64;
            for row in 0..d {
                let pos_i = row as f64;
                for i in 0..d {
                    positions.push(VisualizePosition::new(pos_i, i as f64 + 0.5, pos_t));
                }
                positions.push(VisualizePosition::new(pos_i, -1. + 0.5, pos_t));
            }
        }
        for (i, position) in positions.into_iter().enumerate() {
            code.vertices[i].position = position;
        }
        code
    }
}

/// (not accurate) circuit-level noise model is multiple measurement rounds with errors between each two-qubit gates
/// e.g. this is the decoding graph of a CSS surface code (standard one, not rotated one) with X-type stabilizers
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct CircuitLevelPlanarCode {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for CircuitLevelPlanarCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {CircuitLevelPlanarCode}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl CircuitLevelPlanarCode {
    /// by default diagonal edge has error rate p/3 to mimic the behavior of unequal weights
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, noisy_measurements, p, max_half_weight = 500)))]
    pub fn new(d: VertexNum, noisy_measurements: VertexNum, p: f64, max_half_weight: Weight) -> Self {
        Self::new_diagonal(d, noisy_measurements, p, max_half_weight, Some(p / 3.))
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, noisy_measurements, p, max_half_weight = 500, diagonal_p = None)))]
    #[allow(clippy::unnecessary_cast)]
    pub fn new_diagonal(
        d: VertexNum,
        noisy_measurements: VertexNum,
        p: f64,
        max_half_weight: Weight,
        diagonal_p: Option<f64>,
    ) -> Self {
        let mut code = Self::create_code(d, noisy_measurements);
        code.set_probability(p);
        if let Some(diagonal_p) = diagonal_p {
            let (vertices, edges) = code.vertices_edges();
            for edge in edges.iter_mut() {
                let (v1, v2) = edge.vertices;
                let v1p = &vertices[v1 as usize].position;
                let v2p = &vertices[v2 as usize].position;
                let manhattan_distance = (v1p.i - v2p.i).abs() + (v1p.j - v2p.j).abs() + (v1p.t - v2p.t).abs();
                if manhattan_distance > 1. {
                    edge.p = diagonal_p;
                }
            }
        }
        code.compute_weights(max_half_weight);
        code
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[allow(clippy::unnecessary_cast)]
    pub fn create_code(d: VertexNum, noisy_measurements: VertexNum) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let row_vertex_num = (d - 1) + 2; // two virtual nodes at left and right
        let t_vertex_num = row_vertex_num * d; // `d` rows
        let td = noisy_measurements + 1; // a perfect measurement round is capped at the end
        let vertex_num = t_vertex_num * td; // `td` layers
                                            // create edges
        let mut edges = Vec::new();
        for t in 0..td {
            let t_bias = t * t_vertex_num;
            for row in 0..d {
                let bias = t_bias + row * row_vertex_num;
                for i in 0..d - 1 {
                    edges.push(CodeEdge::new(bias + i, bias + i + 1));
                }
                edges.push(CodeEdge::new(bias, bias + d)); // left most edge
                if row + 1 < d {
                    for i in 0..d - 1 {
                        edges.push(CodeEdge::new(bias + i, bias + i + row_vertex_num));
                    }
                }
            }
            // inter-layer connection
            if t + 1 < td {
                for row in 0..d {
                    let bias = t_bias + row * row_vertex_num;
                    for i in 0..d - 1 {
                        edges.push(CodeEdge::new(bias + i, bias + i + t_vertex_num));
                        let diagonal_diffs: Vec<(isize, isize)> = vec![(0, 1), (1, 0), (1, 1)];
                        for (di, dj) in diagonal_diffs {
                            let new_row = row as isize + di; // row corresponds to `i`
                            let new_i = i as isize + dj; // i corresponds to `j`
                            if new_row >= 0 && new_i >= 0 && new_row < d as isize && new_i < (d - 1) as isize {
                                let new_bias = t_bias + (new_row as VertexNum) * row_vertex_num + t_vertex_num;
                                edges.push(CodeEdge::new(bias + i, new_bias + new_i as VertexNum));
                            }
                        }
                    }
                }
            }
        }
        let mut code = Self {
            vertices: Vec::new(),
            edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        for t in 0..td {
            let t_bias = t * t_vertex_num;
            for row in 0..d {
                let bias = t_bias + row * row_vertex_num;
                code.vertices[(bias + d - 1) as usize].is_virtual = true;
                code.vertices[(bias + d) as usize].is_virtual = true;
            }
        }
        let mut positions = Vec::new();
        for t in 0..td {
            let pos_t = t as f64;
            for row in 0..d {
                let pos_i = row as f64;
                for i in 0..d {
                    positions.push(VisualizePosition::new(pos_i, i as f64 + 0.5, pos_t));
                }
                positions.push(VisualizePosition::new(pos_i, -1. + 0.5, pos_t));
            }
        }
        for (i, position) in positions.into_iter().enumerate() {
            code.vertices[i].position = position;
        }
        code
    }
}

/// CSS surface code (the rotated one) with X-type stabilizers
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct CodeCapacityRotatedCode {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for CodeCapacityRotatedCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {CodeCapacityRotatedCode}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl CodeCapacityRotatedCode {
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, p, max_half_weight = 500)))]
    pub fn new(d: VertexNum, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[allow(clippy::unnecessary_cast)]
    pub fn create_code(d: VertexNum) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let row_vertex_num = (d - 1) / 2 + 1; // a virtual node at either left or right
        let vertex_num = row_vertex_num * (d + 1); // d+1 rows
                                                   // create edges
        let mut edges = Vec::new();
        for row in 0..d {
            let bias = row * row_vertex_num;
            if row % 2 == 0 {
                for i in 0..d {
                    if i % 2 == 0 {
                        edges.push(CodeEdge::new(bias + i / 2, bias + row_vertex_num + i / 2));
                    } else {
                        edges.push(CodeEdge::new(bias + (i - 1) / 2, bias + row_vertex_num + (i + 1) / 2));
                    }
                }
            } else {
                for i in 0..d {
                    if i % 2 == 0 {
                        edges.push(CodeEdge::new(bias + i / 2, bias + row_vertex_num + i / 2));
                    } else {
                        edges.push(CodeEdge::new(bias + (i + 1) / 2, bias + row_vertex_num + (i - 1) / 2));
                    }
                }
            }
        }
        let mut code = Self {
            vertices: Vec::new(),
            edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        for row in 0..d + 1 {
            let bias = row * row_vertex_num;
            if row % 2 == 0 {
                code.vertices[(bias + row_vertex_num - 1) as usize].is_virtual = true;
            } else {
                code.vertices[(bias) as usize].is_virtual = true;
            }
        }
        let mut positions = Vec::new();
        for row in 0..d + 1 {
            let pos_i = row as f64;
            for i in 0..row_vertex_num {
                let pos_bias = (row % 2 == 0) as VertexNum;
                positions.push(VisualizePosition::new(pos_i, (i * 2 + pos_bias) as f64, 0.));
            }
        }
        for (i, position) in positions.into_iter().enumerate() {
            code.vertices[i].position = position;
        }
        code
    }
}

/// CSS surface code (the rotated one) with X-type stabilizers
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct PhenomenologicalRotatedCode {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

impl ExampleCode for PhenomenologicalRotatedCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {PhenomenologicalRotatedCode}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl PhenomenologicalRotatedCode {
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (d, noisy_measurements, p, max_half_weight = 500)))]
    pub fn new(d: VertexNum, noisy_measurements: VertexNum, p: f64, max_half_weight: Weight) -> Self {
        let mut code = Self::create_code(d, noisy_measurements);
        code.set_probability(p);
        code.compute_weights(max_half_weight);
        code
    }

    #[cfg_attr(feature = "python_binding", staticmethod)]
    #[allow(clippy::unnecessary_cast)]
    pub fn create_code(d: VertexNum, noisy_measurements: VertexNum) -> Self {
        assert!(d >= 3 && d % 2 == 1, "d must be odd integer >= 3");
        let row_vertex_num = (d - 1) / 2 + 1; // a virtual node at either left or right
        let t_vertex_num = row_vertex_num * (d + 1); // d+1 rows
        let td = noisy_measurements + 1; // a perfect measurement round is capped at the end
        let vertex_num = t_vertex_num * td; // `td` layers
                                            // create edges
        let mut edges = Vec::new();
        for t in 0..td {
            let t_bias = t * t_vertex_num;
            for row in 0..d {
                let bias = t_bias + row * row_vertex_num;
                if row % 2 == 0 {
                    for i in 0..d {
                        if i % 2 == 0 {
                            edges.push(CodeEdge::new(bias + i / 2, bias + row_vertex_num + i / 2));
                        } else {
                            edges.push(CodeEdge::new(bias + (i - 1) / 2, bias + row_vertex_num + (i + 1) / 2));
                        }
                    }
                } else {
                    for i in 0..d {
                        if i % 2 == 0 {
                            edges.push(CodeEdge::new(bias + i / 2, bias + row_vertex_num + i / 2));
                        } else {
                            edges.push(CodeEdge::new(bias + (i + 1) / 2, bias + row_vertex_num + (i - 1) / 2));
                        }
                    }
                }
            }
            // inter-layer connection
            if t + 1 < td {
                for row in 0..d + 1 {
                    let bias = t_bias + row * row_vertex_num;
                    for i in 0..row_vertex_num {
                        edges.push(CodeEdge::new(bias + i, bias + i + t_vertex_num));
                    }
                }
            }
        }
        let mut code = Self {
            vertices: Vec::new(),
            edges,
        };
        // create vertices
        code.fill_vertices(vertex_num);
        for t in 0..td {
            let t_bias = t * t_vertex_num;
            for row in 0..d + 1 {
                let bias = t_bias + row * row_vertex_num;
                if row % 2 == 0 {
                    code.vertices[(bias + row_vertex_num - 1) as usize].is_virtual = true;
                } else {
                    code.vertices[(bias) as usize].is_virtual = true;
                }
            }
        }
        let mut positions = Vec::new();
        for t in 0..td {
            let pos_t = t as f64 * 2f64.sqrt();
            for row in 0..d + 1 {
                let pos_i = row as f64;
                for i in 0..row_vertex_num {
                    let pos_bias = (row % 2 == 0) as VertexNum;
                    positions.push(VisualizePosition::new(pos_i, (i * 2 + pos_bias) as f64, pos_t));
                }
            }
        }
        for (i, position) in positions.into_iter().enumerate() {
            code.vertices[i].position = position;
        }
        code
    }
}

/// example code with QEC-Playground as simulator
#[cfg(feature = "qecp_integrate")]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct QECPlaygroundCode {
    simulator: qecp::simulator::Simulator,
    noise_model: std::sync::Arc<qecp::noise_model::NoiseModel>,
    adaptor: std::sync::Arc<qecp::decoder_fusion::FusionBlossomAdaptor>,
    vertex_index_map: std::sync::Arc<HashMap<usize, VertexIndex>>,
    edge_index_map: std::sync::Arc<HashMap<usize, EdgeIndex>>,
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
}

#[cfg(feature = "qecp_integrate")]
impl ExampleCode for QECPlaygroundCode {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
    // override simulation function
    #[allow(clippy::unnecessary_cast)]
    fn generate_random_errors(&mut self, seed: u64) -> SyndromePattern {
        use qecp::simulator::SimulatorGenerics;
        let rng = qecp::reproducible_rand::Xoroshiro128StarStar::seed_from_u64(seed);
        self.simulator.set_rng(rng);
        let (error_count, erasure_count) = self.simulator.generate_random_errors(&self.noise_model);
        let sparse_detected_erasures = if erasure_count != 0 {
            self.simulator.generate_sparse_detected_erasures()
        } else {
            qecp::simulator::SparseErasures::new()
        };
        let sparse_measurement = if error_count != 0 {
            self.simulator.generate_sparse_measurement()
        } else {
            qecp::simulator::SparseMeasurement::new()
        };
        let syndrome_pattern = self
            .adaptor
            .generate_syndrome_pattern(&sparse_measurement, &sparse_detected_erasures);
        for vertex in self.vertices.iter_mut() {
            vertex.is_defect = false;
        }
        for &vertex_index in syndrome_pattern.defect_vertices.iter() {
            if let Some(new_index) = self.vertex_index_map.get(&vertex_index) {
                self.vertices[*new_index as usize].is_defect = true;
            }
        }
        for edge in self.edges.iter_mut() {
            edge.is_erasure = false;
        }
        for &edge_index in syndrome_pattern.erasures.iter() {
            if let Some(new_index) = self.edge_index_map.get(&edge_index) {
                self.edges[*new_index as usize].is_erasure = true;
            }
        }
        self.get_syndrome()
    }
}

#[cfg(all(feature = "qecp_integrate", feature = "python_binding"))]
bind_trait_example_code! {QECPlaygroundCode}

#[cfg(feature = "qecp_integrate")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QECPlaygroundCodeConfig {
    // default to d
    pub di: Option<usize>,
    pub dj: Option<usize>,
    pub nm: Option<usize>,
    #[serde(default = "qec_playground_default_configs::pe")]
    pub pe: f64,
    pub noise_model_modifier: Option<serde_json::Value>,
    #[serde(default = "qec_playground_default_configs::code_type")]
    pub code_type: qecp::code_builder::CodeType,
    #[serde(default = "qec_playground_default_configs::bias_eta")]
    pub bias_eta: f64,
    pub noise_model: Option<qecp::noise_model_builder::NoiseModelBuilder>,
    #[serde(default = "qec_playground_default_configs::noise_model_configuration")]
    pub noise_model_configuration: serde_json::Value,
    #[serde(default = "qec_playground_default_configs::parallel_init")]
    pub parallel_init: usize,
    #[serde(default = "qec_playground_default_configs::use_brief_edge")]
    pub use_brief_edge: bool,
    // specify the target qubit type
    pub qubit_type: Option<qecp::types::QubitType>,
    #[serde(default = "qecp::decoder_fusion::fusion_default_configs::max_half_weight")]
    pub max_half_weight: usize,
    #[serde(default = "qec_playground_default_configs::trim_isolated_vertices")]
    pub trim_isolated_vertices: bool,
}

#[cfg(feature = "qecp_integrate")]
pub mod qec_playground_default_configs {
    pub fn pe() -> f64 {
        0.
    }
    pub fn bias_eta() -> f64 {
        0.5
    }
    pub fn noise_model_configuration() -> serde_json::Value {
        json!({})
    }
    pub fn code_type() -> qecp::code_builder::CodeType {
        qecp::code_builder::CodeType::StandardPlanarCode
    }
    pub fn parallel_init() -> usize {
        1
    }
    pub fn use_brief_edge() -> bool {
        false
    }
    pub fn trim_isolated_vertices() -> bool {
        true
    }
}

#[cfg(feature = "qecp_integrate")]
impl QECPlaygroundCode {
    #[allow(clippy::unnecessary_cast)]
    pub fn new(d: usize, p: f64, config: serde_json::Value) -> Self {
        let config: QECPlaygroundCodeConfig = serde_json::from_value(config).unwrap();
        let di = config.di.unwrap_or(d);
        let dj = config.dj.unwrap_or(d);
        let nm = config.nm.unwrap_or(d);
        let mut simulator = qecp::simulator::Simulator::new(config.code_type, qecp::code_builder::CodeSize::new(nm, di, dj));
        let mut noise_model = qecp::noise_model::NoiseModel::new(&simulator);
        let px = p / (1. + config.bias_eta) / 2.;
        let py = px;
        let pz = p - 2. * px;
        simulator.set_error_rates(&mut noise_model, px, py, pz, config.pe);
        // apply customized noise model
        if let Some(noise_model_builder) = &config.noise_model {
            noise_model_builder.apply(
                &mut simulator,
                &mut noise_model,
                &config.noise_model_configuration,
                p,
                config.bias_eta,
                config.pe,
            );
        }
        simulator.compress_error_rates(&mut noise_model); // by default compress all error rates
        let noise_model = std::sync::Arc::new(noise_model);
        // construct vertices and edges
        let fusion_decoder = qecp::decoder_fusion::FusionDecoder::new(
            &simulator,
            noise_model.clone(),
            &serde_json::from_value(json!({
                "max_half_weight": config.max_half_weight
            }))
            .unwrap(),
            config.parallel_init,
            config.use_brief_edge,
        );
        let adaptor = fusion_decoder.adaptor;
        let initializer = &adaptor.initializer;
        let positions = &adaptor.positions;
        let mut vertex_index_map = HashMap::new();
        // filter the specific qubit type and also remove isolated virtual vertices
        let is_vertex_isolated = if config.trim_isolated_vertices {
            let mut is_vertex_isolated = vec![true; initializer.vertex_num];
            for (left_vertex, right_vertex, _) in initializer.weighted_edges.iter().cloned() {
                is_vertex_isolated[left_vertex] = false;
                is_vertex_isolated[right_vertex] = false;
            }
            is_vertex_isolated
        } else {
            vec![false; initializer.vertex_num]
        };
        for (vertex_index, is_isolated) in is_vertex_isolated.iter().cloned().enumerate() {
            let position = &adaptor.vertex_to_position_mapping[vertex_index];
            let qubit_type = simulator.get_node(position).as_ref().unwrap().qubit_type;
            if !config.qubit_type.is_some_and(|expect| expect != qubit_type) && !is_isolated {
                let new_index = vertex_index_map.len() as VertexIndex;
                vertex_index_map.insert(vertex_index, new_index);
            }
        }
        let mut code = Self {
            simulator,
            noise_model,
            adaptor: adaptor.clone(),
            vertex_index_map: std::sync::Arc::new(vertex_index_map),
            edge_index_map: std::sync::Arc::new(HashMap::new()), // overwrite later
            vertices: Vec::with_capacity(initializer.vertex_num),
            edges: Vec::with_capacity(initializer.weighted_edges.len()),
        };
        let mut edge_index_map = HashMap::new();
        for (edge_index, (left_vertex, right_vertex, weight)) in initializer.weighted_edges.iter().cloned().enumerate() {
            assert!(weight % 2 == 0, "weight must be even number");
            let contains_left = code.vertex_index_map.contains_key(&left_vertex);
            let contains_right = code.vertex_index_map.contains_key(&right_vertex);
            assert_eq!(contains_left, contains_right, "should not connect different type of qubits");
            if contains_left {
                let new_index = edge_index_map.len() as EdgeIndex;
                edge_index_map.insert(edge_index, new_index);
                code.edges.push(CodeEdge {
                    vertices: (code.vertex_index_map[&left_vertex], code.vertex_index_map[&right_vertex]),
                    p: 0.,  // doesn't matter
                    pe: 0., // doesn't matter
                    half_weight: (weight as Weight) / 2,
                    is_erasure: false, // doesn't matter
                });
            }
        }
        code.edge_index_map = std::sync::Arc::new(edge_index_map);
        // automatically create the vertices and nearest-neighbor connection
        code.fill_vertices(code.vertex_index_map.len() as VertexNum);
        // set virtual vertices and positions
        for (vertex_index, position) in positions.iter().cloned().enumerate() {
            if let Some(new_index) = code.vertex_index_map.get(&vertex_index) {
                code.vertices[*new_index as usize].position = VisualizePosition::new(position.i, position.j, position.t);
            }
        }
        for vertex_index in initializer.virtual_vertices.iter() {
            if let Some(new_index) = code.vertex_index_map.get(vertex_index) {
                code.vertices[*new_index as usize].is_virtual = true;
            }
        }
        code
    }
}

/// read from file, including the error patterns;
/// the point is to avoid bad cache performance, because generating random error requires iterating over a large memory space,
/// invalidating all cache. also, this can reduce the time of decoding by prepare the data before hand and could be shared between
/// different partition configurations
#[derive(Clone, Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct ErrorPatternReader {
    /// vertices in the code
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertices: Vec<CodeVertex>,
    /// nearest-neighbor edges in the decoding graph
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub edges: Vec<CodeEdge>,
    /// pre-generated syndrome patterns
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub syndrome_patterns: Vec<SyndromePattern>,
    /// cursor of current errors
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub defect_index: usize,
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub cyclic_syndrome: bool,
}

impl ExampleCode for ErrorPatternReader {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        (&mut self.vertices, &mut self.edges)
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        (&self.vertices, &self.edges)
    }
    fn generate_random_errors(&mut self, _seed: u64) -> SyndromePattern {
        if self.cyclic_syndrome {
            if self.defect_index >= self.syndrome_patterns.len() {
                self.defect_index = 0; // cyclic
            }
        } else {
            assert!(
                self.defect_index < self.syndrome_patterns.len(),
                "reading syndrome pattern more than in the file, consider generate the file with more data points"
            );
        }
        let syndrome_pattern = self.syndrome_patterns[self.defect_index].clone();
        self.defect_index += 1;
        syndrome_pattern
    }
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl ErrorPatternReader {
    #[allow(clippy::unnecessary_cast)]
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (filename, cyclic_syndrome = false)))]
    pub fn py_new(filename: String, cyclic_syndrome: bool) -> Self {
        Self::new(json!({
            "filename": filename,
            "cyclic_syndrome": cyclic_syndrome,
        }))
    }
}

#[cfg(feature = "python_binding")]
bind_trait_example_code! {ErrorPatternReader}

impl ErrorPatternReader {
    #[allow(clippy::unnecessary_cast)]
    pub fn new(mut config: serde_json::Value) -> Self {
        let mut filename = "tmp/syndrome_patterns.txt".to_string();
        let config = config.as_object_mut().expect("config must be JSON object");
        if let Some(value) = config.remove("filename") {
            filename = value.as_str().expect("filename string").to_string();
        }
        let cyclic_syndrome = if let Some(cyclic_syndrome) = config.remove("cyclic_syndrome") {
            cyclic_syndrome.as_bool().expect("cyclic_syndrome: bool")
        } else {
            false
        }; // by default not enable cyclic syndrome, to avoid problem
        if !config.is_empty() {
            panic!("unknown config keys: {:?}", config.keys().collect::<Vec<&String>>());
        }
        let file = File::open(filename).unwrap();
        let mut syndrome_patterns = vec![];
        let mut initializer: Option<SolverInitializer> = None;
        let mut positions: Option<Vec<VisualizePosition>> = None;
        for (line_index, line) in io::BufReader::new(file).lines().enumerate() {
            if let Ok(value) = line {
                match line_index {
                    0 => {
                        assert!(value.starts_with("Syndrome Pattern v1.0 "), "incompatible file version");
                    }
                    1 => {
                        initializer = Some(serde_json::from_str(&value).unwrap());
                    }
                    2 => {
                        positions = Some(serde_json::from_str(&value).unwrap());
                    }
                    _ => {
                        let syndrome_pattern: SyndromePattern = serde_json::from_str(&value).unwrap();
                        syndrome_patterns.push(syndrome_pattern);
                    }
                }
            }
        }
        let initializer = initializer.expect("initializer not present in file");
        let positions = positions.expect("positions not present in file");
        assert_eq!(positions.len(), initializer.vertex_num as usize);
        let mut code = Self {
            vertices: Vec::with_capacity(initializer.vertex_num as usize),
            edges: Vec::with_capacity(initializer.weighted_edges.len()),
            syndrome_patterns,
            defect_index: 0,
            cyclic_syndrome,
        };
        for (left_vertex, right_vertex, weight) in initializer.weighted_edges.iter() {
            assert!(weight % 2 == 0, "weight must be even number");
            code.edges.push(CodeEdge {
                vertices: (*left_vertex, *right_vertex),
                p: 0.,  // doesn't matter
                pe: 0., // doesn't matter
                half_weight: weight / 2,
                is_erasure: false, // doesn't matter
            });
        }
        // automatically create the vertices and nearest-neighbor connection
        code.fill_vertices(initializer.vertex_num);
        // set virtual vertices and positions
        for (vertex_index, position) in positions.into_iter().enumerate() {
            code.vertices[vertex_index].position = position;
        }
        for vertex_index in initializer.virtual_vertices {
            code.vertices[vertex_index as usize].is_virtual = true;
        }
        code
    }
}

/// generate error patterns in parallel by hold multiple instances of the same code type
pub struct ExampleCodeParallel<CodeType: ExampleCode + Sync + Send + Clone> {
    /// used to provide graph
    pub example: CodeType,
    /// list of codes
    pub codes: Vec<ArcRwLock<CodeType>>,
    /// syndrome patterns generated by individual code
    pub syndrome_patterns: Vec<SyndromePattern>,
    /// currently using code
    pub code_index: usize,
}

impl<CodeType: ExampleCode + Sync + Send + Clone> ExampleCodeParallel<CodeType> {
    pub fn new(example: CodeType, code_count: usize) -> Self {
        let mut codes = vec![];
        for _ in 0..code_count {
            codes.push(ArcRwLock::<CodeType>::new_value(example.clone()));
        }
        Self {
            example,
            codes,
            syndrome_patterns: vec![],
            code_index: 0,
        }
    }
}

impl<CodeType: ExampleCode + Sync + Send + Clone> ExampleCode for ExampleCodeParallel<CodeType> {
    fn vertices_edges(&mut self) -> (&mut Vec<CodeVertex>, &mut Vec<CodeEdge>) {
        self.example.vertices_edges()
    }
    fn immutable_vertices_edges(&self) -> (&Vec<CodeVertex>, &Vec<CodeEdge>) {
        self.example.immutable_vertices_edges()
    }
    fn generate_random_errors(&mut self, seed: u64) -> SyndromePattern {
        if self.code_index == 0 {
            // run generator in parallel
            (0..self.codes.len())
                .into_par_iter()
                .map(|code_index| {
                    self.codes[code_index]
                        .write()
                        .generate_random_errors(seed + (code_index * 1_000_000_000) as u64)
                })
                .collect_into_vec(&mut self.syndrome_patterns);
        }
        let syndrome_pattern = self.syndrome_patterns[self.code_index].clone();
        self.code_index = (self.code_index + 1) % self.codes.len();
        syndrome_pattern
    }
}

#[cfg(feature = "python_binding")]
#[pyfunction]
pub(crate) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<CodeVertex>()?;
    m.add_class::<CodeEdge>()?;
    m.add_function(wrap_pyfunction!(weight_of_p, m)?)?;
    m.add_class::<CodeCapacityRepetitionCode>()?;
    m.add_class::<CodeCapacityPlanarCode>()?;
    m.add_class::<PhenomenologicalPlanarCode>()?;
    m.add_class::<CircuitLevelPlanarCode>()?;
    m.add_class::<CodeCapacityRotatedCode>()?;
    m.add_class::<PhenomenologicalRotatedCode>()?;
    m.add_class::<ErrorPatternReader>()?;
    Ok(())
}

pub fn visualize_code(code: &mut impl ExampleCode, visualize_filename: String) {
    print_visualize_link(visualize_filename.clone());
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    visualizer.snapshot("code".to_string(), code).unwrap();
    for round in 0..3 {
        code.generate_random_errors(round);
        visualizer.snapshot(format!("syndrome {}", round + 1), code).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_code_capacity_repetition_code() {
        // cargo test example_code_capacity_repetition_code -- --nocapture
        let mut code = CodeCapacityRepetitionCode::new(7, 0.2, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_code_capacity_repetition_code.json".to_string());
    }

    #[test]
    fn example_code_capacity_planar_code() {
        // cargo test example_code_capacity_planar_code -- --nocapture
        let mut code = CodeCapacityPlanarCode::new(7, 0.1, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_code_capacity_planar_code.json".to_string());
    }

    #[test]
    fn example_phenomenological_planar_code() {
        // cargo test example_phenomenological_planar_code -- --nocapture
        let mut code = PhenomenologicalPlanarCode::new(7, 7, 0.01, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_phenomenological_planar_code.json".to_string());
    }

    #[test]
    fn example_large_phenomenological_planar_code() {
        // cargo test example_large_phenomenological_planar_code -- --nocapture
        let mut code = PhenomenologicalPlanarCode::new(7, 30, 0.01, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_large_phenomenological_planar_code.json".to_string());
    }

    #[test]
    fn example_circuit_level_planar_code() {
        // cargo test example_circuit_level_planar_code -- --nocapture
        let mut code = CircuitLevelPlanarCode::new(7, 7, 0.01, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_circuit_level_planar_code.json".to_string());
    }

    #[test]
    fn example_code_capacity_rotated_code() {
        // cargo test example_code_capacity_rotated_code -- --nocapture
        let mut code = CodeCapacityRotatedCode::new(5, 0.1, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_code_capacity_rotated_code.json".to_string());
    }

    #[test]
    fn example_code_phenomenological_rotated_code() {
        // cargo test example_code_phenomenological_rotated_code -- --nocapture
        let mut code = PhenomenologicalRotatedCode::new(5, 5, 0.01, 500);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_code_phenomenological_rotated_code.json".to_string());
    }

    #[cfg(feature = "qecp_integrate")]
    #[test]
    fn example_qec_playground_code() {
        // cargo test example_qec_playground_code -- --nocapture
        let config = json!({
            "qubit_type": qecp::types::QubitType::StabZ,
            "max_half_weight": 50,
        });
        let mut code = QECPlaygroundCode::new(5, 0.001, config);
        code.sanity_check().unwrap();
        visualize_code(&mut code, "example_qec_playground_code.json".to_string());
    }
}
