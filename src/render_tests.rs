//! Isolated offscreen QA: use the real egui panels and fonts without opening
//! native windows or loading the user's library. PNG output is opt-in.
use super::*;
use std::collections::HashMap;

fn rasterize(
    ctx: &egui::Context,
    output: egui::FullOutput,
    textures: &mut HashMap<egui::TextureId, image::RgbaImage>,
    pixels: Vec2,
) -> image::RgbaImage {
    for (id, delta) in output.textures_delta.set {
        let [width, height] = delta.image.size();
        let pixels: Vec<_> = match delta.image {
            egui::ImageData::Color(image) => image.pixels.clone(),
            egui::ImageData::Font(ref image) => image.srgba_pixels(None).collect(),
        };
        let bytes: Vec<_> = pixels.iter().flat_map(|color| color.to_array()).collect();
        let update = image::RgbaImage::from_raw(width as u32, height as u32, bytes).unwrap();
        if let Some([x, y]) = delta.pos {
            image::imageops::replace(textures.get_mut(&id).unwrap(), &update, x as i64, y as i64);
        } else {
            textures.insert(id, update);
        }
    }
    let mut canvas = image::RgbaImage::from_pixel(
        pixels.x as u32,
        pixels.y as u32,
        image::Rgba([25, 25, 25, 255]),
    );
    let ppp = ctx.pixels_per_point();
    for primitive in ctx.tessellate(output.shapes, ppp) {
        let egui::epaint::Primitive::Mesh(mut mesh) = primitive.primitive else {
            continue;
        };
        let Some(texture) = textures.get(&mesh.texture_id) else {
            continue;
        };
        for vertex in &mut mesh.vertices {
            vertex.pos = Pos2::new(vertex.pos.x * ppp, vertex.pos.y * ppp);
        }
        let clip =
            egui::Rect::from_min_max(primitive.clip_rect.min * ppp, primitive.clip_rect.max * ppp);
        for triangle in mesh.indices.chunks_exact(3) {
            let [a, b, c] = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let cross = |a: Vec2, b: Vec2| a.x * b.y - a.y * b.x;
            let area = cross(b.pos - a.pos, c.pos - a.pos);
            if area.abs() < 0.0001 {
                continue;
            }
            let bounds =
                egui::Rect::from_min_max(a.pos.min(b.pos).min(c.pos), a.pos.max(b.pos).max(c.pos))
                    .intersect(clip)
                    .intersect(egui::Rect::from_min_max(
                        Pos2::ZERO,
                        Pos2::new(pixels.x, pixels.y),
                    ));
            if bounds.is_negative() {
                continue;
            }
            for y in bounds.min.y.max(0.0) as u32..bounds.max.y.ceil().min(pixels.y) as u32 {
                for x in bounds.min.x.max(0.0) as u32..bounds.max.x.ceil().min(pixels.x) as u32 {
                    let p = Pos2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let wa = cross(b.pos - p, c.pos - p) / area;
                    let wb = cross(c.pos - p, a.pos - p) / area;
                    let wc = 1.0 - wa - wb;
                    if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                        continue;
                    }
                    let uv = a.uv.to_vec2() * wa + b.uv.to_vec2() * wb + c.uv.to_vec2() * wc;
                    let tx = (uv.x * texture.width() as f32 - 0.5)
                        .clamp(0.0, texture.width() as f32 - 1.0);
                    let ty = (uv.y * texture.height() as f32 - 0.5)
                        .clamp(0.0, texture.height() as f32 - 1.0);
                    let x0 = tx as u32;
                    let y0 = ty as u32;
                    let x1 = (x0 + 1).min(texture.width() - 1);
                    let y1 = (y0 + 1).min(texture.height() - 1);
                    let dx = tx.fract();
                    let dy = ty.fract();
                    let sampled = [
                        (x0, y0, (1.0 - dx) * (1.0 - dy)),
                        (x1, y0, dx * (1.0 - dy)),
                        (x0, y1, (1.0 - dx) * dy),
                        (x1, y1, dx * dy),
                    ];
                    let mut source = [0.0; 4];
                    for channel in 0..4 {
                        let tint = a.color.to_array()[channel] as f32 * wa
                            + b.color.to_array()[channel] as f32 * wb
                            + c.color.to_array()[channel] as f32 * wc;
                        source[channel] = sampled
                            .iter()
                            .map(|&(x, y, w)| texture.get_pixel(x, y)[channel] as f32 * w)
                            .sum::<f32>()
                            * tint
                            / 255.0;
                    }
                    let dest = canvas.get_pixel_mut(x, y);
                    for channel in 0..3 {
                        dest[channel] = (source[channel]
                            + dest[channel] as f32 * (1.0 - source[3] / 255.0))
                            .clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
    }
    canvas
}

#[test]
fn isolated_ui_panels_render_without_loading_user_data() {
    let Some(directory) = std::env::var_os("FLOWRITE_UI_REVIEW_DIR").map(PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&directory).unwrap();
    for pixels in [Vec2::new(1280.0, 820.0), Vec2::new(900.0, 600.0)] {
        for scale in [1.0, 1.25, 1.5] {
            for mode in [
                "home",
                "gallery",
                "folder",
                "draft",
                "mixed",
                "settings",
                "settings-search",
                "page-actions",
                "slash-menu",
                "apa",
                "follow-up",
            ] {
                let mut app = FlowriteApp::empty();
                app.workspace.directory = directory.join("isolated-library");
                app.title =
                    "QA Mixed Blocks — import/export with a long readable page title".into();
                app.blocks = vec![Block::new(1, BlockKind::Heading1, "Ideas deserve room to grow"), Block::new(2, BlockKind::Text, "A thoughtful workspace keeps the focus on your words, with useful tools just a click away. Café naïve — Ω."), Block::new(3, BlockKind::Callout, "A useful note stands apart from the rest of the page."), Block::new(4, BlockKind::Code, "fn main() {\n    println!(\"Hello, flowrite!\");\n}"), Block::new(5, BlockKind::Todo, "Completed task"), Block::new(6, BlockKind::Text, "")];
                app.blocks[3].language = "rust".into();
                app.blocks[4].checked = true;
                if mode == "follow-up" {
                    app.title = "Layout refinements".into();
                    app.blocks = vec![
                        Block::new(1, BlockKind::Quote, "Test\nfdsn\nfndslkn"),
                        Block::new(
                            2,
                            BlockKind::Code,
                            "print(\"café Ω\")\n    print(\"second line\")",
                        ),
                    ];
                }
                let id = app.workspace.add(app.local_document(), None);
                app.active_document = id.clone();
                let folder = app.workspace.create_folder("Writing").unwrap();
                app.workspace.move_document(&id, Some(folder.clone()));
                app.show_home = matches!(mode, "home" | "gallery" | "folder");
                app.settings.appearance.list_view = mode != "gallery";
                app.settings.appearance.scale = scale;
                app.settings_open = mode.starts_with("settings");
                app.settings_section = 1;
                if mode == "settings-search" {
                    app.settings_query = "spelling color".into();
                }
                if mode == "folder" {
                    app.current_folder = Some(folder);
                }
                if mode == "draft" {
                    app.start_fresh();
                }
                if mode == "mixed" {
                    app.show_citations = true;
                    app.citations = vec!["Example, Author. A source used in this document.".into()];
                }
                if mode == "apa" {
                    app.style.apply_preset(PaperFormat::Apa);
                    app.show_inspector = true;
                }
                if mode == "slash-menu" {
                    app.slash_target = Some(5);
                    app.blocks[5].text = "/".into();
                    app.command_position = Pos2::new(310.0, 280.0);
                    app.command_query = "/".into();
                }
                let ctx = egui::Context::default();
                configure_fonts(&ctx);
                app.apply_settings(&ctx);
                let mut textures = HashMap::new();
                let mut final_output = None;
                for _ in 0..6 {
                    final_output = Some(ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                Pos2::ZERO,
                                pixels / scale,
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            app.workspace_view(ctx);
                            if mode == "page-actions" {
                                egui::Window::new("Page actions")
                                    .default_pos(Pos2::new(300.0, 120.0))
                                    .show(ctx, |ui| {
                                        context_menu::page_menu(ui, &id, &app.workspace);
                                    });
                            }
                            if mode == "slash-menu" {
                                app.popups(ctx);
                            }
                        },
                    ));
                    // Keep texture deltas from every settling pass, but rasterize only the last.
                    if let Some(output) = &mut final_output {
                        for (texture, delta) in output.textures_delta.set.drain(..) {
                            let [w, h] = delta.image.size();
                            let colors: Vec<_> = match delta.image {
                                egui::ImageData::Color(image) => image.pixels.clone(),
                                egui::ImageData::Font(ref image) => {
                                    image.srgba_pixels(None).collect()
                                }
                            };
                            let bytes: Vec<_> = colors.iter().flat_map(|c| c.to_array()).collect();
                            let update =
                                image::RgbaImage::from_raw(w as u32, h as u32, bytes).unwrap();
                            if let Some([x, y]) = delta.pos {
                                image::imageops::replace(
                                    textures.get_mut(&texture).unwrap(),
                                    &update,
                                    x as i64,
                                    y as i64,
                                );
                            } else {
                                textures.insert(texture, update);
                            }
                        }
                    }
                }
                let output = final_output.unwrap();
                if mode == "mixed" {
                    let title = output
                        .shapes
                        .iter()
                        .find_map(|s| match &s.shape {
                            egui::Shape::Text(t)
                                if t.galley.job.text == app.title
                                    && t.galley
                                        .job
                                        .sections
                                        .first()
                                        .is_some_and(|s| s.format.font_id.size > 25.0) =>
                            {
                                Some(t)
                            }
                            _ => None,
                        })
                        .expect("full title is painted");
                    assert!(
                        !title.galley.elided,
                        "title must wrap rather than truncate at {pixels:?} / {scale}"
                    );
                    assert!(
                        title.pos.x + title.galley.rect.right() < ctx.screen_rect().right(),
                        "title extends outside screen"
                    );
                }
                let image = rasterize(&ctx, output, &mut textures, pixels);
                image
                    .save(directory.join(format!(
                        "{mode}-{}-{}.png",
                        pixels.x as u32,
                        (scale * 100.0) as u32
                    )))
                    .unwrap();
            }
        }
    }
}
