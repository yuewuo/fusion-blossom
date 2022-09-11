extern crate clap;
extern crate pbr;

use fusion_blossom::example::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::example_partition;
use fusion_blossom::mwpm_solver::*;
use pbr::ProgressBar;
use rand::{Rng, thread_rng};

use std::sync::Arc;
use clap::{ValueEnum, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;


pub fn main() {

    Cli::parse().run();

}

#[derive(Parser, Clone)]
#[clap(author = clap::crate_authors!(", "))]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "Fusion Blossom Algorithm for fast Quantum Error Correction Decoding")]
#[clap(color = clap::ColorChoice::Auto)]
#[clap(propagate_version = true)]
#[clap(subcommand_required = true)]
#[clap(arg_required_else_help = true)]
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
        /// the configuration of the code builder
        #[clap(long, default_value_t = json!({}))]
        code_config: serde_json::Value,
        /// logging to the default visualizer file at visualize/data/static.json
        #[clap(long, action)]
        enable_visualizer: bool,
        /// print syndrome patterns
        #[clap(long, action)]
        print_syndrome_pattern: bool,
        /// the method to verify the correctness of the decoding result
        #[clap(long, arg_enum, default_value_t = Verifier::BlossomV)]
        verifier: Verifier,
        /// the number of iterations to run
        #[clap(short = 'r', long, default_value_t = 1000)]
        total_rounds: usize,
        /// select the combination of primal and dual module
        #[clap(short = 'p', long, arg_enum, default_value_t = PrimalDualType::Serial)]
        primal_dual_type: PrimalDualType,
        /// the configuration of primal and dual module
        #[clap(long, default_value_t = json!({}))]
        primal_dual_config: serde_json::Value,
        /// partition strategy
        #[clap(long, arg_enum, default_value_t = PartitionStrategy::None)]
        partition_strategy: PartitionStrategy,
        /// the configuration of the partition strategy
        #[clap(long, default_value_t = json!({}))]
        partition_config: serde_json::Value,
        /// message on the progress bar
        #[clap(long, default_value_t = format!(""))]
        pb_message: String,
        /// use deterministic seed for debugging purpose
        #[clap(long, action)]
        use_deterministic_seed: bool,
        #[clap(long)]
        benchmark_profiler_output: Option<String>,
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
    Serial {
        /// print out the command to test
        #[clap(short = 'c', long, action)]
        print_command: bool,
        /// enable visualizer
        #[clap(short = 'v', long, action)]
        enable_visualizer: bool,
        /// enable the blossom verifier
        #[clap(short = 'd', long, action)]
        disable_blossom: bool,
    },
    /// test parallel dual module only, with serial primal module
    DualParallel {
        /// print out the command to test
        #[clap(short = 'c', long, action)]
        print_command: bool,
        /// enable visualizer
        #[clap(short = 'v', long, action)]
        enable_visualizer: bool,
        /// enable the blossom verifier
        #[clap(short = 'd', long, action)]
        disable_blossom: bool,
    },
    /// test parallel primal and dual module
    Parallel {
        /// print out the command to test
        #[clap(short = 'c', long, action)]
        print_command: bool,
        /// enable visualizer
        #[clap(short = 'v', long, action)]
        enable_visualizer: bool,
        /// enable the blossom verifier
        #[clap(short = 'd', long, action)]
        disable_blossom: bool,
    },
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
    /// read from error pattern file, generated using option `--primal-dual-type error-pattern-logger`
    ErrorPatternReader,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum PartitionStrategy {
    /// no partition
    None,
    /// partition a planar code into top half and bottom half
    CodeCapacityPlanarCodeVerticalPartitionHalf,
    /// partition a planar code into 4 pieces: top left and right, bottom left and right
    CodeCapacityPlanarCodeVerticalPartitionFour,
    /// partition a repetition code into left and right half
    CodeCapacityRepetitionCodePartitionHalf,
    /// partition a phenomenological (or circuit-level) planar code with time axis
    PhenomenologicalPlanarCodeTimePartition,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Serialize, Debug)]
