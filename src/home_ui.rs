use super::*;

enum HomeAction {
    Open(String),
    New(PaperFormat),
    Move(String, Option<String>),
}

impl FlowWriteApp {
    pub(super) fn go_home(&mut self) {
        if self.active_document.is_empty() || self.save_workspace() {
            self.show_home = true;
            self.current_folder = None;
            self.home_query.clear();
            self.slash_target = None;
            self.block_picker_target = None;
            self.find_open = false;
            self.focus_request = None;
        }
    }

    pub(super) fn home_header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("home_header")
            .exact_height(78.0)
            .frame(
                Frame::none()
                    .fill(CANVAS)
                    .inner_margin(Margin::symmetric(24.0, 18.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.brand_button(ui);
                    ui.add_space(36.0);
                    let width = (ui.available_width() - 180.0).clamp(200.0, 620.0);
                    ui.add(
                        TextEdit::singleline(&mut self.home_query)
                            .id(egui::Id::new("home_search"))
                            .hint_text("Search your documents")
                            .desired_width(width)
                            .margin(Margin::symmetric(16.0, 11.0)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .button("Open file…")
                            .on_hover_text("Import a saved document · Ctrl+O")
                            .clicked()
                        {
                            self.open_document();
                        }
                    });
                });
            });
    }

    pub(super) fn home_page(&mut self, ctx: &egui::Context) {
        let mut action = None;
        let mut folder = None;
        egui::CentralPanel::default().frame(Frame::none().fill(CANVAS)).show(ctx,|ui|{
            egui::ScrollArea::vertical().id_salt(("home_scroll",&self.current_folder)).show(ui,|ui|{
                let width=(ui.available_width()-80.0).clamp(400.0,1120.0);
                let inset=((ui.available_width()-width)/2.0).max(24.0);
                ui.horizontal(|ui|{
                    ui.add_space(inset);
                    ui.allocate_ui_with_layout(Vec2::new(width,0.0),Layout::top_down(Align::LEFT),|ui|{
                        ui.add_space(30.0);
                        if let Some(id)=&self.current_folder {
                            ui.horizontal(|ui|{
                                if ui.button("All documents").clicked(){folder=Some(String::new());}
                                ui.label(RichText::new("/").color(MUTED));
                                let name=self.workspace.folders.iter().find(|f|&f.id==id).map(|f|f.name.as_str()).unwrap_or("Folder");
                                ui.label(RichText::new(name).size(26.0).strong());
                            });
                        } else {
                            ui.label(RichText::new("Make room for your next idea.").size(26.0).strong());
                            ui.add_space(7.0);
                            ui.label(RichText::new("Your pages, papers, and notes. All in one quiet workspace.").color(MUTED).size(14.0));
                        }
                        ui.add_space(25.0);
                        ui.horizontal(|ui|{
                            if ui.add_sized([132.0,38.0],egui::Button::new("+  New page")).clicked(){action=Some(HomeAction::New(PaperFormat::Freeform));}
                            if ui.add_sized([140.0,38.0],egui::Button::new("+  Create folder").frame(false)).clicked(){
                                self.folder_dialog=true;self.folder_name.clear();self.folder_error=None;
                                self.folder_focus=true;
                            }
                        });
                        ui.add_space(30.0);
                        if self.current_folder.is_none() {
                        ui.label(RichText::new("Start with a page").size(14.0).strong());
                        ui.add_space(12.0);
                        ui.horizontal(|ui|{
                            for (label,detail,format) in [("Blank page","A fresh start",PaperFormat::Freeform),("MLA essay","Ready for your first draft",PaperFormat::Mla),("APA paper","A structured starting point",PaperFormat::Apa)] {
                                if template_card(ui,label,detail,format).clicked(){action=Some(HomeAction::New(format));}
                                ui.add_space(10.0);
                            }
                        });
                        ui.add_space(34.0);
                        ui.separator();
                        ui.add_space(24.0);
                        }
                        if self.current_folder.is_none() && !self.workspace.folders.is_empty() {
                            ui.label(RichText::new("Folders").size(17.0).strong());
                            ui.add_space(12.0);
                            ui.horizontal_wrapped(|ui|{
                                for entry in &self.workspace.folders {
                                    let count=self.workspace.documents.iter().filter(|d|d.folder.as_ref()==Some(&entry.id)).count();
                                    if folder_card(ui,&entry.name,count).clicked(){folder=Some(entry.id.clone());}
                                }
                            });
                            ui.add_space(28.0);
                        }
                        ui.horizontal(|ui|{
                            if self.current_folder.is_some() {
                                ui.label(RichText::new("Documents in this folder").size(18.0).strong());
                            }else{ui.label(RichText::new(if self.home_query.is_empty(){"Recent documents"}else{"Search results"}).size(18.0).strong());}
                            ui.with_layout(Layout::right_to_left(Align::Center),|ui|{ui.label(RichText::new("Last edited").size(12.0).color(MUTED));});
                        });
                        ui.add_space(18.0);
                        let query=self.home_query.trim().to_lowercase();
                        let mut documents:Vec<_>=self.workspace.documents.iter().filter(|d|{
                            (self.current_folder.is_none()||d.folder==self.current_folder) && d.document.title.to_lowercase().contains(&query)
                        }).collect();
                        documents.sort_by_key(|d|std::cmp::Reverse(d.modified));
                        if documents.is_empty() {
                            ui.add_space(38.0);
                            ui.label(RichText::new(if !query.is_empty(){"No documents match your search."}else if self.current_folder.is_some(){"This folder is ready for your first page."}else{"A little space for something good."}).size(17.0));
                            ui.add_space(10.0);
                            ui.label(RichText::new(if query.is_empty(){"Choose New page above to begin. Your writing saves automatically."}else{"Try a different title."}).color(MUTED));
                        }
                        let columns=((width+18.0)/218.0).floor().max(1.0) as usize;
                        let card_width=(width-18.0*(columns-1) as f32)/columns as f32;
                        egui::Grid::new("document_grid").spacing(Vec2::new(18.0,22.0)).show(ui,|ui|{
                            for (index,entry) in documents.iter().enumerate(){
                                ui.push_id(&entry.id,|ui|{
                                    let (response,menu_rect)=document_card(ui,&entry.document,entry.modified,card_width);
                                    if response.clicked(){action=Some(HomeAction::Open(entry.id.clone()));}
                                    let mut menu=|ui:&mut egui::Ui|{
                                        if ui.button("Open document").clicked(){action=Some(HomeAction::Open(entry.id.clone()));ui.close_menu();}
                                        ui.separator();
                                        ui.label(RichText::new("Move to folder").small().color(MUTED));
                                        if ui.selectable_label(entry.folder.is_none(),"No folder").clicked(){action=Some(HomeAction::Move(entry.id.clone(),None));ui.close_menu();}
                                        for dest in &self.workspace.folders {
                                            if ui.selectable_label(entry.folder.as_ref()==Some(&dest.id),&dest.name).clicked(){action=Some(HomeAction::Move(entry.id.clone(),Some(dest.id.clone())));ui.close_menu();}
                                        }
                                    };
                                    response.context_menu(&mut menu);
                                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(menu_rect),|ui|{ui.menu_button("…",&mut menu);});
                                });
                                if (index+1)%columns==0 {ui.end_row();}
                            }
                        });
                        ui.add_space(70.0);
                    });
                });
            });
        });
        if let Some(id) = folder {
            self.current_folder = if id.is_empty() { None } else { Some(id) };
            self.home_query.clear();
        }
        if let Some(action) = action {
            match action {
                HomeAction::Open(id) => self.switch_document(&id),
                HomeAction::New(format) => {
                    self.start_fresh();
                    if !self.show_home && format != PaperFormat::Freeform {
                        self.style.apply_preset(format);
                        self.title = self.workspace.unique_title(if format == PaperFormat::Mla {
                            "Untitled MLA essay"
                        } else {
                            "Untitled APA paper"
                        });
                        self.show_inspector = true;
                        self.dirty = true;
                        self.save_workspace();
                    }
                }
                HomeAction::Move(id, dest) => {
                    self.commit_current();
                    self.workspace.move_document(&id, dest);
                    if let Err(error) = self.workspace.save() {
                        self.notice = Some(format!("Could not save the folder change: {error}"));
                    }
                }
            }
        }
        self.folder_dialog_ui(ctx);
    }

    fn folder_dialog_ui(&mut self, ctx: &egui::Context) {
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
                ui.label(RichText::new("Keep related pages together.").color(MUTED));
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
                        .clicked()
                        || (edit.has_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
                    cancel = ui.button("Cancel").clicked()
                        || ui.input(|i| i.key_pressed(egui::Key::Escape));
                });
            });
        if cancel {
            self.folder_dialog = false;
        }
        if create {
            match self.workspace.create_folder(&self.folder_name) {
                Ok(_) => match self.workspace.save() {
                    Ok(()) => {
                        self.folder_dialog = false;
                        self.folder_error = None;
                    }
                    Err(error) => {
                        self.folder_error = Some(format!("Could not save: {error}"));
                    }
                },
                Err(error) => self.folder_error = Some(error),
            }
        }
    }
}

