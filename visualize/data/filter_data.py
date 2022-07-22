import sys, json

def main(data):
    print(filter_beautify_primal_data(data))

def filter_beautify_primal_data(data):
    for (idx, (name, snapshot)) in enumerate(data["snapshots"]):
        primal_nodes = snapshot["primal_nodes"]
        print(f"{idx}:")
        for (node_idx, primal_node) in enumerate(primal_nodes):
            print(f"    {node_idx}: {primal_node}")



if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("usage: <filename>")
        exit(0)
    filename = sys.argv[1]
    with open(filename, encoding="utf-8") as f:
        data = json.load(f)
    main(data)
