"""
A demo of how to use the library for decoding
"""


import fusion_blossom as fb

code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
initializer = code.get_initializer()  # the decoding graph structure (you can easily construct your own)
positions = code.get_positions()  # the positions of vertices in the 3D visualizer, optional
syndrome = code.generate_random_errors(seed=1000)

visualize_filename = fb.static_visualize_data_filename()
visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

solver = fb.SolverSerial(initializer)
solver.solve(syndrome)  # enable visualizer for debugging
perfect_matching = solver.perfect_matching()
subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}\n\n")  # Vec<EdgeIndex>

# test visualizer API bindings
visualizer.snapshot("snapshot object", solver)
visualizer.snapshot_combined("snapshot combined object", [solver])
visualizer.snapshot_value("snapshot value", solver.snapshot())
visualizer.snapshot_combined_value("snapshot combined value", [solver.snapshot()])

solver.clear()  # clear is O(1) complexity, recommended for repetitive simulation

fb.print_visualize_link(filename=visualize_filename)
fb.helper.open_visualizer(visualize_filename, open_browser=True)
