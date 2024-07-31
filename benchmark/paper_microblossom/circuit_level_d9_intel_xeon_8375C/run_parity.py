import os
import sys
import time
import subprocess
import pymatching
from scipy.sparse import lil_matrix
import numpy as np
from msgspec.json import decode
from msgspec import Struct

git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(
    __file__)), shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
sys.path.insert(0, benchmark_dir)

if True:
    from util import *
    import util
util.FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER = True  # better performance, still safe
compile_code_if_necessary()

d = 9
noisy_measurements = d
p = 0.001
total_rounds = 100000

syndrome_file_path = os.path.join(
    tmp_dir, f"generated.T{noisy_measurements}.syndromes")
assert os.path.exists(syndrome_file_path)

# run simulation
benchmark_profile_path = os.path.join(
    tmp_dir, f"T{noisy_measurements}.parity.profile")
if os.path.exists(benchmark_profile_path):
    print("[warning] found existing profile (if you think it's stale, delete it and rerun)")
else:
    command = fusion_blossom_benchmark_command(
        d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
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
print("noisy_measurements:", noisy_measurements)
print("    average_decoding_time:", profile.average_decoding_time())
print("    average_decoding_time_per_round:",
      profile.average_decoding_time() / (noisy_measurements + 1))
print("    average_decoding_time_per_defect:",
      profile.average_decoding_time_per_defect())
print("    average_defect_per_measurement:", profile.sum_defect_num() /
      (noisy_measurements + 1) / len(profile.entries))
print("    decoding_time_relative_dev:", profile.decoding_time_relative_dev())
