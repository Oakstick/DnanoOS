//! The window, the event loop and the frame: fixed 120 Hz simulation steps,
//! rendering as fast as the display allows.

use std::collections::HashSet;
use std::sync::Arc;

use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Icon, Window, WindowId};

use crate::game::{Game, Screen};
use crate::input::{pad_kind, trigger_latch, PadButton, PadState, Rumbler};
use crate::platform;
use crate::render::characters::ragnar;
use crate::render::gpu::Gpu;
use crate::render::hud;
use crate::render::overlay::Overlay;
use crate::render::view::Frame;
use crate::render::voxels::sprite_from;
#[cfg(feature = "steam")]
use crate::steam::Steam;

/// Fixed simulation step.
pub const STEP: f64 = 1.0 / 120.0;

#[derive(Clone, Debug, Default)]
pub struct Options {
    pub windowed: bool,
    pub fullscreen: bool,
    pub no_vsync: bool,
    /// The quality preset (`--preset high|low`); none takes the build's default.
    pub preset: Option<crate::render::preset::Preset>,
    /// `--dev`: the developer's map on the title and the pause menu, and the testing keys; heeded
    /// only in developer builds (`game::DEV_BUILD`), ignored in the distributables.
    pub dev: bool,
}

#[cfg(target_arch = "wasm32")]
type Pending = std::rc::Rc<std::cell::RefCell<Option<Result<Gpu, String>>>>;

/// Word from the renderer starting on its own thread.
#[cfg(not(target_arch = "wasm32"))]
enum Starting {
    /// The window has been painted: it can be shown.
    Painted,
    Ready(Result<Box<Gpu>, String>),
}

pub struct App {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    opts: Options,
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    #[cfg(target_arch = "wasm32")]
    pending: Pending,
    /// The renderer starting on its own thread, so the window answers Windows meanwhile.
    #[cfg(not(target_arch = "wasm32"))]
    starting: Option<std::sync::mpsc::Receiver<Starting>>,
    game: Game,
    overlay: Overlay,
    frame: Frame,
    gilrs: Option<gilrs::Gilrs>,
    /// The controller read last (the one that rumbles), and its triggers as buttons.
    pad_id: Option<gilrs::GamepadId>,
    triggers: [bool; 2],
    rumbler: Rumbler,
    last: Instant,
    acc: f64,
    fps_frames: u32,
    fps_at: Instant,
    /// Where the last frame's CPU time went, for the F3 readout.
    cpu: crate::render::profile::CpuTimes,
    fps: u32,
    modifiers: ModifiersState,
    fullscreen: bool,
    /// QA (`VH_LOSE_DEVICE=SECS`): seconds after launch at which the GPU device is lost on purpose
    /// (and again that long after each restart), to see the way back (QA.md Q2).
    lose_device: Option<(f64, Instant)>,
    #[cfg(feature = "steam")]
    steam: Option<Steam>,
}

impl App {
    pub fn new(opts: Options) -> Self {
        let saved_full = platform::load("vh.fullscreen").map(|v| v == "1");
        let fullscreen = if opts.windowed {
            false
        } else if opts.fullscreen {
            true
        } else {
            saved_full.unwrap_or(!cfg!(debug_assertions) && !cfg!(target_arch = "wasm32"))
        };
        let mut game = Game::new();
        game.ask_dev(opts.dev, crate::game::DEV_BUILD);
        Self {
            opts,
            window: None,
            gpu: None,
            #[cfg(target_arch = "wasm32")]
            pending: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            starting: None,
            game,
            overlay: Overlay::new(),
            frame: Frame::default(),
            gilrs: gilrs::Gilrs::new().ok(),
            pad_id: None,
            triggers: [false; 2],
            rumbler: Rumbler::default(),
            last: Instant::now(),
            acc: 0.0,
            fps_frames: 0,
            fps_at: Instant::now(),
            cpu: Default::default(),
            fps: 0,
            modifiers: ModifiersState::empty(),
            fullscreen,
            lose_device: std::env::var("VH_LOSE_DEVICE").ok().and_then(|v| v.parse().ok()).map(|s: f64| (s, Instant::now())),
            #[cfg(feature = "steam")]
            steam: Steam::init(),
        }
    }

