// cargo run --bin aps2023

use dual_module::*;
use dual_module_serial::*;
use example_codes::*;
use fusion_blossom::*;
use pointers::*;
use primal_module::*;
use primal_module_serial::*;
use serde_json::*;
use std::fs::File;
use std::io::Write;
use util::*;
use visualize::*;

const APS2023_DECODING_GRAPH_SYNDROME_GRAPH_DEFECT_VERTICES: [VertexIndex; 16] =
    [64, 62, 37, 26, 15, 17, 30, 43, 56, 80, 91, 102, 113, 111, 98, 85];

fn demo_aps2023_decoding_graph_growing() {
    let visualize_filename = "demo_aps2023_decoding_graph_growing.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(11, 0.1, half_weight);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(APS2023_DECODING_GRAPH_SYNDROME_GRAPH_DEFECT_VERTICES.into());
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer
        .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    // create dual nodes and grow them by half length
    let dual_node_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
    for _ in 0..2 {
        dual_module.grow_dual_node(&dual_node_ptr, 2 * half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }
}

type MinPaths = std::collections::HashMap<(VertexIndex, VertexIndex), Weight>;
#[allow(clippy::unnecessary_cast)]
fn get_min_paths(code: &impl ExampleCode) -> MinPaths {
    use petgraph::algo::floyd_warshall;
    use petgraph::prelude::*;
    use std::collections::HashMap;
    let mut graph = UnGraph::<(), ()>::new_undirected();
    let mut nodes = vec![];
    let (vertices, edges) = code.immutable_vertices_edges();
    for _ in 0..vertices.len() {
        nodes.push(graph.add_node(()));
    }
    let mut weight_map = HashMap::<(NodeIndex, NodeIndex), Weight>::new();
    for edge in edges.iter() {
        let pair = (nodes[edge.vertices.0 as usize], nodes[edge.vertices.1 as usize]);
        graph.extend_with_edges([pair]);
        weight_map.insert(pair, edge.half_weight * 2);
    }
    let res = floyd_warshall(&graph, |edge| {
        if let Some(weight) = weight_map.get(&(edge.source(), edge.target())) {
            *weight
        } else {
            Weight::MAX
        }
    })
    .unwrap();
    let mut min_paths = MinPaths::new();
    for vertex_1 in 0..vertices.len() {
        for vertex_2 in vertex_1 + 1..vertices.len() {
            min_paths.insert(
                (vertex_1 as VertexIndex, vertex_2 as VertexIndex),
                *res.get(&(nodes[vertex_1], nodes[vertex_2])).unwrap(),
            );
            min_paths.insert(
                (vertex_2 as VertexIndex, vertex_1 as VertexIndex),
                *res.get(&(nodes[vertex_1], nodes[vertex_2])).unwrap(),
            );
        }
    }
    min_paths
}

#[allow(clippy::unnecessary_cast)]
fn get_nearest_virtual(
    min_paths: &MinPaths,
    code: &impl ExampleCode,
    source_vertex_index: VertexIndex,
) -> Option<VertexIndex> {
    assert!(!code.is_virtual(source_vertex_index as usize));
    let (vertices, _edges) = code.immutable_vertices_edges();
    let mut min_weight = Weight::MAX;
    let mut nearest_virtual = None;
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        if vertex.is_virtual {
            let path_weight = *min_paths
                .get(&(vertex_index as VertexIndex, source_vertex_index as VertexIndex))
                .unwrap();
            if path_weight < min_weight {
                nearest_virtual = Some(vertex_index as VertexIndex);
                min_weight = path_weight;
            }
        }
    }
    nearest_virtual
}

