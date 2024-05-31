from IPython.display import display, HTML

def display_test_matrix():
    # HTML template for displaying the parity matrix
    matrix_html = """
    <div id="parity_matrix">
        <!-- Include JavaScript code to render the parity matrix -->
        <script>
            // JavaScript code to render the parity matrix
        </script>
    </div>
    """
    display(HTML(matrix_html))

def display_test_hypergraph():
    # HTML template for displaying the decoding hypergraph
    hypergraph_html = """
    <div id="decoding_hypergraph">
        <!-- Include JavaScript code to render the decoding hypergraph -->
        <script>
            // JavaScript code to render the decoding hypergraph
        </script>
    </div>
    """
    display(HTML(hypergraph_html))

# Call the functions to display the visualization tools
display_test_matrix()
display_test_hypergraph()
