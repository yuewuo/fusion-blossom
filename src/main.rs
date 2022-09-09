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
use fusion_blossom::example_partition;
use pbr::ProgressBar;

use dual_module_serial::DualModuleSerial;
use primal_module_serial::PrimalModuleSerial;
use dual_module_parallel::DualModuleParallel;
use primal_module_parallel::PrimalModuleParallel;
use std::sync::Arc;
use clap::{ValueEnum, Parser, Subcommand};
use serde::Serialize;


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
        /// logging to the default visualizer file at visualize/data/static.json
        #[clap(long, action)]
        enable_visualizer: bool,
        /// the method to verify the correctness of the decoding result
        #[clap(long, arg_enum, default_value_t = Verifier::BlossomV)]
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
                let partition_info = partition_config.into_info(&initializer);
                let mut primal_dual_solver = primal_dual_type.build(&initializer, &partition_info);
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
                print!("\"{word}\" ")
            } else {
                print!("{word} ")
            }
        }
        println!();
    }
    Cli::parse_from(iter).run();
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
        use example_partition::*;
        let partition_num = 10;
        let partition_config = match self {
            Self::None => NoPartition::new().build_apply(code),
            Self::CodeCapacityPlanarCodeVerticalPartitionHalf => CodeCapacityPlanarCodeVerticalPartitionHalf::new(d, d / 2).build_apply(code),
            Self::CodeCapacityPlanarCodeVerticalPartitionFour => CodeCapacityPlanarCodeVerticalPartitionFour::new(d, d / 2, d / 2).build_apply(code),
            Self::CodeCapacityRepetitionCodePartitionHalf => CodeCapacityRepetitionCodePartitionHalf::new(d, d / 2).build_apply(code),
            Self::PhenomenologicalPlanarCodeTimePartition => PhenomenologicalPlanarCodeTimePartition::new(d, noisy_measurements, partition_num).build_apply(code),
        };
        (code.get_initializer(), partition_config)
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

struct SolverDualParallel {
    dual_module: DualModuleParallel<DualModuleSerial>,
    primal_module: PrimalModuleSerial,
    interface: DualModuleInterface,
}

impl SolverDualParallel {
    fn new(initializer: &SolverInitializer, partition_info: &Arc<PartitionInfo>) -> Self {
        let config = dual_module_parallel::DualModuleParallelConfig::default();
        Self {
            dual_module: DualModuleParallel::new_config(&initializer, Arc::clone(partition_info), config),
            primal_module: PrimalModuleSerial::new(&initializer),
            interface: DualModuleInterface::new_empty(),
        }
    }
}

impl PrimalDualSolver for SolverDualParallel {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
    }
    fn solve_visualizer(&mut self, syndrome_vertices: &Vec<VertexIndex>, erasures: &Vec<EdgeIndex>, visualizer: Option<&mut Visualizer>) {
        self.dual_module.static_fuse_all();
        self.dual_module.load_erasures(&erasures);
        self.interface = self.primal_module.solve_visualizer(syndrome_vertices, &mut self.dual_module, visualizer);
    }
    fn perfect_matching(&mut self) -> PerfectMatching { self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module) }
    fn sum_dual_variables(&self) -> Weight { self.interface.sum_dual_variables }
}

struct SolverParallel {
    dual_module: DualModuleParallel<DualModuleSerial>,
    primal_module: PrimalModuleParallel,
    interface: DualModuleInterface,
}

impl SolverParallel {
    fn new(initializer: &SolverInitializer, partition_info: &Arc<PartitionInfo>) -> Self {
        let dual_config = dual_module_parallel::DualModuleParallelConfig::default();
        let primal_config = primal_module_parallel::PrimalModuleParallelConfig::default();
        Self {
            dual_module: DualModuleParallel::new_config(&initializer, Arc::clone(partition_info), dual_config),
            primal_module: PrimalModuleParallel::new_config(&initializer, Arc::clone(&partition_info), primal_config),
            interface: DualModuleInterface::new_empty(),
        }
    }
}

impl PrimalDualSolver for SolverParallel {
    fn clear(&mut self) {
        self.dual_module.clear();
        self.primal_module.clear();
    }
    fn solve_visualizer(&mut self, syndrome_vertices: &Vec<VertexIndex>, erasures: &Vec<EdgeIndex>, visualizer: Option<&mut Visualizer>) {
        self.dual_module.load_erasures(&erasures);
        self.interface = self.primal_module.parallel_solve_visualizer(syndrome_vertices, &mut self.dual_module, visualizer);
    }
    fn perfect_matching(&mut self) -> PerfectMatching { self.primal_module.perfect_matching(&mut self.interface, &mut self.dual_module) }
    fn sum_dual_variables(&self) -> Weight { self.interface.sum_dual_variables }
}

impl PrimalDualType {
    fn build(&self, initializer: &SolverInitializer, partition_info: &Arc<PartitionInfo>) -> Box<dyn PrimalDualSolver> {
        match self {
            Self::Serial => {
                assert_eq!(partition_info.config.partitions.len(), 1, "no partition is supported by serial algorithm, consider using other primal-dual-type");
                Box::new(SolverSerial::new(initializer))
            },
            Self::DualParallel => Box::new(SolverDualParallel::new(initializer, partition_info)),
            Self::Parallel => Box::new(SolverParallel::new(initializer, partition_info)),
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
