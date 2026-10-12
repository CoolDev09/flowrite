# flowrite

A native Rust writing workspace with a dark, Notion-inspired editor, a document home page, academic presets, and PDF/Markdown export.

Current version: **0.2.3**. This includes the 0.2.0 implementation batch and the compatibility and icon-toolbar updates, not completion of the entire requested roadmap. See [IMPLEMENTATION.md](IMPLEMENTATION.md) for finished work and remaining details.

[Download flowrite 0.2.3 for Windows](https://github.com/CoolDev09/flowrite/releases/download/v0.2.3/flowrite-0.2.3.exe), or browse the [release notes and older versions](https://github.com/CoolDev09/flowrite/releases).

**0.2.1 compatibility fix:** older library indexes that omit `next_id` now load
normally. The counter is reconstructed from existing page/folder IDs. Loading
does not rename these indexes as corrupt; the next save upgrades the index and
keeps its previous contents as a backup. Missing optional index metadata also
uses backward-compatible defaults. Newer-format protection is unchanged.

## Run and build

Double-click the downloaded `flowrite-0.2.3.exe`. Rust is not needed to run the Windows executable. Windows may show a SmartScreen warning because the executable is unsigned.

Building requires stable Rust with the Windows MSVC toolchain, Visual Studio C++ build tools, and a Windows SDK:

```powershell
cargo test --release --locked
cargo build --release --locked
```

The output is `target/release/flowrite.exe`. Fonts, spelling dictionary, and English grammar data are bundled in the executable. A normal build does not download grammar data.

Before the first build of a fresh checkout, download the version-pinned build
assets with Python (standard library only):

```powershell
python scripts/prepare_assets.py
```

This retrieves `flowrite-build-assets-0.2.3.zip` from the GitHub release,
verifies its SHA-256 and every file against `assets/BUILD_ASSETS.json`, then
installs the fonts, dictionary, grammar data, and upstream license notices.
For offline setup, download the ZIP separately and run
`python scripts/prepare_assets.py --archive path/to/flowrite-build-assets-0.2.3.zip`.
The large binary assets live in the release, not the source Git tree. After
setup, normal builds and the app itself do not need network access (Cargo
dependencies must also have been downloaded for an offline build).

Before building a fresh checkout, prepare the licensed icon assets (not needed
to run the executable). Use Node/npm and Python with fonttools 4.60.1:

```powershell
npm pack @flaticon/flaticon-uicons@3.3.1 --ignore-scripts
python -m pip install fonttools==4.60.1
python scripts/prepare_icons.py flaticon-flaticon-uicons-3.3.1.tgz
```

This is the officially supported package route, approved in place of manual
website downloads. It does not automate the Flaticon website. The npm release
contains webfont vectors; the preparation step converts just the app's glyphs
to SVG, preserving the original artwork. Raw vendor files are excluded from Git.

## Your documents

- The logo at the top left returns home. The sidebar is visible in documents, not on the home page. Folders appear as collapsible sidebar groups.
- The start row has New folder, Blank page, MLA essay, and APA paper cards. New-page cards also work inside a folder.
- A blank page stays a temporary draft until the first edit. Closing the last tab returns home; it does not create another empty page.
- Right-click a page card, sidebar entry, or tab for the shared page menu: open, rename, duplicate, move, export, and delete. Rename currently uses a dialog.
- Delete requires confirmation. Cancel has initial focus, Esc cancels, and Enter never confirms deletion. Deleting an externally opened page never deletes its original file. Deleted entries are **not yet restorable through a Trash interface**; their document files are retained in the library folder.
- Autosave runs after a pause (default 1.5 seconds), on document switches, and on close. It writes changed documents independently. Undo retains the caret and groups consecutive typing.
- Open imports image-containing `.flowrite` bundles and older JSON `.flowrite`/`.flowwrite` files. Save as makes an editable bundle including referenced images. Further edits also update that external file.
- `[[Page title]]` links resolve to stable document IDs on commit. Renaming the target does not break them. IDs are hidden in the editor. Linked Markdown documents must be exported separately into the same folder.

The local library is `%LOCALAPPDATA%\\flowrite`: `index.json`, independent `documents/<id>.json` files, and images in `documents/<id>/assets/`. Previous files are kept as `.bak`. Old single-file libraries migrate once and retain `workspace.json.migrated`; the original `FlowWrite` folder is left untouched. Back up the **entire flowrite folder**, not only its index. Automatic backups and library ZIP import/export are not implemented yet.

If files cannot be opened, flowrite tries backups and explains recovery. The recovery screen can retry, open the backup folder, or start fresh after typing START. Starting fresh preserves manifests as `.corrupt` files and retains old document/image files. A newer library format is refused without overwriting its index. `FLOWRITE_DATA_DIR` selects an isolated library for development/tests.

## Writing

- Type `/` at a paragraph's beginning, filter by typing, choose with Up/Down and Enter, and dismiss with Esc. Code, image, and typed callout commands are available.
- Enter makes the next paragraph; Shift+Enter adds a line break. Empty blocks only show a writing hint when focused. Inactive empty blocks have no filler.
- Arrows cross blocks at visual/text boundaries, retaining horizontal position for vertical movement. Image frames and dividers have whole-block keyboard focus. Shift-selection across blocks is pending.
- Backspace at the beginning or Delete at the end merges adjacent text blocks, preserving Unicode and formatting. Non-text blocks are focused, not merged into text.
- Tab/Shift+Tab indent/outdent lists up to six levels. Enter on an empty nested list outdents; an empty top-level item becomes a paragraph.
- Code has a language selector, wrap toggle, and Copy button. Enter adds a line, Tab inserts four spaces, and Ctrl+Enter or Enter on an empty final line exits. Three backticks then Enter also creates code.
- Callouts have a tinted container and colored bar with Note, Tip, Warning, Important, and Neutral variants. They support multiline rich text and code-like exit behavior, with licensed Flaticon icons.
- Image offers Inline, Full width, Square, 4:3, 16:9, Circle, and Rounded frames. Choose a file, drop one onto a frame, or paste a clipboard image with the frame focused. Right-click to replace, change frame, or add a caption. Imports are resized to at most 2000 pixels per dimension.
- Select text for bold, italic, underline, color, or highlight. Without a selection these apply to the block. Color also offers block/page backgrounds and clear formatting. Custom color wheels/recent colors are pending.
- Find highlights exact matches; replace supports case and whole-word matching, Replace, and Replace all.

MLA/APA presets use Liberation Serif (Times-compatible metrics), double spacing, one-inch margins, and half-inch first-line indentation. New headings use today's date and Settings defaults for name/instructor/course. Adjust details in Page setup and follow your instructor's requirements.

## Proofing and settings

Proofing is offline. Red spelling underlines come from `spellbook` with the SCOWL/Hunspell US English dictionary; blue grammar underlines come from `nlprule` and LanguageTool-derived English rules. Checking runs in the background on changed blocks. Right-click an issue for suggestions, ignore controls, or a custom dictionary addition. F7 checks/navigates issues.

Writing & proofing settings include independent spelling/grammar switches, highlight-only mode (no replacements or explanations), rule categories, underline appearance, debounce, ignored entries, and dictionary import/export. Settings apply immediately and save locally to `settings.json`.

Other settings cover startup, export folders, heading defaults/date format, dark/dim appearance, accent, interface scale, home cards, sidebar, editor defaults, statistics and word goal, autosave, citations, export, licenses, and diagnostics. Advanced backup/privacy settings and shortcut rebinding remain pending.

## Citations and export

Paste one ready-made citation per entry in Citations. Entries are alphabetized by default and use hanging indents. PDF starts Works Cited/References on a new page. A citation generator and in-text citation picker are pending.

PDF embeds the same fonts as the editor and supports Letter/A4, academic headers, rich text, nested lists, code/callout containers, images, and captions. It defaults to black on white; optionally use document page colors. Markdown preserves styles, links, fenced code/language, alert callouts, lists, and citations. Images go in a neighboring `_images` folder with cropped PNG files. A suffix avoids overwriting existing asset folders. Markdown ignores colors. DOCX and system Print remain pending.

## Shortcuts

| Action | Keys |
| --- | --- |
| Commands and page search | Ctrl+K |
| New page / new folder | Ctrl+N / Ctrl+Shift+N |
| Open / save / save a copy | Ctrl+O / Ctrl+S / Ctrl+Shift+S |
| Settings | Ctrl+, |
| Find / replace | Ctrl+F / Ctrl+H |
| Undo / redo | Ctrl+Z / Ctrl+Shift+Z or Ctrl+Y |
| Bold / italic / underline | Ctrl+B / Ctrl+I / Ctrl+U |
| Close / reopen tab | Ctrl+W / Ctrl+Shift+T |
| Next / previous tab | Ctrl+Tab / Ctrl+Shift+Tab |
| Jump to tab | Ctrl+1 through Ctrl+9 |
| Check / next issue | F7 |
| Focus mode (hides tabs, toolbar, side panels) | F11 or Ctrl+Shift+F |
| First / last text block | Ctrl+Up / Ctrl+Down |
| Move text block with nested children | Ctrl+Shift+Up / Ctrl+Shift+Down |
| Indent / outdent list | Tab / Shift+Tab |
| Exit code/callout | Ctrl+Enter |

## Licenses and development

Liberation fonts use the SIL Open Font License 1.1; see `assets/fonts/LICENSE`. Grammar binaries are LGPL-2.1, derived from [LanguageTool 5.2](https://github.com/languagetool-org/languagetool/tree/v5.2), distributed by [nlprule](https://github.com/bminixhofer/nlprule). The LGPL text and rebuild/source information are in `assets/nlprule/`. Dictionary licenses are in `assets/spelling/README_en_US.txt`. About provides these notices and an export of dependency notices generated from Cargo.lock and downloaded crate sources at build time.

**Uicons by [Flaticon](https://www.flaticon.com/uicons/interface-icons)** — Regular
Rounded, official package 3.3.1. Version 0.2.2 uses consistent monochrome outline
icons for toolbar controls, tabs, page/block actions, callouts, slash commands,
and palette results. Undo/redo use backward/forward arrows; deletes retain red
text beside a red trashcan. Icons are rasterized at twice display resolution,
cached, and recolored for theme/disabled/selected states. Tooltips, accessible
labels, hand cursors, shortcuts, and deletion safeguards are retained.

Version 0.2.3 makes the main header and toolbar actions icon-only, including File,
Save, Export, Open, Copy, and Settings Done. Hover reveals each action's name and
shortcut; clicks open menus. Tooltips do not automatically open dropdowns. Menu
choices, selected font/format values, and confirmation labels stay readable.

The original Flaticon license and attribution are in Settings > About. See
`assets/icons/ICONS.md` for source classes, package version, and archive hash.
Only embedded app artwork is distributed; the raw vendor collection is ignored
by Git, not published. The custom flowrite vector logo remains unchanged.

Code is split into document, workspace, editor, navigation, home/sidebar, settings, theme, fonts, history, proofing, search, commands, context actions, colors, containers, images, bundle, recovery, and export modules.

Tests cover compatibility, recovery, incremental saves, links, keyboard behavior, placeholders, styling, proofing, cursor helpers, and exports. Set `FLOWRITE_SAMPLE_DIR` to retain PDF/Markdown fixtures, or `FLOWRITE_UI_REVIEW_DIR` for isolated offscreen PNG previews of the real egui panels. These previews do not replace native Windows interaction testing.
