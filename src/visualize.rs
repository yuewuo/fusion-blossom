//! Visualizer
//! 
//! This module helps visualize the progress of a fusion blossom algorithm
//! 

use crate::serde_json;
use std::fs::File;
use crate::serde::{Serialize};
use std::io::{Write, Seek, SeekFrom};
use crate::chrono::Local;
use crate::urlencoding;

pub trait FusionVisualizer {
    /// take a snapshot, set `abbrev` to true to save space
    fn snapshot(&self, abbrev: bool) -> serde_json::Value;
}

#[derive(Debug, Serialize, Clone)]
pub struct VisualizePosition {
    /// vertical axis, -i is up, +i is down (left-up corner is smallest i,j)
    pub i: f64,
    /// horizontal axis, -j is left, +j is right (left-up corner is smallest i,j)
    pub j: f64,
    /// time axis, top and bottom (orthogonal to the initial view, which looks at -t direction)
    pub t: f64,
}

impl VisualizePosition {
    /// create a visualization position
    pub fn new(i: f64, j: f64, t: f64) -> Self {
        Self {
            i: i, j: j, t: t
        }
    }
}

#[derive(Debug)]
pub struct Visualizer {
    /// save to file if applicable
    file: Option<File>,
    /// basic snapshot
    base: serde_json::Value,
    /// positions of the nodes
    positions: Vec<VisualizePosition>,
    /// all snapshots
    snapshots: Vec<(String, serde_json::Value)>,
}

pub fn snapshot_fix_missing_fields(value: &mut serde_json::Value, abbrev: bool) {
    let value = value.as_object_mut().expect("snapshot must be an object");
    // fix vertices missing fields
    let vertices = value.get_mut("nodes").expect("missing unrecoverable field").as_array_mut().expect("vertices must be an array");
    for vertex in vertices {
        let vertex = vertex.as_object_mut().expect("each vertex must be an object");
        let key_is_virtual = if abbrev { "v" } else { "is_virtual" };
        let key_is_syndrome = if abbrev { "s" } else { "is_syndrome" };
        // recover
        assert!(vertex.contains_key(key_is_virtual), "missing unrecoverable field");
        if !vertex.contains_key(key_is_syndrome) {
            vertex[key_is_syndrome] = json!(0);  // by default no syndrome
        }
    }
    // fix edges missing fields
    let edges = value.get_mut("edges").expect("missing unrecoverable field").as_array_mut().expect("edges must be an array");
    for edge in edges {
        let edge = edge.as_object_mut().expect("each edge must be an object");
        let key_weight = if abbrev { "w" } else { "weight" };
        let key_left = if abbrev { "l" } else { "left" };
        let key_right = if abbrev { "r" } else { "right" };
        let key_left_growth = if abbrev { "lg" } else { "left_growth" };
        let key_right_growth = if abbrev { "rg" } else { "right_growth" };
        // recover
        assert!(edge.contains_key(key_weight), "missing unrecoverable field");
        assert!(edge.contains_key(key_left), "missing unrecoverable field");
        assert!(edge.contains_key(key_right), "missing unrecoverable field");
        if !edge.contains_key(key_left_growth) {
            edge.insert(key_left_growth.to_string(), json!(0));  // by default no growth
        }
        if !edge.contains_key(key_right_growth) {
            edge.insert(key_right_growth.to_string(), json!(0));  // by default no growth
        }
    }
    // fix tree node missing fields
    if !value.contains_key("tree_nodes") {
        value.insert("tree_nodes".to_string(), json!([]));  // by default no tree nodes
    }
    let tree_nodes = value.get_mut("tree_nodes").unwrap().as_array_mut().expect("tree_nodes must be an array");
    for _tree_node in tree_nodes {
        unimplemented!();
    }
}

pub type ObjectMap = serde_json::Map<String, serde_json::Value>;
pub fn snapshot_combine_object_known_key(obj: &mut ObjectMap, obj_2: &mut ObjectMap, key: &str) {
    match (obj.contains_key(key), obj_2.contains_key(key)) {
        (_, false) => { },  // do nothing
        (false, true) => { obj.insert(key.to_string(), obj_2.remove(key).unwrap()); }
        (true, true) => { assert_eq!(obj[key], obj_2[key], "cannot combine different values: please make sure values don't conflict"); }
    }
}

