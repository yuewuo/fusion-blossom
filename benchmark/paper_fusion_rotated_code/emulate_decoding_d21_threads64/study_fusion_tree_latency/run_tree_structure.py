"""
batch decoding receives all syndrome data and start decoding
"""

import enum
import os, sys
import subprocess, sys
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
sys.path.insert(0, benchmark_dir)

import util
from util import *
util.FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER = True  # better performance, still safe
compile_code_if_necessary()
import numpy as np

d = 21
p = 0.005
total_rounds = 100
noisy_measurements = 100000
thread_pool_size = 64
maximum_tree_leaf_size = 100
delta_T = 20
interleaving_base_fusion = 2 * thread_pool_size + 1
partition_num_delta = 100

# determine which number is optimum
ranking_test_count = 1000
measurement_cycle = 1e-6  # targeting 1us physical measurement cycle
gnuplot_data = GnuplotData(os.path.join(script_dir, "..", "..", "fusion_time_children_count", "balance_tree.txt"))
assert gnuplot_data.data[0][0] == '1'  # 1 child, meaning leaf partition
leaf_partition_time = float(gnuplot_data.data[0][1]) / 100 * delta_T  # see ../../fusion_time_children_count/balance_tree
print(f"leaf_partition_time: {leaf_partition_time*1e6:.2f}us")
delta_depth = math.log2(int(gnuplot_data.data[-1][0])) - math.log2(int(gnuplot_data.data[1][0]))
delta_fusion_time = float(gnuplot_data.data[-1][1]) - float(gnuplot_data.data[1][1])
basic_fusion_time = float(gnuplot_data.data[1][1])
print(f"basic_fusion_time: {basic_fusion_time*1e6:.2f}us")
def fusion_time(depth):  # see ../../fusion_time_children_count/balance_tree
    return basic_fusion_time + delta_fusion_time / delta_depth * depth
print(f"depth10_fusion_time: {fusion_time(10)*1e6:.2f}us")
average_fusion_time = fusion_time(math.log2(maximum_tree_leaf_size) / 2)

# small size debug
# noisy_measurements = 10000
# delta_T = 5


data_file = os.path.join(script_dir, f"tree_structure.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write("<partition_num> <predicted_latency> <max_latency_j> <latency_vec(ms)>\n")

    partition_num_center = noisy_measurements // delta_T
    partition_num_vec = [partition_num_center + i for i in range(-partition_num_delta, partition_num_delta+1)]
    # partition_num_vec = [2048]

    command = fusion_blossom_bin_command("partition-strategy")
    command += ["phenomenological-rotated-code-time-partition-vec"]
    command += [f"{d}"]
    command += [f"{noisy_measurements}"]
    command += [f"[" + ",".join([str(e) for e in partition_num_vec]) + "]"]
    command += [f"--enable-tree-fusion"]
    command += [f"--maximum-tree-leaf-size-vec", f"[{maximum_tree_leaf_size}]"]

    # print(command)
    stdout, returncode = run_command_get_stdout(command)
    # print("\n" + stdout)
    assert returncode == 0, "command fails..."

    lines = stdout.strip("\r\n ").split("\n")
    assert len(lines) == 2 * len(partition_num_vec)
    for idx, partition_num in enumerate(partition_num_vec):
        line = lines[0 + 2 * idx].strip("\r\n ")
        assert json.loads(line)["partition_num"] == partition_num
        line = lines[1 + 2 * idx].strip("\r\n ")
        partition_config = PartitionConfig.from_json(json.loads(line))
        last_partition_index = len(partition_config.partitions) - 1
        depth_vec = [partition_config.unit_depth(last_partition_index - i) for i in range(len(partition_config.partitions))]
        predicted_latency = 0

        display_latency_number = 10
        display_latency_vec = []
        max_latency_j = 0
        for j, depth in enumerate(depth_vec):
            latency = leaf_partition_time + depth * average_fusion_time - j * measurement_cycle
            if latency > predicted_latency:
                max_latency_j = j
            predicted_latency = max(predicted_latency, latency)
            if j < display_latency_number:
                display_latency_vec.append(latency)

        print(f"partition_num {partition_num}: predicted_latency {predicted_latency} (max_latency_j: {max_latency_j})")
        display_latency_str_vec = [f'{e*1e3:.3e}' for e in display_latency_vec]
        display_latency_str = f"[{','.join(display_latency_str_vec)}]"
        f.write("%d %.5e %d %s\n" % (
            partition_num,
            predicted_latency,
            max_latency_j,
            display_latency_str,
        ))
        f.flush()
