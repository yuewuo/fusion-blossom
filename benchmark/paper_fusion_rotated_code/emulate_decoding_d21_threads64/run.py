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
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
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
maximum_tree_leaf_size = 64  # see maximum_tree_leaf_size_64_threads
#measure_interval_vec = [0.5e-6 * (1.3 ** i) for i in range(20)]
measure_interval_vec = [0.2e-6 * (1.15 ** i) for i in range(20)]
# print(measure_interval_vec)
delta_T_vec = [100, 50, 20, 10]
interleaving_base_fusion = 2 * thread_pool_size + 1


# # small size debug
# d = 5
# noisy_measurements = 1000
# measure_interval_vec = measure_interval_vec[:5]



syndrome_file_path = os.path.join(tmp_dir, "generated.syndromes")
if os.path.exists(syndrome_file_path):
    print("[warning] use existing syndrome data (if you think it's stale, delete it and rerun)")
else:
    command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
    command += ["--code-type", "phenomenological-rotated-code"]
    command += ["--primal-dual-type", "error-pattern-logger"]
    command += ["--verifier", "none"]
    command += ["--primal-dual-config", f'{{"filename":"{syndrome_file_path}"}}']
    print(command)
    stdout, returncode = run_command_get_stdout(command)
    print("\n" + stdout)
    assert returncode == 0, "command fails..."

for delta_T in delta_T_vec:

    data_file = os.path.join(script_dir, f"data_deltaT{delta_T}.txt")
    with open(data_file, "w", encoding="utf8") as f:
        f.write("<measure_interval> <median_latency> <average_latency> <stddev_latency> <sample_latency>\n")

        for idx, measure_interval in enumerate(measure_interval_vec):
            partition_num = noisy_measurements // delta_T

            benchmark_profile_path = os.path.join(tmp_dir, f"deltaT{delta_T}_{'%.3e' % measure_interval}.profile")
            command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
            command += ["--code-type", "error-pattern-reader"]
            command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
            command += ["--primal-dual-type", "parallel"]
            command += ["--primal-dual-config", f'{{"primal":{{"thread_pool_size":{thread_pool_size},"pin_threads_to_cores":true,"streaming_decode_mock_measure_interval":{delta_T*measure_interval},"streaming_decode_use_spin_lock":true,"interleaving_base_fusion":{interleaving_base_fusion}}},"dual":{{"thread_pool_size":{thread_pool_size}}}}}']
            command += ["--partition-strategy", "phenomenological-rotated-code-time-partition"]
            # use `maximum_tree_leaf_size` to make sure fusion jobs are distributed to multiple cores while limiting the size of tree
            command += ["--partition-config", f'{{"partition_num":{partition_num},"enable_tree_fusion":true,"maximum_tree_leaf_size":{maximum_tree_leaf_size}}}']
            command += ["--verifier", "none"]
            command += ["--benchmark-profiler-output", benchmark_profile_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

            profile = Profile(benchmark_profile_path)
            latency_vec = []
            syndrome_ready_time = delta_T*measure_interval * (partition_num + 1)
            for entry in profile.entries:
                latency = entry["events"]["decoded"] - syndrome_ready_time
                latency_vec.append(latency)
            median_latency = np.median(latency_vec)
            average_latency = sum(latency_vec) / len(latency_vec)
            stddev_latency = math.sqrt(sum([(time - average_latency) ** 2 for time in latency_vec]) / len(latency_vec))
            samples_str = ["%.3e" % time for time in latency_vec]
            print(f"measure_interval {measure_interval}: median {median_latency}, average {average_latency}, stddev {stddev_latency}")
            f.write("%.5e %.5e %.5e %.3e %s\n" % (
                measure_interval,
                median_latency,
                average_latency,
                stddev_latency,
                "[" + ",".join(samples_str) + "]",
            ))
            f.flush()
