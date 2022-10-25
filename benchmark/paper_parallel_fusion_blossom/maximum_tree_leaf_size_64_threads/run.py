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

syndrome_file_path = os.path.join(tmp_dir, "generated.syndromes")
if os.path.exists(syndrome_file_path):
    print("[warning] use existing syndrome data (if you think it's stale, delete it and rerun)")
else:
    command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
    command += ["--code-type", "phenomenological-planar-code"]
    command += ["--primal-dual-type", "error-pattern-logger"]
    command += ["--verifier", "none"]
    command += ["--primal-dual-config", f'{{"filename":"{syndrome_file_path}"}}']
    print(command)
    stdout, returncode = run_command_get_stdout(command)
    print("\n" + stdout)
    assert returncode == 0, "command fails..."

"""
Run simulations

study the effect of maximum_tree_leaf_size given 64 threads

expectation: when maximum_tree_leaf_size is 1, all fusions are executed sequentially, so the performance will be bad;
    when maximum_tree_leaf_size is too large, the tree is too high and the final fusion will take a lot of time
    a middle maximum_tree_leaf_size value is expected to reach the best performance, although it could be much larger than the number of threads

"""

repeat_vec = [8, 9, 12]
maximum_tree_leaf_size_vec = [1, 2, 3, 4, 5, 6, 7]
for i in range(7):
    maximum_tree_leaf_size_vec += [e * (2 ** i) for e in repeat_vec]
maximum_tree_leaf_size_vec += [1000]  # for a full tree
maximum_tree_leaf_size_vec.sort()
print(maximum_tree_leaf_size_vec)
benchmark_profile_path_vec = []
for maximum_tree_leaf_size in maximum_tree_leaf_size_vec:
    benchmark_profile_path = os.path.join(tmp_dir, f"{maximum_tree_leaf_size}.profile")
    benchmark_profile_path_vec.append(benchmark_profile_path)
    if os.path.exists(benchmark_profile_path):
        print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
    else:
        command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
        command += ["--code-type", "error-pattern-reader"]
        command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
        command += ["--primal-dual-type", "parallel"]
        command += ["--primal-dual-config", '{"primal":{"thread_pool_size":64,"pin_threads_to_cores":true},"dual":{"thread_pool_size":64}}']  # keep using single thread
        command += ["--partition-strategy", "phenomenological-planar-code-time-partition"]
        command += ["--partition-config", f'{{"partition_num":1000,"enable_tree_fusion":true,"maximum_tree_leaf_size":{maximum_tree_leaf_size}}}']
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
    f.write("<maximum_tree_leaf_size> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect>\n")
    for idx, maximum_tree_leaf_size in enumerate(maximum_tree_leaf_size_vec):
        benchmark_profile_path = benchmark_profile_path_vec[idx]
        print(benchmark_profile_path)
        profile = Profile(benchmark_profile_path)
        print("maximum_tree_leaf_size:", maximum_tree_leaf_size)
        print("    average_decoding_time:", profile.average_decoding_time())
        print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
        print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
        print("    average_defect_per_measurement:", profile.sum_defect_num() / (noisy_measurements + 1) / len(profile.entries))
        f.write("%d %.5e %.5e %.5e\n" % (
            maximum_tree_leaf_size,
            profile.average_decoding_time(),
            profile.average_decoding_time() / (noisy_measurements + 1),
            profile.average_decoding_time_per_defect(),
        ))
