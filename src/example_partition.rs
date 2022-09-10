//! Example Partition
//! 
//! This module contains example partition for some of the example codes
//! 

use super::util::*;
use super::example::*;


pub trait ExamplePartition {

    /// customize partition, note that this process may re-order the vertices in `code`
    fn build_apply(&mut self, code: &mut Box<dyn ExampleCode>) -> PartitionConfig {
        // first apply reorder
        if let Some(reordered_vertices) = self.build_reordered_vertices(code) {
            code.reorder_vertices(&reordered_vertices);
        }
        self.build_partition(code)
    }

    fn re_index_syndrome_vertices(&mut self, code: &Box<dyn ExampleCode>, syndrome_vertices: &Vec<VertexIndex>) -> Vec<VertexIndex> {
        if let Some(reordered_vertices) = self.build_reordered_vertices(code) {
            translated_syndrome_to_reordered(&reordered_vertices, syndrome_vertices)
        } else {
            syndrome_vertices.clone()
        }
    }

    /// build reorder vertices 
    fn build_reordered_vertices(&mut self, _code: &Box<dyn ExampleCode>) -> Option<Vec<VertexIndex>> { None }

    /// build the partition, using the indices after reordered vertices
    fn build_partition(&mut self, code: &Box<dyn ExampleCode>) -> PartitionConfig;

}

/// no partition
pub struct NoPartition { }

impl NoPartition {
    pub fn new() -> Self {
        Self { }
    }
}

impl ExamplePartition for NoPartition {
    fn build_partition(&mut self, code: &Box<dyn ExampleCode>) -> PartitionConfig {
        PartitionConfig::default(code.vertex_num())
    }
}

/// partition into top half and bottom half
#[derive(Default)]
pub struct CodeCapacityPlanarCodeVerticalPartitionHalf {
    d: usize,
    /// the row of splitting: in the visualization tool, the top row is the 1st row, the bottom row is the d-th row
    partition_row: usize,
}

impl CodeCapacityPlanarCodeVerticalPartitionHalf {
    pub fn new(d: usize, partition_row: usize) -> Self {
        Self { d, partition_row }
    }
}

impl ExamplePartition for CodeCapacityPlanarCodeVerticalPartitionHalf {
    fn build_partition(&mut self, code: &Box<dyn ExampleCode>) -> PartitionConfig {
        let (d, partition_row) = (self.d, self.partition_row);
        assert_eq!(code.vertex_num(), d * (d + 1), "code size incompatible");
        let mut config = PartitionConfig::default(code.vertex_num());
        assert!(partition_row > 1 && partition_row < d);
        config.partitions = vec![
            VertexRange::new(0, (partition_row - 1) * (d + 1)),
            VertexRange::new(partition_row * (d + 1), d * (d + 1)),
        ];
        config.fusions = vec![
            (0, 1),
        ];
        config
    }
}

/// partition into 4 pieces: top left and right, bottom left and right
#[derive(Default)]
pub struct CodeCapacityPlanarCodeVerticalPartitionFour {
    d: usize,
    /// the row of splitting: in the visualization tool, the top row is the 1st row, the bottom row is the d-th row
    partition_row: usize,
    /// the row of splitting: in the visualization tool, the left (non-virtual) column is the 1st column, the right (non-virtual) column is the (d-1)-th column
    partition_column: usize,
}

impl CodeCapacityPlanarCodeVerticalPartitionFour {
    pub fn new(d: usize, partition_row: usize, partition_column: usize) -> Self {
        Self { d, partition_row, partition_column }
    }
}

