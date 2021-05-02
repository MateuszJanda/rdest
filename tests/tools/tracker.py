#!/usr/bin/env python3

# Copyright 2020 Mateusz Janda.
#
# Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
# https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
# <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
# option. This file may not be copied, modified, or distributed
# except according to those terms.

import http.server
import socketserver

TRACKER_PORT = 8000
PEER_PORT = 6881


class HttpRequestHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-type", "text/html")

        # Whenever using 'send_header', you also have to call 'end_headers'
        self.end_headers()

        html = "d8:intervali1800e5:peersld2:ip9:127.0.0.17:peer id20:AAAAABBBBBCCCCCDDDDD4:porti" + str(PEER_PORT) + "eeee"

        self.wfile.write(bytes(html, "utf8"))

        exit()
        return


with socketserver.TCPServer(("", TRACKER_PORT), HttpRequestHandler) as httpd:
    print("Server tracker on http://127.0.0.1:%d" % TRACKER_PORT)
    httpd.serve_forever()
