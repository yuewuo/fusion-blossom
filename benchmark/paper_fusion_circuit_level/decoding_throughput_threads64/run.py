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

d_vec = [11, 13, 15, 17, 19, 21, 23, 25, 27, 29, 31]
p_vec = [0.001, 0.002, 0.003]
total_rounds = 100
noisy_measurements = 100000

# # small size debug
# d_vec = [3,5,7]
# noisy_measurements = 1000
# p_vec = [0.005, 0.01, 0.02]


for p in p_vec:

    data_file = os.path.join(script_dir, f"data_p{p}.txt")
    with open(data_file, "w", encoding="utf8") as data_f:
        data_f.write("<d> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect> <decoding_time_relative_dev>\n")

        for d in d_vec:
            syndrome_file_path = os.path.join(tmp_dir, f"generated_p{p}_d{d}.syndromes")
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

            benchmark_profile_path = os.path.join(tmp_dir, f"p{p}_d{d}.profile")
            command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
            command += ["--code-type", "error-pattern-reader"]
            command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
            command += ["--primal-dual-type", "parallel"]
            command += ["--primal-dual-config", f'{{"primal":{{"thread_pool_size":64,"pin_threads_to_cores":true}},"dual":{{"thread_pool_size":64}}}}']
            command += ["--partition-strategy", "phenomenological-rotated-code-time-partition"]
            # use `maximum_tree_leaf_size` to make sure fusion jobs are distributed to multiple cores while limiting the size of tree
            partition_num = noisy_measurements // 100
            command += ["--partition-config", f'{{"partition_num":{partition_num},"enable_tree_fusion":true}}']
            command += ["--verifier", "none"]
            command += ["--benchmark-profiler-output", benchmark_profile_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

            profile = Profile(benchmark_profile_path)
            print("d:", d, ", p", p)
            print("    average_decoding_time:", profile.average_decoding_time())
            print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
            print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
            print("    average_defect_per_measurement:", profile.sum_defect_num() / (noisy_measurements + 1) / len(profile.entries))
            print("    decoding_time_relative_dev:", profile.decoding_time_relative_dev())
            data_f.write("%d %.5e %.5e %.5e %.3e\n" % (
                d,
                profile.average_decoding_time(),
                profile.average_decoding_time() / (noisy_measurements + 1),
                profile.average_decoding_time_per_defect(),
                profile.decoding_time_relative_dev(),
            ))
            data_f.flush()