pub enum PrimalDualType {
    /// serial primal and dual
    Serial,
    /// parallel dual and serial primal
    DualParallel,
    /// parallel primal and dual
    Parallel,
    /// log error into a file for later fetch
    ErrorPatternLogger,
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
                    , partition_strategy, pb_message, primal_dual_config, code_config, partition_config, use_deterministic_seed
                    , benchmark_profiler_output, print_syndrome_pattern } => {
                // check for dependency early
                if matches!(verifier, Verifier::BlossomV) {
                    if cfg!(not(feature = "blossom_v")) {
                        panic!("need blossom V library, see README.md")
                    }
                }
                let mut code: Box<dyn ExampleCode> = code_type.build(d, p, noisy_measurements, max_half_weight, code_config);
                if pe != 0. { code.set_erasure_probability(pe); }
                if enable_visualizer {  // print visualizer file path only once
                    print_visualize_link(&static_visualize_data_filename());
                }
                // create initializer and solver
                let (initializer, partition_config) = partition_strategy.build(&mut code, d, noisy_measurements, partition_config);
                let partition_info = partition_config.into_info();
                let mut primal_dual_solver = primal_dual_type.build(&initializer, &partition_info, &code, primal_dual_config);
                let mut result_verifier = verifier.build(&initializer);
                let mut benchmark_profiler = BenchmarkProfiler::new(noisy_measurements, benchmark_profiler_output.map(|x| (x, partition_info.as_ref())));
                // prepare progress bar display
                let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                pb.message(format!("{pb_message} ").as_str());
                let mut rng = thread_rng();
                for round in 0..(total_rounds as u64) {
                    pb.set(round);
                    let seed = if use_deterministic_seed { round } else { rng.gen() };
                    let syndrome_pattern = code.generate_random_errors(seed);
                    if print_syndrome_pattern {
                        println!("syndrome_pattern: {:?}", syndrome_pattern);
                    }
                    // create a new visualizer each round
                    let mut visualizer = None;
                    if enable_visualizer {
                        let mut new_visualizer = Visualizer::new(Some(visualize_data_folder() + static_visualize_data_filename().as_str())).unwrap();
                        new_visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
                        visualizer = Some(new_visualizer);
                    }
                    // println!("syndrome_vertices: {syndrome_vertices:?}");
                    // println!("erasures: {erasures:?}");
                    benchmark_profiler.begin(&syndrome_pattern);
                    primal_dual_solver.clear();  // including the clear operation
                    primal_dual_solver.solve_visualizer(&syndrome_pattern, visualizer.as_mut());
                    benchmark_profiler.end(Some(&primal_dual_solver));
                    if pb_message.is_empty() {
                        pb.message(format!("{} ", benchmark_profiler.brief()).as_str());
                    }
                    result_verifier.verify(&mut primal_dual_solver, &syndrome_pattern);
                }
                pb.finish();
                println!("");
            },
            Commands::Test { command } => {
                match command {
                    TestCommands::Serial { print_command, enable_visualizer, disable_blossom } => {
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
                        let command_head = vec![format!(""), format!("benchmark")];
                        let mut command_tail = vec![];
                        if !disable_blossom { command_tail.append(&mut vec![format!("--verifier"), format!("blossom-v")]); }
                        if enable_visualizer { command_tail.append(&mut vec![format!("--enable-visualizer")]); }
                        for parameter in parameters.iter() {
                            execute_in_cli(command_head.iter().chain(parameter.iter()).chain(command_tail.iter()), print_command);
                        }
                    },
                    TestCommands::DualParallel { print_command, enable_visualizer, disable_blossom } => {
                        let mut parameters = vec![];
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-repetition-code")
                                    , format!("--partition-strategy"), format!("code-capacity-repetition-code-partition-half")
                                    , format!("--pb-message"), format!("2-partition repetition {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // simple partition into top and bottom
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--partition-strategy"), format!("code-capacity-planar-code-vertical-partition-half")
                                    , format!("--pb-message"), format!("2-partition planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // complex partition into 4 blocks
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--partition-strategy"), format!("code-capacity-planar-code-vertical-partition-four")
                                    , format!("--pb-message"), format!("4-partition planar {d} {p}")]);
                            }
                        }
                        let command_head = vec![format!(""), format!("benchmark")];
                        let mut command_tail = vec![format!("--primal-dual-type"), format!("dual-parallel")];
                        if !disable_blossom { command_tail.append(&mut vec![format!("--verifier"), format!("blossom-v")]); }
                        if enable_visualizer { command_tail.append(&mut vec![format!("--enable-visualizer")]); }
                        for parameter in parameters.iter() {
                            execute_in_cli(command_head.iter().chain(parameter.iter()).chain(command_tail.iter()), print_command);
                        }
                    },
                    TestCommands::Parallel { print_command, enable_visualizer, disable_blossom } => {
                        let mut parameters = vec![];
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-repetition-code")
                                    , format!("--partition-strategy"), format!("code-capacity-repetition-code-partition-half")
                                    , format!("--pb-message"), format!("2-partition repetition {d} {p}")]);
                            }
                        }
                        let command_head = vec![format!(""), format!("benchmark")];
                        let mut command_tail = vec![format!("--primal-dual-type"), format!("parallel")];
                        if !disable_blossom { command_tail.append(&mut vec![format!("--verifier"), format!("blossom-v")]); }
                        if enable_visualizer { command_tail.append(&mut vec![format!("--enable-visualizer")]); }
                        for parameter in parameters.iter() {
                            execute_in_cli(command_head.iter().chain(parameter.iter()).chain(command_tail.iter()), print_command);
                        }
                    },
                }
            },
        }
    }
}

