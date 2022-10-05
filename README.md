![Build Status](https://jenkins.fusionblossom.com/buildStatus/icon?job=FusionBlossomBuild&subject=build&style=flat-square)
![Test Status](https://jenkins.fusionblossom.com/buildStatus/icon?job=FusionBlossomCI&subject=test&style=flat-square)

# fusion-blossom
A fast minimum-weight perfect matching solver for quantum error correction

## Interface

Since the weights in QEC decoding graph are computed by taking the log of error probability, e.g. <!-- $w_e = \log\{(1-p)/p\}$ --> <img style="transform: translateY(0.1em); background: white;" src="https://render.githubusercontent.com/render/math?math=w_e%20%3D%20%5Clog%5C%7B(1-p)%2Fp%5C%7D">
or simply <!-- $w_e = -\log{p}$ --> <img style="transform: translateY(0.1em); background: white;" src="https://render.githubusercontent.com/render/math?math=w_e%20%3D%20-%5Clog%7Bp%7D">, we can safely use integers to save weights by e.g. scaling the weights by 1e6 and truncate to nearest integer.
In this way, the truncation error <!-- $\Delta w_e = 1$ --> <img style="transform: translateY(0.1em); background: white;" src="https://render.githubusercontent.com/render/math?math=%5CDelta%20w_e%20%3D%201"> of integer weights corresponds to relative error <!-- $\Delta p /{p}=10^{-6}$ --> <img style="transform: translateY(0.1em); background: white;" src="https://render.githubusercontent.com/render/math?math=%5CDelta%20p%20%2F%7Bp%7D%3D10%5E%7B-6%7D"> which is small enough.
Suppose physical error rate <!-- $p$ --> <img style="transform: translateY(0.1em); background: white;" src="https://render.githubusercontent.com/render/math?math=p"> is in the range of a `f64` variable (2.2e-308 to 1), the maximum weight is 7e7,which is well below
the maximum number of a `u32` variable (4.3e9). Since weights only sum up (no multiplication), `u32` is large enough and accurate enough.

We use integer also for ease of migrating to FPGA implementation. In order to fit more vertices into a single FPGA, it's necessary to reduce the
resource usage for each vertex. Integers are much cheaper than floating-point numbers, and also it allows flexible trade-off between resource usage and accuracy,
e.g. if all weights are equal, we can simply use a 2 bit integer.

Note that other libraries of MWPM solver like [Blossom V](https://doi.org/10.1007/s12532-009-0002-8) also default to integer weights.
Although one can change the macro to use floating-point weights, it's not recommended because "the code may even get stuck due to rounding errors".

## Installation

Here is an example installation on Ubuntu20.04.

```sh
# install rust compiler and package manager
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
# install build dependencies
sudo apt install build-essential
```

## Tests

In order to test the correctness of our MWPM solver, we need a ground truth MWPM solver.
[Blossom V](https://doi.org/10.1007/s12532-009-0002-8) is widely-used in existing MWPM decoders, but according to the license we cannot embed it in this library.
To run the test cases with ground truth comparison or enable the functions like `blossom_v_mwpm`, you need to download this library
[at this website](https://pub.ist.ac.at/~vnk/software.html) to a folder named `blossomV` at the root directory of this git repo.

```shell
wget -c https://pub.ist.ac.at/~vnk/software/blossom5-v2.05.src.tar.gz -O - | tar -xz
cp -r blossom5-v2.05.src/* blossomV/
rm -r blossom5-v2.05.src
```

# Visualize

To start a server, run the following
```sh
cd visualize
npm install  # to download packages
# you can choose to render locally or to view it in a browser interactively
# interactive: open url using a browser (Chrome recommended)
node index.js <url> <width> <height>  # local render

# for example you can run the following command to get url
cd ..
cargo test visualize_paper_weighted_union_find_decoder -- --nocapture
```

# TODOs

- [ ] add option for visualizer using high-quality renderer: https://github.com/erichlof/THREE.js-PathTracing-Renderer
