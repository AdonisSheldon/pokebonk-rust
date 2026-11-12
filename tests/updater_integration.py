"""Exercise the real read-only Rust updater against locally served release bytes."""
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading

project = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(project / "packaging"))
from release import TARGETS, asset_name  # noqa: E402

probe, packages, report = [Path(value).resolve() for value in sys.argv[1:]]
version = json.loads((project / "src-tauri/tauri.conf.json").read_text())["version"]
mode = "valid"


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *_):
        pass

    def do_GET(self):
        if self.path == "/latest.json":
            platforms = {}
            for target, (key, _, _) in TARGETS.items():
                name = asset_name(version, target)
                platforms[key] = {
                    "url": f"http://127.0.0.1:{self.server.server_port}/{name}",
                    "signature": (packages / (name + ".sig")).read_text().strip(),
                }
            data = {"version": "99.0.0" if mode == "mismatch" else version,
                    "platforms": {} if mode == "missing" else platforms}
            content = json.dumps(data).encode()
        else:
            name = self.path.removeprefix("/")
            if name not in {asset_name(version, target) for target in TARGETS}:
                self.send_error(404)
                return
            content = (packages / name).read_bytes()
            if mode == "tampered":
                content = bytes([content[0] ^ 1]) + content[1:]
        self.send_response(200)
        self.send_header("Content-Length", str(len(content)))
        self.end_headers()
        self.wfile.write(content)


server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
threading.Thread(target=server.serve_forever, daemon=True).start()
results = {}
try:
    with tempfile.TemporaryDirectory(prefix="pokebonk-update-probe-") as temporary:
        cases = [(key, key, "valid", "0.1.2", "verified") for key, _, _ in TARGETS.values()]
        cases += [("current", "darwin-aarch64", "valid", version, "current"),
                  ("wrong-signed-version", "darwin-aarch64", "mismatch", "0.1.2", "error"),
                  ("tampered-package", "darwin-aarch64", "tampered", "0.1.2", "error"),
                  ("missing-platform", "darwin-aarch64", "missing", "0.1.2", "error")]
        for name, target, mode, current, expected in cases:
            output = Path(temporary) / f"{name}.json"
            env = os.environ.copy()
            env["POKEBONK_PROBE_VERSION"] = current
            result = subprocess.run([str(probe), f"http://127.0.0.1:{server.server_port}/latest.json",
                                     str(output), target], env=env, capture_output=True, timeout=45)
            assert output.is_file(), (name, result.returncode, result.stderr.decode(errors="replace"))
            data = json.loads(output.read_text())
            assert data["outcome"] == expected, (name, data)
            if expected == "verified":
                target_triple = next(item for item, spec in TARGETS.items() if spec[0] == target)
                original = packages / asset_name(version, target_triple)
                assert output.with_suffix(".download").read_bytes() == original.read_bytes()
                assert data["version"] == version and data["current_version"] == current
            else:
                assert not output.with_suffix(".download").exists(), name
            results[name] = data
            print(f"{name}: {data['outcome']}", flush=True)
finally:
    server.shutdown()
report.write_text(json.dumps(results, indent=2) + "\n")
