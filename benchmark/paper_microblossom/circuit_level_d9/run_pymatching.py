import os
import sys
import time
import subprocess
import pymatching
from scipy.sparse import lil_matrix
import numpy as np
from msgspec.json import decode
from msgspec import Struct

git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(
    __file__)), shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
sys.path.insert(0, benchmark_dir)

if True:
    from util import *
    import util
util.FUSION_BLOSSOM_ENABLE_UNSAFE_POINTER = True  # better performance, still safe
compile_code_if_necessary()

d = 9
noisy_measurements = d
p = 0.001
total_rounds = 100000


syndrome_file_path = os.path.join(
    tmp_dir, f"generated.T{noisy_measurements}.syndromes")
assert os.path.exists(syndrome_file_path)


# load the generated graph and syndrome
class SolverInitializer:
    def __init__(self, vertex_num, weighted_edges, virtual_vertices):
        self.vertex_num = vertex_num
        self.weighted_edges = weighted_edges
        self.virtual_vertices = virtual_vertices


class SyndromePattern(Struct):
    defect_vertices: list[int]
    erasures: list[int]


syndromes = []
defect_nums = []
with open(syndrome_file_path, "r", encoding='utf8') as f:
    head = f.readline()
    assert head.startswith("Syndrome Pattern v1.0 ")
    # Syndrome Pattern v1.0   <initializer> <positions> <syndrome_pattern>*
    initializer_str = f.readline()
    vertex_num_start = initializer_str.find("vertex_num") + 12
    vertex_num_end = initializer_str.find(",", vertex_num_start)
    vertex_num = int(initializer_str[vertex_num_start:vertex_num_end])
    weighted_edges_start = initializer_str.find("weighted_edges") + 18
    weighted_edges_end = initializer_str.find("]]", weighted_edges_start)
    weighted_edges_vec = initializer_str[weighted_edges_start:weighted_edges_end].split(
        "],[")
    weighted_edges = np.ndarray((len(weighted_edges_vec), 3), dtype=np.int32)
    for i, weighted_edges_str in enumerate(weighted_edges_vec):
        [v1, v2, weight] = weighted_edges_str.split(",")
        weighted_edges[i, 0] = int(v1)
        weighted_edges[i, 1] = int(v2)
        weighted_edges[i, 2] = int(weight)
    virtual_vertices_start = initializer_str.find("virtual_vertices") + 19
    virtual_vertices_end = initializer_str.find("]", virtual_vertices_start)
    virtual_vertices_vec = initializer_str[virtual_vertices_start:virtual_vertices_end].split(
        ",")
    virtual_vertices = np.empty(len(virtual_vertices_vec), dtype=np.int32)
    for i, virtual_vertex_str in enumerate(virtual_vertices_vec):
        virtual_vertices[i] = int(virtual_vertex_str)
    initializer = SolverInitializer(
        vertex_num=vertex_num, weighted_edges=weighted_edges, virtual_vertices=virtual_vertices)
    assert initializer.vertex_num == (
        noisy_measurements + 1) * (d+1) * (d+1) // 2
    positions = f.readline()  # don't care
    line = f.readline()
    while line != "":
        syndrome_pattern = decode(line, type=SyndromePattern)
        syndrome = np.full(initializer.vertex_num, 0, dtype=np.int8)
        for defect_vertex in syndrome_pattern.defect_vertices:
            syndrome[defect_vertex] = 1
        syndromes.append(syndrome)
        defect_nums.append(len(syndrome_pattern.defect_vertices))
        line = f.readline()
    print(len(syndromes))
    assert len(syndromes) >= total_rounds
print("initializer loaded")

# construct the binary parity check matrix
is_virtual = np.full(initializer.vertex_num, False, dtype=bool)
for virtual_vertex in initializer.virtual_vertices:
    is_virtual[virtual_vertex] = True
H = lil_matrix((initializer.vertex_num, len(
    initializer.weighted_edges)), dtype=np.int8)
weights = np.full(len(initializer.weighted_edges), 0, dtype=np.int32)
for i, [v1, v2, weight] in enumerate(initializer.weighted_edges):
    if not is_virtual[v1]:
        H[v1, i] = 1
    if not is_virtual[v2]:
        H[v2, i] = 1
    weights[i] = weight
H = H.tocsc()
print("initializer created")
matching = pymatching.Matching(H, weights=weights)
print("matching initialized")


# ignore performance of cold start
prediction = matching.decode_batch(syndromes[:20])
start = time.perf_counter()
prediction = matching.decode_batch(syndromes[20:])
end = time.perf_counter()

decoding_time = end - start
print(f"decoding time: {decoding_time}")
avr_decoding_latency = decoding_time / (len(syndromes) - 20)
print(f"average decoding latency: {avr_decoding_latency:e}")
