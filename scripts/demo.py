"""
A demo of how to use the library for decoding
"""


import fusion_blossom as fb

# create an example code
code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
initializer = code.get_initializer()  # the decoding graph structure (you can easily construct your own)
positions = code.get_positions()  # the positions of vertices in the 3D visualizer, optional

# randomly generate a syndrome according to the noise model
syndrome = code.generate_random_errors(seed=1000)
with fb.PyMut(syndrome, "defect_vertices") as defect_vertices:
    defect_vertices.append(0)  # you can modify the defect vertices
print(syndrome)

# visualizer (optional for debugging)
visualizer = None
if True:  # change to False to disable visualizer for much faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

solver = fb.SolverSerial(initializer)
solver.solve(syndrome, visualizer)  # enable visualizer for debugging
perfect_matching = solver.perfect_matching()
print(f"\n\nMinimum Weight Perfect Matching (MWPM):")
print(f"    - peer_matchings: {perfect_matching.peer_matchings}")  # Vec<(DefectIndex, DefectIndex)>
print(f"          = vertices: {[(defect_vertices[a], defect_vertices[b]) for a, b in perfect_matching.peer_matchings]}")
print(f"    - virtual_matchings: {perfect_matching.virtual_matchings}")  # Vec<(DefectIndex, VertexIndex)>
print(f"             = vertices: {[(defect_vertices[a], b) for a, b in perfect_matching.virtual_matchings]}")
subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}\n\n")  # Vec<EdgeIndex>
solver.clear()  # clear is O(1) complexity, recommended for repetitive simulation

# view in browser
if visualizer is not None:
    fb.print_visualize_link(filename=visualize_filename)
    fb.helper.open_visualizer(visualize_filename, open_browser=True)
