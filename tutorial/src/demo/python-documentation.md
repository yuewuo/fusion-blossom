# Python Library Documentation 

* `CircuitLevelPlanarCode`
* `CodeCapacityPlanarCode(d=int, p=float, max_half_weight=int)`
    * `d`: code distance
    * `p`: physical error rate
    * `max_half_weight`: the library scales the edge weight such that the maximum weight is 500 * 2 

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
* `SolverParallel`
* `SolverSerial`
* `SyndromePattern(defect_vertices=[], erasures=[], dynamic_weights=[])`
    * the vertices corresponding to defect measurements
    * `defect_vertices`: Vec<VertexIndex>, the edges that experience erasures, i.e. known errors;
    * `erasures`: Vec<EdgeIndex>, note that erasure decoding can also be implemented using `dynamic_weights`, but for user convenience we keep this interface
    * `dynamic_weights`: Vec<(EdgeIndex, Weight)>, general dynamically weighted edges

* `SyndromeRange`
* `VertexRange`
* `VisualizePosition`
* `Visualizer`