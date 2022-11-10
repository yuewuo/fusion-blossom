"""
Render an image locally
"""


import fusion_blossom as fb
import os

code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)
syndrome = code.generate_random_errors(seed=1000)

visualize_filename = fb.static_visualize_data_filename()
visualizer = fb.Visualizer(filepath=visualize_filename, positions=code.get_positions())

solver = fb.SolverSerial(code.get_initializer())
solver.solve(syndrome, visualizer)
subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")
solver.clear()

# snapshot names
snapshot_names = visualizer.snapshots
print(snapshot_names)

folder = os.path.dirname(os.path.abspath(__file__))
renderer_folder = os.path.join(folder, "local_renderer")
image_filename = "rendered"
width = 1024
height = 1024
snapshot_idx = len(snapshot_names) - 1  # render the last image
fb.helper.local_render_visualizer(visualize_filename, image_filename, snapshot_idx=snapshot_idx
    , renderer_folder=renderer_folder, width=width, height=height)
