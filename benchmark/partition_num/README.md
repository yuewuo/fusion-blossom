# Partition Number

test how `partition_num` effects decoding speed given a large enough decoding problem.

## About my machine

```ini
Architecture:                    x86_64
CPU op-mode(s):                  32-bit, 64-bit
Byte Order:                      Little Endian
Address sizes:                   46 bits physical, 48 bits virtual
CPU(s):                          48
On-line CPU(s) list:             0-47
Thread(s) per core:              2
Core(s) per socket:              24
Socket(s):                       1
NUMA node(s):                    1
Vendor ID:                       GenuineIntel
CPU family:                      6
Model:                           85
Model name:                      Intel(R) Xeon(R) Platinum 8275CL CPU @ 3.00GHz
Stepping:                        7
CPU MHz:                         2999.998
BogoMIPS:                        5999.99
Hypervisor vendor:               KVM
Virtualization type:             full
L1d cache:                       768 KiB
L1i cache:                       768 KiB
L2 cache:                        24 MiB
L3 cache:                        35.8 MiB
NUMA node0 CPU(s):               0-47
Vulnerability Itlb multihit:     KVM: Mitigation: VMX unsupported
Vulnerability L1tf:              Mitigation; PTE Inversion
Vulnerability Mds:               Vulnerable: Clear CPU buffers attempted, no microcode; SMT Host state unknown
Vulnerability Meltdown:          Mitigation; PTI
Vulnerability Spec store bypass: Vulnerable
Vulnerability Spectre v1:        Mitigation; usercopy/swapgs barriers and __user pointer sanitization
Vulnerability Spectre v2:        Mitigation; Retpolines, STIBP disabled, RSB filling
Vulnerability Srbds:             Not affected
Vulnerability Tsx async abort:   Not affected
Flags:                           fpu vme de pse tsc msr pae mce cx8 apic sep mtrr pge mca cmov pat pse36 clflush mmx fxsr sse sse2 ss ht syscall nx pdpe1gb rdtscp lm constant_tsc arch_perfmon rep_good nopl xt
                                 opology nonstop_tsc cpuid aperfmperf tsc_known_freq pni pclmulqdq monitor ssse3 fma cx16 pcid sse4_1 sse4_2 x2apic movbe popcnt tsc_deadline_timer aes xsave avx f16c rdrand hy
                                 pervisor lahf_lm abm 3dnowprefetch invpcid_single pti fsgsbase tsc_adjust bmi1 avx2 smep bmi2 erms invpcid mpx avx512f avx512dq rdseed adx smap clflushopt clwb avx512cd avx512
                                 bw avx512vl xsaveopt xsavec xgetbv1 xsaves ida arat pku ospke avx512_vnni
```

## Generating syndrome patterns

I'm using an AWS c5.12xlarge instance, which has 48vCPUs and 96GB of memory.
The following command is designed for this machine, by adjusting `code_count` (parallelism) so that the random syndrome generation
is accelerated while still fit into the memory.
Peak memory usage is about 80GB.

About file size: d = 15 generates a 7.5GB file, so the overall space needed is roughly 60GB.

```sh
# d = 3
cargo run --release -- benchmark 3 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/3-100000-0.005-phenomenological-planar.syndromes"}'
# d = 5
cargo run --release -- benchmark 5 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/5-100000-0.005-phenomenological-planar.syndromes"}'
# d = 7
cargo run --release -- benchmark 7 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/7-100000-0.005-phenomenological-planar.syndromes"}'
# d = 9
cargo run --release -- benchmark 9 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/9-100000-0.005-phenomenological-planar.syndromes"}'
# d = 11
cargo run --release -- benchmark 11 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/11-100000-0.005-phenomenological-planar.syndromes"}'
# d = 13
cargo run --release -- benchmark 13 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/13-100000-0.005-phenomenological-planar.syndromes"}'
# d = 15
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":10}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}'
# d = 17
cargo run --release -- benchmark 17 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":8}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/17-100000-0.005-phenomenological-planar.syndromes"}'
# d = 19
cargo run --release -- benchmark 19 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":6}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/19-100000-0.005-phenomenological-planar.syndromes"}'
# d = 21
cargo run --release -- benchmark 21 -n 100000 0.005 --code-type phenomenological-planar-code-parallel --code-config '{"code_count":5}' --primal-dual-type error-pattern-logger --verifier none --primal-dual-config '{"filename":"benchmark/partition_num/21-100000-0.005-phenomenological-planar.syndromes"}'
```

## Evaluating decoding time under various partition number

I pick data from `d = 15` because for any `partition_num < 50`, there are more than 2000 rounds for each partition which is more than 100 times `d`.
Also, for each partition, the average number of syndrome is at least $2000 \times 15^2 \times 0.005 \times 4 = 9000$, which is large enough to minimize the overhead of fusion.

I use tree fusion `"enable_tree_fusion":true` since it's shown to reduce decoding time on average.

```sh
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":1,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-1.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":2,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-2.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":3,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-3.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":4,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-4.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":6,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-6.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":8,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-8.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":12,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-12.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":16,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-16.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":24,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-24.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":32,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-32.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":48,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-48.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":64,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-64.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":96,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-96.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":128,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-128.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":192,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-192.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":256,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-256.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":384,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-384.profile
cargo run --release -- benchmark 15 -n 100000 0.005 --code-type error-pattern-reader --code-config '{"filename":"benchmark/partition_num/15-100000-0.005-phenomenological-planar.syndromes"}' --primal-dual-type parallel --partition-strategy phenomenological-planar-code-time-partition --partition-config '{"partition_num":512,"enable_tree_fusion":true}' --verifier none --benchmark-profiler-output benchmark/partition_num/15-100000-0.005-phenomenological-planar-tree-512.profile
```

## Results

```sh
  1: total:  9.281e0, round: 9.281e-5, syndrome: 1.544e-5
  2: total:  4.975e0, round: 4.975e-5, syndrome: 8.275e-6
  3: total:  3.570e0, round: 3.570e-5, syndrome: 5.939e-6
  4: total:  2.801e0, round: 2.801e-5, syndrome: 4.659e-6
  6: total:  1.992e0, round: 1.992e-5, syndrome: 3.313e-6
  8: total:  1.562e0, round: 1.562e-5, syndrome: 2.599e-6
 12: total:  1.183e0, round: 1.183e-5, syndrome: 1.967e-6
 16: total: 9.933e-1, round: 9.933e-6, syndrome: 1.652e-6
 24: total: 8.172e-1, round: 8.171e-6, syndrome: 1.359e-6
 32: total: 7.647e-1, round: 7.647e-6, syndrome: 1.272e-6
 48: total: 6.674e-1, round: 6.674e-6, syndrome: 1.110e-6
 64: total: 7.936e-1, round: 7.936e-6, syndrome: 1.320e-6
 96: total: 7.287e-1, round: 7.287e-6, syndrome: 1.212e-6
128: total: 7.347e-1, round: 7.347e-6, syndrome: 1.222e-6
192: total: 7.270e-1, round: 7.270e-6, syndrome: 1.209e-6
256: total: 7.233e-1, round: 7.233e-6, syndrome: 1.203e-6
384: total: 7.372e-1, round: 7.372e-6, syndrome: 1.226e-6
512: total: 7.269e-1, round: 7.269e-6, syndrome: 1.209e-6
```
