import os
import sys
import subprocess
import fusion_blossom as fb


git_root_dir = subprocess.run("git rev-parse --show-toplevel",
                              cwd=os.path.dirname(
                                  os.path.abspath(__file__)),
                              shell=True,
                              check=True,
                              capture_output=True
                              ).stdout.decode(sys.stdout.encoding).strip(" \r\n")
data_folder = os.path.join(git_root_dir, "visualize", "data")


def solver_tester(max_tree_size=None) -> fb.SolverSerial:
    # construct solver
    code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
    positions = code.get_positions()
    initializer = code.get_initializer()
    solver = fb.SolverSerial(initializer, max_tree_size=max_tree_size)
    # construct visualizer
    filename = f"test_max_tree_size_{max_tree_size}.json"
    visualizer = fb.Visualizer(
        filepath=os.path.join(data_folder, filename), positions=positions)
    # decode
    syndrome = fb.SyndromePattern([39, 52, 63, 90, 100])
    solver.solve(syndrome, visualizer)
    solver.subgraph(visualizer)


def test_union_find_decoder():
    # http://localhost:8066/?filename=test_max_tree_size_0.json
    solver_tester(max_tree_size=0)


def test_mwpm_decoder():
    # http://localhost:8066/?filename=test_max_tree_size_None.json
    solver_tester()


def test_mixture_decoder():
    # http://localhost:8066/?filename=test_max_tree_size_10.json
    solver_tester(max_tree_size=10)
