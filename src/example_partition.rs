//! Example Partition
//!
//! This module contains example partition for some of the example codes
//!

use super::example_codes::*;
use super::util::*;
use clap::Parser;
use serde::Serialize;
use std::collections::VecDeque;

pub trait ExamplePartition {
    /// customize partition, note that this process may re-order the vertices in `code`
    fn build_apply(&mut self, code: &mut dyn ExampleCode) -> PartitionConfig {
        // first apply reorder
        if let Some(reordered_vertices) = self.build_reordered_vertices(code) {
            code.reorder_vertices(&reordered_vertices);
        }
        self.build_partition(code)
    }

    fn re_index_defect_vertices(&mut self, code: &dyn ExampleCode, defect_vertices: &[VertexIndex]) -> Vec<VertexIndex> {
        if let Some(reordered_vertices) = self.build_reordered_vertices(code) {
            translated_defect_to_reordered(&reordered_vertices, defect_vertices)
        } else {
            defect_vertices.into()
        }
    }

    /// build reorder vertices
    fn build_reordered_vertices(&mut self, _code: &dyn ExampleCode) -> Option<Vec<VertexIndex>> {
        None
    }

    /// build the partition, using the indices after reordered vertices
    fn build_partition(&mut self, code: &dyn ExampleCode) -> PartitionConfig;
}

/// no partition
pub struct NoPartition {}

impl Default for NoPartition {
    fn default() -> Self {
        Self::new()
    }
}

impl NoPartition {
    pub fn new() -> Self {
        Self {}
    }
}

impl ExamplePartition for NoPartition {
    fn build_partition(&mut self, code: &dyn ExampleCode) -> PartitionConfig {
        PartitionConfig::new(code.vertex_num())
    }
}

/// partition into top half and bottom half
#[derive(Default)]
pub struct CodeCapacityPlanarCodeVerticalPartitionHalf {
    d: VertexNum,
    /// the row of splitting: in the visualization tool, the top row is the 1st row, the bottom row is the d-th row
    partition_row: VertexNum,
}

impl CodeCapacityPlanarCodeVerticalPartitionHalf {
    pub fn new(d: VertexNum, partition_row: VertexNum) -> Self {
        Self { d, partition_row }
    }
}

impl ExamplePartition for CodeCapacityPlanarCodeVerticalPartitionHalf {
    fn build_partition(&mut self, code: &dyn ExampleCode) -> PartitionConfig {
        let (d, partition_row) = (self.d, self.partition_row);
        assert_eq!(code.vertex_num(), d * (d + 1), "code size incompatible");
        let mut config = PartitionConfig::new(code.vertex_num());
        assert!(partition_row > 1 && partition_row < d);
        config.partitions = vec![
            VertexRange::new(0, (partition_row - 1) * (d + 1)),
            VertexRange::new(partition_row * (d + 1), d * (d + 1)),
        ];
        config.fusions = vec![(0, 1)];
        config
    }
}

/// partition into top half and bottom half
#[derive(Default)]
pub struct CodeCapacityRotatedCodeVerticalPartitionHalf {
    d: VertexNum,
    /// the row of splitting: in the visualization tool, the top row is the 1st row, the bottom row is the d-th row
    partition_row: VertexNum,
}

impl CodeCapacityRotatedCodeVerticalPartitionHalf {
    pub fn new(d: VertexNum, partition_row: VertexNum) -> Self {
        Self { d, partition_row }
    }
}

impl ExamplePartition for CodeCapacityRotatedCodeVerticalPartitionHalf {
    fn build_partition(&mut self, code: &dyn ExampleCode) -> PartitionConfig {
        let (d, partition_row) = (self.d, self.partition_row);
        let row_vertex_num = (d - 1) / 2 + 1;
        assert_eq!(code.vertex_num(), row_vertex_num * (d + 1), "code size incompatible");
        let mut config = PartitionConfig::new(code.vertex_num());
        assert!(partition_row >= 1 && partition_row < d);
        config.partitions = vec![
            VertexRange::new(0, partition_row * row_vertex_num),
            VertexRange::new((partition_row + 1) * row_vertex_num, row_vertex_num * (d + 1)),
        ];
        config.fusions = vec![(0, 1)];
        config
    }
}

