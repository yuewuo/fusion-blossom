const { ref, reactive, watch, computed } = Vue
import * as gui3d from './gui3d.js'

var primal_div_control = {
    chart: null,
}

const match_width = 12
const unmatch_width = 3

export const show_primal = ref(false)
export async function show_snapshot(snapshot_idx, fusion_data) {
    if (gui3d.is_mock) {
        return // skip primal plot in mock mode
    }

    let chart = primal_div_control.chart
    console.assert(chart != null, "chart should not be null when calling `show_snapshot`")

    // if primal nodes or dual nodes are not present, we cannot show it
    const snapshot = fusion_data.snapshots[snapshot_idx][1]
    if (snapshot.dual_nodes == null || snapshot.primal_nodes == null) {
        // console.error("snapshot doesn't have dual and primal nodes, so primal module is disabled")
        show_primal.value = false
        return
    }
    show_primal.value = true

    function get_child_count(dual_node) {
        if (dual_node.s != null) { return 1 }  // syndrome vertex
        let count = 0
        for (const child_idx of dual_node.o) {
            const child_dual_node = snapshot.dual_nodes[child_idx]
            count += get_child_count(child_dual_node)
        }
        return count
    }

    const nodes_count = snapshot.primal_nodes.length
    let nodes = []
    let links = []
    for (let i = 0; i < nodes_count; ++i) {
        const primal_node = snapshot.primal_nodes[i]
        const dual_node = snapshot.dual_nodes[i]
        if (primal_node == null) { continue }  // expanded blossom
        const id = `${i}`
        let display_node = dual_node.p == null
        // however, if primal node explicitly says it has a matching (usually when visualize a perfect matching), then force it to be shown
        display_node |= snapshot.primal_nodes && i < snapshot.primal_nodes.length && snapshot.primal_nodes[i] && snapshot.primal_nodes[i].m != undefined
        display_node &= snapshot.primal_nodes == undefined || i >= snapshot.primal_nodes.length || snapshot.primal_nodes[i] != undefined
        if (!display_node) { continue }  // internal node of a blossom
        const child_count = get_child_count(dual_node)
        if (child_count % 2 == 0) {
            console.error("found even child count, invalid blossom")
        }
        nodes.push({
            id: id,
            radius: Math.pow(child_count, 0.3),
            stroke_color: "#fff"
        })
        if (primal_node.t != null) {
            const tree_node = primal_node.t
            if (tree_node.d % 2 == 1) {
                // shrinking node has black stroke color
                nodes[nodes.length - 1].stroke_color = "#888"
            }
            if (tree_node.p != null) {
                links.push({
                    source: id,
                    target: `${tree_node.p}`,
                    color: tree_node.d % 2 == 1 ? "#f00" : "#00f",
                    width: tree_node.d % 2 == 1 ? unmatch_width : match_width,
                })
            }
        }
        if (primal_node.m != null) {
            const match_target = primal_node.m
            if (match_target != null) {
                if (match_target.p != null) {  // matching to peer
                    if (i < match_target.p) {
                        links.push({
                            source: id,
                            target: `${match_target.p}`,
                            color: "#090",
                            width: match_width,
                        })
                    }
                }
                if (match_target.v != null) {  // matching to virtual vertex
                    nodes.push({
                        id: `v${match_target.v}`,
                        radius: 1.2,
                        stroke_color: "#ff0"  // virtual node has yellow stroke color
                    })
                    links.push({
                        source: id,
                        target: `v${match_target.v}`,
                        color: "#090",
                        width: match_width,
                    })
                }
            }
        }
    }

    let graph = {
        nodes: nodes,
        links: links,
    }

    chart.update(graph)

}


// it took me an hour to realize this has to be initialized AFTER all vue things have been updated;
// otherwise it won't be able to dynamically update the variables
export function initialize_primal_div() {
    if (gui3d.is_mock) {
        return // skip primal plot in mock mode
    }

    const d3 = window.d3

    const width = 580
    const height = width

    const color = d3.scaleOrdinal(d3.schemeTableau10)

    const svg = d3.create("svg")
        .attr("width", width)
        .attr("height", height)
        .attr("viewBox", [-width / 2, -height / 2, width, height]);

    let link = svg.append("g")
        .attr("stroke", "#000")
        .attr("stroke-width", 3)
        .selectAll("line");

    let node = svg.append("g")
        // .attr("stroke", "#fff")
        .attr("stroke-width", 2)
        .selectAll("circle");

    let text = svg.append("g")
        .attr("stroke-width", 0)
        .selectAll("text");

    function ticked() {
        node.attr("cx", d => d.x)
            .attr("cy", d => d.y);

        text.attr("x", d => d.x)
            .attr("y", d => d.y);

        link.attr("x1", d => d.source.x)
            .attr("y1", d => d.source.y)
            .attr("x2", d => d.target.x)
            .attr("y2", d => d.target.y);
    }

    const simulation = d3.forceSimulation()
        .force("charge", d3.forceManyBody().strength(-120))
        .force("link", d3.forceLink().id(d => d.id).distance(60))
        .force("x", d3.forceX())
        .force("y", d3.forceY())
        .on("tick", ticked);

    const drag = simulation => {

        function dragstarted(event, d) {
            if (!event.active) simulation.alphaTarget(1).restart();
            d.fx = d.x;
            d.fy = d.y;
        }

        function dragged(event, d) {
            d.fx = event.x;
            d.fy = event.y;
        }

        function dragended(event, d) {
            if (!event.active) simulation.alphaTarget(0);
            d.fx = null;
            d.fy = null;
        }

        return d3.drag()
            .on("start", dragstarted)
            .on("drag", dragged)
            .on("end", dragended);
    }

    function node_on_click(d) {
        const data = d.path[0].__data__
        const id = data.id
        console.log(id)
    }

    let chart = Object.assign(svg.node(), {
        update({ nodes, links }) {
            // Make a shallow copy to protect against mutation, while
            // recycling old nodes to preserve position and velocity.
            const old = new Map(node.data().map(d => [d.id, d]));
            nodes = nodes.map(d => Object.assign(old.get(d.id) || {}, d));
            links = links.map(d => Object.assign({}, d));

            simulation.nodes(nodes);
            simulation.force("link").links(links);
            simulation.alpha(1).restart();

            node = node
                .data(nodes, d => d.id)
                .join(enter => enter.append("circle").call(drag(simulation)))
                .attr("fill", d => color(d.id))
                .attr("r", d => d.radius * 15)
                .attr("stroke", d => d.stroke_color)

            node.on("click", node_on_click)

            text = text
                .data(nodes, d => d.id)
                .join(enter => enter.append("text")
                    .attr("style", "color: #f0f; font-size: 18px;")
                    .attr("text-anchor", "middle")
                    .attr("alignment-baseline", "middle")
                    .attr("dy", "1px")
                    .call(drag(simulation))
                    .text(d => d.id));

            text.on("click", node_on_click)

            link = link
                .data(links, d => `${d.source.id}\t${d.target.id}`)
                .join(enter => enter.append("line"))
                .attr("stroke", d => d.color)
                .attr("stroke-width", d => d.width);
        }
    });

    d3.select('#primal-div').append(() => chart)

    primal_div_control.chart = chart

}

window.initialize_primal_div = initialize_primal_div
