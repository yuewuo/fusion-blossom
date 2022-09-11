# Benchmark

## General Instruction

In order to test pure decoding performance efficiently, one should consider first generate the syndrome patterns to a `.syndrome` file
and then try different configurations on the same syndrome patterns.
This also improves cache efficiency, because generating random syndrome patterns would iterate over all edges and thus pollute the cache.

#### To generate a syndrome pattern file:

```sh
cargo run --release -- benchmark 15 -n 10000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --primal-dual-config '{"filename":"tmp/15-10000-0.005-phenomenological-planar.syndromes"}' --verifier none
```

In this example,

- I'm using a code type with suffix `-parallel`. This option enables parallel computing when generating error patterns and thus utilize multi-core CPU more efficiently. The configuration `--code-config '{"code_count":10}'` specifies how many parallel instances is created and computing in parallel.
- The `--primal-dual-type error-pattern-logger` option is used to output all syndrome patterns into a file, specified by `'{"filename":"tmp/15-10000-0.005-phenomenological-planar.syndromes"}'`.
- `--verifier none` is necessary to skip the verifier procedure, because our solver `error-pattern-logger` is not a real solver but just a syndrome pattern recorder.

#### To use a syndrome pattern file:

```sh
cargo run --release -- benchmark 15 -n 10000 0.005 --code-type error-pattern-reader --code-config '"filename":"tmp/15-10000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":4,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output visualize/data/15-10000-0.005-phenomenological-planar/tree-16.profile
```

In this example,

- One should use the exactly same options such as `d`, `noisy_measurements`, `p` as when generating the syndrome pattern files, otherwise it might cause incompatibility.
- The `--code-type error-pattern-reader` option is used to load syndrome patterns from file instead of generating them dynamically. The input file is specified by `--code-config '"filename":"tmp/15-10000-0.005-phenomenological-planar.syndromes"}'`
- I'm using `--primal-dual-type parallel`, the parallel primal and dual solver. The number of parallel tasks are running in a thread pool is determined by `rayon`'s default value, unless specified by user in `--primal-dual-config`.
- The partition strategy is specified. Since the partition is independent from syndrome pattern generation, a single `.syndromes` file can be used by benchmarking multiple different partition strategies.
- `--verifier none` is suggested when benchmarking the speed, since a verifier is generally much slower than the solver to be benchmarked.
- An output file is specified by `--benchmark-profiler-output visualize/data/15-10000-0.005-phenomenological-planar/tree-16.profile`, which can be later visualized by opening the visualization tool in a browser: `/visualize/partition-profile.html?filename=15-10000-0.005-phenomenological-planar/tree-16.profile`
