//! Parallel Primal Module
//! 
//! A parallel implementation of the primal module, by calling functions provided by the serial primal module
//!

use super::util::*;
// use crate::derivative::Derivative;
use super::primal_module::*;
use super::primal_module_serial::*;
// use super::visualize::*;
use super::dual_module::*;


pub struct PrimalModuleParallel {
    /// the basic wrapped serial modules at the beginning, afterwards the fused units are appended after them
    pub units: Vec<ArcRwLock<PrimalModuleParallelUnit>>,
    /// thread pool used to execute async functions in parallel
    pub thread_pool: rayon::ThreadPool,
}

pub struct PrimalModuleParallelUnit {
    /// the index
    pub unit_index: usize,
    /// the owned serial primal module
    pub serial_module: PrimalModuleSerial,
}

pub type PrimalModuleParallelUnitPtr = ArcRwLock<PrimalModuleParallelUnit>;
pub type PrimalModuleParallelUnitWeak = WeakRwLock<PrimalModuleParallelUnit>;

impl std::fmt::Debug for PrimalModuleParallelUnitPtr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let unit = self.read_recursive();
        write!(f, "{}", unit.unit_index)
    }
}

impl std::fmt::Debug for PrimalModuleParallelUnitWeak {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.upgrade_force().fmt(f)
    }
}

impl PrimalModuleParallel {

}
