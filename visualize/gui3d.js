// 3d related apis

import * as THREE from 'three'
import { OrbitControls } from './node_modules/three/examples/jsm/controls/OrbitControls.js'
import { ConvexGeometry } from './node_modules/three/examples/jsm/geometries/ConvexGeometry.js'
import Stats from './node_modules/three/examples/jsm/libs/stats.module.js'
import GUI from './node_modules/three/examples/jsm/libs/lil-gui.module.min.js'


if (typeof window === 'undefined' || typeof document === 'undefined') {
    global.THREE = THREE
    global.mocker = await import('./mocker.js')
}

// to work both in browser and nodejs
if (typeof Vue === 'undefined') {
    global.Vue = await import('vue')
}
const { ref, reactive, watch, computed } = Vue

const urlParams = new URLSearchParams(window.location.search)
export const root = document.documentElement

export const is_mock = typeof mockgl !== 'undefined'
export const webgl_renderer_context = is_mock ? mockgl : () => undefined

export const window_inner_width = ref(0)
export const window_inner_height = ref(0)
function on_resize() {
    window_inner_width.value = window.innerWidth
    window_inner_height.value = window.innerHeight
}
on_resize()
window.addEventListener('resize', on_resize)
window.addEventListener('orientationchange', on_resize)

export const sizes = reactive({
    control_bar_width: 0,
    canvas_width: 0,
    canvas_height: 0,
    scale: 1,
})

watch([window_inner_width, window_inner_height], () => {
    sizes.scale = window_inner_width.value / 1920
    if (sizes.scale > window_inner_height.value / 1080) {  // ultra-wide
        sizes.scale = window_inner_height.value / 1080
    }
    if (sizes.scale < 0.5) {
        sizes.scale = 0.5
    }
    if (window_inner_width.value * 0.9 < 300) {
        sizes.scale = window_inner_width.value / 600 * 0.9
    }
    root.style.setProperty('--s', sizes.scale)
    // sizes.scale = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--s'))
    sizes.control_bar_width = 600 * sizes.scale
    sizes.canvas_width = window_inner_width.value - sizes.control_bar_width
    sizes.canvas_height = window_inner_height.value
}, { immediate: true })
if (is_mock) {
    sizes.canvas_width = mocker.mock_canvas_width
    sizes.canvas_height = mocker.mock_canvas_height
}

export const scene = new THREE.Scene()
scene.background = new THREE.Color(0xffffff)  // for better image output
scene.add(new THREE.AmbientLight(0xffffff))
window.scene = scene
export const perspective_camera = new THREE.PerspectiveCamera(75, sizes.canvas_width / sizes.canvas_height, 0.1, 10000)
const orthogonal_camera_init_scale = 6
export const orthogonal_camera = new THREE.OrthographicCamera(sizes.canvas_width / sizes.canvas_height * (-orthogonal_camera_init_scale)
    , sizes.canvas_width / sizes.canvas_height * orthogonal_camera_init_scale, orthogonal_camera_init_scale, -orthogonal_camera_init_scale, 0.1, 100000)
export const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, context: webgl_renderer_context() })

document.body.appendChild(renderer.domElement)

watch(sizes, () => {
    perspective_camera.aspect = sizes.canvas_width / sizes.canvas_height
    perspective_camera.updateProjectionMatrix()
    orthogonal_camera.left = sizes.canvas_width / sizes.canvas_height * (-orthogonal_camera_init_scale)
    orthogonal_camera.right = sizes.canvas_width / sizes.canvas_height * (orthogonal_camera_init_scale)
    orthogonal_camera.updateProjectionMatrix()
    renderer.setSize(sizes.canvas_width, sizes.canvas_height, false)
    const ratio = window.devicePixelRatio  // looks better on devices with a high pixel ratio, such as iPhones with Retina displays
    renderer.setPixelRatio(ratio)
    const canvas = renderer.domElement
    canvas.width = sizes.canvas_width * ratio
    canvas.height = sizes.canvas_height * ratio
    canvas.style.width = `${sizes.canvas_width}px`
    canvas.style.height = `${sizes.canvas_height}px`
}, { immediate: true })

export const orbit_control_perspective = new OrbitControls(perspective_camera, renderer.domElement)
export const orbit_control_orthogonal = new OrbitControls(orthogonal_camera, renderer.domElement)
export const enable_control = ref(true)
watch(enable_control, (enabled) => {
    orbit_control_perspective.enabled = enabled
    orbit_control_orthogonal.enabled = enabled
}, { immediate: true })
window.enable_control = enable_control

export const use_perspective_camera = ref(false)
export const camera = computed(() => {
    return use_perspective_camera.value ? perspective_camera : orthogonal_camera
})
window.camera = camera
export const orbit_control = computed(() => {
    return use_perspective_camera.value ? orbit_control_perspective : orbit_control_orthogonal
})