    fn set_capture(&mut self, on: bool) {
        let Some(w) = &self.window else { return };
        if on == self.game.input.mouse_captured {
            return;
        }
        if on {
            let ok = w.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| w.set_cursor_grab(CursorGrabMode::Confined)).is_ok();
            if ok {
                w.set_cursor_visible(false);
            }
            self.game.input.mouse_captured = ok;
        } else {
            let _ = w.set_cursor_grab(CursorGrabMode::None);
            w.set_cursor_visible(true);
            self.game.input.mouse_captured = false;
        }
    }

    fn apply_fullscreen(&mut self) {
        if let Some(w) = &self.window {
            w.set_fullscreen(if self.fullscreen { Some(Fullscreen::Borderless(None)) } else { None });
        }
    }

    fn toggle_fullscreen(&mut self) {
        self.fullscreen = !self.fullscreen;
        platform::save("vh.fullscreen", if self.fullscreen { "1" } else { "0" });
        self.apply_fullscreen();
    }

    /// The first connected controller as a reading: its buttons by position (the analog triggers
    /// latched into buttons past their threshold), its sticks, and its family. The game reads the
    /// buttons through the bindings (`Input::poll`).
    fn poll_pad(&mut self) -> Option<PadState> {
        use gilrs::{Axis, Button};
        let gilrs = self.gilrs.as_mut()?;
        while gilrs.next_event().is_some() {}
        let Some((id, pad)) = gilrs.gamepads().find(|(_, g)| g.is_connected()) else {
            self.pad_id = None;
            self.triggers = [false; 2];
            return None;
        };
        self.pad_id = Some(id);
        let map: [(Button, PadButton); 14] = [
            (Button::South, PadButton::South),
            (Button::East, PadButton::East),
            (Button::West, PadButton::West),
            (Button::North, PadButton::North),
            // (gilrs's LeftTrigger is the bumper; LeftTrigger2 the trigger.)
            (Button::LeftTrigger, PadButton::LeftBumper),
            (Button::RightTrigger, PadButton::RightBumper),
            (Button::LeftThumb, PadButton::LeftStick),
            (Button::RightThumb, PadButton::RightStick),
            (Button::DPadUp, PadButton::DPadUp),
            (Button::DPadDown, PadButton::DPadDown),
            (Button::DPadLeft, PadButton::DPadLeft),
            (Button::DPadRight, PadButton::DPadRight),
            (Button::Select, PadButton::View),
            (Button::Start, PadButton::Menu),
        ];
        let mut buttons: HashSet<PadButton> = map.iter().filter(|(b, _)| pad.is_pressed(*b)).map(|&(_, p)| p).collect();
        // A trigger without an analog reading is all or nothing.
        let pull = |b: Button| pad.button_data(b).map(|d| d.value()).unwrap_or(if pad.is_pressed(b) { 1.0 } else { 0.0 });
        for (k, (b, p)) in
            [(Button::LeftTrigger2, PadButton::LeftTrigger), (Button::RightTrigger2, PadButton::RightTrigger)].into_iter().enumerate()
        {
            self.triggers[k] = trigger_latch(self.triggers[k], pull(b));
            if self.triggers[k] {
                buttons.insert(p);
            }
        }
        // gilrs reports y up; the game uses the web convention, y down.
        Some(PadState {
            left: (pad.value(Axis::LeftStickX), -pad.value(Axis::LeftStickY)),
            right: (pad.value(Axis::RightStickX), -pad.value(Axis::RightStickY)),
            buttons,
            kind: Some(pad_kind(pad.vendor_id(), pad.name())),
            ..Default::default()
        })
    }

    /// Plays the rumbles the game asked for, on the controller, scaled by the setting, while a
    /// controller is the device in use.
    fn play_rumbles(&mut self) {
        let wanted = std::mem::take(&mut self.game.rumbles);
        self.rumbler.tidy();
        let level = self.game.input.rumble();
        let (Some(gilrs), Some(id)) = (self.gilrs.as_mut(), self.pad_id) else { return };
        if !self.game.input.last_device().is_pad() {
            return;
        }
        for p in wanted.into_iter().filter_map(|r| r.pulse(level)) {
            self.rumbler.play(gilrs, id, p);
        }
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64().min(0.25);
        self.last = now;
        self.acc += dt;

        let had_pad = self.pad_id.is_some();
        let pad = self.poll_pad();
        if had_pad && pad.is_none() {
            // Unplugged, or its battery gone: the game waits for him on the pause menu.
            self.game.controller_lost();
        }
        self.game.input.set_pad(pad);
        #[cfg(target_arch = "wasm32")]
        {
            // The browser drops pointer lock on Esc without telling winit.
            let locked = web_sys::window().and_then(|w| w.document()).and_then(|d| d.pointer_lock_element()).is_some();
            if !locked {
                self.game.input.mouse_captured = false;
            }
        }
        let sim = Instant::now();
        let mut guard = 0;
        while self.acc >= STEP && guard < 10 {
            self.game.update(STEP);
            self.acc -= STEP;
            guard += 1;
        }
        if guard == 10 {
            self.acc = 0.0;
        }
        self.cpu.update = sim.elapsed().as_secs_f64() * 1e3;
        self.play_rumbles();
        if std::mem::take(&mut self.game.want_fullscreen_toggle) {
            self.toggle_fullscreen();
        }
        self.game.fullscreen = self.fullscreen;
        // The quality preset chosen on the settings page, switched at once (the shadow maps made
        // again at their new size).
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(gpu) = &mut self.gpu {
            if gpu.preset().name != self.game.settings.preset {
                if let Some(p) = crate::render::preset::Preset::named(self.game.settings.preset) {
                    gpu.set_preset(crate::render::preset::Preset { gpu_cull: gpu.preset().gpu_cull, ..p });
                    self.game.view.lod_bias = p.lod_bias;
                }
            }
        }
        // The pointer belongs to the game only while playing.
        if !matches!(self.game.screen, Screen::Playing | Screen::Ready | Screen::Dying) {
            self.set_capture(false);
        }
        #[cfg(feature = "steam")]
        if let Some(s) = &mut self.steam {
            s.frame(&mut self.game.events);
        }
        self.game.events.clear();

        // The new level is ready on the GPU: the loading screen lifts, and the clock starts again
        // from now, so the fade-in does not jump the time the load took (RP03).
        if self.game.loading.is_some() && self.gpu.as_ref().is_some_and(|g| g.level_ready(&self.game.view)) {
            self.game.finish_loading();
            self.acc = 0.0;
            self.last = Instant::now();
        }

        self.fps_frames += 1;
        let since = (now - self.fps_at).as_secs_f64();
        if since >= 0.5 {
            self.fps = (self.fps_frames as f64 / since).round() as u32;
            self.fps_frames = 0;
            self.fps_at = now;
        }
        self.render();
    }

    /**
     * The GPU device was lost (QA.md Q2): wgpu would end the process on the next present, so
     * nothing more is presented. The game saves what is marked and pauses; the old renderer is
     * dropped with everything it held, and a new one starts as at launch (the engine's loading
     * screen in the window meanwhile, the game holding), and the level goes up to it whole.
     */
    fn restart_renderer(&mut self) {
        crate::render::gpu::log_gpu_line("the renderer starts again after the device was lost");
        self.game.renderer_lost();
        self.set_capture(false);
        self.gpu = None;
        self.overlay = Overlay::new();
        self.frame = Frame::default();
        if let Some(window) = self.window.clone() {
            self.start_renderer(&window);
        }
    }

    /// Starts the renderer for `window`: on its own thread on the desktop (its shaders take
    /// seconds to compile, and the window keeps answering), as a task on the web.
    fn start_renderer(&mut self, window: &Arc<Window>) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // (Its surfaces are made here, where Windows gives out the window's handle.)
            let size = window.inner_size();
            let candidates = Gpu::surfaces(Some(window));
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let painted = tx.clone();
                let gpu = Gpu::start(candidates, (size.width, size.height), move || {
                    let _ = painted.send(Starting::Painted);
                });
                let _ = tx.send(Starting::Ready(pollster::block_on(gpu).map(Box::new)));
            });
            self.starting = Some(rx);
        }
        #[cfg(target_arch = "wasm32")]
        {
            let pending = self.pending.clone();
            let w = window.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let size = w.inner_size();
                let gpu = Gpu::new(Some(w.clone()), (size.width.max(1), size.height.max(1))).await;
                *pending.borrow_mut() = Some(gpu);
                w.request_redraw();
            });
        }
        self.last = Instant::now();
        window.request_redraw();
    }

    fn render(&mut self) {
        if let (Some((secs, since)), Some(gpu)) = (self.lose_device, &self.gpu) {
            if since.elapsed().as_secs_f64() >= secs {
                gpu.simulate_device_loss();
                self.lose_device = Some((secs, Instant::now() + std::time::Duration::from_secs(10)));
            }
        }
        if self.gpu.as_ref().is_some_and(|g| g.device_lost()) {
            self.restart_renderer();
            return;
        }
        let (Some(window), Some(gpu)) = (&self.window, &mut self.gpu) else { return };
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return;
        }
        gpu.resize(size.width, size.height);
        let scale = window.scale_factor() as f32;
        let logical = (size.width as f32 / scale, size.height as f32 / scale);
        self.game.viewport = logical;
        self.game.view.aspect = size.width as f32 / size.height as f32;
        self.game.view.viewport_h = size.height as f32;
        let t = Instant::now();
        self.game.view.emit(&self.game.scene(), &mut self.frame);
        self.cpu.emit = t.elapsed().as_secs_f64() * 1e3;
        let t = Instant::now();
        self.overlay.begin(logical.0, logical.1);
        hud::draw(&mut self.overlay, &self.game, self.fps);
        self.cpu.hud = t.elapsed().as_secs_f64() * 1e3;
        // The minimap's image where the fog lifted or the ground changed (a new level's whole).
        self.overlay.map.upload(|r, px| gpu.write_atlas(r, px));
        let t = Instant::now();
        gpu.sync_static(&mut self.game.view);
        self.cpu.sync = t.elapsed().as_secs_f64() * 1e3;
        // The portraits the HUD asked for, made before the frame that shows them.
        gpu.render_portraits(&self.game.view, &self.overlay.portraits.due);

        let Some(surface) = &gpu.surface else { return };
        match surface.get_current_texture() {
            Ok(tex) => {
                let view = tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let t = Instant::now();
                gpu.draw(&view, &self.game.view, &self.frame, &self.overlay.verts, logical);
                self.cpu.record = t.elapsed().as_secs_f64() * 1e3;
                // The readout shows next frame (it is drawn into the HUD before this one is recorded).
                self.game.perf_lines = if self.game.show_fps {
                    let mut lines = crate::render::profile::readout(&self.cpu, &gpu.counts, gpu.timer.as_ref());
                    // What the run's save cost when it was last written (`game::persist`).
                    let p = &self.game.persist;
                    if p.writes > 0 {
                        lines.push(format!("save {:.2} ms, worst {:.2}, {} writes", p.last_ms, p.worst_ms, p.writes));
                    }
                    lines
                } else {
                    Vec::new()
                };
                if gpu.device_lost() {
                    // Lost during this frame's submit: presenting its texture, or dropping it
                    // (a discard), would end the process; it is let go with the old renderer,
                    // which the next frame replaces.
                    std::mem::forget(tex);
                    return;
                }
                window.pre_present_notify();
                tex.present();
            }
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                surface.configure(&gpu.device, &gpu.config);
            }
            Err(e) => eprintln!("surface: {e:?}"),
        }
    }

    fn window_icon() -> Option<Icon> {
        let s = sprite_from(&ragnar());
        let size = 64u32;
        let scale = (size as f32 / s.height as f32).floor().max(1.0) as u32;
        let ox = (size - s.width as u32 * scale) / 2;
        let oy = (size - s.height as u32 * scale) / 2;
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        for (y, row) in s.pixels.iter().enumerate() {
            for (x, c) in row.iter().enumerate() {
                let Some(c) = c else { continue };
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = ox + x as u32 * scale + dx;
                        let py = oy + y as u32 * scale + dy;
                        let i = ((py * size + px) * 4) as usize;
                        rgba[i..i + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, *c as u8, 255]);
                    }
                }
            }
        }
        Icon::from_rgba(rgba, size, size).ok()
    }
}

