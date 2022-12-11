//! Distributed Primal Module
//! 
//! A distributed implementation of the primal module, by calling functions provided by the parallel primal module
//! 
//! It's using MPI interfaces to control and communicate between multiple nodes.
//! The tasks are divided into multiple chunks equally and then assign them to different nodes.
//! Each node will run the parallel primal module internally to solve the problem.
//! Then one of the nodes will fuse the two chunks.
//! The root may or may not fuse the last two chunks, but it will know when all the chunks are fused and then can communicate to gather the decoded results.
//!

use crate::mpi;
use mpi::traits::*;
use mpi::environment::Universe;
use parking_lot::RwLock;

lazy_static! {
    /// This is an example for using doc comment attributes
    pub static ref UNIVERSE: RwLock<Option<Universe>> = RwLock::new(None);
}

#[cfg(test)]
pub mod tests {
    use super::*;

    // all the test case below requires manual OpenMPI execution
    // first run: cargo test --no-run --features distributed

    /// test a simple case
    #[test]
    fn primal_module_distributed_mpi_test_1() {  // mpirun -n 5 --oversubscribe target/debug/deps/fusion_blossom-94387e1137c75fb1 primal_module_distributed_mpi_test_1 --nocapture
        let universe = mpi::initialize().unwrap();
        let world = universe.world();
        let size = world.size();
        let rank = world.rank();
        println!("size: {}, rank: {}", size, rank);
    }
}
