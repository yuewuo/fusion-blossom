import * as gui3d from './gui3d.js'
import * as patches from './patches.js'
const { ref, watch } = Vue

// fetch fusion blossom runtime data
const urlParams = new URLSearchParams(window.location.search)
const filename = urlParams.get('filename') || "default.json"

var fusion_data

// create vue3 app
const App = {
    setup() {
        return {
            error_message: ref(null),
            warning_message: ref(null),
            snapshot_num: ref(1),
            snapshot_select: ref(0),
            snapshot_select_label: ref(1),
            snapshot_labels: ref([]),
            use_perspective_camera: gui3d.use_perspective_camera,
            sizes: gui3d.sizes,
            export_scale_selected: ref(1),
            export_resolution_options: ref([]),
            // GUI related states
            show_stats: gui3d.show_stats,
            show_config: gui3d.show_config,
            show_hover_effect: gui3d.show_hover_effect,
            lock_view: ref(false),
            // select
            current_selected: gui3d.current_selected,
            selected_node_neighbor_edges: ref([]),
            selected_node_attributes: ref(""),
            selected_edge: ref(null),
        }
    },
    async mounted() {
        gui3d.root.style.setProperty('--control-visibility', 'visible')
        try {
            let response = await fetch('./data/' + filename, { cache: 'no-cache', })
            fusion_data = await response.json()
            console.log(fusion_data)
        } catch (e) {
            this.error_message = "fetch file error"
            throw e
        }
        this.show_snapshot(0)  // load the first snapshot
        this.snapshot_num = fusion_data.snapshots.length
        for (let [idx, [name, _]] of fusion_data.snapshots.entries()) {
            this.snapshot_labels.push(`[${idx}] ${name}`)
        }
        this.snapshot_select_label = this.snapshot_labels[0]
        // only if data loads successfully will the animation starts
        gui3d.animate()
        // add keyboard shortcuts
        document.onkeydown = (event) => {
            if (!event.metaKey) {
                if (event.key == "t" || event.key == "T") {
                    this.reset_camera("top")
                } else if (event.key == "l" || event.key == "L") {
                    this.reset_camera("left")
                } else if (event.key == "f" || event.key == "F") {
                    this.reset_camera("front")
                } else if (event.key == "c" || event.key == "C") {
                    this.show_config = !this.show_config
                } else if (event.key == "s" || event.key == "S") {
                    this.show_stats = !this.show_stats
                } else if (event.key == "v" || event.key == "V") {
                    this.lock_view = !this.lock_view
                } else if (event.key == "o" || event.key == "O") {
                    this.use_perspective_camera = false
                } else if (event.key == "p" || event.key == "P") {
                    this.use_perspective_camera = true
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
        // get command from url parameters
        Vue.nextTick(() => {
            let snapshot_idx = urlParams.get('si') || urlParams.get('snapshot_idx')
            if (snapshot_idx != null) {
                snapshot_idx = parseInt(snapshot_idx)
                if (snapshot_idx < 0) {  // iterate from the end, like python list[-1]
                    snapshot_idx = this.snapshot_num + snapshot_idx
                    if (snapshot_idx < 0) {  // too small
                        snapshot_idx = 0
                    }
                }
                if (snapshot_idx >= this.snapshot_num) {
                    snapshot_idx = this.snapshot_num - 1
                }
                this.snapshot_select = snapshot_idx
            }
        })
        // update resolution options when sizes changed
        watch(gui3d.sizes, this.update_export_resolutions, { immediate: true })
        // execute patch scripts
        setTimeout(() => {
            const patch_name = urlParams.get('patch')
            if (patch_name != null) {
                console.log(`running patch ${patch_name}`)
                const patch_function = patches[patch_name]
                patch_function.bind(this)()
            }
        }, 100);
    },
    methods: {
        show_snapshot(snapshot_idx) {
            try {
                gui3d.show_snapshot(snapshot_idx, fusion_data)
            } catch (e) {
                this.error_message = "load data error"
                throw e
            }
        },
        reset_camera(direction) {
            gui3d.reset_camera_position(direction)
        },
        update_selected_display() {
            if (this.current_selected == null) return
            if (this.current_selected.type == "node") {
                let node_index = this.current_selected.node_index
                let node = this.snapshot.nodes[node_index]
                this.selected_node_attributes = ""
                if (node.s == 1) {
                    this.selected_node_attributes = "(syndrome)"
                } else if (node.v == 1) {
                    this.selected_node_attributes = "(virtual)"
                }
                console.assert(!(node.s == 1 && node.v == 1), "a node cannot be both syndrome and virtual")
                // fetch edge list
                let neighbor_edges = []
                for (let [edge_index, edge] of this.snapshot.edges.entries()) {
                    if (edge.l == node_index) {
                        const [translated_left_grown, translated_right_grown] = gui3d.translate_edge(edge.lg, edge.rg, edge.w)
                        const translated_unexplored = edge.w - translated_left_grown - translated_right_grown
                        neighbor_edges.push({
                            edge_index: edge_index,
                            left_grown: edge.lg,
                            unexplored: edge.w - edge.lg - edge.rg,
                            right_grown: edge.rg,
                            weight: edge.w,
                            node_index: edge.r,
                            translated_left_grown,
                            translated_right_grown,
                            translated_unexplored,
                        })
                    } else if (edge.r == node_index) {
                        const [translated_left_grown, translated_right_grown] = gui3d.translate_edge(edge.rg, edge.lg, edge.w)
                        const translated_unexplored = edge.w - translated_left_grown - translated_right_grown
                        neighbor_edges.push({
                            edge_index: edge_index,
                            left_grown: edge.rg,
                            unexplored: edge.w - edge.lg - edge.rg,
                            right_grown: edge.lg,
                            weight: edge.w,
                            node_index: edge.l,
                            translated_left_grown,
                            translated_right_grown,
                            translated_unexplored,
                        })
                    }
                }
                this.selected_node_neighbor_edges = neighbor_edges
            }
            if (this.current_selected.type == "edge") {
                const edge_index = this.current_selected.edge_index
                const edge = this.snapshot.edges[edge_index]
                const [translated_left_grown, translated_right_grown] = gui3d.translate_edge(edge.lg, edge.rg, edge.w)
                const translated_unexplored = edge.w - translated_left_grown - translated_right_grown
                this.selected_edge = {
                    edge_index: edge_index,
                    left_grown: edge.lg,
                    unexplored: edge.w - edge.lg - edge.rg,
                    right_grown: edge.rg,
                    weight: edge.w,
                    left_node_index: edge.l,
                    right_node_index: edge.r,
                    translated_left_grown,
                    translated_right_grown,
                    translated_unexplored,
                }
            }
        },
        jump_to(type, data, is_click=true) {
            let current_ref = is_click ? gui3d.current_selected : gui3d.current_hover
            if (type == "edge") {
                current_ref.value = {
                    type, edge_index: data
                }
            }
            if (type == "node") {
                current_ref.value = {
                    type, node_index: data
                }
            }
        },
        mouseenter(type, data) {
            this.jump_to(type, data, false)
        },
        mouseleave() {
            gui3d.current_hover.value = null
        },
        update_export_resolutions() {
            this.export_resolution_options.splice(0, this.export_resolution_options.length)
            let exists_in_new_resolution = false
            for (let i=-100; i<100; ++i) {
                let scale = 1 * Math.pow(10, i/10)
                let width = Math.round(this.sizes.canvas_width * scale)
                let height = Math.round(this.sizes.canvas_height * scale)
                if (width > 5000 || height > 5000) {  // to large, likely exceeds WebGL maximum buffer size
                    break
                }
                if (width >= 300 || height >= 300) {
                    let label = `${width} x ${height}`
                    this.export_resolution_options.push({
                        label: label,
                        value: scale,
                    })
                    if (scale == this.export_scale_selected) {
                        exists_in_new_resolution = true
                    }
                }
            }
            if (!exists_in_new_resolution) {
                this.export_scale_selected = null
            }
        },
        preview_image() {
            const data = gui3d.render_png(this.export_scale_selected)
            gui3d.open_png(data)
        },
        download_image() {
            const data = gui3d.render_png(this.export_scale_selected)
            gui3d.download_png(data)
        },
    },
    watch: {
        snapshot_select() {
            // console.log(this.snapshot_select)
            this.show_snapshot(this.snapshot_select)  // load the snapshot
            this.snapshot_select_label = this.snapshot_labels[this.snapshot_select]
            this.update_selected_display()
        },
        snapshot_select_label() {
            this.snapshot_select = parseInt(this.snapshot_select_label.split(']')[0].split('[')[1])
        },
        current_selected() {
            this.update_selected_display()
        },
        lock_view() {
            gui3d.enable_control.value = !this.lock_view
        },
    },
    computed: {
        scale() {
            return this.sizes.scale
        },
        vertical_thumb_style() {
            return {
                right: `4px`,
                borderRadius: `5px`,
                backgroundColor: '#027be3',
                width: `5px`,
                opacity: 0.75
            }
        },
        horizontal_thumb_style() {
            return {
                bottom: `4px`,
                borderRadius: `5px`,
                backgroundColor: '#027be3',
                height: `5px`,
                opacity: 0.75
            }
        },
        vertical_bar_style() {
            return {
                right: `2px`,
                borderRadius: `9px`,
                backgroundColor: '#027be3',
                width: `9px`,
                opacity: 0.2
            }
        },
        horizontal_bar_style() {
            return {
                bottom: `2px`,
                borderRadius: `9px`,
                backgroundColor: '#027be3',
                height: `9px`,
                opacity: 0.2
            }
        },
        snapshot() {
            return fusion_data.snapshots[this.snapshot_select][1]
        },
        is_browser_supported() {
            // console.log(navigator.userAgent)
            const is_chrome = navigator.userAgent.toLowerCase().indexOf('chrome/') > -1
            const is_firefox = navigator.userAgent.toLowerCase().indexOf('firefox/') > -1
            return is_chrome || is_firefox
        },
    },
}
const app = Vue.createApp(App)
app.use(Quasar)
Quasar.Screen.setSizes({ sm: 1200, md: 1600, lg: 2880, xl: 3840 })
app.mount("#app")
window.app = app
