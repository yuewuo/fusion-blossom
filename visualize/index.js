import * as THREE from 'three'
import { OrbitControls } from 'OrbitControls'
import * as Stats from 'Stats'
const { ref, reactive } = Vue

const root = document.documentElement
var control_bar_width
var canvas_width
var canvas_height
var scale
var scale_ref = ref(0)
var default_size = ref("md")
function compute_size() {
    if (window.innerWidth >= 3840) {
        root.style.setProperty('--s', 2)
        default_size.value = "xl"
    } else if (window.innerWidth >= 2880) {
        root.style.setProperty('--s', 1.5)
        default_size.value = "lg"
    } else if (window.innerWidth >= 1600) {
        root.style.setProperty('--s', 1)
        default_size.value = "md"
    } else if (window.innerWidth >= 1200) {
        root.style.setProperty('--s', 0.75)
        default_size.value = "sm"
    } else {
        root.style.setProperty('--s', 0.6)
        default_size.value = "xs"
    }
    scale = parseFloat(getComputedStyle(document.documentElement).getPropertyValue('--s'))
    scale_ref.value = scale
    control_bar_width = 600 * scale
    canvas_width = window.innerWidth - control_bar_width
    canvas_height = window.innerHeight
}
compute_size()

const scene = new THREE.Scene()
const perspective_camera = new THREE.PerspectiveCamera( 75, canvas_width / canvas_height, 0.1, 10000 )
const orthogonal_camera_init_scale = 8
const orthogonal_camera = new THREE.OrthographicCamera( canvas_width / canvas_height * (-orthogonal_camera_init_scale), canvas_width / canvas_height * orthogonal_camera_init_scale, orthogonal_camera_init_scale, -orthogonal_camera_init_scale, 0.1, 10000 )
const renderer = new THREE.WebGLRenderer({ alpha: true })
function on_resize() {
    compute_size()
    perspective_camera.aspect = canvas_width / canvas_height
    perspective_camera.updateProjectionMatrix()
    orthogonal_camera.aspect = canvas_width / canvas_height
    orthogonal_camera.updateProjectionMatrix()
    renderer.setSize( canvas_width, canvas_height, false )
    const ratio = window.devicePixelRatio  // looks better on devices with a high pixel ratio, such as iPhones with Retina displays
    renderer.setPixelRatio( ratio )
    const canvas = renderer.domElement
    canvas.width = canvas_width * ratio
    canvas.height = canvas_height * ratio
    canvas.style.width = `${canvas_width}px`
    canvas.style.height = `${canvas_height}px`
}
on_resize()
document.body.appendChild( renderer.domElement )
window.addEventListener('resize', on_resize)
const orbit_control_perspective = new OrbitControls( perspective_camera, renderer.domElement )
const orbit_control_orthogonal = new OrbitControls( orthogonal_camera, renderer.domElement )

var three = {
    camera: orthogonal_camera,
    orbit_control: orbit_control_orthogonal,
}

function reset_camera_position(direction="top") {
    for (let [camera, control, distance] of [[perspective_camera, orbit_control_perspective, 10], [orthogonal_camera, orbit_control_orthogonal, 1000]]) {
        control.reset()
        camera.position.x = (direction == "left" ? -distance : 0)
        camera.position.y = (direction == "top" ? distance : 0)
        camera.position.z = (direction == "front" ? distance : 0)
    }
}
reset_camera_position()

scene.add( new THREE.AmbientLight( 0xffffff ) )

// const axesHelper = new THREE.AxesHelper( 5 )
// scene.add( axesHelper )

const stats = Stats.default()
document.body.appendChild(stats.dom)

