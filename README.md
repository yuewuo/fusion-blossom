![Build Status](https://jenkins.fusionblossom.com/buildStatus/icon?job=FusionBlossomBuild&subject=build&style=flat-square)
![Test Status](https://jenkins.fusionblossom.com/buildStatus/icon?job=FusionBlossomCI&subject=test&style=flat-square)

# Fusion Blossom
A fast Minimum-Weight Perfect Matching (MWPM) solver for Quantum Error Correction (QEC)

## Key Features

- **Linear Complexity**: The decoding time is roughly $O(N)$, proportional to the number of syndrome vertices $N$.
- **Parallelism**: A single MWPM decoding problem can be partitioned and solved in parallel, then *fused* together to find an **exact** global MWPM solution.

## Benchmark Highlights

- In phenomenological noise model with **$p$ = 0.005**, code distance **$d$ = 21**, planar code with $d(d-1)$ = 420 $Z$ stabilizers, 100000 measurement rounds
  - single-thread: **3.4us per syndrome** or 41us per measurement round
  - 64-thread: 85ns per sydnrome or **1.0us per measurement round**

## Background and Key Ideas

MWPM decoders are widely known for its high accuracy [[1]](#fowler2012topological) and several optimizations that further improves its accuracy [[2]](#criger2018multi). However, there weren't many publications that improve the speed of the MWPM decoder over the past 10 years. Fowler implemented an $O(N)$ asymptotic complexity MWPM decoder in [[3]](#fowler2012towards) and proposed an $O(1)$ complexity parallel MWPM decoder in [[4]](#fowler2013minimum), but none of these are publicly available to our best knowledge. Higgott implemented a fast but approximate MWPM decoder (namely "local matching") with roughly $O(N)$ complexity in [[5]](#higgott2022pymatching). With recent experiments of successful QEC on real hardware, it's time for a fast and accurate MWPM decoder to become available to the community.

Our idea comes from our study on the Union-Find (UF) decoder [[6]](#delfosse2021almost). UF decoder is a fast decoder with $O(N)$ worst-case time complexity, at the cost of being less accurate compared to the MWPM decoder. Inspired by the Fowler's diagram [[3]](#fowler2012towards), we found a relationship between the UF decoder [[7]](#wu2022interpretation). This [nice animation](https://us.wuyue98.cn/aps2022/#/3/1) (press space to trigger animation) could help people see the analogy between UF and MWPM decoders. With this interpretation, we're able to combind the strength of UF and MWPM decoders together.

- From the UF decoder, we learnt to use a sparse decoding graph representation for fast speed
- From the MWPM decoder, we learnt to find an exact minimum-weight perfect matching for high accuracy

## Demo

We highly suggest you watch through several demos here to get a sense of how the algorithm works. All our demos are captured from real algorithm execution. In fact, we're showing you the visualized debugger tool we wrote for fusion blossom. The demo is a 3D website and you can control the view point as you like.

For more details of why it finds an exact MWPM, please read our paper [coming soon 💪].

Click the demo image below to view the corresponding demo

#### Serial Execution

[<img src="./visualize/img/serial_random.png" width="40%"/>](http://localhost:8066/?filename=primal_module_serial_basic_11.json)
[<img style="margin-left: 10%;" src="./visualize/img/serial_random.png" width="40%"/>](http://localhost:8066/?filename=primal_module_serial_basic_11.json)

#### Parallel Execution (Shown in Serial For Better Visual)



## Evaluation

We use Intel(R) Xeon(R) Platinum 8375C CPU for evaluation, with 64 physical cores and 128 threads. Note that Apple m1max CPU has roughly 2x single-core decoding speed, but it has limited number of cores so we do not use data from m1max. By default, we test phenomenological noise model with **$p$ = 0.005**, code distance **$d$ = 21**, planar code with $d(d-1)$ = 420 $Z$ stabilizers, 100000 measurement rounds.

First of all, the number of partitions will effect the speed. Intuitively, the more partitions there are, the more overhead because fusing two partitions consumes more computation than solving them as a whole. But in practice, memory access is not always at the same speed. If cache cannot hold the data, then solving big partition may consume even more time than solving small ones. We test on a single-thread decoder, and try different partition numbers. At partition number = 1000, we get roughly the minimum decoding time of 3.4us per syndrome. This corresponds to each partition hosting 100 measurement rounds.

![](./visualize/img/partition_num_single_thread.svg)

Given the optimal partition number of a single thread, we keep the partition number the same and try increasing the number of threads. Note that the partition number may not be optimal for large number of threads, but even in this case, we reach 41x speed up given 64 physical cores. The decoding time is pushed to 85ns per sydnrome or 1.0us per measurement round. This can catch up with the 1us measurement round of a superconducting circuit. Interestingly, we found that hyperthreading won't help much in this case, perhaps because this decoder is memory-bounded, meaning memory throughput is the bottleneck. Although the number of syndrome is only a small portion, they are randomly distributed so every time a new syndrome is given, the memory is almost always cold and incur large cache miss panelty.

![](./visualize/img/thread_pool_size_partition_1k.svg)

## Interface

#### Sparse Decoding Graph and Integer Weights

The weights in QEC decoding graph are computed by taking the log of error probability, e.g. $w_e = \log\{(1-p)/p\}$ or roughly $w_e = -\log{p}$, we can safely use integers to save weights by e.g. multiplying the weights by 1e6 and truncate to nearest integer. In this way, the truncation error $\Delta w_e = 1$ of integer weights corresponds to relative error $\Delta p /{p}=10^{-6}$ which is small enough. Suppose physical error rate $p$ is in the range of a positive `f64` variable (2.2e-308 to 1), the maximum weight is 7e7,which is well below the maximum value of a `u32` variable (4.3e9). Since weights only sum up in our algorithm (no multiplication), `u32` is large enough and accurate enough. By default we use `usize` which is platform dependent (usually 64 bits), but you can 

We use integer also for ease of migrating to FPGA implementation. In order to fit more vertices into a single FPGA, it's necessary to reduce the
resource usage for each vertex. Integers are much cheaper than floating-point numbers, and also it allows flexible trade-off between resource usage and accuracy, e.g. if all weights are equal, we can simply use a 2 bit integer.

Note that other libraries of MWPM solver like [Blossom V](https://doi.org/10.1007/s12532-009-0002-8) also default to integer weights as well. Although one can change the macro to use floating-point weights, it's not recommended because "the code may even get stuck due to rounding errors".

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

- [ ] bind to Python and publish to pip
- [ ] support erasures in parallel solver

# References
<a id="fowler2012topological">[1]</a> Fowler, Austin G., et al. "Topological code autotune." Physical Review X 2.4 (2012): 041003.

<a id="criger2018multi">[2]</a> Criger, Ben, and Imran Ashraf. "Multi-path summation for decoding 2D topological codes." Quantum 2 (2018): 102.

<a id="fowler2012towards">[3]</a> Fowler, Austin G., Adam C. Whiteside, and Lloyd CL Hollenberg. "Towards practical classical processing for the surface code: timing analysis." Physical Review A 86.4 (2012): 042313.

<a id="fowler2013minimum">[4]</a> Fowler, Austin G. "Minimum weight perfect matching of fault-tolerant topological quantum error correction in average $ O (1) $ parallel time." arXiv preprint arXiv:1307.1740 (2013).

<a id="higgott2022pymatching">[5]</a> Higgott, Oscar. "PyMatching: A Python package for decoding quantum codes with minimum-weight perfect matching." ACM Transactions on Quantum Computing 3.3 (2022): 1-16.

<a id="delfosse2021almost">[6]</a> Delfosse, Nicolas, and Naomi H. Nickerson. "Almost-linear time decoding algorithm for topological codes." Quantum 5 (2021): 595.

<a id="wu2022interpretation">[7]</a> Wu, Yue. APS 2022 March Meeting Talk "Interpretation of Union-Find Decoder on Weighted Graphs and Application to XZZX Surface Code". Slides: [https://us.wuyue98.cn/aps2022](https://us.wuyue98.cn/aps2022), Video: [https://youtu.be/BbhqUHKPdQk](https://youtu.be/BbhqUHKPdQk)
