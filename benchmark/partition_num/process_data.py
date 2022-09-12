import os, sys
import subprocess, sys
root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__)), shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
rust_dir = root_dir
sys.path.insert(0, os.path.join(root_dir, "benchmark"))

from util import *

partition_num_vec = [1]
for i in range(8):
    for j in [2, 3]:
        partition_num_vec.append(j * (2 ** i))
partition_num_vec.append(512)
print("partition_num_vec:", partition_num_vec)

def main():

    with open("data.txt", "w", encoding="utf8") as f:
        f.write(f"<partition_num> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_syndrome>\n")
        noisy_measurements = 100000
        for partition_num in partition_num_vec:
            filename = f"15-100000-0.005-phenomenological-planar-tree-{partition_num}.profile"
            profile = Profile(filename)
            print("partition_num:", partition_num)
            
            print("    average_decoding_time:", profile.average_decoding_time())
            print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
            print("    average_decoding_time_per_syndrome:", profile.average_decoding_time_per_syndrome())
            print("    average_syndrome_per_measurement:", profile.sum_syndrome_num() / (noisy_measurements + 1) / len(profile.entries))
            f.write(f"{partition_num} {profile.average_decoding_time()} {profile.average_decoding_time() / (noisy_measurements + 1)} {profile.average_decoding_time_per_syndrome()}\n")

            

if __name__ == "__main__":
    main()
