extern crate clap;
extern crate pbr;

use fusion_blossom::example::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use fusion_blossom::dual_module_serial;
use fusion_blossom::primal_module_serial;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::dual_module_parallel;
use fusion_blossom::primal_module_parallel;
use pbr::ProgressBar;

use dual_module_serial::DualModuleSerial;
use primal_module_serial::PrimalModuleSerial;
use dual_module_parallel::DualModuleParallel;
use std::sync::Arc;
use clap::{ValueEnum, Parser, Subcommand};
use serde::Serialize;


#[derive(Parser, Clone)]
#[clap(author = clap::crate_authors!(", "), version = env!("CARGO_PKG_VERSION")
    , about = "Fusion Blossom Algorithm for fast Quantum Error Correction Decoding", long_about = None)]
#[clap(propagate_version = true)]
pub struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Clone)]
enum Commands {
    /// benchmark the speed (and also correctness if enabled)
    Benchmark {
        /// code distance
        #[clap(value_parser)]
        d: usize,
        /// physical error rate: the probability of each edge to 
        #[clap(value_parser)]
        p: f64,
        /// rounds of noisy measurement, valid only when multiple rounds
        #[clap(short = 'e', long, default_value_t = 0.)]
        pe: f64,
        /// rounds of noisy measurement, valid only when multiple rounds
        #[clap(short = 'n', long, default_value_t = 0)]
        noisy_measurements: usize,
        /// maximum half weight of edges
        #[clap(long, default_value_t = 500)]
        max_half_weight: Weight,
        /// example code type
        #[clap(short = 'c', long, arg_enum, default_value_t = ExampleCodeType::CodeCapacityPlanarCode)]
        code_type: ExampleCodeType,
        /// logging to the default visualizer file at visualize/data/static.json
        #[clap(long, action)]
        enable_visualizer: bool,
        /// the method to verify the correctness of the decoding result
        #[clap(long, arg_enum, default_value_t = Verifier::None)]
        verifier: Verifier,
        /// the number of iterations to run
        #[clap(short = 't', long, default_value_t = 1000)]
        total_rounds: usize,
        /// select the combination of primal and dual module
        #[clap(short = 'p', long, arg_enum, default_value_t = PrimalDualType::Serial)]
        primal_dual_type: PrimalDualType,
        /// partition strategy
        #[clap(long, arg_enum, default_value_t = PartitionStrategy::None)]
        partition_strategy: PartitionStrategy,
        /// message on the progress bar
        #[clap(long, default_value_t = format!(""))]
        pb_message: String,
    },
    /// built-in tests
    Test {
        #[clap(subcommand)]
        command: TestCommands,
    }
}

#[derive(Subcommand, Clone)]
enum TestCommands {
    /// test serial implementation
    Serial,
}

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
            .subcommand(clap::Command::new("parallel_dual").about("test the correctness of the parallel dual module implementation")
                .arg(clap::Arg::new("enable_visualizer").long("enable_visualizer").help("logging to the default visualizer file"))
                .arg(clap::Arg::new("disable_blossom").long("disable_blossom").help("disable assertion that compares with ground truth from blossom V library"))
            )
            .subcommand(clap::Command::new("parallel").about("test the correctness of the parallel dual module implementation")
                .arg(clap::Arg::new("enable_visualizer").long("enable_visualizer").help("logging to the default visualizer file"))
                .arg(clap::Arg::new("disable_blossom").long("disable_blossom").help("disable assertion that compares with ground truth from blossom V library"))
                .arg(clap::Arg::new("debug_sequential").long("debug_sequential").help("sequentially run the primal module to enable more visualization steps"))
            )
        )
        .subcommand(clap::Command::new("benchmark")
            .about("benchmark the speed (and also correctness if enabled)")
            .arg(clap::Arg::new("code_type").long("code_type").help("example code type").takes_value(true).default_value("code-capacity-planar-code")
                .possible_values(ExampleCodeType::value_variants().iter().filter_map(ValueEnum::to_possible_value)))
            .arg(clap::Arg::new("enable_visualizer").long("enable_visualizer").help("logging to the default visualizer file"))
            .arg(clap::Arg::new("verifier").long("verifier").help("verify the correctness of the decoding result").takes_value(true)
                .possible_values(Verifier::value_variants().iter().filter_map(ValueEnum::to_possible_value)))
        )
}

