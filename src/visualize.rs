//! Visualizer
//!
//! This module helps visualize the progress of a fusion blossom algorithm
//!

use crate::chrono::Local;
use crate::serde::{Deserialize, Serialize};
use crate::serde_json;
use crate::urlencoding;
#[cfg(feature = "python_binding")]
use crate::util::*;
#[cfg(feature = "python_binding")]
use pyo3::prelude::*;
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};

pub trait FusionVisualizer {
    /// take a snapshot, set `abbrev` to true to save space
    fn snapshot(&self, abbrev: bool) -> serde_json::Value;
}

#[macro_export]
macro_rules! bind_trait_fusion_visualizer {
    ($struct_name:ident) => {
        #[cfg(feature = "python_binding")]
        #[pymethods]
        impl $struct_name {
            #[pyo3(name = "snapshot", signature = (abbrev = true))]
            fn trait_snapshot(&self, abbrev: bool) -> PyObject {
                json_to_pyobject(self.snapshot(abbrev))
            }
        }
    };
}
#[allow(unused_imports)]
pub use bind_trait_fusion_visualizer;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct VisualizePosition {
    /// vertical axis, -i is up, +i is down (left-up corner is smallest i,j)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub i: f64,
    /// horizontal axis, -j is left, +j is right (left-up corner is smallest i,j)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub j: f64,
    /// time axis, top and bottom (orthogonal to the initial view, which looks at -t direction)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub t: f64,
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl VisualizePosition {
    /// create a visualization position
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new(i: f64, j: f64, t: f64) -> Self {
        Self { i, j, t }
    }
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct Visualizer {
    /// save to file if applicable
    file: Option<File>,
    /// if waiting for the first snapshot
    empty_snapshot: bool,
    /// names of the snapshots
    #[cfg_attr(feature = "python_binding", pyo3(get))]
    pub snapshots: Vec<String>,
}

pub fn snapshot_fix_missing_fields(value: &mut serde_json::Value, abbrev: bool) {
    let value = value.as_object_mut().expect("snapshot must be an object");
    // fix vertices missing fields
    let vertices = value
        .get_mut("vertices")
        .expect("missing unrecoverable field")
        .as_array_mut()
        .expect("vertices must be an array");
    for vertex in vertices {
        if vertex.is_null() {
            continue;
        } // vertex not present, probably currently don't care
        let vertex = vertex.as_object_mut().expect("each vertex must be an object");
        let key_is_virtual = if abbrev { "v" } else { "is_virtual" };
        let key_is_defect = if abbrev { "s" } else { "is_defect" };
        // recover
        assert!(vertex.contains_key(key_is_virtual), "missing unrecoverable field");
        if !vertex.contains_key(key_is_defect) {
            vertex.insert(key_is_defect.to_string(), json!(0)); // by default no syndrome
        }
    }
    // fix edges missing fields
    let edges = value
        .get_mut("edges")
        .expect("missing unrecoverable field")
        .as_array_mut()
        .expect("edges must be an array");
    for edge in edges {
        if edge.is_null() {
            continue;
        } // edge not present, probably currently don't care
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
            edge.insert(key_left_growth.to_string(), json!(0)); // by default no growth
        }
        if !edge.contains_key(key_right_growth) {
            edge.insert(key_right_growth.to_string(), json!(0)); // by default no growth
        }
    }
}

pub type ObjectMap = serde_json::Map<String, serde_json::Value>;
pub fn snapshot_combine_object_known_key(obj: &mut ObjectMap, obj_2: &mut ObjectMap, key: &str) {
    match (obj.contains_key(key), obj_2.contains_key(key)) {
        (_, false) => {} // do nothing
        (false, true) => {
            obj.insert(key.to_string(), obj_2.remove(key).unwrap());
        }
        (true, true) => {
            // println!("[snapshot_combine_object_known_key] {}: {:?} == {:?}", key, obj[key], obj_2[key]);
            assert_eq!(
                obj[key], obj_2[key],
                "cannot combine different values: please make sure values don't conflict"
            );
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
            false => {
                obj.insert(key.to_string(), obj_2.remove(key).unwrap());
            }
            true => {
                // println!("[snapshot_copy_remaining_fields] {}: {:?} == {:?}", key, obj[key], obj_2[key]);
                // println!("obj: {obj:?}");
                // println!("obj_2: {obj_2:?}");
                assert_eq!(
                    obj[key], obj_2[key],
                    "cannot combine unknown fields of key `{}`: please modify `snapshot_combine_values` function",
                    key
                );
                obj_2.remove(key).unwrap();
            }
        }
    }
}

pub fn snapshot_combine_values(value: &mut serde_json::Value, mut value_2: serde_json::Value, abbrev: bool) {
    let value = value.as_object_mut().expect("snapshot must be an object");
    let value_2 = value_2.as_object_mut().expect("snapshot must be an object");
    match (value.contains_key("vertices"), value_2.contains_key("vertices")) {
        (_, false) => {} // do nothing
        (false, true) => {
            value.insert("vertices".to_string(), value_2.remove("vertices").unwrap());
        }
        (true, true) => {
            // combine
            let vertices = value
                .get_mut("vertices")
                .unwrap()
                .as_array_mut()
                .expect("vertices must be an array");
            let vertices_2 = value_2
                .get_mut("vertices")
                .unwrap()
                .as_array_mut()
                .expect("vertices must be an array");
            assert!(vertices.len() == vertices_2.len(), "vertices must be compatible");
            for (vertex_idx, vertex) in vertices.iter_mut().enumerate() {
                let vertex_2 = &mut vertices_2[vertex_idx];
                if vertex_2.is_null() {
                    continue;
                }
                if vertex.is_null() {
                    *vertex = vertex_2.clone();
                    continue;
                }
                // println!("vertex_idx: {vertex_idx}");
                let vertex = vertex.as_object_mut().expect("each vertex must be an object");
                let vertex_2 = vertex_2.as_object_mut().expect("each vertex must be an object");
                // list known keys
                let key_is_virtual = if abbrev { "v" } else { "is_virtual" };
                let key_is_defect = if abbrev { "s" } else { "is_defect" };
                let known_keys = [key_is_virtual, key_is_defect];
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
        (_, false) => {} // do nothing
        (false, true) => {
            value.insert("edges".to_string(), value_2.remove("edges").unwrap());
        }
        (true, true) => {
            // combine
            let edges = value
                .get_mut("edges")
                .unwrap()
                .as_array_mut()
                .expect("edges must be an array");
            let edges_2 = value_2
                .get_mut("edges")
                .unwrap()
                .as_array_mut()
                .expect("edges must be an array");
            assert!(edges.len() == edges_2.len(), "edges must be compatible");
            for (edge_idx, edge) in edges.iter_mut().enumerate() {
                let edge_2 = &mut edges_2[edge_idx];
                if edge_2.is_null() {
                    continue;
                }
                if edge.is_null() {
                    *edge = edge_2.clone();
                    continue;
                }
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
        (_, false) => {} // do nothing
        (false, true) => {
            value.insert("dual_nodes".to_string(), value_2.remove("dual_nodes").unwrap());
        }
        (true, true) => {
            // combine
            let dual_nodes = value
                .get_mut("dual_nodes")
                .unwrap()
                .as_array_mut()
                .expect("dual_nodes must be an array");
            let dual_nodes_2 = value_2
                .get_mut("dual_nodes")
                .unwrap()
                .as_array_mut()
                .expect("dual_nodes must be an array");
            assert!(dual_nodes.len() == dual_nodes_2.len(), "dual_nodes must be compatible");
            for (dual_node_idx, dual_node) in dual_nodes.iter_mut().enumerate() {
                let dual_node_2 = &mut dual_nodes_2[dual_node_idx];
                if dual_node.is_null() {
                    assert!(
                        dual_node_2.is_null(),
                        "dual node must be simultaneously be null, if necessary"
                    );
                    continue;
                }
                let dual_node = dual_node.as_object_mut().expect("each dual_node must be an object");
                let dual_node_2 = dual_node_2.as_object_mut().expect("each dual_node must be an object");
                // list known keys
                let key_boundary = if abbrev { "b" } else { "boundary" };
                let key_dual_variable = if abbrev { "d" } else { "dual_variable" };
                let key_blossom = if abbrev { "o" } else { "blossom" };
                let key_defect_vertex = if abbrev { "s" } else { "defect_vertex" };
                let key_grow_state = if abbrev { "g" } else { "grow_state" };
                let key_unit_growth = if abbrev { "u" } else { "unit_growth" };
                let key_parent_blossom = if abbrev { "p" } else { "parent_blossom" };
                let known_keys = [
                    key_boundary,
                    key_dual_variable,
                    key_blossom,
                    key_defect_vertex,
                    key_grow_state,
                    key_unit_growth,
                    key_parent_blossom,
                ];
                for key in known_keys {
                    snapshot_combine_object_known_key(dual_node, dual_node_2, key);
                }
                snapshot_copy_remaining_fields(dual_node, dual_node_2);
                assert_eq!(dual_node_2.len(), 0, "there should be nothing left");
            }
            value_2.remove("dual_nodes").unwrap();
        }
    }
    match (value.contains_key("primal_nodes"), value_2.contains_key("primal_nodes")) {
        (_, false) => {} // do nothing
        (false, true) => {
            value.insert("primal_nodes".to_string(), value_2.remove("primal_nodes").unwrap());
        }
        (true, true) => {
            // combine
            let primal_nodes = value
                .get_mut("primal_nodes")
                .unwrap()
                .as_array_mut()
                .expect("primal_nodes must be an array");
            let primal_nodes_2 = value_2
                .get_mut("primal_nodes")
                .unwrap()
                .as_array_mut()
                .expect("primal_nodes must be an array");
            // ideally, the two primal nodes should have the same length, but here we omit it
            // assert!(primal_nodes.len() == primal_nodes_2.len(), "primal_nodes must be compatible");
            if primal_nodes_2.len() > primal_nodes.len() {
                std::mem::swap(primal_nodes, primal_nodes_2);
            }
            debug_assert!(primal_nodes.len() >= primal_nodes_2.len());
            for (primal_node_idx, primal_node) in primal_nodes.iter_mut().enumerate() {
                if primal_node_idx >= primal_nodes_2.len() {
                    break;
                }
                let primal_node_2 = &mut primal_nodes_2[primal_node_idx];
                if primal_node_2.is_null() {
                    continue;
                }
                if primal_node.is_null() {
                    std::mem::swap(primal_node, primal_node_2);
                    continue;
                }
                let primal_node = primal_node.as_object_mut().expect("each primal_node must be an object");
                let primal_node_2 = primal_node_2.as_object_mut().expect("each primal_node must be an object");
                // list known keys
                let key_tree_node = if abbrev { "t" } else { "tree_node" };
                let key_temporary_match = if abbrev { "m" } else { "temporary_match" };
                let known_keys = [key_tree_node, key_temporary_match];
                for key in known_keys {
                    snapshot_combine_object_known_key(primal_node, primal_node_2, key);
                }
                snapshot_copy_remaining_fields(primal_node, primal_node_2);
                assert_eq!(primal_node_2.len(), 0, "there should be nothing left");
            }
            value_2.remove("primal_nodes").unwrap();
        }
    }
    snapshot_copy_remaining_fields(value, value_2);
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn center_positions(mut positions: Vec<VisualizePosition>) -> Vec<VisualizePosition> {
    if !positions.is_empty() {
        let mut max_i = positions[0].i;
        let mut min_i = positions[0].i;
        let mut max_j = positions[0].j;
        let mut min_j = positions[0].j;
        let mut max_t = positions[0].t;
        let mut min_t = positions[0].t;
        for position in positions.iter_mut() {
            if position.i > max_i {
                max_i = position.i;
            }
            if position.j > max_j {
                max_j = position.j;
            }
            if position.t > max_t {
                max_t = position.t;
            }
            if position.i < min_i {
                min_i = position.i;
            }
            if position.j < min_j {
                min_j = position.j;
            }
            if position.t < min_t {
                min_t = position.t;
            }
        }
        let (ci, cj, ct) = ((max_i + min_i) / 2., (max_j + min_j) / 2., (max_t + min_t) / 2.);
        for position in positions.iter_mut() {
            position.i -= ci;
            position.j -= cj;
            position.t -= ct;
        }
    }
    positions
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl Visualizer {
    /// create a new visualizer with target filename and node layout
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (filepath, positions=vec![], center=true)))]
    pub fn new(mut filepath: Option<String>, mut positions: Vec<VisualizePosition>, center: bool) -> std::io::Result<Self> {
        if cfg!(feature = "disable_visualizer") {
            filepath = None; // do not open file
        }
        if center {
            positions = center_positions(positions);
        }
        let mut file = match filepath {
            Some(filepath) => Some(File::create(filepath)?),
            None => None,
        };
        if let Some(file) = file.as_mut() {
            file.set_len(0)?; // truncate the file
            file.seek(SeekFrom::Start(0))?; // move the cursor to the front
            file.write_all(
                format!(
                    "{{\"format\":\"fusion_blossom\",\"version\":\"{}\"",
                    env!("CARGO_PKG_VERSION")
                )
                .as_bytes(),
            )?;
            file.write_all(b",\"positions\":")?;
            file.write_all(json!(positions).to_string().as_bytes())?;
            file.write_all(b",\"snapshots\":[]}")?;
            file.sync_all()?;
        }
        Ok(Self {
            file,
            empty_snapshot: true,
            snapshots: vec![],
        })
    }

    #[cfg(feature = "python_binding")]
    #[pyo3(name = "snapshot_combined")]
    pub fn snapshot_combined_py(&mut self, name: String, object_pys: Vec<&PyAny>) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let mut values = Vec::<serde_json::Value>::with_capacity(object_pys.len());
        for object_py in object_pys.into_iter() {
            values.push(pyobject_to_json(object_py.call_method0("snapshot")?.extract::<PyObject>()?));
        }
        self.snapshot_combined_value(name, values)
    }

    #[cfg(feature = "python_binding")]
    #[pyo3(name = "snapshot")]
    pub fn snapshot_py(&mut self, name: String, object_py: &PyAny) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let value = pyobject_to_json(object_py.call_method0("snapshot")?.extract::<PyObject>()?);
        self.snapshot_value(name, value)
    }

    #[cfg(feature = "python_binding")]
    #[pyo3(name = "snapshot_combined_value")]
    pub fn snapshot_combined_value_py(&mut self, name: String, value_pys: Vec<PyObject>) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let values: Vec<_> = value_pys.into_iter().map(|value_py| pyobject_to_json(value_py)).collect();
        self.snapshot_combined_value(name, values)
    }

    #[cfg(feature = "python_binding")]
    #[pyo3(name = "snapshot_value")]
    pub fn snapshot_value_py(&mut self, name: String, value_py: PyObject) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let value = pyobject_to_json(value_py);
        self.snapshot_value(name, value)
    }
}

