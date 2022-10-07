"""
A demo of how to use the library for decoding
"""


import fusion_blossom as fb

# create an example code
code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
initializer = code.get_initializer()  # the decoding graph structure (you can easily construct your own)
positions = code.get_positions()  # the positions of vertices in the 3D visualizer, optional

# randomly generate a syndrome according to the error model
syndrome = code.generate_random_errors(seed=1000)
with fb.PyMut(syndrome, "syndrome_vertices") as syndrome_vertices:
    syndrome_vertices.append(0)  # you can modify the syndrome vertices
print(syndrome)

# visualizer (optional for debugging)
visualizer = None
if True:  # change to False to disable visualizer for much faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    fb.print_visualize_link(filename=visualize_filename)
    visualizer = fb.Visualizer(filepath=fb.visualize_data_folder() + visualize_filename)
    visualizer.load_positions(positions)  # so that visualizer can display vertices in user-defined view

solver = fb.SolverSerial(initializer)
solver.solve_visualizer(syndrome, visualizer)  # enable visualizer for debugging
perfect_matching = solver.perfect_matching()
perfect_matching = solver.perfect_matching()
print(f"perfect_matching: {perfect_matching}")
print(f"    - peer_matchings: {perfect_matching.peer_matchings}")
print(f"    - virtual_matchings: {perfect_matching.virtual_matchings}")
solver.clear()  # clear is very fast (O(1) complexity), recommended for repetitive simulation
