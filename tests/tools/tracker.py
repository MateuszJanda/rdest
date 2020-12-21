#!/usr/bin/env python3

import http.server
import socketserver

TRACKER_PORT = 8001
PEER_PORT = 6882


class HttpRequestHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-type", "text/html")

        # Whenever using 'send_header', you also have to call 'end_headers'
        self.end_headers()

        html = "d8:intervali1800e5:peersld2:ip9:127.0.0.17:peer id20:AAAAABBBBBCCCCCDDDDD4:porti" + str(PEER_PORT) + "eeee"

        self.wfile.write(bytes(html, "utf8"))

        return

assert len(str(PEER_PORT)) == 4, "Encoding fail if port doesn't have 4 characters"


with socketserver.TCPServer(("", TRACKER_PORT), HttpRequestHandler) as httpd:
    print("Server tracker on http://127.0.0.1:%d" % TRACKER_PORT)
    httpd.serve_forever()
