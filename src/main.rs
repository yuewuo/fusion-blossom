extern crate clap;
extern crate pbr;

use fusion_blossom::example::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use fusion_blossom::dual_module_serial;
use fusion_blossom::primal_module_serial;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::complete_graph;
use fusion_blossom::blossom_v;
use pbr::ProgressBar;


fn create_clap_parser<'a>(color_choice: clap::ColorChoice) -> clap::Command<'a> {
    clap::Command::new("Fusion Blossom")
        .version(env!("CARGO_PKG_VERSION"))
        .author(clap::crate_authors!(", "))
        .about("Fusion Blossom Algorithm for fast Quantum Error Correction")
        .color(color_choice)
        .propagate_version(true)
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(clap::Command::new("test")
            .about("testing features")
            .subcommand_required(true)
            .arg_required_else_help(true)
            .subcommand(clap::Command::new("serial").about("test the correctness of the serial implementation")
                .arg(clap::Arg::new("enable_visualizer").long("enable_visualizer").help("disable logging to the default visualizer file")))
        )
}

pub fn main() {
    
    let matches = create_clap_parser(clap::ColorChoice::Auto).get_matches();

    match matches.subcommand() {
        Some(("test", matches)) => {
            match matches.subcommand() {
                Some(("serial", matches)) => {
                    if cfg!(not(feature = "blossom_v")) {
                        panic!("need blossom V library, see README.md")
                    }
                    let enable_visualizer = matches.is_present("enable_visualizer");
                    let mut codes = Vec::<(String, Box<dyn ExampleCode>)>::new();
                    let total_rounds = 10000;
                    let max_half_weight: Weight = 500;
                    // for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                    //     for d in [3, 7, 11, 15, 19] {
                    //         codes.push((format!("repetition {d} {p}"), Box::new(CodeCapacityRepetitionCode::new(d, p, max_half_weight))));
                    //     }
                    // }
                    // for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                    for p in [0.03, 0.1, 0.3, 0.499] {
                        // for d in [3, 7, 11, 15, 19] {
                        for d in [19] {
                            codes.push((format!("planar {d} {p}"), Box::new(CodeCapacityPlanarCode::new(d, p, max_half_weight))));
                        }
                    }
                    // for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                    //     for d in [3, 7, 11, 15, 19] {
                    //         codes.push((format!("phenomenological {d} {p}"), Box::new(PhenomenologicalPlanarCode::new(d, d, p, max_half_weight))));
                    //     }
                    // }
                    // for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                    //     for d in [3, 7, 11, 15, 19] {
                    //         codes.push((format!("circuit-level {d} {p}"), Box::new(CircuitLevelPlanarCode::new(d, d, p, max_half_weight))));
                    //     }
                    // }
                    if enable_visualizer {  // print visualizer file path only once
                        print_visualize_link(&static_visualize_data_filename());
                    }
                    let codes_len = codes.len();
                    for (code_idx, (code_name, code)) in codes.iter_mut().enumerate() {
                        let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                        pb.message(format!("{code_name} [{code_idx}/{codes_len}] ").as_str());
                        // create dual module
                        let (vertex_num, weighted_edges, virtual_vertices) = code.get_initializer();
                        let mut dual_module = dual_module_serial::DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
                        // create primal module
                        let mut primal_module = primal_module_serial::PrimalModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
                        primal_module.debug_resolve_only_one = true;  // to enable debug mode
                        // create blossom V decoder
                        let mut complete_graph = complete_graph::CompleteGraph::new(vertex_num, &weighted_edges);
                        for round in 0..total_rounds {
                            dual_module.clear();
                            primal_module.clear();
                            pb.set(round);
                            let syndrome_vertices = code.generate_random_errors(round);
                            let mut visualizer = None;
                            if enable_visualizer {
                                let mut new_visualizer = Visualizer::new(Some(visualize_data_folder() + static_visualize_data_filename().as_str())).unwrap();
                                new_visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
                                visualizer = Some(new_visualizer);
                            }
                            // try to work on a simple syndrome
                            code.set_syndrome(&syndrome_vertices);
                            // println!("syndrome_vertices: {syndrome_vertices:?}");
                            let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
                            // interface.debug_print_actions = true;
                            primal_module.load(&interface);  // load syndrome and connect to the dual module interface
                            visualizer.as_mut().map(|v| v.snapshot_combined(format!("syndrome"), vec![&interface, &dual_module, &primal_module]).unwrap());
                            // grow until end
                            let mut group_max_update_length = dual_module.compute_maximum_update_length();
                            while !group_max_update_length.is_empty() {
                                // println!("group_max_update_length: {:?}", group_max_update_length);
                                if let Some(length) = group_max_update_length.get_none_zero_growth() {
                                    interface.grow(length, &mut dual_module);
                                    visualizer.as_mut().map(|v| v.snapshot_combined(format!("grow {length}"), vec![&interface, &dual_module, &primal_module]).unwrap());
                                } else {
                                    let first_conflict = format!("{:?}", group_max_update_length.get_conflicts().peek().unwrap());
                                    primal_module.resolve(group_max_update_length, &mut interface, &mut dual_module);
                                    visualizer.as_mut().map(|v| v.snapshot_combined(format!("resolve {first_conflict}"), vec![&interface, &dual_module, &primal_module]).unwrap());
                                }
                                group_max_update_length = dual_module.compute_maximum_update_length();
                            }
                            // use blossom V to compute ground truth
                            let mut mapping_to_syndrome_nodes: Vec<usize> = (0..vertex_num).map(|_| usize::MAX).collect();
                            for (i, &syndrome_node) in syndrome_vertices.iter().enumerate() {
                                mapping_to_syndrome_nodes[syndrome_node] = i;
                            }
                            let legacy_vertex_num = syndrome_vertices.len() * 2;
                            let mut legacy_weighted_edges = Vec::<(usize, usize, u32)>::new();
                            let mut boundaries = Vec::<Option<(usize, Weight)>>::new();
                            for i in 0..syndrome_vertices.len() {
                                let complete_graph_edges = complete_graph.all_edges(syndrome_vertices[i]);
                                let mut boundary: Option<(usize, Weight)> = None;
                                for (&peer, &(_, weight)) in complete_graph_edges.iter() {
                                    if code.is_virtual(peer) {
                                        if boundary.is_none() || weight < boundary.as_ref().unwrap().1 {
                                            boundary = Some((peer, weight));
                                        }
                                    }
                                }
                                match boundary {
                                    Some((_, weight)) => {
                                        // connect this real vertex to it's corresponding virtual vertex
                                        legacy_weighted_edges.push((i, i + syndrome_vertices.len(), weight as u32));
                                    }, None => { }
                                }
                                boundaries.push(boundary);  // save for later resolve legacy matchings
                                for (&peer, &(_, weight)) in complete_graph_edges.iter() {
                                    if code.is_syndrome(peer) {
                                        let j = mapping_to_syndrome_nodes[peer];
                                        if i < j {  // remove duplicated edges
                                            legacy_weighted_edges.push((i, j, weight as u32));
                                            // println!{"edge {} {} {} ", i, j, weight};
                                        }
                                    }
                                }
                                for j in (i+1)..syndrome_vertices.len() {
                                    // virtual boundaries are always fully connected with weight 0
                                    legacy_weighted_edges.push((i + syndrome_vertices.len(), j + syndrome_vertices.len(), 0));
                                }
                            }
                            let blossom_matchings = blossom_v::safe_minimum_weight_perfect_matching(legacy_vertex_num, &legacy_weighted_edges);
                            let mut blossom_mwpm_result = Vec::new();
                            for i in 0..syndrome_vertices.len() {
                                let j = blossom_matchings[i];
                                if j < syndrome_vertices.len() {  // match to a real node
                                    blossom_mwpm_result.push(syndrome_vertices[j]);
                                } else {
                                    assert_eq!(j, i + syndrome_vertices.len(), "if not matched to another real node, it must match to it's corresponding virtual node");
                                    blossom_mwpm_result.push(boundaries[i].as_ref().expect("boundary must exist if match to virtual node").0);
                                }
                            }
                            let mut blossom_total_weight = 0;
                            for i in 0..syndrome_vertices.len() {
                                let a = syndrome_vertices[i];
                                let b = blossom_mwpm_result[i];
                                if !code.is_syndrome(b) || a < b {
                                    let (_path, weight) = complete_graph.get_path(a, b);
                                    blossom_total_weight += weight;
                                }
                            }
                            // compare with ground truth from the blossom V algorithm
                            assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
                        }
                        pb.finish();
                        println!("");
                    }
                },
                _ => unreachable!()
            }
        },
        _ => unreachable!()
    }

}
