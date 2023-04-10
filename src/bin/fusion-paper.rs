// cargo run --bin fusion-paper

use fusion_blossom::*;
use example_codes::*;
use util::*;
use visualize::*;
use dual_module_serial::*;
use dual_module::*;
use pointers::*;
use primal_module_serial::*;
use primal_module::*;
use petgraph;

type MinPaths = std::collections::HashMap<(VertexIndex, VertexIndex), Weight>;
fn get_min_paths(code: &impl ExampleCode) -> MinPaths {
    use crate::petgraph::{prelude::*};
    use crate::petgraph::graph::{NodeIndex, UnGraph};
    use crate::petgraph::algo::floyd_warshall;
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
        graph.extend_with_edges(&[pair]);
        weight_map.insert(pair, edge.half_weight * 2);
    }
    let res = floyd_warshall(&graph, |edge| {
        if let Some(weight) = weight_map.get(&(edge.source(), edge.target())) {
            *weight
        } else {
            Weight::MAX
        }
    }).unwrap();
    let mut min_paths = MinPaths::new();
    for vertex_1 in 0..vertices.len() {
        for vertex_2 in vertex_1+1..vertices.len() {
            min_paths.insert((vertex_1 as VertexIndex, vertex_2 as VertexIndex), *res.get(&(nodes[vertex_1], nodes[vertex_2])).unwrap());
            min_paths.insert((vertex_2 as VertexIndex, vertex_1 as VertexIndex), *res.get(&(nodes[vertex_1], nodes[vertex_2])).unwrap());
        }
    }
    min_paths
}

fn get_nearest_virtual(min_paths: &MinPaths, code: &impl ExampleCode, source_vertex_index: VertexIndex) -> Option<VertexIndex> {
    assert!(!code.is_virtual(source_vertex_index as usize));
    let (vertices, _edges) = code.immutable_vertices_edges();
    let mut min_weight = Weight::MAX;
    let mut nearest_virtual = None;
    for (vertex_index, vertex) in vertices.iter().enumerate() {
        if vertex.is_virtual {
            let path_weight = *min_paths.get(&(vertex_index as VertexIndex, source_vertex_index as VertexIndex)).unwrap();
            if path_weight < min_weight {
                nearest_virtual = Some(vertex_index as VertexIndex);
                min_weight = path_weight;
            }
        }
    }
    nearest_virtual
}

fn demo_construct_syndrome_graph(code: &impl ExampleCode, defect_vertices: &[VertexIndex]) -> (SolverInitializer, SyndromePattern, Vec<VisualizePosition>) {
    use std::collections::{BTreeMap};
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
        for j in i+1..defect_vertices.len() {
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
            if !virtual_vertices_map.contains_key(&virtual_vertex) {
                virtual_vertices_map.insert(virtual_vertex, vec![]);
            }
            virtual_vertices_map.get_mut(&virtual_vertex).as_mut().unwrap().push((defect_vertex, *min_paths.get(&(defect_vertex, virtual_vertex)).unwrap()));
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
    let visualize_filename = format!("fusion_paper_decoding_graph_static.json");
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), code.get_positions(), true).unwrap();
    print_visualize_link(visualize_filename.clone());
    // create dual module
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let syndrome = SyndromePattern::new_vertices(vec![]);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer.snapshot_combined(format!("syndrome"), vec![&interface_ptr, &dual_module]).unwrap();
}

const APS2023_EXAMPLE_DEFECT_VERTICES: [VertexIndex; 6] = [ 0, 1, 4, 10, 11, 13 ];

fn fusion_paper_example_decoding_graph() {
    let visualize_filename = format!("fusion_paper_example_decoding_graph.json");
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), code.get_positions(), true).unwrap();
    print_visualize_link(visualize_filename.clone());
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
    visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![&interface_ptr, &dual_module
        , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
}

fn fusion_paper_example_syndrome_graph() {
    let visualize_filename = format!("fusion_paper_example_syndrome_graph.json");
    let half_weight = 500;
    let code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
    // construct the syndrome graph
    let (initializer, syndrome, positions) = demo_construct_syndrome_graph(&code, &APS2023_EXAMPLE_DEFECT_VERTICES);
    // create dual module
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), positions, true).unwrap();
    print_visualize_link(visualize_filename.clone());
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let mut primal_module = PrimalModuleSerialPtr::new_empty(&initializer);
    let interface_ptr = DualModuleInterfacePtr::new_empty();
    primal_module.solve_visualizer(&interface_ptr, &syndrome, &mut dual_module, Some(&mut visualizer));
    let perfect_matching = primal_module.perfect_matching(&interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![&interface_ptr, &dual_module
        , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
}

fn fusion_paper_example_partition() {
    use crate::example_partition::*;
    use crate::dual_module_parallel::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = format!("fusion_paper_example_partition.json");
    let half_weight = 500;
    let mut code = CodeCapacityRotatedCode::new(5, 0.1, half_weight);
    let mut partition = CodeCapacityRotatedCodeVerticalPartitionHalf::new(5, 3);
    let defect_vertices = partition.re_index_defect_vertices(&code, &APS2023_EXAMPLE_DEFECT_VERTICES);
    println!("defect_vertices: {defect_vertices:?}");
    let partition_config = partition.build_apply(&mut code);
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), code.get_positions(), true).unwrap();
    print_visualize_link(visualize_filename.clone());
    let initializer = code.get_initializer();
    let partition_info = partition_config.info();
    // create dual module
    let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(&initializer, &partition_info, DualModuleParallelConfig::default());
    let mut primal_config = PrimalModuleParallelConfig::default();
    primal_config.debug_sequential = true;
    let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
    code.set_defect_vertices(&defect_vertices);
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
    let useless_interface_ptr = DualModuleInterfacePtr::new_empty();  // don't actually use it
    let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    let last_interface_ptr = &primal_module.units.last().unwrap().read_recursive().interface_ptr;
    visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![last_interface_ptr, &dual_module
        , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
}

