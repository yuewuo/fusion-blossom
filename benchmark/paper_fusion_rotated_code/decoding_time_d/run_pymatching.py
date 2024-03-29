import numpy as np
from scipy.sparse import csc_matrix, lil_matrix
import pymatching
import os, sys, time
import subprocess
from msgspec.json import decode
from msgspec import Struct


d_vec = [3, 5, 7]  # for debugging script
d_vec = [3, 5, 7, 9, 11, 13, 17, 19, 23, 27, 33, 39, 47, 57, 67, 81, 97]
p = 0.005
total_rounds = 1000

PYMATCHING_BATCH_DECODING = True

# first generate graph
git_root_dir = subprocess.run("git rev-parse --show-toplevel", cwd=os.path.dirname(os.path.abspath(__file__))
    , shell=True, check=True, capture_output=True).stdout.decode(sys.stdout.encoding).strip(" \r\n")
# useful folders
rust_dir = git_root_dir
benchmark_dir = os.path.join(git_root_dir, "benchmark")
script_dir = os.path.dirname(__file__)
tmp_dir = os.path.join(script_dir, "tmp")
os.makedirs(tmp_dir, exist_ok=True)  # make sure tmp directory exists
sys.path.insert(0, benchmark_dir)

for d in d_vec:
    syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}.syndromes")
    assert os.path.exists(syndrome_file_path), "run `run_fusion.py` first to generate syndrome data and graphs"


data_file = os.path.join(script_dir, "data_pymatching.txt")
with open(data_file, "w", encoding="utf8") as data_f:
    data_f.write("<d> <average_decoding_time> <average_decoding_time_per_round> <average_decoding_time_per_defect>\n")

    for d in d_vec:
        print(f"d = {d}")
        syndrome_file_path = os.path.join(tmp_dir, f"generated-d{d}.syndromes")

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
            weighted_edges_vec = initializer_str[weighted_edges_start:weighted_edges_end].split("],[")
            weighted_edges = np.ndarray((len(weighted_edges_vec), 3), dtype=np.int32)
            for i, weighted_edges_str in enumerate(weighted_edges_vec):
                [v1, v2, weight] = weighted_edges_str.split(",")
                weighted_edges[i,0] = int(v1)
                weighted_edges[i,1] = int(v2)
                weighted_edges[i,2] = int(weight)
            virtual_vertices_start = initializer_str.find("virtual_vertices") + 19
            virtual_vertices_end = initializer_str.find("]", virtual_vertices_start)
            virtual_vertices_vec = initializer_str[virtual_vertices_start:virtual_vertices_end].split(",")
            virtual_vertices = np.empty(len(virtual_vertices_vec), dtype=np.int32)
            for i, virtual_vertex_str in enumerate(virtual_vertices_vec):
                virtual_vertices[i] = int(virtual_vertex_str)
            initializer = SolverInitializer(vertex_num=vertex_num, weighted_edges=weighted_edges, virtual_vertices=virtual_vertices)
            assert initializer.vertex_num == (d + 1) * ((d+1) * (d+1) / 2)
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
            assert len(syndromes) == total_rounds
        print("initializer loaded")

        # construct the binary parity check matrix
        is_virtual = np.full(initializer.vertex_num, False, dtype=bool)
        for virtual_vertex in initializer.virtual_vertices:
            is_virtual[virtual_vertex] = True
        H = lil_matrix((initializer.vertex_num, len(initializer.weighted_edges)), dtype=np.int8)
        weights = np.full(len(initializer.weighted_edges), 0, dtype=np.int32)
        for i, [v1, v2, weight] in enumerate(initializer.weighted_edges):
            if not is_virtual[v1]:
                H[v1,i] = 1
            if not is_virtual[v2]:
                H[v2,i] = 1
            weights[i] = weight
        H = H.tocsc()
        print("initializer created")
        matching = pymatching.Matching(H, weights=weights)
        print("matching initialized")

        # run simulation
        raw_time_file = os.path.join(tmp_dir, f"raw_time_d{d}.txt")
        with open(raw_time_file, "w", encoding="utf8") as f:
            if PYMATCHING_BATCH_DECODING:
                prediction = matching.decode_batch(syndromes[:20])  # ignore performance of cold start
                start = time.perf_counter()
                prediction = matching.decode_batch(syndromes[20:])
                end = time.perf_counter()
                f.write(f"{end - start} {sum(defect_nums[20:])}\n")
            else:
                for i in range(total_rounds):
                    syndrome = syndromes[i]
                    # start timer
                    start = time.perf_counter()
                    prediction = matching.decode(syndromes[i])
                    end = time.perf_counter()
                    f.write(f"{end - start} {defect_nums[i]}\n")

        with open(raw_time_file, "r", encoding="utf8") as f:
            lines = f.readlines()
            if not PYMATCHING_BATCH_DECODING:
                assert len(lines) == total_rounds
                lines = lines[20:]  # like our profiling, skip the first 20 records to remove the effect of cold start
            raw_data = [line.split(" ") for line in lines]
            decoding_time_vec = [float(data[0]) for data in raw_data]
            defect_num_vec = [int(data[1]) for data in raw_data]

        average_decoding_time = sum(decoding_time_vec) / len(decoding_time_vec)
        average_decoding_time_per_round = average_decoding_time / (d + 1)
        if PYMATCHING_BATCH_DECODING:
            average_decoding_time_per_round /= (total_rounds - 20)
        average_decoding_time_per_defect = average_decoding_time / (sum(defect_num_vec) / len(defect_num_vec))
        data_f.write("%d %.5e %.5e %.5e\n" % (
            d,
            average_decoding_time,
            average_decoding_time_per_round,
            average_decoding_time_per_defect
        ))
        data_f.flush()
