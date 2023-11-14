
function cmd_data_source() {
    return [window.fusion_data, window.snapshot_idx]
}

window.cmd = {

    // report defect vertices
    get_defect_vertices() {
        let [fusion_data, snapshot_idx] = cmd_data_source()
        const snapshot = fusion_data.snapshots[snapshot_idx][1]
        let defect_vertices = []
        for (let [i, vertex] of snapshot.vertices.entries()) {
            if (vertex.s) {
                defect_vertices.push(i)
            }
        }
        return defect_vertices
    },

}