export function reset_camera_position(direction = "top") {
    for (let [camera, control, distance] of [[perspective_camera, orbit_control_perspective, 8], [orthogonal_camera, orbit_control_orthogonal, 1000]]) {
        control.reset()
        camera.position.x = (direction == "left" ? -distance : 0)
        camera.position.y = (direction == "top" ? distance : 0)
        camera.position.z = (direction == "front" ? distance : 0)
        camera.lookAt(0, 0, 0)
    }
}
reset_camera_position()

// const axesHelper = new THREE.AxesHelper( 5 )
// scene.add( axesHelper )

var stats
export const show_stats = ref(false)
if (!is_mock) {
    stats = Stats()
    document.body.appendChild(stats.dom)
    watch(show_stats, function () {
        if (show_stats.value) {
            stats.dom.style.display = "block"
        } else {
            stats.dom.style.display = "none"
        }
    }, { immediate: true })
    watch(sizes, () => {
        stats.dom.style.transform = `scale(${sizes.scale})`
        stats.dom.style["transform-origin"] = "left top"
    }, { immediate: true })
}

export function animate() {
    requestAnimationFrame(animate)
    orbit_control.value.update()
    renderer.render(scene, camera.value)
    if (stats) stats.update()
}

// commonly used vectors
const zero_vector = new THREE.Vector3(0, 0, 0)
const unit_up_vector = new THREE.Vector3(0, 1, 0)

// create common geometries
const segment = parseInt(urlParams.get('segment') || 128)  // higher segment will consume more GPU resources
const vertex_radius = parseFloat(urlParams.get('vertex_radius') || 0.15)
export const vertex_radius_scale = ref(1)
const scaled_vertex_radius = computed(() => {
    return vertex_radius * vertex_radius_scale.value
})
const vertex_geometry = new THREE.SphereGeometry(vertex_radius, segment, segment)
const edge_radius = parseFloat(urlParams.get('edge_radius') || 0.03)
const edge_radius_scale = ref(1)
const scaled_edge_radius = computed(() => {
    return edge_radius * edge_radius_scale.value
})
const edge_geometry = new THREE.CylinderGeometry(edge_radius, edge_radius, 1, segment, 1, true)
edge_geometry.translate(0, 0.5, 0)

// create common materials
export const defect_vertex_material = new THREE.MeshStandardMaterial({
    color: 0xff0000,
    opacity: 1,
    transparent: true,
    side: THREE.FrontSide,
})
export const disabled_mirror_vertex_material = new THREE.MeshStandardMaterial({
    color: 0x1e81b0,
    opacity: 1,
    transparent: true,
    side: THREE.FrontSide,
})
export const real_vertex_material = new THREE.MeshStandardMaterial({
    color: 0xffffff,
    opacity: 0.1,
    transparent: true,
    side: THREE.FrontSide,
})
export const virtual_vertex_material = new THREE.MeshStandardMaterial({
    color: 0xffff00,
    opacity: 0.5,
    transparent: true,
    side: THREE.FrontSide,
})
export const defect_vertex_outline_material = new THREE.MeshStandardMaterial({
    color: 0x000000,
    opacity: 1,
    transparent: true,
    side: THREE.BackSide,
})
export const real_vertex_outline_material = new THREE.MeshStandardMaterial({
    color: 0x000000,
    opacity: 1,
    transparent: true,
    side: THREE.BackSide,
})
export const virtual_vertex_outline_material = new THREE.MeshStandardMaterial({
    color: 0x000000,
    opacity: 1,
    transparent: true,
    side: THREE.BackSide,
})
export const edge_material = new THREE.MeshStandardMaterial({
    color: 0x000000,
    opacity: 0.1,
    transparent: true,
    side: THREE.FrontSide,
})
export const grown_edge_material = new THREE.MeshStandardMaterial({
    color: 0xff0000,
    opacity: 1,
    transparent: true,
    side: THREE.FrontSide,
})
export const subgraph_edge_material = new THREE.MeshStandardMaterial({
    color: 0x0000ff,
    opacity: 1,
    transparent: true,
    side: THREE.FrontSide,
})
export const hover_material = new THREE.MeshStandardMaterial({  // when mouse is on this object (vertex or edge)
    color: 0x6FDFDF,
    side: THREE.DoubleSide,
})
export const selected_material = new THREE.MeshStandardMaterial({  // when mouse is on this object (vertex or edge)
    color: 0x4B7BE5,
    side: THREE.DoubleSide,
})
export const blossom_convex_material = new THREE.MeshStandardMaterial({
    color: 0x82A284,
    opacity: 0.7,
    transparent: true,
    side: THREE.BackSide,
})
export const blossom_convex_material_2d = blossom_convex_material.clone()
blossom_convex_material_2d.side = THREE.DoubleSide