/// partition into 4 pieces: top left and right, bottom left and right
#[derive(Default)]
pub struct CodeCapacityPlanarCodeVerticalPartitionFour {
    d: VertexNum,
    /// the row of splitting: in the visualization tool, the top row is the 1st row, the bottom row is the d-th row
    partition_row: VertexNum,
    /// the row of splitting: in the visualization tool, the left (non-virtual) column is the 1st column, the right (non-virtual) column is the (d-1)-th column
    partition_column: VertexNum,
}

impl CodeCapacityPlanarCodeVerticalPartitionFour {
    pub fn new(d: VertexNum, partition_row: VertexNum, partition_column: VertexNum) -> Self {
        Self {
            d,
            partition_row,
            partition_column,
        }
    }
}

impl ExamplePartition for CodeCapacityPlanarCodeVerticalPartitionFour {
    fn build_reordered_vertices(&mut self, code: &dyn ExampleCode) -> Option<Vec<VertexIndex>> {
        let (d, partition_row, partition_column) = (self.d, self.partition_row, self.partition_column);
        assert_eq!(code.vertex_num(), d * (d + 1), "code size incompatible");
        assert!(partition_row > 1 && partition_row < d);
        let mut reordered_vertices = vec![];
        let split_horizontal = partition_row - 1;
        let split_vertical = partition_column - 1;
        for i in 0..split_horizontal {
            // left-top block
            for j in 0..split_vertical {
                reordered_vertices.push(i * (d + 1) + j);
            }
            reordered_vertices.push(i * (d + 1) + d);
        }
        for i in 0..split_horizontal {
            // interface between the left-top block and the right-top block
            reordered_vertices.push(i * (d + 1) + split_vertical);
        }
        for i in 0..split_horizontal {
            // right-top block
            for j in (split_vertical + 1)..d {
                reordered_vertices.push(i * (d + 1) + j);
            }
        }
        {
            // the big interface between top and bottom
            for j in 0..(d + 1) {
                reordered_vertices.push(split_horizontal * (d + 1) + j);
            }
        }
        for i in (split_horizontal + 1)..d {
            // left-bottom block
            for j in 0..split_vertical {
                reordered_vertices.push(i * (d + 1) + j);
            }
            reordered_vertices.push(i * (d + 1) + d);
        }
        for i in (split_horizontal + 1)..d {
            // interface between the left-bottom block and the right-bottom block
            reordered_vertices.push(i * (d + 1) + split_vertical);
        }
        for i in (split_horizontal + 1)..d {
            // right-bottom block
            for j in (split_vertical + 1)..d {
                reordered_vertices.push(i * (d + 1) + j);
            }
        }
        Some(reordered_vertices)
    }
    fn build_partition(&mut self, _code: &dyn ExampleCode) -> PartitionConfig {
        let (d, partition_row, partition_column) = (self.d, self.partition_row, self.partition_column);
        let mut config = PartitionConfig::new(d * (d + 1));
        let b0_count = (partition_row - 1) * partition_column;
        let b1_count = (partition_row - 1) * (d - partition_column);
        let b2_count = (d - partition_row) * partition_column;
        let b3_count = (d - partition_row) * (d - partition_column);
        config.partitions = vec![
            VertexRange::new_length(0, b0_count),
            VertexRange::new_length(b0_count + (partition_row - 1), b1_count),
            VertexRange::new_length(partition_row * (d + 1), b2_count),
            VertexRange::new_length(partition_row * (d + 1) + b2_count + (d - partition_row), b3_count),
        ];
        config.fusions = vec![(0, 1), (2, 3), (4, 5)];
        config
    }
}

/// partition into top half and bottom half
#[derive(Default)]
pub struct CodeCapacityRepetitionCodePartitionHalf {
    d: VertexNum,
    /// the position of splitting: in the visualization tool, the left (non-virtual) vertex is the 1st column, the right (non-virtual) vertex is the (d-1)-th column
    partition_index: VertexIndex,
}

impl CodeCapacityRepetitionCodePartitionHalf {
    pub fn new(d: VertexNum, partition_index: VertexIndex) -> Self {
        Self { d, partition_index }
    }
}