#[allow(clippy::unnecessary_cast)]
fn demo_construct_syndrome_graph(
    code: &impl ExampleCode,
    defect_vertices: &[VertexIndex],
) -> (SolverInitializer, SyndromePattern, Vec<VisualizePosition>) {
    use std::collections::BTreeMap;
    let min_paths = get_min_paths(code);
    let mut new_vertex_to_old = vec![];
    let mut old_vertex_to_new = BTreeMap::new();
    let mut new_defect_vertices = vec![];
    for &defect_vertex in defect_vertices {
        old_vertex_to_new.insert(defect_vertex, new_vertex_to_old.len() as VertexIndex);
        new_defect_vertices.push(new_vertex_to_old.len() as VertexIndex);
        new_vertex_to_old.push(defect_vertex);
    }
    // build complete graph between defect vertices
    let mut syndrome_graph_edges = vec![];
    for i in 0..defect_vertices.len() {
        for j in i + 1..defect_vertices.len() {
            let vertex_1 = defect_vertices[i];
            let vertex_2 = defect_vertices[j];
            let weight = *min_paths.get(&(vertex_1, vertex_2)).unwrap();
            syndrome_graph_edges.push((old_vertex_to_new[&vertex_1], old_vertex_to_new[&vertex_2], weight));
        }
    }
    // find the nearest virtual vertex
    let mut virtual_vertices_map = BTreeMap::<VertexIndex, Vec<(VertexIndex, Weight)>>::new();
    for &defect_vertex in defect_vertices {
        let virtual_vertex = get_nearest_virtual(&min_paths, code, defect_vertex);
        if let Some(virtual_vertex) = virtual_vertex {
            virtual_vertices_map.entry(virtual_vertex).or_default();
            virtual_vertices_map
                .get_mut(&virtual_vertex)
                .as_mut()
                .unwrap()
                .push((defect_vertex, *min_paths.get(&(defect_vertex, virtual_vertex)).unwrap()));
        }
    }
    let mut virtual_vertices = vec![];
    for (&virtual_vertex, edges) in virtual_vertices_map.iter() {
        old_vertex_to_new.insert(virtual_vertex, new_vertex_to_old.len() as VertexIndex);
        virtual_vertices.push(new_vertex_to_old.len() as VertexIndex);
        new_vertex_to_old.push(virtual_vertex);
        for &(defect_vertex, weight) in edges.iter() {
            syndrome_graph_edges.push((old_vertex_to_new[&defect_vertex], old_vertex_to_new[&virtual_vertex], weight));
        }
    }
    let initializer = SolverInitializer::new(new_vertex_to_old.len() as VertexNum, syndrome_graph_edges, virtual_vertices);
    let syndrome_pattern = SyndromePattern::new_vertices(new_defect_vertices);
    let old_positions = code.get_positions();
    let mut new_positions = vec![];
    for &old_vertex in new_vertex_to_old.iter() {
        new_positions.push(old_positions[old_vertex as usize].clone());
    }
    (initializer, syndrome_pattern, new_positions)
}

