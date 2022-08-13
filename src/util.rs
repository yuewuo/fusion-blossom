use super::rand_xoshiro;
use crate::rand_xoshiro::rand_core::RngCore;
use std::sync::Arc;
use crate::parking_lot::{RwLock, RawRwLock};
use crate::parking_lot::lock_api::{RwLockReadGuard, RwLockWriteGuard};
use crate::futures::executor::ThreadPool;
use crate::futures::future::join_all;
use core::future::Future;
use crate::futures::task::SpawnExt;

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
    pub vertex_num: VertexIndex,
    pub weighted_edges: Vec<(VertexIndex, VertexIndex, Weight)>,
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

/// use a thread pool to execute a vector of items
pub async fn thread_pool_join_all<Item>(thread_pool: &ThreadPool, async_tasks: Vec<Item>) -> Vec<<Item as futures::Future>::Output>
        where for<'a> Item: Future + std::marker::Send + 'a
        , <Item as futures::Future>::Output: std::marker::Send {
    if async_tasks.is_empty() {
        vec![]
    } else {
        thread_pool.spawn_with_handle(async { join_all(async_tasks).await }).unwrap().await
    }
}