const FUSION_PAPER_LARGE_DEMO_RNG_SEED: u64 = 671;

fn fusion_paper_large_demo() {
    use crate::example_partition::*;
    use crate::dual_module_parallel::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = format!("fusion_paper_large_demo.json");
    let half_weight = 500;
    let noisy_measurements = 10 * 4;
    let d = 5;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.03, half_weight);
    let random_syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements
        , 4, true, usize::MAX);
    let defect_vertices = partition.re_index_defect_vertices(&code, &random_syndrome.defect_vertices);
    let partition_config = partition.build_apply(&mut code);
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), code.get_positions(), true).unwrap();
    print_visualize_link(visualize_filename.clone());
    let initializer = code.get_initializer();
    let partition_info = partition_config.info();
    // create dual module
    let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(&initializer, &partition_info, DualModuleParallelConfig::default());
    let mut primal_config = PrimalModuleParallelConfig::default();
    primal_config.debug_sequential = true;
    let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
    code.set_defect_vertices(&defect_vertices);
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
    let useless_interface_ptr = DualModuleInterfacePtr::new_empty();  // don't actually use it
    let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    let last_interface_ptr = &primal_module.units.last().unwrap().read_recursive().interface_ptr;
    visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![last_interface_ptr, &dual_module
        , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
}

fn fusion_paper_large_demo_no_partition() {
    let visualize_filename = format!("fusion_paper_large_demo_no_partition.json");
    let half_weight = 500;
    let noisy_measurements = 10 * 4;
    let d = 5;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.03, half_weight);
    let syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), code.get_positions(), true).unwrap();
    print_visualize_link(visualize_filename.clone());
    let initializer = code.get_initializer();
    let mut dual_module = DualModuleSerial::new_empty(&initializer);
    let interface_ptr = DualModuleInterfacePtr::new_load(&syndrome, &mut dual_module);
    visualizer.snapshot_combined(format!("syndrome"), vec![&interface_ptr, &dual_module]).unwrap();
}

fn fusion_paper_example_partition_16() {
    use crate::example_partition::*;
    use crate::dual_module_parallel::*;
    use crate::primal_module_parallel::*;
    let visualize_filename = format!("fusion_paper_example_partition_16.json");
    let half_weight = 500;
    let noisy_measurements = 4 * 16 - 1;
    let d = 9;
    let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.02, half_weight);
    let random_syndrome = code.generate_random_errors(FUSION_PAPER_LARGE_DEMO_RNG_SEED);
    let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(d, noisy_measurements
        , 16, true, usize::MAX);
    let defect_vertices = partition.re_index_defect_vertices(&code, &random_syndrome.defect_vertices);
    let partition_config = partition.build_apply(&mut code);
    let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str()), code.get_positions(), true).unwrap();
    print_visualize_link(visualize_filename.clone());
    let initializer = code.get_initializer();
    let partition_info = partition_config.info();
    // create dual module
    let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(&initializer, &partition_info, DualModuleParallelConfig::default());
    let mut primal_config = PrimalModuleParallelConfig::default();
    primal_config.debug_sequential = true;
    let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
    code.set_defect_vertices(&defect_vertices);
    primal_module.parallel_solve_visualizer(&code.get_syndrome(), &mut dual_module, Some(&mut visualizer));
    let useless_interface_ptr = DualModuleInterfacePtr::new_empty();  // don't actually use it
    let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
    let mut subgraph_builder = SubGraphBuilder::new(&initializer);
    subgraph_builder.load_perfect_matching(&perfect_matching);
    let subgraph = subgraph_builder.get_subgraph();
    let last_interface_ptr = &primal_module.units.last().unwrap().read_recursive().interface_ptr;
    visualizer.snapshot_combined("perfect matching and subgraph".to_string(), vec![last_interface_ptr, &dual_module
        , &perfect_matching, &VisualizeSubgraph::new(&subgraph)]).unwrap();
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

fn main() {
    fusion_paper_decoding_graph_static();
    fusion_paper_example_decoding_graph();
    fusion_paper_example_syndrome_graph();
    fusion_paper_example_partition();
    fusion_paper_large_demo();
    fusion_paper_large_demo_no_partition();
    fusion_paper_example_partition_16();
    fusion_paper_print_partition_configs();
}
