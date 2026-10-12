use super::*;
use image::DynamicImage;
use serde::{Deserialize, Serialize};

/// Apply the same center-cover crop and shape mask used by the editor. Keeping
/// this in one place makes standalone Markdown assets and print exports agree.
pub(super) fn framed_image(path: &Path, frame: ImageFrame) -> Result<image::RgbaImage, String> {
    let source = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
    let natural = source.width() as f32 / source.height() as f32;
    let ratio = frame.ratio(natural);
    let (width, height) = if natural > ratio {
        (
            (source.height() as f32 * ratio).round() as u32,
            source.height(),
        )
    } else {
        (
            source.width(),
            (source.width() as f32 / ratio).round() as u32,
        )
    };
    let width = width.max(1).min(source.width());
    let height = height.max(1).min(source.height());
    let mut output = image::imageops::crop_imm(
        &source,
        (source.width() - width) / 2,
        (source.height() - height) / 2,
        width,
        height,
    )
    .to_image();
    if matches!(frame, ImageFrame::Circle | ImageFrame::Rounded) {
        let radius = if frame == ImageFrame::Circle {
            width.min(height) as f32 / 2.0
        } else {
            width.min(height) as f32 * 0.04
        };
        for (x, y, pixel) in output.enumerate_pixels_mut() {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let dx = px - px.clamp(radius, width as f32 - radius);
            let dy = py - py.clamp(radius, height as f32 - radius);
            if dx * dx + dy * dy > radius * radius {
                pixel[3] = 0;
            }
        }
    }
    Ok(output)
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) enum ImageFrame {
    #[default]
    Inline,
    FullWidth,
    Square,
    Ratio4x3,
    Ratio16x9,
    Circle,
    Rounded,
}
impl ImageFrame {
    pub const ALL: [Self; 7] = [
        Self::Inline,
        Self::FullWidth,
        Self::Square,
        Self::Ratio4x3,
        Self::Ratio16x9,
        Self::Circle,
        Self::Rounded,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Inline => "Inline",
            Self::FullWidth => "Full width",
            Self::Square => "Square",
            Self::Ratio4x3 => "4:3",
            Self::Ratio16x9 => "16:9",
            Self::Circle => "Circle",
            Self::Rounded => "Rounded",
        }
    }
    pub fn ratio(self, natural: f32) -> f32 {
        match self {
            Self::Square | Self::Circle => 1.0,
            Self::Ratio4x3 => 4.0 / 3.0,
            Self::Ratio16x9 => 16.0 / 9.0,
            _ => natural,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct ImageAsset {
    pub name: String,
    pub frame: ImageFrame,
    pub caption: String,
    pub show_caption: bool,
}
pub(super) fn safe_asset(name: &str) -> bool {
    !name.is_empty()
        && name.ends_with(".png")
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        && !name.contains("..")
}
pub(super) fn asset_directory(workspace: &Workspace, id: &str) -> PathBuf {
    workspace
        .directory
        .join("documents")
        .join(id)
        .join("assets")
}
impl FlowriteApp {
    pub(super) fn image_button(&mut self, ui: &mut egui::Ui) {
        icons::menu(ui, Icon::Image, "", |ui| {
            for frame in ImageFrame::ALL {
                if ui.button(frame.label()).hand().clicked() {
                    let index = self
                        .focused_block
                        .and_then(|id| self.blocks.iter().position(|b| b.id == id))
                        .unwrap_or(self.blocks.len() - 1);
                    let mut block = self.make_block(BlockKind::Image, "");
                    block.image = Some(ImageAsset {
                        frame,
                        ..Default::default()
                    });
                    let id = block.id;
                    self.blocks.insert(index + 1, block);
                    self.focused_block = Some(id);
                    self.dirty = true;
                    self.force_history = true;
                    self.commit_current();
                    ui.close_menu();
                }
            }
        })
        .response
        .hand();
    }
    fn choose_image(&mut self, index: usize) {
        if let Some(path) = FileDialog::new()
            .set_title("Choose an image")
            .add_filter("Images", &["png", "jpg", "jpeg", "gif", "webp"])
            .pick_file()
        {
            match image::ImageReader::open(path)
                .map_err(|e| e.to_string())
                .and_then(|r| r.decode().map_err(|e| e.to_string()))
            {
                Ok(image) => self.import_image(index, image),
                Err(error) => self.notice = Some(format!("Could not open image: {error}")),
            }
        }
    }
    pub(super) fn import_image(&mut self, index: usize, image: DynamicImage) {
        if !self.workspace_available {
            self.notice=Some("Recover your library before importing images. Its existing files have not been changed.".into());
            return;
        }
        self.dirty = true;
        self.commit_current();
        let image = if image.width() > 2000 || image.height() > 2000 {
            image.resize(2000, 2000, image::imageops::FilterType::Lanczos3)
        } else {
            image
        };
        let name = format!("image-{}-{}.png", workspace::timestamp(), self.next_id);
        self.next_id += 1;
        let directory = asset_directory(&self.workspace, &self.active_document);
        let result = std::fs::create_dir_all(&directory)
            .map_err(|e| e.to_string())
            .and_then(|_| {
                image
                    .save_with_format(directory.join(&name), image::ImageFormat::Png)
                    .map_err(|e| e.to_string())
            });
        match result {
            Ok(()) => {
                self.blocks[index]
                    .image
                    .get_or_insert_with(Default::default)
                    .name = name;
                self.dirty = true;
                self.force_history = true;
            }
            Err(error) => self.notice = Some(format!("Could not store image: {error}")),
        }
    }
    pub(super) fn image_editor(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        index: usize,
        width: f32,
    ) -> egui::Response {
        let id = self.blocks[index].id;
        let widget = egui::Id::new(("block", &self.active_document, id));
        let asset = self.blocks[index]
            .image
            .get_or_insert_with(Default::default)
            .clone();
        let key = format!("{}/{}", self.active_document, asset.name);
        let natural = self
            .image_cache
            .get(&key)
            .map(|t| t.size_vec2().x / t.size_vec2().y)
            .or_else(|| {
                if safe_asset(&asset.name) {
                    image::image_dimensions(
                        asset_directory(&self.workspace, &self.active_document).join(&asset.name),
                    )
                    .ok()
                    .map(|(w, h)| w as f32 / h.max(1) as f32)
                } else {
                    None
                }
            })
            .unwrap_or(16.0 / 9.0);
        let ratio = asset.frame.ratio(natural).clamp(0.1, 10.0);
        let frame_width = match asset.frame {
            ImageFrame::Inline => width.min(520.0),
            ImageFrame::Circle => width.min(280.0),
            ImageFrame::Square | ImageFrame::Rounded => width.min(420.0),
            _ => width,
        };
        let (rect, _) = ui.allocate_exact_size(
            Vec2::new(frame_width, (frame_width / ratio).min(1000.0)),
            Sense::hover(),
        );
        // Decode only visible frames; page layout needs just the image header.
        if ui.is_rect_visible(rect)
            && safe_asset(&asset.name)
            && !self.image_cache.contains_key(&key)
        {
            match image::open(
                asset_directory(&self.workspace, &self.active_document).join(&asset.name),
            ) {
                Ok(image) => {
                    let image = image.to_rgba8();
                    let color = egui::ColorImage::from_rgba_unmultiplied(
                        [image.width() as usize, image.height() as usize],
                        image.as_raw(),
                    );
                    self.image_cache.insert(
                        key.clone(),
                        ctx.load_texture(&key, color, egui::TextureOptions::LINEAR),
                    );
                }
                Err(error) => self.notice = Some(format!("Missing image {}: {error}", asset.name)),
            }
        }
        let texture = self.image_cache.get(&key);
        let response = ui.interact(rect, widget, Sense::click()).hand();
        response.widget_info(|| {
            egui::WidgetInfo::labeled(
                egui::WidgetType::Button,
                true,
                if asset.name.is_empty() {
                    "Add an image"
                } else {
                    "Image frame"
                },
            )
        });
        if self.focus_request.is_some_and(|(target, _)| target == id) {
            response.request_focus();
            self.focus_request = None;
        }
        if response.has_focus() {
            ctx.memory_mut(|m| {
                m.set_focus_lock_filter(
                    widget,
                    egui::EventFilter {
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        ..Default::default()
                    },
                )
            });
            if response.gained_focus() {
                ctx.request_discard("Settle image keyboard focus");
            }
            self.focused_block = Some(id);
            self.selection = Some((id, 0, 0));
        }
        let rounding = if asset.frame == ImageFrame::Rounded {
            16.0
        } else {
            6.0
        };
        if let Some(texture) = texture {
            let uv = cover_uv(texture.size_vec2(), rect.size());
            if asset.frame == ImageFrame::Circle || asset.frame == ImageFrame::Rounded {
                let mut mesh = egui::Mesh::with_texture(texture.id());
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: rect.center(),
                    uv: uv.center(),
                    color: Color32::WHITE,
                });
                let points: Vec<_> = if asset.frame == ImageFrame::Circle {
                    (0..64)
                        .map(|n| {
                            rect.center()
                                + Vec2::angled(n as f32 * std::f32::consts::TAU / 64.0)
                                    * rect.width()
                                    / 2.0
                        })
                        .collect()
                } else {
                    let r = 16.0;
                    [
                        (rect.right() - r, rect.bottom() - r, 0.0),
                        (
                            rect.left() + r,
                            rect.bottom() - r,
                            std::f32::consts::FRAC_PI_2,
                        ),
                        (rect.left() + r, rect.top() + r, std::f32::consts::PI),
                        (
                            rect.right() - r,
                            rect.top() + r,
                            3.0 * std::f32::consts::FRAC_PI_2,
                        ),
                    ]
                    .into_iter()
                    .flat_map(|(x, y, angle)| {
                        (0..16).map(move |n| {
                            Pos2::new(x, y)
                                + Vec2::angled(
                                    angle + n as f32 * std::f32::consts::FRAC_PI_2 / 15.0,
                                ) * r
                        })
                    })
                    .collect()
                };
                for p in &points {
                    let relative = (*p - rect.min) / rect.size();
                    mesh.vertices.push(egui::epaint::Vertex {
                        pos: *p,
                        uv: uv.min + relative * uv.size(),
                        color: Color32::WHITE,
                    });
                }
                for n in 0..points.len() {
                    mesh.indices.extend_from_slice(&[
                        0,
                        (n + 1) as u32,
                        ((n + 1) % points.len() + 1) as u32,
                    ]);
                }
                ui.painter().add(egui::Shape::mesh(mesh));
            } else {
                ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
            }
        } else {
            ui.painter()
                .rect_filled(rect, rounding, theme::current().panel);
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Add an image",
                FontId::proportional(15.0),
                theme::current().muted,
            );
        }
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect,
                rounding,
                Stroke::new(2.0_f32, theme::current().accent),
            );
        }
        if response.clicked() {
            response.request_focus();
            self.focused_block = Some(id);
            if asset.name.is_empty() {
                self.choose_image(index);
            }
        }
        if response.hovered() {
            let dropped = ctx.input(|i| i.raw.dropped_files.first().cloned());
            if let Some(file) = dropped {
                let image = file
                    .path
                    .and_then(|p| image::open(p).ok())
                    .or_else(|| file.bytes.and_then(|b| image::load_from_memory(&b).ok()));
                if let Some(image) = image {
                    self.import_image(index, image);
                }
            }
        }
        if response.has_focus()
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::V))
        {
            match arboard::Clipboard::new().and_then(|mut c| c.get_image()) {
                Ok(image) => {
                    if let Some(image) = image::RgbaImage::from_raw(
                        image.width as u32,
                        image.height as u32,
                        image.bytes.into_owned(),
                    ) {
                        self.import_image(index, DynamicImage::ImageRgba8(image));
                    }
                }
                Err(_) => self.notice = Some("The clipboard does not contain an image.".into()),
            }
        }
        if self.blocks[index]
            .image
            .as_ref()
            .is_some_and(|i| i.show_caption)
        {
            if ui
                .add(
                    TextEdit::singleline(&mut self.blocks[index].image.as_mut().unwrap().caption)
                        .hint_text("Figure caption…")
                        .desired_width(frame_width),
                )
                .changed()
            {
                self.dirty = true;
            }
        }
        response
    }
    pub(super) fn image_options(&mut self, ui: &mut egui::Ui, index: usize) {
        if icons::labelled(ui, Icon::Image, "Replace image…").clicked() {
            self.choose_image(index);
            ui.close_menu();
        }
        ui.menu_button("Change frame", |ui| {
            for frame in ImageFrame::ALL {
                if ui
                    .selectable_value(
                        &mut self.blocks[index].image.as_mut().unwrap().frame,
                        frame,
                        frame.label(),
                    )
                    .hand()
                    .changed()
                {
                    self.dirty = true;
                    self.force_history = true;
                }
            }
        })
        .response
        .hand();
        if ui
            .checkbox(
                &mut self.blocks[index].image.as_mut().unwrap().show_caption,
                "Caption",
            )
            .hand()
            .changed()
        {
            self.dirty = true;
            self.force_history = true;
        }
        if icons::danger(ui, "Remove image").clicked() {
            self.blocks[index].image.as_mut().unwrap().name.clear();
            self.dirty = true;
            self.force_history = true;
            ui.close_menu();
        }
    }
}
fn cover_uv(image: Vec2, frame: Vec2) -> egui::Rect {
    let source = image.x / image.y;
    let target = frame.x / frame.y;
    let size = if source > target {
        Vec2::new(target / source, 1.0)
    } else {
        Vec2::new(1.0, source / target)
    };
    egui::Rect::from_center_size(Pos2::new(0.5, 0.5), size)
}
#[cfg(test)]
mod tests {
    #[test]
    fn cover_crops_without_stretching() {
        let uv = super::cover_uv(
            eframe::egui::vec2(1600.0, 900.0),
            eframe::egui::vec2(400.0, 400.0),
        );
        assert!((uv.width() - 0.5625).abs() < 0.001);
        assert_eq!(uv.height(), 1.0);
    }
    #[test]
    fn asset_names_cannot_escape_library() {
        assert!(super::safe_asset("image-1.png"));
        assert!(!super::safe_asset("../../private.png"));
        assert!(!super::safe_asset("C:\\image.png"));
    }
}