pub fn execute_in_cli<'a>(iter: impl Iterator<Item=&'a String> + Clone, print_command: bool) {
    if print_command {
        print!("[command]");
        for word in iter.clone() {
            if word.contains(char::is_whitespace) {
                print!("'{word}' ")
            } else {
                print!("{word} ")
            }
        }
        println!();
    }
    Cli::parse_from(iter).run();
}

impl ExampleCodeType {
    fn build(&self, d: usize, p: f64, noisy_measurements: usize, max_half_weight: Weight, code_config: serde_json::Value) -> Box<dyn ExampleCode> {
        match self {
            Self::CodeCapacityRepetitionCode => {
                assert_eq!(code_config, json!({}), "config not supported");
                Box::new(CodeCapacityRepetitionCode::new(d, p, max_half_weight))
            },
            Self::CodeCapacityPlanarCode => {
                assert_eq!(code_config, json!({}), "config not supported");
                Box::new(CodeCapacityPlanarCode::new(d, p, max_half_weight))
            },
            Self::PhenomenologicalPlanarCode => {
                assert_eq!(code_config, json!({}), "config not supported");
                Box::new(PhenomenologicalPlanarCode::new(d, noisy_measurements, p, max_half_weight))
            },
            Self::CircuitLevelPlanarCode => {
                assert_eq!(code_config, json!({}), "config not supported");
                Box::new(CircuitLevelPlanarCode::new(d, noisy_measurements, p, max_half_weight))
            },
            Self::ErrorPatternReader => {
                Box::new(ErrorPatternReader::new(code_config))
            },
            _ => unimplemented!()
        }
    }
}

impl PartitionStrategy {
    fn build(&self, code: &mut Box<dyn ExampleCode>, d: usize, noisy_measurements: usize, mut partition_config: serde_json::Value) -> (SolverInitializer, PartitionConfig) {
        use example_partition::*;
        let partition_config = match self {
            Self::None => {
                assert_eq!(partition_config, json!({}), "config not supported");
                NoPartition::new().build_apply(code)
            },
            Self::CodeCapacityPlanarCodeVerticalPartitionHalf => {
                assert_eq!(partition_config, json!({}), "config not supported");
                CodeCapacityPlanarCodeVerticalPartitionHalf::new(d, d / 2).build_apply(code)
            },
            Self::CodeCapacityPlanarCodeVerticalPartitionFour => {
                assert_eq!(partition_config, json!({}), "config not supported");
                CodeCapacityPlanarCodeVerticalPartitionFour::new(d, d / 2, d / 2).build_apply(code)
            },
            Self::CodeCapacityRepetitionCodePartitionHalf => {
                assert_eq!(partition_config, json!({}), "config not supported");
                CodeCapacityRepetitionCodePartitionHalf::new(d, d / 2).build_apply(code)
            },
            Self::PhenomenologicalPlanarCodeTimePartition => {
                let config = partition_config.as_object_mut().expect("config must be JSON object");
                let mut partition_num = 10;
                config.remove("partition_num").map(|value| partition_num = value.as_u64().expect("partition_num: usize") as usize);
                if !config.is_empty() { panic!("unknown config keys: {:?}", config.keys().collect::<Vec<&String>>()); }
                PhenomenologicalPlanarCodeTimePartition::new(d, noisy_measurements, partition_num).build_apply(code)
            },
        };
        (code.get_initializer(), partition_config)
    }
}