pub fn snapshot_copy_remaining_fields(obj: &mut ObjectMap, obj_2: &mut ObjectMap) {
    let mut keys = Vec::<String>::new();
    for key in obj_2.keys() {
        keys.push(key.clone());
    }
    for key in keys.iter() {
        match obj.contains_key(key) {
            false => { obj.insert(key.to_string(), obj_2.remove(key).unwrap()); }
            true => { assert_eq!(obj[key], obj_2[key], "cannot combine unknown fields: don't know what to do, please modify `snapshot_combine_values` function"); }
        }
    }
}

pub fn snapshot_combine_values(value: &mut serde_json::Value, mut value_2: serde_json::Value, abbrev: bool) {
    let value = value.as_object_mut().expect("snapshot must be an object");
    let value_2 = value_2.as_object_mut().expect("snapshot must be an object");
    match (value.contains_key("nodes"), value_2.contains_key("nodes")) {
        (_, false) => { },  // do nothing
        (false, true) => { value.insert("nodes".to_string(), value_2.remove("nodes").unwrap()); }
        (true, true) => {  // combine
            let vertices = value.get_mut("nodes").unwrap().as_array_mut().expect("vertices must be an array");
            let vertices_2 = value_2.get_mut("nodes").unwrap().as_array_mut().expect("vertices must be an array");
            assert!(vertices.len() == vertices_2.len(), "vertices must be compatible");
            for (vertex_idx, vertex) in vertices.iter_mut().enumerate() {
                let vertex_2 = &mut vertices_2[vertex_idx];
                let vertex = vertex.as_object_mut().expect("each vertex must be an object");
                let vertex_2 = vertex_2.as_object_mut().expect("each vertex must be an object");
                // list known keys
                let key_is_virtual = if abbrev { "v" } else { "is_virtual" };
                let key_is_syndrome = if abbrev { "s" } else { "is_syndrome" };
                let known_keys = [key_is_virtual, key_is_syndrome];
                for key in known_keys {
                    snapshot_combine_object_known_key(vertex, vertex_2, key);
                }
                snapshot_copy_remaining_fields(vertex, vertex_2);
            }
        }
    }
    match (value.contains_key("edges"), value_2.contains_key("edges")) {
        (_, false) => { },  // do nothing
        (false, true) => { value.insert("edges".to_string(), value_2.remove("edges").unwrap()); }
        (true, true) => {  // combine
            let edges = value.get_mut("edges").unwrap().as_array_mut().expect("edges must be an array");
            let edges_2 = value_2.get_mut("edges").unwrap().as_array_mut().expect("edges must be an array");
            assert!(edges.len() == edges_2.len(), "edges must be compatible");
            for (edge_idx, edge) in edges.iter_mut().enumerate() {
                let edge_2 = &mut edges_2[edge_idx];
                let edge = edge.as_object_mut().expect("each edge must be an object");
                let edge_2 = edge_2.as_object_mut().expect("each edge must be an object");
                // list known keys
                let key_weight = if abbrev { "w" } else { "weight" };
                let key_left = if abbrev { "l" } else { "left" };
                let key_right = if abbrev { "r" } else { "right" };
                let key_left_growth = if abbrev { "lg" } else { "left_growth" };
                let key_right_growth = if abbrev { "rg" } else { "right_growth" };
                let known_keys = [key_weight, key_left, key_right, key_left_growth, key_right_growth];
                for key in known_keys {
                    snapshot_combine_object_known_key(edge, edge_2, key);
                }
                snapshot_copy_remaining_fields(edge, edge_2);
            }
        }
    }
    match (value.contains_key("tree_nodes"), value_2.contains_key("tree_nodes")) {
        (_, false) => { },  // do nothing
        (false, true) => { value.insert("tree_nodes".to_string(), value_2.remove("tree_nodes").unwrap()); }
        (true, true) => {  // combine
            unimplemented!();
        }
    }
}

impl Visualizer {
    /// create a new visualizer with target filename and node layout
    pub fn new(filename: Option<String>) -> std::io::Result<Self> {
        let file = match filename {
            Some(filename) => Some(File::create(filename)?),
            None => None,
        };
        Ok(Self {
            file: file,
            base: json!({}),
            positions: Vec::new(),
            snapshots: Vec::new(),
        })
    }

