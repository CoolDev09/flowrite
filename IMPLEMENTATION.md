# Requested changes: implementation status

This file tracks the supplied Needed Fixes/Changes document. **Partial means the
feature is usable but does not yet satisfy every bullet in that item.** The full
roadmap is not complete. The 0.2.2 style pass uses the official Flaticon Uicons
package, which the user approved instead of manual website SVG downloads.
No new GitHub release was performed.

## Foundations and requested fixes

| Item | Status | Implemented / remaining |
| --- | --- | --- |
| F1 | Implemented | Compact per-document files, index, incremental writes, atomic saves, backups/recovery, legacy migration, newer-version refusal. Fresh recovery preserves old manifests and documents. |
| F2 | Implemented | Lazy drafts, first-edit creation, last-tab home, confirmed empty-page cleanup. |
| F3 | Implemented | No unconditional frame snapshots; dirty-only grouped history, byte cap, file-backed image data. |
| F4 | Implemented | Coalesced style runs; 5,000-word run-count regression test. |
| F5 | Implemented | Runtime Theme, shared colors/accent/callout palettes. |
| F6 | Partial | Bundled licensed Serif/Sans/Mono in editor and PDF, consistent fallback notice. Optional installed Windows font choices remain pending. |
| F7 | Implemented | Persisted counter IDs, title uniqueness, stored ID links and rename resolution. |
| F8 | Implemented | flowrite package/executable/data names, old library migration, explicit isolation environment variable. Legacy extensions remain supported. |
| F9 | Implemented | Caret retained by undo/redo snapshots and restored focus. |
| G1 | Partial | Shared cursor helper, buttons/cards/menus/settings covered, disabled/text cursor tests and source lint. Full audit of every dropdown/slider and drag-handle grab states remains. |
| A1 | Partial | Shared page menus, duplicate/move/export/delete, folder actions, safe confirmations. Rename uses a dialog rather than an inline edit. |
| A2 | Implemented | Last tab returns home; Ctrl+W and middle-click close. |
| A3 | Implemented via approved package route | Uicons Regular Rounded vectors from official npm package 3.3.1 converted to embedded SVGs. Shared themed/DPI-cached controls, menu/slash/callout/tab icons, red trashcan + Delete, attribution and license in About. Raw vendor artwork excluded from Git. No website download automation. |
| A4 | Partial | Preset frames, file/drop/clipboard import, cover crop/shapes/captions, file-backed autosave/undo, editable bundles, PDF/Markdown image exports. Slash command currently inserts Inline directly, and icon/handle-menu polish remains. |
| A5 | Implemented | New-folder/Blank/MLA/APA start cards; page cards retained inside folders. |
| A6 | Partial | Visual-edge arrows, whole-block images/dividers, text merges preserving styles/Unicode, text-block jump/move shortcuts. Cross-block Shift-selection and full multi-selection keyboard behavior remain. |
| A7 | Implemented | Focus-only empty hints, no bottom continuation text, blank empty previews. |
| A8 | Partial | Bundled offline nlprule + spellbook, background/debounced changed-block checking, underlines, suggestions, persisted ignore/dictionary controls, highlight-only and F7. Hover explanation, temporary Ignore once and toolbar switch remain. |
| A9 | Partial | Ctrl+K searchable actions/documents/folders, keyboard/mouse selection, settings commands and confirmation-only delete. Full dimmed modal/outside-dismiss behavior, search-only mode, body-ranked results, inline folder prompt and icons remain. |
| A10 | Partial | Selection/block colors/highlights, block and dark page backgrounds, reset formatting, color export opt-in. Custom wheel/hex/recent colors and context-menu background control remain. |
| A11 | Partial | Real container/header, language, wrap/scroll, Copy feedback, code keyboard behavior, monospace boxed PDF and safe fenced Markdown. Syntax highlighting is B17; header alignment/scroll polish remains. |
| A12 | Partial | Six-level list indentation, level-specific markers, parent/children moves, empty-item outdent, nested Markdown/PDF. Multi-block selection indentation remains. |
| A13 | Partial | Distinct containers/bar/tints, types and aliases, multiline rich text/exit, PDF container/Markdown alerts, licensed leading type-picker icons. Type-picker-on-insert and rich nested lists remain. |

## Settings

Typed versioned settings, atomic persistence, recovery, full-page categories,
search, immediate application and per-section defaults are implemented.

| Section | Status | Remaining details |
| --- | --- | --- |
| S1 General | Mostly implemented | Explicit startup choice wording and additional cleanup edge-case tests. |
| S2 Appearance | Partial | Remembering sidebar expansion preference, reduce motion. |
| S3 Editor | Partial | Smart quotes/dashes/capitalization, list/typing preferences, cursor style, typewriter scrolling, page-break lines, zoom defaults, default callout choice. |
| S4 Proofing | Partial | Confirmed clear-all lists, editable dictionary entries, additional skip/category controls, disabled language dropdown. |
| S5 Saving/backups | Partial | Autosave/indicator/path exist; history, Trash, backups, ZIP import/export, relocation remain. |
| S6 Citations | Partial | Heading/sort/indent/running header exist; default style/source/access-date and generator remain. |
| S7 Export | Partial | Letter/A4, image and number switches, PDF colors, notifications/auto-open exist; default format and remaining Markdown/font/toast options remain. |
| S8 Shortcuts | Reference only | Rebinding, conflict resolution and searchable complete action table remain. |
| S9 Privacy/data | Partial | Offline privacy statement exists; logs, update preferences and confirmed data clearing remain. |
| S10 About | Partial | Version/links/diagnostics, bundled font/dictionary/LGPL notices and generated dependency notice export exist. Build date, update check, cargo-about/license integration and icon credits remain. |

