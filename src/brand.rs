use super::*;

const PAPER: Color32 = Color32::from_rgb(222, 228, 219);
const MARK: Color32 = Color32::from_rgb(31, 36, 32);

pub(super) fn logo(ui: &egui::Ui, rect: egui::Rect) {
    let scale = rect.width() / 32.0;
    let point = |x: f32, y: f32| rect.min + Vec2::new(x * scale, y * scale);
    ui.painter().rect_filled(rect, 7.0 * scale, PAPER);
    for (x1, y1, x2, y2) in [
        (9.0, 8.0, 12.0, 23.0),
        (9.0, 8.0, 23.0, 11.0),
        (9.0, 15.0, 19.0, 18.0),
    ] {
        ui.painter().rect_filled(
            egui::Rect::from_min_max(point(x1, y1), point(x2, y2)),
            0.0,
            MARK,
        );
    }
    for points in [
        [(23.0, 8.0), (26.0, 8.0), (23.0, 11.0)],
        [(19.0, 15.0), (22.0, 15.0), (19.0, 18.0)],
        [(9.0, 23.0), (12.0, 23.0), (9.0, 26.0)],
    ] {
        ui.painter().add(egui::Shape::convex_polygon(
            points.map(|(x, y)| point(x, y)).to_vec(),
            MARK,
            Stroke::NONE,
        ));
    }
}

impl FlowriteApp {
    pub(super) fn brand_button(&mut self, ui: &mut egui::Ui) {
        let (rect, response) = ui.allocate_exact_size(Vec2::new(102.0, 32.0), Sense::click());
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "flowrite · Home")
        });
        let icon = egui::Rect::from_min_size(rect.min + Vec2::new(0.0, 3.0), Vec2::splat(26.0));
        logo(ui, icon);
        ui.painter().text(
            rect.min + Vec2::new(36.0, 16.0),
            egui::Align2::LEFT_CENTER,
            "flowrite",
            FontId::proportional(16.0),
            theme::current().ink,
        );
        if response
            .on_hover_text("Back to your documents")
            .hand()
            .clicked()
        {
            self.go_home();
        }
    }
}

pub(super) fn window_icon() -> egui::IconData {
    let size = 64;
    let mut rgba = Vec::with_capacity(size * size * 4);
    for py in 0..size {
        for px in 0..size {
            let x = (px as f32 + 0.5) / 2.0;
            let y = (py as f32 + 0.5) / 2.0;
            let dx = (x - 16.0).abs() - 9.0;
            let dy = (y - 16.0).abs() - 9.0;
            let inside = dx.max(0.0).hypot(dy.max(0.0)) <= 7.0;
            let stem = x >= 9.0 && x <= 12.0 && y >= 8.0 && y <= 23.0;
            let top = x >= 9.0 && y >= 8.0 && y <= 11.0 && x + y <= 34.0;
            let middle = x >= 9.0 && y >= 15.0 && y <= 18.0 && x + y <= 37.0;
            let nib = x >= 9.0 && y >= 23.0 && x + y <= 35.0;
            let color = if stem || top || middle || nib {
                MARK
            } else {
                PAPER
            };
            rgba.extend_from_slice(&[
                color.r(),
                color.g(),
                color.b(),
                if inside { 255 } else { 0 },
            ]);
        }
    }
    egui::IconData {
        rgba,
        width: size as u32,
        height: size as u32,
    }
}
