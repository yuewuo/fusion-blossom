import sys, json, copy

def main(data):
    # filter_beautify_primal_data(data)
    filter_beautify_dual_data(data)

def filter_beautify_primal_data(data):
    for (idx, (name, snapshot)) in enumerate(data["snapshots"]):
        primal_nodes = snapshot["primal_nodes"]
        print(f"{idx}: ({name})")
        for (node_idx, primal_node) in enumerate(primal_nodes):
            print(f"    {node_idx}: {primal_node}")

def filter_beautify_dual_data(data):
    for (idx, (name, snapshot)) in enumerate(data["snapshots"]):
        dual_nodes = snapshot["dual_nodes"]
        print(f"{idx}: ({name})")
        for (node_idx, dual_node) in enumerate(dual_nodes):
            if dual_node is None:
                print(f"    {node_idx}: None")
                continue
            dual_node = copy.copy(dual_node)
            dual_node.pop("b")
            print(f"    {node_idx}: {dual_node}")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: <filename>")
        exit(0)
    filename = sys.argv[1]
    with open(filename, encoding="utf-8") as f:
        data = json.load(f)
    main(data)
