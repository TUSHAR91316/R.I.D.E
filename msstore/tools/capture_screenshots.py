#!/usr/bin/env python3
"""
R.I.D.E — Microsoft Store Screenshot Capture Tool
Automates capturing 1920x1080 high-resolution screenshots for Microsoft Partner Center Store submission.
"""

import http.server
import os
import socketserver
import subprocess
import threading
import time

SCREENSHOTS_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "Screenshots"))
DIST_DIR = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", "..", "crates", "ride-desktop", "ui", "dist"))
os.makedirs(SCREENSHOTS_DIR, exist_ok=True)

PORT = 8990
EDGE_PATH = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
CHROME_PATH = r"C:\Program Files\Google\Chrome\Application\chrome.exe"

BROWSER = EDGE_PATH if os.path.exists(EDGE_PATH) else CHROME_PATH


class QuietHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=DIST_DIR, **kwargs)

    def log_message(self, format, *args):
        pass  # Suppress console logging


def capture(url, out_filename):
    out_path = os.path.join(SCREENSHOTS_DIR, out_filename)
    cmd = [
        BROWSER,
        "--headless=new",
        "--hide-scrollbars",
        "--window-size=1920,1080",
        "--force-device-scale-factor=1",
        f"--screenshot={out_path}",
        url,
    ]
    subprocess.run(cmd, check=True, timeout=15)
    size_kb = os.path.getsize(out_path) / 1024
    print(f"  Captured: {out_filename} (1920x1080, {size_kb:.1f} KB)")


def main():
    print("=" * 70)
    print("  R.I.D.E — Generating Microsoft Store High-Resolution Screenshots")
    print(f"  Target Directory: {SCREENSHOTS_DIR}")
    print("=" * 70)

    httpd = socketserver.TCPServer(("127.0.0.1", PORT), QuietHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    time.sleep(0.5)

    base_url = f"http://127.0.0.1:{PORT}/index.html"

    screenshots = [
        ("?view=explorer", "01_Code_Editor_Workspace.png"),
        ("?layout=claude_code&mockChat=1", "02_Intelligent_MoE_Agent.png"),
        ("?view=changes", "03_Visual_Diff_Code_Review.png"),
        ("?view=skills", "04_Skills_Manager_Local_AI.png"),
        ("?layout=codex", "05_Zen_Codex_Focus_Mode.png"),
    ]

    try:
        for query, filename in screenshots:
            capture(f"{base_url}{query}", filename)
    finally:
        httpd.shutdown()

    print("\n[SUCCESS] All Microsoft Store screenshots captured successfully!")


if __name__ == "__main__":
    main()
