# 0.3.0 review implementation

Scope: all seven bugs and sixteen improvements in the supplied Flowrite 0.2.4 review. Original review fixtures, evidence, synced project sources, and the user's library are unchanged.

## Implementation checklist

| ID | Change | Verification |
| --- | --- | --- |
| B01 | Code/list Tab and Shift+Tab retain editing focus and caret; draft caret transfers on first save | Actual TextEdit event regressions and native Tab/Shift+Tab/Undo/typing checks pass |
| B02 | Page title wraps; cramped detail panels overlay rather than consume writing width | Default/minimum 100/125/150% wrapping and overlay checks pass |
| B03 | Settings search derives its index from real labels/descriptions and matches every query word | Actual form search test passes |
| B04 | Literal Markdown punctuation escaped independently of deliberate rich styles/links | Marked-rendered text and style assertions pass |
| B05 | Child list indentation follows parent content columns, including multi-digit markers and multiline/mixed lists | Parsed hierarchy assertions pass |
| B06 | Shared citation heading for sidebar, panel, content and export; APA References/custom override | Format/override regression passes |
| B07 | Named selectors, spinners, task checkboxes, color controls, captions; preserved native values/state and label relationships | Native names/value and keyboard checks pass; no full screen-reader certification |
| I01 | Shared compact shell and stable Home/folder/document navigation | UI matrix |
| I02 | Optional compact formatting bar, contextual selection toolbar, subtle save state, one stats footer, help on demand | UI matrix |
| I03 | New notes default to sans-serif 12 pt/16 px with comfortable spacing; academic content keeps presets | Defaults/preset tests |
| I04 | Shared neutral surfaces, spacing, control heights, truncated navigation/full tooltips, wrapping page title | UI matrix |
| I05 | Compact creation/templates; readable list/gallery previews; visible sort/search/view controls | UI matrix |
| I06 | Consistent folder hierarchy and separate outline; direct Move to folder picker | UI matrix/native check |
| I07 | Unified 17 px outline icons, action labels, disabled treatment, tooltips | Icon provenance/parsing/control tests; mapping below |
| I08 | Quiet selection fill, hover treatment, explicit keyboard focus ring; completed tasks muted/struck | UI matrix/native check |
| I09 | Contextual block handles/formatting; grouped proofing and block menus; restrained code header | Menu/code tests and UI matrix |
| I10 | Responsive label/control form rows; scrollable categories/content; separate maintenance; actual matching controls | Search test and scale matrix |
| I11 | Brief dismissible notifications, optional full details, plain-language failed-import reassurance; shared popover density | Notification/native check |
| I12 | Exact paginated PDF preview opens in the system PDF viewer; templates explicitly describe layout defaults, not automatic academic compliance | PDF QA passes; native preview opens in the system viewer |
| I13 | Minimum writing-width policy, overlay details, expand-writing action, toolbar scrolling, wrapped Find/Replace | Default/minimum 100/125/150% matrix |
| I14 | Immediate temporary draft tab, close inside tab, full-title tooltips, overflow menu | Temporary-tab/no-empty-record test passes |
| I15 | English (US)/severity labels, compact writing-review window, accessible Ignore/Dictionary actions, quieter default marks | Proofing engine tests/native check |
| I16 | Image and caption centered together; vector task boxes and quote rules; mono code and tinted callout print equivalents | Every page visually inspected; image/caption centers and full text preservation verified |

## Action-to-icon mapping

All toolbar icons are 17 px from Flaticon Uicons Regular Rounded; buttons use a shared 32 px target. Ambiguous actions have readable menu labels and icon-only actions expose accessible names and full tooltips.

| Action | Icon |
| --- | --- |
| Home / Search / Settings | home / search / settings |
| New page / New folder | plus / folder |
| File menu / Open / Save | document / folder-open / disk |
| Undo / Redo | arrow-left / arrow-right |
| Formatting / Color / Line spacing | text / palette / interlining |
| Image / Citations / Page setup | picture / book / layout-fluid |
| Paper preview / Export | file-pdf / file-export |
| More / Rename / Duplicate / Delete | menu-dots / pencil / copy / trash |
| List / Gallery view | list / apps |
| Close / Confirmation | cross / check |

## Print presentation

Default PDF is black on white. Quotes receive an inset vertical rule; tasks receive vector empty/checked boxes; code uses Liberation Mono in a padded neutral container; callouts retain a restrained semantic side rule. Images and captions share the page center. Existing optional page-color export remains available. Preview exports the same current in-memory snapshot without saving or changing the document.

## Verification status

All 23 review items have implementation coverage in the local 0.3.0 build. Verification: 74 automated tests pass; the opt-in matrix rendered 60 real egui views (Home/list/gallery/folders/drafts/mixed documents/Settings/search/menus/APA, at default/minimum sizes and 100/125/150%). The nine Settings categories also pass control-bound checks at each scale. Every page of the six-page mixed and six-page MLA PDFs was visually inspected; all fixture block text is retained, and image/caption centers match. Marked parsing asserts literal punctuation, deliberate rich styles, ordered hierarchy, multi-digit parents, mixed lists and multiline children.

Native Windows checks confirmed code Tab + next character, fast Shift+Tab, Undo and retained caret; list Tab + next character and persisted indent; named block/font selectors, font-size value, code-language control, task checkboxes, and Spelling color search/control. Final-build task and caption names, list outdent persistence, Undo caret, and system PDF preview launch were also verified. A full screen-reader session, multi-monitor audit, large-library benchmark and complete academic compliance certification were not performed. These are testing limits, not claims that every conceivable variation is covered.

Optimized Windows build SHA-256: `c97f539a343054ac3152c761f5d89c08aa459919aadfe07735c14d4c1806ba4a`. Version 0.3.0 is kept local at the user's request; no GitHub release, commit, tag or push was made.
