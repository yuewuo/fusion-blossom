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

ONLY_PROCESS_DATA_IN_FOLDER = None
# ONLY_PROCESS_DATA_IN_FOLDER = "raw-data-2023-04-11"  # only process data, don't run simulation

d = 21
p = 0.001
total_rounds = 200
benchmark_total_run = 3 * total_rounds  # run benchmark longer to get rid of cold start
noisy_measurements = 100000
thread_pool_size = 128
maximum_tree_leaf_size_vec = [1000, 100, 10]  # see ./study_fusion_tree_latency
measure_interval_vec = [0.2e-6 * (1.15 ** i) for i in range(20)]
# print(measure_interval_vec)
delta_T = 20  # roughly 10us * 20 = 200us base decoding time; in the ideal case, subtree = 8, roughly 600us latency
interleaving_base_fusion = 2 * thread_pool_size + 1


# # small size debug
# noisy_measurements = 100
# measure_interval_vec = [1e-6]


if ONLY_PROCESS_DATA_IN_FOLDER == None:
    syndrome_file_path = os.path.join(tmp_dir, "generated.syndromes")
    if os.path.exists(syndrome_file_path):
        print("[warning] use existing syndrome data (if you think it's stale, delete it and rerun)")
    else:
        command = fusion_blossom_qecp_generate_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=min(d, 4))
        command += ["--code-type", "rotated-planar-code"]
        command += ["--noise-model", "stim-noise-model"]
        command += ["--decoder", "fusion", "--decoder-config", '{"only_stab_z":true,"use_combined_probability":false,"skip_decoding":true}']
        command += ["--debug-print", "fusion-blossom-syndrome-file", "--fusion-blossom-syndrome-export-filename", syndrome_file_path]
        command += ["--use-compact-simulator", "--use-compact-simulator-compressed"]
        command += ["--simulator-compact-extender-noisy-measurements", f"{noisy_measurements}"]
        command += ["--parallel", "0"]  # use all cores
        command += ["--use-brief-edge"]  # to save memory; it takes 23GB to run d=97 and 15min to initialize it...
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."
else:
    tmp_dir = os.path.join(script_dir, ONLY_PROCESS_DATA_IN_FOLDER)

for maximum_tree_leaf_size in maximum_tree_leaf_size_vec:

    data_file = os.path.join(script_dir, f"data_subtree{maximum_tree_leaf_size}.txt")
    with open(data_file, "w", encoding="utf8") as f:
        f.write("<measure_interval> <median_latency> <average_latency> <stddev_latency> <sample_latency>\n")

        for idx, measure_interval in enumerate(measure_interval_vec):
            partition_num = noisy_measurements // delta_T

            benchmark_profile_path = os.path.join(tmp_dir, f"subtree{maximum_tree_leaf_size}_{'%.3e' % measure_interval}.profile")

            if ONLY_PROCESS_DATA_IN_FOLDER == None:
                command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=benchmark_total_run, noisy_measurements=noisy_measurements)
                command += ["--code-type", "error-pattern-reader"]
                command += ["--code-config", f'{{"filename":"{syndrome_file_path}","cyclic_syndrome":true}}']
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

            profile = Profile(benchmark_profile_path, benchmark_total_run-total_rounds)
            latency_vec = []
            syndrome_ready_time = delta_T*measure_interval * partition_num
            for entry in profile.entries:
                event_time_vec = entry["solver_profile"]["primal"]["event_time_vec"]
                last_fusion_finished = event_time_vec[-1]["end"]
                latency = last_fusion_finished - syndrome_ready_time
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
