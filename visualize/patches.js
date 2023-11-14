import * as gui3d from './gui3d.js'
import * as THREE from 'three'
const { ref, reactive, watch, computed } = Vue


export async function visualize_paper_weighted_union_find_decoder() {
    this.warning_message = "please adjust your screen to 1920 * 1080 in order to get best draw effect (use developer tool of your browser if your screen resolution is not native 1920 * 1080); The default output image is set to 2634 * 2155 in this case, click \"download\" to save it."
    this.lock_view = true
    // hide virtual vertices
    gui3d.controller.virtual_vertex_opacity.setValue(0)
    gui3d.controller.virtual_vertex_outline_opacity.setValue(0)
    gui3d.controller.real_vertex_opacity.setValue(1)
    gui3d.controller.edge_opacity.setValue(0.05)
    gui3d.controller.vertex_radius_scale.setValue(0.7)
    gui3d.controller.edge_radius_scale.setValue(0.7)
    await gui3d.wait_changes()  // make sure all changes have been applied
    // adjust camera location (use `camera.value.position`, `camera.value.quaternion` and `camera.value.zoom` to update it here)
    gui3d.camera.value.position.set(498.95430264554204, 226.03495620714534, 836.631820123962)
    gui3d.camera.value.lookAt(0, 0, 0)
    gui3d.camera.value.zoom = 1.43
    gui3d.camera.value.updateProjectionMatrix()  // need to call after setting zoom
    // remove the bottom vertices
    const fusion_data = gui3d.active_fusion_data.value
    const snapshot_idx = gui3d.active_snapshot_idx.value
    const snapshot = fusion_data.snapshots[snapshot_idx][1]
    for (let [i, vertex] of snapshot.vertices.entries()) {
        let position = fusion_data.positions[i]
        if (position.t <= -3) {
            const vertex_mesh = gui3d.vertex_meshes[i]
            vertex_mesh.visible = false
            const vertex_outline_mesh = gui3d.vertex_outline_meshes[i]
            vertex_outline_mesh.visible = false
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
    // add bottom image
    var image_data
    var image_buffer
    try {
        let response = await fetch('./img/basic_CSS_3D_bottom_image.png')
        image_buffer = await response.arrayBuffer()
        image_data = "data:image/png;base64," + gui3d.base64_encode(image_buffer)
    } catch (e) {
        this.error_message = "fetch image error"
        throw e
    }
    var bottom_image_texture
    if (gui3d.is_mock) {  // image doesn't work, has to use https://threejs.org/docs/#api/en/textures/DataTexture
        const image = await mocker.read_from_png_buffer(image_buffer)
        bottom_image_texture = new THREE.DataTexture(image.bitmap.data, image.bitmap.width, image.bitmap.height)
        bottom_image_texture.minFilter = THREE.LinearFilter
        bottom_image_texture.flipY = true  // otherwise it's inconsistent with image loader
        bottom_image_texture.needsUpdate = true
    } else {
        const bottom_image_loader = new THREE.TextureLoader()
        bottom_image_texture = await new Promise((resolve, reject) => {
            bottom_image_loader.load(image_data, resolve, undefined, reject)
        })
    }
    const bottom_image_material = new THREE.MeshStandardMaterial({
        map: bottom_image_texture,
        side: THREE.DoubleSide,
    })
    const bottom_image_geometry = new THREE.PlaneGeometry(5, 5, 100, 100)
    const bottom_image_mesh = new THREE.Mesh(bottom_image_geometry, bottom_image_material)
    bottom_image_mesh.position.set(0, -2.8, 0)
    bottom_image_mesh.rotateX(-Math.PI / 2)
    gui3d.scene.add(bottom_image_mesh)
    if (true) {  // add transparent layers
        // change the color of vertices
        const stab_z_material = gui3d.real_vertex_material.clone()
        stab_z_material.color = new THREE.Color(0xCFE2F3)
        const stab_x_material = gui3d.real_vertex_material.clone()
        stab_x_material.color = new THREE.Color(0xFFFF00)
        for (let [i, vertex] of snapshot.vertices.entries()) {
            let position = fusion_data.positions[i]
            const vertex_mesh = gui3d.vertex_meshes[i]
            if (!vertex.s && !vertex.v) {
                vertex_mesh.material = position.i % 2 == 0 ? stab_z_material : stab_x_material
            }
        }
        // remove all edges, otherwise too messy
        for (let [i, edge] of snapshot.edges.entries()) {
            if (edge.lg + edge.rg == 0) {
                const left_position = fusion_data.positions[edge.l]
                const right_position = fusion_data.positions[edge.r]
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
        const transparent_layer_material = new THREE.MeshStandardMaterial({
            // color: 0xffff00,
            map: bottom_image_texture,
            opacity: 0.5,
            transparent: true,
            side: THREE.DoubleSide,
            depthWrite: false,  // otherwise it will block something...
        })
        for (let i = 0; i < 3; ++i) {
            const transparent_layer_mesh = new THREE.Mesh(bottom_image_geometry, transparent_layer_material)
            transparent_layer_mesh.position.set(0, -1 + i * 2 - 0.01, 0)
            transparent_layer_mesh.rotateX(-Math.PI / 2)
            gui3d.scene.add(transparent_layer_mesh)
        }
    }
    // set background as white to prevent strange pixels around bottom image
    gui3d.scene.background = new THREE.Color(0xffffff)
    // set output scale
    this.export_scale_selected = Math.pow(10, 3 / 10)
}

function retain_only_indices_smaller_than(snapshot_select, retain_index) {
    const snapshot = gui3d.active_fusion_data.value.snapshots[snapshot_select][1]
    let vertices = []
    for (let [i, vertex] of snapshot.vertices.entries()) {
        if (i < retain_index) {
            vertices.push(vertex)
        }
    }
    snapshot.vertices = vertices
    let edges = []
    for (let [i, edge] of snapshot.edges.entries()) {
        if (edge.l < retain_index || edge.r < retain_index) {
            edges.push(edge)
        }
    }
    snapshot.edges = edges
    let dual_nodes = []
    for (let [i, dual_node] of snapshot.dual_nodes.entries()) {
        let has_dual_node = false
        if (dual_node.s != null && dual_node.s < retain_index) {
            has_dual_node = true
        }
        for (let node_index of dual_node.b) {
            if (node_index < retain_index) {
                has_dual_node = true
            }
        }
        if (has_dual_node) {
            dual_nodes.push(dual_node)
        }
    }
    snapshot.dual_nodes = dual_nodes
}

function shift_all_positions_upward(shift_t = null) {
    if (shift_t == null) {
        shift_t = gui3d.active_fusion_data.value.positions[0].t
    }
    for (let position of gui3d.active_fusion_data.value.positions) {
        position.t -= shift_t
    }
}

export async function visualize_rough_idea_fusion_blossom() {
    // select changed
    watch(gui3d.active_snapshot_idx, () => {
        // console.log(gui3d.active_snapshot_idx.value)
    })
    // shift all positions upwards so that the lowest t = 0
    shift_all_positions_upward()
    // keep only part of vertices for each step
    const layer_amount = 56
    let layer_counter = 0;
    for (const _ of Array(4).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 1)
    for (const _ of Array(1).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 2)
    for (const _ of Array(1).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 3)
    for (const _ of Array(3).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 4)
    for (const _ of Array(5).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 5)
    for (const _ of Array(1).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 6)
    for (const _ of Array(1).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 7)
    for (const _ of Array(1).keys()) retain_only_indices_smaller_than(layer_counter++, layer_amount * 8)
    // updated the fusion data, redraw
    this.show_snapshot(gui3d.active_snapshot_idx.value)
    gui3d.refresh_snapshot_data()
}
