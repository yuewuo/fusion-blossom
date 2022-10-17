"""
Note: this file is compiled as part of the binary, recompile if you change this to take effect
"""

import webbrowser, threading, os, time
# import bottle  # embedded 0.13-dev version for WSGIRefServer support

fb = None

"""called by fusion_blossom package at import, to set up global variables"""
def register(module):
    global fb
    fb = module

"""
start a server to host the visualizer websites locally
"""
def serve(host='localhost', port=51665, data_folder=".", return_server=False, quiet=True):
    from bottle import WSGIRefServer, route, abort, run, response, static_file
    global visualizer_website
    def guess_mime(filename):
        if filename.endswith("html"):
            response.content_type = 'text/html; charset=utf8'
        elif filename.endswith("js"):
            response.content_type = 'text/javascript; charset=utf8'
        elif filename.endswith("json"):
            response.content_type = 'application/json; charset=utf8'
    @route('/data/<filename:path>')
    def send_data(filename):
        return static_file(filename, root=data_folder)
    @route('/')
    @route('/<filename:path>')
    def send_root(filename=""):
        if filename == "":
            filename = "index.html"
        if filename in visualizer_website:
            guess_mime(filename)
            return visualizer_website[filename]
        abort(404)
    server = WSGIRefServer(host=host, port=port)
    def run_server():
        run(server=server, quiet=quiet)
    if return_server:
        return server, run_server
    run_server()


"""
open the website directly after starting the browser
"""
def open_visualizer(filename, host='localhost', port=51666, data_folder=".", open_browser=True, quiet=True):
    server, run_server = serve(host, port, data_folder, return_server=True, quiet=quiet)
    threading.Thread(target=run_server).start()
    if open_browser:
        def open_browser():
            webbrowser.open(f"http://{host}:{port}/?filename={filename}")
        threading.Timer(0.5, open_browser).start()  # wait 500ms for the server to start
    print("Hit ENTER to exit server.")
    input()
    server.srv.shutdown()
    time.sleep(0.3)

"""
open the website to have a look at the code object
"""
def peek_code(code, host='localhost', port=51667, data_folder=".", open_browser=True, quiet=True):
    positions = code.get_positions()
    initializer = code.get_initializer()
    solver = fb.SolverSerial(initializer)
    solver.solve(fb.SyndromePattern())
    visualize_filename = fb.static_visualize_data_filename()
    visualizer = fb.Visualizer(filepath=os.path.join(data_folder, visualize_filename), positions=positions)
    solver.perfect_matching(visualizer)
    fb.helper.open_visualizer(visualize_filename, host=host, port=port, data_folder=data_folder, open_browser=open_browser)
