extern crate clap;
extern crate pbr;

use fusion_blossom::example_codes::*;
use fusion_blossom::util::*;
use fusion_blossom::visualize::*;
use fusion_blossom::dual_module::*;
use fusion_blossom::primal_module::*;
use fusion_blossom::example_partition;
use fusion_blossom::mwpm_solver::*;
use pbr::ProgressBar;
use rand::{Rng, thread_rng};

use clap::{ValueEnum, Parser, Subcommand};
use serde::Serialize;
use serde_json::json;
use std::env;


const TEST_EACH_ROUNDS: usize = 100;

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
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// benchmark the speed (and also correctness if enabled)
    Benchmark {
        /// code distance
        #[clap(value_parser)]
        d: VertexNum,
        /// physical error rate: the probability of each edge to 
        #[clap(value_parser)]
        p: f64,
        /// rounds of noisy measurement, valid only when multiple rounds
        #[clap(short = 'e', long, default_value_t = 0.)]
        pe: f64,
        /// rounds of noisy measurement, valid only when multiple rounds
        #[clap(short = 'n', long, default_value_t = 0)]
        noisy_measurements: VertexNum,
        /// maximum half weight of edges
        #[clap(long, default_value_t = 500)]
        max_half_weight: Weight,
        /// example code type
        #[clap(short = 'c', long, value_enum, default_value_t = ExampleCodeType::CodeCapacityPlanarCode)]
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
        #[clap(long, value_enum, default_value_t = Verifier::BlossomV)]
        verifier: Verifier,
        /// the number of iterations to run
        #[clap(short = 'r', long, default_value_t = 1000)]
        total_rounds: usize,
        /// select the combination of primal and dual module
        #[clap(short = 'p', long, value_enum, default_value_t = PrimalDualType::Serial)]
        primal_dual_type: PrimalDualType,
        /// the configuration of primal and dual module
        #[clap(long, default_value_t = json!({}))]
        primal_dual_config: serde_json::Value,
        /// partition strategy
        #[clap(long, value_enum, default_value_t = PartitionStrategy::None)]
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
        /// the benchmark profile output file path
        #[clap(long)]
        benchmark_profiler_output: Option<String>,
        /// skip some iterations, useful when debugging
        #[clap(long, default_value_t = 0)]
        starting_iteration: usize,
    },
    /// built-in tests
    Test {
        #[clap(subcommand)]
        command: TestCommands,
    },
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
        /// enable print syndrome pattern
        #[clap(short = 's', long, action)]
        print_syndrome_pattern: bool,
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
        /// enable print syndrome pattern
        #[clap(short = 's', long, action)]
        print_syndrome_pattern: bool,
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
        /// enable print syndrome pattern
        #[clap(short = 's', long, action)]
        print_syndrome_pattern: bool,
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
    /// parallel version
    PhenomenologicalPlanarCodeParallel,
    /// planar surface code with circuit-level noise model
    CircuitLevelPlanarCode,
    /// parallel version
    CircuitLevelPlanarCodeParallel,
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
                    , benchmark_profiler_output, print_syndrome_pattern, starting_iteration } => {
                // check for dependency early
                if matches!(verifier, Verifier::BlossomV) && cfg!(not(feature = "blossom_v")) {
                    panic!("need blossom V library, see README.md")
                }
                // whether to disable progress bar, useful when running jobs in background
                let disable_progress_bar = env::var("DISABLE_PROGRESS_BAR").is_ok();
                let mut code: Box<dyn ExampleCode> = code_type.build(d, p, noisy_measurements, max_half_weight, code_config);
                if pe != 0. { code.set_erasure_probability(pe); }
                if enable_visualizer {  // print visualizer file path only once
                    print_visualize_link(static_visualize_data_filename());
                }
                // create initializer and solver
                let (initializer, partition_config) = partition_strategy.build(&mut *code, d, noisy_measurements, partition_config);
                let partition_info = partition_config.info();
                let mut primal_dual_solver = primal_dual_type.build(&initializer, &partition_info, &*code, primal_dual_config);
                let mut result_verifier = verifier.build(&initializer);
                let mut benchmark_profiler = BenchmarkProfiler::new(noisy_measurements, benchmark_profiler_output.map(|x| (x, &partition_info)));
                // prepare progress bar display
                let mut pb = if !disable_progress_bar {
                    let mut pb = ProgressBar::on(std::io::stderr(), total_rounds as u64);
                    pb.message(format!("{pb_message} ").as_str());
                    Some(pb)
                } else {
                    if !pb_message.is_empty() {
                        print!("{pb_message} ");
                    }
                    None
                };
                let mut rng = thread_rng();
                for round in (starting_iteration as u64)..(total_rounds as u64) {
                    pb.as_mut().map(|pb| pb.set(round));
                    let seed = if use_deterministic_seed { round } else { rng.gen() };
                    let syndrome_pattern = code.generate_random_errors(seed);
                    if print_syndrome_pattern {
                        println!("syndrome_pattern: {:?}", syndrome_pattern);
                    }
                    // create a new visualizer each round
                    let mut visualizer = None;
                    if enable_visualizer {
                        let new_visualizer = Visualizer::new(Some(visualize_data_folder() + static_visualize_data_filename().as_str())
                            , code.get_positions(), true).unwrap();
                        visualizer = Some(new_visualizer);
                    }
                    benchmark_profiler.begin(&syndrome_pattern);
                    primal_dual_solver.solve_visualizer(&syndrome_pattern, visualizer.as_mut());
                    benchmark_profiler.event("decoded".to_string());
                    result_verifier.verify(&mut primal_dual_solver, &syndrome_pattern, visualizer.as_mut());
                    benchmark_profiler.event("verified".to_string());
                    primal_dual_solver.clear();  // also count the clear operation
                    benchmark_profiler.end(Some(&*primal_dual_solver));
                    if let Some(pb) = pb.as_mut() {
                        if pb_message.is_empty() {
                            pb.message(format!("{} ", benchmark_profiler.brief()).as_str());
                        }
                    }
                }
                if disable_progress_bar {  // always print out brief
                    println!("{}", benchmark_profiler.brief());
                } else {
                    if let Some(pb) = pb.as_mut() { pb.finish() }
                    println!();
                }
            },
            Commands::Test { command } => {
                match command {
                    TestCommands::Serial { print_command, enable_visualizer, disable_blossom, print_syndrome_pattern } => {
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
                                    , format!("--noisy-measurements"), format!("{d}")
                                    , format!("--pb-message"), format!("phenomenological {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("circuit-level-planar-code")
                                    , format!("--noisy-measurements"), format!("{d}")
                                    , format!("--pb-message"), format!("circuit-level {d} {p}")]);
                            }
                        }
                        let command_head = vec![format!(""), format!("benchmark")];
                        let mut command_tail = vec!["--total-rounds".to_string(), format!("{TEST_EACH_ROUNDS}")];
                        if !disable_blossom { command_tail.append(&mut vec![format!("--verifier"), format!("blossom-v")]); }
                        if enable_visualizer { command_tail.append(&mut vec![format!("--enable-visualizer")]); }
                        if print_syndrome_pattern { command_tail.append(&mut vec![format!("--print-syndrome-pattern")]); }
                        for parameter in parameters.iter() {
                            execute_in_cli(command_head.iter().chain(parameter.iter()).chain(command_tail.iter()), print_command);
                        }
                    },
                    TestCommands::DualParallel { print_command, enable_visualizer, disable_blossom, print_syndrome_pattern } => {
                        let mut parameters = vec![];
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-repetition-code")
                                    , format!("--partition-strategy"), format!("code-capacity-repetition-code-partition-half")
                                    , format!("--pb-message"), format!("dual-parallel 2-partition repetition {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // simple partition into top and bottom
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--partition-strategy"), format!("code-capacity-planar-code-vertical-partition-half")
                                    , format!("--pb-message"), format!("dual-parallel 2-partition planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // complex partition into 4 blocks
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--partition-strategy"), format!("code-capacity-planar-code-vertical-partition-four")
                                    , format!("--pb-message"), format!("dual-parallel 4-partition planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("phenomenological-planar-code")
                                    , format!("--noisy-measurements"), format!("{d}")
                                    , format!("--partition-strategy"), format!("phenomenological-planar-code-time-partition")
                                    , format!("--partition-config"), "{\"partition_num\":2,\"enable_tree_fusion\":true}".to_string()
                                    , format!("--pb-message"), format!("dual-parallel 2-partition phenomenological {d} {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("circuit-level-planar-code")
                                    , format!("--noisy-measurements"), format!("{d}")
                                    , format!("--partition-strategy"), format!("phenomenological-planar-code-time-partition")
                                    , format!("--partition-config"), "{\"partition_num\":2,\"enable_tree_fusion\":true}".to_string()
                                    , format!("--pb-message"), format!("dual-parallel 2-partition circuit-level {d} {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for partition_num in [2, 3, 4, 5, 6, 7, 8, 9, 10] {  // test large number of fusion without tree fusion
                                let d = 5;
                                let noisy_measurement = 20;
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("circuit-level-planar-code")
                                    , format!("--noisy-measurements"), format!("{noisy_measurement}")
                                    , format!("--partition-strategy"), format!("phenomenological-planar-code-time-partition")
                                    , format!("--partition-config"), format!("{{\"partition_num\":{partition_num},\"enable_tree_fusion\":false}}")
                                    , format!("--pb-message"), format!("dual-parallel {partition_num}-partition circuit-level {d} {noisy_measurement} {p}")]);
                            }
                        }
                        let command_head = vec![format!(""), format!("benchmark")];
                        let mut command_tail = vec![format!("--primal-dual-type"), format!("dual-parallel")
                            , "--total-rounds".to_string(), format!("{TEST_EACH_ROUNDS}")];
                        if !disable_blossom { command_tail.append(&mut vec![format!("--verifier"), format!("blossom-v")]); }
                        if enable_visualizer { command_tail.append(&mut vec![format!("--enable-visualizer")]); }
                        if print_syndrome_pattern { command_tail.append(&mut vec![format!("--print-syndrome-pattern")]); }
                        for parameter in parameters.iter() {
                            execute_in_cli(command_head.iter().chain(parameter.iter()).chain(command_tail.iter()), print_command);
                        }
                    },
                    TestCommands::Parallel { print_command, enable_visualizer, disable_blossom, print_syndrome_pattern } => {
                        let mut parameters = vec![];
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-repetition-code")
                                    , format!("--partition-strategy"), format!("code-capacity-repetition-code-partition-half")
                                    , format!("--pb-message"), format!("parallel 2-partition repetition {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // simple partition into top and bottom
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--partition-strategy"), format!("code-capacity-planar-code-vertical-partition-half")
                                    , format!("--pb-message"), format!("parallel 2-partition planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {  // complex partition into 4 blocks
                            for d in [7, 11, 15, 19] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("code-capacity-planar-code")
                                    , format!("--partition-strategy"), format!("code-capacity-planar-code-vertical-partition-four")
                                    , format!("--pb-message"), format!("parallel 4-partition planar {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("phenomenological-planar-code")
                                    , format!("--noisy-measurements"), format!("{d}")
                                    , format!("--partition-strategy"), format!("phenomenological-planar-code-time-partition")
                                    , format!("--partition-config"), "{\"partition_num\":2,\"enable_tree_fusion\":true}".to_string()
                                    , format!("--pb-message"), format!("parallel 2-partition phenomenological {d} {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for d in [3, 7, 11] {
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("circuit-level-planar-code")
                                    , format!("--noisy-measurements"), format!("{d}")
                                    , format!("--partition-strategy"), format!("phenomenological-planar-code-time-partition")
                                    , format!("--partition-config"), "{\"partition_num\":2,\"enable_tree_fusion\":true}".to_string()
                                    , format!("--pb-message"), format!("parallel 2-partition circuit-level {d} {d} {p}")]);
                            }
                        }
                        for p in [0.001, 0.003, 0.01, 0.03, 0.1, 0.3, 0.499] {
                            for partition_num in [2, 3, 4, 5, 6, 7, 8, 9, 10] {  // test large number of fusion without tree fusion
                                let d = 5;
                                let noisy_measurement = 20;
                                parameters.push(vec![format!("{d}"), format!("{p}"), format!("--code-type"), format!("circuit-level-planar-code")
                                    , format!("--noisy-measurements"), format!("{noisy_measurement}")
                                    , format!("--partition-strategy"), format!("phenomenological-planar-code-time-partition")
                                    , format!("--partition-config"), format!("{{\"partition_num\":{partition_num},\"enable_tree_fusion\":false}}")
                                    , format!("--pb-message"), format!("parallel {partition_num}-partition circuit-level {d} {noisy_measurement} {p}")]);
                            }
                        }
                        let command_head = vec![format!(""), format!("benchmark")];
                        let mut command_tail = vec![format!("--primal-dual-type"), format!("parallel")
                            , "--total-rounds".to_string(), format!("{TEST_EACH_ROUNDS}")];
                        if !disable_blossom { command_tail.append(&mut vec![format!("--verifier"), format!("blossom-v")]); }
                        if enable_visualizer { command_tail.append(&mut vec![format!("--enable-visualizer")]); }
                        if print_syndrome_pattern { command_tail.append(&mut vec![format!("--print-syndrome-pattern")]); }
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
    fn build(&self, d: VertexNum, p: f64, noisy_measurements: VertexNum, max_half_weight: Weight, mut code_config: serde_json::Value) -> Box<dyn ExampleCode> {
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
            Self::PhenomenologicalPlanarCodeParallel => {
                let mut code_count = 1;
                let config = code_config.as_object_mut().expect("config must be JSON object");
                if let Some(value) = config.remove("code_count") {
                    code_count = value.as_u64().expect("code_count number") as usize;
                }
                Box::new(ExampleCodeParallel::new(PhenomenologicalPlanarCode::new(d, noisy_measurements, p, max_half_weight), code_count))
            },
            Self::CircuitLevelPlanarCode => {
                assert_eq!(code_config, json!({}), "config not supported");
                Box::new(CircuitLevelPlanarCode::new(d, noisy_measurements, p, max_half_weight))
            },
            Self::CircuitLevelPlanarCodeParallel => {
                let mut code_count = 1;
                let config = code_config.as_object_mut().expect("config must be JSON object");
                if let Some(value) = config.remove("code_count") {
                    code_count = value.as_u64().expect("code_count number") as usize;
                }
                Box::new(ExampleCodeParallel::new(CircuitLevelPlanarCode::new(d, noisy_measurements, p, max_half_weight), code_count))
            },
            Self::ErrorPatternReader => {
                Box::new(ErrorPatternReader::new(code_config))
            },
            _ => unimplemented!()
        }
    }
}

