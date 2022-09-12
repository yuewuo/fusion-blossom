import os, sys
import subprocess, sys
root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__)), shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
rust_dir = root_dir
sys.path.insert(0, os.path.join(root_dir, "benchmark"))

from util import *

d_vec = []
for i in range(3):
    for j in [3, 5, 7]:
        d_vec.append(j * (3 ** i))
d_vec.append(81)
print("d_vec:", d_vec)

def main():

    with open("data.txt", "w", encoding="utf8") as f:
        f.write(f"<d> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_syndrome>\n")
        noisy_measurements = 1000
        for d in d_vec:
            filename = f"{d}-1000-0.005-phenomenological-planar.profile"
            profile = Profile(filename)
            print("d:", d)

            print("    average_decoding_time:", profile.average_decoding_time())
            print("    average_decoding_time_per_round:", profile.average_decoding_time() / (noisy_measurements + 1))
            print("    average_decoding_time_per_syndrome:", profile.average_decoding_time_per_syndrome())
            print("    average_syndrome_per_measurement:", profile.sum_syndrome_num() / (noisy_measurements + 1) / len(profile.entries))
            f.write(f"{d} {profile.average_decoding_time()} {profile.average_decoding_time() / (noisy_measurements + 1)} {profile.average_decoding_time_per_syndrome()}\n")

            

if __name__ == "__main__":
    main()
