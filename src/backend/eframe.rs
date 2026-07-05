use crate::ui::LauncherApp;
use anyhow::Result;
use egui::{ViewportBuilder, ViewportCommand, vec2};

pub(crate) fn run(app: LauncherApp) -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_app_id(env!("CARGO_PKG_NAME"))
            .with_inner_size(vec2(800.0, 600.0))
            .with_decorations(false)
            .with_transparent(true)
            .with_resizable(false)
            .with_always_on_top(),
        centered: true,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        env!("CARGO_PKG_NAME"),
        options,
        Box::new(move |_| {
            Ok(Box::new(X11App {
                app,
                received_focus: false,
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!(error.to_string()))
}

struct X11App {
    app: LauncherApp,
    received_focus: bool,
}

impl eframe::App for X11App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Request native window focus until the window manager confirms it.
        // Requesting focus for the TextEdit only controls focus inside egui and
        // cannot make an inactive native window receive keyboard events.
        if !self.received_focus {
            self.received_focus = ui.input(|input| input.focused);
            if !self.received_focus {
                ui.ctx().send_viewport_cmd(ViewportCommand::Focus);
            }
        }

        self.app.update(ui);
        if self.app.closing() {
            ui.ctx().send_viewport_cmd(ViewportCommand::Close);
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        self.app.clear_color()
    }
}
