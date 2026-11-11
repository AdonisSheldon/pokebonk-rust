"""Collect immutable updater bytes and generate a complete GitHub release feed."""
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import sys

TARGETS = {
    "x86_64-pc-windows-msvc": ("windows-x86_64", "Windows-x64-setup", ".exe"),
    "aarch64-apple-darwin": ("darwin-aarch64", "macOS-arm64", ".app.tar.gz"),
    "x86_64-apple-darwin": ("darwin-x86_64", "macOS-x86_64", ".app.tar.gz"),
}


def config(project):
    return json.loads((project / "src-tauri/tauri.conf.json").read_text(encoding="utf-8"))


def asset_name(version, target):
    _, label, suffix = TARGETS[target]
    return f"PokeBonk-Rust-{version}-{label}{suffix}"


def collect(project, target, destination):
    settings = config(project)
    version, product = settings["version"], settings["productName"]
    bundle = project / "src-tauri/target" / target / "release/bundle"
    source = (bundle / "nsis" / f"{product}_{version}_x64-setup.exe"
              if target == "x86_64-pc-windows-msvc"
              else bundle / "macos" / f"{product}.app.tar.gz")
    sources = [source, Path(str(source) + ".sig")]
    for path in sources:
        if not path.is_file():
            raise FileNotFoundError(path)
    destination.mkdir(parents=True, exist_ok=True)
    name = asset_name(version, target)
    for path, suffix in zip(sources, ("", ".sig")):
        with (destination / (name + suffix)).open("xb") as output, path.open("rb") as stream:
            shutil.copyfileobj(stream, output)


def manifest(project, destination, repository):
    version = config(project)["version"]
    platforms = {}
    for target, (platform, _, _) in TARGETS.items():
        name = asset_name(version, target)
        package = destination / name
        if not package.is_file():
            raise FileNotFoundError(package)
        signature = (destination / (name + ".sig")).read_text(encoding="utf-8").strip()
        if not signature:
            raise ValueError(f"Empty signature for {name}")
        platforms[platform] = {
            "url": f"https://github.com/{repository}/releases/download/v{version}/{name}",
            "signature": signature,
        }
    result = {"version": version,
              "pub_date": datetime.now(timezone.utc).isoformat().replace("+00:00", "Z"),
              "platforms": platforms}
    (destination / "latest.json").write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    checksums = []
    for path in sorted(destination.iterdir()):
        if path.is_file() and path.name != "SHA256SUMS.txt":
            with path.open("rb") as stream:
                digest = hashlib.file_digest(stream, "sha256").hexdigest()
            checksums.append(f"{digest}  {path.name}\n")
    (destination / "SHA256SUMS.txt").write_text("".join(checksums), encoding="utf-8")
    return result


if __name__ == "__main__":
    root = Path(__file__).resolve().parents[1]
    if len(sys.argv) == 4 and sys.argv[1] == "collect":
        collect(root, sys.argv[2], Path(sys.argv[3]))
    elif len(sys.argv) == 4 and sys.argv[1] == "manifest":
        manifest(root, Path(sys.argv[2]), sys.argv[3])
    else:
        raise SystemExit("Usage: release.py collect TARGET DIR | manifest DIR OWNER/REPO")
