use super::mwpm_solver::PrimalDualSolver;
use super::pointers::*;
use super::rand_xoshiro;
use crate::rand_xoshiro::rand_core::RngCore;
#[cfg(feature = "python_binding")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs::File;
use std::io::prelude::*;
use std::time::Instant;

cfg_if::cfg_if! {
    if #[cfg(feature="i32_weight")] {
        /// use i32 to store weight to be compatible with blossom V library (c_int)
        pub type Weight = i32;
    } else {
        pub type Weight = isize;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature="u32_index")] {
        // use u32 to store index, for less memory usage
        pub type EdgeIndex = u32;
        pub type VertexIndex = u32;  // the vertex index in the decoding graph
        pub type NodeIndex = VertexIndex;
        pub type DefectIndex = VertexIndex;
        pub type VertexNodeIndex = VertexIndex;  // must be same as VertexIndex, NodeIndex, DefectIndex
        pub type VertexNum = VertexIndex;
        pub type NodeNum = VertexIndex;
    } else {
        pub type EdgeIndex = usize;
        pub type VertexIndex = usize;
        pub type NodeIndex = VertexIndex;
        pub type DefectIndex = VertexIndex;
        pub type VertexNodeIndex = VertexIndex;  // must be same as VertexIndex, NodeIndex, DefectIndex
        pub type VertexNum = VertexIndex;
        pub type NodeNum = VertexIndex;
    }
}

