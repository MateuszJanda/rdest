#!/usr/bin/env python3

import http.server
import socketserver

PORT = 8001


class HttpRequestHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-type", "text/html")

        # Whenever using 'send_header', you also have to call 'end_headers'
        self.end_headers()

        html = "Hello world"

        self.wfile.write(bytes(html, "utf8"))

        return


with socketserver.TCPServer(("", PORT), HttpRequestHandler) as httpd:
    print("Server tracker on http://0.0.0.0:%d" % PORT)
    httpd.serve_forever()
