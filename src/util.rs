use super::rand_xoshiro;
use crate::rand_xoshiro::rand_core::RngCore;


cfg_if::cfg_if! {
    if #[cfg(feature="i32_weight")] {
        /// use i32 to store weight to be compatible with blossom V library (c_int)
        pub type Weight = i32;
    } else {
        pub type Weight = i64;
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
