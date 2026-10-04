//! Headless screenshots for checking builds without a person at the keyboard:
//!
//!   void-harvest --capture out.png [--size 1600x900] [--time 6.5]
//!                [--script "0.1:confirm,3:+up,4.2:-up,4.5:bomb"]
//!
//! The script is `seconds:command` pairs. Commands: an action (confirm,
//! attack, bomb, jump, camera, pause, tap), `+dir`/`-dir` to hold or release
//! up/down/left/right, `look:dx:dy` for a mouse turn in pixels, `nugget`,
//! `hurt`, `boss` (the F1 cheat), `wound:N` (N damage to the boss), `daze`,
//! `shop` (open the blacksmith), `coins:N`, `tab`, `drink`, and `potion:I`
//! (drink potion I of the shop's list), `arm:L:N` (weapon line L at level N,
//! equipped), `shield:I` (shield I, or none past the end) and `tour` (play
//! the first-run tour on the next confirm), `streak` (three quick kills) and
//! `goo` (a slime trail around Ragnar) and `gate` (the F2 cheat: every key, at the gate),
//! `foes:N[:S]` (N rows of one of each foe before him, of stage S if given, best in the Cyclops' open meadow),
//! `dungeon:N` (straight into dungeon N, 0 being the first), `map` (the map page), `hub`, `realm:N` and `court:N`
//! (`wound:N` and `slay` reach a prince in his court too; `prince:M` has him begin move M of his:
//! 0 sceptre, 1 coin toss, 2 grasp, 3 coin volley, 4 coin rain, 5 hoard slam)
//! (the Portals of Hell, and sin N's realm or its prince's court, 0 Pride to 6 Lust), `sins:N` (the keys held) and `at:X:Y`, `bless:N` (an archangel's blessing:
//! 0 Michael .. 3 Uriel) and `angel` (use it),
//! and `nav:left` / `nav:right` for a menu step (the character screen), and `beat:N:T` (Scene I
//! at beat N, T seconds in, its line shown in full; the story also plays from Play on the title,
//! where `pause` skips it).
//! `--iso` renders the whole level from an isometric map camera instead;
//! `--closeup [angle] [distance]` a portrait of the hero (angle in degrees around him, 0 = his front);
//! `--view x:y:z:angle:distance[:rise]` looks at a world point from `angle` degrees around it
//! (and `rise` times the distance above it, 0.28 unless given); `--bench N` then times N
//! more draws of the same frame; `--all-heroes` shows the locked heroes in colour on the
//! character screen. `--lookdev neutral|dungeon|realm:N` shows the Look Dev room (the material
//! chart) under that light; `--fx-off shadows,lights,grass,materials,fog,ao,bloom,dof,grade,prepass,mist`
//! switches effects off to see what each adds; `--frame-stats` lists what the view emitted;
//! `--lod-bias B` drops rigs' detail sooner (B > 1) or never (0), and `--lod-debug` tints each
//! level of detail (whole, half green, quarter red); `--light-heat` tints each pixel by how many
//! lights its cluster holds (blue none, green some, red 16 or more).
//! `juice:K` fires juice event K on the ring's nearest foe (WARBAND.md §4; 0 kick, 1 FOV punch,
//! 2-13 chips of each matter, 14 shatter, 15 shatter in slow motion, 16 sparks off the shield,
//! 17 a flinch, 18 the shield jolted: `game::juice::JUICE_EVENTS`), e.g.
//! `--script "0.1:ring:1,0.6:juice:14" --time 0.75`. `--fx-off juice` (or `dilation`, `hitstop`,
//! `shake`, `debris`, `springs`) switches the juice off. Captures never write the settings.

use crate::app::STEP;
use crate::audio::synth::Synth;
use crate::game::{Game, Screen, Stage};
use crate::input::Action;
use crate::logic::angels::{Blessing, ARCHANGELS};
use crate::logic::gear::{ShieldGear, ALL_POTIONS};
use crate::logic::items::DiamondKind;
use crate::logic::progression::TIERS;
use crate::logic::sins::{ALL_SINS, SINS};
use crate::logic::types::{Dir, P};
use crate::render::gpu::Gpu;
use crate::render::overlay::Overlay;
use crate::render::view::Frame;
use crate::render::{characters, hud, voxels};

pub(crate) fn arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

