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
# tmp_dir = os.path.join(script_dir, "tmp")
tmp_dir = os.path.join(script_dir, "raw-data-2023-04-09-3")
sys.path.insert(0, benchmark_dir)

import util
from util import *
import numpy as np

# measure_interval_vec = [0.5e-6 * (1.3 ** i) for i in range(20)]
measure_interval_vec = [0.2e-6 * (1.1 ** i) for i in range(20)]
noisy_measurements = 100000


data_file = os.path.join(script_dir, "data.txt")
with open(data_file, "w", encoding="utf8") as f:
    f.write("<measure_interval> <median_latency> <average_latency> <stddev_latency> <sample_latency>\n")
    for idx, measure_interval in enumerate(measure_interval_vec):
        benchmark_profile_path = os.path.join(tmp_dir, f"measure_interval_{'%.3e' % measure_interval}.profile")
        profile = Profile(benchmark_profile_path)
        latency_vec = []
        syndrome_ready_time = measure_interval * (noisy_measurements + 1)
        for entry in profile.entries:
            latency = entry["events"]["decoded"] - syndrome_ready_time
            latency_vec.append(latency)
        median_latency = np.median(latency_vec)
        average_latency = sum(latency_vec) / len(latency_vec)
        stddev_latency = math.sqrt(sum([(time - average_latency) ** 2 for time in latency_vec]) / len(latency_vec))
        samples_str = ["%.3e" % time for time in latency_vec]
        print(f"measure_interval {measure_interval}: median {median_latency}, average {average_latency}, stddev {stddev_latency}")
        f.write("%.5e %.5e %.5e %.3e %s\n" % (
            measure_interval,
            median_latency,
            average_latency,
            stddev_latency,
            "[" + ",".join(samples_str) + "]",
        ))
        f.flush()