// meshes that can be reused across different snapshots
export var vertex_meshes = []
window.vertex_meshes = vertex_meshes
export const outline_ratio = ref(1.2)
export var vertex_outline_meshes = []
window.vertex_outline_meshes = vertex_outline_meshes
const scaled_vertex_outline_radius = computed(() => {
    return scaled_vertex_radius.value * outline_ratio.value
})
export var vertex_caches = []  // store some information that can be useful
export var left_edge_meshes = []
export var right_edge_meshes = []
export var middle_edge_meshes = []
export var edge_caches = []  // store some information that can be useful
window.left_edge_meshes = left_edge_meshes
window.right_edge_meshes = right_edge_meshes
window.middle_edge_meshes = middle_edge_meshes
export var blossom_convex_meshes = []
window.blossom_convex_meshes = blossom_convex_meshes

// update the sizes of objects
watch(vertex_radius_scale, (newVal, oldVal) => {
    vertex_geometry.scale(1 / oldVal, 1 / oldVal, 1 / oldVal)
    vertex_geometry.scale(newVal, newVal, newVal)
})
watch(edge_radius_scale, (newVal, oldVal) => {
    edge_geometry.scale(1 / oldVal, 1, 1 / oldVal)
    edge_geometry.scale(newVal, 1, newVal)
})
watch([scaled_edge_radius, scaled_vertex_outline_radius], async () => {
    await refresh_snapshot_data()
})
function update_mesh_outline(mesh) {
    mesh.scale.x = outline_ratio.value
    mesh.scale.y = outline_ratio.value
    mesh.scale.z = outline_ratio.value
}
watch([outline_ratio, vertex_radius_scale], () => {
    for (let mesh of vertex_outline_meshes) {
        update_mesh_outline(mesh)
    }
})

// helper functions
export function compute_vector3(data_position) {
    let vector = new THREE.Vector3(0, 0, 0)
    load_position(vector, data_position)
    return vector
}
export function load_position(mesh_position, data_position) {
    mesh_position.z = data_position.i
    mesh_position.x = data_position.j
    mesh_position.y = data_position.t
}

/// translate to a format that is easy to plot (gracefully handle the overgrown edges)
export function translate_edge(left_grown, right_grown, weight) {
    console.assert(left_grown >= 0 && right_grown >= 0, "grown should be non-negative")
    if (left_grown + right_grown <= weight) {
        return [left_grown, right_grown]
    } else {
        const middle = (left_grown + weight - right_grown) / 2
        if (middle < 0) {
            return [0, weight]
        }
        if (middle > weight) {
            return [weight, 0]
        }
        return [middle, weight - middle]
    }
}

