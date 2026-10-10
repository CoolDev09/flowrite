# flowrite

A native Rust editor with a quiet, dark Notion-inspired workspace, document formatting, academic presets, and PDF/Markdown export.

## Run

Double-click **flowrite.exe** if you have a compiled copy. Rust is not required to run the app. The source repository does not include compiled executables or personal documents.

To build on Windows, install stable Rust with the MSVC toolchain and Visual Studio C++ build tools (including a Windows SDK), then run these commands from the repository folder:

```powershell
cargo test --release --locked
cargo build --release --locked
Copy-Item target/release/flowwrite.exe flowrite.exe
```

The internal Cargo package name remains `flowwrite` for compatibility; the app is branded **flowrite**. Windows is the primary supported platform, including its font loading and PDF export.

## Your documents

- The dark home page shows templates, folders, search, and previews of your recent documents. Click the flowrite logo in the top-left to return home from any document.
- **Create folder**, beside **New page**, adds a folder. Open the folder to create pages inside it. A document card's **…** menu moves existing documents into or out of folders.
- **Open file** or **File > Open** imports a .flowrite or older .flowwrite file. **File > Save as** saves a separate editable .flowrite file; further edits also update that file automatically.
- The sidebar appears while editing and contains your local document library. Search by title or click a document to open it. The sidebar is hidden on the home page.
- Tabs let you switch between open documents. Closing a tab keeps its document in the library.
- Changes save automatically after a short pause, when switching documents, and when closing the app. The header shows saving status. Documents and open tabs return on the next launch.
- Your library is stored in `%LOCALAPPDATA%\FlowWrite\workspace.json`. This location is retained for compatibility with earlier versions. Back up this file to keep the entire library. There is no cloud sync.

- Type **[[Document title]]** to reference another document. An **Open Document title** link appears below that paragraph. Names match without regard to capitalization. Renaming a document requires updating references to its old title.

## Writing and formatting

- Type / at the start of a paragraph to open a compact insertion menu. Keep typing to filter (for example /h1); choose with Up/Down and Enter. Escape dismisses the menu.
- Enter creates the next paragraph; Shift+Enter adds a line break. Lists continue on Enter; Enter on an empty list item returns to a paragraph.
- Use the six-dot handle or right-click for block actions. Blocks have no permanent type labels.
- Select text and use **B**, **I**, or **U**, or their Ctrl keyboard shortcuts. With no selection, these controls format the active paragraph.
- Change font, size, spacing, or block type in the toolbar. Page setup controls margins and indentation.
- Choose **MLA 9th** or **APA 7th** for an academic preset, then fill in the paper details in Page setup. PDF uses Letter pages, running page numbers, double spacing, and first-line indentation. MLA adds the first-page author heading; APA adds a student title page.
- Presets are a starting point for paper layout. Follow your instructor's requirements for headings, title-page information, and citation content.

## Works Cited / References

Open **Citations** and choose **Add citation**. Paste one complete citation per entry; flowrite keeps the entries alphabetized and lays them out with hanging indents. APA names the section **References**; other modes use **Works Cited**. The editor shows a preview at the end of the document. PDF starts this section on a new page; Markdown includes the entries under a heading (Markdown itself does not preserve print margins or hanging indents).

This formats citations you supply. It does not look up sources or generate bibliographic details. The test fixtures use fictional demonstration citations.

## Export and shortcuts

Export to **PDF** for printing or **Markdown** for portable text. PDF preserves styled text and uses embedded Windows fonts. Markdown includes headings, lists, tasks, code, bold/italic/underline, document links, and citations. Markdown document links target neighboring files named after the linked documents; export those documents separately to the same folder.

Ctrl+N: new document · Ctrl+O: open · Ctrl+S: save now · Ctrl+Shift+S: save as · Ctrl+F: find · Ctrl+Z: undo · Ctrl+Shift+Z / Ctrl+Y: redo.

## Source layout

- main.rs: app state and file operations.
- document.rs: document model, rich text, and format presets.
- editor_ui.rs: editor, toolbar, slash commands, and keyboard handling.
- workspace.rs: persistent library and atomic saves.
- workspace_ui.rs: sidebar, tabs, citations, and document switching.
- home_ui.rs: home page, document previews, and folder controls.
- brand.rs: vector logo drawing and native window icon. The editable SVG is in assets/flowrite.svg.
- wikilinks.rs: document-reference parsing.
- export.rs: Markdown and PDF layout.

Run `cargo test --release` for editor, persistence, linking, compatibility, and export checks.
