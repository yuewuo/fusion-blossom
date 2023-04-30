import os, sys, subprocess
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
sys.path.insert(0, benchmark_dir)

from util import *


for (filename, starting_row, ending_row) in [("data_fusion.txt", 13, None), ("data_pymatching.txt", 8, None), ("data_blossomV.txt", 9, 12)]:
    print(filename)
    data = GnuplotData(filename)
    assert data.titles[0] == "d"
    assert data.titles[2] == "average_decoding_time_per_round"
    slope, intercept, r = data.fit(0, 2, x_func=lambda x:math.log((float(x)**2)), y_func=lambda y:math.log(float(y)), starting_row=starting_row, ending_row = ending_row)
    print(f"time ~= N ^ {slope}")

    slope, intercept, r = data.fit(0, 2, x_func=lambda x:math.log(float(x)), y_func=lambda y:math.log(float(y)), starting_row=starting_row, ending_row=ending_row)
    print(f"fit: {math.exp(intercept)} * (x ** {slope})")
