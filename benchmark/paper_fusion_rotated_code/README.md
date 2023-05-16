from ../paper_parallel_fusion_blossom but change codes to rotated code


# Experiment Note

remember to 
- download all data from AWS machine, just in case we need to check it later.
- change all planar code to rotated code
- change partition_num from 2000 to 1000
- when pymatching checks for number of vertices, change $(T+1)*d*(d+1)$ to $(T+1)*((d+1)*(d+1)/2)$

## partition_num_single_thread_2_tree

New optimal $\Delta T = 100$.

## decoding_time_d

reduced decoding time by roughly 2x, as expected.

## emulate_real_decoding

use $\Delta T = 100$, and increase measurement interval from 300us to 350us.

## fusion_time_d

as expected, 2x faster fusion because 2x fewer boundary vertices

## fusion_time_delta_T

as expected, fusion time doesn't change with delta T

## pymatching_compare_various_T

## fusion_time_children_count

## decoding_throughput_threads64

the rotated code now reaches d=33 with decoding time per round less than 1us!

## thread_pool_size_partition_1k

## emulate_decoding_d21_threads64
