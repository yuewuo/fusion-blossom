#!/usr/bin/env python3
# Inspired by  https://stackoverflow.com/a/25708957/51280
# from https://gist.github.com/opyate/6e5fcabc6f41474d248613c027373856
from http.server import SimpleHTTPRequestHandler
import socketserver
import os
import urllib
import posixpath

SCRIPT_FOLDER = os.path.dirname(os.path.abspath(__file__))


class MyHTTPRequestHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=SCRIPT_FOLDER, **kwargs)

    def end_headers(self):
        self.send_my_headers()
        SimpleHTTPRequestHandler.end_headers(self)

    def send_my_headers(self):
        self.send_header(
            "Cache-Control", "no-cache, no-store, must-revalidate")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")

    # from https://hg.python.org/cpython/file/3.5/Lib/http/server.py
    def translate_path(self, path):
        # abandon query parameters
        path = path.split('?', 1)[0]
        path = path.split('#', 1)[0]
        # allowing visualizer to access benchmark data directly
        is_benchmark = path.startswith("/data/benchmark/")
        if is_benchmark:
            path = path[len("/data/benchmark/"):]  # remove head
        # Don't forget explicit trailing slash when normalizing. Issue17324
        trailing_slash = path.rstrip().endswith('/')
        try:
            path = urllib.parse.unquote(path, errors='surrogatepass')
        except UnicodeDecodeError:
            path = urllib.parse.unquote(path)
        path = posixpath.normpath(path)
        words = path.split('/')
        words = filter(None, words)
        path = os.getcwd()
        if is_benchmark:
            path = os.path.join(os.path.dirname(path), "benchmark")
        for word in words:
            if os.path.dirname(word) or word in (os.curdir, os.pardir):
                # Ignore components that are not a simple file/directory name
                continue
            path = os.path.join(path, word)
        if trailing_slash:
            path += '/'
        return path


if __name__ == '__main__':
    print(f"running server to host folder {SCRIPT_FOLDER}")
    with socketserver.TCPServer(("0.0.0.0", 8066), MyHTTPRequestHandler) as httpd:
        print("serving at port", 8066)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("KeyboardInterrupt")
            pass
        finally:
            httpd.server_close()
