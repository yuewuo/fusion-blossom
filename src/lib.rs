#![cfg_attr(feature = "unsafe_pointer", feature(get_mut_unchecked))]
#![cfg_attr(feature = "unsafe_pointer", allow(unused_mut))]
#![cfg_attr(feature = "python_binding", feature(cfg_eval))]

extern crate cfg_if;
extern crate libc;
extern crate parking_lot;
extern crate priority_queue;
extern crate rand_xoshiro;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate chrono;
extern crate clap;
extern crate core_affinity;
extern crate derivative;
#[cfg(test)]
extern crate petgraph;
#[cfg(feature = "python_binding")]
extern crate pyo3;
#[cfg(feature = "qecp_integrate")]
pub extern crate qecp;
extern crate rand;
extern crate rayon;
extern crate urlencoding;
#[cfg(feature = "wasm_binding")]
extern crate wasm_bindgen;
extern crate weak_table;

pub mod blossom_v;
pub mod cli;
pub mod complete_graph;
pub mod dual_module;
pub mod dual_module_parallel;
pub mod dual_module_serial;
pub mod example_codes;
pub mod example_partition;
pub mod mwpm_solver;
pub mod pointers;
pub mod primal_module;
pub mod primal_module_parallel;
pub mod primal_module_serial;
pub mod util;
pub mod visualize;
#[cfg(feature = "python_binding")]
use pyo3::prelude::*;

use complete_graph::*;
use util::*;

#[cfg(feature = "python_binding")]
#[pymodule]
fn fusion_blossom(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    util::register(py, m)?;
    mwpm_solver::register(py, m)?;
    example_codes::register(py, m)?;
    visualize::register(py, m)?;
    primal_module::register(py, m)?;
    let helper_code = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/helper.py"));
    let helper_module = PyModule::from_code(py, helper_code, "helper", "helper")?;
    helper_module.add("visualizer_website", generate_visualizer_website(py))?;
    let bottle_code = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bottle.py")); // embed bottle
    helper_module.add_submodule(PyModule::from_code(py, bottle_code, "bottle", "bottle")?)?;
    m.add_submodule(helper_module)?;
    let helper_register = helper_module.getattr("register")?;
    helper_register.call1((m,))?;
    Ok(())
}

/// use fusion blossom to solve MWPM (to optimize speed, consider reuse a [`mwpm_solver::SolverSerial`] object)
#[allow(clippy::unnecessary_cast)]
pub fn fusion_mwpm(initializer: &SolverInitializer, syndrome_pattern: &SyndromePattern) -> Vec<VertexIndex> {
    // sanity check
    assert!(initializer.vertex_num > 1, "at least one vertex required");
    let max_safe_weight = ((Weight::MAX as usize) / initializer.vertex_num as usize) as Weight;
    for (i, j, weight) in initializer.weighted_edges.iter() {
        if weight > &max_safe_weight {
            panic!(
                "edge {}-{} has weight {} > max safe weight {}, it may cause fusion blossom to overflow",
                i, j, weight, max_safe_weight
            );
        }
    }
    // by default use serial implementation fusion blossom
    mwpm_solver::LegacySolverSerial::mwpm_solve(initializer, syndrome_pattern)
}

/// fall back to use blossom V library to solve MWPM (install blossom V required)
#[allow(clippy::unnecessary_cast)]
pub fn blossom_v_mwpm(initializer: &SolverInitializer, defect_vertices: &[VertexIndex]) -> Vec<VertexIndex> {
    // this feature will be automatically enabled if you install blossom V source code, see README.md for more information
    if cfg!(not(feature = "blossom_v")) {
        panic!("need blossom V library, see README.md")
    }
    // sanity check
    assert!(initializer.vertex_num > 1, "at least one vertex required");
    let max_safe_weight = ((i32::MAX as usize) / initializer.vertex_num as usize) as Weight;
    for (i, j, weight) in initializer.weighted_edges.iter() {
        if weight > &max_safe_weight {
            panic!(
                "edge {}-{} has weight {} > max safe weight {}, it may cause blossom V library to overflow",
                i, j, weight, max_safe_weight
            );
        }
    }
    let mut complete_graph = CompleteGraph::new(initializer.vertex_num, &initializer.weighted_edges);
    blossom_v_mwpm_reuse(&mut complete_graph, initializer, defect_vertices)
}