impl PartitionStrategy {
    fn build(&self, code: &mut dyn ExampleCode, d: VertexNum, noisy_measurements: VertexNum, mut partition_config: serde_json::Value) -> (SolverInitializer, PartitionConfig) {
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
                let mut enable_tree_fusion = false;
                let mut maximum_tree_leaf_size = usize::MAX;
                if let Some(value) = config.remove("partition_num") {
                    partition_num = value.as_u64().expect("partition_num: usize") as usize;
                }
                if let Some(value) = config.remove("enable_tree_fusion") {
                    enable_tree_fusion = value.as_bool().expect("enable_tree_fusion: bool");
                }
                if let Some(value) = config.remove("maximum_tree_leaf_size") {
                    maximum_tree_leaf_size = value.as_u64().expect("maximum_tree_leaf_size: usize") as usize;
                }
                if !config.is_empty() { panic!("unknown config keys: {:?}", config.keys().collect::<Vec<&String>>()); }
                PhenomenologicalPlanarCodeTimePartition::new_tree(d, noisy_measurements, partition_num
                        , enable_tree_fusion, maximum_tree_leaf_size).build_apply(code)
            },
        };
        (code.get_initializer(), partition_config)
    }
}

impl PrimalDualType {
    fn build(&self, initializer: &SolverInitializer, partition_info: &PartitionInfo, code: &dyn ExampleCode
            , primal_dual_config: serde_json::Value) -> Box<dyn PrimalDualSolver> {
        match self {
            Self::Serial => {
                assert_eq!(primal_dual_config, json!({}));
                assert_eq!(partition_info.config.partitions.len(), 1, "no partition is supported by serial algorithm, consider using other primal-dual-type");
                Box::new(SolverSerial::new(initializer))
            },
            Self::DualParallel => {
                Box::new(SolverDualParallel::new(initializer, partition_info, primal_dual_config))
            },
            Self::Parallel => {
                Box::new(SolverParallel::new(initializer, partition_info, primal_dual_config))
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
                subgraph_builder: SubGraphBuilder::new(initializer),
            }),
            _ => unimplemented!()
        }
    }
}

