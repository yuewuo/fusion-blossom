//! Visualizer
//! 
//! This module helps visualize the progress of a fusion blossom algorithm
//! 

use crate::serde_json;
use std::fs::File;
use crate::serde::{Serialize, Deserialize};
use std::io::{Write, Seek, SeekFrom};
use crate::chrono::Local;
use crate::urlencoding;

pub trait FusionVisualizer {
    /// take a snapshot, set `abbrev` to true to save space
    fn snapshot(&self, abbrev: bool) -> serde_json::Value;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            i, j, t
        }
    }
}

#[derive(Debug)]
pub struct Visualizer {
    /// save to file if applicable
    file: Option<File>,
    /// basic snapshot
    base: serde_json::Value,
    /// positions of the vertices
    positions: Vec<VisualizePosition>,
    /// all snapshots
    snapshots: Vec<(String, serde_json::Value)>,
}

pub fn snapshot_fix_missing_fields(value: &mut serde_json::Value, abbrev: bool) {
    let value = value.as_object_mut().expect("snapshot must be an object");
    // fix vertices missing fields
    let vertices = value.get_mut("vertices").expect("missing unrecoverable field").as_array_mut().expect("vertices must be an array");
    for vertex in vertices {
        if vertex.is_null() { continue }  // vertex not present, probably currently don't care
        let vertex = vertex.as_object_mut().expect("each vertex must be an object");
        let key_is_virtual = if abbrev { "v" } else { "is_virtual" };
        let key_is_syndrome = if abbrev { "s" } else { "is_syndrome" };
        // recover
        assert!(vertex.contains_key(key_is_virtual), "missing unrecoverable field");
        if !vertex.contains_key(key_is_syndrome) {
            vertex.insert(key_is_syndrome.to_string(), json!(0));  // by default no syndrome
        }
    }
    // fix edges missing fields
    let edges = value.get_mut("edges").expect("missing unrecoverable field").as_array_mut().expect("edges must be an array");
    for edge in edges {
        if edge.is_null() { continue }  // edge not present, probably currently don't care
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
}

pub type ObjectMap = serde_json::Map<String, serde_json::Value>;
pub fn snapshot_combine_object_known_key(obj: &mut ObjectMap, obj_2: &mut ObjectMap, key: &str) {
    match (obj.contains_key(key), obj_2.contains_key(key)) {
        (_, false) => { },  // do nothing
        (false, true) => { obj.insert(key.to_string(), obj_2.remove(key).unwrap()); }
        (true, true) => {
            // println!("[snapshot_combine_object_known_key] {}: {:?} == {:?}", key, obj[key], obj_2[key]);
            assert_eq!(obj[key], obj_2[key], "cannot combine different values: please make sure values don't conflict");
            obj_2.remove(key).unwrap();
        }
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
            true => {
                // println!("[snapshot_copy_remaining_fields] {}: {:?} == {:?}", key, obj[key], obj_2[key]);
                // println!("obj: {obj:?}");
                // println!("obj_2: {obj_2:?}");
                assert_eq!(obj[key], obj_2[key], "cannot combine unknown fields: don't know what to do, please modify `snapshot_combine_values` function");
                obj_2.remove(key).unwrap();
            }
        }
    }
}