#[allow(clippy::unnecessary_cast)]
pub fn blossom_v_mwpm_reuse(
    complete_graph: &mut CompleteGraph,
    initializer: &SolverInitializer,
    defect_vertices: &[VertexIndex],
) -> Vec<VertexIndex> {
    // first collect virtual vertices and real vertices
    let mut is_virtual: Vec<bool> = (0..initializer.vertex_num).map(|_| false).collect();
    let mut is_defect: Vec<bool> = (0..initializer.vertex_num).map(|_| false).collect();
    for &virtual_vertex in initializer.virtual_vertices.iter() {
        assert!(virtual_vertex < initializer.vertex_num, "invalid input");
        assert!(!is_virtual[virtual_vertex as usize], "same virtual vertex appears twice");
        is_virtual[virtual_vertex as usize] = true;
    }
    let mut mapping_to_defect_vertices: Vec<usize> = (0..initializer.vertex_num).map(|_| usize::MAX).collect();
    for (i, &defect_vertex) in defect_vertices.iter().enumerate() {
        assert!(defect_vertex < initializer.vertex_num, "invalid input");
        assert!(!is_virtual[defect_vertex as usize], "syndrome vertex cannot be virtual");
        assert!(!is_defect[defect_vertex as usize], "same syndrome vertex appears twice");
        is_defect[defect_vertex as usize] = true;
        mapping_to_defect_vertices[defect_vertex as usize] = i;
    }
    // for each real vertex, add a corresponding virtual vertex to be matched
    let defect_num = defect_vertices.len();
    let legacy_vertex_num = defect_num * 2;
    let mut legacy_weighted_edges = Vec::<(usize, usize, u32)>::new();
    let mut boundaries = Vec::<Option<(VertexIndex, Weight)>>::new();
    for (i, &defect_vertex) in defect_vertices.iter().enumerate() {
        let complete_graph_edges = complete_graph.all_edges(defect_vertex);
        let mut boundary: Option<(VertexIndex, Weight)> = None;
        for (&peer, &(_, weight)) in complete_graph_edges.iter() {
            if is_virtual[peer as usize] && (boundary.is_none() || weight < boundary.as_ref().unwrap().1) {
                boundary = Some((peer, weight));
            }
        }
        if let Some((_, weight)) = boundary {
            // connect this real vertex to it's corresponding virtual vertex
            legacy_weighted_edges.push((i, i + defect_num, weight as u32));
        }
        boundaries.push(boundary); // save for later resolve legacy matchings
        for (&peer, &(_, weight)) in complete_graph_edges.iter() {
            if is_defect[peer as usize] {
                let j = mapping_to_defect_vertices[peer as usize];
                if i < j {
                    // remove duplicated edges
                    legacy_weighted_edges.push((i, j, weight as u32));
                    // println!{"edge {} {} {} ", i, j, weight};
                }
            }
        }
        for j in (i + 1)..defect_num {
            // virtual boundaries are always fully connected with weight 0
            legacy_weighted_edges.push((i + defect_num, j + defect_num, 0));
        }
    }
    // run blossom V to get matchings
    // println!("[debug] legacy_vertex_num: {:?}", legacy_vertex_num);
    // println!("[debug] legacy_weighted_edges: {:?}", legacy_weighted_edges);
    let matchings = blossom_v::safe_minimum_weight_perfect_matching(legacy_vertex_num, &legacy_weighted_edges);
    let mut mwpm_result = Vec::new();
    for i in 0..defect_num {
        let j = matchings[i];
        if j < defect_num {
            // match to a real vertex
            mwpm_result.push(defect_vertices[j]);
        } else {
            assert_eq!(
                j,
                i + defect_num,
                "if not matched to another real vertex, it must match to it's corresponding virtual vertex"
            );
            mwpm_result.push(
                boundaries[i]
                    .as_ref()
                    .expect("boundary must exist if match to virtual vertex")
                    .0,
            );
        }
    }
    mwpm_result
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DetailedMatching {
    /// must be a real vertex
    pub a: DefectIndex,
    /// might be a virtual vertex, but if it's a real vertex, then b > a stands
    pub b: DefectIndex,
    /// every vertex in between this pair, in the order `a -> path[0].0 -> path[1].0 -> .... -> path[-1].0` and it's guaranteed that path[-1].0 = b; might be empty if a and b are adjacent
    pub path: Vec<(DefectIndex, Weight)>,
    /// the overall weight of this path
    pub weight: Weight,
}

/// compute detailed matching information, note that the output will not include duplicated matched pairs
#[allow(clippy::unnecessary_cast)]
pub fn detailed_matching(
    initializer: &SolverInitializer,
    defect_vertices: &[DefectIndex],
    mwpm_result: &[DefectIndex],
) -> Vec<DetailedMatching> {
    let defect_num = defect_vertices.len();
    let mut is_defect: Vec<bool> = (0..initializer.vertex_num).map(|_| false).collect();
    for &defect_vertex in defect_vertices.iter() {
        assert!(defect_vertex < initializer.vertex_num, "invalid input");
        assert!(!is_defect[defect_vertex as usize], "same syndrome vertex appears twice");
        is_defect[defect_vertex as usize] = true;
    }
    assert_eq!(defect_num, mwpm_result.len(), "invalid mwpm result");
    let mut complete_graph = complete_graph::CompleteGraph::new(initializer.vertex_num, &initializer.weighted_edges);
    let mut details = Vec::new();
    for i in 0..defect_num {
        let a = defect_vertices[i];
        let b = mwpm_result[i];
        if !is_defect[b as usize] || a < b {
            let (path, weight) = complete_graph.get_path(a, b);
            let detail = DetailedMatching { a, b, path, weight };
            details.push(detail);
        }
    }
    details
}

#[cfg(feature = "python_binding")]
macro_rules! include_visualize_file {
    ($mapping:ident, $filepath:expr) => {
        $mapping.insert($filepath.to_string(), include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/visualize/", $filepath)).to_string());
    };
    ($mapping:ident, $filepath:expr, $($other_filepath:expr),+) => {
        include_visualize_file!($mapping, $filepath);
        include_visualize_file!($mapping, $($other_filepath),+);
    };
}

#[cfg(feature = "python_binding")]
fn generate_visualizer_website(py: Python<'_>) -> &pyo3::types::PyDict {
    use pyo3::types::IntoPyDict;
    let mut mapping = std::collections::BTreeMap::<String, String>::new();
    include_visualize_file!(
        mapping,
        "gui3d.js",
        "index.js",
        "patches.js",
        "primal.js",
        "cmd.js",
        "mocker.js"
    );
    include_visualize_file!(mapping, "index.html", "partition-profile.html", "icon.svg");
    include_visualize_file!(mapping, "package.json", "package-lock.json");
    mapping.into_py_dict(py)
}

#[cfg(feature = "wasm_binding")]
use wasm_bindgen::prelude::*;

#[cfg_attr(feature = "wasm_binding", wasm_bindgen)]
pub fn get_version() -> String {
    "hello world".to_string()
}
