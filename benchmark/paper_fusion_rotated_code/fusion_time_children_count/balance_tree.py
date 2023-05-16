import os, sys, subprocess, random
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
sys.path.insert(0, benchmark_dir)

from util import *

filename = os.path.join(script_dir, "balance_tree.profile")
profile = Profile(filename)

config = profile.partition_config
num_partition = len(config.partitions)

children_count_vec = []
for i in range(num_partition):
    children_count_vec.append(1)  # base partition has 1 child (itself)

for (i, j) in config.fusions:
    children_count_vec.append(children_count_vec[i] + children_count_vec[j])

max_children_count = max(children_count_vec)
time_vec = [[] for i in range(max_children_count + 1)]

for entry in profile.entries:
    event_time_vec = entry["solver_profile"]["primal"]["event_time_vec"]
    assert len(event_time_vec) == 2 * num_partition - 1
    for i, event_time in enumerate(event_time_vec):
        duration = event_time["end"] - event_time["start"]
        time_vec[children_count_vec[i]].append(duration)

with open("balance_tree.txt", "w", encoding="utf-8") as f:
    MAX_SAMPLE_COUNT = 100
    f.write(f"<children_count> <average_time> <stddev_time> <list of samples (maximum of {MAX_SAMPLE_COUNT})>\n")
    for children_count in range(1, max_children_count+1):
        if len(time_vec[children_count]) > 0:
            average_time = sum(time_vec[children_count]) / len(time_vec[children_count])
            stddev_time = math.sqrt(sum([(time - average_time) ** 2 for time in time_vec[children_count]]) / len(time_vec[children_count]))
            samples = time_vec[children_count] if len(time_vec[children_count]) < MAX_SAMPLE_COUNT else random.sample(time_vec[children_count], MAX_SAMPLE_COUNT)
            samples_str = ["%.3e" % time for time in samples]
            print(f"children count {children_count}: average {average_time}, stddev {stddev_time}")
            f.write(f"%d %.5e %.3e %s\n" % (
                children_count,
                average_time,
                stddev_time,
                "[" + ",".join(samples_str) + "]"
            ))
