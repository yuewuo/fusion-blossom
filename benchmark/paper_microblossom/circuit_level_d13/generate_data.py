import os, sys, git

git_root_dir = git.Repo(".", search_parent_directories=True).working_tree_dir
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
sys.path.insert(0, benchmark_dir)

from util import *
import util

util.FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER = True  # better performance, still safe
compile_code_if_necessary()

d = 13
noisy_measurements = d - 1
p = 0.001
total_rounds = 1000000

syndrome_file_path = os.path.join(
    tmp_dir, f"generated.d{d}.T{noisy_measurements}.syndromes"
)
if os.path.exists(syndrome_file_path):
    print(
        "[warning] use existing syndrome data (if you think it's stale, delete it and rerun)"
    )
else:
    command = fusion_blossom_qecp_generate_command(
        d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements
    )
    command += ["--code-type", "rotated-planar-code"]
    command += ["--noise-model", "stim-noise-model"]
    command += [
        "--decoder",
        "fusion",
        "--decoder-config",
        '{"only_stab_z":true,"use_combined_probability":false,"skip_decoding":true}',
    ]
    command += [
        "--debug-print",
        "fusion-blossom-syndrome-file",
        "--fusion-blossom-syndrome-export-filename",
        syndrome_file_path,
    ]
    command += ["--ignore-logical-i", "--ignore-logical-j"]
    command += ["--parallel", "0"]  # use all cores
    command += ["--use-brief-edge"]
    print(command)
    stdout, returncode = run_command_get_stdout(command)
    print("\n" + stdout)
    assert returncode == 0, "command fails..."
