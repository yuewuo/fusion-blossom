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
    pub fn snapshot<FusionType: FusionVisualizer>(&mut self, name: String, fusion_algorithm: &FusionType) -> std::io::Result<()> {
        self.snapshots.push((name, fusion_algorithm.snapshot(true)));
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