impl ExamplePartition for CodeCapacityRepetitionCodePartitionHalf {
    fn build_reordered_vertices(&mut self, code: &dyn ExampleCode) -> Option<Vec<VertexIndex>> {
        let (d, partition_index) = (self.d, self.partition_index);
        assert_eq!(code.vertex_num(), d + 1, "code size incompatible");
        assert!(partition_index > 1 && partition_index < d);
        let mut reordered_vertices = vec![];
        let split_vertical = partition_index - 1;
        for j in 0..split_vertical {
            reordered_vertices.push(j);
        }
        reordered_vertices.push(d);
        for j in split_vertical..d {
            reordered_vertices.push(j);
        }
        Some(reordered_vertices)
    }
    fn build_partition(&mut self, _code: &dyn ExampleCode) -> PartitionConfig {
        let (d, partition_index) = (self.d, self.partition_index);
        let mut config = PartitionConfig::new(d + 1);
        config.partitions = vec![
            VertexRange::new(0, partition_index),
            VertexRange::new(partition_index + 1, d + 1),
        ];
        config.fusions = vec![(0, 1)];
        config
    }
}

/// evenly partition along the time axis
pub struct PhenomenologicalPlanarCodeTimePartition {
    d: VertexNum,
    noisy_measurements: VertexNum,
    /// the number of partition
    partition_num: usize,
    /// enable tree fusion (to minimize latency but incur log(partition_num) more memory copy)
    enable_tree_fusion: bool,
    /// maximum amount of tree leaf; if the total partition is greater than this, it will be cut into multiple regions and each region is a separate tree;
    /// those trees are then fused sequentially
    maximum_tree_leaf_size: usize,
}

impl PhenomenologicalPlanarCodeTimePartition {
    pub fn new_tree(
        d: VertexNum,
        noisy_measurements: VertexNum,
        partition_num: usize,
        enable_tree_fusion: bool,
        maximum_tree_leaf_size: usize,
    ) -> Self {
        Self {
            d,
            noisy_measurements,
            partition_num,
            enable_tree_fusion,
            maximum_tree_leaf_size,
        }
    }
    pub fn new(d: VertexNum, noisy_measurements: VertexNum, partition_num: usize) -> Self {
        Self::new_tree(d, noisy_measurements, partition_num, false, usize::MAX)
    }
}