pub(crate) enum Cmd {
    Nugget,
    Hurt,
    Wound(i32),
    Daze,
    /// Sets him down on a tile.
    At(i32, i32),
    /// Into the Portals of Hell.
    Hub,
    /// The sins whose keys he holds, as a bit set (127 for all seven).
    Sins(u8),
    Realm(usize),
    Map,
    Gym,
    /// Into the gym's Sparring Ring, for bout N (1-based; 0 keeps the current one).
    Ring(usize),
    /// A bot plays from here on (0 novice, 1 adept, 2 expert).
    Bot(usize),
    Court(usize),
    /// A prince begins move M of his (see `Game::debug_prince_move`).
    Prince(usize),
    /// The Cyclops falls at once.
    Slay,
    /// `god`: god mode on (F5).
    God,
    Shop,
    Tour,
    Streak,
    Goo,
    Gate,
    Foes(usize, Option<u8>),
    Dungeon(usize),
    /// An archangel's blessing: 0 Michael, 1 Gabriel, 2 Raphael, 3 Uriel.
    Bless(usize),
    /// Every tree owned up to tier N, that weapon in hand and that shield on his back.
    Arm(u8),
    Shield(u8),
    Coins(u32),
    Runes(u32),
    /// The heroic path learned to level N.
    Path(u8),
    /// In the old man's wares: highlight row N.
    Row(usize),
    /// The old man's wares, on the loadout tab.
    Loadout,
    /// The patron archangel (0 Michael .. 3 Uriel), the outfit, the blade's edge in percent.
    Patron(usize),
    Outfit(u8),
    Edge(u8),
    /// The controls page, and its keyboard tab.
    Controls,
    Keyboard,
    /// The shield raised, and lowered.
    Unblock,
    /// Call the old man from the dungeon.
    Call,
    Potion(usize),
    Press(Action),
    Hold(Dir, bool),
    /// A menu step (the arrow keys' one-shot navigation).
    Nav(Dir),
    /// Scene I at a beat, seconds into it.
    Beat(usize, f64),
    Look(f64, f64),
    /// The chase camera held at a heading (degrees) and pitch (radians), as `--camera-check` reports poses.
    Aim(f32, f32),
    /// Juice event K (`game::juice::JUICE_EVENTS`) on the ring's nearest foe.
    Juice(usize),
}

