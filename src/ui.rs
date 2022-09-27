use crate::model::Application;
use anyhow::Result;
use std::{thread::JoinHandle, vec};

pub fn run_ui(apps_thread: JoinHandle<Result<Vec<Application>>>) {
    let options = eframe::NativeOptions {
        decorated: false,
        transparent: true,
        resizable: false,
        always_on_top: true,
        ..Default::default()
    };

    eframe::run_native(
        env!("CARGO_PKG_NAME"),
        options,
        Box::new(|_cc| Box::new(LauncherApp::new(apps_thread))),
    );
}

struct LauncherApp {
    /// Application discovery thread
    apps_thread: Option<JoinHandle<Result<Vec<Application>>>>,

    /// Application list
    apps: Vec<Application>,

    /// Indices of filtered applications (points to self.apps)
    filtered_apps: Vec<usize>,

    /// Index of selected application (points to self.filtered_apps)
    selected: Option<usize>,

    /// Search field state
    search_state: String,
}

impl LauncherApp {
    fn new(apps_thread: JoinHandle<Result<Vec<Application>>>) -> Self {
        Self {
            apps_thread: Some(apps_thread),
            apps: vec![],
            filtered_apps: vec![],
            selected: None,
            search_state: String::with_capacity(16),
        }
    }

    fn ensure_init(&mut self) -> Result<()> {
        if let Some(apps_thread) = self.apps_thread.take() {
            let apps = apps_thread.join().expect("failed to join apps_thread")?;
            let filtered_apps = (0..apps.len()).collect();
            self.apps = apps;
            self.filtered_apps = filtered_apps;
        }

        Ok(())
    }

    fn on_search_update(&mut self) {
        let prev_selected_idx = self.selected.take().map(|idx| self.filtered_apps[idx]);

        self.filtered_apps.clear();

        for (app_idx, app) in self.apps.iter().enumerate() {
            if app.matches(&self.search_state) {
                self.filtered_apps.push(app_idx);

                // retain previous selection (if any)
                if prev_selected_idx == Some(app_idx) {
                    self.selected = Some(self.filtered_apps.len() - 1);
                }
            }
        }

        // always select exact match
        if self.filtered_apps.len() == 1 {
            self.selected = Some(0);
        }
    }
}

impl eframe::App for LauncherApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> egui::Rgba {
        egui::Rgba::TRANSPARENT // Make sure we don't paint anything behind the rounded corners
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        use egui::*;

        self.ensure_init().expect("Failed to initialize app");

        {
            let input = ctx.input();

            if input.key_pressed(Key::Escape) {
                frame.close();
            } else if input.key_pressed(Key::ArrowDown) {
                let selected = match self.selected {
                    None => 0,
                    Some(x) => (x + 1).min(self.filtered_apps.len().saturating_sub(1)),
                };
                self.selected = Some(selected);
            } else if input.key_pressed(Key::ArrowUp) {
                let selected = match self.selected {
                    None => 0,
                    Some(x) => x.saturating_sub(1),
                };
                self.selected = Some(selected);
            } else if input.key_pressed(Key::Enter) {
                if let Some(selected) = self.selected.as_ref() {
                    self.apps[self.filtered_apps[*selected]]
                        .launch()
                        .expect("Failed to launch application");
                }
            }
        }

        let text_color = ctx.style().visuals.text_color();

        // Height of the title bar
        let height = 32.0;

        CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                let rect = ui.max_rect();
                let painter = ui.painter();

                // Frame
                painter.rect(
                    rect.shrink(1.0),
                    10.0,
                    ctx.style().visuals.window_fill(),
                    Stroke::new(1.0, text_color),
                );

                // Paint the line under the search
                painter.line_segment(
                    [
                        rect.left_top() + vec2(2.0, height),
                        rect.right_top() + vec2(-2.0, height),
                    ],
                    Stroke::new(1.0, text_color),
                );

                let search_rect = {
                    let mut rect = rect;
                    rect.max.y = rect.min.y + height;
                    rect
                }
                .shrink(4.0);

                let search_response = ui.put(
                    search_rect,
                    TextEdit::singleline(&mut self.search_state)
                        .font(TextStyle::Heading)
                        .hint_text("🔎 Search"),
                );

                // always focus on search
                {
                    let mut memory = ctx.memory();
                    memory.request_focus(search_response.id);
                    memory.lock_focus(search_response.id, true);
                }

                if search_response.changed() {
                    self.on_search_update();
                }

                let content_rect = {
                    let mut rect = rect;
                    rect.min.y = search_rect.max.y;
                    rect
                }
                .shrink(8.0);

                let mut application_list_ui = ui.child_ui(content_rect, *ui.layout());

                // draw filtered applications
                application_list_ui.vertical(|ui| {
                    for (loop_idx, app_idx) in self.filtered_apps.iter().enumerate() {
                        let current = Some(loop_idx);

                        let app_name = &self.apps[*app_idx].name;
                        let mut app_name_widget =
                            RichText::new(app_name).text_style(TextStyle::Heading);

                        // apply highlight for selected application
                        if self.selected == current {
                            app_name_widget = app_name_widget.background_color(Color32::DARK_RED);
                        }

                        let select_response =
                            Label::new(app_name_widget).sense(Sense::click()).ui(ui);

                        if select_response.clicked() {
                            self.selected = current;
                        }
                    }
                });
            });
    }
}