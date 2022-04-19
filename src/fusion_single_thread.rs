//! Single-thread implementation of fusion blossom algorithm
//! 

use super::util::*;

pub struct Vertex {
    
}

pub struct Edge {
    pub weight: Weight,
}

pub struct FusionSingleThread {

}




#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use crate::rand_xoshiro::rand_core::SeedableRng;

    #[test]
    fn single_thread_repetition_code_d11() {  // cargo test single_thread_repetition_code_d11 -- --nocapture
        let d = 11;
        let p = 0.2;
        let node_num = (d-1) + 2;  // two virtual nodes at left and right
        let weight: Weight = 1000;
        let weighted_edges = {
            let mut weighted_edges: Vec<(usize, usize, Weight)> = Vec::new();
            for i in 0..d-1 {
                weighted_edges.push((i, i+1, weight));
            }
            weighted_edges.push((0, d, weight));  // left most edge
            weighted_edges
        };
        let virtual_nodes = vec![d-1, d];
        println!("[debug] weighted_edges: {:?}", weighted_edges);
        let mut errors: Vec<bool> = (0..d).map(|_| false).collect();
        let mut measurements: Vec<bool> = (0..d-1).map(|_| false).collect();
        let rounds = 5;
        for round in 0..rounds {
            let mut rng = DeterministicRng::seed_from_u64(round);
            // generate random error
            for i in 0..d-1 { measurements[i] = false; }  // clear measurement errors
            for i in 0..d {
                errors[i] = rng.next_f64() < p;
                if errors[i] {
                    if i > 0 {
                        measurements[i-1] ^= true;  // flip left
                    }
                    if i < d-1 {
                        measurements[i] ^= true;  // flip right
                    }
                }
            }
            // println!("[debug {}] errors: {:?}", round, errors);
            // generate syndrome
            let mut syndrome_nodes = Vec::new();
            for i in 0..d-1 {
                if measurements[i] {
                    syndrome_nodes.push(i);
                }
            }
            println!("[debug {}] syndrome_nodes: {:?}", round, syndrome_nodes);
            // run ground truth blossom V algorithm
            let blossom_v_matchings = blossom_v_mwpm(node_num, &weighted_edges, &virtual_nodes, &syndrome_nodes);
            let blossom_v_details = detailed_matching(node_num, &weighted_edges, &syndrome_nodes, &blossom_v_matchings);
            let mut blossom_v_weight: Weight = 0;
            for detail in blossom_v_details.iter() {
                blossom_v_weight += detail.weight;
            }
            
            println!("[debug {}] blossom_v_matchings: {:?}", round, blossom_v_matchings);
            println!("[debug {}] blossom_v_weight: {:?}", round, blossom_v_weight);
        }
    }
}