impl ExamplePartition for CodeCapacityPlanarCodeVerticalPartitionFour {
    fn build_reordered_vertices(&mut self, code: &Box<dyn ExampleCode>) -> Option<Vec<VertexIndex>> {
        let (d, partition_row, partition_column) = (self.d, self.partition_row, self.partition_column);
        assert_eq!(code.vertex_num(), d * (d + 1), "code size incompatible");
        assert!(partition_row > 1 && partition_row < d);
        let mut reordered_vertices = vec![];
        let split_horizontal = partition_row - 1;
        let split_vertical = partition_column - 1;
        for i in 0..split_horizontal {  // left-top block
            for j in 0..split_vertical {
                reordered_vertices.push(i * (d+1) + j);
            }
            reordered_vertices.push(i * (d+1) + d);
        }
        for i in 0..split_horizontal {  // interface between the left-top block and the right-top block
            reordered_vertices.push(i * (d+1) + split_vertical);
        }
        for i in 0..split_horizontal {  // right-top block
            for j in (split_vertical+1)..d {
                reordered_vertices.push(i * (d+1) + j);
            }
        }
        {  // the big interface between top and bottom
            for j in 0..(d+1) {
                reordered_vertices.push(split_horizontal * (d+1) + j);
            }
        }
        for i in (split_horizontal+1)..d {  // left-bottom block
            for j in 0..split_vertical {
                reordered_vertices.push(i * (d+1) + j);
            }
            reordered_vertices.push(i * (d+1) + d);
        }
        for i in (split_horizontal+1)..d {  // interface between the left-bottom block and the right-bottom block
            reordered_vertices.push(i * (d+1) + split_vertical);
        }
        for i in (split_horizontal+1)..d {  // right-bottom block
            for j in (split_vertical+1)..d {
                reordered_vertices.push(i * (d+1) + j);
            }
        }
        Some(reordered_vertices)
    }
    fn build_partition(&mut self, _code: &Box<dyn ExampleCode>) -> PartitionConfig {
        let (d, partition_row, partition_column) = (self.d, self.partition_row, self.partition_column);
        let mut config = PartitionConfig::default(d * (d + 1));
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
        config.fusions = vec![
            (0, 1),
            (2, 3),
            (4, 5),
        ];
        config
    }
}

/// partition into top half and bottom half
#[derive(Default)]
pub struct CodeCapacityRepetitionCodePartitionHalf {
    d: usize,
    /// the position of splitting: in the visualization tool, the left (non-virtual) vertex is the 1st column, the right (non-virtual) vertex is the (d-1)-th column
    partition_index: usize,
}

impl CodeCapacityRepetitionCodePartitionHalf {
    pub fn new(d: usize, partition_index: usize) -> Self {
        Self { d, partition_index }
    }
}

