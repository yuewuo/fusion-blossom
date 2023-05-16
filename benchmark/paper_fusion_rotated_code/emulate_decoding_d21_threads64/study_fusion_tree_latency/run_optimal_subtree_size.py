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

maximum_tree_leaf_size_vec = [i for i in range(64, 96-5, 5)] + [i for i in range(96-5, 96+6)] + [128, 192, 256, 384, 512, 768, 1024]
delta_T = 20
interleaving_base_fusion = 2 * thread_pool_size + 1

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

# # small size debug
# noisy_measurements = 10000
# delta_T = 5


data_file = os.path.join(script_dir, f"optimal_subtree_size.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write("<maximum_tree_leaf_size> <predicted_latency>\n")
    for maximum_tree_leaf_size in maximum_tree_leaf_size_vec:
        average_fusion_time = fusion_time(math.log2(maximum_tree_leaf_size) / 2)

        partition_num = noisy_measurements // delta_T

        command = fusion_blossom_bin_command("partition-strategy")
        command += ["phenomenological-rotated-code-time-partition-vec"]
        command += [f"{d}"]
        command += [f"{noisy_measurements}"]
        command += [f"[{partition_num}]"]
        command += [f"--enable-tree-fusion"]
        command += [f"--maximum-tree-leaf-size-vec", f"[{maximum_tree_leaf_size}]"]

        # print(command)
        stdout, returncode = run_command_get_stdout(command)
        # print("\n" + stdout)
        assert returncode == 0, "command fails..."

        lines = stdout.strip("\r\n ").split("\n")
        assert len(lines) == 2
        line = lines[0].strip("\r\n ")
        assert json.loads(line)["partition_num"] == partition_num
        line = lines[1].strip("\r\n ")
        partition_config = PartitionConfig.from_json(json.loads(line))
        last_partition_index = len(partition_config.partitions) - 1
        depth_vec = [partition_config.unit_depth(last_partition_index - i) for i in range(len(partition_config.partitions))]
        predicted_latency = 0
        for j, depth in enumerate(depth_vec):
            latency = leaf_partition_time + depth * average_fusion_time - j * measurement_cycle
            predicted_latency = max(predicted_latency, latency)

        print(f"maximum_tree_leaf_size {maximum_tree_leaf_size}: predicted_latency {predicted_latency}")
        f.write("%d %.5e\n" % (
            maximum_tree_leaf_size,
            predicted_latency,
        ))
        f.flush()