trait ResultVerifier {
    fn verify(&mut self, primal_dual_solver: &mut Box<dyn PrimalDualSolver>, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>);
}

struct VerifierNone { }

impl ResultVerifier for VerifierNone {
    fn verify(&mut self, _primal_dual_solver: &mut Box<dyn PrimalDualSolver>, _syndrome_pattern: &SyndromePattern, _visualizer: Option<&mut Visualizer>) { }
}

struct VerifierBlossomV {
    initializer: SolverInitializer,
    subgraph_builder: SubGraphBuilder,
}

impl ResultVerifier for VerifierBlossomV {
    fn verify(&mut self, primal_dual_solver: &mut Box<dyn PrimalDualSolver>, syndrome_pattern: &SyndromePattern, visualizer: Option<&mut Visualizer>) {
        // prepare modified weighted edges
        let mut edge_modifier = EdgeWeightModifier::new();
        for edge_index in syndrome_pattern.erasures.iter() {
            let (vertex_idx_1, vertex_idx_2, original_weight) = &self.initializer.weighted_edges[*edge_index as usize];
            edge_modifier.push_modified_edge(*edge_index, *original_weight);
            self.initializer.weighted_edges[*edge_index as usize] = (*vertex_idx_1, *vertex_idx_2, 0);
        }
        // use blossom V to compute ground truth
        let blossom_mwpm_result = fusion_blossom::blossom_v_mwpm(&self.initializer, &syndrome_pattern.defect_vertices);
        let blossom_details = fusion_blossom::detailed_matching(&self.initializer, &syndrome_pattern.defect_vertices, &blossom_mwpm_result);
        let mut blossom_total_weight = 0;
        for detail in blossom_details.iter() {
            blossom_total_weight += detail.weight;
        }
        // if blossom_total_weight > 0 { println!("w {} {}", primal_dual_solver.sum_dual_variables(), blossom_total_weight); }
        assert_eq!(primal_dual_solver.sum_dual_variables(), blossom_total_weight, "unexpected final dual variable sum");
        // also construct the perfect matching from fusion blossom to compare them
        let fusion_mwpm = primal_dual_solver.perfect_matching();
        let fusion_mwpm_result = fusion_mwpm.legacy_get_mwpm_result(syndrome_pattern.defect_vertices.clone());
        let fusion_details = fusion_blossom::detailed_matching(&self.initializer, &syndrome_pattern.defect_vertices, &fusion_mwpm_result);
        let mut fusion_total_weight = 0;
        for detail in fusion_details.iter() {
            fusion_total_weight += detail.weight;
        }
        // compare with ground truth from the blossom V algorithm
        assert_eq!(fusion_total_weight, blossom_total_weight, "unexpected final dual variable sum");
        // recover those weighted_edges
        while edge_modifier.has_modified_edges() {
            let (edge_index, original_weight) = edge_modifier.pop_modified_edge();
            let (vertex_idx_1, vertex_idx_2, _) = &self.initializer.weighted_edges[edge_index as usize];
            self.initializer.weighted_edges[edge_index as usize] = (*vertex_idx_1, *vertex_idx_2, original_weight);
        }
        // also test subgraph builder
        self.subgraph_builder.clear();
        self.subgraph_builder.load_erasures(&syndrome_pattern.erasures);
        self.subgraph_builder.load_perfect_matching(&fusion_mwpm);
        // println!("blossom_total_weight: {blossom_total_weight} = {} = {fusion_total_weight}", self.subgraph_builder.total_weight());
        assert_eq!(self.subgraph_builder.total_weight(), blossom_total_weight, "unexpected final dual variable sum");
        if visualizer.is_some() {
            primal_dual_solver.subgraph_visualizer(visualizer);
        }
    }
}
