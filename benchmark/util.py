import json


class Profile:
    def __init__(self, filename):
        assert isinstance(filename, str)
        with open(filename, "r", encoding="utf8") as f:
            lines = f.readlines()
        self.partition_config = None
        self.entries = []
        for line_idx, line in enumerate(lines):
            line = line.strip("\r\n ")
            if line == "":
                break
            value = json.loads(line)
            if line_idx == 0:
                self.partition_config = PartitionConfig.from_json(value)
            elif line_idx == 1:
                self.benchmark_config = value
            else:
                self.entries.append(value)
    def __repr__(self):
        return f"Profile {{ partition_config: {self.partition_config}, entries: [...{len(self.entries)}] }}"
    def sum_decoding_time(self):
        decoding_time = 0
        for entry in self.entries:
            decoding_time += entry["decoding_time"]
        return decoding_time
    def average_decoding_time(self):
        return self.sum_decoding_time() / len(self.entries)
    def sum_syndrome_num(self):
        syndrome_num = 0
        for entry in self.entries:
            syndrome_num += entry["syndrome_num"]
        return syndrome_num
    def average_decoding_time_per_syndrome(self):
        return self.sum_decoding_time() / self.sum_syndrome_num()
    def sum_computation_cpu_seconds(self):
        total_computation_cpu_seconds = 0
        for entry in self.entries:
            computation_cpu_seconds = 0
            for event_time in entry["solver_profile"]["primal"]["event_time_vec"]:
                computation_cpu_seconds += event_time["end"] - event_time["children_return"]
            total_computation_cpu_seconds += computation_cpu_seconds
        return total_computation_cpu_seconds
    def average_computation_cpu_seconds(self):
        return self.sum_computation_cpu_seconds() / len(self.entries)

class VertexRange:
    def __init__(self, start, end):
        self.range = (start, end)
    def __repr__(self):
        return f"[{self.range[0]}, {self.range[1]}]"

class PartitionConfig:
    def __init__(self, vertex_num):
        self.vertex_num = vertex_num
        self.partitions = [VertexRange(0, vertex_num)]
        self.fusions = []
    def __repr__(self):
        return f"PartitionConfig {{ vertex_num: {self.vertex_num}, partitions: {self.partitions}, fusions: {self.fusions} }}"
    @staticmethod
    def from_json(value):
        vertex_num = value['vertex_num']
        config = PartitionConfig(vertex_num)
        config.partitions.clear()
        for range in value['partitions']:
            config.partitions.append(VertexRange(range[0], range[1]))
        for pair in value['fusions']:
            config.fusions.append((pair[0], pair[1]))
        return config
