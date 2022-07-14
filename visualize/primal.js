const d3 = window.d3

var primal_div_control = {
    chart: null,
}

export function show_snapshot(snapshot_idx, fusion_data) {
    let chart = primal_div_control.chart
    console.assert(chart != null, "chart should not be null when calling `show_snapshot`")

    let graph_example = {
        nodes: [
            {id: "a"},
            {id: "b"},
            {id: "c"},
            {id: "0"},
            {id: "1"},
            {id: "2"},
            {id: "a2"},
            {id: "b2"},
            {id: "c2"},
            {id: "a3"},
            {id: "b3"},
            {id: "c3"},
            {id: "a4"},
            {id: "b4"},
            {id: "c4"},
            {id: "a5"},
            {id: "b5"},
            {id: "c5"},
            {id: "a6"},
            {id: "b6"},
            {id: "c6"},
            {id: "a7"},
            {id: "b7"},
            {id: "c7"},
            {id: "a8"},
            {id: "b8"},
            {id: "c8"},
            {id: "a9"},
            {id: "b9"},
            {id: "c9"},
            {id: "aa"},
            {id: "ba"},
            {id: "ca"},
            {id: "ab"},
            {id: "bb"},
            {id: "cb"},
            {id: "ac"},
            {id: "bc"},
            {id: "cc"},
            {id: "ad"},
            {id: "bd"},
            {id: "cd"},
            {id: "ae"},
            {id: "be"},
            {id: "ce"},
            {id: "af"},
            {id: "bf"},
            {id: "cf"},
            {id: "ag"},
            {id: "bg"},
            {id: "cg"},
            {id: "ah"},
            {id: "bh"},
            {id: "ch"},
            {id: "ai"},
            {id: "bi"},
            {id: "ci"},
            {id: "aj"},
            {id: "bj"},
            {id: "cj"},
        ],
        links: [
            {source: "a", target: "b"},
            {source: "b", target: "c"},
            {source: "c", target: "a"}
        ],
    }
    
    chart.update(graph_example)

}


// it took me an hour to realize this has to be initialized AFTER all vue things have been updated;
// otherwise it won't be able to dynamically update the variables
export function initialize_primal_div() {

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
        .attr("stroke", "#fff")
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
        .force("charge", d3.forceManyBody().strength(-80))
        .force("link", d3.forceLink().id(d => d.id).distance(50))
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
        update({nodes, links}) {
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
                .join(enter => enter.append("circle")
                    .attr("r", 15)
                    .call(drag(simulation))
                    .attr("fill", d => color(d.id)));
                
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
                .join("line");
        }
    });

    d3.select('#primal-div').append(() => chart)

    primal_div_control.chart = chart

}

window.initialize_primal_div = initialize_primal_div