    /// append another snapshot of the fusion type, and also update the file in case 
    pub fn snapshot_combined(&mut self, name: String, fusion_algorithms: Vec<&dyn FusionVisualizer>) -> std::io::Result<()> {
        let abbrev = true;
        let mut value = json!({});
        for fusion_algorithm in fusion_algorithms.iter() {
            let value_2 = fusion_algorithm.snapshot(abbrev);
            snapshot_combine_values(&mut value, value_2, abbrev);
        }
        snapshot_fix_missing_fields(&mut value, abbrev);
        self.snapshots.push((name, value));
        self.save()?;
        Ok(())
    }

    /// append another snapshot of the fusion type, and also update the file in case 
    pub fn snapshot(&mut self, name: String, fusion_algorithm: &impl FusionVisualizer) -> std::io::Result<()> {
        let abbrev = true;
        let mut value = fusion_algorithm.snapshot(abbrev);
        snapshot_fix_missing_fields(&mut value, abbrev);
        self.snapshots.push((name, value));
        self.save()?;
        Ok(())
    }

    /// save to file
    pub fn save(&mut self) -> std::io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            file.set_len(0)?;  // truncate the file
            file.seek(SeekFrom::Start(0))?;  // move the cursor to the front
            file.write_all(json!({
                "base": &self.base,
                "snapshots": &self.snapshots,
                "positions": &self.positions,
            }).to_string().as_bytes())?;
            file.sync_all()?;
        }
        Ok(())
    }

    /// set positions of the node and optionally center all positions
    pub fn set_positions(&mut self, mut positions: Vec<VisualizePosition>, center: bool) {
        if center {
            let (mut ci, mut cj, mut ct) = (0., 0., 0.);
            for position in positions.iter() {
                ci += position.i;
                cj += position.j;
                ct += position.t;
            }
            ci /= positions.len() as f64;
            cj /= positions.len() as f64;
            ct /= positions.len() as f64;
            for position in positions.iter_mut() {
                position.i -= ci;
                position.j -= cj;
                position.t -= ct;
            }
        }
        self.positions = positions;
    }

}

const DEFAULT_VISUALIZE_DATA_FOLDER: &'static str = concat!(env!("CARGO_MANIFEST_DIR"), "/visualize/data/");

pub fn visualize_data_folder() -> String {
    DEFAULT_VISUALIZE_DATA_FOLDER.to_string()
}

pub fn static_visualize_data_filename() -> String {
    format!("static.json")
}

pub fn auto_visualize_data_filename() -> String {
    format!("{}.json", Local::now().format("%Y%m%d-%H-%M-%S%.3f"))
}

pub fn print_visualize_link_with_parameters(filename: &String, parameters: Vec<(String, String)>) {
    let mut link = format!("http://localhost:8066?filename={}", filename);
    for (key, value) in parameters.iter() {
        link.push_str("&");
        link.push_str(&urlencoding::encode(key));
        link.push_str("=");
        link.push_str(&urlencoding::encode(value));
    }
    println!("opening link {} (start local server by running ./visualize/server.sh) or call `node index.js <link>` to render locally", link)
}

pub fn print_visualize_link(filename: &String) {
    print_visualize_link_with_parameters(filename, Vec::new())
}



