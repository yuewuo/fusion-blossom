

We generate decoding graph using [QEC-Playground](https://github.com/yuewuo/QEC-Playground).

To visualize the code, first run the following in QEC-Playground

```sh
# visualize the noise model.
cargo run --release -- tool benchmark [3] [3] [0.001] --code-type rotated-planar-code --noise-model stim-noise-model --decoder fusion -m10 --enable-visualizer --visualizer-filename sparse-blossom-noise-model.json
# open your browser can open http://localhost:8069/?filename=sparse-blossom-noise-model.json
# you may need to start the server in QEC-Playground/visualize/server.py

# test the logical error rate
cargo run --release -- tool benchmark [3] [3] [0.001] --code-type rotated-planar-code --noise-model stim-noise-model --decoder fusion -e1000
```

In order to run the simulation in a reasonable time and memory, we use a new feature to do it.
```sh
cargo run --release -- tool benchmark [5] [1000] [0.001] --code-type rotated-planar-code --noise-model stim-noise-model --use-brief-edge --decoder none --decoder-config '{"only_stab_z":true,"use_combined_probability":false,"skip_decoding":true}' -m1000 --debug-print fusion-blossom-syndrome-file
cargo run --release -- tool benchmark [5] [5] [0.001] --code-type rotated-planar-code --noise-model stim-noise-model --use-brief-edge --decoder fusion --decoder-config '{"only_stab_z":true,"use_combined_probability":false,"skip_decoding":true}' -m1000 --simulator-compact-extender-noisy-measurements 1000 --use-compact-simulator --debug-print fusion-blossom-syndrome-file
```

After testing the noise model correctly, we can use fusion blossom to call qecp to export a file including the decoding graph and several syndromes.
Note that this is different from the `ExampleCode` in this repo, where the decoding graph is ideal from the noise model.
The decoding graph generated from QEC-Playground is an approximation to the real noise model, by removing hyperedges that generates more than 2 defect vertices.
Thus, do not construct an `ExampleCode` instance and simulate noises using that. It will be different from the noise model!!!

```sh
cargo run --release --features qecp_integrate -- qecp-generate-syndrome [3] [3] [0.001] -m100 --code-type rotated-planar-code --noise-model stim-noise-model --fusion-blossom-syndrome-export-config '{"filename":"./tmp/test.syndromes","only_stab_z":true,"use_combined_probability":false}'
# visualize the generated syndrome by decoding them
cargo run --release --features qecp_integrate -- benchmark 3 -n3 0.001 -r1  --code-type error-pattern-reader --code-config '{"filename":"./tmp/test.syndromes"}' --primal-dual-type serial --verifier none --enable-visualizer
# decode all of them and check the speed
cargo run --release --features qecp_integrate -- benchmark 3 -n3 0.001 -r100  --code-type error-pattern-reader --code-config '{"filename":"./tmp/test.syndromes"}' --primal-dual-type serial --verifier none
```


from ../paper_fusion_rotated_code but change noise model to circuit-level noise

# Experiment Note

remember to 
- download all data from AWS machine, just in case we need to check it later.
- change to circuit-level noise, using qecp-generate-syndrome

# Evaluation Plan

Since calling QECP requires a huge amount of memory and CPU time, we need to run almost all of them on m6i.metal instance...
Well, I optimized QECP simulation to support 21 * 21 * 10^5 simulation with ~5GB memory, so it now runs on m6i.4xlarge as well.

on m6i.4xlarge
- [ ] decoding_time_d (code distance)
- [ ] partition_num_single_thread_2_tree (what is optimal partition size)
- [ ] pymatching_compare_various_T (decoding time scaling with T)
- [ ] fusion_time_d
- [ ] fusion_time_delta_T
- [ ] fusion_time_children_count

on m6i.metal (128 vCPU, 512GB memory)
- [ ] thread_pool_size_partition_1k (#threads)
- [ ] decoding_throughput_threads64 (different p)
- [ ] emulate_decoding_d21_threads64 (latency evaluation)
