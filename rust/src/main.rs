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
    if let Some(i) = args.iter().position(|a| a == "--audio-sheet") {
        // --audio-sheet DIR: every breed's tell, swing, hit and death, the hits and the war drum, as WAVs.
        let dir = args.get(i + 1).map(String::as_str).unwrap_or("target/audio-sheet");
        std::process::exit(match void_harvest::audio::voices::write_sheet(dir) {
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
    if let Some(i) = args.iter().position(|a| a == "--crawl") {
        // Phase 4 of COMBAT.md: the bots crawl the Vaults of Mammon (N seeds; tiers 1, 4 and 7).
        let n: u32 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(2);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let seeds: Vec<u32> = (1..=n).collect();
        print!("{}", void_harvest::game::bot::crawl_report(&seeds, &[1, 4, 7], 900.0));
        return;
    }
    if args.iter().any(|a| a == "--drum-trace") {
        // WARBAND.md §6: the Drum through one crawl (the expert, Ragnar's tier-4 kit), second by second.
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        use void_harvest::game::bot::{crawl_traced, Loadout, Skill};
        let mut trace = Some(Vec::new());
        let c = crawl_traced(Skill::Expert, 1, Some(Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, 4)), 900.0, &mut trace);
        let mut csv = String::from("secs,intensity,phase\n");
        for (t, i, p) in trace.unwrap_or_default() {
            csv += &format!("{t:.0},{i:.3},{}\n", p.name());
        }
        let _ = std::fs::create_dir_all("perf");
        let _ = std::fs::write("perf/drum_trace.csv", csv);
        println!("perf/drum_trace.csv: at peak {:.0}% of the fighting, {} peaks, mean rest {:.1} s", c.peak_share * 100.0, c.peaks, c.rest);
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
    if let Some(i) = args.iter().position(|a| a == "--stage-three") {
        // COMBAT.md §10: the third stage's bouts, every bot skill, Ragnar's kit at tiers 1, 4 and 7, N seeds.
        let n: u32 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let seeds: Vec<u32> = (1..=n).collect();
        print!("{}", void_harvest::game::bot::stage_report(&seeds, 3, &[1, 4, 7]));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--bout") {
        // One bout, for a closer look: `--bout K SKILL TIER SEED` (K from 1, skill 0 novice, 1 adept,
        // 2 expert); with VH_TRACE set, every blow and wound on the hero is printed as it comes.
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (k, skill, tier, seed) = (num(1, 1) as usize, num(2, 2), num(3, 1) as u8, num(4, 1));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let kit = (tier > 1).then(|| void_harvest::game::bot::Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, tier));
        let r = void_harvest::game::bot::play_bout_as(k.saturating_sub(1), skill, seed, kit);
        println!(
            "{} {skill:?} tier {tier} seed {seed}: won {} in {:.1} s, {:?}",
            void_harvest::logic::gym::BOUTS[k.saturating_sub(1)].name,
            r.won,
            r.secs,
            r.stats
        );
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--rivals") {
        // WARBAND.md §15.2: RUNS crawls in a row with one Ledger: `--rivals RUNS SEED SKILL TIER`
        // (skill 0 novice, 1 adept, 2 expert).
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (runs, seed, skill, tier) = (num(1, 6), num(2, 1), num(3, 1), num(4, 1) as u8);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        print!("{}", void_harvest::game::bot::rivals_report(runs, seed, skill, tier));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--crawl-one") {
        // One crawl through the Vaults of Mammon: `--crawl-one SKILL TIER SEED` (skill 0 novice,
        // 1 adept, 2 expert); what is left standing at the end is listed.
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, tier, seed) = (num(1, 2), num(2, 1) as u8, num(3, 1));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        std::env::set_var("VH_CRAWL_LEFT", "1");
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let kit = Some(void_harvest::game::bot::Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, tier));
        let r = void_harvest::game::bot::crawl(skill, seed, kit, 900.0);
        println!("THE VAULTS {skill:?} tier {tier} seed {seed}: {r:?}");
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--court") {
        // Greed's prince in his court, played by a bot: `--court SKILL TIER SEED` (skill 0 novice,
        // 1 adept, 2 expert).
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, tier, seed) = (num(1, 2), num(2, 1) as u8, num(3, 1));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let kit = (tier > 1).then(|| void_harvest::game::bot::Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, tier));
        let r = void_harvest::game::bot::court_fight(skill, seed, kit, 240.0);
        println!("THE COUNTING HOUSE {skill:?} tier {tier} seed {seed}: {r:?}");
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--stage") {
        // COMBAT.md §11: stage S's bouts, every bot skill, Ragnar's kit at three tiers about the
        // bouts' own, N seeds: `--stage S N`.
        let stage: u8 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(4);
        let n: u32 = args.get(i + 2).and_then(|s| s.parse().ok()).unwrap_or(3);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let seeds: Vec<u32> = (1..=n).collect();
        let tiers = void_harvest::game::bot::stage_tiers(stage);
        print!("{}", void_harvest::game::bot::stage_report(&seeds, stage, &tiers));
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
    if args.iter().any(|a| a == "--perf") {
        std::process::exit(void_harvest::perf::run(&args));
    }
    if args.iter().any(|a| a == "--camera-check") {
        std::process::exit(void_harvest::camcheck::run(&args));
    }
    if args.iter().any(|a| a == "--capture") {
        std::process::exit(void_harvest::capture::run(&args));
    }
    let opts = Options {
        windowed: args.iter().any(|a| a == "--windowed"),
        fullscreen: args.iter().any(|a| a == "--fullscreen"),
        no_vsync: args.iter().any(|a| a == "--no-vsync"),
        preset: args
            .iter()
            .position(|a| a == "--preset")
            .and_then(|i| args.get(i + 1))
            .and_then(|name| void_harvest::render::preset::Preset::named(name))
            .or_else(|| args.iter().any(|a| a == "--gpu-cull").then(void_harvest::render::preset::Preset::default_for_build))
            .map(|p| void_harvest::render::preset::Preset { gpu_cull: p.gpu_cull || args.iter().any(|a| a == "--gpu-cull"), ..p }),
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
