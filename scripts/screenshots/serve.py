"""Serves the screenshot build for Firefox's headless screenshot mode.

Firefox takes the screenshot when the page load event fires. The harness embeds
a hidden image from `/slow.png`; delaying that response gives React, the layout
effects and the canvas draw a moment to settle before the capture.
"""

import http.server
import time

# 1x1 transparent PNG.
PIXEL = bytes.fromhex(
    "89504e470d0a1a0a0000000d494844520000000100000001080600000"
    "01f15c4890000000d49444154789c6360000002000154a24f6b0000000"
    "049454e44ae426082"
)


class Handler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self) -> None:  # noqa: N802 - stdlib name
        if self.path.startswith("/slow.png"):
            time.sleep(0.6)
            self.send_response(200)
            self.send_header("Content-Type", "image/png")
            self.send_header("Content-Length", str(len(PIXEL)))
            self.end_headers()
            self.wfile.write(PIXEL)
            return
        super().do_GET()


if __name__ == "__main__":
    http.server.ThreadingHTTPServer(("127.0.0.1", 4179), Handler).serve_forever()
