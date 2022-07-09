use super::rand_xoshiro;
use crate::rand_xoshiro::rand_core::RngCore;
use std::sync::Arc;
use crate::parking_lot::{RwLock, RawRwLock};
use crate::parking_lot::lock_api::{RwLockReadGuard, RwLockWriteGuard};

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
        pub type NodeIndex = u32;
        pub type SyndromeIndex = u32;
    } else {
        pub type VertexIndex = usize;
        pub type NodeIndex = usize;
        pub type SyndromeIndex = usize;
    }
}


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
    fn get_timestamp(&self) -> usize;

    /// set timestamp
    fn set_timestamp(&mut self, timestamp: usize);

    /// dynamically clear it if not already cleared; it's safe to call many times
    #[inline(always)]
    fn dynamic_clear(&mut self, active_timestamp: usize) {
        if self.get_timestamp() != active_timestamp {
            self.hard_clear();
            self.set_timestamp(active_timestamp);
        }
    }

    /// when debugging your program, you can put this function every time you obtained a lock of a new object
    #[inline(always)]
    fn debug_assert_dynamic_cleared(&self, active_timestamp: usize) {
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
    fn read_recursive(&self, active_timestamp: usize) -> RwLockReadGuard<RawRwLock, ObjType> {
        let ret = self.ptr().read_recursive();
        ret.debug_assert_dynamic_cleared(active_timestamp);  // only assert during debug modes
        ret
    }

    #[inline(always)]
    fn write(&self, active_timestamp: usize) -> RwLockWriteGuard<RawRwLock, ObjType> {
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

    fn clone(&self) -> Self where Self: Sized {
        Self::new_ptr(Arc::clone(self.ptr()))
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self.ptr(), other.ptr())
    }

}