fn template_card(
    ui: &mut egui::Ui,
    title: &str,
    subtitle: &str,
    format: PaperFormat,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(190.0, 164.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, title));
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        8.0,
        if response.hovered() {
            Color32::from_rgb(42, 44, 48)
        } else {
            PANEL
        },
    );
    painter.rect_stroke(
        rect,
        8.0,
        Stroke::new(
            1.0_f32,
            if response.hovered() {
                MUTED
            } else {
                PANEL_STROKE
            },
        ),
    );
    let page = egui::Rect::from_min_size(rect.min + Vec2::new(64.0, 18.0), Vec2::new(62.0, 78.0));
    painter.rect_filled(page, 3.0, Color32::from_rgb(204, 207, 199));
    let dark = Color32::from_rgb(76, 81, 76);
    if format == PaperFormat::Freeform {
        painter.line_segment(
            [
                page.center() - Vec2::new(10.0, 0.0),
                page.center() + Vec2::new(10.0, 0.0),
            ],
            Stroke::new(1.8_f32, dark),
        );
        painter.line_segment(
            [
                page.center() - Vec2::new(0.0, 10.0),
                page.center() + Vec2::new(0.0, 10.0),
            ],
            Stroke::new(1.8_f32, dark),
        );
    } else {
        for line in 0..8 {
            let y = page.top() + 14.0 + line as f32 * 6.0;
            let left = if line < 3 && format == PaperFormat::Apa {
                19.0
            } else {
                10.0
            };
            let right = if line < 3 { 35.0 } else { 51.0 };
            painter.line_segment(
                [
                    Pos2::new(page.left() + left, y),
                    Pos2::new(page.left() + right, y),
                ],
                Stroke::new(0.7_f32, dark),
            );
        }
    }
    painter.text(
        rect.min + Vec2::new(16.0, 116.0),
        egui::Align2::LEFT_CENTER,
        title,
        FontId::proportional(14.0),
        INK,
    );
    painter.text(
        rect.min + Vec2::new(16.0, 140.0),
        egui::Align2::LEFT_CENTER,
        subtitle,
        FontId::proportional(11.0),
        MUTED,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn folder_card(ui: &mut egui::Ui, name: &str, count: usize) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(212.0, 62.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Folder: {name}, {count} documents"),
        )
    });
    let p = ui.painter();
    p.rect_filled(
        rect,
        7.0,
        if response.hovered() {
            Color32::from_rgb(43, 45, 49)
        } else {
            PANEL
        },
    );
    p.rect_stroke(rect, 7.0, Stroke::new(1.0_f32, PANEL_STROKE));
    p.rect_filled(
        egui::Rect::from_min_size(rect.min + Vec2::new(16.0, 23.0), Vec2::new(25.0, 18.0)),
        3.0,
        MUTED,
    );
    p.rect_filled(
        egui::Rect::from_min_size(rect.min + Vec2::new(16.0, 19.0), Vec2::new(12.0, 9.0)),
        2.0,
        MUTED,
    );
    let title = shorten(name, 20);
    p.text(
        rect.min + Vec2::new(54.0, 23.0),
        egui::Align2::LEFT_CENTER,
        title,
        FontId::proportional(13.0),
        INK,
    );
    p.text(
        rect.min + Vec2::new(54.0, 43.0),
        egui::Align2::LEFT_CENTER,
        format!("{count} pages"),
        FontId::proportional(11.0),
        MUTED,
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn document_card(
    ui: &mut egui::Ui,
    document: &SavedDocument,
    modified: u64,
    width: f32,
) -> (egui::Response, egui::Rect) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 260.0), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            true,
            format!("Open {}", document.title),
        )
    });
    let p = ui.painter();
    p.rect_filled(rect, 7.0, PANEL);
    p.rect_stroke(
        rect,
        7.0,
        Stroke::new(
            1.0_f32,
            if response.hovered() {
                MUTED
            } else {
                PANEL_STROKE
            },
        ),
    );
    let preview = egui::Rect::from_min_max(
        rect.min + Vec2::new(16.0, 14.0),
        Pos2::new(rect.right() - 16.0, rect.top() + 188.0),
    );
    p.rect_filled(preview, 2.0, Color32::from_rgb(39, 41, 44));
    let pp = p.with_clip_rect(preview.shrink(12.0));
    pp.text(
        preview.min + Vec2::new(12.0, 14.0),
        egui::Align2::LEFT_TOP,
        shorten(&document.title, 30),
        FontId::proportional(8.0),
        Color32::from_gray(194),
    );
    let text = document
        .blocks
        .iter()
        .filter(|b| !b.text.is_empty())
        .take(5)
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n");
    let text = if text.is_empty() {
        "Your next idea starts here.".to_owned()
    } else {
        text.chars().take(1500).collect()
    };
    let galley = ui.fonts(|fonts| {
        fonts.layout(
            text,
            FontId::proportional(6.5),
            Color32::from_gray(150),
            preview.width() - 24.0,
        )
    });
    pp.galley(preview.min + Vec2::new(12.0, 36.0), galley, INK);
    let title = ui.fonts(|fonts| {
        let mut job = egui::text::LayoutJob::simple_singleline(
            document.title.clone(),
            FontId::proportional(13.0),
            INK,
        );
        job.wrap.max_width = width - 42.0;
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        fonts.layout_job(job)
    });
    p.galley(rect.min + Vec2::new(14.0, 204.0), title, INK);
    let days = workspace::timestamp().saturating_sub(modified) / 86_400_000;
    let label = if days == 0 {
        "Edited today".to_owned()
    } else if days == 1 {
        "Edited yesterday".to_owned()
    } else {
        format!("Edited {days} days ago")
    };
    p.text(
        rect.min + Vec2::new(14.0, 239.0),
        egui::Align2::LEFT_CENTER,
        label,
        FontId::proportional(10.0),
        MUTED,
    );
    let menu_rect = egui::Rect::from_min_size(
        Pos2::new(rect.right() - 36.0, rect.bottom() - 35.0),
        Vec2::new(30.0, 28.0),
    );
    (
        response.on_hover_cursor(egui::CursorIcon::PointingHand),
        menu_rect,
    )
}

fn shorten(text: &str, max: usize) -> String {
    if text.chars().count() > max {
        format!("{}…", text.chars().take(max - 1).collect::<String>())
    } else {
        text.to_owned()
    }
}
