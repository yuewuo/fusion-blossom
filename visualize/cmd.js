
function cmd_data_source() {
    return [window.fusion_data, window.snapshot_idx]
}

window.cmd = {

    // report syndrome vertices
    get_syndrome() {
        let [fusion_data, snapshot_idx] = cmd_data_source()
        const snapshot = fusion_data.snapshots[snapshot_idx][1]
        let syndrome_vertices = []
        for (let [i, vertex] of snapshot.vertices.entries()) {
            if (vertex.s) {
                syndrome_vertices.push(i)
            }
        }
        return syndrome_vertices
    },
    
}
