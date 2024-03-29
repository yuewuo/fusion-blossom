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

d_vec = [3, 5, 7]  # for debugging script
# d_vec = [3, 5, 7, 9, 11, 13, 17, 19, 23, 27, 33, 39, 47, 57, 67, 81, 97]
p = 0.001
total_rounds = 1000

for d in d_vec:
    syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}.syndromes")
    if os.path.exists(syndrome_file_path):
        print("[warning] use existing syndrome data (if you think it's stale, delete it and rerun)")
    else:
        command = fusion_blossom_qecp_generate_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=min(d, 4))
        command += ["--code-type", "rotated-planar-code"]
        command += ["--noise-model", "stim-noise-model"]
        command += ["--decoder", "fusion", "--decoder-config", '{"only_stab_z":true,"use_combined_probability":false,"skip_decoding":true}']
        command += ["--debug-print", "fusion-blossom-syndrome-file", "--fusion-blossom-syndrome-export-filename", syndrome_file_path]
        command += ["--use-compact-simulator", "--use-compact-simulator-compressed"]
        command += ["--simulator-compact-extender-noisy-measurements", f"{d}"]
        command += ["--parallel", "0"]  # use all cores
        command += ["--use-brief-edge"]  # to save memory; it takes 23GB to run d=97 and 15min to initialize it...
        print(command)
        stdout, returncode = run_command_get_stdout(command)
        print("\n" + stdout)
        assert returncode == 0, "command fails..."

"""
Run simulations

study the effect of partition_num given a single thread

"""

data_file = os.path.join(script_dir, "data_fusion.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write("<d> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect> <decoding_time_relative_dev>\n")

    for idx, d in enumerate(d_vec):

        syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}.syndromes")
        benchmark_profile_path = os.path.join(tmp_dir, f"d{d}.profile")
        if os.path.exists(benchmark_profile_path):
            print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
        else:
            command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=d)
            command += ["--code-type", "error-pattern-reader"]
            command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
            command += ["--primal-dual-type", "serial"]
            command += ["--verifier", "none"]
            command += ["--benchmark-profiler-output", benchmark_profile_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

        print(benchmark_profile_path)
        profile = Profile(benchmark_profile_path)
        print("d:", d)
        print("    average_decoding_time:", profile.average_decoding_time())
        print("    average_decoding_time_per_round:", profile.average_decoding_time() / (d + 1))
        print("    average_decoding_time_per_defect:", profile.average_decoding_time_per_defect())
        print("    average_defect_per_measurement:", profile.sum_defect_num() / (d + 1) / len(profile.entries))
        print("    decoding_time_relative_dev:", profile.decoding_time_relative_dev())
        f.write("%d %.5e %.5e %.5e %.3e\n" % (
            d,
            profile.average_decoding_time(),
            profile.average_decoding_time() / (d + 1),
            profile.average_decoding_time_per_defect(),
            profile.decoding_time_relative_dev(),
        ))
