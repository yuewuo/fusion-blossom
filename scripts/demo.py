"""
A demo of how to use the library for decoding
"""


import fusion_blossom as fb

# print(dir(fb))
# solver_initializer = fb.SolverInitializer(1, [(1,1,1)], [1])
# print(solver_initializer)
# print(fb.weight_of_p(0.2))

code = fb.CodeCapacityPlanarCode(5, 0.1, 500)
with fb.PyMut(code, "vertices") as vertices:
    with fb.PyMut(vertices[0], "position") as position:
        position.i = -0.5  # just to verify that I can modify the position
initializer = code.get_initializer()
positions = code.get_positions()
# print(code.snapshot())

seed = 0
syndrome = code.generate_random_errors(seed)
with fb.PyMut(syndrome, "syndrome_vertices") as syndrome_vertices:
    syndrome_vertices.append(21)
print(syndrome)

vertex_range = fb.VertexRange(0, 2)
print(vertex_range)

visualize_filename = fb.static_visualize_data_filename()
fb.print_visualize_link(visualize_filename)
visualizer = fb.Visualizer(fb.visualize_data_folder() + visualize_filename)
visualizer.load_positions(positions)

solver = fb.SolverSerial(initializer)
solver.clear()
solver.solve_visualizer(syndrome, visualizer)
perfect_matching = solver.perfect_matching()
print(perfect_matching)
print(perfect_matching.peer_matchings)
print(perfect_matching.virtual_matchings)
