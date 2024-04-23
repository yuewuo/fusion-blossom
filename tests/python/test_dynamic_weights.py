"""
The graph is like below: 

   1     2     3       edge
o --- * --- * --- o
0     1     2     3   vertex

The weights determine the matching result
"""

import os
import sys
import subprocess
from typing import Tuple
import fusion_blossom as fb


def prepare_solver() -> fb.SolverSerial:
    vertex_num = 4
    weighted_edges = [(0, 1, 100), (1, 2, 100), (2, 3, 100)]
    virtual_vertices = [0, 3]
    initializer = fb.SolverInitializer(
        vertex_num, weighted_edges, virtual_vertices)
    solver = fb.SolverSerial(initializer)
    return solver


def test_default_weight():
    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [1]


def test_erasure_weight():
    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2], erasures=[0, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [0, 2]


def test_dynamic_weight():
    solver = prepare_solver()
    solver.solve(fb.SyndromePattern(
        [1, 2], dynamic_weights=[(0, 48), (2, 48)]))
    subgraph = solver.subgraph()
    assert subgraph == [0, 2]
    solver.clear()

    solver.solve(fb.SyndromePattern(
        [1, 2], dynamic_weights=[(0, 52), (2, 52)]))
    subgraph = solver.subgraph()
    assert subgraph == [1]
    solver.clear()

    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [1]
    solver.clear()

    solver = prepare_solver()
    solver.solve(fb.SyndromePattern([1, 2], erasures=[0, 2]))
    subgraph = solver.subgraph()
    assert subgraph == [0, 2]


def prepare_repetition_code_solver(measure_weight: int) -> Tuple[fb.SolverSerial, fb.Visualizer]:
    vertex_num = 12
    weighted_edges = [(j + 4*i, 1 + j + 4*i, 100)
                      for i in range(3) for j in range(3)]
    weighted_edges += [(1+j+4*i, 1+j+4*(i+1), measure_weight)
                       for i in range(2) for j in range(2)]
    virtual_vertices = [0, 3, 4, 7, 8, 11]
    initializer = fb.SolverInitializer(
        vertex_num, weighted_edges, virtual_vertices)
    # also initialize visualizer
    scale = 1
    positions = [fb.VisualizePosition((i // 4) * scale, (i % 4) * scale, 0)
                 for i in range(vertex_num)]
    git_root_dir = subprocess.run("git rev-parse --show-toplevel",
                                  cwd=os.path.dirname(
                                      os.path.abspath(__file__)),
                                  shell=True,
                                  check=True,
                                  capture_output=True
                                  ).stdout.decode(sys.stdout.encoding).strip(" \r\n")
    data_folder = os.path.join(git_root_dir, "visualize", "data")
    print(data_folder)
    filename = f"repetition_code_measure_{measure_weight}.json"
    visualizer = fb.Visualizer(
        filepath=os.path.join(data_folder, filename), positions=positions)
    solver = fb.SolverSerial(initializer)
    return solver, visualizer


def test_repetition_code():
    for measure_weight in [80, 100, 120]:
        solver, visualizer = prepare_repetition_code_solver(measure_weight)
        solver.solve_visualizer(fb.SyndromePattern([5, 2]), visualizer)
        solver.subgraph(visualizer)

    solver, visualizer = prepare_repetition_code_solver(100)
    solver.solve_visualizer(fb.SyndromePattern([]), visualizer)
    solver.subgraph(visualizer)
