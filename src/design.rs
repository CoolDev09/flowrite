//! Shared layout and interaction rules for the workspace, forms and popovers.
use super::*;

pub(super) fn detail_panel(
    ctx: &egui::Context,
    id: &'static str,
    overlay: bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let frame = Frame::none()
        .fill(theme::current().panel)
        .inner_margin(Margin::same(16.0));
    if overlay {
        egui::Window::new(id)
            .id(egui::Id::new(("detail", id)))
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, Vec2::new(-12.0, 60.0))
            .fixed_size(Vec2::new(
                284.0,
                (ctx.screen_rect().height() - 110.0).max(220.0),
            ))
            .frame(Frame::popup(&ctx.style()).inner_margin(Margin::same(16.0)))
            .show(ctx, content);
    } else {
        egui::SidePanel::right(id)
            .exact_width(284.0)
            .resizable(false)
            .frame(frame)
            .show(ctx, content);
    }
}

pub(super) const SPACE: [f32; 8] = [4.0, 8.0, 12.0, 16.0, 24.0, 32.0, 48.0, 64.0];
pub(super) const ROW_HEIGHT: f32 = 32.0;
pub(super) const CONTROL_RADIUS: f32 = 5.0;
pub(super) const SURFACE_RADIUS: f32 = 8.0;
pub(super) const WRITING_MIN: f32 = 420.0;

pub(super) fn named(
    response: egui::Response,
    kind: egui::WidgetType,
    label: &str,
) -> egui::Response {
    let enabled = response.enabled();
    response.widget_info(|| egui::WidgetInfo::labeled(kind, enabled, label));
    response
}

pub(super) fn focus_ring(ui: &egui::Ui, response: &egui::Response) {
    if response.has_focus() {
        ui.painter().rect_stroke(
            response.rect.expand(2.0),
            CONTROL_RADIUS,
            Stroke::new(1.5_f32, theme::current().accent),
        );
    }
}

pub(super) fn nav_row(
    ui: &mut egui::Ui,
    icon: Icon,
    title: &str,
    selected: bool,
) -> egui::Response {
    let button = egui::Button::image_and_text(
        icon.image(ui, theme::current().ink),
        RichText::new(title).size(14.0),
    )
    .frame(selected)
    .selected(selected)
    .truncate()
    .min_size(Vec2::new(ui.available_width(), ROW_HEIGHT));
    let response = ui
        .add_sized([ui.available_width(), ROW_HEIGHT], button)
        .hand()
        .on_hover_text(title);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, true, selected, title)
    });
    focus_ring(ui, &response);
    response
}

pub(super) fn panel_widths(available: f32, sidebar: f32, detail: bool) -> (f32, bool) {
    let sidebar = sidebar
        .clamp(164.0, 280.0)
        .min((available - WRITING_MIN - 32.0).max(164.0));
    (sidebar, detail && available - sidebar - 286.0 < WRITING_MIN)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cramped_detail_panels_use_an_overlay_instead_of_squeezing_writing() {
        for pixels in [900.0, 1205.0, 1280.0] {
            for scale in [1.0, 1.25, 1.5] {
                let available = pixels / scale;
                let (sidebar, overlay) = panel_widths(available, 222.0, true);
                assert!(available - sidebar - if overlay { 0.0 } else { 286.0 } >= 400.0);
            }
        }
    }
}
