#!/usr/bin/env python3
"""Live training dashboard: PSNR of every student run, from the logs in ~/wonder.  python server.py [port]"""
import http.server, json, os, re, subprocess, sys, time

ROOT = os.path.expanduser("~/wonder")
HERE = os.path.dirname(os.path.abspath(__file__))
RUNS = [  # (log, label, total steps)
    ("student_q4.log", "v1 · plain", 30000),
    ("student_q4d.log", "v1 · distilled", 100000),
    ("student_q4d2.log", "v1 · 2 passes", 100000),
    ("student_q4v2.log", "v2 · 4-bit from 60k", 100000),
    ("student_q4v28.log", "v2 · 8-bit from 90k", 100000),
]
PT = re.compile(r"^(\d+) psnr\(train crops\) (-?[\d.]+)\s+(\d+)s")
QB = re.compile(r"^(\d+): (\d+)-bit weights from here")

def runs():
    out = []
    for log, label, total in RUNS:
        p = os.path.join(ROOT, log)
        if not os.path.exists(p): continue
        pts, marks, done = [], [], False
        for line in open(p, errors="ignore"):
            m = PT.match(line)
            if m: pts.append([int(m[1]), float(m[2]), int(m[3])])
            m = QB.match(line)
            if m: marks.append([int(m[1]), int(m[2])])
            if line.startswith("finished"): done = True
        out.append({"label": label, "total": total, "points": pts, "marks": marks, "done": done or (pts and pts[-1][0] >= total),
                    "mtime": os.path.getmtime(p)})
    return out

def gpu():
    try:
        r = subprocess.run(["nvidia-smi", "--query-gpu=utilization.gpu,memory.used,memory.total,temperature.gpu,power.draw", "--format=csv,noheader,nounits"],
                           capture_output=True, text=True, timeout=5).stdout.strip().split(", ")
        return {"util": float(r[0]), "mem": float(r[1]), "memtot": float(r[2]), "temp": float(r[3]), "power": float(r[4])}
    except Exception: return None

class H(http.server.BaseHTTPRequestHandler):
    def log_message(self, *a): pass
    def do_GET(self):
        if self.path.startswith("/data"):
            body = json.dumps({"runs": runs(), "gpu": gpu(), "now": time.time()}).encode(); ctype = "application/json"
        else:
            body = open(os.path.join(HERE, "index.html"), "rb").read(); ctype = "text/html; charset=utf-8"
        self.send_response(200); self.send_header("Content-Type", ctype); self.send_header("Cache-Control", "no-store")
        self.send_header("Content-Length", str(len(body))); self.end_headers(); self.wfile.write(body)

http.server.ThreadingHTTPServer(("0.0.0.0", int(sys.argv[1]) if len(sys.argv) > 1 else 8766), H).serve_forever()
