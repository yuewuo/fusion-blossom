import fusion_blossom as fb

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
solver = fb.SolverSerial(initializer)

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
