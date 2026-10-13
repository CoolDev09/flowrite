use super::*;

fn summary(message: &str) -> (&str, bool) {
    if message.starts_with("Could not open:") || message.contains("newer flowrite format") {
        ("Could not open this file. Try a valid .flowrite file or update Flowrite. Your current document is unchanged.", true)
    } else if message.starts_with("Opened ") {
        ("Document opened.", false)
    } else if message.starts_with("Saved ") {
        ("Document saved.", false)
    } else if message.starts_with("Exported ") {
        ("Export complete.", false)
    } else if message.to_lowercase().contains("could not")
        || message.contains("needs attention")
        || message.starts_with("Missing image ")
    {
        (
            "This action could not finish. Open Details for the next step.",
            true,
        )
    } else {
        (message, false)
    }
}

impl FlowriteApp {
    pub(super) fn notification(&mut self, ctx: &egui::Context) {
        if let Some(message) = self.notice.clone() {
            let now = ctx.input(|i| i.time);
            if self.notice_previous.as_deref() != Some(&message) {
                self.notice_start = Some(now);
                self.notice_previous = Some(message.clone());
            }
            let (brief, error) = summary(&message);
            if !error
                && self
                    .notice_start
                    .is_some_and(|start| now - start >= self.settings.export.toast_seconds.max(3.0))
            {
                self.notice = None;
                self.notice_previous = None;
                self.exported_path = None;
            } else {
                ctx.request_repaint_after(std::time::Duration::from_millis(200));
                egui::Area::new(egui::Id::new("notice"))
                    .order(egui::Order::Foreground)
                    .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-16.0, -42.0))
                    .show(ctx, |ui| {
                        Frame::popup(ui.style()).inner_margin(12.0).show(ui, |ui| {
                            ui.set_width((ctx.screen_rect().width() - 40.0).min(400.0));
                            ui.label(RichText::new(brief).size(14.0));
                            ui.horizontal(|ui| {
                                if brief != message && ui.button("Details").hand().clicked() {
                                    ctx.data_mut(|d| {
                                        d.insert_temp(
                                            egui::Id::new("notice_details"),
                                            message.clone(),
                                        )
                                    });
                                }
                                if message.starts_with("Exported ")
                                    && ui.button("Open folder").hand().clicked()
                                {
                                    if let Some(parent) =
                                        self.exported_path.as_ref().and_then(|p| p.parent())
                                    {
                                        settings_ui::open_path(parent);
                                    }
                                }
                                if ui.button("Dismiss").hand().clicked() {
                                    self.notice = None;
                                }
                            });
                        });
                    });
            }
        }
        if let Some(message) = ctx.data(|d| d.get_temp::<String>(egui::Id::new("notice_details"))) {
            let mut open = true;
            egui::Window::new("Action details")
                .open(&mut open)
                .collapsible(false)
                .default_width(400.0)
                .max_width((ctx.screen_rect().width() - 48.0).max(200.0))
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("Copy details").hand().clicked() {
                        ctx.copy_text(
                            ctx.data(|d| d.get_temp::<String>(egui::Id::new("notice_details")))
                                .unwrap_or_default(),
                        );
                    }
                });
            if !open {
                ctx.data_mut(|d| d.remove::<String>(egui::Id::new("notice_details")));
            }
        }
    }
}
