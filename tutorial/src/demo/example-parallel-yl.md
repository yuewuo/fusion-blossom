# Example Parallel Configuration 

In this chapter, you will learn how about the configuration of graph partitions and different fusion plans as explained in the paper [1]. Make sure you understand the decoding and syndrome graph before proceeding. 

In paper [1], Fusion Blossom solves the MWPM problem with a Divide and Conquer approach. Fusion Blossom *divides* the decoding problem into two sub-problems that can be solved, or "conquered," independently and *fuses* their solutions recursively to obtain the overall solution. This recursive division/fusion is represented as a full binary tree, denoted as a *fusion tree*. 



## Code Generation

```python
import fusion_blossom as fb

code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
```

Current available QEC codes and noise models are as follows:
* `CodeCapacityRepetitionCode(d, p)`, where `d` is code distance, `p` is physical error rate. 
* `CodeCapacityPlanarCode(d, p)`
* `PhenomenologicalPlanarCode(d, noisy_measurements, p)`
* `CircuitLevelPlanarCode(d, noisy_measurements, p)`

More details can be found in the *Construct Decoding Graph* section. 

## Partition Configuration 

We can now proceed to define the partitions we would like to split our vertices into. 


[1] Wu, Yue, and Lin Zhong. "Fusion blossom: Fast mwpm decoders for qec." 2023 IEEE International Conference on Quantum Computing and Engineering (QCE). Vol. 1. IEEE, 2023.

