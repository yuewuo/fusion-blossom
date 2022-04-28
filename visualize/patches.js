import * as gui3d from './gui3d.js'
import * as THREE from 'three'

export async function visualize_paper_weighted_union_find_decoder() {
    this.warning_message = "please adjust your screen to 1920 * 1080 in order to get best draw effect (use developer tool of your browser if your screen resolution is not native 1920 * 1080)"
    this.lock_view = true
    // hide virtual nodes
    gui3d.controller.virtual_node_opacity.setValue(0)
    gui3d.controller.virtual_node_outline_opacity.setValue(0)
    gui3d.controller.real_node_opacity.setValue(1)
    gui3d.controller.edge_opacity.setValue(0.05)
    gui3d.controller.node_radius_scale.setValue(0.7)
    gui3d.controller.edge_radius_scale.setValue(0.7)
    await Vue.nextTick()  // make sure all changes have been applied
    // adjust camera location (use `camera.value.position` and `camera.value.zoom` to update it here)
    gui3d.camera.value.position.set(498.95430264554204, 226.03495620714534, 836.631820123962)
    gui3d.camera.value.zoom = 1.43
    camera.value.updateProjectionMatrix()  // need to call after setting zoom
    // remove the bottom nodes
    const fusion_data = gui3d.active_fusion_data.value
    const snapshot_idx = gui3d.active_snapshot_idx.value
    const snapshot = fusion_data.snapshots[snapshot_idx][1]
    for (let [i, node] of snapshot.nodes.entries()) {
        let position = fusion_data.positions[i]
        if (position.t <= -3) {
            const node_mesh = node_meshes[i]
            node_mesh.visible = false
            const node_outline_mesh = node_outline_meshes[i]
            node_outline_mesh.visible = false
        }
    }
    // remove bottom edges except straight up and down
    for (let [i, edge] of snapshot.edges.entries()) {
        const left_position = fusion_data.positions[edge.l]
        const right_position = fusion_data.positions[edge.r]
        if (left_position.t <= -3 || right_position.t <= -3) {
            let same_i_j = (left_position.i == right_position.i) && (left_position.j == right_position.j)
            if (!same_i_j) {
                for (let edge_meshes of [gui3d.left_edge_meshes, gui3d.middle_edge_meshes, gui3d.right_edge_meshes]) {
                    for (let j of [0, 1]) {
                        const edge_mesh = edge_meshes[i][j]
                        edge_mesh.visible = false
                    }
                }
            }
        }
    }
}