pub fn snapshot_combine_values(value: &mut serde_json::Value, mut value_2: serde_json::Value, abbrev: bool) {
    let value = value.as_object_mut().expect("snapshot must be an object");
    let value_2 = value_2.as_object_mut().expect("snapshot must be an object");
    match (value.contains_key("vertices"), value_2.contains_key("vertices")) {
        (_, false) => { },  // do nothing
        (false, true) => { value.insert("vertices".to_string(), value_2.remove("vertices").unwrap()); }
        (true, true) => {  // combine
            let vertices = value.get_mut("vertices").unwrap().as_array_mut().expect("vertices must be an array");
            let vertices_2 = value_2.get_mut("vertices").unwrap().as_array_mut().expect("vertices must be an array");
            assert!(vertices.len() == vertices_2.len(), "vertices must be compatible");
            for (vertex_idx, vertex) in vertices.iter_mut().enumerate() {
                let vertex_2 = &mut vertices_2[vertex_idx];
                if vertex_2.is_null() { continue }
                if vertex.is_null() { *vertex = vertex_2.clone(); continue }
                // println!("vertex_idx: {vertex_idx}");
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
                assert_eq!(vertex_2.len(), 0, "there should be nothing left");
            }
            value_2.remove("vertices").unwrap();
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
                if edge_2.is_null() { continue }
                if edge.is_null() { *edge = edge_2.clone(); continue }
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
                assert_eq!(edge_2.len(), 0, "there should be nothing left");
            }
            value_2.remove("edges").unwrap();
        }
    }
    match (value.contains_key("dual_nodes"), value_2.contains_key("dual_nodes")) {
        (_, false) => { },  // do nothing
        (false, true) => { value.insert("dual_nodes".to_string(), value_2.remove("dual_nodes").unwrap()); }
        (true, true) => {  // combine
            let dual_nodes = value.get_mut("dual_nodes").unwrap().as_array_mut().expect("dual_nodes must be an array");
            let dual_nodes_2 = value_2.get_mut("dual_nodes").unwrap().as_array_mut().expect("dual_nodes must be an array");
            assert!(dual_nodes.len() == dual_nodes_2.len(), "dual_nodes must be compatible");
            for (dual_node_idx, dual_node) in dual_nodes.iter_mut().enumerate() {
                let dual_node_2 = &mut dual_nodes_2[dual_node_idx];
                if dual_node.is_null() {
                    assert!(dual_node_2.is_null(), "dual node must be simultaneously be null, if necessary");
                    continue
                }
                let dual_node = dual_node.as_object_mut().expect("each dual_node must be an object");
                let dual_node_2 = dual_node_2.as_object_mut().expect("each dual_node must be an object");
                // list known keys
                let key_boundary = if abbrev { "b" } else { "boundary" };
                let key_dual_variable = if abbrev { "d" } else { "dual_variable" };
                let key_blossom = if abbrev { "o" } else { "blossom" };
                let key_syndrome_vertex = if abbrev { "s" } else { "syndrome_vertex" };
                let key_grow_state = if abbrev { "g" } else { "grow_state" };
                let key_unit_growth = if abbrev { "u" } else { "unit_growth" };
                let key_parent_blossom = if abbrev { "p" } else { "parent_blossom" };
                let known_keys = [key_boundary, key_dual_variable, key_blossom, key_syndrome_vertex, key_grow_state, key_unit_growth, key_parent_blossom];
                for key in known_keys {
                    snapshot_combine_object_known_key(dual_node, dual_node_2, key);
                }
                snapshot_copy_remaining_fields(dual_node, dual_node_2);
                assert_eq!(dual_node_2.len(), 0, "there should be nothing left");
            }
            value_2.remove("dual_nodes").unwrap();
        }
    }
    snapshot_copy_remaining_fields(value, value_2);
}

impl Visualizer {
    /// create a new visualizer with target filename and node layout
    pub fn new(mut filename: Option<String>) -> std::io::Result<Self> {
        if cfg!(feature = "disable_visualizer") {
            filename = None;  // do not open file
        }
        let file = match filename {
            Some(filename) => Some(File::create(filename)?),
            None => None,
        };
        Ok(Self {
            file,
            base: json!({}),
            positions: Vec::new(),
            snapshots: Vec::new(),
        })
    }

    /// append another snapshot of the fusion type, and also update the file in case 
    pub fn snapshot_combined(&mut self, name: String, fusion_algorithms: Vec<&dyn FusionVisualizer>) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(())
        }
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
        if cfg!(feature = "disable_visualizer") {
            return Ok(())
        }
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

const DEFAULT_VISUALIZE_DATA_FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/visualize/data/");

pub fn visualize_data_folder() -> String {
    DEFAULT_VISUALIZE_DATA_FOLDER.to_string()
}

pub fn static_visualize_data_filename() -> String {
    "static.json".to_string()
}

pub fn auto_visualize_data_filename() -> String {
    format!("{}.json", Local::now().format("%Y%m%d-%H-%M-%S%.3f"))
}

