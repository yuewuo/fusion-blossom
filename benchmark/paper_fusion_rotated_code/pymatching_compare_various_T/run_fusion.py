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
small_T_vec = [i for i in range(1, 10)] + [i * 10 for i in range(1, 11)]
noisy_measurements_vec = small_T_vec + [300, 1000, 3000, 10000, 30000, 100000]
#noisy_measurements_vec = small_T_vec + [300, 1000, 3000]  # small-scale debug

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

data_file = os.path.join(script_dir, "data_fusion.txt")
with open(data_file, "w", encoding="utf8") as data_f:
    data_f.write("<noisy_measurements> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect> <decoding_time_relative_dev>\n")

    for noisy_measurements in noisy_measurements_vec:
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
        benchmark_profile_path = os.path.join(tmp_dir, f"T{noisy_measurements}.profile")
        if os.path.exists(benchmark_profile_path):
            print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
        else:
            command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
            command += ["--code-type", "error-pattern-reader"]
            command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
            command += ["--primal-dual-type", "parallel"]
            command += ["--primal-dual-config", f'{{"primal":{{"thread_pool_size":{1},"pin_threads_to_cores":true}},"dual":{{"thread_pool_size":{1}}}}}']
            command += ["--partition-strategy", "phenomenological-rotated-code-time-partition"]
            # use `maximum_tree_leaf_size` to make sure fusion jobs are distributed to multiple cores while limiting the size of tree
            partition_num = 1
            if noisy_measurements > 100:
                partition_num = noisy_measurements // 100
            command += ["--partition-config", f'{{"partition_num":{partition_num},"enable_tree_fusion":true}}']
            command += ["--verifier", "none"]
            command += ["--benchmark-profiler-output", benchmark_profile_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

        profile = Profile(benchmark_profile_path)
        print("noisy_measurements:", noisy_measurements)
        print("    average_decoding_time:", profile.average_decoding_time())
        print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
        print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
        print("    average_defect_per_measurement:", profile.sum_defect_num() / (noisy_measurements + 1) / len(profile.entries))
        print("    decoding_time_relative_dev:", profile.decoding_time_relative_dev())
        data_f.write("%d %.5e %.5e %.5e %.3e\n" % (
            noisy_measurements,
            profile.average_decoding_time(),
            profile.average_decoding_time() / (noisy_measurements + 1),
            profile.average_decoding_time_per_defect(),
            profile.decoding_time_relative_dev(),
        ))
        data_f.flush()