export const active_fusion_data = ref(null)
export const active_snapshot_idx = ref(0)
window.is_vertices_2d_plane = false  // will be true only if all vertices' t position = 0
export async function refresh_snapshot_data() {
    // console.log("refresh_snapshot_data")
    if (active_fusion_data.value != null) {  // no fusion data provided
        const fusion_data = active_fusion_data.value
        const snapshot_idx = active_snapshot_idx.value
        const snapshot = fusion_data.snapshots[snapshot_idx][1]
        // clear hover and select
        current_hover.value = null
        let current_selected_value = JSON.parse(JSON.stringify(current_selected.value))
        current_selected.value = null
        await Vue.nextTick()
        await Vue.nextTick()
        // update vertex cache
        vertex_caches = []
        window.is_vertices_2d_plane = true
        for (let position of fusion_data.positions) {
            if (position.t != 0) {
                window.is_vertices_2d_plane = false
            }
            vertex_caches.push({
                position: {
                    center: compute_vector3(position),
                }
            })
        }
        // draw vertices
        for (let [i, vertex] of snapshot.vertices.entries()) {
            if (vertex == null) {
                if (i < vertex_meshes.length) {  // hide
                    vertex_meshes[i].visible = false
                }
                continue
            }
            let position = fusion_data.positions[i]
            while (vertex_meshes.length <= i) {
                const vertex_mesh = new THREE.Mesh(vertex_geometry, real_vertex_material)
                vertex_mesh.visible = false
                vertex_mesh.userData = {
                    type: "vertex",
                    vertex_index: vertex_meshes.length,
                }
                scene.add(vertex_mesh)
                vertex_meshes.push(vertex_mesh)
            }
            const vertex_mesh = vertex_meshes[i]
            load_position(vertex_mesh.position, position)
            if (vertex.mi != null && vertex.me == 0) {
                vertex_mesh.material = disabled_mirror_vertex_material
            } else if (vertex.s) {
                vertex_mesh.material = defect_vertex_material
            } else if (vertex.v) {
                vertex_mesh.material = virtual_vertex_material
            } else {
                vertex_mesh.material = real_vertex_material
            }
            vertex_mesh.visible = true
        }
        for (let i = snapshot.vertices.length; i < vertex_meshes.length; ++i) {
            vertex_meshes[i].visible = false
        }
        // draw edges
        let subgraph_set = {}
        if (snapshot.subgraph != null) {
            for (let edge_index of snapshot.subgraph) {
                subgraph_set[edge_index] = true
            }
        }
        let edge_offset = 0
        if (scaled_edge_radius.value < scaled_vertex_outline_radius.value) {
            edge_offset = Math.sqrt(Math.pow(scaled_vertex_outline_radius.value, 2) - Math.pow(scaled_edge_radius.value, 2))
        }
        edge_caches = []  // clear cache
        for (let [i, edge] of snapshot.edges.entries()) {
            if (edge == null) {
                if (i < left_edge_meshes.length) {  // hide
                    for (let j of [0, 1]) {
                        left_edge_meshes[i][j].visible = false
                        right_edge_meshes[i][j].visible = false
                        middle_edge_meshes[i][j].visible = false
                    }
                }
                continue
            }
            const left_position = fusion_data.positions[edge.l]
            const right_position = fusion_data.positions[edge.r]
            const relative = compute_vector3(right_position).add(compute_vector3(left_position).multiplyScalar(-1))
            const direction = relative.clone().normalize()
            // console.log(direction)
            const quaternion = new THREE.Quaternion()
            quaternion.setFromUnitVectors(unit_up_vector, direction)
            const reverse_quaternion = new THREE.Quaternion()
            reverse_quaternion.setFromUnitVectors(unit_up_vector, direction.clone().multiplyScalar(-1))
            let local_edge_offset = edge_offset
            const distance = relative.length()
            let edge_length = distance - 2 * edge_offset
            if (edge_length < 0) {  // edge length should be non-negative
                local_edge_offset = distance / 2
                edge_length = 0
            }
            const left_start = local_edge_offset
            const [left_grown, right_grown] = translate_edge(edge.lg, edge.rg, edge.w)
            let left_end = local_edge_offset + edge_length * (edge.w == 0 ? 0.5 : (left_grown / edge.w))  // always show 0-weight edge as fully-grown
            let right_end = local_edge_offset + edge_length * (edge.w == 0 ? 0.5 : (edge.w - right_grown) / edge.w)  // always show 0-weight edge as fully-grown
            const right_start = local_edge_offset + edge_length
            edge_caches.push({
                position: {
                    left_start: compute_vector3(left_position).add(relative.clone().multiplyScalar(left_start / distance)),
                    left_end: compute_vector3(left_position).add(relative.clone().multiplyScalar(left_end / distance)),
                    right_end: compute_vector3(left_position).add(relative.clone().multiplyScalar(right_end / distance)),
                    right_start: compute_vector3(left_position).add(relative.clone().multiplyScalar(right_start / distance)),
                }
            })
            // console.log(`${left_start}, ${left_end}, ${right_end}, ${right_start}`)
            for (let [start, end, edge_meshes, is_grown_part] of [[left_start, left_end, left_edge_meshes, true], [left_end, right_end, middle_edge_meshes, false]
                , [right_end, right_start, right_edge_meshes, true]]) {
                while (edge_meshes.length <= i) {
                    let two_edges = [null, null]
                    for (let j of [0, 1]) {
                        const edge_mesh = new THREE.Mesh(edge_geometry, edge_material)
                        edge_mesh.userData = {
                            type: "edge",
                            edge_index: edge_meshes.length,
                        }
                        edge_mesh.visible = false
                        scene.add(edge_mesh)
                        two_edges[j] = edge_mesh
                    }
                    edge_meshes.push(two_edges)
                }
                const start_position = compute_vector3(left_position).add(relative.clone().multiplyScalar(start / distance))
                const end_position = compute_vector3(left_position).add(relative.clone().multiplyScalar(end / distance))
                for (let j of [0, 1]) {
                    const edge_mesh = edge_meshes[i][j]
                    edge_mesh.position.copy(j == 0 ? start_position : end_position)
                    edge_mesh.scale.set(1, (end - start) / 2, 1)
                    edge_mesh.setRotationFromQuaternion(j == 0 ? quaternion : reverse_quaternion)
                    edge_mesh.visible = true
                    if (start >= end) {
                        edge_mesh.visible = false
                    }
                    edge_mesh.material = is_grown_part ? grown_edge_material : edge_material
                    if (snapshot.subgraph != null) {
                        edge_mesh.material = edge_material  // do not display grown edges
                    }
                    if (subgraph_set[i]) {
                        edge_mesh.material = subgraph_edge_material
                    }
                }
            }
        }
        for (let i = snapshot.edges.length; i < left_edge_meshes.length; ++i) {
            for (let j of [0, 1]) {
                left_edge_meshes[i][j].visible = false
                right_edge_meshes[i][j].visible = false
                middle_edge_meshes[i][j].visible = false
            }
        }
        // draw vertex outlines
        for (let [i, vertex] of snapshot.vertices.entries()) {
            if (vertex == null) {
                if (i < vertex_outline_meshes.length) {  // hide
                    vertex_outline_meshes[i].visible = false
                }
                continue
            }
            let position = fusion_data.positions[i]
            while (vertex_outline_meshes.length <= i) {
                const vertex_outline_mesh = new THREE.Mesh(vertex_geometry, real_vertex_outline_material)
                vertex_outline_mesh.visible = false
                update_mesh_outline(vertex_outline_mesh)
                scene.add(vertex_outline_mesh)
                vertex_outline_meshes.push(vertex_outline_mesh)
            }
            const vertex_outline_mesh = vertex_outline_meshes[i]
            load_position(vertex_outline_mesh.position, position)
            if (vertex.s) {
                vertex_outline_mesh.material = defect_vertex_outline_material
            } else if (vertex.v) {
                vertex_outline_mesh.material = virtual_vertex_outline_material
            } else {
                vertex_outline_mesh.material = real_vertex_outline_material
            }
            vertex_outline_mesh.visible = true
        }
        for (let i = snapshot.vertices.length; i < vertex_meshes.length; ++i) {
            vertex_outline_meshes[i].visible = false
        }
        // draw convex
        if (snapshot.dual_nodes != null) {
            for (let blossom_convex_mesh of blossom_convex_meshes) {
                scene.remove(blossom_convex_mesh)
                blossom_convex_mesh.geometry.dispose()
            }
            for (let [i, dual_node] of snapshot.dual_nodes.entries()) {
                if (dual_node == null) { continue }
                if (snapshot.subgraph != null) { continue }  // do not display convex if subgraph is displayed
                // for child node in a blossom, this will not display properly; we should avoid plotting child nodes
                let display_node = dual_node.p == null && (dual_node.d > 0 || dual_node.o != null)
                if (display_node) {  // no parent and (positive dual variable or it's a blossom)
                    let points = []
                    if (dual_node.b != null) {
                        for (let [is_left, edge_index] of dual_node.b) {
                            let cached_position = edge_caches[edge_index].position
                            const edge = snapshot.edges[edge_index]
                            if (edge.ld == edge.rd && edge.lg + edge.rg >= edge.w) {
                                continue  // do not draw this edge, this is an internal edge
                            }
                            if (is_left) {
                                if (edge.lg == edge.w) {
                                    points.push(vertex_caches[edge.r].position.center.clone())
                                } else if (edge.lg == 0) {
                                    points.push(vertex_caches[edge.l].position.center.clone())
                                } else {
                                    points.push(cached_position.left_end.clone())
                                }
                            } else {
                                if (edge.rg == edge.w) {
                                    points.push(vertex_caches[edge.l].position.center.clone())
                                } else if (edge.rg == 0) {
                                    points.push(vertex_caches[edge.r].position.center.clone())
                                } else {
                                    points.push(cached_position.right_end.clone())
                                }
                            }
                        }
                    }
                    if (points.length >= 3) {  // only display if points is more than 3
                        if (window.is_vertices_2d_plane) {
                            // special optimization for 2D points, because ConvexGeometry doesn't work well on them
                            const points_2d = []
                            for (let point of points) {
                                points_2d.push([point.x, point.z])
                            }
                            const hull_points = hull(points_2d, 1)
                            const shape_points = []
                            for (let hull_point of hull_points) {
                                shape_points.push(new THREE.Vector2(hull_point[0], hull_point[1]));
                            }
                            const shape = new THREE.Shape(shape_points)
                            const geometry = new THREE.ShapeGeometry(shape)
                            const blossom_convex_mesh = new THREE.Mesh(geometry, blossom_convex_material_2d)
                            blossom_convex_mesh.position.set(0, -0.2, 0)  // place the plane to slightly below the vertices for better viz
                            blossom_convex_mesh.rotation.set(Math.PI / 2, 0, 0);
                            scene.add(blossom_convex_mesh)
                            blossom_convex_meshes.push(blossom_convex_mesh)
                        } else {
                            const geometry = new ConvexGeometry(points)
                            const blossom_convex_mesh = new THREE.Mesh(geometry, blossom_convex_material)
                            scene.add(blossom_convex_mesh)
                            blossom_convex_meshes.push(blossom_convex_mesh)
                        }
                    }
                }
            }
        }
        // reset select
        await Vue.nextTick()
        if (is_user_data_valid(current_selected_value)) {
            current_selected.value = current_selected_value
        }
    }
}
watch([active_fusion_data, active_snapshot_idx, scaled_vertex_outline_radius], refresh_snapshot_data)
export function show_snapshot(snapshot_idx, fusion_data) {
    active_snapshot_idx.value = snapshot_idx
    active_fusion_data.value = fusion_data
}