impl App {
    /// Takes the renderer's word as it starts: show the window once it is painted, and take the
    /// renderer when it is ready.
    #[cfg(not(target_arch = "wasm32"))]
    fn poll_starting(&mut self, el: &ActiveEventLoop) {
        let Some(rx) = self.starting.take() else { return };
        loop {
            match rx.try_recv() {
                Ok(Starting::Painted) => {
                    if let Some(w) = &self.window {
                        w.set_visible(true);
                    }
                }
                Ok(Starting::Ready(Ok(mut gpu))) => {
                    if self.opts.no_vsync {
                        gpu.set_vsync(false);
                    }
                    // The preset asked for (`--preset`), else the one chosen in the settings.
                    let chosen = crate::render::preset::Preset::named(self.game.settings.preset);
                    if let Some(p) =
                        self.opts.preset.or(chosen.map(|p| crate::render::preset::Preset { gpu_cull: gpu.preset().gpu_cull, ..p }))
                    {
                        gpu.set_preset(p);
                    }
                    self.game.settings.preset = gpu.preset().name;
                    if let Some(w) = &self.window {
                        // In case it was resized (or made fullscreen) while the renderer started.
                        let size = w.inner_size();
                        gpu.resize(size.width, size.height);
                        w.set_visible(true);
                    }
                    self.game.view.lod_bias = gpu.preset().lod_bias;
                    self.game.await_renderer = true;
                    eprintln!("renderer: {}", gpu.backend);
                    self.gpu = Some(*gpu);
                    self.last = Instant::now();
                    return;
                }
                Ok(Starting::Ready(Err(e))) => {
                    eprintln!("could not start the renderer: {e}");
                    el.exit();
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    eprintln!("could not start the renderer: its thread stopped");
                    el.exit();
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.starting = Some(rx);
                    return;
                }
            }
        }
    }
}

