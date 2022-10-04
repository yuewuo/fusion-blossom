import enum
import os, sys
import subprocess, sys
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "..", "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
sys.path.insert(0, benchmark_dir)

import util
from util import *
# util.FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER = True  # better performance, still safe
# compile_code_if_necessary()


benchmark_profile_path = os.path.join(tmp_dir, f"1000.profile")
profile = Profile(benchmark_profile_path)

data_file = os.path.join(script_dir, "base_partition_complexity.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write("<unit_index> <average_job_time>\n")
    for unit_index in range(len(profile.partition_config.partitions)):
        f.write("%d %.5e\n" % (
            unit_index,
            profile.average_job_time(unit_index),
        ))
