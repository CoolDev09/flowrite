use super::*;

#[test]
fn citation_names_are_shared_for_every_format_and_override() {
    let mut app = FlowriteApp::empty();
    for (format, heading) in [
        (PaperFormat::Freeform, "Works Cited"),
        (PaperFormat::Mla, "Works Cited"),
        (PaperFormat::Apa, "References"),
    ] {
        app.style.paper_format = format;
        app.citations = vec!["Example, Author. Test.".into()];
        assert_eq!(app.citations_heading(), heading);
        assert!(app.markdown().contains(&format!("## {heading}")));
    }
    app.settings.citations.heading = "Sources & notes".into();
    assert_eq!(app.citations_heading(), "Sources & notes");
    assert!(app.markdown().contains("## Sources \\& notes"));
}

#[test]
fn temporary_draft_has_a_visible_tab_without_a_library_record() {
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    let mut app = FlowriteApp::empty();
    app.start_fresh();
    let output = ctx.run(egui::RawInput::default(), |ctx| app.document_tabs(ctx));
    assert!(output.shapes.iter().any(
        |s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.job.text.contains("Untitled"))
    ));
    app.close_temporary_draft();
    assert!(app.workspace.documents.is_empty());
    assert!(app.show_home);
}

#[test]
fn first_draft_edit_preserves_title_or_body_focus_and_following_typing() {
    for title in [true, false] {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);
        let mut app = FlowriteApp::empty();
        app.workspace.directory =
            std::env::temp_dir().join(format!("draft-focus-{}-{title}", workspace::timestamp()));
        app.start_fresh();
        app.focus_request = None;
        let block_id = app.blocks[0].id;
        let old = if title {
            egui::Id::new(("document_title", &app.active_document))
        } else {
            egui::Id::new(("block", &app.active_document, block_id))
        };
        let frame = |app: &mut FlowriteApp, events| {
            let _ = ctx.run(
                egui::RawInput {
                    events,
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(1280.0, 820.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    app.workspace_view(ctx);
                    app.record_ui_edits(ctx, 1.0);
                },
            );
        };
        frame(&mut app, vec![]);
        ctx.memory_mut(|m| m.request_focus(old));
        frame(&mut app, vec![]);
        assert!(
            ctx.memory(|m| m.has_focus(old)),
            "test must focus the intended editor first: title={title}, focused={:?}",
            ctx.memory(|m| m.focused())
        );
        frame(&mut app, vec![egui::Event::Text("é".into())]);
        assert!(
            !app.active_document.is_empty(),
            "edit was not delivered: title={title}, contents={:?}/{:?}",
            app.title,
            app.blocks[0].text
        );
        let target = if title {
            egui::Id::new(("document_title", &app.active_document))
        } else {
            egui::Id::new(("block", &app.active_document, block_id))
        };
        assert!(ctx.memory(|m| m.has_focus(target)));
        frame(&mut app, vec![egui::Event::Text("x".into())]);
        assert!(if title {
            app.title.contains("éx")
        } else {
            app.blocks[0].text == "éx"
        });
    }
}

