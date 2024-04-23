// cargo run --bin fusion-paper

use dual_module::*;
use dual_module_serial::*;
use example_codes::*;
use fusion_blossom::*;
use pointers::*;
use primal_module::*;
use primal_module_serial::*;
use util::*;
use visualize::*;

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

fn fusion_paper_decoding_graph_static() {
    let visualize_filename = "fusion_paper_decoding_graph_static.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
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

const APS2023_EXAMPLE_DEFECT_VERTICES: [VertexIndex; 6] = [0, 1, 4, 10, 11, 13];

fn fusion_paper_example_decoding_graph() {
    let visualize_filename = "fusion_paper_example_decoding_graph.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
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

fn fusion_paper_example_syndrome_graph() {
    let visualize_filename = "fusion_paper_example_syndrome_graph.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
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

fn fusion_paper_example_partition() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = "fusion_paper_example_partition.json".to_string();
    let half_weight = 500;
    let mut code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
    let mut partition = CodeCapacityRotatedCodeVerticalPartitionHalf::new(5, 3);
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

const FUSION_PAPER_LARGE_DEMO_RNG_SEED: u64 = 671;

fn fusion_paper_large_demo() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = "fusion_paper_large_demo.json".to_string();
    let half_weight = 500;
    let noisy_measurements = 10 * 4;
    let d = 5;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.03, half_weight);
    let random_syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 4, true, usize::MAX);
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

fn fusion_paper_large_demo_no_partition() {
    let visualize_filename = "fusion_paper_large_demo_no_partition.json".to_string();
    let half_weight = 500;
    let noisy_measurements = 10 * 4;
    let d = 5;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.03, half_weight);
    let syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
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

fn fusion_paper_example_partition_16() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = "fusion_paper_example_partition_16.json".to_string();
    let half_weight = 500;
    let noisy_measurements = 4 * 16 - 1;
    let d = 9;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.02, half_weight);
    let random_syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 16, true, usize::MAX);
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

fn fusion_paper_print_partition_configs() {
    use crate::example_partition::*;
    let noisy_measurements = 4 * 16 - 1;
    let d = 9;
    let code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.005, 500);
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 16, true, usize::MAX);
    println!("\nmaximum_tree_leaf_size = inf: {:?}\n", partition.build_partition(&code));
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 16, true, 1);
    println!("\nmaximum_tree_leaf_size = 1: {:?}\n", partition.build_partition(&code));
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 16, true, 4);
    println!("\nmaximum_tree_leaf_size = 4: {:?}\n", partition.build_partition(&code));
}

fn fusion_paper_example_partition_8() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = "fusion_paper_example_partition_8.json".to_string();
    let half_weight = 500;
    let noisy_measurements = 10 * 8 - 1;
    let d = 5;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.02, half_weight);
    let random_syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 8, true, usize::MAX);
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

#[cfg(feature = "qecp_integrate")]
fn fusion_paper_example_partition_8_circuit_level() {
    use crate::dual_module_parallel::*;
    use crate::example_partition::*;
    use crate::primal_module_parallel::*;
    use crate::visualize::*;
    use clap::Parser;
    use serde_json::json;
    let syndromes_filename = format!(
        "{}fusion_paper_example_partition_8_circuit_level.syndromes",
        visualize_data_folder()
    );
    let visualize_filename = "fusion_paper_example_partition_8_circuit_level.json".to_string();
    let noisy_measurements = 10 * 8 - 1;
    let d = 5;
    let benchmark_parameters = qecp::cli::BenchmarkParameters::parse_from([
        "qecp",
        format!("[{d}]").as_str(),
        format!("[{noisy_measurements}]").as_str(),
        "[0.008]",
        "--code-type",
        "rotated-planar-code",
        "--noise-model",
        "stim-noise-model",
        "--decoder",
        "fusion",
        "--decoder-config",
        r#"{"only_stab_z":true,"use_combined_probability":false,"skip_decoding":true,"max_half_weight":500}"#,
        "--debug-print",
        "fusion-blossom-syndrome-file",
        "--fusion-blossom-syndrome-export-filename",
        syndromes_filename.as_str(),
        "--use-brief-edge",
        "-m10",
    ]);
    benchmark_parameters.run().unwrap();
    let mut code = ErrorPatternReader::new(json!({ "filename": syndromes_filename }));
    let random_syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut partition: PhenomenologicalRotatedCodeTimePartition =
        PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements, 8, true, usize::MAX);
    let defect_vertices = partition.re_index_defect_vertices(&code, &random_syndrome.defect_vertices);
    let partition_config = partition.build_apply(&mut code);
    let mut positions = code.get_positions();
    // modify the positions
    let ratio = 0.5;
    for position in positions.iter_mut() {
        let (i, j) = (position.i, position.j);
        position.i = -(i + j) * ratio;
        position.j = (j - i) * ratio;
        position.t *= ratio * (2f64).sqrt();
    }
    // construct visualizer
    let mut visualizer =
        Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
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
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
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

