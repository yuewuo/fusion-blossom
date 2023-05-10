import { py, python } from 'pythonia'
import util from 'util'

const fb = await python('fusion_blossom')

export class Position {
    constructor(i, j) {
        this.i = i
        this.j = j
    }
    toString() {
        return `Pos(${this.i},${this.j})`
    }
    [util.inspect.custom]() {  // Nodejs pretty print
        return this.toString()
    }
}

export class CodeSimulator {

    constructor(d, solver, layouts, code) {
        this.code = code
        this.d = d
        this.solver = solver
        this.layouts = layouts
        this.errors = {}
        this.syndrome = {}
        this.correction = {}
    }

    static async create(d) {
        let code = await fb.YqiArtCode(d, 0.1)
        let initializer = await code.get_initializer()
        let solver = await fb.SolverSerial(initializer)
        // also calculate layout
        let vertices = await code.vertices
        let vertex_num = await vertices.length
        let edges = await code.edges
        let edge_num = await edges.length
        let layouts = {
            vertex_num,
            edge_num,
            stabilizers_positions: [],  // VertexIndex -> Pos
            stabilizers_indices: {},  // Pos -> VertexIndex
            data_qubits_positions: [],  // EdgeIndex -> Pos
            data_qubits_x_error_index: {},  // Pos -> EdgeIndex
            data_qubits_z_error_index: {},  // Pos -> EdgeIndex
        }
        for (let i=0; i < vertex_num; ++i) {
            let vertex = await vertices[i]
            if (await vertex.is_virtual) {
                layouts.stabilizers_positions.push(null)
                continue
            }
            let position = new Position(await vertex.position.i, await vertex.position.j)
            layouts.stabilizers_indices[position] = i
            layouts.stabilizers_positions.push(position)
        }
        console.assert(edge_num == 2 * d * d, "each data qubit corresponds to two edges")
        for (let i=0; i < edge_num; ++i) {
            let edge = await edges[i]
            let [vi, vj] = [await edge.vertices[0], await edge.vertices[1]]
            let vertex1 = await vertices[vi]
            let vertex2 = await vertices[vj]
            let position = new Position((await vertex1.position.i + await vertex2.position.i - 1) / 2
                , (await vertex1.position.j + await vertex2.position.j - 1) / 2)
            layouts.data_qubits_positions.push(position)
            let is_Z_error = i >= edge_num / 2
            let set = is_Z_error ? layouts.data_qubits_z_error_index : layouts.data_qubits_x_error_index
            set[position] = i
        }
        return new CodeSimulator(d, solver, layouts, code)
    }

    clear() {
        this.errors = {}
        this.syndrome = {}
        this.correction = {}
    }

    // error_type is one of ["I", "X", "Y", "Z"]
    set_qubit_error(position, error_type) {
        this.errors[position] = error_type
    }

    get_qubit_error(position) {
        return this.errors[position]
    }

    has_stabilizer(position) {
        return this.layouts.stabilizers_indices[position] != null
    }

    is_nontrivial_measurement(position) {
        return this.syndrome[position] != null
    }

    is_Z_stabilizer(position) {
        return this.layouts.stabilizers_indices[position] < this.layouts.vertex_num / 2
    }

    // return the simulation to get stabilizer checks
    async simulate() {
        let edge_indices = []
        for (const [position, error] of Object.entries(this.errors)) {
            let has_x = false
            let has_z = false
            if (error == "I") { }
            else if (error == "X") { has_x = true }
            else if (error == "Y") { has_x = true; has_z = true }
            else if (error == "Z") { has_z = true }
            else { console.error(`unknown error type ${error} at ${position}`) }
            if (has_x) {
                edge_indices.push(this.layouts.data_qubits_x_error_index[position])
            }
            if (has_z) {
                edge_indices.push(this.layouts.data_qubits_z_error_index[position])
            }
        }
        let syndrome = await this.code.generate_errors(edge_indices)
        await this.code.clear_errors()
        this.syndrome = {}
        for (let i=0; i<await syndrome.defect_vertices.length; ++i) {
            let defect_vertex = await syndrome.defect_vertices[i]
            let position = this.layouts.stabilizers_positions[defect_vertex]
            this.syndrome[position] = true
        }
    }

    async decode() {
        let defect_vertices = []
        for (const [position, _] of Object.entries(this.syndrome)) {
            let vertex_index = this.layouts.stabilizers_indices[position]
            defect_vertices.push(vertex_index)
        }
        let syndrome = await fb.SyndromePattern(defect_vertices)
        await this.solver.solve(syndrome)
        let subgraph = await this.solver.subgraph()
        await this.solver.clear()
        this.subgraph = []
        for (let i=0; i<await subgraph.length; ++i) {
            this.subgraph.push(await subgraph[i])
        }
        // create correction pattern
        this.correction = {}
        for (let edge_index of this.subgraph) {
            
        }
    }

}


// a mock function that show the code in text
export function display(code) {
    console.log("mock display: (·) trivial stabilizer check, (XZ) nontrivial stabilizer check")
    for (let i=0; i<=code.d; ++i) {
        let row_string = ""
        for (let j=0; j<=code.d; ++j) {
            let pos = new Position(i, j)
            if (code.has_stabilizer(pos)) {
                if (code.is_nontrivial_measurement(pos)) {
                    if (code.is_Z_stabilizer(pos)) {
                        row_string += " Z"
                    } else {
                        row_string += " X"
                    }
                } else {
                    row_string += " ·"
                }
            } else {
                row_string += "  "
            }
        }
        console.log(row_string)
    }
}

// a mock function that show the animation of the decoding
export function animate_decoding(code) {
    console.log("mock animation: (·) trivial stabilizer check, (XZ) nontrivial stabilizer check")
    for (let i=0; i<=code.d; ++i) {
        let row_string = ""
        for (let j=0; j<=code.d; ++j) {
            let pos = new Position(i, j)
            if (code.has_stabilizer(pos)) {
                if (code.is_nontrivial_measurement(pos)) {
                    if (code.is_Z_stabilizer(pos)) {
                        row_string += " Z"
                    } else {
                        row_string += " X"
                    }
                } else {
                    row_string += " ·"
                }
            } else {
                row_string += "  "
            }
        }
        console.log(row_string)
    }
}


export function exit() {
    python.exit()
}
