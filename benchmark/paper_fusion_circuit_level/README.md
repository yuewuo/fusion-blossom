

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
