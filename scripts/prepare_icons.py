"""Prepare only the app's icons from the officially distributed Uicons npm pack.

Run `npm pack @flaticon/flaticon-uicons@3.3.1 --ignore-scripts` first. This script
does not access or download anything from the Flaticon website. Version 3.3.1
ships WOFF vectors; SVGs are exact glyph-outline derivatives, not replacement
artwork. Install fonttools==4.60.1, or pass its directory with --tools-dir.
"""
import argparse
import hashlib
import io
import re
import sys
import tarfile
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("archive", type=Path)
parser.add_argument("--tools-dir", type=Path)
args = parser.parse_args()
if args.tools_dir:
    sys.path.insert(0, str(args.tools_dir))
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen

ROOT = Path(__file__).resolve().parents[1]
DEST = ROOT / "assets/icons/vendor"
NAMES = ["arrow-left", "arrow-right", "trash", "search", "disk", "settings",
         "sidebar", "picture", "text", "list", "list-check", "quote-right",
         "info", "bulb", "triangle-warning", "exclamation", "copy", "cross",
         "plus", "menu-dots", "file-export", "folder", "folder-open", "pencil",
         "bold", "italic", "underline", "interlining", "document", "book",
         "check", "code-simple", "heading", "horizontal-rule", "layout-fluid",
         "palette", "file-import", "arrow-up", "arrow-down", "home", "file-pdf", "apps"]
with tarfile.open(args.archive) as archive:
    # Extract known regular files only, never arbitrary archive paths/symlinks.
    def read(name):
        entry = archive.getmember(name)
        if not entry.isfile() or entry.size > 20_000_000:
            raise ValueError(f"Unexpected package member: {name}")
        return archive.extractfile(entry).read()
    css = read("package/css/regular/rounded.css").decode()
    codes = dict(re.findall(r'\.fi-rr-([a-z0-9-]+):before\{content:"\\([a-f0-9]+)"\}', css))
    font = TTFont(io.BytesIO(read("package/css/uicons-regular-rounded-KDJ23353.woff")))
    glyphs = font.getGlyphSet()
    cmap = font.getBestCmap()
    units = font["head"].unitsPerEm
    ascent = font["hhea"].ascent
    prepared = {}
    for name in NAMES:
        glyph = glyphs[cmap[int(codes[name], 16)]]
        pen = SVGPathPen(glyphs)
        glyph.draw(pen)
        prepared[name] = (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {units} {units}">'
                          f'<path transform="translate(0 {ascent}) scale(1 -1)" d="{pen.getCommands()}"/>'
                          '</svg>\n')
    license_text = read("package/LICENSE").decode()
DEST.mkdir(parents=True, exist_ok=True)
for name, svg in prepared.items():
    (DEST / f"{name}.svg").write_text(svg, encoding="utf-8")
(DEST / "LICENSE").write_text(license_text, encoding="utf-8")
sha = hashlib.sha256(args.archive.read_bytes()).hexdigest()
rows = "\n".join(f"| {name}.svg | fi-rr-{name} | Flaticon | Regular Rounded 3.3.1 |" for name in NAMES)
(DEST.parent / "ICONS.md").write_text(f"""# Icon provenance

Uicons by [Flaticon](https://www.flaticon.com/uicons/interface-icons).
Downloaded from the officially supported npm package `@flaticon/flaticon-uicons@3.3.1`.
User approved this package route in place of manual website SVG downloads.

Source: https://www.npmjs.com/package/@flaticon/flaticon-uicons/v/3.3.1
Guide: https://www.flaticon.com/uicons/get-started/
Archive SHA-256: `{sha}`
Prepared 2026-10-10. This package contains webfonts, not separate SVGs; the SVGs
are exact vector-outline derivatives of its Regular Rounded WOFF glyphs.

Only these icons are embedded in the application, never served over a network.
The original Flaticon license is bundled in About. Raw vendor artwork is ignored
by Git and must not be published as a standalone icon collection. Recreate it
with `scripts/prepare_icons.py` and the official npm archive before building.

| File | Source class | Designer | Pack |
| --- | --- | --- | --- |
{rows}
""", encoding="utf-8")
print(f"Prepared {len(prepared)} SVGs, units={units}, ascent={ascent}.")
