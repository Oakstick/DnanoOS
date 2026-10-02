// No console window behind the game in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(target_arch = "wasm32")]
fn main() {}

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    use void_harvest::app::{self, Options};
    install_crash_log();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Some(i) = args.iter().position(|a| a == "--write-icon") {
        let path = args.get(i + 1).map(String::as_str).unwrap_or("assets/icon.ico");
        std::process::exit(match void_harvest::capture::write_icon(path) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    if let Some(i) = args.iter().position(|a| a == "--write-music") {
        // --write-music out.wav [seconds] [intensity]
        let path = args.get(i + 1).map(String::as_str).unwrap_or("music.wav");
        let secs = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(40.0);
        let intensity = args.get(i + 3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
        std::process::exit(match void_harvest::capture::write_music(path, secs, intensity) {
            Ok(()) => 0,
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    if let Some(i) = args.iter().position(|a| a == "--kit-parts") {
        let tier: u8 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(4);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        for hero in void_harvest::logic::heroes::ROSTER {
            print!("{}", void_harvest::game::bot::kit_parts(hero, tier, &[1, 2, 3]));
        }
        return;
    }
    if args.iter().any(|a| a == "--kit-table") {
        print!("{}", void_harvest::game::bot::kit_table());
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--kits") {
        // Phase 3 of COMBAT.md: every hero's kit at tiers 1, 4, 7 and 10, played by the bots.
        let n: u32 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let seeds: Vec<u32> = (1..=n).collect();
        print!("{}", void_harvest::game::bot::kit_sweep(&seeds));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--balance") {
        // The balance harness (COMBAT.md phase 2): every bout of the Sparring Ring, every bot skill, N seeds.
        let n: u32 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let seeds: Vec<u32> = (1..=n).collect();
        print!("{}", void_harvest::game::bot::balance(&seeds));
        return;
    }
    if args.iter().any(|a| a == "--capture") {
        std::process::exit(void_harvest::capture::run(&args));
    }
    let opts = Options {
        windowed: args.iter().any(|a| a == "--windowed"),
        fullscreen: args.iter().any(|a| a == "--fullscreen"),
        no_vsync: args.iter().any(|a| a == "--no-vsync"),
    };
    app::run(opts);
}

#[cfg(not(target_arch = "wasm32"))]
/// A release build has no console, so a panic is written next to the save file.
fn install_crash_log() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(dir) = void_harvest::platform::save_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("crash.log"), format!("{info}\n"));
        }
        default(info);
    }));
}
