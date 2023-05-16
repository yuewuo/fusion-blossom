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
noisy_measurements = 100000
partition_num = 1000

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

"""
Run simulations

study the effect of thread numbers given the optimal partition_num for a single thread

expectation: when partition_num is small, the performance should not be affected; only if partition_num is greater than 1000 where each partition has <100
    measurement rounds will the performance starts to degrade

"""

thread_pool_size_vec = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256]
print(thread_pool_size_vec)
benchmark_profile_path_vec = []
for thread_pool_size in thread_pool_size_vec:
    benchmark_profile_path = os.path.join(tmp_dir, f"{thread_pool_size}.profile")
    benchmark_profile_path_vec.append(benchmark_profile_path)
    if os.path.exists(benchmark_profile_path):
        print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
    else:
        command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
        command += ["--code-type", "error-pattern-reader"]
        command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
        command += ["--primal-dual-type", "parallel"]
        command += ["--primal-dual-config", f'{{"primal":{{"thread_pool_size":{thread_pool_size},"pin_threads_to_cores":true}},"dual":{{"thread_pool_size":{thread_pool_size}}}}}']
        command += ["--partition-strategy", "phenomenological-rotated-code-time-partition"]
        # use `maximum_tree_leaf_size` to make sure fusion jobs are distributed to multiple cores while limiting the size of tree
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
    f.write("<thread_pool_size> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect>\n")
    for idx, thread_pool_size in enumerate(thread_pool_size_vec):
        benchmark_profile_path = benchmark_profile_path_vec[idx]
        print(benchmark_profile_path)
        profile = Profile(benchmark_profile_path)
        print("thread_pool_size:", thread_pool_size)
        print("    average_decoding_time:", profile.average_decoding_time())
        print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
        print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
        print("    average_defect_per_measurement:", profile.sum_defect_num() / (noisy_measurements + 1) / len(profile.entries))
        print("    decoding_time_relative_dev:", profile.decoding_time_relative_dev())
        f.write("%d %.5e %.5e %.5e %.3e\n" % (
            thread_pool_size,
            profile.average_decoding_time(),
            profile.average_decoding_time() / (noisy_measurements + 1),
            profile.average_decoding_time_per_defect(),
            profile.decoding_time_relative_dev(),
        ))
