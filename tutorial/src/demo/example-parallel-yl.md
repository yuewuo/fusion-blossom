# Example Parallel Configuration 

In this chapter, you will learn how about the configuration of graph partitions and different fusion plans as explained in the paper [1]. Make sure you understand the decoding and syndrome graph before proceeding. You can [download the complete code here](./example-parallel-yl.py).

In paper [1], Fusion Blossom solves the MWPM problem with a Divide and Conquer approach. Fusion Blossom *divides* the decoding problem into two sub-problems that can be solved, or "conquered", independently and *fuses* their solutions recursively to obtain the overall solution. This recursive division/fusion is represented as a full binary tree, denoted as a *fusion tree*. 



## Code Generation

```python
import fusion_blossom as fb

code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
```

Current available QEC codes and noise models are as follows:
* `CodeCapacityRepetitionCode(d, p)`, where `d` is code distance, `p` is physical error rate. 
* `CodeCapacityPlanarCode(d, p)`
* `PhenomenologicalPlanarCode(d, noisy_measurements, p)`
* `CircuitLevelPlanarCode(d, noisy_measurements, p)`

More details can be found in the *Construct Decoding Graph* section. 

## Partition Index Reordering

We can now proceed to define the partitions we would like to split our vertices into. In the example code, we split the vertices into 4 partitions (top-left, top-right, bottom-left, bottom-right). Note that we need to reassign the index to the vertices in all 4 partitions such that the vertices in each partition have continuous indexes. In the example python script, we defined the function `split_into_4_partitions_vertices()` to obtain the reordered vertices of the 4 partitions. We then reorder the vertices of the Code as follows. Note that we can use the visualization tool to see the vertex indexes after the reordering. 

```python
code.reorder_vertices(reordered_vertices)
# fb.helper.peek_code(code)  
```

We also calculate the reordered defect vertices using the `translated_defect_to_reordered()` helper function and sort them in ascending order. This is because `SyndromPattern` only takes in defect vertices whose indexes are in ascending order. 


## Partition Configuration

Now we can proceed to define the partition configuration as follows. 

```python
## Get initializer
initializer = code.get_initializer()

## Define Partition Configuration
partition_config = fb.PartitionConfig(initializer.vertex_num)
# The ranges below are the range of vertices (after reordering) of the 4 partitions we find the range below by looking at the partition visualization. For example, the range 0-36 is found by taking the min and the max indexes of the verticies in the top left parition
partition_config.partitions = [fb.NodeRange(0,36), # unit 0
                                fb.NodeRange(42,72), # unit 1
                                fb.NodeRange(84,108), # unit 2
                                fb.NodeRange(112,132) # unit 3
                                ]
partition_config.fusions = [(0, 1), (2, 3), (4, 5)] # refer to the Balanced tree in Figure 5 in [1]
partition_info = partition_config.info()
```

Note that we define `partition_config.partitions` and `partition_config.fusions` differently for different partition and fusion plans. The `NodeRange()` in the example code indicates the range of vertex indexes (after reordering) of the specific partition. This range can be obtained by finding the minimum and maximum value of the specific parition using the visualization tool `fb.helper.peek_code()`. 

There are mainly 3 categories of fusion plan as denoted by the 3 types of Fusion Trees. 
* Balanced Tree
    * Balanced tree has the shortest path from leaf to root, making it a good candidate for *Batch Decoding*. For Batch Decoding, the syndrome of all N rounds of measurement is available at the time of decoding, making the decoding latency the same as decoding time. Since decoding time is determined by the longest path from a leaf to the root given enouch parallel resources, *balanced tree* is a good candidate for fusion plan. 
* Linear Tree
    * Linear tree is preferable for *Streem Decoding*, where the decoding latency is subtaintially shorter than the decoding time as stream decoding starts as soon as rounds of measurement are ready for a leaf node. Therefore, besides the decoding time (represented by the paths between leaves and the root), we must also take into account the time when rounds of measurement for a leaf is ready. To minimize the decoding latency, one must balance the path length plus the ready time for all paths, allowing a shorter path for a later leaf. 
* Mixed Tree
    * Mixed tree is a continuum of trees between the 2 extreme cases of balanced tree and linear tree. To construct a mixed tree, one selects a certain height in the balanced tree, keeps balanced sub-trees below the height but constructs a linear tree above it. The higher this height, the smaller path difference between earlier and later leaves. For the balanced and linear trees, this mix height is root and leaf, respectively. We note that the mix height can be determined dynamically: the decoder can start with a balanced tree and switch to a linear tree to optimize the performance of the system.

More details can be found in the paper [1].

## Define Primal Dual Config

The configurations of primal and dual solvers are defined as follows: 

```python
primal_dual_config = {
    "dual": {
        "thread_pool_size": 1,
        "enable_parallel_execution": False
    },
    "primal": {
        "thread_pool_size": 1,
        "debug_sequential": True,
        "prioritize_base_partition": True, # by default enable because this is faster by placing time-consuming tasks in the front
        # "interleaving_base_fusion": usize::MAX, # starts interleaving base and fusion after this unit_index
        "pin_threads_to_cores": False # pin threads to cores to achieve the most stable result
        # "streaming_decode_mock_measure_interval": 
    }
}
```

## Define Syndrome Graph 

We define the syndrome graph with the reordered defect vertices as follows: 

```python
syndrome = fb.SyndromePattern(
    defect_vertices = new_defect_vertices,
)
```

## Visualize Result

The same process as in [Example QEC Codes Chapter](./example-qec-codes.md).

```python
visualizer = None
if True:  # change to False to disable visualizer for faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    positions = code.get_positions()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

## Initialize Solver
solver = fb.SolverParallel(initializer, partition_info, primal_dual_config)
## Run Solver
solver.solve(syndrome, visualizer)

## Print Minimum-Weight Parity Subgraph (MWPS)
subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")  # Vec<EdgeIndex>

if visualizer is not None:
    fb.print_visualize_link(filename=visualize_filename)
    fb.helper.open_visualizer(visualize_filename, open_browser=True)
```


[1] Wu, Yue, and Lin Zhong. "Fusion blossom: Fast mwpm decoders for qec." 2023 IEEE International Conference on Quantum Computing and Engineering (QCE). Vol. 1. IEEE, 2023.