impl ExamplePartition for PhenomenologicalPlanarCodeTimePartition {
    #[allow(clippy::unnecessary_cast)]
    fn build_partition(&mut self, code: &dyn ExampleCode) -> PartitionConfig {
        let (d, noisy_measurements, partition_num) = (self.d, self.noisy_measurements, self.partition_num);
        let round_vertex_num = d * (d + 1);
        let vertex_num = round_vertex_num * (noisy_measurements + 1);
        assert_eq!(code.vertex_num(), vertex_num, "code size incompatible");
        assert!(partition_num >= 1 && partition_num <= noisy_measurements as usize + 1);
        // do not use fixed partition_length, because it would introduce super long partition; do it on the fly
        let mut config = PartitionConfig::new(vertex_num);
        config.partitions.clear();
        for partition_index in 0..partition_num as VertexIndex {
            let start_round_index = partition_index * (noisy_measurements + 1) / partition_num as VertexNum;
            let end_round_index = (partition_index + 1) * (noisy_measurements + 1) / partition_num as VertexNum;
            assert!(end_round_index > start_round_index, "empty partition occurs");
            if partition_index == 0 {
                config.partitions.push(VertexRange::new(
                    start_round_index * round_vertex_num,
                    end_round_index * round_vertex_num,
                ));
            } else {
                config.partitions.push(VertexRange::new(
                    (start_round_index + 1) * round_vertex_num,
                    end_round_index * round_vertex_num,
                ));
            }
        }
        config.fusions.clear();
        if !self.enable_tree_fusion || self.maximum_tree_leaf_size == 1 {
            for unit_index in partition_num..(2 * partition_num - 1) {
                if unit_index == partition_num {
                    config.fusions.push((0, 1));
                } else {
                    config.fusions.push((unit_index as usize - 1, unit_index - partition_num + 1));
                }
            }
        } else {
            let mut whole_ranges = vec![];
            let mut left_right_leaf = vec![];
            for (unit_index, partition) in config.partitions.iter().enumerate() {
                assert!(
                    partition.end() <= vertex_num,
                    "invalid vertex index {} in partitions",
                    partition.end()
                );
                whole_ranges.push(*partition);
                left_right_leaf.push((unit_index, unit_index));
            }
            // first cut into multiple regions
            let region_count = if config.partitions.len() <= self.maximum_tree_leaf_size {
                1
            } else {
                (config.partitions.len() + self.maximum_tree_leaf_size - 1) / self.maximum_tree_leaf_size
            };
            let mut last_sequential_unit: Option<usize> = None;
            for region_index in 0..region_count {
                let region_start = region_index * self.maximum_tree_leaf_size;
                let region_end = std::cmp::min((region_index + 1) * self.maximum_tree_leaf_size, config.partitions.len());
                // build the local tree
                let mut pending_fusion = VecDeque::new();
                for unit_index in region_start..region_end {
                    pending_fusion.push_back(unit_index);
                }
                let local_fusion_start_index = whole_ranges.len();
                for unit_index in local_fusion_start_index..(local_fusion_start_index + region_end - region_start - 1) {
                    let mut unit_index_1 = pending_fusion.pop_front().unwrap();
                    // iterate over all pending fusions to find a neighboring one
                    for i in 0..pending_fusion.len() {
                        let mut unit_index_2 = pending_fusion[i];
                        let is_neighbor = left_right_leaf[unit_index_1].0 == left_right_leaf[unit_index_2].1 + 1
                            || left_right_leaf[unit_index_2].0 == left_right_leaf[unit_index_1].1 + 1;
                        if is_neighbor {
                            pending_fusion.remove(i);
                            if whole_ranges[unit_index_1].start() > whole_ranges[unit_index_2].start() {
                                (unit_index_1, unit_index_2) = (unit_index_2, unit_index_1);
                                // only lower range can fuse higher range
                            }
                            config.fusions.push((unit_index_1, unit_index_2));
                            pending_fusion.push_back(unit_index);
                            // println!("unit_index_1: {unit_index_1} {:?}, unit_index_2: {unit_index_2} {:?}", whole_ranges[unit_index_1], whole_ranges[unit_index_2]);
                            let (whole_range, _) = whole_ranges[unit_index_1].fuse(&whole_ranges[unit_index_2]);
                            whole_ranges.push(whole_range);
                            left_right_leaf.push((left_right_leaf[unit_index_1].0, left_right_leaf[unit_index_2].1));
                            break;
                        }
                        assert!(i != pending_fusion.len() - 1, "unreachable: cannot find a neighbor");
                    }
                }
                assert!(pending_fusion.len() == 1, "only the final unit is left");
                let tree_root_unit_index = pending_fusion.pop_front().unwrap();
                if let Some(last_sequential_unit) = last_sequential_unit.as_mut() {
                    config.fusions.push((*last_sequential_unit, tree_root_unit_index));
                    let (whole_range, _) = whole_ranges[*last_sequential_unit].fuse(&whole_ranges[tree_root_unit_index]);
                    whole_ranges.push(whole_range);
                    left_right_leaf.push((
                        left_right_leaf[*last_sequential_unit].0,
                        left_right_leaf[tree_root_unit_index].1,
                    ));
                    *last_sequential_unit = tree_root_unit_index + 1;
                } else {
                    last_sequential_unit = Some(tree_root_unit_index);
                }
            }
        }
        config
    }
}

/// evenly partition along the time axis
#[derive(Parser, Clone, Serialize)]
pub struct PhenomenologicalRotatedCodeTimePartition {
    /// code distance
    #[clap(value_parser)]
    pub d: VertexNum,
    /// rounds of noisy measurement, valid only when multiple rounds
    #[clap(value_parser)]
    pub noisy_measurements: VertexNum,
    /// the number of partition
    #[clap(value_parser)]
    pub partition_num: usize,
    /// enable tree fusion (to minimize latency but incur log(partition_num) more memory copy)
    #[clap(short = 't', long, default_value_t = false)]
    pub enable_tree_fusion: bool,
    /// maximum amount of tree leaf; if the total partition is greater than this, it will be cut into multiple regions and each region is a separate tree;
    /// those trees are then fused sequentially
    #[clap(short = 'l', long, default_value_t = usize::MAX)]
    pub maximum_tree_leaf_size: usize,
}

impl PhenomenologicalRotatedCodeTimePartition {
    pub fn new_tree(
        d: VertexNum,
        noisy_measurements: VertexNum,
        partition_num: usize,
        enable_tree_fusion: bool,
        maximum_tree_leaf_size: usize,
    ) -> Self {
        Self {
            d,
            noisy_measurements,
            partition_num,
            enable_tree_fusion,
            maximum_tree_leaf_size,
        }
    }
    pub fn new(d: VertexNum, noisy_measurements: VertexNum, partition_num: usize) -> Self {
        Self::new_tree(d, noisy_measurements, partition_num, false, usize::MAX)
    }
}