/// note that these code type is only for example, to test and demonstrate the correctness of the algorithm, but not for real QEC simulation;
/// for real simulation, please refer to <https://github.com/yuewuo/QEC-Playground>
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum ExampleCodeType {
    /// quantum repetition code with perfect stabilizer measurement
    CodeCapacityRepetitionCode,
    /// quantum repetition code with phenomenological error model
    PhenomenologicalRepetitionCode,
    /// quantum repetition code with circuit-level noise model
    CircuitLevelRepetitionCode,
    /// planar surface code with perfect stabilizer measurement
    CodeCapacityPlanarCode,
    /// planar surface code with phenomenological error model
    PhenomenologicalPlanarCode,
    /// planar surface code with circuit-level noise model
    CircuitLevelPlanarCode,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum PartitionStrategy {
    /// no partition
    None,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum PrimalDualType {
    /// serial primal and dual
    Serial,
    /// parallel dual and serial primal
    DualParallel,
    /// parallel primal and dual
    Parallel,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum Verifier {
    /// disable verifier
    None,
    /// use blossom V library to verify the correctness of result
    BlossomV,
    /// use the serial version of fusion algorithm to verify the correctness of result
    FusionSerial,
}

impl Cli {
    pub fn run(self) {
        match self.command {
            Commands::Benchmark { d, p, pe, noisy_measurements, max_half_weight, code_type, enable_visualizer, verifier, total_rounds, primal_dual_type
                    , partition_strategy, pb_message } => {
                // check for dependency early
                if matches!(verifier, Verifier::BlossomV) {
                    if cfg!(not(feature = "blossom_v")) {
                        panic!("need blossom V library, see README.md")
                    }
                }
                let mut code: Box<dyn ExampleCode> = code_type.build(d, p, noisy_measurements, max_half_weight);
                if pe != 0. { code.set_erasure_probability(pe); }
                if enable_visualizer {  // print visualizer file path only once
                    print_visualize_link(&static_visualize_data_filename());
                }
                // prepare progress bar display
                let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                pb.message(format!("{pb_message} ").as_str());
                // create initializer and solver
                let (initializer, partition_config) = partition_strategy.build(&mut code, d, noisy_measurements);
                let mut primal_dual_solver = primal_dual_type.build(&initializer, &partition_config);
                let mut result_verifier = verifier.build(&initializer);
                for round in 0..(total_rounds as u64) {
                    primal_dual_solver.clear();
                    pb.set(round);
                    let (syndrome_vertices, erasures) = code.generate_random_errors(round);
                    // create a new visualizer each round
                    let mut visualizer = None;
                    if enable_visualizer {
                        let mut new_visualizer = Visualizer::new(Some(visualize_data_folder() + static_visualize_data_filename().as_str())).unwrap();
                        new_visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
                        visualizer = Some(new_visualizer);
                    }
                    // println!("syndrome_vertices: {syndrome_vertices:?}");
                    // println!("erasures: {erasures:?}");
                    primal_dual_solver.solve_visualizer(&syndrome_vertices, &erasures, visualizer.as_mut());
                    result_verifier.verify(&mut primal_dual_solver, &syndrome_vertices, &erasures);
                }
                pb.finish();
                println!("");
            },
            Commands::Test { command } => {
                match command {
                    TestCommands::Serial => {
                        let mut parameters = vec![];
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-repetition-code")
                                    , format!("--pb-message"), format!("repetition {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--pb-message"), format!("planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // test erasures
                            for d in [3, 7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--pe"), format!("{p}")
                                    , format!("--pb-message"), format!("mixed erasure planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("phenomenological-planar-code")
                                    , format!("--pb-message"), format!("phenomenological {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("circuit-level-planar-code")
                                    , format!("--pb-message"), format!("circuit-level {d} {p}")]);
                            }
                        }
                        let global = vec![format!("--verifier"), format!("blossom-v")];
                        for parameter in parameters.iter() {
                            Cli::parse_from([format!(""), format!("benchmark")].iter().chain(parameter.iter()).chain(global.iter())).run();
                        }
                    },
                }
            },
        }
    }
}

pub fn main() {

    Cli::parse().run();

    if true {
        return
    }

    let matches = create_clap_parser(clap::ColorChoice::Auto).get_matches();

    match matches.subcommand() {
        Some(("test", matches)) => {
            match matches.subcommand() {
                Some(("parallel_dual", matches)) => {
                    if cfg!(not(feature = "blossom_v")) {
                        panic!("need blossom V library, see README.md")
                    }
                    let enable_visualizer = matches.is_present("enable_visualizer");
                    let disable_blossom = matches.is_present("disable_blossom");
                    let mut codes = Vec::<(String, (
                        Box<dyn ExampleCode>,
                        Box<dyn Fn(&SolverInitializer, &mut PartitionConfig)>,
                    ))>::new();
                    let total_rounds = 1000;
                    let max_half_weight: Weight = 500;
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [7, 11, 15, 19] {
                            let mut reordered_vertices = vec![];
                            let split_vertical = (d + 1) / 2;
                            for j in 0..split_vertical {
                                reordered_vertices.push(j);
                            }
                            reordered_vertices.push(d);
                            for j in split_vertical..d {
                                reordered_vertices.push(j);
                            }
                            codes.push((format!("2-partition repetition {d} {p}"), (
                                Box::new((|| {
                                    let mut code = CodeCapacityRepetitionCode::new(d, p, max_half_weight);
                                    code.reorder_vertices(&reordered_vertices);
                                    code
                                })()),
                                Box::new(move |initializer, config| {
                                    config.partitions = vec![
                                        VertexRange::new(0, split_vertical + 1),
                                        VertexRange::new(split_vertical + 2, initializer.vertex_num),
                                    ];
                                    config.fusions = vec![
                                        (0, 1),
                                    ];
                                }),
                            )));
                        }
                    }
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // simple partition into top and bottom
                        for d in [7, 11, 15, 19] {
                            let split_horizontal = (d + 1) / 2;
                            let row_count = d + 1;
                            codes.push((format!("2-partition planar {d} {p}"), (
                                Box::new((|| {
                                    let code = CodeCapacityPlanarCode::new(d, p, max_half_weight);
                                    code
                                })()),
                                Box::new(move |initializer, config| {
                                    config.partitions = vec![
                                        VertexRange::new(0, split_horizontal * row_count),
                                        VertexRange::new((split_horizontal + 1) * row_count, initializer.vertex_num),
                                    ];
                                    config.fusions = vec![
                                        (0, 1),
                                    ];
                                }),
                            )));
                        }
                    }
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // complex partition into 4 blocks
                        for d in [7, 11, 15, 19] {
                            let mut reordered_vertices = vec![];
                            let row_count = d + 1;
                            let split_horizontal = (d + 1) / 2;
                            let split_vertical = (d + 1) / 2;
                            let start_1 = 0;
                            for i in 0..split_horizontal {  // left-top block
                                for j in 0..split_vertical {
                                    reordered_vertices.push(i * row_count + j);
                                }
                                reordered_vertices.push(i * row_count + (row_count-1));
                            }
                            let end_1 = reordered_vertices.len();
                            for i in 0..split_horizontal {  // interface between the left-top block and the right-top block
                                reordered_vertices.push(i * row_count + split_vertical);
                            }
                            let start_2 = reordered_vertices.len();
                            for i in 0..split_horizontal {  // right-top block
                                for j in (split_vertical+1)..(row_count-1) {
                                    reordered_vertices.push(i * row_count + j);
                                }
                            }
                            let end_2 = reordered_vertices.len();
                            {  // the big interface between top and bottom
                                for j in 0..row_count {
                                    reordered_vertices.push(split_horizontal * row_count + j);
                                }
                            }
                            let start_3 = reordered_vertices.len();
                            for i in (split_horizontal+1)..(row_count-1) {  // left-bottom block
                                for j in 0..split_vertical {
                                    reordered_vertices.push(i * row_count + j);
                                }
                                reordered_vertices.push(i * row_count + (row_count-1));
                            }
                            let end_3 = reordered_vertices.len();
                            for i in (split_horizontal+1)..(row_count-1) {  // interface between the left-bottom block and the right-bottom block
                                reordered_vertices.push(i * row_count + split_vertical);
                            }
                            let start_4 = reordered_vertices.len();
                            for i in (split_horizontal+1)..(row_count-1) {  // right-bottom block
                                for j in (split_vertical+1)..(row_count-1) {
                                    reordered_vertices.push(i * row_count + j);
                                }
                            }
                            let end_4 = reordered_vertices.len();
                            codes.push((format!("4-partition planar {d} {p}"), (
                                Box::new((|| {
                                    let mut code = CodeCapacityPlanarCode::new(d, p, max_half_weight);
                                    code.reorder_vertices(&reordered_vertices);
                                    code
                                })()),
                                Box::new(move |_initializer, config| {
                                    config.partitions = vec![
                                        VertexRange::new(start_1, end_1),
                                        VertexRange::new(start_2, end_2),
                                        VertexRange::new(start_3, end_3),
                                        VertexRange::new(start_4, end_4),
                                    ];
                                    config.fusions = vec![
                                        (0, 1),
                                        (2, 3),
                                        (4, 5),
                                    ];
                                }),
                            )));
                        }
                    }
                    if enable_visualizer {  // print visualizer file path only once
                        print_visualize_link(&static_visualize_data_filename());
                    }
                    let codes_len = codes.len();
                    for (code_idx, (code_name, (code, partition_func))) in codes.iter_mut().enumerate() {
                        let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                        pb.message(format!("{code_name} [{code_idx}/{codes_len}] ").as_str());
                        // create dual module
                        let mut initializer = code.get_initializer();
                        let config = dual_module_parallel::DualModuleParallelConfig::default();
                        let mut partition_config = PartitionConfig::default(&initializer);
                        partition_func(&initializer, &mut partition_config);
                        let partition_info = partition_config.into_info(&initializer);
                        let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(&initializer, Arc::clone(&partition_info), config);
                        // create primal module
                        let mut primal_module = PrimalModuleSerial::new(&initializer);
                        primal_module.debug_resolve_only_one = false;  // to enable debug mode
                        let mut subgraph_builder = SubGraphBuilder::new(&initializer);
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
                            // println!("syndrome_vertices: {syndrome_vertices:?}");
                            // println!("erasures: {erasures:?}");
                            dual_module.static_fuse_all();
                            dual_module.load_erasures(&erasures);
                            let mut interface = primal_module.solve_visualizer(&code.get_syndrome(), &mut dual_module, visualizer.as_mut());
                            if !disable_blossom {
                                // prepare modified weighted edges
                                let mut edge_modifier = EdgeWeightModifier::new();
                                for edge_index in erasures.iter() {
                                    let (vertex_idx_1, vertex_idx_2, original_weight) = &initializer.weighted_edges[*edge_index];
                                    edge_modifier.push_modified_edge(*edge_index, *original_weight);
                                    initializer.weighted_edges[*edge_index] = (*vertex_idx_1, *vertex_idx_2, 0);
                                }
                                // use blossom V to compute ground truth
                                let blossom_mwpm_result = fusion_blossom::blossom_v_mwpm(&initializer, &syndrome_vertices);
                                let blossom_details = fusion_blossom::detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
                                let mut blossom_total_weight = 0;
                                for detail in blossom_details.iter() {
                                    blossom_total_weight += detail.weight;
                                }
                                // if blossom_total_weight > 0 { println!("w {} {}", interface.sum_dual_variables, blossom_total_weight); }
                                assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
                                // also construct the perfect matching from fusion blossom to compare them
                                let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
                                let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
                                let fusion_details = fusion_blossom::detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
                                let mut fusion_total_weight = 0;
                                for detail in fusion_details.iter() {
                                    fusion_total_weight += detail.weight;
                                }
                                // recover those weighted_edges
                                while edge_modifier.has_modified_edges() {
                                    let (edge_index, original_weight) = edge_modifier.pop_modified_edge();
                                    let (vertex_idx_1, vertex_idx_2, _) = &initializer.weighted_edges[edge_index];
                                    initializer.weighted_edges[edge_index] = (*vertex_idx_1, *vertex_idx_2, original_weight);
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
                Some(("parallel", matches)) => {
                    if cfg!(not(feature = "blossom_v")) {
                        panic!("need blossom V library, see README.md")
                    }
                    let enable_visualizer = matches.is_present("enable_visualizer");
                    let disable_blossom = matches.is_present("disable_blossom");
                    let debug_sequential = matches.is_present("debug_sequential");
                    let mut codes = Vec::<(String, (
                        Box<dyn ExampleCode>,
                        Box<dyn Fn(&SolverInitializer, &mut PartitionConfig)>,
                    ))>::new();
                    let total_rounds = 1000;
                    let max_half_weight: Weight = 500;
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                        for d in [7, 11, 15, 19] {
                            let mut reordered_vertices = vec![];
                            let split_vertical = (d + 1) / 2;
                            for j in 0..split_vertical {
                                reordered_vertices.push(j);
                            }
                            reordered_vertices.push(d);
                            for j in split_vertical..d {
                                reordered_vertices.push(j);
                            }
                            codes.push((format!("2-partition repetition {d} {p}"), (
                                Box::new((|| {
                                    let mut code = CodeCapacityRepetitionCode::new(d, p, max_half_weight);
                                    code.reorder_vertices(&reordered_vertices);
                                    code
                                })()),
                                Box::new(move |initializer, config| {
                                    config.partitions = vec![
                                        VertexRange::new(0, split_vertical + 1),
                                        VertexRange::new(split_vertical + 2, initializer.vertex_num),
                                    ];
                                    config.fusions = vec![
                                        (0, 1),
                                    ];
                                }),
                            )));
                        }
                    }
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // simple partition into top and bottom
                        for d in [7, 11, 15, 19] {
                            let split_horizontal = (d + 1) / 2;
                            let row_count = d + 1;
                            codes.push((format!("2-partition planar {d} {p}"), (
                                Box::new((|| {
                                    let code = CodeCapacityPlanarCode::new(d, p, max_half_weight);
                                    code
                                })()),
                                Box::new(move |initializer, config| {
                                    config.partitions = vec![
                                        VertexRange::new(0, split_horizontal * row_count),
                                        VertexRange::new((split_horizontal + 1) * row_count, initializer.vertex_num),
                                    ];
                                    config.fusions = vec![
                                        (0, 1),
                                    ];
                                }),
                            )));
                        }
                    }
                    for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // complex partition into 4 blocks
                        for d in [7, 11, 15, 19] {
                            let mut reordered_vertices = vec![];
                            let row_count = d + 1;
                            let split_horizontal = (d + 1) / 2;
                            let split_vertical = (d + 1) / 2;
                            let start_1 = 0;
                            for i in 0..split_horizontal {  // left-top block
                                for j in 0..split_vertical {
                                    reordered_vertices.push(i * row_count + j);
                                }
                                reordered_vertices.push(i * row_count + (row_count-1));
                            }
                            let end_1 = reordered_vertices.len();
                            for i in 0..split_horizontal {  // interface between the left-top block and the right-top block
                                reordered_vertices.push(i * row_count + split_vertical);
                            }
                            let start_2 = reordered_vertices.len();
                            for i in 0..split_horizontal {  // right-top block
                                for j in (split_vertical+1)..(row_count-1) {
                                    reordered_vertices.push(i * row_count + j);
                                }
                            }
                            let end_2 = reordered_vertices.len();
                            {  // the big interface between top and bottom
                                for j in 0..row_count {
                                    reordered_vertices.push(split_horizontal * row_count + j);
                                }
                            }
                            let start_3 = reordered_vertices.len();
                            for i in (split_horizontal+1)..(row_count-1) {  // left-bottom block
                                for j in 0..split_vertical {
                                    reordered_vertices.push(i * row_count + j);
                                }
                                reordered_vertices.push(i * row_count + (row_count-1));
                            }
                            let end_3 = reordered_vertices.len();
                            for i in (split_horizontal+1)..(row_count-1) {  // interface between the left-bottom block and the right-bottom block
                                reordered_vertices.push(i * row_count + split_vertical);
                            }
                            let start_4 = reordered_vertices.len();
                            for i in (split_horizontal+1)..(row_count-1) {  // right-bottom block
                                for j in (split_vertical+1)..(row_count-1) {
                                    reordered_vertices.push(i * row_count + j);
                                }
                            }
                            let end_4 = reordered_vertices.len();
                            codes.push((format!("4-partition planar {d} {p}"), (
                                Box::new((|| {
                                    let mut code = CodeCapacityPlanarCode::new(d, p, max_half_weight);
                                    code.reorder_vertices(&reordered_vertices);
                                    code
                                })()),
                                Box::new(move |_initializer, config| {
                                    config.partitions = vec![
                                        VertexRange::new(start_1, end_1),
                                        VertexRange::new(start_2, end_2),
                                        VertexRange::new(start_3, end_3),
                                        VertexRange::new(start_4, end_4),
                                    ];
                                    config.fusions = vec![
                                        (0, 1),
                                        (2, 3),
                                        (4, 5),
                                    ];
                                }),
                            )));
                        }
                    }
                    if enable_visualizer {  // print visualizer file path only once
                        print_visualize_link(&static_visualize_data_filename());
                    }
                    let codes_len = codes.len();
                    for (code_idx, (code_name, (code, partition_func))) in codes.iter_mut().enumerate() {
                        let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                        pb.message(format!("{code_name} [{code_idx}/{codes_len}] ").as_str());
                        let mut initializer = code.get_initializer();
                        let mut partition_config = PartitionConfig::default(&initializer);
                        partition_func(&initializer, &mut partition_config);
                        let partition_info = partition_config.into_info(&initializer);
                        // create dual module
                        let dual_config = dual_module_parallel::DualModuleParallelConfig::default();
                        let mut dual_module = DualModuleParallel::<DualModuleSerial>::new_config(&initializer, Arc::clone(&partition_info), dual_config);
                        // create primal module
                        let mut primal_config = primal_module_parallel::PrimalModuleParallelConfig::default();
                        primal_config.debug_sequential = debug_sequential;
                        let mut primal_module = primal_module_parallel::PrimalModuleParallel::new_config(&initializer, Arc::clone(&partition_info), primal_config);
                        let mut subgraph_builder = SubGraphBuilder::new(&initializer);
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
                            // println!("syndrome_vertices: {syndrome_vertices:?}");
                            // println!("erasures: {erasures:?}");
                            dual_module.load_erasures(&erasures);
                            let mut interface = primal_module.parallel_solve_visualizer(&code.get_syndrome(), &mut dual_module, visualizer.as_mut());
                            if !disable_blossom {
                                // prepare modified weighted edges
                                let mut edge_modifier = EdgeWeightModifier::new();
                                for edge_index in erasures.iter() {
                                    let (vertex_idx_1, vertex_idx_2, original_weight) = &initializer.weighted_edges[*edge_index];
                                    edge_modifier.push_modified_edge(*edge_index, *original_weight);
                                    initializer.weighted_edges[*edge_index] = (*vertex_idx_1, *vertex_idx_2, 0);
                                }
                                // use blossom V to compute ground truth
                                let blossom_mwpm_result = fusion_blossom::blossom_v_mwpm(&initializer, &syndrome_vertices);
                                let blossom_details = fusion_blossom::detailed_matching(&initializer, &syndrome_vertices, &blossom_mwpm_result);
                                let mut blossom_total_weight = 0;
                                for detail in blossom_details.iter() {
                                    blossom_total_weight += detail.weight;
                                }
                                // if blossom_total_weight > 0 { println!("w {} {}", interface.sum_dual_variables, blossom_total_weight); }
                                assert_eq!(interface.sum_dual_variables, blossom_total_weight, "unexpected final dual variable sum");
                                // also construct the perfect matching from fusion blossom to compare them
                                let fusion_mwpm = primal_module.perfect_matching(&mut interface, &mut dual_module);
                                let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
                                let fusion_details = fusion_blossom::detailed_matching(&initializer, &syndrome_vertices, &fusion_mwpm_result);
                                let mut fusion_total_weight = 0;
                                for detail in fusion_details.iter() {
                                    fusion_total_weight += detail.weight;
                                }
                                // recover those weighted_edges
                                while edge_modifier.has_modified_edges() {
                                    let (edge_index, original_weight) = edge_modifier.pop_modified_edge();
                                    let (vertex_idx_1, vertex_idx_2, _) = &initializer.weighted_edges[edge_index];
                                    initializer.weighted_edges[edge_index] = (*vertex_idx_1, *vertex_idx_2, original_weight);
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

impl ExampleCodeType {
    fn build(&self, d: usize, p: f64, noisy_measurements: usize, max_half_weight: Weight) -> Box<dyn ExampleCode> {
        match self {
            Self::CodeCapacityRepetitionCode => Box::new(CodeCapacityRepetitionCode::new(d, p, max_half_weight)),
            Self::CodeCapacityPlanarCode => Box::new(CodeCapacityPlanarCode::new(d, p, max_half_weight)),
            Self::PhenomenologicalPlanarCode => Box::new(PhenomenologicalPlanarCode::new(d, noisy_measurements, p, max_half_weight)),
            Self::CircuitLevelPlanarCode => Box::new(CircuitLevelPlanarCode::new(d, noisy_measurements, p, max_half_weight)),
            _ => unimplemented!()
        }
    }
}

impl PartitionStrategy {
    fn build(&self, code: &mut Box<dyn ExampleCode>, d: usize, noisy_measurements: usize) -> (SolverInitializer, PartitionConfig) {
        // reorder vertices here, e.g. using [`ExampleCode::reorder_vertices`]
        match self {
            _ => { }
        };
        let initializer = code.get_initializer();
        let partition_config = match self {
            Self::None => PartitionConfig::default(&initializer),
        };
        (initializer, partition_config)
    }
}

trait PrimalDualSolver {
    fn clear(&mut self);
    fn solve_visualizer(&mut self, syndrome_vertices: &Vec<VertexIndex>, erasures: &Vec<EdgeIndex>, visualizer: Option<&mut Visualizer>);
    fn perfect_matching(&mut self) -> PerfectMatching;
    fn sum_dual_variables(&self) -> Weight;
}

struct SolverSerial {
    dual_module: DualModuleSerial,
    primal_module: PrimalModuleSerial,
    interface: DualModuleInterface,
}

impl SolverSerial {
    fn new(initializer: &SolverInitializer) -> Self {
        Self {
            dual_module: DualModuleSerial::new(&initializer),
            primal_module: PrimalModuleSerial::new(&initializer),
            interface: DualModuleInterface::new_empty(),
        }
    }
}

impl PrimalDualSolver for SolverSerial {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
    }
    fn solve_visualizer(&mut self, syndrome_vertices: &Vec<VertexIndex>, erasures: &Vec<EdgeIndex>, visualizer: Option<&mut Visualizer>) {
        self.dual_module.load_erasures(&erasures);
        self.interface = self.primal_module.solve_visualizer(syndrome_vertices, &mut self.dual_module, visualizer);
    }
    fn perfect_matching(&mut self) -> PerfectMatching { self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module) }
    fn sum_dual_variables(&self) -> Weight { self.interface.sum_dual_variables }
}

impl PrimalDualType {
    fn build(&self, initializer: &SolverInitializer, partition_config: &PartitionConfig) -> Box<dyn PrimalDualSolver> {
        match self {
            Self::Serial => Box::new(SolverSerial::new(initializer)),
            _ => unimplemented!()
        }
    }
}

impl Verifier {
    fn build(&self, initializer: &SolverInitializer) -> Box<dyn ResultVerifier> {
        match self {
            Self::None => Box::new(VerifierNone { }),
            Self::BlossomV => Box::new(VerifierBlossomV { 
                initializer: initializer.clone(),
                subgraph_builder: SubGraphBuilder::new(&initializer),
            }),
            _ => unimplemented!()
        }
    }
}

trait ResultVerifier {
    fn verify(&mut self, primal_dual_solver: &mut Box<dyn PrimalDualSolver>, syndrome_vertices: &Vec<VertexIndex>, erasures: &Vec<EdgeIndex>);
}

struct VerifierNone { }

impl ResultVerifier for VerifierNone {
    fn verify(&mut self, _primal_dual_solver: &mut Box<dyn PrimalDualSolver>, _syndrome_vertices: &Vec<VertexIndex>, _erasures: &Vec<EdgeIndex>) { }
}

struct VerifierBlossomV {
    initializer: SolverInitializer,
    subgraph_builder: SubGraphBuilder,
}

impl ResultVerifier for VerifierBlossomV {
    fn verify(&mut self, primal_dual_solver: &mut Box<dyn PrimalDualSolver>, syndrome_vertices: &Vec<VertexIndex>, erasures: &Vec<EdgeIndex>) {
        // prepare modified weighted edges
        let mut edge_modifier = EdgeWeightModifier::new();
        for edge_index in erasures.iter() {
            let (vertex_idx_1, vertex_idx_2, original_weight) = &self.initializer.weighted_edges[*edge_index];
            edge_modifier.push_modified_edge(*edge_index, *original_weight);
            self.initializer.weighted_edges[*edge_index] = (*vertex_idx_1, *vertex_idx_2, 0);
        }
        // use blossom V to compute ground truth
        let blossom_mwpm_result = fusion_blossom::blossom_v_mwpm(&self.initializer, &syndrome_vertices);
        let blossom_details = fusion_blossom::detailed_matching(&self.initializer, &syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            blossom_total_weight += detail.weight;
        }
        // if blossom_total_weight > 0 { println!("w {} {}", primal_dual_solver.sum_dual_variables(), blossom_total_weight); }
        assert_eq!(primal_dual_solver.sum_dual_variables(), blossom_total_weight, "unexpected final dual variable sum");
        // also construct the perfect matching from fusion blossom to compare them
        let fusion_mwpm = primal_dual_solver.perfect_matching();
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_vertices);
        let fusion_details = fusion_blossom::detailed_matching(&self.initializer, &syndrome_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            fusion_total_weight += detail.weight;
        }
        // compare with ground truth from the blossom V algorithm
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        // recover those weighted_edges
        while edge_modifier.has_modified_edges() {
            let (edge_index, original_weight) = edge_modifier.pop_modified_edge();
            let (vertex_idx_1, vertex_idx_2, _) = &self.initializer.weighted_edges[edge_index];
            self.initializer.weighted_edges[edge_index] = (*vertex_idx_1, *vertex_idx_2, original_weight);
        }
        // also test subgraph builder
        self.subgraph_builder.clear();
        self.subgraph_builder.load_erasures(&erasures);
        self.subgraph_builder.load_perfect_matching(&fusion_mwpm);
        // println!("blossom_total_weight: {blossom_total_weight} = {} = {fusion_total_weight}", self.subgraph_builder.total_weight());
        assert_eq!(self.subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
    }
}
