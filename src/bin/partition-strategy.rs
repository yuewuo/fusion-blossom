use clap::{Parser, Subcommand};
use fusion_blossom::example_codes::*;
use fusion_blossom::example_partition::*;
use fusion_blossom::util::*;

#[derive(Parser, Clone)]
#[clap(author = clap::crate_authors!(", "))]
#[clap(version = env!("CARGO_PKG_VERSION"))]
#[clap(about = "Print partition strategy for analysis")]
#[clap(color = clap::ColorChoice::Auto)]
#[clap(propagate_version = true)]
#[clap(subcommand_required = true)]
#[clap(arg_required_else_help = true)]
pub struct PartitionStrategyCli {
    #[clap(subcommand)]
    partition_strategy: PartitionStrategy,
}

#[derive(Subcommand, Clone)]
#[allow(clippy::large_enum_variant)]
enum PartitionStrategy {
    PhenomenologicalRotatedCodeTimePartition(PhenomenologicalRotatedCodeTimePartition),
    PhenomenologicalRotatedCodeTimePartitionVec {
        /// code distance
        #[clap(value_parser)]
        d: VertexNum,
        /// rounds of noisy measurement, valid only when multiple rounds
        #[clap(value_parser)]
        noisy_measurements: VertexNum,
        /// the number of partition: [a,b,c,...]
        #[clap(value_parser)]
        partition_num_vec: String,
        /// enable tree fusion (to minimize latency but incur log(partition_num) more memory copy)
        #[clap(short = 't', long, default_value_t = false)]
        enable_tree_fusion: bool,
        /// maximum amount of tree leaf; if the total partition is greater than this, it will be cut into multiple regions and each region is a separate tree;
        /// those trees are then fused sequentially
        #[clap(short = 'l', long, default_value_t = format!("[{}]", usize::MAX))]
        maximum_tree_leaf_size_vec: String,
    },
}

impl PartitionStrategyCli {
    pub fn run(self) {
        match self.partition_strategy {
            PartitionStrategy::PhenomenologicalRotatedCodeTimePartition(mut partition) => {
                let mut code = PhenomenologicalRotatedCode::new(partition.d, partition.noisy_measurements, 0.01, 1);
                let partition_config = partition.build_apply(&mut code);
                println!("{}", serde_json::to_string(&partition_config).unwrap());
            }
            PartitionStrategy::PhenomenologicalRotatedCodeTimePartitionVec {
                d,
                noisy_measurements,
                partition_num_vec,
                enable_tree_fusion,
                maximum_tree_leaf_size_vec,
            } => {
                let partition_num_vec: Vec<usize> = serde_json::from_str(&partition_num_vec).expect("should be [a,b,c,...]");
                let maximum_tree_leaf_size_vec: Vec<usize> =
                    serde_json::from_str(&maximum_tree_leaf_size_vec).expect("should be [a,b,c,...]");
                // build a single code for public use
                let mut code = PhenomenologicalRotatedCode::new(d, noisy_measurements, 0.01, 1);
                for &partition_num in partition_num_vec.iter() {
                    for &maximum_tree_leaf_size in maximum_tree_leaf_size_vec.iter() {
                        let mut partition = PhenomenologicalRotatedCodeTimePartition::new_tree(
                            d,
                            noisy_measurements,
                            partition_num,
                            enable_tree_fusion,
                            maximum_tree_leaf_size,
                        );
                        assert!(
                            partition.build_reordered_vertices(&code).is_none(),
                            "should not reorder vertices"
                        );
                        let partition_config = partition.build_apply(&mut code);
                        println!("{}", serde_json::to_string(&partition).unwrap());
                        println!("{}", serde_json::to_string(&partition_config).unwrap());
                    }
                }
            }
        }
    }
}

fn main() {
    PartitionStrategyCli::parse().run();
}
