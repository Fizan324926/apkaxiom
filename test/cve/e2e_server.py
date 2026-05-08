#!/usr/bin/env python3
"""e2e_server.py — Minimal APK analysis HTTP service.

Simulates the exact code path used by:
  - Exodus Privacy (exodus_core/analysis/static_analysis.py:258)
  - Quark-Engine (quark/core/apkinfo.py:42)
  - Any Androguard-based APK scanner

Accepts POST /analyze with an APK file upload, runs the same
Androguard calls these tools make, and returns JSON results.

Usage:
    python3 e2e_server.py &       # starts on port 9999
    curl -F apk=@poc.apk http://localhost:9999/analyze
"""

import http.server
import json
import os
import sys
import tempfile
import threading
import traceback

from androguard.core.apk import APK


class AnalysisHandler(http.server.BaseHTTPRequestHandler):
    def log_message(self, fmt, *args):
        sys.stderr.write(f"[server] {fmt % args}\n")

    def do_POST(self):
        if self.path != "/analyze":
            self.send_response(404)
            self.end_headers()
            return

        content_length = int(self.headers.get("Content-Length", 0))
        body = self.rfile.read(content_length)

        boundary = self.headers.get("Content-Type", "").split("boundary=")[-1]
        if boundary:
            parts = body.split(f"--{boundary}".encode())
            for part in parts:
                if b"filename=" in part:
                    header_end = part.find(b"\r\n\r\n")
                    if header_end > 0:
                        body = part[header_end + 4:]
                        if body.endswith(b"\r\n"):
                            body = body[:-2]
                        break

        with tempfile.NamedTemporaryFile(suffix=".apk", delete=False) as f:
            f.write(body)
            tmp_path = f.name

        try:
            # === THIS IS THE EXACT CODE PATH EXODUS PRIVACY USES ===
            # exodus_core/analysis/static_analysis.py line 161:
            #     self.apk = APK(self.apk_path)
            # line 258:
            #     return self.apk.get_androidversion_name()
            a = APK(tmp_path)
            version_name = a.get_androidversion_name()
            version_code = a.get_androidversion_code()
            package = a.get_package()
            permissions = a.get_permissions()

            result = {
                "status": "ok",
                "package": package,
                "version_name": version_name,
                "version_code": version_code,
                "permissions": list(permissions),
            }
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(json.dumps(result).encode())
        except Exception as e:
            tb = traceback.format_exc()
            sys.stderr.write(f"[server] CRASH: {tb}\n")
            raise
        finally:
            os.unlink(tmp_path)

    def do_GET(self):
        if self.path == "/health":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.end_headers()
            self.wfile.write(b'{"status":"alive"}')
        else:
            self.send_response(404)
            self.end_headers()


def run(port=9999):
    server = http.server.HTTPServer(("127.0.0.1", port), AnalysisHandler)
    sys.stderr.write(f"[server] APK analysis service listening on :{port}\n")
    server.serve_forever()


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 9999
    run(port)