impl ExamplePartition for PhenomenologicalRotatedCodeTimePartition {
    #[allow(clippy::unnecessary_cast)]
    fn build_partition(&mut self, code: &dyn ExampleCode) -> PartitionConfig {
        let (d, noisy_measurements, partition_num) = (self.d, self.noisy_measurements, self.partition_num);
        let row_vertex_num = (d - 1) / 2 + 1;
        let round_vertex_num = row_vertex_num * (d + 1);
        let vertex_num = round_vertex_num * (noisy_measurements + 1);
        assert_eq!(code.vertex_num(), vertex_num, "code size incompatible");
        assert!(partition_num >= 1 && partition_num <= noisy_measurements as usize + 1);
        // do not use fixed partition_length, because it would introduce super long partition; do it on the fly
        let mut config = PartitionConfig::new(vertex_num);
        config.partitions.clear();
        for partition_index in 0..partition_num as VertexIndex {
            let start_round_index = partition_index * (noisy_measurements + 1) / partition_num as VertexNum;
            let end_round_index = (partition_index + 1) * (noisy_measurements + 1) / partition_num as VertexNum;
            assert!(end_round_index > start_round_index, "empty partition occurs");
            if partition_index == 0 {
                config.partitions.push(VertexRange::new(
                    start_round_index * round_vertex_num,
                    end_round_index * round_vertex_num,
                ));
            } else {
                config.partitions.push(VertexRange::new(
                    (start_round_index + 1) * round_vertex_num,
                    end_round_index * round_vertex_num,
                ));
            }
        }
        config.fusions.clear();
        if !self.enable_tree_fusion || self.maximum_tree_leaf_size == 1 {
            for unit_index in partition_num..(2 * partition_num - 1) {
                if unit_index == partition_num {
                    config.fusions.push((0, 1));
                } else {
                    config.fusions.push((unit_index as usize - 1, unit_index - partition_num + 1));
                }
            }
        } else {
            let mut whole_ranges = vec![];
            let mut left_right_leaf = vec![];
            for (unit_index, partition) in config.partitions.iter().enumerate() {
                assert!(
                    partition.end() <= vertex_num,
                    "invalid vertex index {} in partitions",
                    partition.end()
                );
                whole_ranges.push(*partition);
                left_right_leaf.push((unit_index, unit_index));
            }
            // first cut into multiple regions
            let region_count = if config.partitions.len() <= self.maximum_tree_leaf_size {
                1
            } else {
                (config.partitions.len() + self.maximum_tree_leaf_size - 1) / self.maximum_tree_leaf_size
            };
            let mut last_sequential_unit: Option<usize> = None;
            for region_index in 0..region_count {
                let region_start = region_index * self.maximum_tree_leaf_size;
                let region_end = std::cmp::min((region_index + 1) * self.maximum_tree_leaf_size, config.partitions.len());
                // build the local tree
                let mut pending_fusion = VecDeque::new();
                for unit_index in region_start..region_end {
                    pending_fusion.push_back(unit_index);
                }
                let local_fusion_start_index = whole_ranges.len();
                for unit_index in local_fusion_start_index..(local_fusion_start_index + region_end - region_start - 1) {
                    let mut unit_index_1 = pending_fusion.pop_front().unwrap();
                    // iterate over all pending fusions to find a neighboring one
                    for i in 0..pending_fusion.len() {
                        let mut unit_index_2 = pending_fusion[i];
                        let is_neighbor = left_right_leaf[unit_index_1].0 == left_right_leaf[unit_index_2].1 + 1
                            || left_right_leaf[unit_index_2].0 == left_right_leaf[unit_index_1].1 + 1;
                        if is_neighbor {
                            pending_fusion.remove(i);
                            if whole_ranges[unit_index_1].start() > whole_ranges[unit_index_2].start() {
                                (unit_index_1, unit_index_2) = (unit_index_2, unit_index_1);
                                // only lower range can fuse higher range
                            }
                            config.fusions.push((unit_index_1, unit_index_2));
                            pending_fusion.push_back(unit_index);
                            // println!("unit_index_1: {unit_index_1} {:?}, unit_index_2: {unit_index_2} {:?}", whole_ranges[unit_index_1], whole_ranges[unit_index_2]);
                            let (whole_range, _) = whole_ranges[unit_index_1].fuse(&whole_ranges[unit_index_2]);
                            whole_ranges.push(whole_range);
                            left_right_leaf.push((left_right_leaf[unit_index_1].0, left_right_leaf[unit_index_2].1));
                            break;
                        }
                        assert!(i != pending_fusion.len() - 1, "unreachable: cannot find a neighbor");
                    }
                }
                assert!(pending_fusion.len() == 1, "only the final unit is left");
                let tree_root_unit_index = pending_fusion.pop_front().unwrap();
                if let Some(last_sequential_unit) = last_sequential_unit.as_mut() {
                    config.fusions.push((*last_sequential_unit, tree_root_unit_index));
                    let (whole_range, _) = whole_ranges[*last_sequential_unit].fuse(&whole_ranges[tree_root_unit_index]);
                    whole_ranges.push(whole_range);
                    left_right_leaf.push((
                        left_right_leaf[*last_sequential_unit].0,
                        left_right_leaf[tree_root_unit_index].1,
                    ));
                    *last_sequential_unit = tree_root_unit_index + 1;
                } else {
                    last_sequential_unit = Some(tree_root_unit_index);
                }
            }
        }
        config
    }
}

