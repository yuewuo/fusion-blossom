import fusion_blossom as fb

## Functions for vertex index transformation 
def build_old_to_new(reordered_vertices):
    old_to_new = [None] * len(reordered_vertices)
    for new_index, old_index in enumerate(reordered_vertices):
        assert old_to_new[old_index] is None, f"duplicate vertex found {old_index}"
        old_to_new[old_index] = new_index
    return old_to_new

def translated_defect_to_reordered(reordered_vertices, old_defect_vertices):
    old_to_new = build_old_to_new(reordered_vertices)
    return [old_to_new[old_index] for old_index in old_defect_vertices]

# obtain the reordered vertices of the 4 partitions the original vertices are splitted into
def split_into_4_partitions_vertices(split_horizontal, split_vertical, num_vertices_per_row, num_vertices_per_column, reordered_vertices):

    for i in range(0, split_horizontal):
        # left-top block
        for j in range(0, split_vertical):
            reordered_vertices.append(i * num_vertices_per_row + j)
        reordered_vertices.append(i * num_vertices_per_row + num_vertices_per_column)

    for i in range(0, split_horizontal):
        # interface between the left-top block and the right-top block
        reordered_vertices.append(i * num_vertices_per_row + split_vertical)

    for i in range(0, split_horizontal):
        # right-top block 
        for j in range(split_vertical + 1, num_vertices_per_column - 1):
            reordered_vertices.append(i * num_vertices_per_row + j)
        reordered_vertices.append(i * num_vertices_per_row + num_vertices_per_column - 1)

    for j in range(0, num_vertices_per_row):
        # the big interface between top and bottom
        reordered_vertices.append(split_horizontal * num_vertices_per_row + j)

    for i in range(split_horizontal + 1, num_vertices_per_column):
        # left-bottom block 
        for j in range(0, split_vertical):
            reordered_vertices.append(i * num_vertices_per_row + j)
        reordered_vertices.append(i * num_vertices_per_row + num_vertices_per_column)

    for i in range(split_horizontal + 1, num_vertices_per_column):
        # interface between the left-bottom block and the right-bottom block
        reordered_vertices.append(i * num_vertices_per_row + split_vertical)
            
    for i in range(split_horizontal + 1, num_vertices_per_column):
        # right-bottom block 
        for j in range(split_vertical + 1, num_vertices_per_column - 1):
            reordered_vertices.append(i * num_vertices_per_row + j)
        reordered_vertices.append(i * num_vertices_per_row + num_vertices_per_column - 1)

## Code Initialization

d = 11
p = 0.005
total_rounds = 100 
noisy_measurements = 100000
code = fb.CodeCapacityPlanarCode(d=d, p=p, max_half_weight=500)

## Define the vertices in different partitions, we split the vertices into 4, 
## with no syndrome vertex on the interface

defect_vertices = [39, 52, 63, 90, 100] # indices are before the reorder
split_horizontal = 6
split_vertical = 5
num_vertices_per_row = 12
num_vertices_per_column = 11
reordered_vertices = []

split_into_4_partitions_vertices(split_horizontal, split_vertical, num_vertices_per_row, num_vertices_per_column, reordered_vertices)

# reorder the vertices
code.reorder_vertices(reordered_vertices)
new_defect_vertices = translated_defect_to_reordered(reordered_vertices, defect_vertices)
new_defect_vertices.sort() # the SyndromePattern only accepts defect vertices in ascending order

fb.helper.peek_code(code)  # comment out after constructing the syndrome

## Get initializer

initializer = code.get_initializer()

## Define Partition Configuration

partition_config = fb.PartitionConfig(initializer.vertex_num)
# the ranges below are the range of vertices (after reordering) of the 4 partitions 
# we find the range below by looking at the partition visualization
# for example, the range 0-36 is found by taking the min and the max indexes of the verticies in the top left parition
partition_config.partitions = [fb.NodeRange(0,36), # unit 0
                                fb.NodeRange(42,72), # unit 1
                                fb.NodeRange(84,108), # unit 2
                                fb.NodeRange(112,132) # unit 3
                                ]
partition_config.fusions = [(0, 1), (2, 3), (4, 5)] # refer to tree figure in paper
partition_info = partition_config.info()

## Define primal_dual_config

primal_dual_config = {
    "dual": {
        "thread_pool_size": 1,
        "enable_parallel_execution": False
    },
    "primal": {
        "thread_pool_size": 1,
        "debug_sequential": True,
        "prioritize_base_partition": True, # by default enable because this is faster by placing time-consuming tasks in the front
        # "interleaving_base_fusion": usize::MAX, # starts interleaving base and fusion after this unit_index
        "pin_threads_to_cores": False # pin threads to cores to achieve the most stable result
        # "streaming_decode_mock_measure_interval": 
    }
}

## Define syndrome graph

syndrome = fb.SyndromePattern(
    defect_vertices = new_defect_vertices,
)


## Initialize Visualizer [Optional]

visualizer = None
if True:  # change to False to disable visualizer for faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    positions = code.get_positions()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

## Initialize Solver

solver = fb.SolverParallel(initializer, partition_info, primal_dual_config)

## Run Solver

solver.solve(syndrome, visualizer)

## Print Minimum-Weight Parity Subgraph (MWPS)

subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")  # Vec<EdgeIndex>

## Print Minimum-Weight Perfect Matching (MWPM)

perfect_matching = solver.perfect_matching()
defect_vertices = syndrome.defect_vertices
print("defect_vertices: ", defect_vertices)
print(f"Minimum Weight Perfect Matching (MWPM):")
print(f"    - peer_matchings: {perfect_matching.peer_matchings}")
# peer_matching_vertices = [(defect_vertices[a], defect_vertices[b])
#                             for a, b in perfect_matching.peer_matchings]
# print(f"          = vertices: {peer_matching_vertices}")
virtual_matching_vertices = [(defect_vertices[a], b)
                            for a, b in perfect_matching.virtual_matchings]
print(f"    - virtual_matchings: {perfect_matching.virtual_matchings}")
print(f"             = vertices: {virtual_matching_vertices}")

## Clear Solver

solver.clear()

## Visualization [Optional]

if __name__ == "__main__" and visualizer is not None:
    fb.print_visualize_link(filename=visualize_filename)
    fb.helper.open_visualizer(visualize_filename, open_browser=True)
