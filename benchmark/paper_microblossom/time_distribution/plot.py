import matplotlib.pyplot as plt
import numpy as np

d_vec = [d for d in range(3, 52, 2)]

# Example data
n_points = len(d_vec)
region_labels = [
    "others",
    "syndrome loading",
    "dual module",
    "primal: simple",
    "primal: complex",
]
region_colors = [
    "lightgray",
    "tab:blue",
    "tab:orange",
    "tab:green",
    "tab:red",
]

# Create a stacked bar plot
fig, ax = plt.subplots(figsize=(10, 7))

# Create the initial bottom position for each data point
bottoms = np.zeros(n_points)

# Plot each region as a stacked bar segment
data = []
with open("./distribution.txt") as f:
    for line in f.readlines():
        line = line.strip("\r\n ")
        if line.startswith("#"):
            continue
        lst = line.split(" ")
        assert len(lst) == 9
        (
            d,
            decoded,
            add_defects,
            primal,
            dual,
            simple_match,
            complex_match,
            speedup1,
            speedup2,
        ) = lst
        decoded = float(decoded)
        array = [
            float(add_defects) / decoded,
            float(dual) / decoded,
            float(simple_match) / decoded,
            float(complex_match) / decoded,
        ]
        array = [1 - sum(array)] + array
        data.append(array)
data = np.array(data)

labels = [f"{d}" for d in d_vec]
for i in range(len(region_labels)):
    ax.bar(
        labels,
        data[:, i],
        bottom=bottoms,
        label=region_labels[i],
        color=region_colors[i],
    )
    bottoms += data[:, i]

# Add legend and labels
ax.legend(title="Task")
ax.set_xlabel("Code Distance $d$")
ax.set_ylabel("Proportion")
ax.set_title("Distribution of Tasks in Fusion Blossom")

plt.show()
