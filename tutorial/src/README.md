# Introduction

**fusion-blossom** is a fast Minimum-Weight Perfect Matching (MWPM) solver for Quantum Error Correction (QEC).

## Key Features

- **Correctness**: This is an exact MWPM solver, verified against the [Blossom V library](https://pub.ist.ac.at/~vnk/software.html) with millions of randomized test cases.
- **Linear Complexity**: The decoding time is roughly \\( O(N) \\) given small physical error rate, proportional to the number of syndrome vertices \\( N \\).
- **Parallelism**: A single MWPM decoding problem can be partitioned and solved in parallel, then *fused* together to find an **exact** global MWPM solution.
- **Simple Interface**: The graph problem is abstracted and optimized for QEC decoders.

## Chapters

For beginners, please read how the MWPM decoder works and how we abstract the MWPM decoder interface in [Problem Definition Chapter](problem_definition.md).

For experts in fusion-blossom, please jump to [Installation](installation.md) or directly go to [demos](demo/example-qec-codes.md).

This library is written in Rust programming language for speed and memory safety, however, we do provide a [Python package](https://pypi.org/project/fusion-blossom/) that expose commonly used functions.
Users can install the library using `pip3 install fusion-blossom`.



# Contributing

**fusion-blossom** is free and open source.
You can find the source code on [GitHub](https://github.com/yuewuo/fusion-blossom), where you can report issues and feature requests using [GitHub issue tracker](https://github.com/yuewuo/fusion-blossom/issues).
Currently the project is maintained by [Yue Wu](https://wuyue98.cn/) at [Yale Efficient Computing Lab](http://www.yecl.org/).
If you'd like to contribute to the project, consider [emailing the maintainer](mailto:yue.wu@yale.edu) or opening a [pull request](https://github.com/yuewuo/fusion-blossom/pulls).

# License

The fusion-blossom library is released under the [MIT License](https://opensource.org/licenses/MIT).