pub(crate) fn parse_script(s: &str) -> Vec<(f64, Cmd)> {
    let mut out = Vec::new();
    for item in s.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let mut parts = item.split(':');
        let Some(t) = parts.next().and_then(|t| t.parse::<f64>().ok()) else { continue };
        let Some(cmd) = parts.next() else { continue };
        let dir = |d: &str| match d {
            "up" => Some(Dir::Up),
            "down" => Some(Dir::Down),
            "left" => Some(Dir::Left),
            "right" => Some(Dir::Right),
            _ => None,
        };
        let c = match cmd {
            "confirm" => Cmd::Press(Action::Confirm),
            "attack" => Cmd::Press(Action::Attack),
            "bomb" => Cmd::Press(Action::Bomb),
            "jump" => Cmd::Press(Action::Jump),
            "step" => Cmd::Press(Action::Step),
            "camera" => Cmd::Press(Action::Camera),
            "pause" => Cmd::Press(Action::Pause),
            "tap" => Cmd::Press(Action::Tap),
            "fps" => Cmd::Press(Action::Fps),
            "nugget" => Cmd::Nugget,
            "boss" => Cmd::Press(Action::Cheat),
            "daze" => Cmd::Daze,
            "at" => Cmd::At(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0), parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "hub" => Cmd::Hub,
            "map" => Cmd::Map,
            "gym" => Cmd::Gym,
            "ring" | "bout" => Cmd::Ring(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "bot" => Cmd::Bot(parts.next().and_then(|v| v.parse().ok()).unwrap_or(2)),
            "readout" => Cmd::Press(Action::Readout),
            "realm" => Cmd::Realm(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "court" => Cmd::Court(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "prince" => Cmd::Prince(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "sins" => Cmd::Sins(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "slay" => Cmd::Slay,
            "god" => Cmd::God,
            "shop" => Cmd::Shop,
            "tour" => Cmd::Tour,
            "streak" => Cmd::Streak,
            "goo" => Cmd::Goo,
            "gate" => Cmd::Gate,
            "foes" => Cmd::Foes(parts.next().and_then(|v| v.parse().ok()).unwrap_or(1), parts.next().and_then(|v| v.parse().ok())),
            "dungeon" => Cmd::Dungeon(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "bless" => Cmd::Bless(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "angel" => Cmd::Press(Action::Angel),
            "arm" => Cmd::Arm(parts.next().and_then(|v| v.parse().ok()).unwrap_or(1)),
            "shield" => Cmd::Shield(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "runes" => Cmd::Runes(parts.next().and_then(|v| v.parse().ok()).unwrap_or(10)),
            "path" => Cmd::Path(parts.next().and_then(|v| v.parse().ok()).unwrap_or(10)),
            "loadout" => Cmd::Loadout,
            "patron" => Cmd::Patron(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "outfit" => Cmd::Outfit(parts.next().and_then(|v| v.parse().ok()).unwrap_or(1)),
            "edge" => Cmd::Edge(parts.next().and_then(|v| v.parse().ok()).unwrap_or(10)),
            "controls" => Cmd::Controls,
            "keyboard" => Cmd::Keyboard,
            "block" => Cmd::Press(Action::Block),
            "unblock" => Cmd::Unblock,
            "row" => Cmd::Row(parts.next().and_then(|v| v.parse().ok()).unwrap_or(1)),
            "call" => Cmd::Call,
            "tab" => Cmd::Press(Action::NextPotion),
            "drink" => Cmd::Press(Action::Drink),
            "coins" => Cmd::Coins(parts.next().and_then(|v| v.parse().ok()).unwrap_or(500)),
            "potion" => Cmd::Potion(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "wound" => Cmd::Wound(parts.next().and_then(|v| v.parse().ok()).unwrap_or(10)),
            "hurt" => Cmd::Hurt,
            "juice" => Cmd::Juice(parts.next().and_then(|v| v.parse().ok()).unwrap_or(0)),
            "aim" => {
                let yaw: f32 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let pitch = parts.next().and_then(|v| v.parse().ok()).unwrap_or(crate::render::view::REST_PITCH);
                Cmd::Aim(yaw.to_radians(), pitch)
            }
            "look" => {
                let dx = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let dy = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                Cmd::Look(dx, dy)
            }
            "beat" => {
                let n = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                let at = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                Cmd::Beat(n, at)
            }
            "nav" => match parts.next().and_then(dir) {
                Some(d) => Cmd::Nav(d),
                None => continue,
            },
            c if c.starts_with('+') || c.starts_with('-') => match dir(&c[1..]) {
                Some(d) => Cmd::Hold(d, c.starts_with('+')),
                None => continue,
            },
            _ => continue,
        };
        out.push((t, c));
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    out
}

/// Carries out one script command on the game (`bot` takes over the pad for `bot:K`).
pub(crate) fn exec(game: &mut Game, cmd: &Cmd, bot: &mut Option<crate::game::bot::Bot>) {
    match *cmd {
        Cmd::Press(a) => game.input.inject(a),
        Cmd::Hold(d, down) => game.input.hold_dir(d, down),
        Cmd::Nav(d) => game.input.nav = Some(d),
        Cmd::Beat(n, at) => game.story_jump(n, at),
        Cmd::Look(dx, dy) => game.view.look(dx, dy),
        Cmd::Aim(yaw, pitch) => {
            game.view.cam_yaw = yaw;
            game.view.cam_pitch = pitch;
            game.view.mouse_look = true;
        }
        Cmd::Juice(k) => game.debug_juice(k),
        Cmd::Nugget => game.debug_collect_key(),
        Cmd::Hurt => game.debug_hurt(),
        Cmd::Wound(n) => game.debug_wound_boss(n),
        Cmd::Daze => game.debug_daze_boss(),
        Cmd::At(x, y) => game.hero.place(P::new(x, y)),
        Cmd::Hub => game.map_jump(Stage::Hub),
        Cmd::Map => game.open_map(),
        Cmd::Gym => game.map_jump(Stage::Gym),
        Cmd::Ring(n) => game.debug_ring(n),
        Cmd::Bot(k) => {
            game.hero.free_move = true;
            *bot = Some(crate::game::bot::Bot::new(crate::game::bot::Skill::ALL[k.min(2)], 1));
        }
        Cmd::Realm(n) => game.map_jump(Stage::Realm(SINS[n.min(6)])),
        Cmd::Court(n) => game.map_jump(Stage::Court(SINS[n.min(6)])),
        Cmd::Prince(m) => game.debug_prince_move(m),
        Cmd::Sins(n) => game.sins_cleared = n & ALL_SINS,
        Cmd::Slay => game.debug_wound_boss(1000),
        Cmd::God => game.input.inject(crate::input::Action::Immortal),
        Cmd::Shop => game.open_shop(),
        Cmd::Tour => game.tour_pending = true,
        Cmd::Streak => game.debug_streak(),
        Cmd::Goo => game.debug_goo(),
        Cmd::Gate => game.cheat_to_gate(),
        Cmd::Foes(rows, stage) => game.debug_foes(rows, stage),
        Cmd::Dungeon(n) => game.debug_dungeon(n),
        Cmd::Bless(n) => game.blessing = Some(Blessing::of(ARCHANGELS[n.min(3)])),
        Cmd::Arm(tier) => game.debug_arm(tier),
        Cmd::Shield(tier) => {
            game.hero.shield = (tier > 0).then(|| ShieldGear::new(tier.min(TIERS)));
        }
        Cmd::Coins(n) => game.progress.silver = n,
        Cmd::Runes(n) => game.progress.runes = n,
        Cmd::Path(n) => {
            let hero = game.hero_kind;
            game.progress.hero_mut(hero).owned[1] = n.min(TIERS);
            game.equip_from_progress();
        }
        Cmd::Row(n) => game.shop.row = n,
        Cmd::Loadout => {
            game.open_shop();
            game.shop.loadout = true;
            game.shop.row = 1;
        }
        Cmd::Patron(n) => {
            let hero = game.hero_kind;
            game.progress.hero_mut(hero).patron = Some(crate::logic::angels::ARCHANGELS[n.min(3)]);
        }
        Cmd::Outfit(n) => {
            let hero = game.hero_kind;
            game.progress.hero_mut(hero).outfit = n.min(3);
            game.equip_from_progress();
        }
        Cmd::Edge(n) => {
            let hero = game.hero_kind;
            let h = game.progress.hero_mut(hero);
            let k = (h.wield - 1) as usize;
            h.edge[k] = n.min(100);
            game.equip_from_progress();
        }
        Cmd::Controls => game.open_controls(),
        Cmd::Keyboard => game.controls_keyboard = true,
        Cmd::Unblock => game.input.inject_release(Action::Block),
        Cmd::Call => game.call_old_man(),
        // Drink brew number N (0 = tier 1 .. 9 = tier 10, 10 = an extra life) at once, to see its effect.
        Cmd::Potion(i) => game.debug_drink(ALL_POTIONS[i.min(10)]),
    }
}

pub fn run(args: &[String]) -> i32 {
    std::env::set_var("VH_NO_AUDIO", "1");
    // Keep test runs from touching the player's save, and start each from a clean one
    // (a saved adventure would put Continue first on the title). VH_SAVE_DIR keeps a save.
    if std::env::var_os("VH_SAVE_DIR").is_none() {
        let dir = std::env::temp_dir().join("void-harvest-capture");
        let _ = std::fs::remove_dir_all(&dir);
        std::env::set_var("VH_SAVE_DIR", dir);
    }
    let path = arg(args, "--capture").unwrap_or("capture.png").to_string();
    let (w, h) = arg(args, "--size")
        .and_then(|s| s.split_once('x'))
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .unwrap_or((1600u32, 900u32));
    let until: f64 = arg(args, "--time").and_then(|t| t.parse().ok()).unwrap_or(1.0);
    let script = parse_script(arg(args, "--script").unwrap_or(""));

    let started = std::time::Instant::now();
    let mut gpu = match pollster::block_on(Gpu::new(None, (w, h))) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    eprintln!("renderer: {} (ready in {:.0} ms)", gpu.backend, started.elapsed().as_secs_f64() * 1e3);
    gpu.fx_off = arg(args, "--fx-off").map(crate::render::profile::fx::parse).unwrap_or(0);
    if let Some(p) = arg(args, "--preset").and_then(crate::render::preset::Preset::named) {
        gpu.set_preset(p);
    }
    if args.iter().any(|a| a == "--gpu-cull") {
        let p = *gpu.preset();
        gpu.set_preset(crate::render::preset::Preset { gpu_cull: true, ..p });
    }
    if args.iter().any(|a| a == "--light-heat") {
        gpu.fx_off |= crate::render::profile::fx::LIGHT_HEAT;
    }
    let mut game = Game::new();
    // Fixed seeds, so the same script always renders the same picture.
    game.reseed(1);
    // Levels hold behind the loading screen (RP03); nothing is drawn until the capture, so this
    // shows that screen for a level loaded in the last ten seconds.
    game.await_renderer = args.iter().any(|a| a == "--await-renderer");
    game.viewport = (w as f32, h as f32);
    game.tour_pending = false;
    game.set_juice_off(gpu.fx_off);
    let mut next = 0;
    let mut t = 0.0;
    let mut bot: Option<crate::game::bot::Bot> = None;
    while t < until {
        while next < script.len() && script[next].0 <= t {
            exec(&mut game, &script[next].1, &mut bot);
            next += 1;
        }
        if let Some(b) = bot.as_mut() {
            let pad = b.act(&game);
            game.input.set_pad(Some(pad));
        }
        game.update(STEP);
        t += STEP;
    }

    let mut frame = Frame::default();
    let mut overlay = Overlay::new();
    game.view.aspect = w as f32 / h as f32;
    game.view.viewport_h = h as f32;
    game.view.lod_bias = arg(args, "--lod-bias").and_then(|b| b.parse().ok()).unwrap_or(gpu.preset().lod_bias);
    game.view.lod_debug = args.iter().any(|a| a == "--lod-debug");
    game.view.preview_heroes = args.iter().any(|a| a == "--all-heroes");
    let iso = args.iter().any(|a| a == "--iso");
    if iso {
        game.view.set_map_view(&game.level.grid);
    }
    if let Some(i) = args.iter().position(|a| a == "--closeup") {
        let deg: f32 = args.get(i + 1).and_then(|a| a.parse().ok()).unwrap_or(20.0);
        let dist: f32 = args.get(i + 2).and_then(|a| a.parse().ok()).unwrap_or(2.1);
        let p = game.hero.pos();
        let y = game.hero.altitude as f32 * crate::render::view::STEP;
        let face = match game.hero.facing {
            Dir::Up => std::f32::consts::PI,
            Dir::Down | Dir::None => 0.0,
            Dir::Left => -std::f32::consts::FRAC_PI_2,
            Dir::Right => std::f32::consts::FRAC_PI_2,
        };
        game.view.set_closeup(
            glam::Vec3::new(p.x as f32, y + 0.62 + (2.1 - dist).max(0.0) * 0.12, p.y as f32),
            face + deg.to_radians(),
            dist,
        );
    }
    if let Some(v) = arg(args, "--view") {
        let n: Vec<f32> = v.split(':').filter_map(|s| s.parse().ok()).collect();
        if n.len() >= 5 {
            let target = glam::Vec3::new(n[0], n[1], n[2]);
            game.view.set_closeup(target, n[3].to_radians(), n[4]);
            if let Some(&rise) = n.get(5) {
                game.view.set_closeup_rise(target, n[3].to_radians(), n[4], rise);
            }
        }
    }
    if let Some(light) = arg(args, "--lookdev").and_then(crate::render::view::LookLight::parse) {
        game.view.open_lookdev(light);
        if let Some(v) = arg(args, "--view") {
            let n: Vec<f32> = v.split(':').filter_map(|s| s.parse().ok()).collect();
            if n.len() >= 5 {
                game.view.set_closeup_rise(glam::Vec3::new(n[0], n[1], n[2]), n[3].to_radians(), n[4], n.get(5).copied().unwrap_or(0.28));
            }
        }
    }
    game.view.emit(&game.scene(), &mut frame);
    if args.iter().any(|a| a == "--frame-stats") {
        // What the view emitted this frame, by list and shade mode: where the per-frame upload goes.
        let mut by: std::collections::BTreeMap<(String, i32), usize> = std::collections::BTreeMap::new();
        let lists = frame
            .opaque
            .iter()
            .enumerate()
            .map(|(k, l)| (format!("opaque mesh {k}"), l))
            .chain(frame.transparent.iter().enumerate().map(|(k, l)| (format!("see-through mesh {k}"), l)));
        for (name, list) in lists.chain([
            ("early".to_string(), &frame.transparent_early),
            ("additive".into(), &frame.additive),
            ("glow".into(), &frame.glow),
        ]) {
            for i in list {
                *by.entry((name.clone(), i.emissive[3] as i32)).or_default() += 1;
            }
        }
        for ((name, shade), n) in by {
            eprintln!("frame: {name:>20} shade {shade:>2}: {n}");
        }
        eprintln!("frame: {} solid rigs, {} see-through", frame.rigs.len(), frame.rigs_blend.len());
    }
    // A story shot sets its own camera as it draws; aim again afterwards so `--view` can frame the cast.
    if let Some(v) = arg(args, "--view") {
        let n: Vec<f32> = v.split(':').filter_map(|s| s.parse().ok()).collect();
        if n.len() >= 5 && game.screen == Screen::Story {
            let target = glam::Vec3::new(n[0], n[1], n[2]);
            game.view.set_closeup_rise(target, n[3].to_radians(), n[4], n.get(5).copied().unwrap_or(0.28));
        }
    }
    if iso {
        // Only the lights near Ragnar reach the GPU; lift the ambient so the far corners read.
        let a = &mut frame.atmosphere;
        a.ambient *= 3.0;
        a.hemi_sky *= 2.0;
        a.hemi_ground *= 2.5;
    }
    overlay.begin(w as f32, h as f32);
    if !args.iter().any(|a| a == "--no-hud") {
        hud::draw(&mut overlay, &game, 60);
    }
    gpu.sync_static(&mut game.view);
    let mut rgba = gpu.capture(&game.view, &frame, &overlay.verts, (w as f32, h as f32));
    if game.show_fps && !args.iter().any(|a| a == "--no-hud") {
        // The F3 readout reports the frame before: draw once more with this one's numbers.
        let _ = gpu.device.poll(wgpu::PollType::Wait);
        if let Some(t) = gpu.timer.as_mut() {
            t.collect(&gpu.device);
        }
        game.perf_lines = crate::render::profile::readout(&Default::default(), &gpu.counts, gpu.timer.as_ref());
        overlay.begin(w as f32, h as f32);
        hud::draw(&mut overlay, &game, 60);
        rgba = gpu.capture(&game.view, &frame, &overlay.verts, (w as f32, h as f32));
    }
    if let Some(n) = arg(args, "--bench").and_then(|n| n.parse::<u32>().ok()) {
        let ms = gpu.bench(&game.view, &frame, &overlay.verts, (w as f32, h as f32), n);
        eprintln!("bench: {ms:.2} ms a frame over {n} frames ({:.0} fps)", 1000.0 / ms);
        if let Some(t) = gpu.timer.as_mut() {
            t.collect(&gpu.device);
        }
        let cpu = crate::render::profile::CpuTimes::default();
        for line in crate::render::profile::readout(&cpu, &gpu.counts, gpu.timer.as_ref()).iter().skip(1) {
            eprintln!("bench: {line}");
        }
    }
    match std::fs::write(&path, encode_png(w, h, &rgba)) {
        Ok(()) => {
            let dynamic: usize = frame.opaque.iter().chain(frame.transparent.iter()).map(|l| l.len()).sum::<usize>()
                + frame.additive.len()
                + frame.glow.len();
            let at = game.hero.pos();
            let boss = game.boss.as_ref().map(|b| format!(", boss at {:.1},{:.2},{:.1}", b.pos.x, b.hover, b.pos.y)).unwrap_or_default();
            let shrines: Vec<String> =
                game.diamonds.iter().filter(|d| d.kind == DiamondKind::Angel).map(|d| format!("{},{}", d.tile.x, d.tile.y)).collect();
            let shrines = if shrines.is_empty() { String::new() } else { format!(", archangels' shrines at {}", shrines.join(" ")) };
            let shrines = shrines + &game.red_key.map(|k| format!(", red key at {},{}", k.x, k.y)).unwrap_or_default();
            eprintln!(
                "wrote {path} ({w}x{h}) at t={until:.2}s, screen {:?}, {} static instances, {dynamic} drawn this frame, hero at {:.1},{:.1}{boss}{shrines}",
                game.screen,
                game.view.static_cubes.len(),
                at.x,
                at.y
            );
            0
        }
        Err(e) => {
            eprintln!("could not write {path}: {e}");
            1
        }
    }
}

// ------------------------------------------------------------------ music

/// Renders the soundtrack to a 16-bit mono WAV, for listening outside the game.
/// `intensity` 0..1 previews the danger layers.
pub fn write_music(path: &str, seconds: f64, intensity: f64) -> std::io::Result<()> {
    let rate = 48_000u32;
    let mut synth = Synth::new(rate as f64, 0.42);
    synth.music.running = true;
    synth.intensity = intensity;
    let mut buf = vec![0.0f32; (rate as f64 * seconds) as usize];
    // Render in callback-sized blocks, as the audio device would, so finished notes are retired.
    for block in buf.chunks_mut(512) {
        synth.render(block, 1);
    }
    // Normalized for listening; in the game the music sits under the effects.
    let peak = buf.iter().fold(1e-6f32, |m, v| m.max(v.abs()));
    let gain = 0.7 / peak;
    let data: Vec<u8> = buf.iter().flat_map(|v| (((v * gain).clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes()).collect();
    let mut out = Vec::with_capacity(44 + data.len());
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&data);
    std::fs::write(path, out)
}

// ------------------------------------------------------------------- icon

/// Writes the application icon (Ragnar's front view) as a multi-size .ico.
pub fn write_icon(path: &str) -> std::io::Result<()> {
    let sprite = voxels::sprite_from(&characters::ragnar());
    let sizes = [16u32, 24, 32, 48, 64, 128, 256];
    let mut images = Vec::new();
    for &size in &sizes {
        // Integer scaling keeps the pixel art crisp; small sizes may crop a margin only.
        let scale = ((size as f32 * 0.92) / sprite.height as f32).floor().max(1.0) as u32;
        let ox = (size as i64 - (sprite.width as u32 * scale) as i64) / 2;
        let oy = (size as i64 - (sprite.height as u32 * scale) as i64) / 2;
        let mut rgba = vec![0u8; (size * size * 4) as usize];
        for (y, row) in sprite.pixels.iter().enumerate() {
            for (x, c) in row.iter().enumerate() {
                let Some(c) = c else { continue };
                for dy in 0..scale {
                    for dx in 0..scale {
                        let px = ox + (x as u32 * scale + dx) as i64;
                        let py = oy + (y as u32 * scale + dy) as i64;
                        if px < 0 || py < 0 || px >= size as i64 || py >= size as i64 {
                            continue;
                        }
                        let i = ((py as u32 * size + px as u32) * 4) as usize;
                        rgba[i..i + 4].copy_from_slice(&[(c >> 16) as u8, (c >> 8) as u8, *c as u8, 255]);
                    }
                }
            }
        }
        images.push((size, encode_png(size, size, &rgba)));
    }
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let mut offset = 6 + 16 * images.len() as u32;
    for (size, png) in &images {
        let dim = if *size >= 256 { 0 } else { *size as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(png.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += png.len() as u32;
    }
    for (_, png) in &images {
        out.extend_from_slice(png);
    }
    std::fs::write(path, out)
}

// --------------------------------------------------------------- tiny PNG

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xedb8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    let mut c = 0xffff_ffffu32;
    for &b in data {
        c = table[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

/// RGBA8 to PNG with stored (uncompressed) deflate blocks: big, but no dependencies.
pub fn encode_png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity((w * h * 4 + h) as usize);
    for y in 0..h as usize {
        raw.push(0);
        raw.extend_from_slice(&rgba[y * w as usize * 4..(y + 1) * w as usize * 4]);
    }
    let mut z = vec![0x78, 0x01];
    let mut chunks = raw.chunks(65_535).peekable();
    while let Some(c) = chunks.next() {
        z.push(if chunks.peek().is_none() { 1 } else { 0 });
        let len = c.len() as u16;
        z.extend_from_slice(&len.to_le_bytes());
        z.extend_from_slice(&(!len).to_le_bytes());
        z.extend_from_slice(c);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in &raw {
        a = (a + byte as u32) % 65_521;
        b = (b + a) % 65_521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    let mut chunk = |kind: &[u8; 4], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    };
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &ihdr);
    chunk(b"IDAT", &z);
    chunk(b"IEND", &[]);
    out
}
