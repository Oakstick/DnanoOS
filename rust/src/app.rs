//! The window, the event loop and the frame: fixed 120 Hz simulation steps,
//! rendering as fast as the display allows.

use std::collections::HashSet;
use std::sync::Arc;

use web_time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{CursorGrabMode, Fullscreen, Icon, Window, WindowId};

use crate::game::{Game, Screen};
use crate::input::{Action, PadState};
use crate::logic::types::Dir;
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
    last: Instant,
    acc: f64,
    fps_frames: u32,
    fps_at: Instant,
    /// Where the last frame's CPU time went, for the F3 readout.
    cpu: crate::render::profile::CpuTimes,
    fps: u32,
    modifiers: ModifiersState,
    fullscreen: bool,
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
        Self {
            opts,
            window: None,
            gpu: None,
            #[cfg(target_arch = "wasm32")]
            pending: Default::default(),
            #[cfg(not(target_arch = "wasm32"))]
            starting: None,
            game: Game::new(),
            overlay: Overlay::new(),
            frame: Frame::default(),
            gilrs: gilrs::Gilrs::new().ok(),
            last: Instant::now(),
            acc: 0.0,
            fps_frames: 0,
            fps_at: Instant::now(),
            cpu: Default::default(),
            fps: 0,
            modifiers: ModifiersState::empty(),
            fullscreen,
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

    fn poll_pad(&mut self) -> Option<PadState> {
        use gilrs::{Axis, Button};
        let gilrs = self.gilrs.as_mut()?;
        while gilrs.next_event().is_some() {}
        let (_, pad) = gilrs.gamepads().find(|(_, g)| g.is_connected())?;
        let mut actions = HashSet::new();
        let map: [(Button, &[Action]); 12] = [
            // (LT and RT also page through the old man's wares.)
            (Button::South, &[Action::Jump]),
            (Button::West, &[Action::Attack]),
            (Button::RightTrigger2, &[Action::Attack, Action::TabNext]),
            (Button::East, &[Action::Guide]),
            (Button::LeftTrigger, &[Action::Block]),
            (Button::North, &[Action::NextPotion]),
            (Button::Start, &[Action::Confirm, Action::Pause]),
            (Button::Select, &[Action::Call]),
            (Button::LeftThumb, &[Action::Step]),
            (Button::RightThumb, &[Action::CameraReset]),
            (Button::RightTrigger, &[Action::Drink]),
            (Button::LeftTrigger2, &[Action::Angel, Action::TabPrev]),
        ];
        for (b, acts) in map {
            if pad.is_pressed(b) {
                actions.extend(acts.iter().copied());
            }
        }
        let dpad =
            [(Button::DPadUp, Dir::Up), (Button::DPadDown, Dir::Down), (Button::DPadLeft, Dir::Left), (Button::DPadRight, Dir::Right)]
                .into_iter()
                .find(|(b, _)| pad.is_pressed(*b))
                .map(|(_, d)| d);
        // gilrs reports y up; the game uses the web convention, y down.
        Some(PadState {
            actions,
            dpad,
            left: (pad.value(Axis::LeftStickX), -pad.value(Axis::LeftStickY)),
            right: (pad.value(Axis::RightStickX), -pad.value(Axis::RightStickY)),
        })
    }

    fn frame(&mut self) {
        let now = Instant::now();
        let dt = (now - self.last).as_secs_f64().min(0.25);
        self.last = now;
        self.acc += dt;

        let pad = self.poll_pad();
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
        if std::mem::take(&mut self.game.want_fullscreen_toggle) {
            self.toggle_fullscreen();
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

    fn render(&mut self) {
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
        let t = Instant::now();
        gpu.sync_static(&mut self.game.view);
        self.cpu.sync = t.elapsed().as_secs_f64() * 1e3;

        let Some(surface) = &gpu.surface else { return };
        match surface.get_current_texture() {
            Ok(tex) => {
                let view = tex.texture.create_view(&wgpu::TextureViewDescriptor::default());
                let t = Instant::now();
                gpu.draw(&view, &self.game.view, &self.frame, &self.overlay.verts, logical);
                self.cpu.record = t.elapsed().as_secs_f64() * 1e3;
                // The readout shows next frame (it is drawn into the HUD before this one is recorded).
                self.game.perf_lines = if self.game.show_fps {
                    crate::render::profile::readout(&self.cpu, &gpu.counts, gpu.timer.as_ref())
                } else {
                    Vec::new()
                };
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
                    if let Some(p) = self.opts.preset {
                        gpu.set_preset(p);
                    }
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
        self.game.score.save_high();
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

        #[cfg(not(target_arch = "wasm32"))]
        {
            // The renderer takes seconds to start (its shaders compile): on its own thread, so
            // the event loop keeps answering and Windows never calls the window not responding.
            // (Its surfaces are made here, where Windows gives out the window's handle.)
            let size = window.inner_size();
            let candidates = Gpu::surfaces(Some(&window));
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
            WindowEvent::Touch(t) => {
                let scale = self.window.as_ref().map(|w| w.scale_factor()).unwrap_or(1.0);
                self.game.input.touch(t.id, t.phase, t.location.x / scale, t.location.y / scale);
            }
            WindowEvent::Focused(false) => {
                self.set_capture(false);
                self.game.focus_lost();
            }
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
    let event_loop = EventLoop::new().expect("event loop");
    event_loop.set_control_flow(ControlFlow::Poll);
    #[allow(unused_mut)]
    let mut app = App::new(opts);
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = event_loop.run_app(&mut app);
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
    }
}