## Visible bugs and additional features

C1-C4 and C6-C10 have implementation coverage: branding, lazy pages, safer export
names, heading defaults/date, timed export notifications, relative edits,
Markdown spacing, batched PDF layout, exact find highlights. C5 has grouped
sidebar folders and a level-indented outline; current-section tracking remains.

| Feature | Status |
| --- | --- |
| B1 Stats/goal | Partial: words, characters, reading time, selected words, goal; academic page count pending. |
| B2 Citation generator | Pending. Existing ready-made citation input retained. |
| B3 Outline | Partial: heading navigation/indent; section tracking/drag reorder pending. |
| B4 Find/replace | Implemented case/whole-word, count, exact highlights and replacements; keyboard traversal polish pending. |
| B5 History/Trash | Pending; previous-file .bak copies are not full version history. |
| B6 Backup/import | Pending. |
| B7 DOCX/Print | Pending; export folder is remembered. |
| B8 Link autocomplete/backlinks | Pending; stable IDs are implemented. |
| B9 Favorites/sorting | Pending. |
| B10 Tags/search | Partial: title/body search; tags/snippets/filters pending. |
| B11 Tables | Pending. |
| B12 Footnotes | Pending. |
| B13 Text tools | Partial: existing bold/italic/underline/spacing/quotes/dividers; other styles and URL insertion pending. |
| B14 Drag/multi-block | Pending. |
| B15 Focus/zoom | Partial: focus hides tabs/toolbar/side panels; paragraph dimming and document zoom pending. |
| B16 Tabs | Partial: close/cycle/jump/reopen shortcuts; drag order pending. |
| B17 Syntax highlighting | Pending. |
| B18 Templates | Pending beyond Blank/MLA/APA. |
| B19 Dictionary/thesaurus | Pending. |
| B20 | Covered by Settings, with the gaps above. |
| B21 Accessibility | Partial: scale/contrast, existing focus rings and labels; full audit pending. |
| B22 Updates | Pending; no runtime network requests. |

## Verification

The 0.2.4 code-block Copy fix passed all **66 tests**. Three new tests activate
the actual Copy control by mouse and keyboard, check exact Unicode/whitespace
clipboard output, repeated and empty copies, per-block confirmation, and its
expiry. The timestamp is read before taking the context data write lock; there
is no nested context access during the update. Clipboard output is verified
through egui's platform output, not a native Windows clipboard inspection.

The release build-asset ZIP has been checked against its archive checksum and
all 25 member checksums. Five setup tests cover an isolated install, altered
archives/files, unexpected entries, and path traversal. The source repository
documents a one-time verified setup; the executable still runs offline.

The 0.2.3 icon-only update passed all **63 tests**, including checks that header
actions do not draw immediate labels and hovering reveals the action tooltip.
Menus remain click-to-open. Current font/format values, menu choices, and
confirmation labels remain visible. The tests use workspace-local temporary
fixtures; no live library or settings files were changed.

The 0.2.2 style update passed all **61 tests**, including validation of every
bundled SVG and its provenance, texture reuse, disabled-button cursors, mouse
and keyboard activation, and actual menu rendering. Fresh offscreen previews of
the editor, home, Settings, page actions, slash commands, and delete confirmation
were generated and checked. This does not claim a native hands-on usability run.

The 0.2.0 test run passed all **55 tests**. Formatting and whitespace checks also
passed. Release builds use optimization and link-time optimization.

The 0.2.1 test run passed all **58 tests**. The compatibility patch additionally tests indexes without the persisted ID
counter, optional older metadata, and stale counters. It preserves the loaded
pages, folders, tabs, and citations, and keeps the previous index on save. The
reported screenshot was used to reproduce the error with isolated fixtures;
direct read access to the user's live library was unavailable. No live library
or settings files were changed during this verification.

Automated tests exercise persistence/migration/recovery, compatibility, stable
links, rich Unicode editing, draft/tab behavior, placeholders, cursor rules,
layout/style runs, image bundles, export naming, list/image/container exports,
and real bundled offline proofing including highlight-only mode.

Light and dark PDF fixtures were rendered and visually inspected. Offscreen previews use the
actual egui panels/fonts with isolated synthetic documents. Native test windows
were inaccessible to the UI checker; the real user library was not used for UI
testing. All test instances were closed with user approval. A new interactive
native usability rating is therefore **not claimed**.

The offscreen page-transition test verifies that the logo remains visible on the
home, document, and Settings screens. The user's real library was not opened or
modified during verification.