impl Visualizer {
    pub fn incremental_save(&mut self, name: String, value: serde_json::Value) -> std::io::Result<()> {
        if let Some(file) = self.file.as_mut() {
            self.snapshots.push(name.clone());
            file.seek(SeekFrom::End(-2))?; // move the cursor before the ending ]}
            if !self.empty_snapshot {
                file.write_all(b",")?;
            }
            self.empty_snapshot = false;
            file.write_all(json!((name, value)).to_string().as_bytes())?;
            file.write_all(b"]}")?;
            file.sync_all()?;
        }
        Ok(())
    }

    /// append another snapshot of the fusion type, and also update the file in case
    pub fn snapshot_combined(&mut self, name: String, fusion_algorithms: Vec<&dyn FusionVisualizer>) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let abbrev = true;
        let mut value = json!({});
        for fusion_algorithm in fusion_algorithms.iter() {
            let value_2 = fusion_algorithm.snapshot(abbrev);
            snapshot_combine_values(&mut value, value_2, abbrev);
        }
        snapshot_fix_missing_fields(&mut value, abbrev);
        self.incremental_save(name, value)?;
        Ok(())
    }

    /// append another snapshot of the fusion type, and also update the file in case
    pub fn snapshot(&mut self, name: String, fusion_algorithm: &impl FusionVisualizer) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let abbrev = true;
        let mut value = fusion_algorithm.snapshot(abbrev);
        snapshot_fix_missing_fields(&mut value, abbrev);
        self.incremental_save(name, value)?;
        Ok(())
    }

    pub fn snapshot_combined_value(&mut self, name: String, values: Vec<serde_json::Value>) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let abbrev = true;
        let mut value = json!({});
        for value_2 in values.into_iter() {
            snapshot_combine_values(&mut value, value_2, abbrev);
        }
        snapshot_fix_missing_fields(&mut value, abbrev);
        self.incremental_save(name, value)?;
        Ok(())
    }

    pub fn snapshot_value(&mut self, name: String, mut value: serde_json::Value) -> std::io::Result<()> {
        if cfg!(feature = "disable_visualizer") {
            return Ok(());
        }
        let abbrev = true;
        snapshot_fix_missing_fields(&mut value, abbrev);
        self.incremental_save(name, value)?;
        Ok(())
    }
}