// configurations
const gui = new GUI({ width: 400, title: "render configurations" })
export const show_config = ref(false)
watch(show_config, () => {
    if (show_config.value) {
        gui.domElement.style.display = "block"
    } else {
        gui.domElement.style.display = "none"
    }
}, { immediate: true })
watch(sizes, () => {  // move render configuration GUI to 3D canvas
    // gui.domElement.style.right = sizes.control_bar_width + "px"
    gui.domElement.style.right = 0
}, { immediate: true })
const conf = {
    scene_background: scene.background,
    defect_vertex_color: defect_vertex_material.color,
    defect_vertex_opacity: defect_vertex_material.opacity,
    disabled_mirror_vertex_color: disabled_mirror_vertex_material.color,
    disabled_mirror_vertex_opacity: disabled_mirror_vertex_material.opacity,
    real_vertex_color: real_vertex_material.color,
    real_vertex_opacity: real_vertex_material.opacity,
    virtual_vertex_color: virtual_vertex_material.color,
    virtual_vertex_opacity: virtual_vertex_material.opacity,
    defect_vertex_outline_color: defect_vertex_outline_material.color,
    defect_vertex_outline_opacity: defect_vertex_outline_material.opacity,
    real_vertex_outline_color: real_vertex_outline_material.color,
    real_vertex_outline_opacity: real_vertex_outline_material.opacity,
    virtual_vertex_outline_color: virtual_vertex_outline_material.color,
    virtual_vertex_outline_opacity: virtual_vertex_outline_material.opacity,
    edge_color: edge_material.color,
    edge_opacity: edge_material.opacity,
    edge_side: edge_material.side,
    grown_edge_color: grown_edge_material.color,
    grown_edge_opacity: grown_edge_material.opacity,
    grown_edge_side: grown_edge_material.side,
    subgraph_edge_color: subgraph_edge_material.color,
    subgraph_edge_opacity: subgraph_edge_material.opacity,
    subgraph_edge_side: subgraph_edge_material.side,
    outline_ratio: outline_ratio.value,
    vertex_radius_scale: vertex_radius_scale.value,
    edge_radius_scale: edge_radius_scale.value,
}
const side_options = { "FrontSide": THREE.FrontSide, "BackSide": THREE.BackSide, "DoubleSide": THREE.DoubleSide }
export const controller = {}
window.controller = controller
controller.scene_background = gui.addColor(conf, 'scene_background').onChange(function (value) { scene.background = value })
const vertex_folder = gui.addFolder('vertex')
controller.defect_vertex_color = vertex_folder.addColor(conf, 'defect_vertex_color').onChange(function (value) { defect_vertex_material.color = value })
controller.defect_vertex_opacity = vertex_folder.add(conf, 'defect_vertex_opacity', 0, 1).onChange(function (value) { defect_vertex_material.opacity = Number(value) })
controller.disabled_mirror_vertex_color = vertex_folder.addColor(conf, 'disabled_mirror_vertex_color').onChange(function (value) { disabled_mirror_vertex_material.color = value })
controller.disabled_mirror_vertex_opacity = vertex_folder.add(conf, 'disabled_mirror_vertex_opacity', 0, 1).onChange(function (value) { disabled_mirror_vertex_material.opacity = Number(value) })
controller.real_vertex_color = vertex_folder.addColor(conf, 'real_vertex_color').onChange(function (value) { real_vertex_material.color = value })
controller.real_vertex_opacity = vertex_folder.add(conf, 'real_vertex_opacity', 0, 1).onChange(function (value) { real_vertex_material.opacity = Number(value) })
controller.virtual_vertex_color = vertex_folder.addColor(conf, 'virtual_vertex_color').onChange(function (value) { virtual_vertex_material.color = value })
controller.virtual_vertex_opacity = vertex_folder.add(conf, 'virtual_vertex_opacity', 0, 1).onChange(function (value) { virtual_vertex_material.opacity = Number(value) })
const vertex_outline_folder = gui.addFolder('vertex outline')
controller.defect_vertex_outline_color = vertex_outline_folder.addColor(conf, 'defect_vertex_outline_color').onChange(function (value) { defect_vertex_outline_material.color = value })
controller.defect_vertex_outline_opacity = vertex_outline_folder.add(conf, 'defect_vertex_outline_opacity', 0, 1).onChange(function (value) { defect_vertex_outline_material.opacity = Number(value) })
controller.real_vertex_outline_color = vertex_outline_folder.addColor(conf, 'real_vertex_outline_color').onChange(function (value) { real_vertex_outline_material.color = value })
controller.real_vertex_outline_opacity = vertex_outline_folder.add(conf, 'real_vertex_outline_opacity', 0, 1).onChange(function (value) { real_vertex_outline_material.opacity = Number(value) })
controller.virtual_vertex_outline_color = vertex_outline_folder.addColor(conf, 'virtual_vertex_outline_color').onChange(function (value) { virtual_vertex_outline_material.color = value })
controller.virtual_vertex_outline_opacity = vertex_outline_folder.add(conf, 'virtual_vertex_outline_opacity', 0, 1).onChange(function (value) { virtual_vertex_outline_material.opacity = Number(value) })
const edge_folder = gui.addFolder('edge')
controller.edge_color = edge_folder.addColor(conf, 'edge_color').onChange(function (value) { edge_material.color = value })
controller.edge_opacity = edge_folder.add(conf, 'edge_opacity', 0, 1).onChange(function (value) { edge_material.opacity = Number(value) })
controller.edge_side = edge_folder.add(conf, 'edge_side', side_options).onChange(function (value) { edge_material.side = Number(value) })
controller.grown_edge_color = edge_folder.addColor(conf, 'grown_edge_color').onChange(function (value) { grown_edge_material.color = value })
controller.grown_edge_opacity = edge_folder.add(conf, 'grown_edge_opacity', 0, 1).onChange(function (value) { grown_edge_material.opacity = Number(value) })
controller.grown_edge_side = edge_folder.add(conf, 'grown_edge_side', side_options).onChange(function (value) { grown_edge_material.side = Number(value) })
controller.subgraph_edge_color = edge_folder.addColor(conf, 'subgraph_edge_color').onChange(function (value) { subgraph_edge_material.color = value })
controller.subgraph_edge_opacity = edge_folder.add(conf, 'subgraph_edge_opacity', 0, 1).onChange(function (value) { subgraph_edge_material.opacity = Number(value) })
controller.subgraph_edge_side = edge_folder.add(conf, 'subgraph_edge_side', side_options).onChange(function (value) { subgraph_edge_material.side = Number(value) })
const size_folder = gui.addFolder('size')
controller.outline_ratio = size_folder.add(conf, 'outline_ratio', 0.99, 2).onChange(function (value) { outline_ratio.value = Number(value) })
controller.vertex_radius_scale = size_folder.add(conf, 'vertex_radius_scale', 0.1, 5).onChange(function (value) { vertex_radius_scale.value = Number(value) })
controller.edge_radius_scale = size_folder.add(conf, 'edge_radius_scale', 0.1, 10).onChange(function (value) { edge_radius_scale.value = Number(value) })
watch(sizes, () => {
    gui.domElement.style.transform = `scale(${sizes.scale})`
    gui.domElement.style["transform-origin"] = "right top"
}, { immediate: true })

