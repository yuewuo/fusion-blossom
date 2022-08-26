use super::rand_xoshiro;
use crate::rand_xoshiro::rand_core::RngCore;
use std::sync::{Arc, Weak};
use crate::parking_lot::{RwLock, RawRwLock};
use crate::parking_lot::lock_api::{RwLockReadGuard, RwLockWriteGuard};
use serde::{Serialize, Deserialize};


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

#[derive(Debug, Clone)]
pub struct SolverInitializer {
    /// the number of vertices
    pub vertex_num: VertexIndex,
    /// weighted edges, where vertex indices are within the range [0, vertex_num)
    pub weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>,
    /// the virtual vertices
    pub virtual_vertices: Vec<VertexIndex>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(transparent)]
pub struct VertexRange {
    pub range: [VertexIndex; 2],
}

impl VertexRange {
    pub fn new(start: VertexIndex, end: VertexIndex) -> Self {
        debug_assert!(end >= start, "invalid range [{}, {})", start, end);
        Self { range: [start, end], }
    }
    pub fn iter(&self) -> std::ops::Range<VertexIndex> {
        self.range[0].. self.range[1]
    }
    pub fn len(&self) -> usize {
        self.range[1] - self.range[0]
    }
    pub fn start(&self) -> VertexIndex {
        self.range[0]
    }
    pub fn end(&self) -> VertexIndex {
        self.range[1]
    }
    pub fn sanity_check(&self) {
        assert!(self.start() <= self.end(), "invalid vertex range {:?}", self);
    }
    pub fn contains(&self, vertex_index: &VertexIndex) -> bool {
        *vertex_index >= self.start() && *vertex_index < self.end()
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

create_ptr_types!(PartitionUnit, PartitionUnitPtr, PartitionUnitWeak);

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

#[derive(Debug, Clone)]
pub struct PartitionedSolverInitializer {
    /// the number of all vertices (including those partitioned into other serial modules)
    pub vertex_num: usize,
    /// the number of all edges (including those partitioned into other serial modules)
    pub edge_num: usize,
    /// vertices exclusively owned by this partition; this part must be a continuous range
    pub owning_range: VertexRange,
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

#[macro_export]
macro_rules! create_ptr_types {
    (
        $struct_name:ident, $ptr_name:ident, $weak_name:ident
    ) => {

        pub struct $ptr_name { ptr: Arc<RwLock<$struct_name>>, }
        pub struct $weak_name { ptr: Weak<RwLock<$struct_name>>, }

        impl $ptr_name { pub fn downgrade(&self) -> $weak_name { $weak_name { ptr: Arc::downgrade(&self.ptr) } } }
        impl $weak_name {
            pub fn upgrade_force(&self) -> $ptr_name { $ptr_name { ptr: self.ptr.upgrade().unwrap() } }
            pub fn upgrade(&self) -> Option<$ptr_name> { self.ptr.upgrade().map(|x| $ptr_name { ptr: x }) }
        }

        impl Clone for $ptr_name {
            fn clone(&self) -> Self {
                Self::new_ptr(Arc::clone(self.ptr()))
            }
        }

        impl RwLockPtr<$struct_name> for $ptr_name {
            fn new_ptr(ptr: Arc<RwLock<$struct_name>>) -> Self { Self { ptr: ptr }  }
            fn new(obj: $struct_name) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
            #[inline(always)] fn ptr(&self) -> &Arc<RwLock<$struct_name>> { &self.ptr }
            #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<$struct_name>> { &mut self.ptr }
        }

        impl PartialEq for $ptr_name {
            fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
        }

        impl Eq for $ptr_name { }

        impl Clone for $weak_name {
            fn clone(&self) -> Self {
            Self { ptr: self.ptr.clone() }
            }
        }

        impl PartialEq for $weak_name {
            fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
        }

        impl Eq for $weak_name { }

    }
}
#[allow(unused_imports)] pub use create_ptr_types;

#[macro_export]
macro_rules! create_fast_clear_ptr_types {
    (
        $struct_name:ident, $ptr_name:ident, $weak_name:ident
    ) => {

        pub struct $ptr_name { ptr: Arc<RwLock<$struct_name>>, }
        pub struct $weak_name { ptr: Weak<RwLock<$struct_name>>, }

        impl $ptr_name { pub fn downgrade(&self) -> $weak_name { $weak_name { ptr: Arc::downgrade(&self.ptr) } } }
        impl $weak_name {
            pub fn upgrade_force(&self) -> $ptr_name { $ptr_name { ptr: self.ptr.upgrade().unwrap() } }
            pub fn upgrade(&self) -> Option<$ptr_name> { self.ptr.upgrade().map(|x| $ptr_name { ptr: x }) }
        }

        impl Clone for $ptr_name {
            fn clone(&self) -> Self {
                Self::new_ptr(Arc::clone(self.ptr()))
            }
        }

        impl FastClearRwLockPtr<$struct_name> for $ptr_name {
            fn new_ptr(ptr: Arc<RwLock<$struct_name>>) -> Self { Self { ptr: ptr }  }
            fn new(obj: $struct_name) -> Self { Self::new_ptr(Arc::new(RwLock::new(obj))) }
            #[inline(always)] fn ptr(&self) -> &Arc<RwLock<$struct_name>> { &self.ptr }
            #[inline(always)] fn ptr_mut(&mut self) -> &mut Arc<RwLock<$struct_name>> { &mut self.ptr }
        }

        impl PartialEq for $ptr_name {
            fn eq(&self, other: &Self) -> bool { self.ptr_eq(other) }
        }

        impl Eq for $ptr_name { }

        impl Clone for $weak_name {
            fn clone(&self) -> Self {
            Self { ptr: self.ptr.clone() }
            }
        }

        impl PartialEq for $weak_name {
            fn eq(&self, other: &Self) -> bool { self.ptr.ptr_eq(&other.ptr) }
        }

        impl Eq for $weak_name { }

    }
}
#[allow(unused_imports)] pub use create_fast_clear_ptr_types;
