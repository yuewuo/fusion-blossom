
# Observation

The following configuration has bad latency scaling

```python
d = 21
p = 0.005
total_rounds = 100
noisy_measurements = 100000
thread_pool_size = 64
maximum_tree_leaf_size = 64  # see maximum_tree_leaf_size_64_threads
measure_interval_vec = [0.2e-6 * (1.15 ** i) for i in range(20)]
delta_T_vec = [100, 50, 20, 10]
interleaving_base_fusion = 2 * thread_pool_size + 1
```

I need to figure out the best configuration to use, and then evaluate it on different delta_T choices

## run_optimal_subtree_size

the optimal subtree size is actually not 64 (the number of threads), as shown in `optimal_subtree_size.txt`.
small value results in decoding time accumulation.

The optimal value is maximum_tree_leaf_size = 100. I'll stick to this value in the following tests

## tree structure

It turns out that the latency also depends on the last few leaf partitions and when they finish.
I'll not develop a new partition strategy given limited time.
Now what I can do is to just partition_num slightly and see what happens.
Maybe, some cases have better latency than others :)

