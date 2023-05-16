
AWS m6i.metal instance, with max 128 CPUs (64 cores each 2 thread); price is 6.144USB/hour.
or AWS m6i.4xlarge instance: 16 vCPU and 64 GB memory
should be the same CPU, latter is virtual matching
PyMatching requires large memory to read the syndrome in Python, so it uses m6i.metal (also run on m6i.4xlarge for small ones, and data is almost the same, see commit bb7742f)
Parity Blossom and Fusion Blossom uses m6i.4xlarge.

I found that PyMatching V2 runs much slower if T is too large.
This is probably because priority queue's time complexity or simply because cache miss rate given large memory usage.
Fusion Blossom, on the other hand, can partition the graph to small pieces and thus is immune to this effect when T is sufficient large.
I'm going to show various T values and how the average decoding time per round changes.
