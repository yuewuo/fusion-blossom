import fusion_blossom as fb

## Code Initialization

code = fb.CodeCapacityPlanarCode(d=7, p=0.1, max_half_weight=500)

## Initialize Visualizer [Optional]

visualizer = None
positions_x = code.get_positions()
positions_x = fb.center_positions(positions_x)
positions_z = [fb.VisualizePosition(pos.j, pos.i, 1) for pos in positions_x]
positions = positions_x + positions_z
if True:  # change to False to disable visualizer for faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

## Initialize Solver

initializer_x = code.get_initializer()
bias = initializer_x.vertex_num
weighted_edges_x = initializer_x.weighted_edges
virtual_vertices_x = initializer_x.virtual_vertices
weighted_edges_z = [(i + bias, j + bias, w) for (i, j, w) in weighted_edges_x]
virtual_vertices_z = [i + bias for i in virtual_vertices_x]
vertex_num = initializer_x.vertex_num * 2
weighted_edges = weighted_edges_x + weighted_edges_z
virtual_vertices = virtual_vertices_x + virtual_vertices_z
initializer = fb.SolverInitializer(vertex_num, weighted_edges, virtual_vertices)
solver = fb.SolverSerial(initializer)

## Simulate Random Errors

syndrome_x = code.generate_random_errors(seed=1000)
syndrome_z = code.generate_random_errors(seed=2000)
defect_vertices = syndrome_x.defect_vertices
defect_vertices += [i + bias for i in syndrome_z.defect_vertices]
syndrome = fb.SyndromePattern(defect_vertices)

## Visualize Result

solver.solve(syndrome)

subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")  # Vec<EdgeIndex>

if visualizer is not None:
    fb.print_visualize_link(filename=visualize_filename)
    fb.helper.open_visualizer(visualize_filename, open_browser=True)

## Generate Most Likely Error Pattern

error_pattern = dict()
for edge_index in subgraph:
    v1, v2, _ = weighted_edges[edge_index]
    qubit_i = round(positions[v1].i + positions[v2].i) + 6
    qubit_j = round(positions[v1].j + positions[v2].j) + 6
    error_type = "Z" if edge_index < len(weighted_edges_x) else "X"
    if (qubit_i, qubit_j) in error_pattern:
        error_pattern[(qubit_i, qubit_j)] = "Y"
    else:
        error_pattern[(qubit_i, qubit_j)] = error_type
print("Most Likely Error Pattern:", error_pattern)
