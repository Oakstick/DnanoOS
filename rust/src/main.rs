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
