"""
Note: this file is compiled as part of the binary, recompile if you change this to take effect
"""

import bottle, webbrowser, threading

print(bottle.__version__)

fb = None

"""called by fusion_blossom package at import, to set up global variables"""
def register(module):
    global fb
    fb = module


"""
start a server to host the visualizer websites locally
"""
def serve(host='localhost', port=51666, data_folder=".", return_starter=False):
    from bottle import route, abort, run, response, static_file
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
    def starter():
        run(host=host, port=port, debug=True)
    if return_starter:
        return starter
    starter()


"""
open the website directly after starting the browser
"""
def open_visualizer(filename, host='localhost', port=51666, data_folder=".", open_browser=True):
    starter = serve(host, port, data_folder, return_starter=True)
    if open_browser:
        def open_browser():
            webbrowser.open(f"http://{host}:{port}/?filename={filename}")
        threading.Timer(0.5, open_browser).start()  # wait 500ms for the server to start
    starter()
