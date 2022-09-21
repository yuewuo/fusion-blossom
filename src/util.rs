use super::rand_xoshiro;
use crate::rand_xoshiro::rand_core::RngCore;
use std::sync::{Arc, Weak};
use crate::parking_lot::{RwLock, RawRwLock};
use crate::parking_lot::lock_api::{RwLockReadGuard, RwLockWriteGuard};
use serde::{Serialize, Deserialize};
use std::collections::BTreeSet;
use std::time::Instant;
use std::fs::File;
use std::io::prelude::*;
use super::mwpm_solver::PrimalDualSolver;


cfg_if::cfg_if! {
    if #[cfg(feature="i32_weight")] {
        /// use i32 to store weight to be compatible with blossom V library (c_int)
        pub type Weight = i32;
    } else {
        pub type Weight = i64;
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature="u32_index")] {
        // use u32 to store index, for less memory usage
        pub type VertexIndex = u32;  // the vertex index in the decoding graph
        pub type EdgeIndex = u32;
        pub type NodeIndex = u32;
        pub type SyndromeIndex = u32;
    } else {
        pub type VertexIndex = usize;
        pub type EdgeIndex = usize;
        pub type NodeIndex = usize;
        pub type SyndromeIndex = usize;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverInitializer {
    /// the number of vertices
    pub vertex_num: VertexIndex,
    /// weighted edges, where vertex indices are within the range [0, vertex_num)
    pub weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>,
    /// the virtual vertices
    pub virtual_vertices: Vec<VertexIndex>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyndromePattern {
    /// the vertices corresponding to non-trivial measurements
    pub syndrome_vertices: Vec<VertexIndex>,
    /// the edges that experience erasures, i.e. known errors
    pub erasures: Vec<EdgeIndex>,
}

impl SyndromePattern {
    pub fn new(syndrome_vertices: Vec<VertexIndex>, erasures: Vec<EdgeIndex>) -> Self {
        Self { syndrome_vertices, erasures }
    }
    pub fn new_vertices(syndrome_vertices: Vec<VertexIndex>) -> Self {
        Self::new(syndrome_vertices, vec![])
    }
    pub fn new_empty() -> Self {
        Self::new(vec![], vec![])
    }
}

/// an efficient representation of partitioned vertices and erasures when they're ordered
#[derive(Debug, Clone, Serialize)]
pub struct PartitionedSyndromePattern<'a> {
    /// the original syndrome pattern to be partitioned
    pub syndrome_pattern: &'a SyndromePattern,
    /// the syndrome range of this partition: it must be continuous if the syndrome vertices are ordered
    pub whole_syndrome_range: SyndromeRange,
}

impl<'a> PartitionedSyndromePattern<'a> {
    pub fn new(syndrome_pattern: &'a SyndromePattern) -> Self {
        assert!(syndrome_pattern.erasures.is_empty(), "erasure partition not supported yet;
        even if the edges in the erasure is well ordered, they may not be able to be represented as
        a single range simply because the partition is vertex-based. need more consideration");
        Self {
            syndrome_pattern: syndrome_pattern,
            whole_syndrome_range: SyndromeRange::new(0, syndrome_pattern.syndrome_vertices.len()),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct IndexRange<IndexType> {
    pub range: [IndexType; 2],
}

pub type VertexRange = IndexRange<VertexIndex>;
pub type NodeRange = IndexRange<NodeIndex>;
pub type SyndromeRange = IndexRange<SyndromeIndex>;

impl<IndexType: std::fmt::Display + std::fmt::Debug + Ord + std::ops::Sub<Output=IndexType> + std::convert::Into<usize> + Copy
        + std::ops::Add<Output=IndexType>> IndexRange<IndexType> {
    pub fn new(start: IndexType, end: IndexType) -> Self {
        debug_assert!(end >= start, "invalid range [{}, {})", start, end);
        Self { range: [start, end], }
    }
    pub fn new_length(start: IndexType, length: IndexType) -> Self {
        Self::new(start, start + length)
    }
    pub fn iter(&self) -> std::ops::Range<IndexType> {
        self.range[0].. self.range[1]
    }
    pub fn len(&self) -> usize {
        (self.range[1] - self.range[0]).into()
    }
    pub fn start(&self) -> IndexType {
        self.range[0]
    }
    pub fn end(&self) -> IndexType {
        self.range[1]
    }
    pub fn append_by(&mut self, append_count: IndexType) {
        self.range[1] = self.range[1] + append_count;
    }
    pub fn bias_by(&mut self, bias: IndexType) {
        self.range[0] = self.range[0] + bias;
        self.range[1] = self.range[1] + bias;
    }
    pub fn sanity_check(&self) {
        assert!(self.start() <= self.end(), "invalid vertex range {:?}", self);
    }
    pub fn contains(&self, vertex_index: &IndexType) -> bool {
        *vertex_index >= self.start() && *vertex_index < self.end()
    }
    pub fn contains_any(&self, vertex_indices: &Vec<IndexType>) -> bool {
        for vertex_index in vertex_indices.iter() {
            if self.contains(vertex_index) {
                return true
            }
        }
        false
    }
    /// fuse two ranges together, returning (the whole range, the interfacing range)
    pub fn fuse(&self, other: &Self) -> (Self, Self) {
        self.sanity_check();
        other.sanity_check();
        assert!(self.range[1] <= other.range[0], "only lower range can fuse higher range");
        (Self::new(self.range[0], other.range[1]), Self::new(self.range[1], other.range[0]))
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

pub type PartitionUnitPtr = ArcRwLock<PartitionUnit>;
pub type PartitionUnitWeak = WeakRwLock<PartitionUnit>;

impl std::fmt::Debug for PartitionUnitPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let partition_unit = self.read_recursive();
        write!(f, "{}{}", if partition_unit.enabled { "E" } else { "D" }, partition_unit.unit_index)
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
pub struct PartitionConfig {
    /// the number of vertices
    pub vertex_num: usize,
    /// detailed plan of partitioning serial modules: each serial module possesses a list of vertices, including all interface vertices
    pub partitions: Vec<VertexRange>,
    /// detailed plan of interfacing vertices
    pub fusions: Vec<(usize, usize)>,
}

impl PartitionConfig {

    pub fn default(vertex_num: usize) -> Self {
        Self {
            vertex_num: vertex_num,
            partitions: vec![VertexRange::new(0, vertex_num)],
            fusions: vec![],
        }
    }

    pub fn into_info(self) -> Arc<PartitionInfo> {
        assert!(self.partitions.len() > 0, "at least one partition must exist");
        let mut whole_ranges = vec![];
        let mut owning_ranges = vec![];
        for partition in self.partitions.iter() {
            partition.sanity_check();
            assert!(partition.end() <= self.vertex_num, "invalid vertex index {} in partitions", partition.end());
            whole_ranges.push(partition.clone());
            owning_ranges.push(partition.clone());
        }
        let mut parents: Vec<Option<usize>> = (0..self.partitions.len() + self.fusions.len()).map(|_| None).collect();
        for (fusion_index, (left_index, right_index)) in self.fusions.iter().enumerate() {
            let unit_index = fusion_index + self.partitions.len();
            assert!(*left_index < unit_index, "dependency wrong, {} depending on {}", unit_index, left_index);
            assert!(*right_index < unit_index, "dependency wrong, {} depending on {}", unit_index, right_index);
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
        for unit_index in 0..self.partitions.len() + self.fusions.len() - 1 {
            assert!(parents[unit_index].is_some(), "found unit {} without being fused", unit_index);
        }
        // check that the final node has the full range
        let last_unit_index = self.partitions.len() + self.fusions.len() - 1;
        assert!(whole_ranges[last_unit_index].start() == 0, "final range not covering all vertices {:?}", whole_ranges[last_unit_index]);
        assert!(whole_ranges[last_unit_index].end() == self.vertex_num, "final range not covering all vertices {:?}", whole_ranges[last_unit_index]);
        // construct partition info
        let mut partition_unit_info: Vec<_> = (0..self.partitions.len() + self.fusions.len()).map(|i| {
            PartitionUnitInfo {
                whole_range: whole_ranges[i],
                owning_range: owning_ranges[i],
                children: if i >= self.partitions.len() { Some(self.fusions[i - self.partitions.len()]) } else { None },
                parent: parents[i].clone(),
                leaves: if i < self.partitions.len() { vec![i] } else { vec![] },
                descendants: BTreeSet::new(),
            }
        }).collect();
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
                vertex_to_owning_unit[vertex_index] = unit_index;
            }
        }
        Arc::new(PartitionInfo {
            config: self,
            units: partition_unit_info,
            vertex_to_owning_unit: vertex_to_owning_unit,
        })
    }

}

#[derive(Debug, Clone)]
pub struct PartitionInfo {
    /// the initial configuration that creates this info
    pub config: PartitionConfig,
    /// individual info of each unit
    pub units: Vec<PartitionUnitInfo>,
    /// the mapping from vertices to the owning unit: serial unit (holding real vertices) as well as parallel units (holding interfacing vertices);
    /// used for loading syndrome to the holding units
    pub vertex_to_owning_unit: Vec<usize>,
}

impl PartitionInfo {

    /// split a sequence of syndrome into multiple parts, each corresponds to a unit;
    /// this is a slow method and should only be used when the syndrome pattern is not well-ordered
    pub fn partition_syndrome_unordered(&self, syndrome_pattern: &SyndromePattern) -> Vec<SyndromePattern> {
        let mut partitioned_syndrome: Vec<_> = (0..self.units.len()).map(|_| SyndromePattern::new_empty()).collect();
        for syndrome_vertex in syndrome_pattern.syndrome_vertices.iter() {
            let unit_index = self.vertex_to_owning_unit[*syndrome_vertex];
            partitioned_syndrome[unit_index].syndrome_vertices.push(*syndrome_vertex);
        }
        // TODO: partition edges
        partitioned_syndrome
    }

}

impl<'a> PartitionedSyndromePattern<'a> {

    /// partition the syndrome pattern into 2 partitioned syndrome pattern and my whole range
    pub fn partition(&self, partition_unit_info: &PartitionUnitInfo) -> (SyndromeRange, (Self, Self)) {
        // first binary search the start of owning syndrome vertices
        let owning_start_index = {
            let mut left_index = self.whole_syndrome_range.start();
            let mut right_index = self.whole_syndrome_range.end();
            while left_index != right_index {
                let mid_index = (left_index + right_index) / 2;
                let mid_syndrome_vertex = self.syndrome_pattern.syndrome_vertices[mid_index];
                if mid_syndrome_vertex < partition_unit_info.owning_range.start() {
                    left_index = mid_index + 1;
                } else {
                    right_index = mid_index;
                }
            }
            left_index
        };
        // second binary search the end of owning syndrome vertices
        let owning_end_index = {
            let mut left_index = self.whole_syndrome_range.start();
            let mut right_index = self.whole_syndrome_range.end();
            while left_index != right_index {
                let mid_index = (left_index + right_index) / 2;
                let mid_syndrome_vertex = self.syndrome_pattern.syndrome_vertices[mid_index];
                if mid_syndrome_vertex < partition_unit_info.owning_range.end() {
                    left_index = mid_index + 1;
                } else {
                    right_index = mid_index;
                }
            }
            left_index
        };
        (SyndromeRange::new(owning_start_index, owning_end_index), (Self {
            syndrome_pattern: &self.syndrome_pattern,
            whole_syndrome_range: SyndromeRange::new(self.whole_syndrome_range.start(), owning_start_index),
        }, Self {
            syndrome_pattern: &self.syndrome_pattern,
            whole_syndrome_range: SyndromeRange::new(owning_end_index, self.whole_syndrome_range.end()),
        }))
    }

    pub fn expand(&self) -> SyndromePattern {
        let mut syndrome_vertices = Vec::with_capacity(self.whole_syndrome_range.len());
        for syndrome_index in self.whole_syndrome_range.iter() {
            syndrome_vertices.push(self.syndrome_pattern.syndrome_vertices[syndrome_index]);
        }
        SyndromePattern::new(syndrome_vertices, vec![])
    }

}

#[derive(Debug, Clone)]
pub struct PartitionUnitInfo {
    /// the whole range of units
    pub whole_range: VertexRange,
    /// the owning range of units, meaning vertices inside are exclusively belonging to the unit
    pub owning_range: VertexRange,
    /// left and right
    pub children: Option<(usize, usize)>,
    /// parent dual module
    pub parent: Option<usize>,
    /// all the leaf dual modules
    pub leaves: Vec<usize>,
    /// all the descendants
    pub descendants: BTreeSet<usize>,
}

#[derive(Debug, Clone)]
pub struct PartitionedSolverInitializer {
    /// unit index
    pub unit_index: usize,
    /// the number of all vertices (including those partitioned into other serial modules)
    pub vertex_num: usize,
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
pub fn build_old_to_new(reordered_vertices: &Vec<VertexIndex>) -> Vec<Option<usize>> {
    let mut old_to_new: Vec<Option<usize>> = (0..reordered_vertices.len()).map(|_| None).collect();
    for (new_index, old_index) in reordered_vertices.iter().enumerate() {
        assert_eq!(old_to_new[*old_index], None, "duplicate vertex found {}", old_index);
        old_to_new[*old_index] = Some(new_index);
    }
    old_to_new
}

/// translate syndrome vertices into the current new index given reordered_vertices
pub fn translated_syndrome_to_reordered(reordered_vertices: &Vec<VertexIndex>, old_syndrome_vertices: &Vec<VertexIndex>) -> Vec<VertexIndex> {
    let old_to_new = build_old_to_new(reordered_vertices);
    old_syndrome_vertices.iter().map(|old_index| {
        old_to_new[*old_index].unwrap()
    }).collect()
}

impl SolverInitializer {
    pub fn new(vertex_num: VertexIndex, weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>, virtual_vertices: Vec<VertexIndex>) -> SolverInitializer {
        SolverInitializer {
            vertex_num: vertex_num,
            weighted_edges: weighted_edges,
            virtual_vertices: virtual_vertices,
        }
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

/// allows fast reset of vector of objects without iterating over all objects each time: dynamically clear it
pub trait FastClear {

    /// user provided method to actually clear the fields
    fn hard_clear(&mut self);

    /// get timestamp
    fn get_timestamp(&self) -> FastClearTimestamp;

    /// set timestamp
    fn set_timestamp(&mut self, timestamp: FastClearTimestamp);

    /// dynamically clear it if not already cleared; it's safe to call many times
    #[inline(always)]
    fn dynamic_clear(&mut self, active_timestamp: FastClearTimestamp) {
        if self.get_timestamp() != active_timestamp {
            self.hard_clear();
            self.set_timestamp(active_timestamp);
        }
    }

    /// when debugging your program, you can put this function every time you obtained a lock of a new object
    #[inline(always)]
    fn debug_assert_dynamic_cleared(&self, active_timestamp: FastClearTimestamp) {
        debug_assert!(self.get_timestamp() == active_timestamp, "bug detected: not dynamically cleared, expected timestamp: {}, current timestamp: {}"
            , active_timestamp, self.get_timestamp());
    }

}

pub trait FastClearRwLockPtr<ObjType> where ObjType: FastClear {

    fn new_ptr(ptr: Arc<RwLock<ObjType>>) -> Self;

    fn new(obj: ObjType) -> Self;

    fn ptr(&self) -> &Arc<RwLock<ObjType>>;

    fn ptr_mut(&mut self) -> &mut Arc<RwLock<ObjType>>;

    #[inline(always)]
    fn read_recursive(&self, active_timestamp: FastClearTimestamp) -> RwLockReadGuard<RawRwLock, ObjType> {
        let ret = self.ptr().read_recursive();
        ret.debug_assert_dynamic_cleared(active_timestamp);  // only assert during debug modes
        ret
    }

    /// without sanity check: this data might be outdated, so only use when you're read those immutable fields 
    #[inline(always)]
    fn read_recursive_force(&self) -> RwLockReadGuard<RawRwLock, ObjType> {
        let ret = self.ptr().read_recursive();
        ret
    }

    #[inline(always)]
    fn write(&self, active_timestamp: FastClearTimestamp) -> RwLockWriteGuard<RawRwLock, ObjType> {
        let ret = self.ptr().write();
        ret.debug_assert_dynamic_cleared(active_timestamp);  // only assert during debug modes
        ret
    }

    /// without sanity check: useful only in implementing hard_clear
    #[inline(always)]
    fn write_force(&self) -> RwLockWriteGuard<RawRwLock, ObjType> {
        let ret = self.ptr().write();
        ret
    }

    /// dynamically clear it if not already cleared; it's safe to call many times, but it will acquire a writer lock
    #[inline(always)]
    fn dynamic_clear(&self, active_timestamp: FastClearTimestamp) {
        let mut value = self.write_force();
        value.dynamic_clear(active_timestamp);
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.ptr(), other.ptr())
    }

}

pub trait RwLockPtr<ObjType> {

    fn new_ptr(ptr: Arc<RwLock<ObjType>>) -> Self;

    fn new(obj: ObjType) -> Self;

    fn ptr(&self) -> &Arc<RwLock<ObjType>>;

    fn ptr_mut(&mut self) -> &mut Arc<RwLock<ObjType>>;

    #[inline(always)]
    fn read_recursive(&self) -> RwLockReadGuard<RawRwLock, ObjType> {
        let ret = self.ptr().read_recursive();
        ret
    }

    #[inline(always)]
    fn write(&self) -> RwLockWriteGuard<RawRwLock, ObjType> {
        let ret = self.ptr().write();
        ret
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.ptr(), other.ptr())
    }

}

pub struct ArcRwLock<T> {
    ptr: Arc<RwLock<T>>,
}

pub struct WeakRwLock<T> {
    ptr: Weak<RwLock<T>>,
}

impl<T> ArcRwLock<T> {
    pub fn downgrade(&self) -> WeakRwLock<T> {
        WeakRwLock::<T> {
            ptr: Arc::downgrade(&self.ptr)
        }
    }
}

impl<T> WeakRwLock<T> {
    pub fn upgrade_force(&self) -> ArcRwLock<T> {
        ArcRwLock::<T> {
            ptr: self.ptr.upgrade().unwrap()
        }
    }
    pub fn upgrade(&self) -> Option<ArcRwLock<T>> {
        self.ptr.upgrade().map(|x| ArcRwLock::<T> { ptr: x })
    }
}

impl<T> Clone for ArcRwLock<T> {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl<T> RwLockPtr<T> for ArcRwLock<T> {
    fn new_ptr(ptr: Arc<RwLock<T>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: T) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<T>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<T>> { &mut self.ptr }
}

impl<T> PartialEq for ArcRwLock<T> {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl<T> Eq for ArcRwLock<T> { }

impl<T> Clone for WeakRwLock<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr.clone() }
    }
}

impl<T> PartialEq for WeakRwLock<T> {
    fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
}

impl<T> Eq for WeakRwLock<T> { }

impl<T> std::ops::Deref for ArcRwLock<T> {
    type Target = RwLock<T>;
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T> weak_table::traits::WeakElement for WeakRwLock<T> {
    type Strong = ArcRwLock<T>;
    fn new(view: &Self::Strong) -> Self {
        view.downgrade()
    }
    fn view(&self) -> Option<Self::Strong> {
        self.upgrade()
    }
    fn clone(view: &Self::Strong) -> Self::Strong {
        view.clone()
    }
}

pub struct FastClearArcRwLock<T: FastClear> {
    ptr: Arc<RwLock<T>>,
}

pub struct FastClearWeakRwLock<T: FastClear> {
    ptr: Weak<RwLock<T>>,
}

impl<T: FastClear> FastClearArcRwLock<T> {
    pub fn downgrade(&self) -> FastClearWeakRwLock<T> {
        FastClearWeakRwLock::<T> {
            ptr: Arc::downgrade(&self.ptr)
        }
    }
}

impl<T: FastClear> FastClearWeakRwLock<T> {
    pub fn upgrade_force(&self) -> FastClearArcRwLock<T> {
        FastClearArcRwLock::<T> {
            ptr: self.ptr.upgrade().unwrap()
        }
    }
    pub fn upgrade(&self) -> Option<FastClearArcRwLock<T>> {
        self.ptr.upgrade().map(|x| FastClearArcRwLock::<T> { ptr: x })
    }
}

impl<T: FastClear> Clone for FastClearArcRwLock<T> {
    fn clone(&self) -> Self {
        Self::new_ptr(Arc::clone(self.ptr()))
    }
}

impl<T: FastClear> FastClearRwLockPtr<T> for FastClearArcRwLock<T> {
    fn new_ptr(ptr: Arc<RwLock<T>>) -> Self { Self { ptr: ptr }  }
    fn new(obj: T) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
    #[inline(always)] fn ptr(&self) -> &Arc<RwLock<T>> { &self.ptr }
    #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<T>> { &mut self.ptr }
}

impl<T: FastClear> PartialEq for FastClearArcRwLock<T> {
    fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
}

impl<T: FastClear> Eq for FastClearArcRwLock<T> { }

impl<T: FastClear> Clone for FastClearWeakRwLock<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr.clone() }
    }
}

impl<T: FastClear> PartialEq for FastClearWeakRwLock<T> {
    fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
}

impl<T: FastClear> Eq for FastClearWeakRwLock<T> { }

impl<T: FastClear> std::ops::Deref for FastClearArcRwLock<T> {
    type Target = RwLock<T>;
    fn deref(&self) -> &Self::Target {
        &self.ptr
    }
}

impl<T: FastClear> weak_table::traits::WeakElement for FastClearWeakRwLock<T> {
    type Strong = FastClearArcRwLock<T>;
    fn new(view: &Self::Strong) -> Self {
        view.downgrade()
    }
    fn view(&self) -> Option<Self::Strong> {
        self.upgrade()
    }
    fn clone(view: &Self::Strong) -> Self::Strong {
        view.clone()
    }
}

/// record the decoding time of multiple syndrome patterns
pub struct BenchmarkProfiler {
    /// each record corresponds to a different syndrome pattern
    pub records: Vec<BenchmarkProfilerEntry>,
    /// summation of all decoding time
    pub sum_decoding_time: f64,
    /// syndrome count
    pub sum_syndrome: usize,
    /// noisy measurement round
    pub noisy_measurements: usize,
    /// the file to output the profiler results
    pub benchmark_profiler_output: Option<File>,
}

impl BenchmarkProfiler {
    pub fn new(noisy_measurements: usize, detail_log_file: Option<(String, &PartitionInfo)>) -> Self {
        let benchmark_profiler_output = detail_log_file.map(|(filename, partition_info)| {
            let mut file = File::create(filename).unwrap();
            file.write_all(serde_json::to_string(&partition_info.config).unwrap().as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
            file
        });
        Self {
            records: vec![],
            sum_decoding_time: 0.,
            sum_syndrome: 0,
            noisy_measurements: noisy_measurements,
            benchmark_profiler_output: benchmark_profiler_output,
        }
    }
    /// record the beginning of a decoding procedure
    pub fn begin(&mut self, syndrome_pattern: &SyndromePattern) {
        // sanity check last entry, if exists, is complete
        if let Some(last_entry) = self.records.last() {
            assert!(last_entry.is_complete(), "the last benchmark profiler entry is not complete, make sure to call `begin` and `end` in pairs");
        }
        let entry = BenchmarkProfilerEntry::new(syndrome_pattern);
        self.records.push(entry);
        self.records.last_mut().unwrap().record_begin();
    }
    /// record the ending of a decoding procedure
    pub fn end(&mut self, solver: Option<&Box<dyn PrimalDualSolver>>) {
        let last_entry = self.records.last_mut().expect("last entry not exists, call `begin` before `end`");
        last_entry.record_end();
        self.sum_decoding_time += last_entry.decoding_time.unwrap();
        self.sum_syndrome += last_entry.syndrome_pattern.syndrome_vertices.len();
        if let Some(file) = self.benchmark_profiler_output.as_mut() {
            let mut value = json!({
                "decoding_time": last_entry.decoding_time.unwrap(),
                "syndrome_num": last_entry.syndrome_pattern.syndrome_vertices.len(),
            });
            if let Some(solver) = solver {
                let solver_profile = solver.generate_profiler_report();
                value.as_object_mut().unwrap().insert(format!("solver_profile"), solver_profile);
            }
            file.write_all(serde_json::to_string(&value).unwrap().as_bytes()).unwrap();
            file.write_all(b"\n").unwrap();
        }
    }
    /// print out a brief one-line statistics
    pub fn brief(&self) -> String {
        let total = self.sum_decoding_time / (self.records.len() as f64);
        let per_round = total / (1. + self.noisy_measurements as f64);
        let per_syndrome = self.sum_decoding_time / (self.sum_syndrome as f64);
        format!("total: {total:.3e}, round: {per_round:.3e}, syndrome: {per_syndrome:.3e},")
    }
}

pub struct BenchmarkProfilerEntry {
    /// the syndrome pattern of this decoding problem
    pub syndrome_pattern: SyndromePattern,
    /// the time of beginning a decoding procedure
    begin_time: Option<Instant>,
    /// interval between calling [`Self::record_begin`] to calling [`Self::record_end`]
    pub decoding_time: Option<f64>,
}

impl BenchmarkProfilerEntry {
    pub fn new(syndrome_pattern: &SyndromePattern) -> Self {
        Self {
            syndrome_pattern: syndrome_pattern.clone(),
            begin_time: None,
            decoding_time: None,
        }
    }
    /// record the beginning of a decoding procedure
    pub fn record_begin(&mut self) {
        assert_eq!(self.begin_time, None, "do not call `record_begin` twice on the same entry");
        self.begin_time = Some(Instant::now());
    }
    /// record the ending of a decoding procedure
    pub fn record_end(&mut self) {
        let begin_time = self.begin_time.as_ref().expect("make sure to call `record_begin` before calling `record_end`");
        self.decoding_time = Some(begin_time.elapsed().as_secs_f64());
    }
    pub fn is_complete(&self) -> bool {
        self.decoding_time.is_some()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// test syndrome partition utilities
    #[test]
    fn util_partitioned_syndrome_pattern_1() {  // cargo test util_partitioned_syndrome_pattern_1 -- --nocapture
        let mut partition_config = PartitionConfig::default(132);
        partition_config.partitions = vec![
            VertexRange::new(0, 72),    // unit 0
            VertexRange::new(84, 132),  // unit 1
        ];
        partition_config.fusions = vec![
            (0, 1),  // unit 2, by fusing 0 and 1
        ];
        let partition_info = partition_config.into_info();
        let tests = vec![
            (vec![10, 11, 12, 71, 72, 73, 84, 85, 111], SyndromeRange::new(4, 6)),
            (vec![10, 11, 12, 13, 71, 72, 73, 84, 85, 111], SyndromeRange::new(5, 7)),
            (vec![10, 11, 12, 71, 72, 73, 83, 84, 85, 111], SyndromeRange::new(4, 7)),
            (vec![10, 11, 12, 71, 72, 73, 84, 85, 100, 101, 102, 103, 111], SyndromeRange::new(4, 6)),
        ];
        for (syndrome_vertices, expected_syndrome_range) in tests.into_iter() {
            let syndrome_pattern = SyndromePattern::new(syndrome_vertices, vec![]);
            let partitioned_syndrome_pattern = PartitionedSyndromePattern::new(&syndrome_pattern);
            let (syndrome_range, (_left_partitioned, _right_partitioned)) = partitioned_syndrome_pattern.partition(&partition_info.units[2]);
            println!("syndrome_range: {syndrome_range:?}");
            assert_eq!(syndrome_range, expected_syndrome_range);
        }
    }

}
