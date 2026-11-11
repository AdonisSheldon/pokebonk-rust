import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

module_path = Path(__file__).resolve().parents[1] / "packaging/release.py"
spec = importlib.util.spec_from_file_location("release", module_path)
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.project = Path(self.temporary.name)
        config = self.project / "src-tauri/tauri.conf.json"
        config.parent.mkdir()
        config.write_text(json.dumps({"productName": "PokeBonk Rust", "version": "0.1.3"}))
        self.dist = self.project / "dist"
        self.dist.mkdir()

    def test_collect_keeps_signed_bytes_and_refuses_overwrite(self):
        source = self.project / "src-tauri/target/x86_64-pc-windows-msvc/release/bundle/nsis/PokeBonk Rust_0.1.3_x64-setup.exe"
        source.parent.mkdir(parents=True)
        source.write_bytes(b"signed installer bytes")
        Path(str(source) + ".sig").write_text("signature content")
        release.collect(self.project, "x86_64-pc-windows-msvc", self.dist)
        dest = self.dist / "PokeBonk-Rust-0.1.3-Windows-x64-setup.exe"
        self.assertEqual(dest.read_bytes(), source.read_bytes())
        self.assertEqual(Path(str(dest) + ".sig").read_text(), "signature content")
        with self.assertRaises(FileExistsError):
            release.collect(self.project, "x86_64-pc-windows-msvc", self.dist)

    def prepare_assets(self, platforms):
        for label in platforms:
            suffix = ".exe" if label == "Windows-x64-setup" else ".app.tar.gz"
            path = self.dist / f"PokeBonk-Rust-0.1.3-{label}{suffix}"
            path.write_bytes(b"package bytes")
            Path(str(path) + ".sig").write_text("signature-" + label)

    def test_manifest_routes_all_architectures_and_embeds_signature_content(self):
        self.prepare_assets(["Windows-x64-setup", "macOS-arm64", "macOS-x86_64"])
        result = release.manifest(self.project, self.dist, "AdonisSheldon/pokebonk-rust")
        self.assertEqual(result["version"], "0.1.3")
        self.assertEqual(set(result["platforms"]), {"windows-x86_64", "darwin-aarch64", "darwin-x86_64"})
        arm = result["platforms"]["darwin-aarch64"]
        self.assertEqual(arm["signature"], "signature-macOS-arm64")
        self.assertEqual(arm["url"], "https://github.com/AdonisSheldon/pokebonk-rust/releases/download/v0.1.3/PokeBonk-Rust-0.1.3-macOS-arm64.app.tar.gz")
        self.assertEqual(json.loads((self.dist / "latest.json").read_text()), result)
        self.assertIn("latest.json", (self.dist / "SHA256SUMS.txt").read_text())

    def test_missing_platform_prevents_publishing_a_partial_manifest(self):
        self.prepare_assets(["Windows-x64-setup", "macOS-arm64"])
        with self.assertRaises(FileNotFoundError):
            release.manifest(self.project, self.dist, "AdonisSheldon/pokebonk-rust")
        self.assertFalse((self.dist / "latest.json").exists())


if __name__ == "__main__":
    unittest.main()
