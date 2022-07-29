extern crate clap;
extern crate pbr;

use fusion_blossom::example::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use fusion_blossom::dual_module_serial;
use fusion_blossom::primal_module_serial;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
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
                .arg(clap::Arg::new("enable_visualizer").long("enable_visualizer").help("logging to the default visualizer file"))
                .arg(clap::Arg::new("disable_blossom").long("disable_blossom").help("disable assertion that compares with ground truth from blossom V library"))
            )
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
                    let disable_blossom = matches.is_present("disable_blossom");
                    let mut codes = Vec::<(String, Box<dyn ExampleCode>)>::new();
                    let total_rounds = 1000;
                    let max_half_weight: Weight = 500;
                    for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11, 15, 19] {
                            codes.push((format!("repetition {d} {p}"), Box::new(CodeCapacityRepetitionCode::new(d, p, max_half_weight))));
                        }
                    }
                    for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11, 15, 19] {
                            codes.push((format!("planar {d} {p}"), Box::new(CodeCapacityPlanarCode::new(d, p, max_half_weight))));
                        }
                    }
                    for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // test erasures
                        for d in [3, 7, 11, 15, 19] {
                            let mut code = CodeCapacityPlanarCode::new(d, p, max_half_weight);
                            code.set_erasure_probability(p);
                            codes.push((format!("mixed erasure planar {d} {p}"), Box::new(code)));
                        }
                    }
                    for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11] {
                            codes.push((format!("phenomenological {d} {p}"), Box::new(PhenomenologicalPlanarCode::new(d, d, p, max_half_weight))));
                        }
                    }
                    for p in [0.0001, 0.0003, 0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [3, 7, 11] {
                            codes.push((format!("circuit-level {d} {p}"), Box::new(CircuitLevelPlanarCode::new(d, d, p, max_half_weight))));
                        }
                    }
                    if enable_visualizer {  // print visualizer file path only once
                        print_visualize_link(&static_visualize_data_filename());
                    }
                    let codes_len = codes.len();
                    for (code_idx, (code_name, code)) in codes.iter_mut().enumerate() {
                        let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                        pb.message(format!("{code_name} [{code_idx}/{codes_len}] ").as_str());
                        // create dual module
                        let (vertex_num, mut weighted_edges, virtual_vertices) = code.get_initializer();
                        let mut dual_module = dual_module_serial::DualModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
                        // create primal module
                        let mut primal_module = primal_module_serial::PrimalModuleSerial::new(vertex_num, &weighted_edges, &virtual_vertices);
                        primal_module.debug_resolve_only_one = true;  // to enable debug mode
                        let mut subgraph_builder = SubGraphBuilder::new(vertex_num, &weighted_edges, &virtual_vertices);
                        for round in 0..total_rounds {
                            dual_module.clear();
                            primal_module.clear();
                            pb.set(round);
                            let (syndrome_vertices, erasures) = code.generate_random_errors(round);
                            let mut visualizer = None;
                            if enable_visualizer {
                                let mut new_visualizer = Visualizer::new(Some(visualize_data_folder() + static_visualize_data_filename().as_str())).unwrap();
                                new_visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
                                visualizer = Some(new_visualizer);
                            }
                            // try to work on a simple syndrome
                            code.set_syndrome(&syndrome_vertices);
                            // println!("syndrome_vertices: {syndrome_vertices:?}");
                            // println!("erasures: {erasures:?}");
                            let mut interface = DualModuleInterface::new(&code.get_syndrome(), &mut dual_module);
                            dual_module.load_erasures(&erasures);
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
                            if !disable_blossom {
                                // prepare modified weighted edges
                                let mut erasure_modifier = ErasureModifier::new();
                                for edge_index in erasures.iter() {
                                    let (vertex_idx_1, vertex_idx_2, original_weight) = &weighted_edges[*edge_index];
                                    erasure_modifier.push_modified_edge(*edge_index, *original_weight);
                                    weighted_edges[*edge_index] = (*vertex_idx_1, *vertex_idx_2, 0);
                                }
                                // use blossom V to compute ground truth
                                let blossom_mwpm_result = fusion_blossom::blossom_v_mwpm(vertex_num, &weighted_edges, &virtual_vertices, &syndrome_vertices);
                                let blossom_details = fusion_blossom::detailed_matching(vertex_num, &weighted_edges, &syndrome_vertices, &blossom_mwpm_result);
                                let mut blossom_total_weight = 0;
                                for detail in blossom_details.iter() {
                                    blossom_total_weight += detail.weight;
                                }
                                // if blossom_total_weight > 0 { println!("w {} {}", interface.sum_dual_variables, blossom_total_weight); }
                                assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
                                // also construct the perfect matching from fusion blossom to compare them
                                let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
                                let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
                                let fusion_details = fusion_blossom::detailed_matching(vertex_num, &weighted_edges, &syndrome_vertices, &fusion_mwpm_result);
                                let mut fusion_total_weight = 0;
                                for detail in fusion_details.iter() {
                                    fusion_total_weight += detail.weight;
                                }
                                // recover those weighted_edges
                                while erasure_modifier.has_modified_edges() {
                                    let (edge_index, original_weight) = erasure_modifier.pop_modified_edge();
                                    let (vertex_idx_1, vertex_idx_2, _) = &weighted_edges[edge_index];
                                    weighted_edges[edge_index] = (*vertex_idx_1, *vertex_idx_2, original_weight);
                                }
                                // compare with ground truth from the blossom V algorithm
                                assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
                                // also test subgraph builder
                                subgraph_builder.clear();
                                subgraph_builder.load_erasures(&erasures);
                                subgraph_builder.load_perfect_matching(&fusion_mwpm);
                                // println!("blossom_total_weight: {blossom_total_weight} = {} = {fusion_total_weight}", subgraph_builder.total_weight());
                                assert_eq!(subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
                            }
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