// select logic
const raycaster = new THREE.Raycaster()
const mouse = new THREE.Vector2()
var previous_hover_material = null
export const current_hover = ref(null)
window.current_hover = current_hover
var previous_selected_material = null
export const current_selected = ref(null)
window.current_selected = current_selected
export const show_hover_effect = ref(true)
function is_user_data_valid(user_data) {
    if (user_data == null) return false
    const fusion_data = active_fusion_data.value
    const snapshot_idx = active_snapshot_idx.value
    const snapshot = fusion_data.snapshots[snapshot_idx][1]
    if (user_data.type == "vertex") {
        return user_data.vertex_index < snapshot.vertices.length && snapshot.vertices[user_data.vertex_index] != null
    }
    if (user_data.type == "edge") {
        return user_data.edge_index < snapshot.edges.length && snapshot.edges[user_data.edge_index] != null
    }
    return false
}
function set_material_with_user_data(user_data, material) {  // return the previous material
    if (user_data.type == "vertex") {
        let vertex_index = user_data.vertex_index
        let vertex_mesh = vertex_meshes[vertex_index]
        let previous_material = vertex_mesh.material
        vertex_mesh.material = material
        return previous_material
    }
    if (user_data.type == "edge") {
        let expanded_material = material
        if (!Array.isArray(material)) {
            expanded_material = [[material, material], [material, material], [material, material]]
        }
        let edge_index = user_data.edge_index
        let meshes_lists = [left_edge_meshes, right_edge_meshes, middle_edge_meshes]
        let previous_material = [[null, null], [null, null], [null, null]]
        for (let i = 0; i < meshes_lists.length; ++i) {
            let meshes_list = meshes_lists[i][edge_index]
            for (let j of [0, 1]) {
                let edge_mesh = meshes_list[j]
                previous_material[i][j] = edge_mesh.material
                edge_mesh.material = expanded_material[i][j]
            }
        }
        return previous_material
    }
    console.error(`unknown type ${user_data.type}`)
}
watch(current_hover, (newVal, oldVal) => {
    // console.log(`${oldVal} -> ${newVal}`)
    if (oldVal != null && previous_hover_material != null) {
        set_material_with_user_data(oldVal, previous_hover_material)
        previous_hover_material = null
    }
    if (newVal != null) {
        previous_hover_material = set_material_with_user_data(newVal, hover_material)
    }
})
watch(current_selected, (newVal, oldVal) => {
    if (newVal != null) {
        current_hover.value = null
    }
    Vue.nextTick(() => {  // wait after hover cleaned its data
        if (oldVal != null && previous_selected_material != null) {
            set_material_with_user_data(oldVal, previous_selected_material)
            previous_selected_material = null
        }
        if (newVal != null) {
            previous_selected_material = set_material_with_user_data(newVal, selected_material)
        }
    })
})
function on_mouse_change(event, is_click) {
    mouse.x = (event.clientX / sizes.canvas_width) * 2 - 1
    mouse.y = - (event.clientY / sizes.canvas_height) * 2 + 1
    raycaster.setFromCamera(mouse, camera.value)
    const intersects = raycaster.intersectObjects(scene.children, false)
    for (let intersect of intersects) {
        if (!intersect.object.visible) continue  // don't select invisible object
        let user_data = intersect.object.userData
        if (user_data.type == null) continue  // doesn't contain enough information
        // swap back to the original material
        if (is_click) {
            current_selected.value = user_data
        } else {
            if (show_hover_effect.value) {
                current_hover.value = user_data
            } else {
                current_hover.value = null
            }
        }
        return
    }
    if (is_click) {
        current_selected.value = null
    } else {
        current_hover.value = null
    }
    return
}
var mousedown_position = null
var is_mouse_currently_down = false
window.addEventListener('mousedown', (event) => {
    if (event.clientX > sizes.canvas_width) return  // don't care events on control panel
    mousedown_position = {
        clientX: event.clientX,
        clientY: event.clientY,
    }
    is_mouse_currently_down = true
})
window.addEventListener('mouseup', (event) => {
    if (event.clientX > sizes.canvas_width) return  // don't care events on control panel
    // to prevent triggering select while moving camera
    if (mousedown_position != null && mousedown_position.clientX == event.clientX && mousedown_position.clientY == event.clientY) {
        on_mouse_change(event, true)
    }
    is_mouse_currently_down = false
})
window.addEventListener('mousemove', (event) => {
    if (event.clientX > sizes.canvas_width) return  // don't care events on control panel
    // to prevent triggering hover while moving camera
    if (!is_mouse_currently_down) {
        on_mouse_change(event, false)
    }
})

