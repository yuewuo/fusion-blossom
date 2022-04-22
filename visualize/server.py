#!/usr/bin/env python3
# Inspired by  https://stackoverflow.com/a/25708957/51280
# from https://gist.github.com/opyate/6e5fcabc6f41474d248613c027373856
from http.server import SimpleHTTPRequestHandler
import socketserver
import os

SCRIPT_FOLDER = os.path.dirname(os.path.abspath(__file__))

class MyHTTPRequestHandler(SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=SCRIPT_FOLDER, **kwargs)

    def end_headers(self):
        self.send_my_headers()
        SimpleHTTPRequestHandler.end_headers(self)

    def send_my_headers(self):
        self.send_header("Cache-Control", "no-cache, no-store, must-revalidate")
        self.send_header("Pragma", "no-cache")
        self.send_header("Expires", "0")


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
