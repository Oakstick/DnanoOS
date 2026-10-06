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
    if let Some(i) = args.iter().position(|a| a == "--export-rig") {
        // --export-rig HERO OUT.glb: the jointed hero (ragnar, or one with a joint table) for Blender (ANIMATION.md).
        let hero = args.get(i + 1).map(String::as_str).unwrap_or("ragnar");
        let path = args.get(i + 2).map(String::as_str).unwrap_or("ragnar.glb");
        std::process::exit(match void_harvest::render::anim::export::export_rig(hero, path) {
            Ok(line) => {
                println!("{line}");
                0
            }
            Err(e) => {
                eprintln!("{e}");
                1
            }
        });
    }
    if args.iter().any(|a| a == "--anim-bench") {
        // The clips: how long they take to read, and what sampling and posing him costs a frame.
        print!("{}", void_harvest::render::anim::export::bench());
        return;
    }
    if args.iter().any(|a| a == "--blade-view") {
        // The heroes' strikes: on how many of their active frames the chase camera sees the blade.
        print!("{}", void_harvest::render::anim::export::blade_view());
        return;
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
    if let Some(i) = args.iter().position(|a| a == "--general") {
        // WARBAND.md §15.2: one realm with its general: `--general SIN SKILL TIER SEED` (sin 0 Pride
        // .. 4 Greed .. 6 Lust; skill 0 novice, 1 adept, 2 expert), fought and then left alive.
        use void_harvest::game::bot::{general_report, Skill};
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let num = |k: usize, d: u32| args.get(i + k).and_then(|v| v.parse().ok()).unwrap_or(d);
        let sin = void_harvest::logic::sins::SINS[(num(1, 4) as usize).min(6)];
        let skill = Skill::ALL[(num(2, 2) as usize).min(2)];
        print!("{}", general_report(sin, skill, num(3, 4) as u8, num(4, 1), 900.0));
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
        // A sin's prince in his court, played by a bot: `--court SIN SKILL TIER SEED` (SIN a sin's
        // name: greed, pride, envy, wrath, sloth, gluttony, lust; skill 0 novice, 1 adept, 2
        // expert). Without a sin's name, `--court SKILL TIER SEED` is Greed's.
        let named = args.get(i + 1).and_then(|s| void_harvest::logic::sins::Sin::from_name(s));
        let sin = named.unwrap_or(void_harvest::logic::sins::Sin::Greed);
        let i = if named.is_some() { i + 1 } else { i };
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, tier, seed) = (num(1, 2), num(2, 1) as u8, num(3, 1));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let kit = (tier > 1).then(|| void_harvest::game::bot::Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, tier));
        let r = void_harvest::game::bot::court_fight(sin, skill, seed, kit, 240.0);
        let name = void_harvest::logic::sins::Sin::prince(sin);
        println!("THE COURT OF {} {skill:?} tier {tier} seed {seed}: {r:?}", name.to_uppercase());
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--pit") {
        // The Pit ridden by a bot: `--pit SKILL TIER SEED` (skill 0 novice, 1 adept, 2 expert; TIER
        // its kit's), from level 1 or `--pit-level N`, each level's numbers reported.
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, tier, seed) = (num(1, 2), num(2, 7) as u8, num(3, 1));
        let from = args.iter().position(|a| a == "--pit-level").and_then(|k| args.get(k + 1)).and_then(|s| s.parse().ok()).unwrap_or(1u8);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        print!("{}", void_harvest::game::bot::pit_report(skill, tier, seed, from.clamp(1, 7)));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--lair") {
        // Krampus in his lair at the bottom of the Pit, played by a bot: `--lair SKILL TIER SEED`
        // (skill 0 novice, 1 adept, 2 expert; Ragnar's kit at the tier, all seven keys held).
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, tier, seed) = (num(1, 2), num(2, 8) as u8, num(3, 1));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let kit = Some(void_harvest::game::bot::Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, tier));
        let r = void_harvest::game::bot::lair_fight(skill, seed, kit, 360.0);
        println!("KRAMPUS'S LAIR {skill:?} tier {tier} seed {seed}: {r:?}");
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--run") {
        // WARBAND.md W9: the whole run by one bot, one save carried through it: `--run SKILL SEED
        // [ORDER]` (skill 0 novice, 1 adept, 2 expert; order 0 the vaults first, 1 the reverse).
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, seed, order) = (num(1, 2), num(2, 1), num(3, 0));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join(format!("vh-run-{skill}-{seed}-{order}")));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let r = void_harvest::game::whole_run::whole_run(skill, seed, order as usize);
        print!("{}", void_harvest::game::whole_run::run_table(&r));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--runs") {
        // WARBAND.md W9: `--runs SKILL N [ORDER]`: the whole run by one skill on seeds 1 to N, each
        // run's end and the parts side by side.
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, n, order) = (num(1, 2), num(2, 3), num(3, 0));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join(format!("vh-runs-{skill}-{order}")));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let runs: Vec<_> = (1..=n).map(|seed| void_harvest::game::whole_run::whole_run(skill, seed, order as usize)).collect();
        print!("{}", void_harvest::game::whole_run::runs_summary(&runs));
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--court-at") {
        // A prince as the run meets him: `--court-at SIN SKILL KIT KEYS SEED` (Ragnar's kit at KIT,
        // KEYS other keys held: the court weighed at tier KEYS + 1).
        let sin =
            args.get(i + 1).and_then(|s| void_harvest::logic::sins::Sin::from_name(s)).unwrap_or(void_harvest::logic::sins::Sin::Greed);
        let num = |k: usize, d: u32| args.get(i + k).and_then(|s| s.parse().ok()).unwrap_or(d);
        let (skill, kit, keys, seed) = (num(2, 2), num(3, 1) as u8, num(4, 0) as usize, num(5, 1));
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let skill = void_harvest::game::bot::Skill::ALL[(skill as usize).min(2)];
        let l = void_harvest::game::bot::Loadout::all(void_harvest::logic::heroes::HeroKind::Ragnar, kit);
        let limit = std::env::var("VH_LIMIT").ok().and_then(|v| v.parse().ok()).unwrap_or(240.0);
        let (slain, secs, phase, left, lost) = void_harvest::game::whole_run::court_at(sin, skill, seed, l, keys, limit);
        println!(
            "{} {skill:?} kit {kit} tier {}: slain {slain} in {secs:.0} s, phase {phase}, {:.0}% left, {lost} lives lost",
            sin.prince(),
            keys + 1,
            left * 100.0
        );
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
    if let Some(i) = args.iter().position(|a| a == "--mastery") {
        // WARBAND.md §11.1: every hero's mastery at rank 0 and 5, the novice and the expert, N seeds.
        let n: u32 = args.get(i + 1).and_then(|s| s.parse().ok()).unwrap_or(3);
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-balance"));
        let seeds: Vec<u32> = (1..=n).collect();
        print!("{}", void_harvest::game::bot::mastery_sweep(&seeds));
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
        dev: args.iter().any(|a| a == "--dev"),
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