#[cfg(feature = "python_binding")]
macro_rules! bind_trait_python_json {
    ($struct_name:ident) => {
        #[pymethods]
        impl $struct_name {
            #[pyo3(name = "to_json")]
            fn python_to_json(&self) -> PyResult<String> {
                serde_json::to_string(self).map_err(|err| pyo3::exceptions::PyTypeError::new_err(format!("{err:?}")))
            }
            #[staticmethod]
            #[pyo3(name = "from_json")]
            fn python_from_json(value: String) -> PyResult<Self> {
                serde_json::from_str(value.as_str())
                    .map_err(|err| pyo3::exceptions::PyTypeError::new_err(format!("{err:?}")))
            }
        }
    };
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverInitializer {
    /// the number of vertices
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertex_num: VertexNum,
    /// weighted edges, where vertex indices are within the range [0, vertex_num)
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>,
    /// the virtual vertices
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub virtual_vertices: Vec<VertexIndex>,
}

#[cfg(feature = "python_binding")]
bind_trait_python_json! {SolverInitializer}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct SyndromePattern {
    /// the vertices corresponding to defect measurements
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub defect_vertices: Vec<VertexIndex>,
    /// the edges that experience erasures, i.e. known errors;
    /// note that erasure decoding can also be implemented using `dynamic_weights`,
    /// but for user convenience we keep this interface
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    #[serde(default = "default_erasures")]
    pub erasures: Vec<EdgeIndex>,
    /// general dynamically weighted edges
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    #[serde(default = "default_dynamic_weights")]
    pub dynamic_weights: Vec<(EdgeIndex, Weight)>,
}

pub fn default_dynamic_weights() -> Vec<(EdgeIndex, Weight)> {
    vec![]
}

pub fn default_erasures() -> Vec<EdgeIndex> {
    vec![]
}

impl SyndromePattern {
    pub fn new(defect_vertices: Vec<VertexIndex>, erasures: Vec<EdgeIndex>) -> Self {
        Self {
            defect_vertices,
            erasures,
            dynamic_weights: vec![],
        }
    }
    pub fn new_dynamic_weights(
        defect_vertices: Vec<VertexIndex>,
        erasures: Vec<EdgeIndex>,
        dynamic_weights: Vec<(EdgeIndex, Weight)>,
    ) -> Self {
        Self {
            defect_vertices,
            erasures,
            dynamic_weights,
        }
    }
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl SyndromePattern {
    #[cfg_attr(feature = "python_binding", new)]
    #[cfg_attr(feature = "python_binding", pyo3(signature = (defect_vertices=vec![], erasures=vec![], dynamic_weights=vec![], syndrome_vertices=None)))]
    pub fn py_new(
        mut defect_vertices: Vec<VertexIndex>,
        erasures: Vec<EdgeIndex>,
        dynamic_weights: Vec<(EdgeIndex, Weight)>,
        syndrome_vertices: Option<Vec<VertexIndex>>,
    ) -> Self {
        if let Some(syndrome_vertices) = syndrome_vertices {
            assert!(
                defect_vertices.is_empty(),
                "do not pass both `syndrome_vertices` and `defect_vertices` since they're aliasing"
            );
            defect_vertices = syndrome_vertices;
        }
        assert!(
            erasures.is_empty() || dynamic_weights.is_empty(),
            "erasures and dynamic_weights cannot be provided at the same time"
        );
        Self::new_dynamic_weights(defect_vertices, erasures, dynamic_weights)
    }
    #[cfg_attr(feature = "python_binding", staticmethod)]
    pub fn new_vertices(defect_vertices: Vec<VertexIndex>) -> Self {
        Self::new(defect_vertices, vec![])
    }
    #[cfg_attr(feature = "python_binding", staticmethod)]
    pub fn new_empty() -> Self {
        Self::new(vec![], vec![])
    }
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

/// an efficient representation of partitioned vertices and erasures when they're ordered
#[derive(Debug, Clone, Serialize)]
pub struct PartitionedSyndromePattern<'a> {
    /// the original syndrome pattern to be partitioned
    pub syndrome_pattern: &'a SyndromePattern,
    /// the defect range of this partition: it must be continuous if the defect vertices are ordered
    pub whole_defect_range: DefectRange,
}

impl<'a> PartitionedSyndromePattern<'a> {
    pub fn new(syndrome_pattern: &'a SyndromePattern) -> Self {
        assert!(
            syndrome_pattern.erasures.is_empty(),
            "erasure partition not supported yet;
        even if the edges in the erasure is well ordered, they may not be able to be represented as
        a single range simply because the partition is vertex-based. need more consideration"
        );
        Self {
            syndrome_pattern,
            whole_defect_range: DefectRange::new(0, syndrome_pattern.defect_vertices.len() as DefectIndex),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct IndexRange {
    pub range: [VertexNodeIndex; 2],
}

// just to distinguish them in code, essentially nothing different
pub type VertexRange = IndexRange;
pub type NodeRange = IndexRange;
pub type DefectRange = IndexRange;

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl IndexRange {
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new(start: VertexNodeIndex, end: VertexNodeIndex) -> Self {
        debug_assert!(end >= start, "invalid range [{}, {})", start, end);
        Self { range: [start, end] }
    }
    #[cfg_attr(feature = "python_binding", staticmethod)]
    pub fn new_length(start: VertexNodeIndex, length: VertexNodeIndex) -> Self {
        Self::new(start, start + length)
    }
    pub fn is_empty(&self) -> bool {
        self.range[1] == self.range[0]
    }
    #[allow(clippy::unnecessary_cast)]
    pub fn len(&self) -> usize {
        (self.range[1] - self.range[0]) as usize
    }
    pub fn start(&self) -> VertexNodeIndex {
        self.range[0]
    }
    pub fn end(&self) -> VertexNodeIndex {
        self.range[1]
    }
    pub fn append_by(&mut self, append_count: VertexNodeIndex) {
        self.range[1] += append_count;
    }
    pub fn bias_by(&mut self, bias: VertexNodeIndex) {
        self.range[0] += bias;
        self.range[1] += bias;
    }
    pub fn sanity_check(&self) {
        assert!(self.start() <= self.end(), "invalid vertex range {:?}", self);
    }
    pub fn contains(&self, vertex_index: VertexNodeIndex) -> bool {
        vertex_index >= self.start() && vertex_index < self.end()
    }
    /// fuse two ranges together, returning (the whole range, the interfacing range)
    pub fn fuse(&self, other: &Self) -> (Self, Self) {
        self.sanity_check();
        other.sanity_check();
        assert!(self.range[1] <= other.range[0], "only lower range can fuse higher range");
        (
            Self::new(self.range[0], other.range[1]),
            Self::new(self.range[1], other.range[0]),
        )
    }
    #[cfg(feature = "python_binding")]
    #[pyo3(name = "contains_any")]
    pub fn python_contains_any(&self, vertex_indices: Vec<VertexNodeIndex>) -> bool {
        self.contains_any(&vertex_indices)
    }
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl IndexRange {
    pub fn iter(&self) -> std::ops::Range<VertexNodeIndex> {
        self.range[0]..self.range[1]
    }
    pub fn contains_any(&self, vertex_indices: &[VertexNodeIndex]) -> bool {
        for vertex_index in vertex_indices.iter() {
            if self.contains(*vertex_index) {
                return true;
            }
        }
        false
    }
}

/// a general partition unit that could contain mirrored vertices
#[derive(Debug, Clone)]
pub struct PartitionUnit {
    /// unit index
    pub unit_index: usize,
    /// whether it's enabled; when disabled, the mirrored vertices behaves just like virtual vertices
    pub enabled: bool,
}

pub type PartitionUnitPtr = ArcManualSafeLock<PartitionUnit>;
pub type PartitionUnitWeak = WeakManualSafeLock<PartitionUnit>;

impl std::fmt::Debug for PartitionUnitPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let partition_unit = self.read_recursive();
        write!(
            f,
            "{}{}",
            if partition_unit.enabled { "E" } else { "D" },
            partition_unit.unit_index
        )
    }
}

impl std::fmt::Debug for PartitionUnitWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

/// user input partition configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct PartitionConfig {
    /// the number of vertices
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertex_num: VertexNum,
    /// detailed plan of partitioning serial modules: each serial module possesses a list of vertices, including all interface vertices
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub partitions: Vec<VertexRange>,
    /// detailed plan of interfacing vertices
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub fusions: Vec<(usize, usize)>,
}

#[cfg(feature = "python_binding")]
bind_trait_python_json! {PartitionConfig}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl PartitionConfig {
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new(vertex_num: VertexNum) -> Self {
        Self {
            vertex_num,
            partitions: vec![VertexRange::new(0, vertex_num as VertexIndex)],
            fusions: vec![],
        }
    }

    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn info(&self) -> PartitionInfo {
        assert!(!self.partitions.is_empty(), "at least one partition must exist");
        let mut whole_ranges = vec![];
        let mut owning_ranges = vec![];
        for &partition in self.partitions.iter() {
            partition.sanity_check();
            assert!(
                partition.end() <= self.vertex_num as VertexIndex,
                "invalid vertex index {} in partitions",
                partition.end()
            );
            whole_ranges.push(partition);
            owning_ranges.push(partition);
        }
        let unit_count = self.partitions.len() + self.fusions.len();
        let mut parents: Vec<Option<usize>> = (0..unit_count).map(|_| None).collect();
        for (fusion_index, (left_index, right_index)) in self.fusions.iter().enumerate() {
            let unit_index = fusion_index + self.partitions.len();
            assert!(
                *left_index < unit_index,
                "dependency wrong, {} depending on {}",
                unit_index,
                left_index
            );
            assert!(
                *right_index < unit_index,
                "dependency wrong, {} depending on {}",
                unit_index,
                right_index
            );
            assert!(parents[*left_index].is_none(), "cannot fuse {} twice", left_index);
            assert!(parents[*right_index].is_none(), "cannot fuse {} twice", right_index);
            parents[*left_index] = Some(unit_index);
            parents[*right_index] = Some(unit_index);
            // fusing range
            let (whole_range, interface_range) = whole_ranges[*left_index].fuse(&whole_ranges[*right_index]);
            whole_ranges.push(whole_range);
            owning_ranges.push(interface_range);
        }
        // check that all nodes except for the last one has been merged
        for (unit_index, parent) in parents.iter().enumerate().take(unit_count - 1) {
            assert!(parent.is_some(), "found unit {} without being fused", unit_index);
        }
        // check that the final node has the full range
        let last_unit_index = self.partitions.len() + self.fusions.len() - 1;
        assert!(
            whole_ranges[last_unit_index].start() == 0,
            "final range not covering all vertices {:?}",
            whole_ranges[last_unit_index]
        );
        assert!(
            whole_ranges[last_unit_index].end() == self.vertex_num as VertexIndex,
            "final range not covering all vertices {:?}",
            whole_ranges[last_unit_index]
        );
        // construct partition info
        let mut partition_unit_info: Vec<_> = (0..self.partitions.len() + self.fusions.len())
            .map(|i| PartitionUnitInfo {
                whole_range: whole_ranges[i],
                owning_range: owning_ranges[i],
                children: if i >= self.partitions.len() {
                    Some(self.fusions[i - self.partitions.len()])
                } else {
                    None
                },
                parent: parents[i],
                leaves: if i < self.partitions.len() { vec![i] } else { vec![] },
                descendants: BTreeSet::new(),
            })
            .collect();
        // build descendants
        for (fusion_index, (left_index, right_index)) in self.fusions.iter().enumerate() {
            let unit_index = fusion_index + self.partitions.len();
            let mut leaves = vec![];
            leaves.extend(partition_unit_info[*left_index].leaves.iter());
            leaves.extend(partition_unit_info[*right_index].leaves.iter());
            partition_unit_info[unit_index].leaves.extend(leaves.iter());
            let mut descendants = vec![];
            descendants.push(*left_index);
            descendants.push(*right_index);
            descendants.extend(partition_unit_info[*left_index].descendants.iter());
            descendants.extend(partition_unit_info[*right_index].descendants.iter());
            partition_unit_info[unit_index].descendants.extend(descendants.iter());
        }
        let mut vertex_to_owning_unit: Vec<_> = (0..self.vertex_num).map(|_| usize::MAX).collect();
        for (unit_index, unit_range) in partition_unit_info.iter().map(|x| x.owning_range).enumerate() {
            for vertex_index in unit_range.iter() {
                vertex_to_owning_unit[vertex_index as usize] = unit_index;
            }
        }
        PartitionInfo {
            config: self.clone(),
            units: partition_unit_info,
            vertex_to_owning_unit,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct PartitionInfo {
    /// the initial configuration that creates this info
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub config: PartitionConfig,
    /// individual info of each unit
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub units: Vec<PartitionUnitInfo>,
    /// the mapping from vertices to the owning unit: serial unit (holding real vertices) as well as parallel units (holding interfacing vertices);
    /// used for loading syndrome to the holding units
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub vertex_to_owning_unit: Vec<usize>,
}

#[cfg(feature = "python_binding")]
bind_trait_python_json! {PartitionInfo}

#[cfg_attr(feature = "python_binding", pymethods)]
impl PartitionInfo {
    /// split a sequence of syndrome into multiple parts, each corresponds to a unit;
    /// this is a slow method and should only be used when the syndrome pattern is not well-ordered
    #[allow(clippy::unnecessary_cast)]
    pub fn partition_syndrome_unordered(&self, syndrome_pattern: &SyndromePattern) -> Vec<SyndromePattern> {
        let mut partitioned_syndrome: Vec<_> = (0..self.units.len()).map(|_| SyndromePattern::new_empty()).collect();
        for defect_vertex in syndrome_pattern.defect_vertices.iter() {
            let unit_index = self.vertex_to_owning_unit[*defect_vertex as usize];
            partitioned_syndrome[unit_index].defect_vertices.push(*defect_vertex);
        }
        // TODO: partition edges
        partitioned_syndrome
    }

    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl<'a> PartitionedSyndromePattern<'a> {
    /// partition the syndrome pattern into 2 partitioned syndrome pattern and my whole range
    #[allow(clippy::unnecessary_cast)]
    pub fn partition(&self, partition_unit_info: &PartitionUnitInfo) -> (Self, (Self, Self)) {
        // first binary search the start of owning defect vertices
        let owning_start_index = {
            let mut left_index = self.whole_defect_range.start();
            let mut right_index = self.whole_defect_range.end();
            while left_index != right_index {
                let mid_index = (left_index + right_index) / 2;
                let mid_defect_vertex = self.syndrome_pattern.defect_vertices[mid_index as usize];
                if mid_defect_vertex < partition_unit_info.owning_range.start() {
                    left_index = mid_index + 1;
                } else {
                    right_index = mid_index;
                }
            }
            left_index
        };
        // second binary search the end of owning defect vertices
        let owning_end_index = {
            let mut left_index = self.whole_defect_range.start();
            let mut right_index = self.whole_defect_range.end();
            while left_index != right_index {
                let mid_index = (left_index + right_index) / 2;
                let mid_defect_vertex = self.syndrome_pattern.defect_vertices[mid_index as usize];
                if mid_defect_vertex < partition_unit_info.owning_range.end() {
                    left_index = mid_index + 1;
                } else {
                    right_index = mid_index;
                }
            }
            left_index
        };
        (
            Self {
                syndrome_pattern: self.syndrome_pattern,
                whole_defect_range: DefectRange::new(owning_start_index, owning_end_index),
            },
            (
                Self {
                    syndrome_pattern: self.syndrome_pattern,
                    whole_defect_range: DefectRange::new(self.whole_defect_range.start(), owning_start_index),
                },
                Self {
                    syndrome_pattern: self.syndrome_pattern,
                    whole_defect_range: DefectRange::new(owning_end_index, self.whole_defect_range.end()),
                },
            ),
        )
    }

    #[allow(clippy::unnecessary_cast)]
    pub fn expand(&self) -> SyndromePattern {
        let mut defect_vertices = Vec::with_capacity(self.whole_defect_range.len());
        for defect_index in self.whole_defect_range.iter() {
            defect_vertices.push(self.syndrome_pattern.defect_vertices[defect_index as usize]);
        }
        SyndromePattern::new(defect_vertices, vec![])
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pyclass)]
pub struct PartitionUnitInfo {
    /// the whole range of units
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub whole_range: VertexRange,
    /// the owning range of units, meaning vertices inside are exclusively belonging to the unit
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub owning_range: VertexRange,
    /// left and right
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub children: Option<(usize, usize)>,
    /// parent dual module
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub parent: Option<usize>,
    /// all the leaf dual modules
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub leaves: Vec<usize>,
    /// all the descendants
    #[cfg_attr(feature = "python_binding", pyo3(get, set))]
    pub descendants: BTreeSet<usize>,
}

#[cfg(feature = "python_binding")]
bind_trait_python_json! {PartitionUnitInfo}

#[cfg(feature = "python_binding")]
#[pymethods]
impl PartitionUnitInfo {
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

#[derive(Debug, Clone)]
pub struct PartitionedSolverInitializer {
    /// unit index
    pub unit_index: usize,
    /// the number of all vertices (including those partitioned into other serial modules)
    pub vertex_num: VertexNum,
    /// the number of all edges (including those partitioned into other serial modules)
    pub edge_num: usize,
    /// vertices exclusively owned by this partition; this part must be a continuous range
    pub owning_range: VertexRange,
    /// applicable when all the owning vertices are partitioned (i.e. this belongs to a fusion unit)
    pub owning_interface: Option<PartitionUnitWeak>,
    /// if applicable, parent interface comes first, then the grandparent interface, ... note that some ancestor might be skipped because it has no mirrored vertices;
    /// we skip them because if the partition is in a chain, most of them would only have to know two interfaces on the left and on the right; nothing else necessary.
    /// (unit_index, list of vertices owned by this ancestor unit and should be mirrored at this partition and whether it's virtual)
    pub interfaces: Vec<(PartitionUnitWeak, Vec<(VertexIndex, bool)>)>,
    /// weighted edges, where the first vertex index is within the range [vertex_index_bias, vertex_index_bias + vertex_num) and
    /// the second is either in [vertex_index_bias, vertex_index_bias + vertex_num) or inside
    pub weighted_edges: Vec<(VertexIndex, VertexIndex, Weight, EdgeIndex)>,
    /// the virtual vertices
    pub virtual_vertices: Vec<VertexIndex>,
}

/// perform index transformation
#[allow(clippy::unnecessary_cast)]
pub fn build_old_to_new(reordered_vertices: &[VertexIndex]) -> Vec<Option<VertexIndex>> {
    let mut old_to_new: Vec<Option<VertexIndex>> = (0..reordered_vertices.len()).map(|_| None).collect();
    for (new_index, old_index) in reordered_vertices.iter().enumerate() {
        assert_eq!(old_to_new[*old_index as usize], None, "duplicate vertex found {}", old_index);
        old_to_new[*old_index as usize] = Some(new_index as VertexIndex);
    }
    old_to_new
}

/// translate defect vertices into the current new index given reordered_vertices
#[allow(clippy::unnecessary_cast)]
pub fn translated_defect_to_reordered(
    reordered_vertices: &[VertexIndex],
    old_defect_vertices: &[VertexIndex],
) -> Vec<VertexIndex> {
    let old_to_new = build_old_to_new(reordered_vertices);
    old_defect_vertices
        .iter()
        .map(|old_index| old_to_new[*old_index as usize].unwrap())
        .collect()
}

#[cfg_attr(feature = "python_binding", cfg_eval)]
#[cfg_attr(feature = "python_binding", pymethods)]
impl SolverInitializer {
    #[cfg_attr(feature = "python_binding", new)]
    pub fn new(
        vertex_num: VertexNum,
        weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>,
        virtual_vertices: Vec<VertexIndex>,
    ) -> SolverInitializer {
        SolverInitializer {
            vertex_num,
            weighted_edges,
            virtual_vertices,
        }
    }
    #[cfg(feature = "python_binding")]
    fn __repr__(&self) -> String {
        format!("{:?}", self)
    }
}

impl SolverInitializer {
    #[allow(clippy::unnecessary_cast)]
    pub fn syndrome_of(&self, subgraph: &[EdgeIndex]) -> BTreeSet<VertexIndex> {
        let mut defects = BTreeSet::new();
        for edge_index in subgraph {
            let (left, right, _weight) = self.weighted_edges[*edge_index as usize];
            for vertex_index in [left, right] {
                if defects.contains(&vertex_index) {
                    defects.remove(&vertex_index);
                } else {
                    defects.insert(vertex_index);
                }
            }
        }
        // remove virtual vertices
        for vertex_index in self.virtual_vertices.iter() {
            defects.remove(vertex_index);
        }
        defects
    }
}

/// timestamp type determines how many fast clear before a hard clear is required, see [`FastClear`]
pub type FastClearTimestamp = usize;

#[allow(dead_code)]
/// use Xoshiro256StarStar for deterministic random number generator
pub type DeterministicRng = rand_xoshiro::Xoshiro256StarStar;

pub trait F64Rng {
    fn next_f64(&mut self) -> f64;
}

impl F64Rng for DeterministicRng {
    fn next_f64(&mut self) -> f64 {
        f64::from_bits(0x3FF << 52 | self.next_u64() >> 12) - 1.
    }
}

/// record the decoding time of multiple syndrome patterns
pub struct BenchmarkProfiler {
    /// each record corresponds to a different syndrome pattern
    pub records: Vec<BenchmarkProfilerEntry>,
    /// summation of all decoding time
    pub sum_round_time: f64,
    /// syndrome count
    pub sum_syndrome: usize,
    /// noisy measurement round
    pub noisy_measurements: VertexNum,
    /// the file to output the profiler results
    pub benchmark_profiler_output: Option<File>,
}

impl BenchmarkProfiler {
    pub fn new(noisy_measurements: VertexNum, detail_log_file: Option<(String, &PartitionInfo)>) -> Self {
        let benchmark_profiler_output = detail_log_file.map(|(filename, partition_info)| {
            let mut file = File::create(filename).unwrap();
            file.write_all(serde_json::to_string(&partition_info.config).unwrap().as_bytes())
                .unwrap();
            file.write_all(b"\n").unwrap();
            file.write_all(
                serde_json::to_string(&json!({
                    "noisy_measurements": noisy_measurements,
                }))
                .unwrap()
                .as_bytes(),
            )
            .unwrap();
            file.write_all(b"\n").unwrap();
            file
        });
        Self {
            records: vec![],
            sum_round_time: 0.,
            sum_syndrome: 0,
            noisy_measurements,
            benchmark_profiler_output,
        }
    }
    /// record the beginning of a decoding procedure
    pub fn begin(&mut self, syndrome_pattern: &SyndromePattern) {
        // sanity check last entry, if exists, is complete
        if let Some(last_entry) = self.records.last() {
            assert!(
                last_entry.is_complete(),
                "the last benchmark profiler entry is not complete, make sure to call `begin` and `end` in pairs"
            );
        }
        let entry = BenchmarkProfilerEntry::new(syndrome_pattern);
        self.records.push(entry);
        self.records.last_mut().unwrap().record_begin();
    }
    pub fn event(&mut self, event_name: String) {
        let last_entry = self
            .records
            .last_mut()
            .expect("last entry not exists, call `begin` before `end`");
        last_entry.record_event(event_name);
    }
    /// record the ending of a decoding procedure
    pub fn end(&mut self, solver: Option<&dyn PrimalDualSolver>) {
        let last_entry = self
            .records
            .last_mut()
            .expect("last entry not exists, call `begin` before `end`");
        last_entry.record_end();
        self.sum_round_time += last_entry.round_time.unwrap();
        self.sum_syndrome += last_entry.syndrome_pattern.defect_vertices.len();
        if let Some(file) = self.benchmark_profiler_output.as_mut() {
            let mut events = serde_json::Map::new();
            for (event_name, time) in last_entry.events.iter() {
                events.insert(event_name.clone(), json!(time));
            }
            let mut value = json!({
                "round_time": last_entry.round_time.unwrap(),
                "defect_num": last_entry.syndrome_pattern.defect_vertices.len(),
                "events": events,
            });
            if let Some(solver) = solver {
                let solver_profile = solver.generate_profiler_report();
                value
                    .as_object_mut()
                    .unwrap()
                    .insert("solver_profile".to_string(), solver_profile);
            }
            file.write_all(serde_json::to_string(&value).unwrap().as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
        }
    }
    /// print out a brief one-line statistics
    pub fn brief(&self) -> String {
        let total = self.sum_round_time / (self.records.len() as f64);
        let per_round = total / (1. + self.noisy_measurements as f64);
        let per_defect = self.sum_round_time / (self.sum_syndrome as f64);
        format!("total: {total:.3e}, round: {per_round:.3e}, defect: {per_defect:.3e},")
    }
}

pub struct BenchmarkProfilerEntry {
    /// the syndrome pattern of this decoding problem
    pub syndrome_pattern: SyndromePattern,
    /// the time of beginning a decoding procedure
    begin_time: Option<Instant>,
    /// record additional events
    pub events: Vec<(String, f64)>,
    /// interval between calling [`Self::record_begin`] to calling [`Self::record_end`]
    pub round_time: Option<f64>,
}

impl BenchmarkProfilerEntry {
    pub fn new(syndrome_pattern: &SyndromePattern) -> Self {
        Self {
            syndrome_pattern: syndrome_pattern.clone(),
            begin_time: None,
            events: vec![],
            round_time: None,
        }
    }
    /// record the beginning of a decoding procedure
    pub fn record_begin(&mut self) {
        assert_eq!(self.begin_time, None, "do not call `record_begin` twice on the same entry");
        self.begin_time = Some(Instant::now());
    }
    /// record the ending of a decoding procedure
    pub fn record_end(&mut self) {
        let begin_time = self
            .begin_time
            .as_ref()
            .expect("make sure to call `record_begin` before calling `record_end`");
        self.round_time = Some(begin_time.elapsed().as_secs_f64());
    }
    pub fn record_event(&mut self, event_name: String) {
        let begin_time = self
            .begin_time
            .as_ref()
            .expect("make sure to call `record_begin` before calling `record_end`");
        self.events.push((event_name, begin_time.elapsed().as_secs_f64()));
    }
    pub fn is_complete(&self) -> bool {
        self.round_time.is_some()
    }
}

/**
 * If you want to modify a field of a Rust struct, it will return a copy of it to avoid memory unsafety.
 * Thus, typical way of modifying a python field doesn't work, e.g. `obj.a.b.c = 1` won't actually modify `obj`.
 * This helper class is used to modify a field easier; but please note this can be very time consuming if not optimized well.
 *
 * Example:
 * with PyMut(code, "vertices") as vertices:
 *     with fb.PyMut(vertices[0], "position") as position:
 *         position.i = 100
*/
#[cfg(feature = "python_binding")]
#[pyclass]
pub struct PyMut {
    /// the python object that provides getter and setter function for the attribute
    #[pyo3(get, set)]
    object: PyObject,
    /// the name of the attribute
    #[pyo3(get, set)]
    attr_name: String,
    /// the python attribute object that is taken from `object[attr_name]`
    #[pyo3(get, set)]
    attr_object: Option<PyObject>,
}

#[cfg(feature = "python_binding")]
#[pymethods]
impl PyMut {
    #[new]
    pub fn new(object: PyObject, attr_name: String) -> Self {
        Self {
            object,
            attr_name,
            attr_object: None,
        }
    }
    pub fn __enter__(&mut self) -> PyObject {
        assert!(self.attr_object.is_none(), "do not enter twice");
        Python::with_gil(|py| {
            let attr_object = self.object.getattr(py, self.attr_name.as_str()).unwrap();
            self.attr_object = Some(attr_object.clone_ref(py));
            attr_object
        })
    }
    pub fn __exit__(&mut self, _exc_type: PyObject, _exc_val: PyObject, _exc_tb: PyObject) {
        Python::with_gil(|py| {
            self.object
                .setattr(py, self.attr_name.as_str(), self.attr_object.take().unwrap())
                .unwrap()
        })
    }
}

#[cfg(feature = "python_binding")]
pub fn json_to_pyobject_locked<'py>(value: serde_json::Value, py: Python<'py>) -> PyObject {
    match value {
        serde_json::Value::Null => py.None(),
        serde_json::Value::Bool(value) => value.to_object(py).into(),
        serde_json::Value::Number(value) => {
            if value.is_i64() {
                value.as_i64().to_object(py).into()
            } else {
                value.as_f64().to_object(py).into()
            }
        }
        serde_json::Value::String(value) => value.to_object(py).into(),
        serde_json::Value::Array(array) => {
            let elements: Vec<PyObject> = array.into_iter().map(|value| json_to_pyobject_locked(value, py)).collect();
            pyo3::types::PyList::new(py, elements).into()
        }
        serde_json::Value::Object(map) => {
            let pydict = pyo3::types::PyDict::new(py);
            for (key, value) in map.into_iter() {
                let pyobject = json_to_pyobject_locked(value, py);
                pydict.set_item(key, pyobject).unwrap();
            }
            pydict.into()
        }
    }
}

#[cfg(feature = "python_binding")]
pub fn json_to_pyobject(value: serde_json::Value) -> PyObject {
    Python::with_gil(|py| json_to_pyobject_locked(value, py))
}

#[cfg(feature = "python_binding")]
pub fn pyobject_to_json_locked<'py>(value: PyObject, py: Python<'py>) -> serde_json::Value {
    let value: &PyAny = value.as_ref(py);
    if value.is_none() {
        serde_json::Value::Null
    } else if value.is_instance_of::<pyo3::types::PyBool>().unwrap() {
        json!(value.extract::<bool>().unwrap())
    } else if value.is_instance_of::<pyo3::types::PyInt>().unwrap() {
        json!(value.extract::<i64>().unwrap())
    } else if value.is_instance_of::<pyo3::types::PyFloat>().unwrap() {
        json!(value.extract::<f64>().unwrap())
    } else if value.is_instance_of::<pyo3::types::PyString>().unwrap() {
        json!(value.extract::<String>().unwrap())
    } else if value.is_instance_of::<pyo3::types::PyList>().unwrap() {
        let elements: Vec<serde_json::Value> = value
            .extract::<Vec<PyObject>>()
            .unwrap()
            .into_iter()
            .map(|object| pyobject_to_json_locked(object, py))
            .collect();
        json!(elements)
    } else if value.is_instance_of::<pyo3::types::PyDict>().unwrap() {
        let map: &pyo3::types::PyDict = value.downcast().unwrap();
        let mut json_map = serde_json::Map::new();
        for (key, value) in map.iter() {
            json_map.insert(
                key.extract::<String>().unwrap(),
                pyobject_to_json_locked(value.to_object(py), py),
            );
        }
        serde_json::Value::Object(json_map)
    } else {
        unimplemented!("unsupported python type, should be (cascaded) dict, list and basic numerical types")
    }
}

#[cfg(feature = "python_binding")]
pub fn pyobject_to_json(value: PyObject) -> serde_json::Value {
    Python::with_gil(|py| pyobject_to_json_locked(value, py))
}

#[cfg(feature = "python_binding")]
#[pyfunction]
pub(crate) fn register(py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<SolverInitializer>()?;
    m.add_class::<PyMut>()?;
    m.add_class::<PartitionUnitInfo>()?;
    m.add_class::<PartitionInfo>()?;
    m.add_class::<PartitionConfig>()?;
    m.add_class::<SyndromePattern>()?;
    use crate::pyo3::PyTypeInfo;
    // m.add_class::<IndexRange>()?;
    m.add("VertexRange", VertexRange::type_object(py))?;
    m.add("DefectRange", DefectRange::type_object(py))?;
    m.add("SyndromeRange", DefectRange::type_object(py))?; // backward compatibility
    m.add("NodeRange", NodeRange::type_object(py))?;
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// test syndrome partition utilities
    #[test]
    fn util_partitioned_syndrome_pattern_1() {
        // cargo test util_partitioned_syndrome_pattern_1 -- --nocapture
        let mut partition_config = PartitionConfig::new(132);
        partition_config.partitions = vec![
            VertexRange::new(0, 72),   // unit 0
            VertexRange::new(84, 132), // unit 1
        ];
        partition_config.fusions = vec![
            (0, 1), // unit 2, by fusing 0 and 1
        ];
        let partition_info = partition_config.info();
        let tests = vec![
            (vec![10, 11, 12, 71, 72, 73, 84, 85, 111], DefectRange::new(4, 6)),
            (vec![10, 11, 12, 13, 71, 72, 73, 84, 85, 111], DefectRange::new(5, 7)),
            (vec![10, 11, 12, 71, 72, 73, 83, 84, 85, 111], DefectRange::new(4, 7)),
            (
                vec![10, 11, 12, 71, 72, 73, 84, 85, 100, 101, 102, 103, 111],
                DefectRange::new(4, 6),
            ),
        ];
        for (defect_vertices, expected_defect_range) in tests.into_iter() {
            let syndrome_pattern = SyndromePattern::new(defect_vertices, vec![]);
            let partitioned_syndrome_pattern = PartitionedSyndromePattern::new(&syndrome_pattern);
            let (owned_partitioned, (_left_partitioned, _right_partitioned)) =
                partitioned_syndrome_pattern.partition(&partition_info.units[2]);
            println!("defect_range: {:?}", owned_partitioned.whole_defect_range);
            assert_eq!(owned_partitioned.whole_defect_range, expected_defect_range);
        }
    }
}
