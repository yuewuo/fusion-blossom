import fusion_blossom as fb

## Code Initialization

code = fb.CodeCapacityPlanarCode(d=11, p=0.05, max_half_weight=500)

## Peek Graph [Optional]

# fb.helper.peek_code(code)  # comment out after constructing the syndrome

## Construct Syndrome

syndrome = fb.SyndromePattern([52])

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
