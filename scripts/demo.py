"""
A demo of how to use the library for decoding
"""


import fusion_blossom as fb
from fusion_blossom import *

print(dir(fb))

solver_initializer = fb.SolverInitializer(1, [(1,1,1)], [1])
print(solver_initializer)

print(fb.weight_of_p(0.2))

code = fb.CodeCapacityPlanarCode(5, 0.1, 500)
print(code.vertex_num())
print(code.get_positions())
with fb.PyMut(code, "vertices") as vertices:
    with fb.PyMut(vertices[0], "position") as position:
        position.i = -1
print(code.vertices[0].position.i)

print(code.snapshot(True))

print(dir(code))