impl ExamplePartition for CodeCapacityRepetitionCodePartitionHalf {
    fn build_reordered_vertices(&mut self, code: &Box<dyn ExampleCode>) -> Option<Vec<VertexIndex>> {
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
    fn build_partition(&mut self, _code: &Box<dyn ExampleCode>) -> PartitionConfig {
        let (d, partition_index) = (self.d, self.partition_index);
        let mut config = PartitionConfig::default(d + 1);
        config.partitions = vec![
            VertexRange::new(0, partition_index),
            VertexRange::new(partition_index + 1, d + 1),
        ];
        config.fusions = vec![
            (0, 1),
        ];
        config
    }
}

/// evenly partition along the time axis
pub struct PhenomenologicalPlanarCodeTimePartition {
    d: usize,
    noisy_measurements: usize,
    /// the number of partition
    partition_num: usize,
}

impl PhenomenologicalPlanarCodeTimePartition {
    pub fn new(d: usize, noisy_measurements: usize, partition_num: usize) -> Self {
        Self { d, noisy_measurements, partition_num }
    }
}

impl ExamplePartition for PhenomenologicalPlanarCodeTimePartition {
    fn build_partition(&mut self, code: &Box<dyn ExampleCode>) -> PartitionConfig {
        let (d, noisy_measurements, partition_num) = (self.d, self.noisy_measurements, self.partition_num);
        let round_vertex_num = d * (d + 1);
        let vertex_num = round_vertex_num * (noisy_measurements + 1);
        assert_eq!(code.vertex_num(), vertex_num, "code size incompatible");
        assert!(partition_num >= 1 && partition_num <= noisy_measurements + 1);
        let partition_length = (noisy_measurements + 1) / partition_num;
        let mut config = PartitionConfig::default(vertex_num);
        config.partitions.clear();
        for partition_index in 0..partition_num {
            if partition_index < partition_num - 1 {
                config.partitions.push(VertexRange::new_length(
                    partition_index * partition_length * round_vertex_num, (partition_length - 1) * round_vertex_num
                ));
            } else {
                config.partitions.push(VertexRange::new(partition_index * partition_length * round_vertex_num, vertex_num));
            }
        }
        config.fusions.clear();
        for unit_index in partition_num..(2 * partition_num - 1) {
            if unit_index == partition_num {
                config.fusions.push((0, 1));
            } else {
                config.fusions.push((unit_index - 1, unit_index - partition_num + 1));
            }
        }
        config
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use super::super::visualize::*;
    use super::super::primal_module_parallel::*;
    use super::super::dual_module_parallel::*;
    use super::super::dual_module::*;
    use super::super::dual_module_serial::*;
    use std::sync::Arc;

    pub fn example_partition_basic_standard_syndrome_optional_viz(mut code: Box<dyn ExampleCode>, visualize_filename: Option<String>
            , mut syndrome_vertices: Vec<VertexIndex>, re_index_syndrome: bool, final_dual: Weight, mut partition: impl ExamplePartition)
            -> (DualModuleInterface, PrimalModuleParallel, DualModuleParallel<DualModuleSerial>) {
        println!("{syndrome_vertices:?}");
        if re_index_syndrome {
            syndrome_vertices = partition.re_index_syndrome_vertices(&code, &syndrome_vertices);
        }
        let partition_config = partition.build_apply(&mut code);
        let mut visualizer = match visualize_filename.as_ref() {
            Some(visualize_filename) => {
                let mut visualizer = Visualizer::new(Some(visualize_data_folder() + visualize_filename.as_str())).unwrap();
                visualizer.set_positions(code.get_positions(), true);  // automatic center all nodes
                print_visualize_link(&visualize_filename);
                Some(visualizer)
            }, None => None
        };
        let initializer = code.get_initializer();
        let partition_info = partition_config.into_info();
        let mut dual_module = DualModuleParallel::new_config(&initializer, Arc::clone(&partition_info), DualModuleParallelConfig::default());
        let mut primal_config = PrimalModuleParallelConfig::default();
        primal_config.debug_sequential = true;
        let mut primal_module = PrimalModuleParallel::new_config(&initializer, Arc::clone(&partition_info), primal_config);
        code.set_syndrome_vertices(&syndrome_vertices);
        let interface = primal_module.parallel_solve_visualizer(&code.get_syndrome(), &mut dual_module, visualizer.as_mut());
        assert_eq!(interface.sum_dual_variables, final_dual * 2, "unexpected final dual variable sum");
        (interface, primal_module, dual_module)
    }

    pub fn example_partition_standard_syndrome(code: Box<dyn ExampleCode>, visualize_filename: String, syndrome_vertices: Vec<VertexIndex>
            , re_index_syndrome: bool, final_dual: Weight, partition: impl ExamplePartition)
            -> (DualModuleInterface, PrimalModuleParallel, DualModuleParallel<DualModuleSerial>) {
        example_partition_basic_standard_syndrome_optional_viz(code, Some(visualize_filename), syndrome_vertices, re_index_syndrome
            , final_dual, partition)
    }

    /// test a simple case
    #[test]
    fn example_partition_basic_1() {  // cargo test example_partition_basic_1 -- --nocapture
        let visualize_filename = format!("example_partition_basic_1.json");
        let syndrome_vertices = vec![39, 52, 63, 90, 100];
        let half_weight = 500;
        example_partition_standard_syndrome(Box::new(CodeCapacityPlanarCode::new(11, 0.1, half_weight)), visualize_filename
            , syndrome_vertices, true, 9 * half_weight, NoPartition::new());
    }

    /// split into 2
    #[test]
    fn example_partition_basic_2() {  // cargo test example_partition_basic_2 -- --nocapture
        let visualize_filename = format!("example_partition_basic_2.json");
        let syndrome_vertices = vec![39, 52, 63, 90, 100];
        let half_weight = 500;
        example_partition_standard_syndrome(Box::new(CodeCapacityPlanarCode::new(11, 0.1, half_weight)), visualize_filename
            , syndrome_vertices, true, 9 * half_weight, CodeCapacityPlanarCodeVerticalPartitionHalf{ d: 11, partition_row: 7 });
    }

    /// split a repetition code into 2 parts
    #[test]
    fn example_partition_basic_3() {  // cargo test example_partition_basic_3 -- --nocapture
        let visualize_filename = format!("example_partition_basic_3.json");
        // reorder vertices to enable the partition;
        let syndrome_vertices = vec![2, 3, 4, 5, 6, 7, 8];  // indices are before the reorder
        let half_weight = 500;
        example_partition_standard_syndrome(Box::new(CodeCapacityRepetitionCode::new(11, 0.1, half_weight)), visualize_filename
            , syndrome_vertices, true, 5 * half_weight, CodeCapacityRepetitionCodePartitionHalf{ d: 11, partition_index: 6 });
    }

    /// split into 4
    #[test]
    fn example_partition_basic_4() {  // cargo test example_partition_basic_4 -- --nocapture
        let visualize_filename = format!("example_partition_basic_4.json");
        // reorder vertices to enable the partition;
        let syndrome_vertices = vec![39, 52, 63, 90, 100];  // indices are before the reorder
        let half_weight = 500;
        example_partition_standard_syndrome(Box::new(CodeCapacityPlanarCode::new(11, 0.1, half_weight)), visualize_filename
            , syndrome_vertices, true, 9 * half_weight, CodeCapacityPlanarCodeVerticalPartitionFour{ d: 11, partition_row: 7, partition_column: 6 });
    }

    /// phenomenological time axis split
    #[test]
    fn example_partition_basic_5() {  // cargo test example_partition_basic_5 -- --nocapture
        let visualize_filename = format!("example_partition_basic_5.json");
        // reorder vertices to enable the partition;
        let syndrome_vertices = vec![352, 365];  // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 10;
        example_partition_standard_syndrome(Box::new(PhenomenologicalPlanarCode::new(11, noisy_measurements, 0.1, half_weight)), visualize_filename
            , syndrome_vertices, true, 2 * half_weight, PhenomenologicalPlanarCodeTimePartition{ d: 11, noisy_measurements, partition_num: 2 });
    }

    /// a demo to show how partition works in phenomenological planar code
    #[test]
    fn example_partition_demo_1() {  // cargo test example_partition_demo_1 -- --nocapture
        let visualize_filename = format!("example_partition_demo_1.json");
        // reorder vertices to enable the partition;
        let syndrome_vertices = vec![57, 113, 289, 304, 305, 331, 345, 387, 485, 493, 528, 536, 569, 570, 587, 588, 696, 745, 801, 833, 834, 884, 904, 940, 1152, 1184, 1208, 1258, 1266, 1344, 1413, 1421, 1481, 1489, 1490, 1546, 1690, 1733, 1740, 1746, 1796, 1825, 1826, 1856, 1857, 1996, 2004, 2020, 2028, 2140, 2196, 2306, 2307, 2394, 2395, 2413, 2417, 2425, 2496, 2497, 2731, 2739, 2818, 2874];  // indices are before the reorder
        let half_weight = 500;
        let noisy_measurements = 51;
        example_partition_standard_syndrome(Box::new(PhenomenologicalPlanarCode::new(7, noisy_measurements, 0.005, half_weight)), visualize_filename
            , syndrome_vertices, true, 35 * half_weight, PhenomenologicalPlanarCodeTimePartition{ d: 7, noisy_measurements, partition_num: 3 });
    }

}
