"""
Note: this file is compiled as part of the binary, recompile if you change this to take effect
"""

import webbrowser, threading, os, time, subprocess, tempfile, sys, shutil
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

def run_command_get_stdout(command, no_stdout=True, use_tmp_out=False, stderr_to_stdout=False, cwd=None):
    env = os.environ.copy()
    stdout = subprocess.PIPE
    if use_tmp_out:
        out_file = tempfile.NamedTemporaryFile(delete=False)
        out_filename = out_file.name
        stdout = out_file
    if no_stdout:
        stdout = sys.stdout
    process = subprocess.Popen(command, universal_newlines=True, env=env, stdout=stdout
        , stderr=(stdout if stderr_to_stdout else sys.stderr), cwd=cwd)
    stdout, _ = process.communicate()
    if use_tmp_out:
        out_file.flush()
        out_file.close()
        with open(out_filename, "r", encoding="utf8") as f:
            stdout = f.read()
        os.remove(out_filename)
    return stdout, process.returncode

"""
render a visualizer image just like a browser does, but locally and save to file.
note that we'll run node.js in `renderer_folder`, which generates a lot of files in `renderer_folder`/node_modules.
this is necessary because some dependencies require local environment to compile native code.
"""
def local_render_visualizer(
        filename, image_filename="rendered", renderer_folder="./local_renderer", width=1024, height=1024
        , data_folder=".", snapshot_idx=0, patch_script=None):
    if not os.path.exists(renderer_folder):
        os.makedirs(renderer_folder)
    global visualizer_website
    for website_filename in visualizer_website:
        filepath = os.path.join(renderer_folder, website_filename)
        # always replace the files because the content may be outdated in a new version
        with open(filepath, "w", encoding="utf-8") as f:
            f.write(visualizer_website[website_filename])
    if not os.path.exists(os.path.join(renderer_folder, "node_modules")):
        run_command_get_stdout(["npm", "install"], cwd=renderer_folder)
    if patch_script is not None:
        patch_filename = f"{filename}.patch.js"
        with open(os.path.join(renderer_folder, patch_filename), "w", encoding="utf8") as f:
            f.write(patch_script)
    renderer_data_folder = os.path.join(renderer_folder, "data")
    if not os.path.exists(renderer_data_folder):
        os.makedirs(renderer_data_folder)
    shutil.copy2(os.path.join(data_folder, filename), renderer_data_folder)
    domain = "http://localhost"  # doesn't matter, won't use in the local renderer
    url = f"{domain}?filename={filename}&snapshot_idx={snapshot_idx}"
    if patch_script is not None:
        url += f"&patch_url=./{patch_filename}"
    commands = ["node", "index.js", url, f"{width}", f"{height}", image_filename]
    print(f"[run] {commands}")
    run_command_get_stdout(commands, cwd=renderer_folder)
