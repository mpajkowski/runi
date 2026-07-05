use crate::ui::LauncherApp;
use anyhow::{Context, Result};
use egui::{Event, Key, Modifiers, RawInput};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_registry,
    delegate_seat,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        keyboard::{KeyEvent, KeyboardHandler, Keysym, Modifiers as SctkModifiers, RawModifiers},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
};
use std::time::Instant;
use wayland_client::{
    Connection, QueueHandle,
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_seat, wl_surface},
};
use wlr_capture::render::Gpu;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 600;

pub fn run(app: LauncherApp) -> Result<()> {
    let connection = Connection::connect_to_env().context("failed to connect to Wayland")?;
    let (globals, mut events) =
        registry_queue_init(&connection).context("failed to read Wayland globals")?;
    let queue = events.handle();

    let compositor =
        CompositorState::bind(&globals, &queue).context("wl_compositor is unavailable")?;
    let layer_shell =
        LayerShell::bind(&globals, &queue).context("wlr-layer-shell is unavailable")?;
    let surface = compositor.create_surface(&queue);
    let layer = layer_shell.create_layer_surface(
        &queue,
        surface,
        Layer::Overlay,
        Some(env!("CARGO_PKG_NAME")),
        None,
    );
    layer.set_size(WIDTH, HEIGHT);
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    layer.set_exclusive_zone(-1);
    layer.commit();

    let mut state = State {
        registry: RegistryState::new(&globals),
        seats: SeatState::new(&globals, &queue),
        outputs: OutputState::new(&globals, &queue),
        layer,
        keyboard: None,
        egui: egui::Context::default(),
        app,
        gpu: None,
        width: WIDTH,
        height: HEIGHT,
        scale: 1,
        started: Instant::now(),
        input: Vec::new(),
        modifiers: Modifiers::default(),
    };

    while !state.app.closing() {
        events
            .blocking_dispatch(&mut state)
            .context("Wayland event dispatch failed")?;
    }
    Ok(())
}

struct State {
    registry: RegistryState,
    seats: SeatState,
    outputs: OutputState,
    layer: LayerSurface,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    egui: egui::Context,
    app: LauncherApp,
    gpu: Option<Gpu>,
    width: u32,
    height: u32,
    scale: u32,
    started: Instant,
    input: Vec<Event>,
    modifiers: Modifiers,
}

impl State {
    fn draw(&mut self, connection: &Connection) {
        let pixel_width = self.width * self.scale;
        let pixel_height = self.height * self.scale;

        let gpu = self.gpu.get_or_insert_with(|| {
            Gpu::new(
                connection,
                self.layer.wl_surface(),
                pixel_width as i32,
                pixel_height as i32,
            )
        });

        let input = RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(self.width as f32, self.height as f32),
            )),
            time: Some(self.started.elapsed().as_secs_f64()),
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.input),
            focused: true,
            ..RawInput::default()
        };

        let app = &mut self.app;
        gpu.render(
            &self.egui,
            input,
            self.scale as f32,
            (pixel_width, pixel_height),
            app.clear_color(),
            |ui, _| app.update(ui),
        );

        self.layer.commit();
    }

    fn resize_gpu(&self) {
        if let Some(gpu) = &self.gpu {
            gpu.resize(
                (self.width * self.scale) as i32,
                (self.height * self.scale) as i32,
            );
        }
    }

    fn key(&mut self, event: KeyEvent, pressed: bool) {
        if let Some(key) = map_key(event.keysym) {
            self.input.push(Event::Key {
                key,
                physical_key: None,
                pressed,
                repeat: false,
                modifiers: self.modifiers,
            });
        }
        if pressed
            && !self.modifiers.ctrl
            && !self.modifiers.alt
            && let Some(text) = event.utf8
            && !text.is_empty()
            && !text.chars().any(char::is_control)
        {
            self.input.push(Event::Text(text));
        }
    }
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        factor: i32,
    ) {
        self.scale = factor.max(1) as u32;
        self.layer.wl_surface().set_buffer_scale(factor.max(1));
        self.resize_gpu();
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for State {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &LayerSurface) {
        self.app.close();
    }

    fn configure(
        &mut self,
        connection: &Connection,
        _queue: &QueueHandle<Self>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        let (width, height) = configure.new_size;
        if width > 0 {
            self.width = width;
        }
        if height > 0 {
            self.height = height;
        }
        self.resize_gpu();
        self.draw(connection);
    }
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _: &Connection,
        queue: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            self.keyboard = self.seats.get_keyboard(queue, &seat, None).ok();
        }
    }

    fn remove_capability(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard {
            self.keyboard = None;
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
        _: &[u32],
        _: &[Keysym],
    ) {
    }

    fn leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: &wl_surface::WlSurface,
        _: u32,
    ) {
    }

    fn press_key(
        &mut self,
        connection: &Connection,
        _queue: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(event, true);
        self.draw(connection);
    }

    fn release_key(
        &mut self,
        connection: &Connection,
        _queue: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(event, false);
        self.draw(connection);
    }

    fn repeat_key(
        &mut self,
        connection: &Connection,
        _queue: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        event: KeyEvent,
    ) {
        self.key(event, true);
        self.draw(connection);
    }

    fn update_modifiers(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_keyboard::WlKeyboard,
        _: u32,
        modifiers: SctkModifiers,
        _: RawModifiers,
        _: u32,
    ) {
        self.modifiers = Modifiers {
            alt: modifiers.alt,
            ctrl: modifiers.ctrl,
            shift: modifiers.shift,
            mac_cmd: false,
            command: modifiers.ctrl,
        };
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState, SeatState];
}

fn map_key(key: Keysym) -> Option<Key> {
    Some(match key {
        Keysym::Escape => Key::Escape,
        Keysym::Return | Keysym::KP_Enter => Key::Enter,
        Keysym::Tab | Keysym::ISO_Left_Tab => Key::Tab,
        Keysym::BackSpace => Key::Backspace,
        Keysym::Delete => Key::Delete,
        Keysym::Left => Key::ArrowLeft,
        Keysym::Right => Key::ArrowRight,
        Keysym::Up => Key::ArrowUp,
        Keysym::Down => Key::ArrowDown,
        Keysym::Home => Key::Home,
        Keysym::End => Key::End,
        Keysym::space => Key::Space,
        _ => return None,
    })
}

delegate_compositor!(State);
delegate_output!(State);
delegate_seat!(State);
delegate_keyboard!(State);
delegate_layer!(State);
delegate_registry!(State);
