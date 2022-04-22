// 3d related apis

import * as THREE from 'three'
import { OrbitControls } from 'OrbitControls'
import * as Stats from 'Stats'
import * as GUI from 'GUI'
const { ref, reactive, watch, computed } = Vue


export const root = document.documentElement

export const window_inner_width = ref(0)
export const window_inner_height = ref(0)
function on_resize() {
    window_inner_width.value = window.innerWidth
    window_inner_height.value = window.innerHeight
}
on_resize()
window.addEventListener('resize', on_resize)

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
    if (sizes.scale * window_inner_width.value < 300) {
        sizes.scale = 300 / window_inner_width.value
    }
    root.style.setProperty('--s', sizes.scale)
    // sizes.scale = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--s'))
    sizes.control_bar_width = 600 * sizes.scale
    sizes.canvas_width = window_inner_width.value - sizes.control_bar_width
    sizes.canvas_height = window_inner_height.value
}, { immediate: true })

export const scene = new THREE.Scene()
scene.add( new THREE.AmbientLight( 0xffffff ) )
export const perspective_camera = new THREE.PerspectiveCamera( 75, sizes.canvas_width / sizes.canvas_height, 0.1, 10000 )
const orthogonal_camera_init_scale = 8
export const orthogonal_camera = new THREE.OrthographicCamera( sizes.canvas_width / sizes.canvas_height * (-orthogonal_camera_init_scale)
    , sizes.canvas_width / sizes.canvas_height * orthogonal_camera_init_scale, orthogonal_camera_init_scale, -orthogonal_camera_init_scale, 0.1, 10000 )
export const renderer = new THREE.WebGLRenderer({ alpha: true })

document.body.appendChild( renderer.domElement )

watch(sizes, () => {
    perspective_camera.aspect = sizes.canvas_width / sizes.canvas_height
    perspective_camera.updateProjectionMatrix()
    orthogonal_camera.left = sizes.canvas_width / sizes.canvas_height * (-orthogonal_camera_init_scale)
    orthogonal_camera.right = sizes.canvas_width / sizes.canvas_height * (orthogonal_camera_init_scale)
    orthogonal_camera.updateProjectionMatrix()
    renderer.setSize( sizes.canvas_width, sizes.canvas_height, false )
    const ratio = window.devicePixelRatio  // looks better on devices with a high pixel ratio, such as iPhones with Retina displays
    renderer.setPixelRatio( ratio )
    const canvas = renderer.domElement
    canvas.width = sizes.canvas_width * ratio
    canvas.height = sizes.canvas_height * ratio
    canvas.style.width = `${sizes.canvas_width}px`
    canvas.style.height = `${sizes.canvas_height}px`
}, { immediate: true })

export const orbit_control_perspective = new OrbitControls( perspective_camera, renderer.domElement )
export const orbit_control_orthogonal = new OrbitControls( orthogonal_camera, renderer.domElement )

export const use_perspective_camera = ref(false)
export const camera = computed(() => {
    return use_perspective_camera.value ? perspective_camera : orthogonal_camera
})
export const orbit_control = computed(() => {
    return use_perspective_camera.value ? orbit_control_perspective : orbit_control_orthogonal
})

export function reset_camera_position(direction="top") {
    for (let [camera, control, distance] of [[perspective_camera, orbit_control_perspective, 10], [orthogonal_camera, orbit_control_orthogonal, 1000]]) {
        control.reset()
        camera.position.x = (direction == "left" ? -distance : 0)
        camera.position.y = (direction == "top" ? distance : 0)
        camera.position.z = (direction == "front" ? distance : 0)
    }
}
reset_camera_position()

// const axesHelper = new THREE.AxesHelper( 5 )
// scene.add( axesHelper )

const stats = Stats.default()
document.body.appendChild(stats.dom)
export const show_stats = ref(true)
watch(show_stats, function() {
    if (show_stats.value) {
        stats.dom.style.display = "block"
    } else {
        stats.dom.style.display = "none"
    }
}, { immediate: true })

export const show_config = ref(false)

export function animate() {
    requestAnimationFrame( animate )
    orbit_control.value.update()
    renderer.render( scene, camera.value )
    stats.update()
}

// commonly used vectors
const zero_vector = new THREE.Vector3( 0, 0, 0 )
const unit_up_vector = new THREE.Vector3( 0, 1, 0 )

// create common geometries
const segment = 32  // higher segment will consume more GPU resources
const node_radius = 0.15
const node_geometry = new THREE.SphereGeometry( node_radius, segment, segment )
const edge_radius = 0.03
const edge_geometry = new THREE.CylinderGeometry( edge_radius, edge_radius, 1, segment )
edge_geometry.translate(0, 0.5, 0)

// create common materials
const syndrome_node_material = new THREE.MeshStandardMaterial({
    color: 0xff0000,
    opacity: 1,
    transparent: true
})
const real_node_material = new THREE.MeshStandardMaterial({
    color: 0x000000,
    opacity: 0.03,
    transparent: true
})
const virtual_node_material = new THREE.MeshStandardMaterial({
    color: 0xffff00,
    opacity: 0.5,
    transparent: true
})
const edge_material = new THREE.MeshStandardMaterial({
    color: 0x0000ff,
    opacity: 0.1,
    transparent: true
})

// meshes that can be reused across different snapshots
export var node_meshes = []
window.node_meshes = node_meshes
export var edge_meshes = []
window.edge_meshes = edge_meshes

export function compute_vector3(data_position) {
    let vector = new THREE.Vector3( 0, 0, 0 )
    load_position(vector, data_position)
    return vector
}
export function load_position(mesh_position, data_position) {
    mesh_position.z = data_position.i
    mesh_position.x = data_position.j
    mesh_position.y = data_position.t
}

export function show_snapshot(snapshot, fusion_data) {
    for (let [i, node] of snapshot.nodes.entries()) {
        let position = fusion_data.positions[i]
        if (node_meshes.length <= i) {
            const node_mesh = new THREE.Mesh( node_geometry, syndrome_node_material )
            scene.add( node_mesh )
            load_position(node_mesh.position, position)
            node_meshes.push(node_mesh)
        }
        const node_mesh = node_meshes[i]
        if (node.s) {
            node_mesh.material = syndrome_node_material
        } else if (node.v) {
            node_mesh.material = virtual_node_material
        } else {
            node_mesh.material = real_node_material
        }
        node_mesh.visible = true
    }
    for (let i = snapshot.nodes.length; i < node_meshes.length; ++i) {
        node_meshes[i].visible = false
    }
    for (let [i, edge] of snapshot.edges.entries()) {
        const left_position = fusion_data.positions[edge.l]
        const right_position = fusion_data.positions[edge.r]
        if (edge_meshes.length <= i) {
            const edge_mesh = new THREE.Mesh( edge_geometry, edge_material )
            scene.add( edge_mesh )
            load_position(edge_mesh.position, left_position)
            const direction = compute_vector3(right_position).add(compute_vector3(left_position).multiplyScalar(-1))
            const edge_length = direction.length()
            // console.log(direction)
            const quaternion = new THREE.Quaternion()
            quaternion.setFromUnitVectors(unit_up_vector, direction.normalize())
            edge_mesh.scale.set(1, edge_length, 1)
            edge_mesh.applyQuaternion(quaternion)
            edge_meshes.push(edge_mesh)
        }
    }
    for (let i = snapshot.edges.length; i < edge_meshes.length; ++i) {
        edge_meshes[i].visible = false
    }
}


// configurations



