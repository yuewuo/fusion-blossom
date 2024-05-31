import fusion_blossom as fb
# import inspect

# get a list of all functions in fusion_blossom
print(dir(fb))

# # the library scales the edge weight such that the maximum weight is 500 * 2 
# # d is code distance, p is probability of error

##############################################################################
##############################################################################

## Code Initialization

code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)

## Simulate Random Errors

syndrome = code.generate_random_errors(seed=1000)
print(syndrome)

## Initialize Visualizer [Optional]

visualizer = None
if True:  # change to False to disable visualizer for faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    positions = code.get_positions()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

## Initialize Solver

initializer = code.get_initializer()
partition_config = fb.PartitionConfig(initializer.vertex_num)
partition_info = partition_config.info()
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
solver = fb.SolverParallel(initializer, partition_info, primal_dual_config)

## Run Solver

solver.solve(syndrome)

## Print Minimum-Weight Parity Subgraph (MWPS)

subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")  # Vec<EdgeIndex>

## Print Minimum-Weight Perfect Matching (MWPM)

perfect_matching = solver.perfect_matching()
defect_vertices = syndrome.defect_vertices
print(f"Minimum Weight Perfect Matching (MWPM):")
print(f"    - peer_matchings: {perfect_matching.peer_matchings}")
peer_matching_vertices = [(defect_vertices[a], defect_vertices[b])
                            for a, b in perfect_matching.peer_matchings]
print(f"          = vertices: {peer_matching_vertices}")
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
