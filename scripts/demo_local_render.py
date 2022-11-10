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

# you can patch the visualizer in JavaScript
patch_script = """
// you can use `Vue`, `THREE`, `gui3d`
// `this` is bind to the Vue app
export async function patch() {
    console.log("patch begin")
    gui3d.camera.value.position.set(100, 800, 836.631820123962)
    gui3d.camera.value.lookAt(0, 0, 0)
    gui3d.camera.value.zoom = 0.9
    gui3d.camera.value.updateProjectionMatrix()  // need to call after setting zoom
}
"""

fb.helper.local_render_visualizer(visualize_filename, image_filename, snapshot_idx=snapshot_idx
    , renderer_folder=renderer_folder, width=width, height=height, patch_script=patch_script)
