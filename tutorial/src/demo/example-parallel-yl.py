import fusion_blossom as fb

import enum 
import os, sys
import subprocess, sys
import subprocess, sys
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
sys.path.insert(0, benchmark_dir)

import util
from util import *
# util.FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER = True  # better performance, still safe
# compile_code_if_necessary()

# fusion_path = os.path.join(rust_dir, "target", "release", "fusion_blossom")




# d = 11
# p = 0.005
# total_rounds = 100 
# noisy_measurements = 100000


# syndrome_file_path = os.path.join(tmp_dir, f"generated_p{p}_d{d}.syndromes")
# if os.path.exists(syndrome_file_path):
#     print("[warning] use existing syndrome data (if you think it's stale, delete it and rerun)")
# else:
#     command = fusion_blossom_benchmark_command(d=d, p=p, total_rounds=total_rounds, noisy_measurements=noisy_measurements)
#     command += ["--code-type", "phenomenological-rotated-code"]
#     command += ["--primal-dual-type", "error-pattern-logger"]
#     command += ["--verifier", "none"]
#     command += ["--primal-dual-config", f'{{"filename":"{syndrome_file_path}"}}']
#     print(command)
#     stdout, returncode = run_command_get_stdout(command)
#     print("\n" + stdout)
#     assert returncode == 0, "command fails..."



# get a list of all functions in fusion_blossom
print(dir(fb))

# # the library scales the edge weight such that the maximum weight is 500 * 2 
# # d is code distance, p is probability of error

##############################################################################
##############################################################################




## Code Initialization
code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
print("original code positions: ", code.get_positions())

syndrome = fb.SyndromePattern(
    defect_vertices = [39, 52, 63, 90, 100],
)
print(syndrome)

# # ### TEST 
# # split into 4, with no syndrome vertex on the interface
# defect_vertices = [39, 52, 63, 90, 100] # indices are before the reorder
# split_horizontal = 6
# split_vertical = 5
# reordered_vertices = []
# for i in range(0, split_horizontal):
#     # left-top block
#     for j in range(0, split_vertical):
#         reordered_vertices.append(i * 12 + j)
#     reordered_vertices.append(i * 12 + 11)
# # print("####################################################\n")
# # print("reordered_vertices: ", reordered_vertices)
# # print("####################################################\n")

# for i in range(0, split_horizontal):
#     # interface between the left-top block and the right-top block
#     reordered_vertices.append(i * 12 + split_vertical)
# # print("####################################################\n")
# # print("reordered_vertices with interface: ", reordered_vertices)
# # print("####################################################\n")

# for i in range(0, split_horizontal):
#     # right-top block 
#     for j in range(split_vertical + 1, 10):
#         reordered_vertices.append(i * 12 + j)
#     reordered_vertices.append(i * 12 + 10)

# for j in range(0, 12):
#     # the big interface between top and bottom
#     reordered_vertices.append(split_horizontal * 12 + j)

# for i in range(split_horizontal + 1, 11):
#     # left-bottom block 
#     for j in range(0, split_vertical):
#         reordered_vertices.append(i * 12 + j)
#     reordered_vertices.append(i * 12 + 11)

# for i in range(split_horizontal + 1, 11):
#     # interface between the left-bottom block and the right-bottom block
#     reordered_vertices.append(i * 12 + split_vertical)
         
# for i in range(split_horizontal + 1, 11):
#     # right-bottom block 
#     for j in range(split_vertical + 1, 10):
#         reordered_vertices.append(i * 12 + j)
#     reordered_vertices.append(i * 12 + 10)
# print("reordered_vertices with interface: ", reordered_vertices)
# print("####################################################\n")

# code.reorder_vertices(reordered_vertices)
# print("####################################################\n")
# print("NEW code positions: ", code.get_positions())
# print("OLD defect_vertices: ", defect_vertices)
# # defect_vertices = (reordered_vertices, defect_vertices)