function animate() {
    requestAnimationFrame( animate )
    three.orbit_control.update()
    renderer.render( scene, three.camera )
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

// fetch fusion blossom runtime data
const urlParams = new URLSearchParams(window.location.search)
const filename = urlParams.get('filename') || "default.json"
var fusion_data

// meshes that can be reused across different snapshots
var node_meshes = []
window.node_meshes = node_meshes
var edge_meshes = []
window.edge_meshes = edge_meshes

function compute_vector3(data_position) {
    let vector = new THREE.Vector3( 0, 0, 0 )
    load_position(vector, data_position)
    return vector
}
function load_position(mesh_position, data_position) {
    mesh_position.z = data_position.i
    mesh_position.x = data_position.j
    mesh_position.y = data_position.t
}

// create vue3 app
const App = {
    setup() {
        return {
            error_message: ref(null),
            snapshot_num: ref(1),
            snapshot_select: ref(0),
            snapshot_select_label: ref(""),
            snapshot_labels: reactive([]),
            use_perspective_camera: ref(false),
            scale: scale_ref,
            size: default_size,
        }
    },
    async mounted() {
        console.log(this.size)
        try {
            let response = await fetch('./data/' + filename, { cache: 'no-cache', })
            fusion_data = await response.json()
            console.log(fusion_data)
        } catch (e) {
            this.error_message = "fetch file error"
            throw e
        }
        this.show_snapshot(fusion_data.snapshots[0][1])  // load the first snapshot
        this.snapshot_num = fusion_data.snapshots.length
        for (let [idx, [name, _]] of fusion_data.snapshots.entries()) {
            this.snapshot_labels.push(`[${idx}] ${name}`)
        }
        this.snapshot_select_label = this.snapshot_labels[0]
        // only if data loads successfully will the animation starts
        animate()
        // add keyboard shortcuts
        document.onkeydown = (event) => {
            if (!event.metaKey) {
                if (event.key == "t" || event.key == "T") {
                    this.reset_camera("top")
                } else if (event.key == "l" || event.key == "L") {
                    this.reset_camera("left")
                } else if (event.key == "f" || event.key == "F") {
                    this.reset_camera("front")
                } else if (event.key == "ArrowRight") {
                    if (this.snapshot_select < this.snapshot_num - 1) {
                        this.snapshot_select += 1
                    }
                } else if (event.key == "ArrowLeft") {
                    if (this.snapshot_select > 0) {
                        this.snapshot_select -= 1
                    }
                } else {
                    return  // unrecognized, propagate to other listeners
                }
                event.preventDefault()
                event.stopPropagation()
            }
        }
    },
    methods: {
        show_snapshot(snapshot) {
            try {
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
            } catch (e) {
                this.error_message = "load data error"
                throw e
            }
        },
        reset_camera(direction) {
            reset_camera_position(direction)
        },
    },
    watch: {
        snapshot_select() {
            // console.log(this.snapshot_select)
            this.show_snapshot(fusion_data.snapshots[this.snapshot_select][1])  // load the snapshot
            this.snapshot_select_label = this.snapshot_labels[this.snapshot_select]
        },
        snapshot_select_label() {
            this.snapshot_select = parseInt(this.snapshot_select_label.split(']')[0].split('[')[1])
        },
        use_perspective_camera() {
            if (this.use_perspective_camera) {
                three.camera = perspective_camera
                three.orbit_control = orbit_control_perspective
            } else {
                three.camera = orthogonal_camera
                three.orbit_control = orbit_control_orthogonal
            }
        },
    },
    computed: {
        vertical_thumb_style() {
            return {
                right: `${4*scale_ref.value}px`,
                borderRadius: `${5*scale_ref.value}px`,
                backgroundColor: '#027be3',
                width: `${5*scale_ref.value}px`,
                opacity: 0.75
            }
        },
        horizontal_thumb_style() {
            return {
                bottom: `${4*scale_ref.value}px`,
                borderRadius: `${5*scale_ref.value}px`,
                backgroundColor: '#027be3',
                height: `${5*scale_ref.value}px`,
                opacity: 0.75
            }
        },
        vertical_bar_style() {
            return {
                right: `${2*scale_ref.value}px`,
                borderRadius: `${9*scale_ref.value}px`,
                backgroundColor: '#027be3',
                width: `${9*scale_ref.value}px`,
                opacity: 0.2
            }
        },
        horizontal_bar_style() {
            return {
                bottom: `${2*scale_ref.value}px`,
                borderRadius: `${9*scale_ref.value}px`,
                backgroundColor: '#027be3',
                height: `${9*scale_ref.value}px`,
                opacity: 0.2
            }
        },
    },
}
const app = Vue.createApp(App)
app.use(Quasar)
Quasar.Screen.setSizes({ sm: 1200, md: 1600, lg: 2880, xl: 3840 })
app.mount("#app")
window.app = app
