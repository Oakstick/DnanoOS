//! Headless screenshots for checking builds without a person at the keyboard:
//!
//!   void-harvest --capture out.png [--size 1600x900] [--time 6.5]
//!                [--script "0.1:confirm,3:+up,4.2:-up,4.5:bomb"]
//!
//! The script is `seconds:command` pairs. Commands: an action (confirm,
//! attack, bomb, jump, camera, pause, tap), `+dir`/`-dir` to hold or release
//! up/down/left/right, and `look:dx:dy` for a mouse turn in pixels.

use crate::app::STEP;
use crate::game::Game;
use crate::input::Action;
use crate::logic::types::Dir;
use crate::render::gpu::Gpu;
use crate::render::hud;
use crate::render::overlay::Overlay;
use crate::render::view::Frame;

fn arg<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).map(|s| s.as_str())
}

enum Cmd {
    Nugget,
    Hurt,
    Press(Action),
    Hold(Dir, bool),
    Look(f64, f64),
}

fn parse_script(s: &str) -> Vec<(f64, Cmd)> {
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
            "camera" => Cmd::Press(Action::Camera),
            "pause" => Cmd::Press(Action::Pause),
            "tap" => Cmd::Press(Action::Tap),
            "fps" => Cmd::Press(Action::Fps),
            "nugget" => Cmd::Nugget,
            "hurt" => Cmd::Hurt,
            "look" => {
                let dx = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let dy = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
                Cmd::Look(dx, dy)
            }
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

pub fn run(args: &[String]) -> i32 {
    std::env::set_var("VH_NO_AUDIO", "1");
    // Keep test runs from touching the player's high score and settings.
    if std::env::var_os("VH_SAVE_DIR").is_none() {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("void-harvest-capture"));
    }
    let path = arg(args, "--capture").unwrap_or("capture.png").to_string();
    let (w, h) = arg(args, "--size")
        .and_then(|s| s.split_once('x'))
        .and_then(|(a, b)| Some((a.parse().ok()?, b.parse().ok()?)))
        .unwrap_or((1600u32, 900u32));
    let until: f64 = arg(args, "--time").and_then(|t| t.parse().ok()).unwrap_or(1.0);
    let script = parse_script(arg(args, "--script").unwrap_or(""));

    let mut gpu = match pollster::block_on(Gpu::new(None, (w, h))) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    eprintln!("renderer: {}", gpu.backend);
    let mut game = Game::new();
    game.viewport = (w as f32, h as f32);
    let mut next = 0;
    let mut t = 0.0;
    while t < until {
        while next < script.len() && script[next].0 <= t {
            match script[next].1 {
                Cmd::Press(a) => game.input.inject(a),
                Cmd::Hold(d, down) => game.input.hold_dir(d, down),
                Cmd::Look(dx, dy) => game.view.look(dx, dy),
                Cmd::Nugget => game.debug_collect_nugget(),
                Cmd::Hurt => game.debug_hurt(),
            }
            next += 1;
        }
        game.update(STEP);
        t += STEP;
    }

    let mut frame = Frame::default();
    let mut overlay = Overlay::new();
    game.view.aspect = w as f32 / h as f32;
    game.view.emit(&game.scene(), &mut frame);
    overlay.begin(w as f32, h as f32);
    if !args.iter().any(|a| a == "--no-hud") {
        hud::draw(&mut overlay, &game, 60);
    }
    gpu.sync_static(&mut game.view);
    let rgba = gpu.capture(&game.view, &frame, &overlay.verts, (w as f32, h as f32));
    match std::fs::write(&path, encode_png(w, h, &rgba)) {
        Ok(()) => {
            eprintln!("wrote {path} ({w}x{h}) at t={until:.2}s, screen {:?}", game.screen);
            0
        }
        Err(e) => {
            eprintln!("could not write {path}: {e}");
            1
        }
    }
}

// ------------------------------------------------------------------- icon

/// Writes the application icon (Sebastian's front view) as a multi-size .ico.
pub fn write_icon(path: &str) -> std::io::Result<()> {
    let sprite = crate::render::voxels::sprite_from(&crate::render::characters::sebastian());
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