#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use super::super::fusion_single_thread::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn visualize_test_1() {  // cargo test visualize_test_1 -- --nocapture
        let d = 11usize;
        let p = 0.2f64;
        let row_node_num = (d-1) + 2;  // two virtual nodes at left and right
        let node_num = row_node_num * d;  // `d` rows
        let mut pos = HashMap::<(isize, isize), usize>::new();
        for row in 0..d {
            let bias = row * row_node_num;
            for i in 0..d-1 {
                pos.insert((row as isize, i as isize), bias + i);
            }
            pos.insert((row as isize, -1), bias + d);
        }
        let half_weight: Weight = (10000. * ((1. - p).ln() - p.ln())).max(1.) as Weight;
        let weight = half_weight * 2;  // to make sure weight is even number for ease of this test function
        println!("half_weight: {}, weight: {}", half_weight, weight);
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for row in 0..d {
                let bias = row * row_node_num;
                for i in 0..d-1 {
                    weighted_edges.push((bias + i, bias + i+1, weight));
                }
                weighted_edges.push((bias + 0, bias + d, weight));  // left most edge
                if row + 1 < d {
                    for i in 0..d-1 {
                        weighted_edges.push((bias + i, bias + i + row_node_num, weight));
                    }
                }
            }
            weighted_edges
        };
        let virtual_nodes = {
            let mut virtual_nodes = Vec::new();
            for row in 0..d {
                let bias = row * row_node_num;
                virtual_nodes.push(bias + d - 1);
                virtual_nodes.push(bias + d);
            }
            virtual_nodes
        };
        // hardcode syndrome
        let syndrome_nodes = vec![39, 63, 52, 100, 90];
        // run single-thread fusion blossom algorithm
        let visualize_filename = static_visualize_data_filename();
        print_visualize_link(&visualize_filename);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        let mut positions = Vec::new();
        for row in 0..d {
            let pos_i = row as f64;
            for i in 0..d {
                positions.push(VisualizePosition::new(pos_i, i as f64, 0.));
            }
            positions.push(VisualizePosition::new(pos_i, -1., 0.));
        }
        visualizer.set_positions(positions, true);  // automatic center all nodes
        let mut fusion_solver = FusionSingleThread::new(node_num, &weighted_edges, &virtual_nodes);
        fusion_solver.load_syndrome(&syndrome_nodes);
        visualizer.snapshot(format!("initial"), &fusion_solver).unwrap();
        let syndrome_tree_nodes: Vec<TreeNodePtr> = syndrome_nodes.iter().map(|&node_index| {
            Arc::clone(fusion_solver.nodes[node_index].read_recursive().tree_node.as_ref().unwrap())
        }).collect();
        // test basic grow and shrink of a single tree node
        for _ in 0..4 {
            fusion_solver.grow_tree_node(&syndrome_tree_nodes[0], half_weight);
            visualizer.snapshot(format!("grow half weight"), &fusion_solver).unwrap();
        }
        for _ in 0..4 {
            fusion_solver.grow_tree_node(&syndrome_tree_nodes[0], -half_weight);
            visualizer.snapshot(format!("shrink half weight"), &fusion_solver).unwrap();
        }
        for _ in 0..3 {
            fusion_solver.grow_tree_node(&syndrome_tree_nodes[0], half_weight);
        }
        visualizer.snapshot(format!("grow 3 half weight"), &fusion_solver).unwrap();
        for _ in 0..3 {
            fusion_solver.grow_tree_node(&syndrome_tree_nodes[0], -half_weight);
        }
        visualizer.snapshot(format!("shrink 3 half weight"), &fusion_solver).unwrap();
        // // test all
        // for i in 0..syndrome_tree_nodes.len() {
        //     fusion_solver.grow_tree_node(&syndrome_tree_nodes[i], half_weight);
        //     visualizer.snapshot(format!("grow half weight"), &fusion_solver).unwrap();
        // }
        // visualizer.snapshot(format!("end"), &fusion_solver).unwrap();
    }


    #[test]
    fn visualize_paper_weighted_union_find_decoder() {  // cargo test visualize_paper_weighted_union_find_decoder -- --nocapture
        let d = 3usize;
        let td = 4usize;
        let p = 0.2f64;
        let row_node_num = (d-1) + 2;  // two virtual nodes at left and right
        let t_node_num = row_node_num * d;  // `d` rows
        let half_node_num = t_node_num * td;  // `td` layers
        let node_num = half_node_num * 2;  // both X and Z type stabilizers altogether
        let half_weight: Weight = (10000. * ((1. - p).ln() - p.ln())).max(1.) as Weight;
        let weight = half_weight * 2;  // to make sure weight is even number for ease of this test function
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for is_z in [true, false] {
                for t in 0..td {
                    let t_bias = t * t_node_num + if is_z { 0 } else { half_node_num };
                    for row in 0..d {
                        let bias = t_bias + row * row_node_num;
                        for i in 0..d-1 {
                            weighted_edges.push((bias + i, bias + i+1, weight));
                        }
                        weighted_edges.push((bias + 0, bias + d, weight));  // left most edge
                        if row + 1 < d {
                            for i in 0..d-1 {
                                weighted_edges.push((bias + i, bias + i + row_node_num, weight));
                            }
                        }
                    }
                    // inter-layer connection
                    if t + 1 < td {
                        for row in 0..d {
                            let bias = t_bias + row * row_node_num;
                            for i in 0..d-1 {
                                weighted_edges.push((bias + i, bias + i + t_node_num, weight));
                                // diagonal edges
                                let diagonal_diffs: Vec<(isize, isize)> = if is_z {
                                    vec![(0, 1), (1, 0), (1, 1)]
                                } else {
                                    // i and j are reversed if x stabilizer, not vec![(0, -2), (2, 0), (2, -2)]
                                    vec![(-1, 0), (0, 1), (-1, 1)]
                                };
                                for (di, dj) in diagonal_diffs {
                                    let new_row = row as isize + di;  // row corresponds to `i`
                                    let new_i = i as isize + dj;  // i corresponds to `j`
                                    if new_row >= 0 && new_i >= 0 && new_row < d as isize && new_i < (d-1) as isize {
                                        let new_bias = t_bias + (new_row as usize) * row_node_num + t_node_num;
                                        weighted_edges.push((bias + i, new_bias + new_i as usize, weight));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            weighted_edges
        };
        let virtual_nodes = {
            let mut virtual_nodes = Vec::new();
            for is_z in [true, false] {
                for t in 0..td {
                    let t_bias = t * t_node_num + if is_z { 0 } else { half_node_num };
                    for row in 0..d {
                        let bias = t_bias + row * row_node_num;
                        virtual_nodes.push(bias + d - 1);
                        virtual_nodes.push(bias + d);
                    }
                }
            }
            virtual_nodes
        };
        // hardcode syndrome
        let syndrome_nodes = vec![16, 29, 88, 72, 32, 44, 20, 21, 68, 69];
        let grow_edges = vec![48, 156, 169, 81, 38, 135];
        // run single-thread fusion blossom algorithm
        let visualize_filename = static_visualize_data_filename();
        print_visualize_link_with_parameters(&visualize_filename, vec![(format!("patch"), format!("visualize_paper_weighted_union_find_decoder"))]);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        let mut positions = Vec::new();
        let scale = 2f64;
        for is_z in [true, false] {
            for t in 0..td {
                let pos_t = t as f64;
                for row in 0..d {
                    let pos_i = row as f64;
                    for i in 0..d {
                        if is_z {
                            positions.push(VisualizePosition::new(pos_i * scale, (i as f64 + 0.5) * scale, pos_t * scale));
                        } else {
                            positions.push(VisualizePosition::new((i as f64 + 0.5) * scale, pos_i * scale, pos_t * scale));
                        }
                    }
                    if is_z {
                        positions.push(VisualizePosition::new(pos_i * scale, (-1. + 0.5) * scale, pos_t * scale));
                    } else {
                        positions.push(VisualizePosition::new((-1. + 0.5) * scale, pos_i * scale, pos_t * scale));
                    }
                }
            }
        }
        visualizer.set_positions(positions, true);  // automatic center all nodes
        let mut fusion_solver = FusionSingleThread::new(node_num, &weighted_edges, &virtual_nodes);
        fusion_solver.load_syndrome(&syndrome_nodes);
        // grow edges
        for &edge_index in grow_edges.iter() {
            let mut edge = fusion_solver.edges[edge_index].write();
            edge.left_growth = edge.weight;
        }
        // save snapshot
        visualizer.snapshot(format!("initial"), &fusion_solver).unwrap();
    }

    #[test]
    fn visualize_rough_idea_fusion_blossom() {  // cargo test visualize_rough_idea_fusion_blossom -- --nocapture
        let d: usize = 7;
        let td: usize = 8;
        let p: f64 = 0.2;
        let is_circuit_level = false;
        let row_node_num = (d-1) + 2;  // two virtual nodes at left and right
        let t_node_num = row_node_num * d;  // `d` rows
        let half_node_num = t_node_num * td;  // `td` layers
        let node_num = half_node_num;  // only Z type stabilizers
        let quarter_weight: Weight = (10000. * ((1. - p).ln() - p.ln())).max(1.) as Weight;
        let half_weight = quarter_weight * 2;
        let weight = half_weight * 2;  // to make sure weight is even number for ease of this test function
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for t in 0..td {
                let t_bias = t * t_node_num;
                for row in 0..d {
                    let bias = t_bias + row * row_node_num;
                    for i in 0..d-1 {
                        weighted_edges.push((bias + i, bias + i+1, weight));
                    }
                    weighted_edges.push((bias + 0, bias + d, weight));  // left most edge
                    if row + 1 < d {
                        for i in 0..d-1 {
                            weighted_edges.push((bias + i, bias + i + row_node_num, weight));
                        }
                    }
                }
                // inter-layer connection
                if t + 1 < td {
                    for row in 0..d {
                        let bias = t_bias + row * row_node_num;
                        for i in 0..d-1 {
                            weighted_edges.push((bias + i, bias + i + t_node_num, weight));
                            // diagonal edges
                            if is_circuit_level {
                                let diagonal_diffs: Vec<(isize, isize)> = vec![(0, 1), (1, 0), (1, 1)];
                                for (di, dj) in diagonal_diffs {
                                    let new_row = row as isize + di;  // row corresponds to `i`
                                    let new_i = i as isize + dj;  // i corresponds to `j`
                                    if new_row >= 0 && new_i >= 0 && new_row < d as isize && new_i < (d-1) as isize {
                                        let new_bias = t_bias + (new_row as usize) * row_node_num + t_node_num;
                                        weighted_edges.push((bias + i, new_bias + new_i as usize, weight));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            weighted_edges
        };
        let virtual_nodes = {
            let mut virtual_nodes = Vec::new();
            for t in 0..td {
                let t_bias = t * t_node_num;
                for row in 0..d {
                    let bias = t_bias + row * row_node_num;
                    virtual_nodes.push(bias + d - 1);
                    virtual_nodes.push(bias + d);
                }
            }
            virtual_nodes
        };
        // hardcode syndrome
        let syndrome_nodes = vec![25, 33, 20, 76, 203, 187, 243, 315];
        // run single-thread fusion blossom algorithm
        let visualize_filename = static_visualize_data_filename();
        print_visualize_link_with_parameters(&visualize_filename, vec![(format!("patch"), format!("visualize_rough_idea_fusion_blossom"))]);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        let mut positions = Vec::new();
        let scale = 1f64;
        for t in 0..td {
            let pos_t = t as f64;
            for row in 0..d {
                let pos_i = row as f64;
                for i in 0..d {
                    positions.push(VisualizePosition::new(pos_i * scale, (i as f64 + 0.5) * scale, pos_t * scale));
                }
                positions.push(VisualizePosition::new(pos_i * scale, (-1. + 0.5) * scale, pos_t * scale));
            }
        }
        visualizer.set_positions(positions, true);  // automatic center all nodes
        let mut fusion_solver = FusionSingleThread::new(node_num, &weighted_edges, &virtual_nodes);
        fusion_solver.load_syndrome(&syndrome_nodes);
        let syndrome_tree_nodes = |fusion_solver: &FusionSingleThread, node_index: usize| {
            Arc::clone(fusion_solver.nodes[node_index].read_recursive().tree_node.as_ref().unwrap())
        };
        // save snapshot
        visualizer.snapshot(format!("initial"), &fusion_solver).unwrap();
        // first layer grow first
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 25), quarter_weight);
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 33), quarter_weight);
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 20), quarter_weight);
        visualizer.snapshot(format!("grow a quarter"), &fusion_solver).unwrap();
        // merge and match
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 25), quarter_weight);
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 33), quarter_weight);
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 20), quarter_weight);
        visualizer.snapshot(format!("find a match"), &fusion_solver).unwrap();
        // grow to boundary
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 20), half_weight);
        visualizer.snapshot(format!("touch temporal boundary"), &fusion_solver).unwrap();
        // add more measurement rounds
        visualizer.snapshot(format!("add measurement #2"), &fusion_solver).unwrap();
        visualizer.snapshot(format!("add measurement #3"), &fusion_solver).unwrap();
        visualizer.snapshot(format!("add measurement #4"), &fusion_solver).unwrap();
        // handle errors at measurement round 4
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 203), half_weight);
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 187), half_weight);
        visualizer.snapshot(format!("grow a half"), &fusion_solver).unwrap();
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 203), half_weight);
        fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 187), half_weight);
        visualizer.snapshot(format!("temporary match"), &fusion_solver).unwrap();
        // handle errors at measurement round 5
        visualizer.snapshot(format!("add measurement #5"), &fusion_solver).unwrap();
        for _ in 0..4 {
            fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 187), -quarter_weight);
            fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 203), quarter_weight);
            fusion_solver.grow_tree_node(&syndrome_tree_nodes(&fusion_solver, 243), quarter_weight);
            visualizer.snapshot(format!("grow or shrink a quarter"), &fusion_solver).unwrap();
        }
        visualizer.snapshot(format!("add measurement #6"), &fusion_solver).unwrap();
        visualizer.snapshot(format!("add measurement #7"), &fusion_solver).unwrap();
        visualizer.snapshot(format!("add measurement #8"), &fusion_solver).unwrap();
    }

}
