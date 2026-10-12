//! Isolated offscreen QA: use the real egui panels and fonts without opening
//! native windows or loading the user's library. PNG output is opt-in.
use super::*;
use std::collections::HashMap;

fn rasterize(
    ctx: &egui::Context,
    output: egui::FullOutput,
    textures: &mut HashMap<egui::TextureId, image::RgbaImage>,
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
    let mut canvas = image::RgbaImage::from_pixel(1280, 820, image::Rgba([24, 25, 28, 255]));
    for primitive in ctx.tessellate(output.shapes, 1.0) {
        let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else {
            continue;
        };
        let Some(texture) = textures.get(&mesh.texture_id) else {
            continue;
        };
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
                    .intersect(primitive.clip_rect)
                    .intersect(egui::Rect::from_min_max(
                        Pos2::ZERO,
                        Pos2::new(1280.0, 820.0),
                    ));
            if bounds.is_negative() {
                continue;
            }
            for y in bounds.min.y.max(0.0) as u32..bounds.max.y.ceil().min(820.0) as u32 {
                for x in bounds.min.x.max(0.0) as u32..bounds.max.x.ceil().min(1280.0) as u32 {
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
    let mut app = FlowriteApp::empty();
    app.workspace.directory = directory.join("isolated-library");
    app.title = "A quiet place to write".into();
    app.blocks=vec![Block::new(1,BlockKind::Heading1,"Ideas deserve room to grow"),Block::new(2,BlockKind::Text,"Lorem ipsum dolor sit amet, consectetur adipiscing elit. A thoughtful workspace keeps the focus on your words, with useful tools just a click away."),Block::new(3,BlockKind::Callout,"A useful note stands apart from the rest of the page."),Block::new(4,BlockKind::Code,"fn main() {\n    println!(\"Hello, flowrite!\");\n}"),Block::new(5,BlockKind::Text,"")];
    app.blocks[3].language = "rust".into();
    let id = app.workspace.add(app.local_document(), None);
    app.active_document = id.clone();
    let folder = app.workspace.create_folder("Writing").unwrap();
    app.workspace.move_document(&id, Some(folder));
    let ctx = egui::Context::default();
    configure_fonts(&ctx);
    app.apply_settings(&ctx);
    let mut textures = HashMap::new();
    for mode in [
        "home",
        "editor",
        "settings",
        "page-actions",
        "delete-confirmation",
        "slash-menu",
    ] {
        app.show_home = mode == "home";
        app.settings_open = mode == "settings";
        app.dialogs.delete = if mode == "delete-confirmation" {
            Some(context_menu::PageAction::Delete(id.clone()))
        } else {
            None
        };
        app.slash_target = if mode == "slash-menu" { Some(4) } else { None };
        if mode == "slash-menu" {
            app.blocks[4].text = "/".into();
            app.command_position = Pos2::new(460.0, 360.0);
            app.command_query = "/".into();
        }
        for pass in 0..12 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        Pos2::ZERO,
                        Vec2::new(1280.0, 820.0),
                    )),
                    ..Default::default()
                },
                |ctx| {
                    if app.settings_open {
                        app.settings_page(ctx);
                    } else if app.show_home {
                        app.home_header(ctx);
                        app.home_page(ctx);
                    } else {
                        app.header(ctx);
                        app.document_tabs(ctx);
                        app.toolbar(ctx);
                        app.status_bar(ctx);
                        app.library_sidebar(ctx);
                        app.canvas(ctx);
                        if mode == "page-actions" {
                            egui::Window::new("Page actions")
                                .default_pos(Pos2::new(750.0, 210.0))
                                .show(ctx, |ui| {
                                    context_menu::page_menu(ui, &id, &app.workspace);
                                });
                        }
                        if mode == "delete-confirmation" {
                            app.library_dialogs(ctx);
                        }
                        if mode == "slash-menu" {
                            app.popups(ctx);
                        }
                    }
                },
            );
            let image = rasterize(&ctx, output, &mut textures);
            if pass == 11 {
                let logo_pixels = (10..60)
                    .flat_map(|y| (10..60).map(move |x| (x, y)))
                    .filter(|&(x, y)| {
                        let p = image.get_pixel(x, y);
                        p[0] > 205 && p[1] > 215 && p[2] > 200
                    })
                    .count();
                assert!(
                    logo_pixels > 200,
                    "The top-left logo must remain visible on {mode}"
                );
            }
            image.save(directory.join(format!("{mode}.png"))).unwrap();
        }
    }
}