impl ApplicationHandler for App {
    fn exiting(&mut self, _el: &ActiveEventLoop) {
        // The run's snapshot, if anything is still marked (on the web, the page unloading), and
        // every write behind done before the game closes.
        self.game.flush_saves();
        crate::platform::finish_writes();
    }

    /// On the web, the page hidden into the browser's history (it may never come back).
    fn suspended(&mut self, _el: &ActiveEventLoop) {
        self.game.flush_saves();
    }

    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        #[allow(unused_mut)]
        let mut attrs = Window::default_attributes()
            .with_title("Adventures of the Ancient Blades")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(480.0, 270.0))
            .with_window_icon(Self::window_icon());
        // Hidden until the renderer has painted it, so it never opens as a white square.
        #[cfg(not(target_arch = "wasm32"))]
        let attrs = attrs.with_visible(false);
        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;
            attrs = attrs.with_append(true).with_prevent_default(true);
        }
        let window = match el.create_window(attrs) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                eprintln!("could not open a window: {e}");
                el.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        if self.fullscreen {
            self.apply_fullscreen();
        }

        // The renderer takes seconds to start (its shaders compile): on its own thread, so the
        // event loop keeps answering and Windows never calls the window not responding.
        self.start_renderer(&window);
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.resize(size.width, size.height);
                }
            }
            WindowEvent::ModifiersChanged(m) => self.modifiers = m.state(),
            WindowEvent::KeyboardInput { event, .. } => {
                let PhysicalKey::Code(code) = event.physical_key else { return };
                let down = event.state == ElementState::Pressed;
                if down && code == KeyCode::Enter && self.modifiers.alt_key() {
                    if !event.repeat {
                        self.toggle_fullscreen();
                    }
                    return;
                }
                if down {
                    if code == KeyCode::Escape {
                        // Esc hands the mouse back, as a browser does.
                        self.set_capture(false);
                    }
                    self.game.input.key_down(code, event.repeat);
                } else {
                    self.game.input.key_up(code);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                self.game.input.cursor_moved_to((position.x / scale) as f32, (position.y / scale) as f32);
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let down = state == ElementState::Pressed;
                if down && button == MouseButton::Left && matches!(self.game.screen, Screen::Playing | Screen::Ready) {
                    self.set_capture(true);
                }
                self.game.input.mouse_button(button, down);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                // A touchpad scrolls in pixels: forty of them make a notch.
                let lines = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y as f64,
                    MouseScrollDelta::PixelDelta(p) => p.y / 40.0,
                };
                self.game.input.wheel(lines);
            }
            WindowEvent::Touch(t) => {
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                self.game.input.touch(t.id, t.phase, t.location.x / scale, t.location.y / scale);
            }
            WindowEvent::Focused(false) => {
                self.set_capture(false);
                self.game.focus_lost();
            }
            WindowEvent::Focused(true) => self.game.focus_gained(),
            WindowEvent::RedrawRequested => {
                #[cfg(not(target_arch = "wasm32"))]
                if self.starting.is_some() {
                    return;
                }
                #[cfg(target_arch = "wasm32")]
                if self.gpu.is_none() {
                    if let Some(result) = self.pending.borrow_mut().take() {
                        match result {
                            Ok(gpu) => {
                                web_sys::console::log_1(&format!("renderer: {}", gpu.backend).into());
                                self.gpu = Some(gpu);
                            }
                            Err(e) => web_sys::console::error_1(&format!("could not start the renderer: {e}").into()),
                        }
                    }
                }
                self.frame();
                if self.game.quit_requested {
                    el.exit();
                    return;
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn device_event(&mut self, _el: &ActiveEventLoop, _id: DeviceId, event: DeviceEvent) {
        if let DeviceEvent::MouseMotion { delta } = event {
            self.game.input.mouse_motion(delta.0, delta.1);
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.starting.is_some() {
                self.poll_starting(el);
            }
            if self.starting.is_some() {
                // Idle while the renderer starts, rather than spin a core its compiler could use.
                el.set_control_flow(ControlFlow::WaitUntil(std::time::Instant::now() + std::time::Duration::from_millis(30)));
                return;
            }
            el.set_control_flow(ControlFlow::Poll);
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        #[cfg(target_arch = "wasm32")]
        let _ = el;
    }
}

pub fn run(opts: Options) {
    // The game's saves are written behind, off the frame (`platform::write_behind`).
    crate::platform::write_behind();
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    #[allow(unused_mut)]
    let mut app = App::new(opts);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = event_loop.run_app(&mut app);
        crate::platform::finish_writes();
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
}
