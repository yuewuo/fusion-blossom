# Usage

```sh
# install a special version of fusion blossom library
pip3 install fusion_blossom-0.2.123-cp37-abi3-macosx_10_9_x86_64.macosx_11_0_arm64.macosx_10_9_universal2.whl
# install npm packages (only `pythonia`)
npm install
# run demos
node demo-simulation.js
node demo-decode.js
```

## Qubit Layout

We use two coordinations for stabilizers and data qubits.

![](./stabilizer-positions.jpeg)

![](./data-qubits-positions.jpeg)

## Yue's Development Note

I need to compile a special python binary for this project

```sh
maturin develop  # for development, quickly apply new version of the library
```
