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

"""
First generate syndrome data under this folder
"""

d = 21
p = 0.005
total_rounds = 100
noisy_measurements = 10000

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
    command += ["--use-brief-edge"]  # to save memory
    print(command)
    stdout, returncode = run_command_get_stdout(command)
    print("\n" + stdout)
    assert returncode == 0, "command fails..."

"""
Run simulations

study the effect of partition_num given a single thread

expectation: when partition_num is small, the performance should not be affected; only if partition_num is greater than 1000 where each partition has <100
    measurement rounds will the performance starts to degrade

"""

partition_num_vec = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
repeat_vec = [10, 15, 22, 33, 50, 75]
partition_num_vec += [e for e in repeat_vec]
partition_num_vec += [e * 10 for e in repeat_vec]
# partition_num_vec += [int(noisy_measurements/e) for e in range(10, 1, -1)]  # how many measurement rounds between two fusion
print(partition_num_vec)
benchmark_profile_path_vec = []
for partition_num in partition_num_vec:
    benchmark_profile_path = os.path.join(tmp_dir, f"{partition_num}.profile")
    benchmark_profile_path_vec.append(benchmark_profile_path)
    if os.path.exists(benchmark_profile_path):
        print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
    else:
        command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
        command += ["--code-type", "error-pattern-reader"]
        command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
        command += ["--primal-dual-type", "parallel"]
        command += ["--primal-dual-config", '{"primal":{"thread_pool_size":1,"pin_threads_to_cores":true},"dual":{"thread_pool_size":1}}']  # keep using single thread
        command += ["--partition-strategy", "phenomenological-rotated-code-time-partition"]
        command += ["--partition-config", f'{{"partition_num":{partition_num},"enable_tree_fusion":true}}']
        command += ["--verifier", "none"]
        command += ["--benchmark-profiler-output", benchmark_profile_path]
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."


"""
Gather useful data
"""

data_file = os.path.join(script_dir, "data.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write("<partition_num> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect>\n")
    for idx, partition_num in enumerate(partition_num_vec):
        benchmark_profile_path = benchmark_profile_path_vec[idx]
        print(benchmark_profile_path)
        profile = Profile(benchmark_profile_path)
        print("partition_num:", partition_num)
        print("    average_decoding_time:", profile.average_decoding_time())
        print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
        print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
        print("    average_defect_per_measurement:", profile.sum_defect_num() / (noisy_measurements + 1) / len(profile.entries))
        f.write("%d %.5e %.5e %.5e\n" % (
            partition_num,
            profile.average_decoding_time(),
            profile.average_decoding_time() / (noisy_measurements + 1),
            profile.average_decoding_time_per_defect(),
        ))
