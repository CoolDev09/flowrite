use super::*;

impl FlowriteApp {
    pub(super) fn go_home(&mut self) {
        if self.active_document.is_empty() || self.save_workspace() {
            self.show_home = true;
            self.settings_open = false;
            self.current_folder = None;
            self.home_query.clear();
            self.slash_target = None;
            self.block_picker_target = None;
            self.find_open = false;
            self.focus_mode = false;
            self.focus_request = None;
        }
    }

    pub(super) fn folder_dialog_ui(&mut self, ctx: &egui::Context) {
        if !self.folder_dialog {
            return;
        }
        let mut create = false;
        let mut cancel = false;
        egui::Window::new("Create a folder")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(340.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Keep related pages together.").color(theme::current().muted),
                );
                ui.add_space(12.0);
                let edit = ui.add(
                    TextEdit::singleline(&mut self.folder_name)
                        .id(egui::Id::new("folder_name"))
                        .hint_text("Folder name")
                        .desired_width(320.0)
                        .margin(10.0),
                );
                if self.folder_focus {
                    edit.request_focus();
                    self.folder_focus = false;
                }
                if let Some(error) = &self.folder_error {
                    ui.label(RichText::new(error).color(Color32::from_rgb(235, 166, 147)));
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    create = ui
                        .add_enabled(
                            !self.folder_name.trim().is_empty(),
                            egui::Button::new("Create folder"),
                        )
                        .hand()
                        .clicked()
                        || (edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    cancel = ui.button("Cancel").hand().clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                });
            });
        if cancel {
            self.folder_dialog = false;
        }
        if create {
            match self.workspace.create_folder(&self.folder_name) {
                Ok(id) => {
                    if let Some(page) = self.dialogs.move_after_create.take() {
                        self.workspace.move_document(&page, Some(id));
                    }
                    match self.workspace.save() {
                        Ok(()) => {
                            self.folder_dialog = false;
                            self.folder_error = None;
                        }
                        Err(error) => {
                            self.folder_error = Some(format!("Could not save: {error}"));
                        }
                    }
                }
                Err(error) => self.folder_error = Some(error),
            }
        }
    }
}

pub(super) fn relative_time(modified: u64) -> String {
    let minutes = workspace::timestamp().saturating_sub(modified) / 60_000;
    if minutes == 0 {
        "Edited just now".into()
    } else if minutes < 60 {
        format!("Edited {minutes} min ago")
    } else {
        let hours = minutes / 60;
        if hours < 24 {
            format!(
                "Edited {hours} {} ago",
                if hours == 1 { "hour" } else { "hours" }
            )
        } else {
            let days = hours / 24;
            format!(
                "Edited {days} {} ago",
                if days == 1 { "day" } else { "days" }
            )
        }
    }
}
