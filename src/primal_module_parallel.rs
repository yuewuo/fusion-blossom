//! Parallel Primal Module
//! 
//! A parallel implementation of the primal module, by calling functions provided by the serial primal module
//!

use super::util::*;
// use crate::derivative::Derivative;
// use super::primal_module::*;
// use super::visualize::*;
// use super::dual_module::*;


pub struct PrimalModuleParallel {
    /// the basic wrapped serial modules at the beginning, afterwards the fused units are appended after them
    pub units: Vec<ArcRwLock<PrimalModuleParallelUnit>>,
    /// thread pool used to execute async functions in parallel
    pub thread_pool: rayon::ThreadPool,
}


pub struct PrimalModuleParallelUnit {

}
