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

T_vec = [1,2,3,5,10,100]  # for debugging script
T_vec = [1, 2, 3, 4, 5, 6, 7, 9, 11, 13, 15, 18, 22, 27, 32, 38, 46, 55, 66, 79, 95, 114, 137, 165, 198, 237, 285, 342, 410, 492, 591, 709, 851, 1021]
d_vec = [3,5]  # for debugging script
d_vec = [3, 9, 27]
p = 0.005
total_rounds = 1000

"""
command to estimate the decoding time: (max 1min)
cargo run --release -- benchmark 27 0.005 -r 1000 -n 1021 --code-type phenomenological-planar-code --primal-dual-type serial --verifier none
"""

for d in d_vec:
    for T in T_vec:
        syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}-T{T}.syndromes")
        if os.path.exists(syndrome_file_path):
            print("[warning] use existing syndrome data (if you think it's stale, delete it and rerun)")
        else:
            command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=T)
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

study the effect of partition_num given a single thread

"""

benchmark_profile_path_vec = []
for d in d_vec:
    for T in T_vec:
        syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}-T{T}.syndromes")
        benchmark_profile_path = os.path.join(tmp_dir, f"d{d}-T{T}.profile")
        benchmark_profile_path_vec.append(benchmark_profile_path)
        if os.path.exists(benchmark_profile_path):
            print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
        else:
            command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=T)
            command += ["--code-type", "error-pattern-reader"]
            command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
            command += ["--primal-dual-type", "serial"]
            command += ["--verifier", "none"]
            command += ["--benchmark-profiler-output", benchmark_profile_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."


"""
Gather useful data
"""

for didx, d in enumerate(d_vec):
    data_file = os.path.join(script_dir, f"data_fusion_d{d}.txt")
    with open(data_file, "w", encoding="utf8") as f:
        f.write("<T> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect> <decoding_time_relative_dev>\n")
        for Tidx, T in enumerate(T_vec):
            benchmark_profile_path = benchmark_profile_path_vec[didx * len(T_vec) + Tidx]
            print(benchmark_profile_path)
            profile = Profile(benchmark_profile_path)
            print("d:", d, "T:", T)
            print("    average_decoding_time:", profile.average_decoding_time())
            print("    average_decoding_time_per_round:", profile.average_decoding_time() / (T + 1))
            print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
            print("    average_defect_per_measurement:", profile.sum_defect_num() / (T + 1) / len(profile.entries))
            print("    decoding_time_relative_dev:", profile.decoding_time_relative_dev())
            f.write("%d %.5e %.5e %.5e %.3e\n" % (
                T,
                profile.average_decoding_time(),
                profile.average_decoding_time() / (T + 1),
                profile.average_decoding_time_per_defect(),
                profile.decoding_time_relative_dev(),
            ))
