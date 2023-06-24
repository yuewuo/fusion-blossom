"""
The graph is like below: 

   1     2     3       edge
o --- * --- * --- o
0     1     2     3   vertex

The weights determine the matching result
"""

import fusion_blossom as fb

def prepare_solver() -> fb.SolverSerial:
    vertex_num = 4
    weighted_edges = [(0, 1, 100), (1, 2, 100), (2, 3, 100)]
    virtual_vertices = [0, 3]
    initializer = fb.SolverInitializer(vertex_num, weighted_edges, virtual_vertices)
    solver = fb.SolverSerial(initializer)
    return solver

def test_default_weight():
    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [1]

def test_erasure_weight():
    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2], erasures = [0, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [0, 2]

def test_dynamic_weight():
    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2], dynamic_weights = [(0, 48), (2, 48)]))
    subgraph = solver.subgraph()
    assert subgraph == [0, 2]
    solver.clear()

    solver.solve(fb.SyndromePattern([1, 2], dynamic_weights = [(0, 52), (2, 52)]))
    subgraph = solver.subgraph()
    assert subgraph == [1]
    solver.clear()

    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [1]
    solver.clear()

    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2], erasures = [0, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [0, 2]