fn demo_aps2023_syndrome_graph_growing() {
    let visualize_filename = "demo_aps2023_syndrome_graph_growing.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(11, 0.1, half_weight);
    // construct the syndrome graph
    let (initializer, syndrome, positions) =
        demo_construct_syndrome_graph(&code, &APS2023_DECODING_GRAPH_SYNDROME_GRAPH_DEFECT_VERTICES);
    // create dual module
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let mut visualizer =
        Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
    print_visualize_link(visualize_filename);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer
        .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    // create dual nodes and grow them by half length
    let dual_node_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
    for _ in 0..2 {
        dual_module.grow_dual_node(&dual_node_ptr, 2 * half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }
}

fn demo_aps2023_decoding_graph_static() {
    let visualize_filename = "demo_aps2023_decoding_graph_static.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(vec![]);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer
        .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
}

const APS2023_EXAMPLE_DEFECT_VERTICES: [VertexIndex; 7] = [14, 13, 6, 3, 21, 25, 18];

fn demo_aps2023_example_decoding_graph() {
    let visualize_filename = "demo_aps2023_example_decoding_graph.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(APS2023_EXAMPLE_DEFECT_VERTICES.into());
    let mut primal_module = PrimalModuleSerialPtr::new_empty(&initializer);
    let interface_ptr = DualModuleInterfacePtr::new_empty();
    primal_module.solve_visualizer(&interface_ptr, &syndrome, &mut dual_module, Some(&mut visualizer));
    let perfect_matching = primal_module.perfect_matching(&interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
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

fn demo_aps2023_example_syndrome_graph() {
    let visualize_filename = "demo_aps2023_example_syndrome_graph.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    // construct the syndrome graph
    let (initializer, syndrome, positions) = demo_construct_syndrome_graph(&code, &APS2023_EXAMPLE_DEFECT_VERTICES);
    // create dual module
    let mut visualizer =
        Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
    print_visualize_link(visualize_filename);
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let mut primal_module = PrimalModuleSerialPtr::new_empty(&initializer);
    let interface_ptr = DualModuleInterfacePtr::new_empty();
    primal_module.solve_visualizer(&interface_ptr, &syndrome, &mut dual_module, Some(&mut visualizer));
    let perfect_matching = primal_module.perfect_matching(&interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
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

#[allow(clippy::unnecessary_cast)]
fn demo_aps2023_example_syndrome_graph_edges() {
    use std::collections::{BTreeMap, BTreeSet};
    let filename = "demo_aps2023_example_syndrome_graph_edges.json".to_string();
    let filepath = visualize_data_folder() + filename.as_str();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    let min_paths = get_min_paths(&code);
    let (vertices, edges) = code.immutable_vertices_edges();
    let defect_vertices: Vec<VertexIndex> = vec![14, 3, 21, 25, 18, 13, 6];
    assert_eq!(
        defect_vertices.iter().cloned().collect::<BTreeSet<_>>(),
        APS2023_EXAMPLE_DEFECT_VERTICES.iter().cloned().collect::<BTreeSet<_>>()
    );
    // construct paths for each defect vertices
    let mut result: BTreeMap<VertexIndex, serde_json::Value> = BTreeMap::new();
    for (idx, &defect_vertex) in defect_vertices.iter().enumerate() {
        // construct weight to other defect vertices
        let mut paths: BTreeMap<VertexIndex, Vec<VertexIndex>> = BTreeMap::new();
        for &peer_vertex in defect_vertices.iter().skip(idx + 1) {
            // find a minimum-weight path
            let mut current_vertex = defect_vertex;
            let mut min_path = vec![defect_vertex];
            while current_vertex != peer_vertex {
                // find next nearest vertex
                let mut next_nearest = current_vertex;
                let mut min_path_weight = *min_paths.get(&(peer_vertex, current_vertex)).unwrap();
                for &edge_index in vertices[current_vertex as usize].neighbor_edges.iter() {
                    let edge = &edges[edge_index as usize];
                    let (v1, v2) = edge.vertices;
                    let neighbor_vertex = if v1 == current_vertex { v2 } else { v1 };
                    if neighbor_vertex == peer_vertex {
                        next_nearest = peer_vertex;
                        break;
                    } else {
                        let path_weight = *min_paths.get(&(peer_vertex, neighbor_vertex)).unwrap();
                        if path_weight < min_path_weight {
                            min_path_weight = path_weight;
                            next_nearest = neighbor_vertex;
                        }
                    }
                }
                current_vertex = next_nearest;
                min_path.push(current_vertex);
            }
            paths.insert(peer_vertex, min_path);
        }
        // construct path to nearest virtual boundary
        let mut current_virtual = 0;
        let mut min_path_weight = Weight::MAX;
        for vertex_index in 0..vertices.len() as VertexIndex {
            let vertex = &vertices[vertex_index as usize];
            if vertex.is_virtual {
                let path_weight = *min_paths.get(&(defect_vertex, vertex_index)).unwrap();
                if path_weight < min_path_weight {
                    current_virtual = vertex_index;
                    min_path_weight = path_weight;
                }
            }
        }
        assert!(min_path_weight != Weight::MAX);
        let peer_vertex = current_virtual;
        let mut current_vertex = defect_vertex;
        let mut min_path = vec![defect_vertex];
        while current_vertex != peer_vertex {
            // find next nearest vertex
            let mut next_nearest = current_vertex;
            let mut min_path_weight = *min_paths.get(&(peer_vertex, current_vertex)).unwrap();
            for &edge_index in vertices[current_vertex as usize].neighbor_edges.iter() {
                let edge = &edges[edge_index as usize];
                let (v1, v2) = edge.vertices;
                let neighbor_vertex = if v1 == current_vertex { v2 } else { v1 };
                if neighbor_vertex == peer_vertex {
                    next_nearest = peer_vertex;
                    break;
                } else {
                    let path_weight = *min_paths.get(&(peer_vertex, neighbor_vertex)).unwrap();
                    if path_weight < min_path_weight {
                        min_path_weight = path_weight;
                        next_nearest = neighbor_vertex;
                    }
                }
            }
            current_vertex = next_nearest;
            min_path.push(current_vertex);
        }
        // add results
        result.insert(
            defect_vertex,
            json!({
                "paths": paths,
                "boundary": min_path,
            }),
        );
    }
    let mut file = File::create(filepath).unwrap();
    let positions = code.get_positions();
    let mut indices = vec![];
    for position in positions.iter() {
        let pos_i = position.i.round() as isize;
        let pos_j = position.j.round() as isize;
        let i = 2 * pos_i + 1;
        let j = 2 * pos_j + 2;
        indices.push((i, j));
    }
    file.set_len(0).unwrap(); // truncate the file
    file.write_all(
        json!({
            "map": result,
            "indices": indices,
            "defect_vertices": defect_vertices,
        })
        .to_string()
        .as_bytes(),
    )
    .unwrap();
    file.sync_all().unwrap();
}

fn demo_aps2023_example_decoding_graph_grow_single() {
    let visualize_filename = "demo_aps2023_example_decoding_graph_grow_single.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(APS2023_EXAMPLE_DEFECT_VERTICES.into());
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer
        .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    let dual_node_ptr = interface_ptr.read_recursive().nodes[2].clone().unwrap();
    dual_module.grow_dual_node(&dual_node_ptr, 2 * half_weight);
    visualizer
        .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    let dual_node_ptr_2 = interface_ptr.read_recursive().nodes[1].clone().unwrap();
    dual_module.grow_dual_node(&dual_node_ptr_2, 2 * half_weight);
    visualizer
        .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
}

fn demo_aps2023_example_syndrome_graph_grow_single() {
    let visualize_filename = "demo_aps2023_example_syndrome_graph_grow_single.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    // construct the syndrome graph
    let (initializer, syndrome, positions) = demo_construct_syndrome_graph(&code, &APS2023_EXAMPLE_DEFECT_VERTICES);
    // create dual module
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let mut visualizer =
        Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
    print_visualize_link(visualize_filename);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer
        .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    let dual_node_ptr = interface_ptr.read_recursive().nodes[2].clone().unwrap();
    dual_module.grow_dual_node(&dual_node_ptr, 2 * half_weight);
    visualizer
        .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    let dual_node_ptr_2 = interface_ptr.read_recursive().nodes[1].clone().unwrap();
    dual_module.grow_dual_node(&dual_node_ptr_2, 2 * half_weight);
    visualizer
        .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
}

fn demo_aps2023_example_partition() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = "demo_aps2023_example_partition.json".to_string();
    let half_weight = 500;
    let mut code = CodeCapacityPlanarCode::new(5, 0.1, half_weight);
    let mut partition = CodeCapacityPlanarCodeVerticalPartitionHalf::new(5, 3);
    let defect_vertices = partition.re_index_defect_vertices(&code, &APS2023_EXAMPLE_DEFECT_VERTICES);
    println!("defect_vertices: {defect_vertices:?}");
    let partition_config = partition.build_apply(&mut code);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    let initializer = code.get_initializer();
    let partition_info = partition_config.info();
    // create dual module
    let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(
        &initializer,
        &partition_info,
        DualModuleParallelConfig::default(),
    );
    let primal_config = PrimalModuleParallelConfig {
        debug_sequential: true,
        ..Default::default()
    };
    let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
    code.set_defect_vertices(&defect_vertices);
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &dual_module, Some(&mut visualizer));
    let useless_interface_ptr = DualModuleInterfacePtr::new_empty(); // don't actually use it
    let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    let last_interface_ptr = &primal_module.units.last().unwrap().read_recursive().interface_ptr;
    visualizer
        .snapshot_combined(
            "perfect matching and subgraph".to_string(),
            vec![
                last_interface_ptr,
                &dual_module,
                &perfect_matching,
                &VisualizeSubgraph::new(&subgraph),
            ],
        )
        .unwrap();
}

const DEMO_APS2023_LARGE_DEMO_RNG_SEED: u64 = 671;

fn demo_aps2023_large_demo() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = "demo_aps2023_large_demo.json".to_string();
    let half_weight = 500;
    let noisy_measurements = 10 * 4;
    let d = 5;
    let mut code = PhenomenologicalPlanarCode::new(d, noisy_measurements, 0.03, half_weight);
    let random_syndrome = code.generate_random_errors(DEMO_APS2023_LARGE_DEMO_RNG_SEED);
    let mut partition = PhenomenologicalPlanarCodeTimePartition::new_tree(d, noisy_measurements, 4, true, usize::MAX);
    let defect_vertices = partition.re_index_defect_vertices(&code, &random_syndrome.defect_vertices);
    let partition_config = partition.build_apply(&mut code);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    let initializer = code.get_initializer();
    let partition_info = partition_config.info();
    // create dual module
    let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(
        &initializer,
        &partition_info,
        DualModuleParallelConfig::default(),
    );
    let primal_config = PrimalModuleParallelConfig {
        debug_sequential: true,
        ..Default::default()
    };
    let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
    code.set_defect_vertices(&defect_vertices);
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &dual_module, Some(&mut visualizer));
    let useless_interface_ptr = DualModuleInterfacePtr::new_empty(); // don't actually use it
    let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    let last_interface_ptr = &primal_module.units.last().unwrap().read_recursive().interface_ptr;
    visualizer
        .snapshot_combined(
            "perfect matching and subgraph".to_string(),
            vec![
                last_interface_ptr,
                &dual_module,
                &perfect_matching,
                &VisualizeSubgraph::new(&subgraph),
            ],
        )
        .unwrap();
}

fn demo_aps2023_large_demo_no_partition() {
    let visualize_filename = "demo_aps2023_large_demo_no_partition.json".to_string();
    let half_weight = 500;
    let noisy_measurements = 10 * 4;
    let d = 5;
    let mut code = PhenomenologicalPlanarCode::new(d, noisy_measurements, 0.03, half_weight);
    let syndrome = code.generate_random_errors(DEMO_APS2023_LARGE_DEMO_RNG_SEED);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer
        .snapshot_combined("syndrome".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
}

fn main() {
    demo_aps2023_decoding_graph_growing();
    demo_aps2023_syndrome_graph_growing();
    demo_aps2023_decoding_graph_static();
    demo_aps2023_example_decoding_graph();
    demo_aps2023_example_syndrome_graph();
    demo_aps2023_example_syndrome_graph_edges();
    demo_aps2023_example_decoding_graph_grow_single();
    demo_aps2023_example_syndrome_graph_grow_single();
    demo_aps2023_example_partition();
    demo_aps2023_large_demo();
    demo_aps2023_large_demo_no_partition();
}
