use super::cfg_if;

cfg_if::cfg_if! {
    if #[cfg(feature="blossom_v")] {

        use super::libc;
        use libc::{c_int};
        use std::collections::BTreeSet;

        #[link(name = "blossomV")]
        extern {
            fn minimum_weight_perfect_matching(node_num: c_int, edge_num: c_int, edges: *const c_int, weights: *const c_int, matched: *mut c_int);
        }

        pub fn safe_minimum_weight_perfect_matching(node_num: usize, weighted_edges: &[(usize, usize, u32)]) -> Vec<usize> {
            let edge_num = weighted_edges.len();
            let mut edges = Vec::with_capacity(2 * edge_num);
            let mut weights = Vec::with_capacity(edge_num);
            debug_assert!({
                let mut existing_edges = BTreeSet::new();
                let mut sanity_check_passed = true;
                for &(i, j, _weight) in weighted_edges.iter() {
                    if i == j {
                        eprintln!("invalid edge between the same vertex {}", i);
                        sanity_check_passed = false;
                    }
                    let left: usize = if i < j { i } else { j };
                    let right: usize = if i < j { j } else { i };
                    if existing_edges.contains(&(left, right)) {
                        eprintln!("duplicate edge between the vertices {} and {}", i, j);
                        sanity_check_passed = false;
                    }
                    existing_edges.insert((left, right));
                }
                sanity_check_passed
            });
            for &(i, j, weight) in weighted_edges.iter() {
                edges.push(i as c_int);
                edges.push(j as c_int);
                assert!(i < node_num && j < node_num);
                weights.push(weight as c_int);
            }
            let mut output = Vec::with_capacity(node_num);
            unsafe {
                minimum_weight_perfect_matching(node_num as c_int, edge_num as c_int, edges.as_ptr(), weights.as_ptr(), output.as_mut_ptr());
                output.set_len(node_num);
            }
            let output: Vec<usize> = output.iter().map(|x| *x as usize).collect();
            output
        }

    } else {

        pub fn safe_minimum_weight_perfect_matching(_node_num: usize, _weighted_edges: &[(usize, usize, u32)]) -> Vec<usize> {
            unimplemented!("need blossom V library, see README.md")
        }

    }
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "blossom_v")]
    use super::*;

    #[test]
    #[cfg(feature = "blossom_v")]
    fn blossom_v_test_1() {
        // cargo test blossom_v_test_1 -- --nocapture
        let node_num = 4;
        let edges: Vec<(usize, usize, u32)> = vec![(0, 1, 100), (2, 3, 110), (0, 2, 500), (1, 3, 300)];
        let output = safe_minimum_weight_perfect_matching(node_num, &edges);
        assert_eq!(output, vec![1, 0, 3, 2]);
    }
}