const DEFAULT_VISUALIZE_DATA_FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/visualize/data/");

// only used locally, because this is compile time directory
pub fn visualize_data_folder() -> String {
    DEFAULT_VISUALIZE_DATA_FOLDER.to_string()
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn static_visualize_data_filename() -> String {
    "visualizer.json".to_string()
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn auto_visualize_data_filename() -> String {
    format!("{}.json", Local::now().format("%Y%m%d-%H-%M-%S%.3f"))
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn print_visualize_link_with_parameters(filename: String, parameters: Vec<(String, String)>) {
    let default_port = if cfg!(feature = "python_binding") { 51666 } else { 8066 };
    let mut link = format!("http://localhost:{}?filename={}", default_port, filename);
    for (key, value) in parameters.iter() {
        link.push('&');
        link.push_str(&urlencoding::encode(key));
        link.push('=');
        link.push_str(&urlencoding::encode(value));
    }
    if cfg!(feature = "python_binding") {
        println!(
            "opening link {} (use `fusion_blossom.open_visualizer(filename)` to start a server and open it in browser)",
            link
        )
    } else {
        println!("opening link {} (start local server by running ./visualize/server.sh) or call `node index.js <link>` to render locally", link)
    }
}

#[cfg_attr(feature = "python_binding", pyfunction)]
pub fn print_visualize_link(filename: String) {
    print_visualize_link_with_parameters(filename, Vec::new())
}

#[cfg(feature = "python_binding")]
#[pyfunction]
pub(crate) fn register(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<VisualizePosition>()?;
    m.add_class::<Visualizer>()?;
    m.add_function(wrap_pyfunction!(static_visualize_data_filename, m)?)?;
    m.add_function(wrap_pyfunction!(auto_visualize_data_filename, m)?)?;
    m.add_function(wrap_pyfunction!(print_visualize_link_with_parameters, m)?)?;
    m.add_function(wrap_pyfunction!(print_visualize_link, m)?)?;
    m.add_function(wrap_pyfunction!(center_positions, m)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::dual_module::*;
    use super::super::dual_module_serial::*;
    use super::super::example_codes::*;
    use super::super::pointers::*;
    use super::super::primal_module::*;
    use super::super::primal_module_serial::*;
    use super::super::*;
    use super::*;

    #[test]
    fn visualize_test_1() {
        // cargo test visualize_test_1 -- --nocapture
        let visualize_filename = "visualize_test_1.json".to_string();
        let half_weight = 500;
        let mut code = CodeCapacityPlanarCode::new(11, 0.2, half_weight);
        let mut visualizer = Visualizer::new(
            Some(visualize_data_folder() + visualize_filename.as_str()),
            code.get_positions(),
            true,
        )
        .unwrap();
        print_visualize_link(visualize_filename.clone());
        // create dual module
        let initializer = code.get_initializer();
        let mut dual_module = DualModuleSerial::new_empty(&initializer);
        let defect_vertices = [39, 63, 52, 100, 90];
        for defect_vertex in defect_vertices.iter() {
            code.vertices[*defect_vertex].is_defect = true;
        }
        let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
        visualizer
            .snapshot_combined("initial".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // create dual nodes and grow them by half length
        // test basic grow and shrink of a single tree node
        for _ in 0..4 {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), half_weight);
            visualizer
                .snapshot_combined("grow half weight".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
        }
        for _ in 0..4 {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), -half_weight);
            visualizer
                .snapshot_combined("shrink half weight".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
        }
        for _ in 0..3 {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), half_weight);
        }
        visualizer
            .snapshot_combined("grow 3 half weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        for _ in 0..3 {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), -half_weight);
        }
        visualizer
            .snapshot_combined("shrink 3 half weight".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
        // test all
        for i in 0..interface_ptr.read_recursive().nodes_length {
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[i].clone().unwrap(), half_weight);
            visualizer
                .snapshot_combined("grow half weight".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
        }
    }

    #[test]
    fn visualize_paper_weighted_union_find_decoder() {
        // cargo test visualize_paper_weighted_union_find_decoder -- --nocapture
        let visualize_filename = "visualize_paper_weighted_union_find_decoder.json".to_string();
        let d: VertexNum = 3;
        let td: VertexNum = 4;
        let p = 0.2f64;
        let row_vertex_num = (d - 1) + 2; // two virtual vertices at left and right
        let t_vertex_num = row_vertex_num * d; // `d` rows
        let half_vertex_num = t_vertex_num * td; // `td` layers
        let vertex_num = half_vertex_num * 2; // both X and Z type stabilizers altogether
        let half_weight: Weight = (10000. * ((1. - p).ln() - p.ln())).max(1.) as Weight;
        let weight = half_weight * 2; // to make sure weight is even number for ease of this test function
        let weighted_edges = {
            let mut weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)> = Vec::new();
            for is_z in [true, false] {
                for t in 0..td {
                    let t_bias = t * t_vertex_num + if is_z { 0 } else { half_vertex_num };
                    for row in 0..d {
                        let bias = t_bias + row * row_vertex_num;
                        for i in 0..d - 1 {
                            weighted_edges.push((bias + i, bias + i + 1, weight));
                        }
                        weighted_edges.push((bias, bias + d, weight)); // left most edge
                        if row + 1 < d {
                            for i in 0..d - 1 {
                                weighted_edges.push((bias + i, bias + i + row_vertex_num, weight));
                            }
                        }
                    }
                    // inter-layer connection
                    if t + 1 < td {
                        for row in 0..d {
                            let bias = t_bias + row * row_vertex_num;
                            for i in 0..d - 1 {
                                weighted_edges.push((bias + i, bias + i + t_vertex_num, weight));
                                // diagonal edges
                                let diagonal_diffs: Vec<(isize, isize)> = if is_z {
                                    vec![(0, 1), (1, 0), (1, 1)]
                                } else {
                                    // i and j are reversed if x stabilizer, not vec![(0, -2), (2, 0), (2, -2)]
                                    vec![(-1, 0), (0, 1), (-1, 1)]
                                };
                                for (di, dj) in diagonal_diffs {
                                    let new_row = row as isize + di; // row corresponds to `i`
                                    let new_i = i as isize + dj; // i corresponds to `j`
                                    if new_row >= 0 && new_i >= 0 && new_row < d as isize && new_i < (d - 1) as isize {
                                        let new_bias = t_bias + (new_row as VertexNum) * row_vertex_num + t_vertex_num;
                                        weighted_edges.push((bias + i, new_bias + new_i as VertexNum, weight));
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
        let defect_vertices = vec![16, 29, 88, 72, 32, 44, 20, 21, 68, 69];
        let grow_edges = [48, 156, 169, 81, 38, 135];
        // run single-thread fusion blossom algorithm
        print_visualize_link_with_parameters(
            visualize_filename.clone(),
            vec![("patch".to_string(), "visualize_paper_weighted_union_find_decoder".to_string())],
        );
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
        let mut visualizer =
            Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
        let initializer = SolverInitializer::new(vertex_num, weighted_edges, virtual_vertices);
        let mut dual_module = DualModuleSerial::new_empty(&initializer);
        let interface_ptr =
            DualModuleInterfacePtr::new_load(&SyndromePattern::new_vertices(defect_vertices), &mut dual_module);
        // grow edges
        for &edge_index in grow_edges.iter() {
            let mut edge = dual_module.edges[edge_index].write_force();
            edge.left_growth = edge.weight;
        }
        // save snapshot
        visualizer
            .snapshot_combined("initial".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }

    #[test]
    fn visualize_rough_idea_fusion_blossom() {
        // cargo test visualize_rough_idea_fusion_blossom -- --nocapture
        let quarter_weight = 250;
        let half_weight = 2 * quarter_weight;
        for is_circuit_level in [false, true] {
            let visualize_filename = if is_circuit_level {
                "visualize_rough_idea_fusion_blossom_circuit_level.json".to_string()
            } else {
                "visualize_rough_idea_fusion_blossom.json".to_string()
            };
            let mut code: Box<dyn ExampleCode> = if is_circuit_level {
                Box::new(CircuitLevelPlanarCode::new_diagonal(7, 7, 0.2, half_weight, None))
            } else {
                Box::new(PhenomenologicalPlanarCode::new(7, 7, 0.2, half_weight))
            };
            let mut visualizer = Visualizer::new(
                Some(visualize_data_folder() + visualize_filename.as_str()),
                code.get_positions(),
                true,
            )
            .unwrap();
            print_visualize_link_with_parameters(
                visualize_filename,
                vec![("patch".to_string(), "visualize_rough_idea_fusion_blossom".to_string())],
            );
            // create dual module
            let initializer = code.get_initializer();
            let mut dual_module = DualModuleSerial::new_empty(&initializer);
            // hardcode syndrome          1   2   0   3    5    4    6    7
            let defect_vertices = vec![25, 33, 20, 76, 203, 187, 243, 315];
            code.set_defect_vertices(&defect_vertices);
            // create dual nodes and grow them by half length
            let interface_ptr = DualModuleInterfacePtr::new_load(&code.get_syndrome(), &mut dual_module);
            // save snapshot
            visualizer
                .snapshot_combined("initial".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            // first layer grow first
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[1].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[2].clone().unwrap(), quarter_weight);
            visualizer
                .snapshot_combined("grow a quarter".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            // merge and match
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[1].clone().unwrap(), quarter_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[2].clone().unwrap(), quarter_weight);
            visualizer
                .snapshot_combined("find a match".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            // grow to boundary
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[0].clone().unwrap(), half_weight);
            visualizer
                .snapshot_combined("touch temporal boundary".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            // add more measurement rounds
            visualizer
                .snapshot_combined("add measurement #2".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            visualizer
                .snapshot_combined("add measurement #3".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            visualizer
                .snapshot_combined("add measurement #4".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            // handle errors at measurement round 4
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[5].clone().unwrap(), half_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[4].clone().unwrap(), half_weight);
            visualizer
                .snapshot_combined("grow a half".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[5].clone().unwrap(), half_weight);
            dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[4].clone().unwrap(), half_weight);
            visualizer
                .snapshot_combined("temporary match".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            // handle errors at measurement round 5
            visualizer
                .snapshot_combined("add measurement #5".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            for _ in 0..4 {
                dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[4].clone().unwrap(), -quarter_weight);
                dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[5].clone().unwrap(), quarter_weight);
                dual_module.grow_dual_node(&interface_ptr.read_recursive().nodes[6].clone().unwrap(), quarter_weight);
                visualizer
                    .snapshot_combined("grow or shrink a quarter".to_string(), vec![&interface_ptr, &dual_module])
                    .unwrap();
            }
            visualizer
                .snapshot_combined("add measurement #6".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            visualizer
                .snapshot_combined("add measurement #7".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
            visualizer
                .snapshot_combined("add measurement #8".to_string(), vec![&interface_ptr, &dual_module])
                .unwrap();
        }
    }

    #[test]
    #[allow(clippy::unnecessary_cast)]
    fn visualize_example_syndrome_graph() {
        // cargo test visualize_example_syndrome_graph -- --nocapture
        let visualize_filename = "visualize_example_syndrome_graph.json".to_string();
        // let defect_vertices = vec![39, 52, 63, 90, 100];
        //                        0   1   2   3   4   5   6   7   8    9
        //                        A  vA   B  vB   C  vC   D  vD   E   vE
        let kept_vertices = [39, 47, 52, 59, 63, 71, 90, 94, 100, 107]; // including some virtual vertices
        let mut old_to_new = std::collections::BTreeMap::<DefectIndex, DefectIndex>::new();
        for (new_index, defect_vertex) in kept_vertices.iter().enumerate() {
            old_to_new.insert(*defect_vertex, new_index as DefectIndex);
        }
        println!("{old_to_new:?}");
        let d = 11;
        let half_weight = 500;
        let code = CodeCapacityPlanarCode::new(d, 0.1, half_weight);
        let positions = code.get_positions();
        let (ci, cj) = (
            (positions[131].i + positions[11].i) / 2.,
            (positions[10].j + positions[11].j) / 2.,
        );
        let syndrome_graph_positions: Vec<_> = kept_vertices
            .iter()
            .map(|i| {
                let mut position = positions[*i as usize].clone();
                position.i -= ci;
                position.j -= cj;
                position
            })
            .collect();
        let visualizer = Visualizer::new(
            Some(visualize_data_folder() + visualize_filename.as_str()),
            syndrome_graph_positions,
            false,
        )
        .unwrap();
        let mut visualizer = Some(visualizer);
        print_visualize_link(visualize_filename.clone());
        let syndrome_graph_edges = vec![
            // virtual to real edges
            (0, 1, 4000),
            (2, 3, 5000),
            (4, 5, 4000),
            (6, 7, 4000),
            (8, 9, 5000),
            // real to real edges
            (0, 2, 2000),
            (0, 4, 2000),
            (0, 6, 7000),
            (0, 8, 6000),
            (2, 4, 2000),
            (2, 6, 5000),
            (2, 8, 4000),
            (4, 6, 5000),
            (4, 8, 4000),
            (6, 8, 3000),
        ];
        let syndrome_graph_initializer =
            SolverInitializer::new(kept_vertices.len() as VertexNum, syndrome_graph_edges, vec![1, 3, 5, 7, 9]);
        println!("syndrome_graph_initializer: {syndrome_graph_initializer:?}");
        let mut dual_module = DualModuleSerial::new_empty(&syndrome_graph_initializer);
        // create primal module
        let mut primal_module = PrimalModuleSerialPtr::new_empty(&syndrome_graph_initializer);
        let interface_ptr = DualModuleInterfacePtr::new_empty();
        let syndrome_graph_syndrome = SyndromePattern::new(vec![0, 2, 4, 6, 8], vec![]);
        primal_module.solve_visualizer(
            &interface_ptr,
            &syndrome_graph_syndrome,
            &mut dual_module,
            visualizer.as_mut(),
        );
        let perfect_matching = primal_module.perfect_matching(&interface_ptr, &mut dual_module);
        let mut subgraph_builder = SubGraphBuilder::new(&syndrome_graph_initializer);
        subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer.as_mut() {
            visualizer
                .snapshot_combined(
                    "perfect matching and subgraph".to_string(),
                    vec![
                        &interface_ptr,
                        &dual_module,
                        &perfect_matching,
                        &VisualizeSubgraph::new(&subgraph),
                    ],
                )
                .unwrap();
        }
    }
}
