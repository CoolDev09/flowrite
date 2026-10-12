"""Install checksum-pinned build assets; never invoked by the running app."""
import argparse
import hashlib
import io
import json
from pathlib import Path, PurePosixPath
import stat
import urllib.request
import zipfile

ROOT = Path(__file__).resolve().parents[1]
MAX_ARCHIVE = 64 * 1024 * 1024


def checked_files(data, manifest):
    if hashlib.sha256(data).hexdigest() != manifest["sha256"]:
        raise ValueError("Build-asset archive checksum mismatch")
    expected = manifest["files"]
    with zipfile.ZipFile(io.BytesIO(data)) as archive:
        entries = [entry for entry in archive.infolist() if not entry.is_dir()]
        names = [entry.filename for entry in entries]
        if len(names) != len(set(names)) or set(names) != set(expected):
            raise ValueError("Unexpected, duplicate, or missing archive files")
        if sum(entry.file_size for entry in entries) > MAX_ARCHIVE:
            raise ValueError("Build assets exceed the unpacked size limit")
        verified = {}
        for entry in entries:
            path = PurePosixPath(entry.filename)
            if path.is_absolute() or ".." in path.parts or "\\" in entry.filename:
                raise ValueError("Unsafe archive path")
            if path.parts[0] not in {"fonts", "nlprule", "spelling"}:
                raise ValueError("Unexpected asset folder")
            if stat.S_ISLNK(entry.external_attr >> 16):
                raise ValueError("Symlinks are not build assets")
            content = archive.read(entry)
            if hashlib.sha256(content).hexdigest() != expected[entry.filename]:
                raise ValueError(f"Asset checksum mismatch: {entry.filename}")
            verified[entry.filename] = content
        return verified


def install(verified, root):
    asset_root = (root / "assets").resolve()
    if not asset_root.is_relative_to(root.resolve()):
        raise ValueError("Asset folder escapes the project")
    targets = []
    for name, content in verified.items():
        target = asset_root.joinpath(*PurePosixPath(name).parts)
        if not target.resolve().is_relative_to(asset_root) or target.is_symlink():
            raise ValueError("Asset destination escapes the project")
        targets.append((target, content))
    for target, content in targets:
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(content)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--archive", type=Path, help="Use a previously downloaded ZIP")
    parser.add_argument("--verify-only", action="store_true", help="Check without installing")
    args = parser.parse_args()
    manifest = json.loads((ROOT / "assets/BUILD_ASSETS.json").read_text(encoding="utf-8"))
    if args.archive:
        if args.archive.stat().st_size > MAX_ARCHIVE:
            raise ValueError("Archive exceeds the size limit")
        data = args.archive.read_bytes()
    else:
        request = urllib.request.Request(manifest["url"], headers={"User-Agent": "flowrite-build-setup"})
        with urllib.request.urlopen(request, timeout=60) as response:
            data = response.read(MAX_ARCHIVE + 1)
        if len(data) > MAX_ARCHIVE:
            raise ValueError("Download exceeds the size limit")
    verified = checked_files(data, manifest)
    if not args.verify_only:
        install(verified, ROOT)
    print(f"Verified {len(verified)} build assets" + ("; installed successfully." if not args.verify_only else "."))


if __name__ == "__main__":
    main()
