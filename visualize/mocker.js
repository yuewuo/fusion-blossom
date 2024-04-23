// prepare mock browser environment
import gl from 'gl'
import 'canvas'
import jsdom from 'jsdom'
import fs from 'fs'
import Jimp from 'jimp'


/*
Examples:

node index.js
node index.js 'http://localhost:8066/?filename=visualize_paper_weighted_union_find_decoder.json' 1024 1024
node index.js 'http://localhost:8066/?filename=visualize_paper_weighted_union_find_decoder.json&patch=visualize_paper_weighted_union_find_decoder' 1024 1024
node index.js 'http://localhost:8066/?filename=primal_module_serial_basic_4.json' 1024 1024
node index.js 'http://localhost:8066/?filename=primal_module_serial_basic_4.json&snapshot_idx=16' 1024 1024
node index.js 'http://localhost:8066/?filename=visualize_rough_idea_fusion_blossom.json&patch=visualize_rough_idea_fusion_blossom&snapshot_idx=2' 1024 1024

 */

// read link from command line
const parameters = process.argv.slice(2)
let link = "http://localhost:8066?filename=visualizer.json"
if (parameters.length >= 1) {
    link = parameters[0]
}
// by default canvas size is 1024 * 1024, can be changed using second and third parameter
export var mock_canvas_width = 1024
export var mock_canvas_height = 1024
if (parameters.length >= 2) {
    mock_canvas_width = parseInt(parameters[1])
}
if (parameters.length >= 3) {
    mock_canvas_height = parseInt(parameters[2])
}
export var mocker_default_filename = "rendered"
if (parameters.length >= 4) {
    mocker_default_filename = parameters[3]
}
console.log(`[render] ${link}`)

// DO NOT merge this file with headless-server, because nodejs resolves dependency (window, global must be set before running gui3d.js)

export var dom = new jsdom.JSDOM('<!DOCTYPE html><html><body><div id="app"></div></body></html>', {
    url: link,
    resources: 'usable',
    includeNodeLocations: true,
})
export var window = dom.window
export var document = window.document
global.window = window
global.document = document
global.navigator = {
    userAgent: "Mozilla/5.0 (Linux; Android 6.0; Nexus 5 Build/MRA58N) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.4896.127 Mobile Safari/537.36"
}

global.mockgl = () => gl(mock_canvas_width, mock_canvas_height, { preserveDrawingBuffer: true })


// read file directly given path
global.fetch = async (filepath) => {
    return {
        async json() {
            return JSON.parse(await fs.promises.readFile(filepath, 'utf-8'))
        },
        async arrayBuffer() {
            const buffer = await fs.promises.readFile(filepath)
            return buffer
        },
    }
}

export function save_data_uri(data_uri, filename) {
    var regex = /^data:.+\/(.+);base64,(.*)$/
    var matches = data_uri.match(regex)
    var ext = matches[1]
    var data = matches[2]
    var buffer = Buffer.from(data, 'base64')
    fs.writeFileSync(filename + '.' + ext, buffer)
}

export async function save_pixels(pixels, filename = null) {
    if (filename == null) {
        filename = mocker_default_filename
    }
    let img = await new Jimp(mock_canvas_width, mock_canvas_height)
    for (let j = 0; j < mock_canvas_height; ++j) {
        for (let i = 0; i < mock_canvas_width; ++i) {
            let b = (j * mock_canvas_width + i) * 4
            img.setPixelColor(Jimp.rgbaToInt(pixels[b + 0], pixels[b + 1], pixels[b + 2], pixels[b + 3]), i, mock_canvas_height - 1 - j)
        }
    }
    img.write(`${filename}.png`)
}

export async function read_from_png_buffer(buffer) {
    let img = await Jimp.read(buffer)
    return img
}

function sleep(ms) {
    return new Promise((resolve) => {
        setTimeout(resolve, ms)
    })
}
global.sleep = sleep

function createCanvas() {
    let c = new canvas.Canvas()
    c.addEventListener = (type, handler) => {
        c['on' + type] = handler.bind(c)
    }
    c.removeEventListener = (type) => {
        c['on' + type] = null
    }
    return c
}
global.createCanvas = createCanvas

global.HTMLImageElement = window.HTMLImageElement