fn fusion_paper_example_covers() {
    let visualize_filename = "fusion_paper_example_covers.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(19, 0.1, half_weight);
    let defect_vertices = vec![42, 75, 102, 6, 88];
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    let initializer = code.get_initializer();
    // create dual module
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(defect_vertices);
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

const OVERLAY_EXAMPLE_DEFECT_VERTICES: [VertexIndex; 13] = [104, 37, 27, 40, 63, 85, 118, 161, 181, 179, 146, 101, 57];
const OVERLAY_D: VertexNum = 21;

fn fusion_paper_overlay_decoding_graph() {
    let visualize_filename = "fusion_paper_overlay_decoding_graph.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(OVERLAY_D, 0.1, half_weight);
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
    let syndrome = SyndromePattern::new_vertices(OVERLAY_EXAMPLE_DEFECT_VERTICES.into());
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    let dual_node_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
    visualizer
        .snapshot_combined("initial".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    for _ in 0..8 {
        dual_module.grow_dual_node(&dual_node_ptr, half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }
}

fn fusion_paper_overlay_syndrome_graph() {
    let visualize_filename = "fusion_paper_overlay_syndrome_graph.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(OVERLAY_D, 0.1, half_weight);
    // construct the syndrome graph
    let (initializer, syndrome, positions) = demo_construct_syndrome_graph(&code, &OVERLAY_EXAMPLE_DEFECT_VERTICES);
    // create dual module
    let mut visualizer =
        Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
    print_visualize_link(visualize_filename);
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    let dual_node_ptr = interface_ptr.read_recursive().nodes[0].clone().unwrap();
    visualizer
        .snapshot_combined("initial".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    for _ in 0..8 {
        dual_module.grow_dual_node(&dual_node_ptr, half_weight);
        visualizer
            .snapshot_combined("grow".to_string(), vec![&interface_ptr, &dual_module])
            .unwrap();
    }
}

fn fusion_paper_pseudo_cover_island() {
    let visualize_filename = "fusion_paper_pseudo_cover_island.json".to_string();
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(11, 0.1, half_weight);
    let mut visualizer = Visualizer::new(
        Some(visualize_data_folder() + visualize_filename.as_str()),
        code.get_positions(),
        true,
    )
    .unwrap();
    print_visualize_link(visualize_filename);
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new(
        vec![28, 57, 13],
        vec![48, 38, 49, 37, 61, 73, 72, 62, 51, 50, 82, 92, 102, 112, 111],
    );
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    let node_ptr_vec = (0..3)
        .map(|i| interface_ptr.read_recursive().nodes[i].clone().unwrap())
        .collect::<Vec<_>>();
    visualizer
        .snapshot_combined("initial".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    for dual_node_ptr in node_ptr_vec.iter() {
        dual_module.grow_dual_node(dual_node_ptr, 2 * half_weight);
    }
    for _ in 0..2 {
        dual_module.prepare_dual_node_growth_single(&node_ptr_vec[2], true);
    }
    for _ in 0..2 {
        dual_module.prepare_dual_node_growth_single(&node_ptr_vec[1], true);
    }
    for _ in 0..10 {
        dual_module.prepare_dual_node_growth_single(&node_ptr_vec[0], true);
    }
    for _ in 0..10 {
        dual_module.prepare_dual_node_growth_single(&node_ptr_vec[2], true);
    }
    for _ in 0..10 {
        dual_module.prepare_dual_node_growth_single(&node_ptr_vec[1], true);
    }
    visualizer
        .snapshot_combined("constructed initial".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    dual_module.prepare_dual_node_growth(&node_ptr_vec[0], false);
    visualizer
        .snapshot_combined("intrude".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
    dual_module.prepare_dual_node_growth(&node_ptr_vec[1], true);
    visualizer
        .snapshot_combined("extrude".to_string(), vec![&interface_ptr, &dual_module])
        .unwrap();
}

fn main() {
    fusion_paper_decoding_graph_static();
    fusion_paper_example_decoding_graph();
    fusion_paper_example_syndrome_graph();
    fusion_paper_example_partition();
    fusion_paper_large_demo();
    fusion_paper_large_demo_no_partition();
    fusion_paper_example_partition_16();
    fusion_paper_print_partition_configs();
    fusion_paper_example_partition_8();
    fusion_paper_example_covers();
    fusion_paper_overlay_decoding_graph();
    fusion_paper_overlay_syndrome_graph();
    fusion_paper_pseudo_cover_island();
    #[cfg(feature = "qecp_integrate")]
    fusion_paper_example_partition_8_circuit_level();
}