# #### translated index transformation function ####
# def build_old_to_new(reordered_vertices):
#     old_to_new = [None] * len(reordered_vertices)
#     for new_index, old_index in enumerate(reordered_vertices):
#         assert old_to_new[old_index] is None, f"duplicate vertex found {old_index}"
#         old_to_new[old_index] = new_index
#     return old_to_new

# def translated_defect_to_reordered(reordered_vertices, old_defect_vertices):
#     old_to_new = build_old_to_new(reordered_vertices)
#     return [old_to_new[old_index] for old_index in old_defect_vertices]


# new_defect_vertices = translated_defect_to_reordered(reordered_vertices, defect_vertices)
# print("new defect vertices: ", new_defect_vertices)

# get initializer
initializer = code.get_initializer()
print("code initializer: ", initializer)

partition_config = fb.PartitionConfig(initializer.vertex_num)

# # Assign the vertices in 2 different partitions according to their indicies
## test for split into 2
partition_config.partitions = [fb.NodeRange(0, 72),
                                fb.NodeRange(84,132)]
partition_config.fusions = [(0,1)]


# partition_config.partitions = [fb.NodeRange(0,36), # unit 0
#                                 fb.NodeRange(42,72), # unit 1
#                                 fb.NodeRange(84,108), # unit 2
#                                 fb.NodeRange(112,132) # unit 3
#                                 ]

# partition_config.fusions = [(0, 1), (2, 3), (4, 5)] # unit 2, by fusing unit 0 and 1, refer to tree figure in paper
print("partition_config: ", partition_config)
partition_info = partition_config.info()
print("partition_info: ", partition_info)
# code.set_defect_vertices(new_defect_vertices)

#################
primal_dual_config = {
    "dual": {
        "thread_pool_size": 10,
        "enable_parallel_execution": False
    },
    "primal": {
        "thread_pool_size": 10,
        "debug_sequential": False,
        "prioritize_base_partition": True, # by default enable because this is faster by placing time-consuming tasks in the front
        # "interleaving_base_fusion": usize::MAX, # starts interleaving base and fusion after this unit_index
        "pin_threads_to_cores": False # pin threads to cores to achieve the most stable result
        # "streaming_decode_mock_measure_interval": 
    }
}


#############################################
## Simulate Random Errors

# syndrome = code.generate_random_errors(seed=1000)

# print(syndrome)

## Initialize Visualizer [Optional]

visualizer = None
if True:  # change to False to disable visualizer for faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    positions = code.get_positions()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

## Initialize Solver

# initializer = code.get_initializer()
# positions = code.get_positions()


solver = fb.SolverParallel(initializer, partition_info, primal_dual_config)

## Run Solver

solver.solve(syndrome)

## Print Minimum-Weight Parity Subgraph (MWPS)

subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")  # Vec<EdgeIndex>

# ## Print Minimum-Weight Perfect Matching (MWPM)

perfect_matching = solver.perfect_matching()
defect_vertices = syndrome.defect_vertices
print("defect_vertices: ", defect_vertices)
print(f"Minimum Weight Perfect Matching (MWPM):")
print(f"    - peer_matchings: {perfect_matching.peer_matchings}")
# peer_matching_vertices = [(defect_vertices[a], defect_vertices[b])
#                             for a, b in perfect_matching.peer_matchings]
# print(f"          = vertices: {peer_matching_vertices}")
# virtual_matching_vertices = [(defect_vertices[a], b)
#                             for a, b in perfect_matching.virtual_matchings]
# print(f"    - virtual_matchings: {perfect_matching.virtual_matchings}")
# print(f"             = vertices: {virtual_matching_vertices}")

## Clear Solver

solver.clear()

## Visualization [Optional]

if __name__ == "__main__" and visualizer is not None:
    fb.print_visualize_link(filename=visualize_filename)
    fb.helper.open_visualizer(visualize_filename, open_browser=True)
