"""
batch decoding receives all syndrome data and start decoding
"""

import enum
import os, sys
import subprocess, sys

git_root_dir = (
    subprocess.run(
        "git rev-parse --show-toplevel",
        cwd=os.path.dirname(os.path.abspath(__file__)),
        shell=True,
        check=True,
        capture_output=True,
    )
    .stdout.decode(sys.stdout.encoding)
    .strip(" \r\n")
)
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

d_vec = [3, 5, 7, 9, 11, 13, 15, 17, 19, 21]
p = 0.001
total_rounds = 10000


def main():

    for d in d_vec:
        noisy_measurements = d
        syndrome_file_path = os.path.join(tmp_dir, f"generated_d{d}.syndromes")
        if os.path.exists(syndrome_file_path):
            print(
                "[warning] use existing syndrome data (if you think it's stale, delete it and rerun)"
            )
        else:
            command = fusion_blossom_qecp_generate_command(
                d=d,
                p=p,
                total_rounds=total_rounds,
                noisy_measurements=d - 1,
            )
            command += ["--code-type", "rotated-planar-code"]
            command += ["--noise-model", "stim-noise-model"]
            command += [
                "--decoder",
                "fusion",
                "--decoder-config",
                '{"only_stab_z":true,"use_combined_probability":true,"skip_decoding":true,"max_half_weight":7}',
            ]
            command += [
                "--debug-print",
                "fusion-blossom-syndrome-file",
                "--fusion-blossom-syndrome-export-filename",
                syndrome_file_path,
            ]

            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

    data_file = os.path.join(script_dir, f"data_parity.txt")
    with open(data_file, "w", encoding="utf8") as f:
        f.write(
            "<d> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect> <decoding_time_relative_dev>\n"
        )

        for d in d_vec:
            noisy_measurements = d - 1
            benchmark_profile_path = os.path.join(tmp_dir, f"generated_d{d}.profile")
            syndrome_file_path = os.path.join(tmp_dir, f"generated_d{d}.syndromes")
            command = fusion_blossom_benchmark_command(
                d=d,
                p=p,
                total_rounds=total_rounds,
                noisy_measurements=noisy_measurements,
            )
            command += ["--code-type", "error-pattern-reader"]
            command += ["--code-config", f'{{"filename":"{syndrome_file_path}"}}']
            command += ["--primal-dual-type", "serial"]
            command += ["--verifier", "none"]
            command += ["--benchmark-profiler-output", benchmark_profile_path]
            print(command)
            stdout, returncode = run_command_get_stdout(command)
            print("\n" + stdout)
            assert returncode == 0, "command fails..."

            profile = Profile(benchmark_profile_path)
            print("d:", d, ", p", p)
            print("    average_decoding_time:", profile.average_decoding_time())
            print(
                "    average_decoding_time_per_round:",
                profile.average_decoding_time() / (noisy_measurements + 1),
            )
            print(
                "    average_decoding_time_per_defect:",
                profile.average_decoding_time_per_defect(),
            )
            print(
                "    average_defect_per_measurement:",
                profile.sum_defect_num()
                / (noisy_measurements + 1)
                / len(profile.entries),
            )
            print(
                "    decoding_time_relative_dev:", profile.decoding_time_relative_dev()
            )
            f.write(
                "%d %.5e %.5e %.5e %.3e\n"
                % (
                    d,
                    profile.average_decoding_time(),
                    profile.average_decoding_time() / (noisy_measurements + 1),
                    profile.average_decoding_time_per_defect(),
                    profile.decoding_time_relative_dev(),
                )
            )
            f.flush()


if __name__ == "__main__":
    main()
