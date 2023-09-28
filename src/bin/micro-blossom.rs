// cargo run --bin micro-blossom
// see micro-blossom/resources/graphs/README.md

// generate by https://app.quicktype.io/

// Example code that deserializes and serializes the model.
// extern crate serde;
// #[macro_use]
// extern crate serde_derive;
// extern crate serde_json;
//
// use generated_module::MicroBlossomSingle;
//
// fn main() {
//     let json = r#"{"answer": 42}"#;
//     let model: MicroBlossomSingle = serde_json::from_str(&json).unwrap();
// }

use example_codes::*;
use fusion_blossom::*;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MicroBlossomSingle {
    positions: Vec<Position>,
    vertex_num: i64,
    weighted_edges: Vec<WeightedEdges>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Position {
    i: f64,
    j: f64,
    t: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WeightedEdges {
    l: i64,
    r: i64,
    w: i64,
}

fn generate_example(name: &str, code: impl ExampleCode) {
    let folder = "micro-blossom-examples";
    fs::create_dir_all(folder).unwrap();
    let filename = format!("{folder}/example_{name}.json");

    let initializer = code.get_initializer();
    let positions = code.get_positions();
    assert_eq!(positions.len(), initializer.vertex_num);

    let micro_blossom = MicroBlossomSingle {
        vertex_num: initializer.vertex_num.try_into().unwrap(),
        positions: positions.iter().map(|p| Position { t: p.t, i: p.i, j: p.j }).collect(),
        weighted_edges: initializer
            .weighted_edges
            .iter()
            .map(|e| WeightedEdges {
                l: e.0.try_into().unwrap(),
                r: e.1.try_into().unwrap(),
                w: e.2,
            })
            .collect(),
    };

    let json_str = serde_json::to_string(&micro_blossom).unwrap();
    fs::write(filename, json_str).unwrap();
}

fn main() {
    generate_example("code_capacity_d3", CodeCapacityRepetitionCode::new(3, 0.1, 50));
    generate_example("code_capacity_d5", CodeCapacityRepetitionCode::new(5, 0.1, 50));
    generate_example("code_capacity_planar_d3", CodeCapacityPlanarCode::new(3, 0.1, 50));
    generate_example("code_capacity_planar_d5", CodeCapacityPlanarCode::new(5, 0.1, 50));
    generate_example("code_capacity_planar_d7", CodeCapacityPlanarCode::new(7, 0.1, 50));
    generate_example("code_capacity_rotated_d3", CodeCapacityRotatedCode::new(3, 0.1, 50));
    generate_example("code_capacity_rotated_d5", CodeCapacityRotatedCode::new(5, 0.1, 50));
    generate_example("code_capacity_rotated_d7", CodeCapacityRotatedCode::new(7, 0.1, 50));
    generate_example("phenomenological_rotated_d3", PhenomenologicalRotatedCode::new(3, 3, 0.1, 50));
    generate_example("phenomenological_rotated_d5", PhenomenologicalRotatedCode::new(5, 5, 0.1, 50));
    generate_example("phenomenological_rotated_d7", PhenomenologicalRotatedCode::new(7, 7, 0.1, 50));
}