pub fn print_visualize_link_with_parameters(filename: &String, parameters: Vec<(String, String)>) {
    let mut link = format!("http://localhost:8066?filename={}", filename);
    for (key, value) in parameters.iter() {
        link.push('&');
        link.push_str(&urlencoding::encode(key));
        link.push('=');
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
    use super::super::example::*;
    use super::super::dual_module_serial::*;
    use super::super::dual_module::*;

    #[test]
    fn visualize_test_1() {  // cargo test visualize_test_1 -- --nocapture
        let visualize_filename = format!("visualize_test_1.json");
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(11, 0.2, half_weight);
        let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
        visualizer.set_positions(code.get_positions(), true);  // automatic center all vertices
        print_visualize_link(&visualize_filename);
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleSerial::new(&initializer);
        let syndrome_vertices = vec![39, 63, 52, 100, 90];
        for syndrome_vertex in syndrome_vertices.iter() {
            code.vertices[*syndrome_vertex].is_syndrome = true;
        }
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer.snapshot_combined(format!("initial"), vec![&interface_ptr, &dual_module]).unwrap();
        // create dual nodes and grow them by half length
        // test basic grow and shrink of a single tree node
        for _ in 0..4 {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), half_weight);
            visualizer.snapshot_combined(format!("grow half weight"), vec![&interface_ptr, &dual_module]).unwrap();
        }
        for _ in 0..4 {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), -half_weight);
            visualizer.snapshot_combined(format!("shrink half weight"), vec![&interface_ptr, &dual_module]).unwrap();
        }
        for _ in 0..3 { dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), half_weight); }
        visualizer.snapshot_combined(format!("grow 3 half weight"), vec![&interface_ptr, &dual_module]).unwrap();
        for _ in 0..3 { dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), -half_weight); }
        visualizer.snapshot_combined(format!("shrink 3 half weight"), vec![&interface_ptr, &dual_module]).unwrap();
        // test all
        for i in 0..interface_ptr.read_recursive().nodes_length {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[i].clone().unwrap(), half_weight);
            visualizer.snapshot_combined(format!("grow half weight"), vec![&interface_ptr, &dual_module]).unwrap();
        }
    }


    #[test]
    fn visualize_paper_weighted_union_find_decoder() {  // cargo test visualize_paper_weighted_union_find_decoder -- --nocapture
        let visualize_filename = format!("visualize_paper_weighted_union_find_decoder.json");
        let d = 3usize;
        let td = 4usize;
        let p = 0.2f64;
        let row_vertex_num = (d-1) + 2;  // two virtual vertices at left and right
        let t_vertex_num = row_vertex_num * d;  // `d` rows
        let half_vertex_num = t_vertex_num * td;  // `td` layers
        let vertex_num = half_vertex_num * 2;  // both X and Z type stabilizers altogether
        let half_weight: Weight = (10000. * ((1. - p).ln() - p.ln())).max(1.) as Weight;
        let weight = half_weight * 2;  // to make sure weight is even number for ease of this test function
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for is_z in [true, false] {
                for t in 0..td {
                    let t_bias = t * t_vertex_num + if is_z { 0 } else { half_vertex_num };
                    for row in 0..d {
                        let bias = t_bias + row * row_vertex_num;
                        for i in 0..d-1 {
                            weighted_edges.push((bias + i, bias + i+1, weight));
                        }
                        weighted_edges.push((bias + 0, bias + d, weight));  // left most edge
                        if row + 1 < d {
                            for i in 0..d-1 {
                                weighted_edges.push((bias + i, bias + i + row_vertex_num, weight));
                            }
                        }
                    }
                    // inter-layer connection
                    if t + 1 < td {
                        for row in 0..d {
                            let bias = t_bias + row * row_vertex_num;
                            for i in 0..d-1 {
                                weighted_edges.push((bias + i, bias + i + t_vertex_num, weight));
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
                                        let new_bias = t_bias + (new_row as usize) * row_vertex_num + t_vertex_num;
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
        let virtual_vertices = {
            let mut virtual_vertices = Vec::new();
            for is_z in [true, false] {
                for t in 0..td {
                    let t_bias = t * t_vertex_num + if is_z { 0 } else { half_vertex_num };
                    for row in 0..d {
                        let bias = t_bias + row * row_vertex_num;
                        virtual_vertices.push(bias + d - 1);
                        virtual_vertices.push(bias + d);
                    }
                }
            }
            virtual_vertices
        };
        // hardcode syndrome
        let syndrome_vertices = vec![16, 29, 88, 72, 32, 44, 20, 21, 68, 69];
        let grow_edges = vec![48, 156, 169, 81, 38, 135];
        // run single-thread fusion blossom algorithm
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
        visualizer.set_positions(positions, true);  // automatic center all vertices
        let initializer = SolverInitializer::new(vertex_num, weighted_edges, virtual_vertices);
        let mut dual_module = DualModuleSerial::new(&initializer);
        let interface_ptr = DualModuleInterfacePtr::new_load(&SyndromePattern::new_vertices(syndrome_vertices), &mut dual_module);
        // grow edges
        for &edge_index in grow_edges.iter() {
            let mut edge = dual_module.edges[edge_index].write_force();
            edge.left_growth = edge.weight;
        }
        // save snapshot
        visualizer.snapshot_combined(format!("initial"), vec![&interface_ptr, &dual_module]).unwrap();
    }

    #[test]
    fn visualize_rough_idea_fusion_blossom() {  // cargo test visualize_rough_idea_fusion_blossom -- --nocapture
        let quarter_weight = 250;
        let half_weight = 2 * quarter_weight;
        for is_circuit_level in [false, true] {
            let visualize_filename = if is_circuit_level {
                format!("visualize_rough_idea_fusion_blossom_circuit_level.json")
            } else {
                format!("visualize_rough_idea_fusion_blossom.json")
            };
            let mut code: Box<dyn ExampleCode> = if is_circuit_level {
                Box::new(CircuitLevelPlanarCode::new_diagonal(7, 7, 0.2, half_weight, 0.2))
            } else {
                Box::new(PhenomenologicalPlanarCode::new(7, 7, 0.2, half_weight))
            };
            let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
            visualizer.set_positions(code.get_positions(), true);  // automatic center all vertices
            print_visualize_link_with_parameters(&visualize_filename, vec![(format!("patch"), format!("visualize_rough_idea_fusion_blossom"))]);
            // create dual module
            let initializer = code.get_initializer();
            let mut dual_module = DualModuleSerial::new(&initializer);
            // hardcode syndrome          1   2   0   3    5    4    6    7
            let syndrome_vertices = vec![25, 33, 20, 76, 203, 187, 243, 315];
            code.set_syndrome_vertices(&syndrome_vertices);
            // create dual nodes and grow them by half length
            let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
            // save snapshot
            visualizer.snapshot_combined(format!("initial"), vec![&interface_ptr, &dual_module]).unwrap();
            // first layer grow first
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[1].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[2].clone().unwrap(), quarter_weight);
            visualizer.snapshot_combined(format!("grow a quarter"), vec![&interface_ptr, &dual_module]).unwrap();
            // merge and match
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[1].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[2].clone().unwrap(), quarter_weight);
            visualizer.snapshot_combined(format!("find a match"), vec![&interface_ptr, &dual_module]).unwrap();
            // grow to boundary
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), half_weight);
            visualizer.snapshot_combined(format!("touch temporal boundary"), vec![&interface_ptr, &dual_module]).unwrap();
            // add more measurement rounds
            visualizer.snapshot_combined(format!("add measurement #2"), vec![&interface_ptr, &dual_module]).unwrap();
            visualizer.snapshot_combined(format!("add measurement #3"), vec![&interface_ptr, &dual_module]).unwrap();
            visualizer.snapshot_combined(format!("add measurement #4"), vec![&interface_ptr, &dual_module]).unwrap();
            // handle errors at measurement round 4
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[5].clone().unwrap(), half_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[4].clone().unwrap(), half_weight);
            visualizer.snapshot_combined(format!("grow a half"), vec![&interface_ptr, &dual_module]).unwrap();
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[5].clone().unwrap(), half_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[4].clone().unwrap(), half_weight);
            visualizer.snapshot_combined(format!("temporary match"), vec![&interface_ptr, &dual_module]).unwrap();
            // handle errors at measurement round 5
            visualizer.snapshot_combined(format!("add measurement #5"), vec![&interface_ptr, &dual_module]).unwrap();
            for _ in 0..4 {
                dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[4].clone().unwrap(), -quarter_weight);
                dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[5].clone().unwrap(), quarter_weight);
                dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[6].clone().unwrap(), quarter_weight);
                visualizer.snapshot_combined(format!("grow or shrink a quarter"), vec![&interface_ptr, &dual_module]).unwrap();
            }
            visualizer.snapshot_combined(format!("add measurement #6"), vec![&interface_ptr, &dual_module]).unwrap();
            visualizer.snapshot_combined(format!("add measurement #7"), vec![&interface_ptr, &dual_module]).unwrap();
            visualizer.snapshot_combined(format!("add measurement #8"), vec![&interface_ptr, &dual_module]).unwrap();
        }
    }

}