#[cfg(test)]
pub mod tests {
    use super::super::dual_module::*;
    use super::super::dual_module_parallel::*;
    use super::super::dual_module_serial::*;
    #[cfg(feature = "unsafe_pointer")]
    use super::super::pointers::UnsafePtr;
    use super::super::primal_module::*;
    use super::super::primal_module_parallel::*;
    use super::super::visualize::*;
    use super::*;

    pub fn example_partition_basic_standard_syndrome_optional_viz(
        code: &mut dyn ExampleCode,
        visualize_filename: Option<String>,
        mut defect_vertices: Vec<VertexIndex>,
        re_index_syndrome: bool,
        final_dual: Weight,
        mut partition: impl ExamplePartition,
    ) -> (PrimalModuleParallel, DualModuleParallel<DualModuleSerial>) {
        println!("{defect_vertices:?}");
        if re_index_syndrome {
            defect_vertices = partition.re_index_defect_vertices(code, &defect_vertices);
        }
        let partition_config = partition.build_apply(code);
        let mut visualizer = match visualize_filename.as_ref() {
            Some(visualize_filename) => {
                let visualizer = Visualizer::new(
                    Some(visualize_data_folder() + visualize_filename.as_str()),
                    code.get_positions(),
                    true,
                )
                .unwrap();
                print_visualize_link(visualize_filename.clone());
                Some(visualizer)
            }
            None => None,
        };
        let initializer = code.get_initializer();
        let partition_info = partition_config.info();
        let mut dual_module =
            DualModuleParallel::new_config(&initializer, &partition_info, DualModuleParallelConfig::default());
        let primal_config = PrimalModuleParallelConfig {
            debug_sequential: true,
            ..Default::default()
        };
        let mut primal_module = PrimalModuleParallel::new_config(&initializer, &partition_info, primal_config);
        code.set_defect_vertices(&defect_vertices);
        primal_module.parallel_solve_visualizer(&code.get_syndrome(), &dual_module, visualizer.as_mut());
        let useless_interface_ptr = DualModuleInterfacePtr::new_empty(); // don't actually use it
        let perfect_matching = primal_module.perfect_matching(&useless_interface_ptr, &mut dual_module);
        let mut subgraph_builder = SubGraphBuilder::new(&initializer);
        subgraph_builder.load_perfect_matching(&perfect_matching);
        let subgraph = subgraph_builder.get_subgraph();
        if let Some(visualizer) = visualizer.as_mut() {
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
        let sum_dual_variables = primal_module
            .units
            .last()
            .unwrap()
            .read_recursive()
            .interface_ptr
            .sum_dual_variables();
        assert_eq!(
            sum_dual_variables,
            subgraph_builder.total_weight(),
            "unmatched sum dual variables"
        );
        assert_eq!(sum_dual_variables, final_dual * 2, "unexpected final dual variable sum");
        (primal_module, dual_module)
    }

    pub fn example_partition_standard_syndrome(
        code: &mut dyn ExampleCode,
        visualize_filename: String,
        defect_vertices: Vec<VertexIndex>,
        re_index_syndrome: bool,
        final_dual: Weight,
        partition: impl ExamplePartition,
    ) -> (PrimalModuleParallel, DualModuleParallel<DualModuleSerial>) {
        example_partition_basic_standard_syndrome_optional_viz(
            code,
            Some(visualize_filename),
            defect_vertices,
            re_index_syndrome,
            final_dual,
            partition,
        )
    }

    /// test a simple case
    #[test]
    fn example_partition_basic_1() {
        // cargo test example_partition_basic_1 -- --nocapture
        let visualize_filename = "example_partition_basic_1.json".to_string();
        let defect_vertices = vec![39, 52, 63, 90, 100];
        let half_weight = 500;
        example_partition_standard_syndrome(
            &mut CodeCapacityPlanarCode::new(11, 0.1, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            9 * half_weight,
            NoPartition::new(),
        );
    }

    /// split into 2
    #[test]
    fn example_partition_basic_2() {
        // cargo test example_partition_basic_2 -- --nocapture
        let visualize_filename = "example_partition_basic_2.json".to_string();
        let defect_vertices = vec![39, 52, 63, 90, 100];
        let half_weight = 500;
        example_partition_standard_syndrome(
            &mut CodeCapacityPlanarCode::new(11, 0.1, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            9 * half_weight,
            CodeCapacityPlanarCodeVerticalPartitionHalf { d: 11, partition_row: 7 },
        );
    }

    /// split a repetition code into 2 parts
    #[test]
    fn example_partition_basic_3() {
        // cargo test example_partition_basic_3 -- --nocapture
        let visualize_filename = "example_partition_basic_3.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![2, 3, 4, 5, 6, 7, 8]; // indices are before the reorder
        let half_weight = 500;
        example_partition_standard_syndrome(
            &mut CodeCapacityRepetitionCode::new(11, 0.1, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            5 * half_weight,
            CodeCapacityRepetitionCodePartitionHalf {
                d: 11,
                partition_index: 6,
            },
        );
    }

    /// split into 4
    #[test]
    fn example_partition_basic_4() {
        // cargo test example_partition_basic_4 -- --nocapture
        let visualize_filename = "example_partition_basic_4.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![39, 52, 63, 90, 100]; // indices are before the reorder
        let half_weight = 500;
        example_partition_standard_syndrome(
            &mut CodeCapacityPlanarCode::new(11, 0.1, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            9 * half_weight,
            CodeCapacityPlanarCodeVerticalPartitionFour {
                d: 11,
                partition_row: 7,
                partition_column: 6,
            },
        );
    }

    /// phenomenological time axis split
    #[test]
    fn example_partition_basic_5() {
        // cargo test example_partition_basic_5 -- --nocapture
        let visualize_filename = "example_partition_basic_5.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![352, 365]; // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 10;
        example_partition_standard_syndrome(
            &mut PhenomenologicalPlanarCode::new(11, noisy_measurements, 0.1, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            2 * half_weight,
            PhenomenologicalPlanarCodeTimePartition::new(11, noisy_measurements, 2),
        );
    }

    /// a demo to show how partition works in phenomenological planar code
    #[test]
    fn example_partition_demo_1() {
        // cargo test example_partition_demo_1 -- --nocapture
        let visualize_filename = "example_partition_demo_1.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![
            57, 113, 289, 304, 305, 331, 345, 387, 485, 493, 528, 536, 569, 570, 587, 588, 696, 745, 801, 833, 834, 884,
            904, 940, 1152, 1184, 1208, 1258, 1266, 1344, 1413, 1421, 1481, 1489, 1490, 1546, 1690, 1733, 1740, 1746, 1796,
            1825, 1826, 1856, 1857, 1996, 2004, 2020, 2028, 2140, 2196, 2306, 2307, 2394, 2395, 2413, 2417, 2425, 2496,
            2497, 2731, 2739, 2818, 2874,
        ]; // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 51;
        example_partition_standard_syndrome(
            &mut PhenomenologicalPlanarCode::new(7, noisy_measurements, 0.005, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            35 * half_weight,
            PhenomenologicalPlanarCodeTimePartition::new(7, noisy_measurements, 3),
        );
    }

    /// a demo to show how partition works in circuit-level planar code
    #[test]
    fn example_partition_demo_2() {
        // cargo test example_partition_demo_2 -- --nocapture
        let visualize_filename = "example_partition_demo_2.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![
            57, 113, 289, 304, 305, 331, 345, 387, 485, 493, 528, 536, 569, 570, 587, 588, 696, 745, 801, 833, 834, 884,
            904, 940, 1152, 1184, 1208, 1258, 1266, 1344, 1413, 1421, 1481, 1489, 1490, 1546, 1690, 1733, 1740, 1746, 1796,
            1825, 1826, 1856, 1857, 1996, 2004, 2020, 2028, 2140, 2196, 2306, 2307, 2394, 2395, 2413, 2417, 2425, 2496,
            2497, 2731, 2739, 2818, 2874,
        ]; // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 51;
        example_partition_standard_syndrome(
            &mut CircuitLevelPlanarCode::new(7, noisy_measurements, 0.005, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            28980 / 2,
            PhenomenologicalPlanarCodeTimePartition::new(7, noisy_measurements, 3),
        );
    }

    /// demo of tree fusion
    #[test]
    fn example_partition_demo_3() {
        // cargo test example_partition_demo_3 -- --nocapture
        let visualize_filename = "example_partition_demo_3.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![
            57, 113, 289, 304, 305, 331, 345, 387, 485, 493, 528, 536, 569, 570, 587, 588, 696, 745, 801, 833, 834, 884,
            904, 940, 1152, 1184, 1208, 1258, 1266, 1344, 1413, 1421, 1481, 1489, 1490, 1546, 1690, 1733, 1740, 1746, 1796,
            1825, 1826, 1856, 1857, 1996, 2004, 2020, 2028, 2140, 2196, 2306, 2307, 2394, 2395, 2413, 2417, 2425, 2496,
            2497, 2731, 2739, 2818, 2874,
        ]; // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 51;
        example_partition_standard_syndrome(
            &mut PhenomenologicalPlanarCode::new(7, noisy_measurements, 0.005, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            35 * half_weight,
            PhenomenologicalPlanarCodeTimePartition::new_tree(7, noisy_measurements, 8, true, usize::MAX),
        );
    }

    /// demo of sequential fuse
    #[test]
    fn example_partition_demo_4() {
        // cargo test example_partition_demo_4 -- --nocapture
        let visualize_filename = "example_partition_demo_4.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![
            57, 113, 289, 304, 305, 331, 345, 387, 485, 493, 528, 536, 569, 570, 587, 588, 696, 745, 801, 833, 834, 884,
            904, 940, 1152, 1184, 1208, 1258, 1266, 1344, 1413, 1421, 1481, 1489, 1490, 1546, 1690, 1733, 1740, 1746, 1796,
            1825, 1826, 1856, 1857, 1996, 2004, 2020, 2028, 2140, 2196, 2306, 2307, 2394, 2395, 2413, 2417, 2425, 2496,
            2497, 2731, 2739, 2818, 2874,
        ]; // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 51;
        example_partition_standard_syndrome(
            &mut PhenomenologicalPlanarCode::new(7, noisy_measurements, 0.005, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            35 * half_weight,
            PhenomenologicalPlanarCodeTimePartition::new(7, noisy_measurements, 8),
        );
    }

    /// demo of tree + sequential fuse
    #[test]
    fn example_partition_demo_5() {
        // cargo test example_partition_demo_5 -- --nocapture
        let visualize_filename = "example_partition_demo_5.json".to_string();
        // reorder vertices to enable the partition;
        let defect_vertices = vec![
            57, 113, 289, 304, 305, 331, 345, 387, 485, 493, 528, 536, 569, 570, 587, 588, 696, 745, 801, 833, 834, 884,
            904, 940, 1152, 1184, 1208, 1258, 1266, 1344, 1413, 1421, 1481, 1489, 1490, 1546, 1690, 1733, 1740, 1746, 1796,
            1825, 1826, 1856, 1857, 1996, 2004, 2020, 2028, 2140, 2196, 2306, 2307, 2394, 2395, 2413, 2417, 2425, 2496,
            2497, 2731, 2739, 2818, 2874,
        ]; // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 51;
        example_partition_standard_syndrome(
            &mut PhenomenologicalPlanarCode::new(7, noisy_measurements, 0.005, half_weight),
            visualize_filename,
            defect_vertices,
            true,
            35 * half_weight,
            PhenomenologicalPlanarCodeTimePartition::new_tree(7, noisy_measurements, 8, true, 3),
        );
    }
}
