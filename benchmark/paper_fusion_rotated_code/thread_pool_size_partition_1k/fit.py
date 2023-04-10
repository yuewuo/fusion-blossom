import os, sys, subprocess
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
sys.path.insert(0, benchmark_dir)

from util import *

filename = "data.txt"
data = GnuplotData(filename)
assert data.titles[0] == "thread_pool_size"
assert data.titles[2] == "average_decoding_time_per_round"

slope, intercept, r = data.fit(0, 2, x_func=lambda x:1/float(x), y_func=lambda y:float(y), starting_row=4, ending_row=12)
print(f"fit: {intercept} + {slope}/x")