impl PrimalDualType {
    fn build(&self, initializer: &SolverInitializer, partition_info: &Arc<PartitionInfo>, code: &Box<dyn ExampleCode>
            , primal_dual_config: serde_json::Value) -> Box<dyn PrimalDualSolver> {
        match self {
            Self::Serial => {
                assert_eq!(primal_dual_config, json!({}));
                assert_eq!(partition_info.config.partitions.len(), 1, "no partition is supported by serial algorithm, consider using other primal-dual-type");
                Box::new(SolverSerial::new(initializer))
            },
            Self::DualParallel => {
                assert_eq!(primal_dual_config, json!({}));
                Box::new(SolverDualParallel::new(initializer, partition_info))
            },
            Self::Parallel => {
                assert_eq!(primal_dual_config, json!({}));
                Box::new(SolverParallel::new(initializer, partition_info))
            },
            Self::ErrorPatternLogger => {
                Box::new(SolverErrorPatternLogger::new(initializer, code, primal_dual_config))
            },
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
    fn verify(&mut self, primal_dual_solver: &mut Box<dyn PrimalDualSolver>, syndrome_pattern: &SyndromePattern);
}

struct VerifierNone { }

impl ResultVerifier for VerifierNone {
    fn verify(&mut self, _primal_dual_solver: &mut Box<dyn PrimalDualSolver>, _syndrome_pattern: &SyndromePattern) { }
}

struct VerifierBlossomV {
    initializer: SolverInitializer,
    subgraph_builder: SubGraphBuilder,
}

impl ResultVerifier for VerifierBlossomV {
    fn verify(&mut self, primal_dual_solver: &mut Box<dyn PrimalDualSolver>, syndrome_pattern: &SyndromePattern) {
        // prepare modified weighted edges
        let mut edge_modifier = EdgeWeightModifier::new();
        for edge_index in syndrome_pattern.erasures.iter() {
            let (vertex_idx_1, vertex_idx_2, original_weight) = &self.initializer.weighted_edges[*edge_index];
            edge_modifier.push_modified_edge(*edge_index, *original_weight);
            self.initializer.weighted_edges[*edge_index] = (*vertex_idx_1, *vertex_idx_2, 0);
        }
        // use blossom V to compute ground truth
        let blossom_mwpm_result = fusion_blossom::blossom_v_mwpm(&self.initializer, &syndrome_pattern.syndrome_vertices);
        let blossom_details = fusion_blossom::detailed_matching(&self.initializer, &syndrome_pattern.syndrome_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            blossom_total_weight += detail.weight;
        }
        // if blossom_total_weight > 0 { println!("w {} {}", primal_dual_solver.sum_dual_variables(), blossom_total_weight); }
        assert_eq!(primal_dual_solver.sum_dual_variables(), blossom_total_weight, "unexpected final dual variable sum");
        // also construct the perfect matching from fusion blossom to compare them
        let fusion_mwpm = primal_dual_solver.perfect_matching();
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(&syndrome_pattern.syndrome_vertices);
        let fusion_details = fusion_blossom::detailed_matching(&self.initializer, &syndrome_pattern.syndrome_vertices, &fusion_mwpm_result);
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
        self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        self.subgraph_builder.load_perfect_matching(&fusion_mwpm);
        // println!("blossom_total_weight: {blossom_total_weight} = {} = {fusion_total_weight}", self.subgraph_builder.total_weight());
        assert_eq!(self.subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
    }
}
