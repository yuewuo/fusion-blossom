import fusion_blossom as fb
import math

## Initialization

class CustomRepetitionCode:
    """A customize repetition code with non-i.i.d. noise model"""
    def __init__(self, d, p_vec):
        assert len(p_vec) == d
        self.d = d
        self.p_vec = p_vec
    def get_initializer(self, max_half_weight=500):
        vertex_num = (self.d - 1) + 2
        virtual_vertices = [0, vertex_num - 1]
        real_weights = [math.log((1 - pe) / pe, math.e) for pe in self.p_vec]
        scale = max_half_weight / max(real_weights)
        half_weights = [round(we * scale) for we in real_weights]
        weighted_edges = [(i, i+1, 2 * half_weights[i]) for i in range(vertex_num-1)]
        return fb.SolverInitializer(vertex_num, weighted_edges, virtual_vertices)
    def get_positions(self):
        return [fb.VisualizePosition(0, i, 0) for i in range(self.d + 1)]

## Code Initialization

p_vec = [1e-3, 0.01, 0.01, 0.01, 0.01, 1e-3, 1e-3]
code = CustomRepetitionCode(d=len(p_vec), p_vec=p_vec)

## Construct Syndrome

syndrome = fb.SyndromePattern(defect_vertices=[1,5])

## Visualize Result

visualizer = None
if True:  # change to False to disable visualizer for faster decoding
    visualize_filename = fb.static_visualize_data_filename()
    positions = code.get_positions()
    visualizer = fb.Visualizer(filepath=visualize_filename, positions=positions)

initializer = code.get_initializer()
solver = fb.SolverSerial(initializer)

solver.solve(syndrome)

subgraph = solver.subgraph(visualizer)
print(f"Minimum Weight Parity Subgraph (MWPS): {subgraph}")  # Vec<EdgeIndex>

if __name__ == "__main__" and visualizer is not None:
    fb.print_visualize_link(filename=visualize_filename)
    fb.helper.open_visualizer(visualize_filename, open_browser=True)
