import numpy as np
from scipy.sparse import csc_matrix, lil_matrix
import pymatching
import os, sys, time
import subprocess
from msgspec.json import decode
from msgspec import Struct

d = 21
p = 0.005
total_rounds = 100
delta_T_vec = [1, 2, 3, 4, 5, 6, 7, 9, 11, 13, 15, 18, 22, 27, 32, 38, 46, 55, 66, 79, 95, 114, 137, 165, 198, 237, 285, 342, 410, 492, 591, 709, 851, 1021]

# last_T = 0
# for i in range(100):
#     T = round(1.2 ** i)
#     if T != last_T:
#         delta_T_vec.append(T)
#         last_T = T
#     if T > 2000:
#         break

# delta_T_vec = delta_T_vec[:10]  # small-scale debug

# first generate graph
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

data_file = os.path.join(script_dir, "data.txt")
with open(data_file, "w", encoding="utf8") as data_f:
    data_f.write("<delta_T> <average_fusion_time> <stddev_time> <samples>\n")

    for delta_T in delta_T_vec:
        partition_num = 32
        noisy_measurements = partition_num * (delta_T + 1) - 2
        syndrome_file_path = os.path.join(tmp_dir, f"generated.T{noisy_measurements}.syndromes")
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

        # run simulation
        benchmark_profile_path = os.path.join(tmp_dir, f"T{noisy_measurements}.parity.profile")
        command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
        command += ["--code-type", "error-pattern-reader"]
        command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
        command += ["--primal-dual-type", "parallel"]
        command += ["--primal-dual-config", f'{{"primal":{{"thread_pool_size":1,"pin_threads_to_cores":true}},"dual":{{"thread_pool_size":1}}}}']
        command += ["--partition-strategy", "phenomenological-rotated-code-time-partition"]
        command += ["--partition-config", f'{{"partition_num":{partition_num},"enable_tree_fusion":true}}']
        command += ["--verifier", "none"]
        command += ["--benchmark-profiler-output", benchmark_profile_path]
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."

        profile = Profile(benchmark_profile_path)
        config = profile.partition_config
        for i in range(partition_num):  # check partition is indeed delta_T height
            assert config.partitions[i].length() == d * (d+1) * delta_T
        fusion_time_vec = []
        for entry in profile.entries:
            event_time_vec = entry["solver_profile"]["primal"]["event_time_vec"]
            assert len(event_time_vec) == 2 * partition_num - 1
            event_time = event_time_vec[-1]
            fusion_time = event_time["end"] - event_time["start"]
            fusion_time_vec.append(fusion_time)
        average_time = sum(fusion_time_vec) / len(fusion_time_vec)
        stddev_time = math.sqrt(sum([(time - average_time) ** 2 for time in fusion_time_vec]) / len(fusion_time_vec))
        samples_str = ["%.3e" % time for time in fusion_time_vec]
        print(f"delta_T {delta_T}: average {average_time}, stddev {stddev_time}")
        data_f.write("%d %.5e %.3e %s\n" % (
            delta_T,
            average_time,
            stddev_time,
            "[" + ",".join(samples_str) + "]",
        ))
        data_f.flush()
