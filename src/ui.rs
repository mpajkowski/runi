use crate::config;
use crate::model::Application;
use anyhow::{Context, Result};
use egui::*;
use std::{thread::JoinHandle, vec};

pub fn run_ui(apps_thread: JoinHandle<Vec<Application>>) {
    let options = eframe::NativeOptions {
        window_builder: Some(Box::new(|v: ViewportBuilder| {
            v.with_decorations(true)
                .with_transparent(true)
                .with_resizable(false)
                .with_always_on_top()
        })),
        ..Default::default()
    };

    if let Err(err) = eframe::run_native(
        env!("CARGO_PKG_NAME"),
        options,
        Box::new(|_cc| Box::new(LauncherApp::new(apps_thread))),
    ) {
        log::error!("UI error: {err}");
    }
}

struct LauncherApp {
    /// Application discovery thread
    apps_thread: Option<JoinHandle<Vec<Application>>>,

    /// Application list
    apps: Vec<Application>,

    /// Indices of filtered applications (points to self.apps)
    filtered_apps: Vec<(usize, f64)>,

    /// Index of selected application (points to self.filtered_apps)
    selected: usize,

    /// Search field state
    search_state: String,

    /// Error (if occurred)
    error: Option<String>,
}

impl LauncherApp {
    fn new(apps_thread: JoinHandle<Vec<Application>>) -> Self {
        Self {
            apps_thread: Some(apps_thread),
            apps: vec![],
            filtered_apps: vec![],
            selected: 0,
            search_state: String::with_capacity(16),
            error: None,
        }
    }

    fn exec_app(&self) -> Result<()> {
        self.apps[self.filtered_apps[self.selected].0]
            .exec()
            .context("Failed to launch application")?;

        Ok(())
    }

    fn ensure_init(&mut self, ctx: &egui::Context) {
        if let Some(apps_thread) = self.apps_thread.take() {
            let apps = apps_thread.join().expect("failed to join apps_thread");
            let filtered_apps = (0..apps.len()).map(|idx| (idx, 1.0)).collect();
            self.apps = apps;
            self.filtered_apps = filtered_apps;
        }

        ctx.output_mut(|o| o.cursor_icon = CursorIcon::None);
    }

    fn on_search_update(&mut self) {
        if self.search_state.is_empty() {
            self.filtered_apps = (0..self.apps.len()).map(|idx| (idx, 1.0)).collect();
            self.selected = 0;
            return;
        }

        self.filtered_apps.clear();

        for (app_idx, app) in self.apps.iter().enumerate() {
            let score = app.score(&self.search_state);

            if score > 0.05 {
                self.filtered_apps.push((app_idx, score));
            }
        }

        // sort by score (reversed order)
        self.filtered_apps.sort_by(|a, b| b.1.total_cmp(&a.1));
        self.selected = 0;
    }

    fn on_error(&mut self, err: anyhow::Error) {
        self.error = Some(format!("{err:?}"));
    }

    fn reset_error(&mut self) {
        self.error = None;
    }

    fn check_input(&mut self, ctx: &egui::Context) -> Result<()> {
        let should_close = ctx.input(|input| {
            let mut close = false;

            if input.key_pressed(Key::Escape) {
                if self.error.is_some() {
                    self.reset_error();
                } else {
                    close = true;
                }
            } else if input.key_pressed(Key::ArrowDown) {
                self.selected = (self.selected + 1).min(self.filtered_apps.len().saturating_sub(1));
            } else if input.key_pressed(Key::ArrowUp) {
                self.selected = self.selected.saturating_sub(1);
            } else if input.key_pressed(Key::Enter) {
                self.exec_app()?;
            }

            anyhow::Ok(close)
        })?;

        if should_close {
            close(ctx);
        }

        Ok(())
    }
}

impl eframe::App for LauncherApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        config::BACKGROUND_COLOR.to_normalized_gamma_f32()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.ensure_init(ctx);

        if let Err(err) = self.check_input(ctx) {
            self.on_error(err);
        }

        if let Some(err) = self.error.as_ref() {
            egui::Window::new("")
                .open(&mut true)
                .title_bar(false)
                .resizable(false)
                .show(ctx, |ui| {
                    let message = format!("{err}\n\nPress <ESC> and try again");
                    let message = RichText::new(message).text_style(TextStyle::Heading);
                    Label::new(message).ui(ui);
                });
        }

        let text_color = ctx.style().visuals.text_color();

        // Height of the title bar
        let height = 32.0;

        CentralPanel::default()
            .frame(Frame::none())
            .show(ctx, |ui| {
                ui.set_enabled(self.error.is_none());
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
                ctx.memory_mut(|memory| {
                    memory.request_focus(search_response.id);
                    memory.set_focus_lock_filter(
                        search_response.id,
                        EventFilter {
                            tab: true,
                            escape: false,
                            horizontal_arrows: true,
                            vertical_arrows: true,
                        },
                    );
                });

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
                ScrollArea::vertical().show(&mut application_list_ui, |ui| {
                    // justify apps for better mouse interaction
                    let list_layout = Layout::top_down(Align::Min).with_cross_justify(true);

                    // draw filtered applications
                    let result: InnerResponse<Result<(), anyhow::Error>> =
                        ui.with_layout(list_layout, |ui| {
                            for (selection, (app_idx, _)) in self.filtered_apps.iter().enumerate() {
                                let mut selected = false;
                                let app_name = &self.apps[*app_idx].name;
                                let mut app_name_widget =
                                    RichText::new(app_name).text_style(TextStyle::Heading);

                                // apply highlight to selected application
                                if self.selected == selection {
                                    app_name_widget =
                                        app_name_widget.background_color(config::SELECTION_COLOR);
                                    selected = true;
                                }

                                let label = Label::new(app_name_widget)
                                    .sense(Sense::focusable_noninteractive());
                                let response = label.ui(ui);

                                if selected {
                                    response.scroll_to_me(None);
                                }
                            }

                            anyhow::Ok(())
                        });

                    if let Err(err) = result.inner {
                        self.on_error(err)
                    }
                })
            });
    }
}

fn close(ctx: &egui::Context) {
    ctx.send_viewport_cmd(ViewportCommand::Close);
}