// export current scene to high-resolution png, useful when generating figures for publication
// (I tried svg renderer but it doesn't work very well... shaders are poorly supported)
export function render_png(scale = 1) {
    const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: true, preserveDrawingBuffer: true, context: webgl_renderer_context() })
    renderer.setSize(sizes.canvas_width * scale, sizes.canvas_height * scale, false)
    renderer.setPixelRatio(window.devicePixelRatio * scale)
    renderer.render(scene, camera.value)
    return renderer.domElement.toDataURL()
}
window.render_png = render_png
export function open_png(data_url) {
    const w = window.open('', '')
    w.document.title = "rendered image"
    w.document.body.style.backgroundColor = "white"
    w.document.body.style.margin = "0"
    const img = new Image()
    img.src = data_url
    img.style = "width: 100%; height: 100%; object-fit: contain;"
    w.document.body.appendChild(img)
}
window.open_png = open_png
export function download_png(data_url) {
    const a = document.createElement('a')
    a.href = data_url.replace("image/png", "image/octet-stream")
    a.download = 'rendered.png'
    a.click()
}
window.download_png = download_png

export async function nodejs_render_png() {  // works only in nodejs
    let context = webgl_renderer_context()
    var pixels = new Uint8Array(context.drawingBufferWidth * context.drawingBufferHeight * 4)
    const renderer = new THREE.WebGLRenderer({ alpha: true, antialias: false, preserveDrawingBuffer: true, context })
    renderer.setSize(sizes.canvas_width, sizes.canvas_height, false)
    renderer.setPixelRatio(window.devicePixelRatio)
    renderer.render(scene, camera.value)
    context.readPixels(0, 0, context.drawingBufferWidth, context.drawingBufferHeight, context.RGBA, context.UNSIGNED_BYTE, pixels)
    return pixels
}

// wait several Vue ticks to make sure all changes have been applied
export async function wait_changes() {
    for (let i = 0; i < 5; ++i) await Vue.nextTick()
}

// https://www.npmjs.com/package/base64-arraybuffer
var chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/'
export function base64_encode(arraybuffer) {
    var bytes = new Uint8Array(arraybuffer), i, len = bytes.length, base64 = ''
    for (i = 0; i < len; i += 3) {
        base64 += chars[bytes[i] >> 2]
        base64 += chars[((bytes[i] & 3) << 4) | (bytes[i + 1] >> 4)]
        base64 += chars[((bytes[i + 1] & 15) << 2) | (bytes[i + 2] >> 6)]
        base64 += chars[bytes[i + 2] & 63]
    }
    if (len % 3 === 2) {
        base64 = base64.substring(0, base64.length - 1) + '='
    }
    else if (len % 3 === 1) {
        base64 = base64.substring(0, base64.length - 2) + '=='
    }
    return base64;
}
