import hashlib
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import tempfile

import dmgbuild
from ds_store import DSStore


def manifest(root):
    entries = {}
    for path in root.rglob("*"):
        name = str(path.relative_to(root))
        if path.is_symlink():
            entries[name] = ("link", os.readlink(path))
        elif path.is_file():
            with path.open("rb") as stream:
                entries[name] = ("file", hashlib.file_digest(stream, "sha256").hexdigest())
        else:
            entries[name] = ("directory",)
    return entries


if len(sys.argv) != 3:
    raise SystemExit('Usage: python3 build-dmg.py "PokeBonk Rust.app" output.dmg')
app, output = [Path(arg).resolve() for arg in sys.argv[1:]]
assert app.name == "PokeBonk Rust.app" and app.is_dir()
assert output.suffix == ".dmg" and not output.exists(), "Refusing to overwrite an existing image"
info = plistlib.loads((app / "Contents/Info.plist").read_bytes())
architecture = subprocess.check_output([
    "/usr/bin/lipo", "-archs", app / "Contents/MacOS" / info["CFBundleExecutable"]
], text=True).strip()
assert architecture in ("arm64", "x86_64"), "Expected a single Apple Silicon or Intel architecture"
expected = manifest(app)
# Keep the background file off canvas even when Finder shows hidden files.
positions = {app.name: (160, 145), "Applications": (480, 145), ".background.tiff": (900, 500)}

# A synced Documents folder can reattach FinderInfo and invalidate strict signatures.
with tempfile.TemporaryDirectory(prefix="pokebonk-installer-", dir="/private/tmp") as temporary:
    root = Path(temporary)
    staged = root / app.name
    subprocess.run(["/usr/bin/ditto", "--norsrc", "--noextattr", app, staged], check=True)
    assert manifest(staged) == expected
    subprocess.run(["/usr/bin/codesign", "--verify", "--deep", "--strict", staged], check=True)
    candidate = root / "installer.dmg"
    dmgbuild.build_dmg(str(candidate), "PokeBonk Rust " + info["CFBundleShortVersionString"], settings={
        "format": "UDZO", "filesystem": "APFS", "compression_level": 9,
        "files": [str(staged)], "symlinks": {"Applications": "/Applications"},
        "background": str(Path(__file__).with_name("dmg-background.tiff").resolve()),
        "window_rect": ((140, 120), (640, 360)),
        "default_view": "icon-view", "icon_locations": positions,
        "icon_size": 96, "text_size": 13, "grid_spacing": 64,
        "include_icon_view_settings": True, "include_list_view_settings": False,
    })
    subprocess.run(["/usr/bin/hdiutil", "verify", candidate], check=True)
    mount = root / "mounted"
    mount.mkdir()
    subprocess.run(["/usr/bin/hdiutil", "attach", "-readonly", "-nobrowse",
                    "-mountpoint", mount, candidate], check=True)
    try:
        assert manifest(mount / app.name) == expected, "Packaged app differs from the original"
        assert os.readlink(mount / "Applications") == "/Applications"
        assert {p.name for p in mount.iterdir() if not p.name.startswith(".")} == {app.name, "Applications"}
        with DSStore.open(str(mount / ".DS_Store"), "r") as store:
            assert store["."]["icvp"]["backgroundType"] == 2
            assert store["."]["bwsp"]["WindowBounds"] == "{{140, 120}, {640, 360}}"
            assert not store["."]["bwsp"]["ShowToolbar"]
            assert not store["."]["bwsp"]["ShowSidebar"]
            for name, position in positions.items():
                assert store[name]["Iloc"] == position
        subprocess.run(["/usr/bin/codesign", "--verify", "--deep", "--strict", mount / app.name], check=True)
    finally:
        subprocess.run(["/usr/bin/hdiutil", "detach", mount], check=True)
    shutil.move(candidate, output)

with output.open("rb") as stream:
    print("Verified:", output, "\nArchitecture:", architecture,
          "\nSHA256:", hashlib.file_digest(stream, "sha256").hexdigest())