/// Optional cross-parser fixtures and real export artifacts, isolated from user files.
#[test]
fn generate_review_export_fixtures() {
    let Some(directory) = std::env::var_os("FLOWRITE_REVIEW_EXPORT_DIR").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&directory).unwrap();
    let mut app = FlowriteApp::empty();
    app.title = "Literal *title* [x] _and_ `code`".into();
    app.blocks = vec![Block::new(1, BlockKind::Text, "Symbols: *literal asterisks* [brackets] #heading _under_scores_ \\ path.\n1. literal\n> quote\n``` fence"), Block::new(2, BlockKind::Text, "Bold italic underline")];
    app.blocks[1].marks = vec![
        FormatSpan {
            start: 0,
            end: 4,
            style: InlineStyle {
                bold: true,
                ..Default::default()
            },
        },
        FormatSpan {
            start: 5,
            end: 11,
            style: InlineStyle {
                italic: true,
                ..Default::default()
            },
        },
        FormatSpan {
            start: 12,
            end: 21,
            style: InlineStyle {
                underline: true,
                ..Default::default()
            },
        },
    ];
    std::fs::write(directory.join("literal.md"), app.markdown()).unwrap();
    app.title = "List hierarchy".into();
    app.blocks.clear();
    for number in 1..=12 {
        app.blocks.push(Block::new(
            number,
            BlockKind::Numbered,
            &format!("Parent {number}"),
        ));
        if number == 1 || number == 10 {
            let mut child = Block::new(
                100 + number,
                BlockKind::Numbered,
                &format!("Child {number}\nContinuation {number}"),
            );
            child.indent = 1;
            app.blocks.push(child);
            let mut bullet = Block::new(200 + number, BlockKind::Bulleted, "Mixed bullet");
            bullet.indent = 2;
            app.blocks.push(bullet);
        }
    }
    std::fs::write(directory.join("lists.md"), app.markdown()).unwrap();
    if let Some(path) = std::env::var_os("FLOWRITE_REVIEW_FIXTURE") {
        let (document, assets) = bundle::read(Path::new(&path)).unwrap();
        app.workspace.directory = directory.join("library");
        app.active_document = app.workspace.add(document.clone(), None);
        app.title = document.title;
        app.blocks = document.blocks;
        app.style = document.style;
        app.details = document.details;
        app.citations = document.citations;
        for (name, bytes) in assets {
            workspace::write_atomic(
                &images::asset_directory(&app.workspace, &app.active_document).join(name),
                &bytes,
            )
            .unwrap();
        }
        app.write_pdf(&directory.join("mixed-blocks.pdf")).unwrap();
        app.style.apply_preset(PaperFormat::Mla);
        app.details.author = "QA Author".into();
        app.details.last_name = "Author".into();
        app.write_pdf(&directory.join("academic-mla.pdf")).unwrap();
        app.write_markdown(&directory.join("mixed-blocks.md"))
            .unwrap();
        if let Some(native_dir) = std::env::var_os("FLOWRITE_NATIVE_QA_DIR").map(PathBuf::from) {
            let mut native = FlowriteApp::empty();
            native.workspace.directory = native_dir;
            native.start_fresh();
            native.title = "Code focus QA".into();
            native.blocks = vec![Block::new(1, BlockKind::Code, "print(\"café Ω\")\n")];
            native.dirty = true;
            assert!(native.save_workspace());
            let code_id = native.active_document.clone();
            native.start_fresh();
            native.title = "List focus QA".into();
            native.blocks = vec![
                Block::new(1, BlockKind::Bulleted, "Parent café Ω"),
                Block::new(2, BlockKind::Todo, "Unchecked task"),
                Block::new(3, BlockKind::Todo, "Completed task"),
            ];
            native.blocks[2].checked = true;
            native.dirty = true;
            assert!(native.save_workspace());
            if let Some(image) = app
                .blocks
                .iter()
                .find(|b| b.kind == BlockKind::Image)
                .cloned()
            {
                native.start_fresh();
                native.title = "Images and captions QA".into();
                native.blocks = vec![image];
                native.dirty = true;
                native.commit_current();
                if let Some(asset) = native.blocks[0].image.as_ref() {
                    let bytes = std::fs::read(
                        images::asset_directory(&app.workspace, &app.active_document)
                            .join(&asset.name),
                    )
                    .unwrap();
                    workspace::write_atomic(
                        &images::asset_directory(&native.workspace, &native.active_document)
                            .join(&asset.name),
                        &bytes,
                    )
                    .unwrap();
                }
                assert!(native.save_workspace());
            }
            native.switch_document(&code_id);
            native.settings.editor.toolbar = true;
            native.settings.proofing.spelling = false;
            native.settings.proofing.grammar = false;
            native.settings.save(&native.workspace.directory).unwrap();
        }
    }
}
