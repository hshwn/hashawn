#!/usr/bin/env python3
"""Local dev server: static files plus the one rewrite Cloudflare Pages does for us (/blog/<slug>/ -> blog/index.html)."""
import http.server, os, sys
ROOT = os.path.dirname(os.path.abspath(__file__))
class H(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *a, **k): super().__init__(*a, directory=ROOT, **k)
    def translate_path(self, path):
        p = path.split('?', 1)[0]
        if p.startswith('/blog/') and p != '/blog/' and not os.path.exists(os.path.join(ROOT, p.lstrip('/'))):
            return os.path.join(ROOT, 'blog', 'index.html')
        return super().translate_path(path)
    def end_headers(self):
        self.send_header('Cache-Control', 'no-store'); super().end_headers()
http.server.ThreadingHTTPServer(('', int(sys.argv[1]) if len(sys.argv) > 1 else 4173), H).serve_forever()
