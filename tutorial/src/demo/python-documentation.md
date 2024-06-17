# Python Library Documentation 

* `CircuitLevelPlanarCode`
* `CodeCapacityPlanarCode(d=int, p=float, max_half_weight=int)`
    * `d`: code distance
    * `p`: physical error rate
    * `max_half_weight`: the library scales the edge weight such that the maximum weight is 2 * `max_half_weight`

* `CodeCapacityRepetitionCode` 
* `CodeCapacityRotatedCode`
* `CodeEdge`
* `CodeVertex`
* `DefectRange`
* `ErrorPatternReader`
* `IntermediateMatching`
* `LegacySolverSerial`
* `NodeRange(int, int)`
    * generates `IndexRange {range: [int, int]}`, used to denote the vertices in specific partition by index

* `PartitionConfig(vertex_num=int, partitions=[], fusion=[( , )])`
    * `vertex_num`: VertexNum, the number of vertices
    * `partitions`: Vec<VertexRange>, detailed plan of partitioning serial modules: each serial module possesses a list of vertices, including all interface vertices
    * `fusions`: Vec<(usize, usize)>, detailed plan of interfacing vertices

* `PartitionInfo`
* `PartitionUnitInfo`
* `PerfectMatching`
* `PhenomenologicalPlanarCode`
* `PhenomenologicalRotatedCode`
* `PyMut`
* `SolverDualParallel`
* `SolverErrorPatternLogger`
* `SolverInitializer`
* `SolverParallel(initializer, partition_info, primal_dual_config)`
    * `initializer` obtained from code 
    * `partition_info` obtained from `PartitionConfig`
    * `primal_dual_config`, a python object defined as 
    ```
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
    * `.solve(syndrome, visualizer)` 
* `SolverSerial`
* `SyndromePattern(defect_vertices=[], erasures=[], dynamic_weights=[])`
    * `defect_vertices`: Vec<VertexIndex>, the vertices corresponding to defect measurements
    * `erasures`: Vec<EdgeIndex>, the edges that experience erasures, i.e. known errors; note that erasure decoding can also be implemented using `dynamic_weights`, but for user convenience we keep this interface
    * `dynamic_weights`: Vec<(EdgeIndex, Weight)>, general dynamically weighted edges

* `SyndromeRange`
* `VertexRange`
* `VisualizePosition`
* `Visualizer`