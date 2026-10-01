//! The game's rules, played through `Game::update` as the player would.

use super::feedback::war_cry;
use super::*;
use crate::game::hub::SEAL_SECONDS;
use crate::input::PadState;
use crate::logic;
use crate::logic::enemy::{enemy_specs, EnemyMode, REASSEMBLE_SECONDS, WOLF_FURY_SPEED};
use crate::logic::gear::{ShieldGear, COINS_BOSS, RUNES_BOSS, RUNES_SHRINE};
use crate::logic::hero::{CONTACT_DAMAGE, HEALTH_POTION};
use crate::logic::heroes::{HeroKind, MIYAMOTO_TEMPO, ROSTER};
use crate::logic::items::{DiamondKind, Offer, RangedKind, WeaponKind};
use crate::logic::levels::{ARENA_GATE, ARENA_HERO};
use crate::logic::progression::{self, Tree};
use crate::logic::sins::Sin;
use crate::logic::story::SCENE_ONE;
use crate::logic::systems::POINTS_NODULE;
use crate::logic::types::{dist_sq, Tile, P};
use crate::logic::{gym, pathing};
use crate::render::{hud, view};

/// Play on the title, then begin with the hero on the character screen (Ragnar).
fn start_from_title(g: &mut Game) {
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Select);
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
}

fn playing() -> Game {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    start_from_title(&mut g);
    for _ in 0..(READY_SECONDS / crate::app::STEP) as usize + 2 {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Playing);
    g
}

#[test]
fn the_character_screen_offers_three_heroes_and_each_can_begin() {
    let mut g = Game::new();
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Select);
    assert_eq!(g.select_hero(), HeroKind::Ragnar, "Ragnar is highlighted first");
    // Left of Ragnar stands Mehmed, right Miyamoto; each of the three can begin.
    g.input.nav = Some(Dir::Left);
    g.update(crate::app::STEP);
    assert_eq!(g.select_hero(), HeroKind::Mehmed);
    for hero in ROSTER {
        g.enter(Screen::Title);
        g.open_select();
        g.select_index = hero.slot();
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
        assert_eq!(g.screen, Screen::Ready, "{hero:?} begins");
        assert_eq!(g.hero_kind, hero);
        assert_eq!(g.hero.kind, hero, "and plays with his own armory");
    }
    g.enter(Screen::Title);
    g.open_select();
    // Esc goes back to the title.
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Title);
}

#[test]
fn play_opens_scene_one_which_plays_every_beat_then_the_heroes() {
    use SCENE_ONE;
    let mut g = Game::new();
    g.intro = true;
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Story, "Play starts the story");
    let mut seen: Vec<usize> = Vec::new();
    for _ in 0..(300.0 / crate::app::STEP) as usize {
        if g.screen != Screen::Story {
            break;
        }
        let st = g.story.as_ref().unwrap();
        if seen.last() != Some(&st.beat) {
            seen.push(st.beat);
        }
        // Wait out the action; press on through the lines.
        if st.beat().waits() && st.t > 0.5 {
            g.input.inject(Action::Confirm);
        }
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Select, "the heroes are chosen after the story");
    assert_eq!(seen, (0..SCENE_ONE.len()).collect::<Vec<_>>(), "every beat plays, in order");
    assert!(g.story.is_none());
}

#[test]
fn a_press_shows_the_whole_line_before_moving_on_and_esc_skips_the_story() {
    use logic::story::{first, Shot};
    let mut g = Game::new();
    g.story_jump(first(Shot::Souls).unwrap(), 0.0);
    let st = g.story.as_mut().unwrap();
    st.shown = 0.0;
    for _ in 0..40 {
        g.update(crate::app::STEP);
    }
    let st = g.story.as_ref().unwrap();
    assert!(st.shown > 0.0 && !st.line_done(), "the line types itself out");
    let beat = st.beat;
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    let st = g.story.as_ref().unwrap();
    assert!(st.line_done() && st.beat == beat, "the first press shows the rest of the line");
    assert_eq!(st.souls, 0b111, "all three warriors' souls rise once they are named");
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.story.as_ref().unwrap().beat, beat + 1, "the next press moves on");
    // Esc skips what is left, straight to the character screen.
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    for _ in 0..(1.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Select);
}

#[test]
fn every_key_enrages_the_dungeon() {
    let mut g = playing();
    assert_eq!(g.rage(), 0.0);
    let (calm_wait, calm_cap) = (g.spawn_every(), g.max_enemies());
    let before = g.enemies.len();
    // Take a key: a reinforcement is on its way within moments.
    let o = g.hero.occupied();
    g.level.grid.set(o.x, o.y, Tile::Nodule);
    g.collect();
    assert!(g.rage() > 0.0);
    assert!(g.spawn_timer <= 1.5, "someone comes at once");
    for _ in 0..(2.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert!(g.enemies.len() > before, "a new foe arrived");
    // Half the keys: shorter waits, more foes allowed; and they never scatter.
    g.keys_got = g.keys_total / 2;
    assert!(g.spawn_every() < calm_wait && g.max_enemies() > calm_cap);
    g.global_mode = GlobalMode::Scatter;
    g.mode_index = 0;
    g.update(crate::app::STEP);
    assert_eq!(g.global_mode, GlobalMode::Chase, "they hunt without a break");
}

#[test]
fn every_second_key_quickens_the_music() {
    let mut g = playing();
    for n in 1..=7u32 {
        g.keys_got = n - 1;
        // Put a nugget under him and let him pick it up.
        let o = g.hero.occupied();
        g.level.grid.set(o.x, o.y, Tile::Nodule);
        g.collect();
        assert_eq!(g.audio.tempo_step(), n / KEYS_PER_TEMPO_STEP, "after {n} keys");
    }
}

#[test]
fn ragnar_starts_in_the_middle_after_a_countdown() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    let spawn = g.level.hero_spawn;
    let (w, h) = (g.level.grid.width, g.level.grid.height);
    assert!((spawn.x - (w - 1) / 2).abs() <= 2 && (spawn.y - (h - 1) / 2).abs() <= 2, "spawn {spawn:?}");
    // Every pickup is still reachable from the new start.
    let seen = pathing::reachable(&g.level.grid, spawn, true, false);
    for (x, y, t) in g.level.grid.cells() {
        if t == Tile::Nodule || t == Tile::Charge {
            assert!(seen.contains(&P::new(x, y)), "pickup at {x},{y} unreachable from the center");
        }
    }
    start_from_title(&mut g);
    assert_eq!(g.screen, Screen::Ready);
    for _ in 0..(2.9 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Ready, "still counting down at 2.9 s");
    for _ in 0..(0.2 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Playing);
    assert_eq!(g.hero.occupied(), spawn);
}

#[test]
fn the_right_stick_turns_the_camera_and_it_stays_put() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = playing();
    let start = g.view.cam_yaw;
    let pad = |x: f32, y: f32| PadState { right: (x, y), ..Default::default() };
    g.input.set_pad(Some(pad(1.0, 0.0)));
    for _ in 0..60 {
        g.update(crate::app::STEP);
    }
    let turned = g.view.cam_yaw;
    assert!(turned - start > 0.5, "stick barely turned the camera: {}", turned - start);
    // Let go and wait: the camera keeps the angle instead of drifting back behind him.
    g.input.set_pad(Some(pad(0.0, 0.0)));
    for _ in 0..(3.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert!((g.view.cam_yaw - turned).abs() < 1e-3, "camera drifted");
    // Tilting the stick up and down pitches it.
    let pitch = g.view.cam_pitch;
    g.input.set_pad(Some(pad(0.0, -1.0)));
    for _ in 0..20 {
        g.update(crate::app::STEP);
    }
    assert!(g.view.cam_pitch != pitch);
    // R3 swings it back behind him.
    let mut click = pad(0.0, 0.0);
    click.actions.insert(Action::CameraReset);
    g.input.set_pad(Some(click));
    g.update(crate::app::STEP);
    g.input.set_pad(Some(pad(0.0, 0.0)));
    for _ in 0..120 {
        g.update(crate::app::STEP);
    }
    let behind = view::dir_to_yaw(g.hero.facing);
    let off = (g.view.cam_yaw - behind).rem_euclid(std::f32::consts::TAU);
    assert!(off < 0.05 || off > std::f32::consts::TAU - 0.05, "not recentered: {off}");
}

#[test]
fn the_menus_can_quit() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit]);
    // Right three times, then Enter, on the title picks Quit.
    for _ in 0..3 {
        g.input.key_down(winit::keyboard::KeyCode::ArrowRight, false);
        g.update(crate::app::STEP);
        g.input.key_up(winit::keyboard::KeyCode::ArrowRight);
        g.update(crate::app::STEP);
    }
    g.input.key_down(winit::keyboard::KeyCode::Enter, false);
    g.update(crate::app::STEP);
    assert!(g.quit_requested);
    assert_eq!(g.screen, Screen::Title);

    // Pause offers Resume / Call the Old Man / Controller / Map / Quit to Title / Quit Game; clicking a button picks it.
    let mut g = playing();
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Paused);
    assert_eq!(
        g.menu_items(),
        vec![MenuItem::Resume, MenuItem::CallOldMan, MenuItem::Controls, MenuItem::Map, MenuItem::QuitToTitle, MenuItem::Quit]
    );
    let r = hud::menu_rects(Screen::Paused, &g.menu_items(), g.viewport.0, g.viewport.1)[4];
    g.input.cursor_moved_to(r[0] + r[2] / 2.0, r[1] + r[3] / 2.0);
    g.input.mouse_button(winit::event::MouseButton::Left, true);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Title);
}

#[test]
fn he_stands_on_his_raised_start_through_the_tour_and_the_countdown() {
    let mut g = Game::new();
    start_from_title(&mut g);
    assert_eq!(g.screen, Screen::Ready);
    let spawn = g.level.hero_spawn;
    let floor = g.level.grid.elevation(spawn.x, spawn.y) as f64;
    assert!(floor > 0.0, "the first start is on the fountain's terrace");
    assert_eq!(g.hero.altitude, floor, "not sunk into the stone before he moves");
}

#[test]
fn the_map_jumps_to_any_dungeon_or_cyclops() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    let stops = map_stops();
    assert_eq!(stops.len(), 3 + 2 * 7 + 1, "the Garden and its Cyclops, the Portals of Hell, each realm and its prince, and the gym");
    assert!(stops[0].label.contains("THE GARDEN OF KRAMPUS"));
    assert_eq!(stops[2].stage, save::Stage::Hub, "the Portals of Hell come after the first Cyclops");
    // From the title: MAP, down to the fourth stop (the realm of Pride), go.
    g.menu_index = g.menu_items().iter().position(|&m| m == MenuItem::Map).unwrap();
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert!(g.show_map);
    for _ in 0..3 {
        g.input.nav = Some(Dir::Down);
        g.update(crate::app::STEP);
    }
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert!(!g.show_map);
    assert_eq!((g.screen, g.dungeon, g.boss.is_some()), (Screen::Ready, 1, false));
    assert_eq!(g.realm, Some(Sin::Pride));
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Realm(Sin::Pride)), "Continue comes back here");
    // From the pause menu, to a Cyclops; Esc closes the map without going anywhere.
    g.enter(Screen::Paused);
    g.open_map();
    assert_eq!(stops[g.map_index].stage, save::Stage::Realm(Sin::Pride), "where he is, highlighted");
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert_eq!((g.show_map, g.screen), (false, Screen::Paused));
    g.map_jump(save::Stage::Arena(0));
    assert_eq!((g.screen, g.dungeon, g.boss.is_some()), (Screen::Ready, 0, true));
}

#[test]
fn mehmed_is_slow_and_sturdy_and_hits_hard_miyamoto_quick_and_frail_and_strikes_fast() {
    let mut g = Game::new();
    // Bare heroes: no path skills to add health or quicken the blade.
    g.progress = Progress::default();
    let mut seen = Vec::new();
    for hero in [HeroKind::Ragnar, HeroKind::Mehmed, HeroKind::Miyamoto] {
        g.hero_kind = hero;
        g.load_level(0);
        assert!(g.hero.try_attack());
        seen.push((hero, g.hero.max_hp, g.hero.hp, g.hero.m.speed, g.hero.blow(3), g.hero.attack_cooldown));
    }
    let [(_, r_max, r_hp, r_speed, r_blow, _), (_, m_max, m_hp, m_speed, m_blow, _), (_, y_max, _, y_speed, y_blow, y_cool)] = seen[..]
    else {
        unreachable!()
    };
    assert_eq!((r_max, m_max, y_max), (100, 120, 80), "health 1 : 1.2 : 0.8");
    assert_eq!((r_hp, m_hp), (100, 120), "and each begins with it full");
    assert!((m_speed / r_speed - 0.9).abs() < 1e-9 && (y_speed / r_speed - 1.1).abs() < 1e-9, "speed 1 : 0.9 : 1.1");
    assert!(m_blow > r_blow && y_blow == r_blow, "Mehmed hits harder: {m_blow} against {r_blow}");
    // Miyamoto's blade waits a fifth less than it would for anyone else.
    let own = progression::blade(HeroKind::Miyamoto, g.hero.blade).cooldown;
    assert!((y_cool - own * MIYAMOTO_TEMPO).abs() < 1e-9, "{y_cool} against {own}");
}

#[test]
fn the_portals_of_hell_open_their_doors_and_break_their_seal() {
    use logic::hub;
    use logic::sins::{Sin, ALL_SINS};
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.map_jump(save::Stage::Hub);
    assert_eq!(g.screen, Screen::Ready);
    assert!(g.hub.is_some() && g.enemies.is_empty() && g.boss.is_none(), "a calm hub: no foes");
    assert_eq!(g.hero.occupied(), hub::arrival(), "he comes in from outside");
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Hub));
    g.enter(Screen::Playing);
    for _ in 0..(10.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.enemies.len(), 0, "and none come later");
    // Before Greed's gate its doors swing open; walking away, they close.
    let greed = hub::gates().into_iter().find(|gt| gt.sin == Sin::Greed).unwrap();
    let i = Sin::Greed.index();
    g.hero.m.teleport(greed.front());
    g.hero.face(greed.facing); // looking away from the doors, or he would go through
    for _ in 0..(1.5 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert!(g.hub.as_ref().unwrap().doors[i] >= 1.0, "the doors open as he comes");
    g.hero.m.teleport(hub::arrival());
    for _ in 0..(1.5 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.hub.as_ref().unwrap().doors[i], 0.0, "and close behind him");
    // A realm he has cleared keeps its doors shut.
    g.sins_cleared = Sin::Greed.bit();
    g.hero.m.teleport(greed.front());
    for _ in 0..(1.5 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.hub.as_ref().unwrap().doors[i], 0.0, "Greed is conquered");
    // All seven keys: the seal breaks and the causeways rise to the House of the Devil.
    let lane = hub::causeways()[2];
    let p = P::new(lane.x0 + 1, lane.y0 + 1);
    assert!(!g.level.grid.is_walkable(p.x, p.y, false), "sunk in the lava");
    g.sins_cleared = ALL_SINS;
    for _ in 0..((SEAL_SECONDS + 0.5) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.hub.as_ref().unwrap().seal, 1.0);
    assert!(hub::causeways().iter().all(|c| c.tiles().all(|t| g.level.grid.is_walkable(t.x, t.y, false))), "the causeways are up");
    let seen = pathing::reachable(&g.level.grid, hub::arrival(), true, false);
    let door = hub::house_door();
    assert!(seen.contains(&P::new(door.x, door.y + 1)), "the way to the House is open");
    // Coming back later, the causeways already stand.
    g.checkpoint(save::Stage::Hub);
    g.load_hub();
    assert_eq!(g.hub.as_ref().unwrap().seal, 1.0);
}

#[test]
fn the_realms_foes_are_tougher_than_the_gardens() {
    use WOLF_FURY_SPEED;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.map_jump(save::Stage::Gym);
    let kinds: Vec<EnemyKind> = g.enemies.iter().map(|e| e.kind()).collect();
    assert_eq!(
        kinds,
        [EnemyKind::STAGE_ONE.as_slice(), EnemyKind::STAGE_TWO.as_slice()].concat(),
        "one of every foe, stage by stage, in the gym"
    );
    assert_eq!(g.hero.occupied(), gym::arrival());
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Gym));
    g.enter(Screen::Playing);
    // A slain foe rises again at its post.
    let post = g.enemies[1].post.expect("a post");
    g.slay_enemy(1);
    for _ in 0..((REASSEMBLE_SECONDS - 0.2) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    assert_eq!((g.enemies[1].mode(), g.enemies[1].occupied()), (EnemyMode::Scattered, post), "in pieces at its post");
    for _ in 0..(0.5 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    assert!(g.enemies[1].is_dangerous(), "back on its feet");
    // The foes keep to their platforms while he crosses the bridge, and nothing there can hurt him.
    let second = gym::platform(1);
    g.hero.m.teleport(P::new(second.x0 + 1, (second.y0 + second.y1) / 2));
    let hp = g.hero.hp;
    for _ in 0..(8.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        for e in &g.enemies {
            if let Some(keep) = e.b.keep {
                assert!(keep.contains(e.occupied()), "{} left its platform", e.b.name);
                assert!(e.m.speed <= e.b.base_speed * e.b.speed_factor * 0.75, "{} wanders at an easy pace", e.b.name);
            }
        }
    }
    assert_eq!(g.hero.hp, hp, "unhurt in the gym");
    assert!(g.enemies.iter().filter(|e| e.post.is_some()).all(|e| e.b.keep.is_some()));
    // Off the road, the stand-in realms keep the first stage's cast for now (Greed's vaults have both).
    g.map_jump(save::Stage::Realm(Sin::Sloth));
    assert!(g.enemies.iter().all(|e| EnemyKind::STAGE_ONE.contains(&e.kind())));
    g.map_jump(save::Stage::Gym);
    g.enter(Screen::Playing);
    let o = g.hero.occupied();
    g.enemies.clear();
    g.view.clear_enemies();
    // A blemmyes takes three blows, staggering after each, and bites twice as hard.
    let brute = foe_at(&mut g, EnemyKind::Blemmyes, P::new(o.x, o.y - 1));
    g.hero.facing = Dir::Up;
    for left in [2u8, 1] {
        g.hitstop = 0.0;
        g.perform_attack();
        assert_eq!(g.enemies[brute].hits_left(), left);
        assert!(g.enemies[brute].is_stunned(), "it staggers");
        assert!(!fell(&g, brute));
        for _ in 0..(WOUND_STAGGER / crate::app::STEP + 2.0) as usize {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0;
        }
        g.enemies[brute].b.mode = EnemyMode::Chase;
        g.enemies[brute].m.teleport(P::new(o.x, o.y - 1));
    }
    g.hitstop = 0.0;
    g.perform_attack();
    assert!(fell(&g, brute), "the third blow fells it");
    g.enemies.clear(); // (a slain foe with no post leaves the gym's list)
    g.view.clear_enemies();
    // The loup-garou howls at the first cut and runs faster, and the helmeted skeleton's cap rings.
    let wolf = foe_at(&mut g, EnemyKind::Werewolf, P::new(o.x, o.y - 1));
    g.hitstop = 0.0;
    g.perform_attack();
    assert!(!fell(&g, wolf) && !g.enemies[wolf].is_stunned(), "wounded, it comes on");
    assert!(g.enemies[wolf].b.fury > 0.0);
    g.hitstop = 0.0;
    g.update(crate::app::STEP);
    let quick = g.enemies[wolf].m.speed;
    g.enemies[wolf].b.fury = 0.0;
    g.update(crate::app::STEP);
    assert!((quick / g.enemies[wolf].m.speed - WOLF_FURY_SPEED).abs() < 0.01, "faster in its fury");
    g.enemies.clear();
    g.view.clear_enemies();
    let helm = foe_at(&mut g, EnemyKind::Helmeted, P::new(o.x, o.y - 1));
    g.hitstop = 0.0;
    g.perform_attack();
    assert_eq!((g.enemies[helm].hits_left(), fell(&g, helm)), (1, false), "the helmet turns the first blow");
    g.hitstop = 0.0;
    g.enemies[helm].b.mode = EnemyMode::Chase;
    g.perform_attack();
    assert_eq!(g.enemies[helm].mode(), EnemyMode::Scattered, "and it falls to pieces at the second, to rise again");
    // A witch keeps her distance and conjures a minion once she has the hero in reach.
    g.enemies.clear();
    g.view.clear_enemies();
    let witch = foe_at(&mut g, EnemyKind::Witch, P::new(o.x, o.y - 4));
    g.enemies[witch].b.cast = 0.1;
    g.hero.invuln = 1.0;
    for _ in 0..30 {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    assert!(g.enemies.iter().any(|e| e.kind() == EnemyKind::Minion && e.is_dangerous()), "a minion appears at her feet");
    // Out in a realm (the gym hurts no one), the blemmyes' touch hurts double.
    g.map_jump(save::Stage::Realm(Sin::Greed));
    g.enter(Screen::Playing);
    g.enemies.clear();
    g.view.clear_enemies();
    let hp = g.hero.hp;
    g.hero.invuln = 0.0;
    g.hero.shield = None; // bare, so the whole bite lands
    let at = g.hero.occupied();
    foe_at(&mut g, EnemyKind::Blemmyes, at);
    g.resolve_contact();
    assert_eq!(hp - g.hero.hp, 2 * CONTACT_DAMAGE, "its touch hurts double");
}

/// A foe of `kind` hunting at `at`, in the game and in the view.
fn foe_at(g: &mut Game, kind: EnemyKind, at: P) -> usize {
    let grid = &g.level.grid;
    let spec = enemy_specs(grid, 1).into_iter().chain(enemy_specs(grid, 2)).find(|s| s.kind == kind).unwrap();
    let mut e = Enemy::new(&spec, at, g.level.data.enemy_speed);
    e.b.mode = EnemyMode::Chase;
    g.view.add_enemy(&e);
    g.enemies.push(e);
    g.enemies.len() - 1
}

fn fell(g: &Game, i: usize) -> bool {
    matches!(g.enemies[i].mode(), EnemyMode::Eaten | EnemyMode::Scattered)
}

/// Into the Cyclops' open meadow, with no foes but those placed; he stands at the south edge facing north.
fn meadow() -> Game {
    let mut g = playing();
    g.load_boss(0);
    g.enemies.clear();
    g.view.clear_enemies();
    g.enter(Screen::Playing);
    g
}

#[test]
fn an_archangels_shrine_switches_when_struck_and_blesses_whoever_steps_on_it() {
    use logic::angels::{Archangel, Blessing};
    let mut g = playing();
    let i = g.diamonds.iter().position(|d| d.kind == DiamondKind::Angel).expect("an archangel's shrine");
    assert_eq!(g.diamonds[i].current(), Offer::Angel(Archangel::Michael));
    // It holds still until struck.
    for _ in 0..600 {
        g.diamonds[i].update(crate::app::STEP);
    }
    assert_eq!(g.diamonds[i].current(), Offer::Angel(Archangel::Michael));
    g.diamonds[i].cycle();
    assert_eq!(g.diamonds[i].current(), Offer::Angel(Archangel::Gabriel));
    g.hero.m.teleport(g.diamonds[i].tile);
    g.visit_shrines();
    assert_eq!(g.blessing, Some(Blessing { angel: Archangel::Gabriel, charges: 3 }));
    // Each archangel gives once a level: Gabriel again, at this shrine or another, is refused.
    g.blessing = None;
    let other = g.diamonds.iter().position(|d| d.kind == DiamondKind::Angel && d.tile != g.diamonds[i].tile).unwrap();
    while g.diamonds[other].current() != Offer::Angel(Archangel::Gabriel) {
        g.diamonds[other].cycle();
    }
    g.hero.m.teleport(g.diamonds[other].tile);
    g.visit_shrines();
    assert_eq!(g.blessing, None, "Gabriel has given this level");
    // The other three, then every shrine stands grey.
    for angel in [Archangel::Michael, Archangel::Raphael, Archangel::Uriel] {
        while g.diamonds[other].current() != Offer::Angel(angel) {
            g.diamonds[other].cycle();
        }
        g.visit_shrines();
        assert_eq!(g.blessing.map(|b| b.angel), Some(angel));
    }
    assert!(g.diamonds.iter().filter(|d| d.kind == DiamondKind::Angel).all(|d| !d.active), "all four have given: grey");
    g.blessing = Some(Blessing::of(Archangel::Gabriel));
    // The archangels stand in every dungeon.
    g.load_level(1);
    assert_eq!(g.diamonds.iter().filter(|d| d.kind == DiamondKind::Angel && d.active).count(), 4, "fresh in the next level");
    assert_eq!(g.angels_used, [false; 4]);
    assert_eq!(g.blessing.map(|b| b.charges), Some(3), "the blessing goes with him");
}

#[test]
fn gabriels_thunder_strikes_every_foe_within_ten_tiles() {
    use logic::angels::{Archangel, Blessing};
    let mut g = meadow();
    let o = g.hero.occupied();
    let near = foe_at(&mut g, EnemyKind::Bat, P::new(o.x + 3, o.y - 4));
    let far = foe_at(&mut g, EnemyKind::Bat, P::new(o.x - 7, o.y - 11));
    g.blessing = Some(Blessing::of(Archangel::Gabriel));
    g.invoke_blessing();
    assert!(fell(&g, near) && !fell(&g, far));
    assert_eq!(g.blessing.map(|b| b.charges), Some(2));
}

#[test]
fn raphaels_beam_fells_all_in_its_path_and_nothing_beside_it() {
    use logic::angels::{Archangel, Blessing};
    let mut g = meadow();
    let o = g.hero.occupied();
    g.hero.face(Dir::Up);
    let ahead = [foe_at(&mut g, EnemyKind::Slime, P::new(o.x, o.y - 2)), foe_at(&mut g, EnemyKind::Bat, P::new(o.x, o.y - 4))];
    let beside = foe_at(&mut g, EnemyKind::Bat, P::new(o.x + 3, o.y - 3));
    g.blessing = Some(Blessing::of(Archangel::Raphael));
    g.invoke_blessing();
    assert!(ahead.iter().all(|&i| fell(&g, i)) && !fell(&g, beside));
    g.invoke_blessing();
    assert_eq!(g.blessing, None, "two uses, then it is spent");
}

#[test]
fn uriels_wave_rolls_out_over_everything_and_uriel_then_rests() {
    use logic::angels::{Archangel, Blessing, URIEL_REST};
    let mut g = meadow();
    let o = g.hero.occupied();
    let foes = [foe_at(&mut g, EnemyKind::Bat, P::new(o.x + 1, o.y - 1)), foe_at(&mut g, EnemyKind::Bat, P::new(o.x - 8, o.y - 12))];
    g.blessing = Some(Blessing::of(Archangel::Uriel));
    g.invoke_blessing();
    assert!(!fell(&g, foes[1]), "the far one only when the wave reaches it");
    for _ in 0..(1.6 / crate::app::STEP) as usize {
        g.update_angels(crate::app::STEP);
    }
    assert!(foes.iter().all(|&i| fell(&g, i)));
    assert!((g.uriel_rest - URIEL_REST).abs() < 2.0, "ten minutes' rest");
    // Meanwhile his shrines turn the hero away.
    let mut g = playing();
    g.uriel_rest = 100.0;
    let i = g.diamonds.iter().position(|d| d.kind == DiamondKind::Angel).unwrap();
    for _ in 0..3 {
        g.diamonds[i].cycle();
    }
    g.hero.m.teleport(g.diamonds[i].tile);
    g.visit_shrines();
    assert_eq!(g.blessing, None);
    assert!(g.diamonds[i].active);
}

#[test]
fn michaels_fire_turns_a_foe_that_then_hunts_the_others() {
    use logic::angels::{Archangel, Blessing};
    let mut g = meadow();
    let o = g.hero.occupied();
    g.hero.face(Dir::Up);
    let turned = foe_at(&mut g, EnemyKind::Slime, P::new(o.x, o.y - 3));
    g.blessing = Some(Blessing::of(Archangel::Michael));
    g.invoke_blessing();
    for _ in 0..(1.0 / crate::app::STEP) as usize {
        g.update_projectiles(crate::app::STEP);
    }
    assert!(g.enemies[turned].is_ally() && !g.enemies[turned].is_dangerous(), "turned to his side");
    assert_eq!(g.blessing.map(|b| b.charges), Some(3));
    // A foe that comes within its reach dies.
    let at = g.enemies[turned].occupied();
    let foe = foe_at(&mut g, EnemyKind::Bat, at);
    g.update_angels(crate::app::STEP);
    assert!(fell(&g, foe));
    assert!(!fell(&g, turned));
}

#[test]
fn the_first_cyclops_takes_fifty_and_the_archangels_wound_it_exactly() {
    use logic::angels::{Archangel, Blessing};
    let mut g = playing();
    g.input.key_down(winit::keyboard::KeyCode::F1, false);
    g.update(crate::app::STEP);
    for _ in 0..((READY_SECONDS + 2.2) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    let hp0 = g.boss.as_ref().unwrap().hp;
    assert_eq!(g.boss.as_ref().unwrap().max_hp, 50);
    let bp = g.boss.as_ref().unwrap().pos;
    g.hero.m.teleport(P::new(bp.x.round() as i32, bp.y.round() as i32 + 4));
    g.blessing = Some(Blessing::of(Archangel::Gabriel));
    g.invoke_blessing();
    assert_eq!(g.boss.as_ref().unwrap().hp, hp0 - 5);
    // Dazed, it would take double from a blade, but an archangel's blow is what it is.
    g.debug_daze_boss();
    for _ in 0..20 {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    let hp1 = g.boss.as_ref().unwrap().hp;
    g.invoke_blessing();
    assert_eq!(g.boss.as_ref().unwrap().hp, hp1 - 5);
}

#[test]
fn one_swing_fells_every_foe_it_sweeps_through() {
    let mut g = playing();
    // Two foes side by side just ahead of him, as in a corridor two tiles wide, and one behind.
    let o = g.hero.occupied();
    g.hero.face(Dir::Up);
    let spots = [P::new(o.x, o.y - 1), P::new(o.x + 1, o.y - 1), P::new(o.x, o.y + 2)];
    for (e, &at) in g.enemies.iter_mut().zip(&spots) {
        e.m.teleport(at);
        e.b.mode = EnemyMode::Chase;
    }
    g.hitstop = 0.0;
    g.perform_attack();
    let fell = |g: &Game, i: usize| matches!(g.enemies[i].mode(), EnemyMode::Eaten | EnemyMode::Scattered);
    assert!(fell(&g, 0) && fell(&g, 1), "both foes ahead fall to one blow");
    assert!(!fell(&g, 2), "the one behind him is untouched");
}

#[test]
fn those_waiting_in_the_hollow_stand_in_it() {
    let g = playing();
    for p in &g.nest_slots[1..] {
        assert_eq!(g.level.grid.get(p.x, p.y), Tile::Nest, "{p:?} is not in the hollow");
    }
}

#[test]
fn draugr_rise_from_the_churchyard() {
    let g = playing();
    let graves = g.grave_spawns();
    assert!(!graves.is_empty(), "Dungeon I has a churchyard");
    let draugr = g.enemies.iter().find(|e| e.kind() == EnemyKind::Skeleton).expect("a draugr");
    assert!(graves.contains(&draugr.m.tile), "it starts by the graves, not in the hollow: {:?}", draugr.m.tile);
}

#[test]
fn the_last_key_unlocks_the_gate_and_going_through_wins() {
    let mut g = playing();
    assert_eq!(g.keys_total, 8);
    g.keys_got = g.keys_total - 1;
    let o = g.hero.occupied();
    g.level.grid.set(o.x, o.y, Tile::Nodule);
    g.collect();
    assert!(g.gate_open);
    assert_eq!(g.screen, Screen::Playing, "the gate must still be reached");
    // Up to the gate: the doors swing open, and he steps through.
    let gate = g.gate.unwrap();
    g.hero.m.teleport(P::new(gate.x, gate.y + 1));
    for _ in 0..((DOOR_SECONDS + 0.2) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert!(g.door >= 1.0);
    assert!(g.level.grid.is_walkable(gate.x, gate.y, false));
    g.hero.m.teleport(gate);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Cleared);
    assert!(g.events.contains(&GameEvent::Won));
    // After the fanfare, the Cyclops' meadow, and a countdown.
    for _ in 0..((CLEARED_SECONDS + 0.1) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Ready);
    assert!(g.level.data.boss);
    assert!(g.boss.is_some());
    assert!(g.enemies.is_empty());
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Arena(0)), "the arena is a checkpoint");
}

#[test]
fn f1_jumps_to_the_boss_and_felling_it_leads_on() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.input.key_down(winit::keyboard::KeyCode::F1, false);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Ready);
    assert!(g.level.data.boss, "F1 goes straight to the boss arena");
    for _ in 0..((READY_SECONDS + 2.2) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0; // keep him alive while we watch
    }
    assert_eq!(g.screen, Screen::Playing);
    assert!(g.boss.as_ref().unwrap().active(), "the Cyclops has landed");
    // An axe blow in reach lands.
    let bp = g.boss.as_ref().unwrap().pos;
    g.hero.m.teleport(P::new(bp.x.round() as i32, bp.y.round() as i32 + 1));
    g.hero.facing = Dir::Up;
    let hp = g.boss.as_ref().unwrap().hp;
    g.hitstop = 0.0;
    g.perform_attack();
    assert!(g.boss.as_ref().unwrap().hp < hp, "axe missed the Cyclops");
    // Beat it and the victory screen leads to dungeon II.
    while g.boss.as_ref().unwrap().alive() {
        g.debug_wound_boss(10);
        for _ in 0..20 {
            g.update(crate::app::STEP);
            g.hero.invuln = 1.0;
        }
    }
    // Back at the south edge while it falls (so he does not pick the key up at once).
    g.hero.m.teleport(P::new(ARENA_HERO.0, ARENA_HERO.1));
    for _ in 0..(6.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    // It drops a red diamond key; the gate in the north hedge stays shut without it.
    assert_eq!(g.screen, Screen::Playing, "no victory yet: the key and the gate first");
    let key = g.red_key.expect("the Cyclops drops a red diamond key");
    assert!(g.level.grid.is_walkable(key.x, key.y, false), "on grass he can reach");
    let gate = g.gate.expect("a gate in the arena");
    assert_eq!((gate.x, gate.y), ARENA_GATE);
    let front = P::new(gate.x, gate.y + 1);
    assert!(g.level.grid.is_walkable(front.x, front.y, false), "the way to the gate is clear");
    g.hero.m.teleport(front);
    for _ in 0..(2.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert!(!g.gate_open && g.door == 0.0, "locked without the key");
    // He takes the key: the gate unlocks, and its doors open as he comes to it.
    g.hero.m.teleport(key);
    g.update(crate::app::STEP);
    assert!(g.red_key.is_none() && g.gate_open, "the red key opens the gate");
    g.hero.m.teleport(front);
    for _ in 0..(3.0 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert!(g.door >= 1.0, "the doors swing open");
    g.hero.m.teleport(gate);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Won);
    assert!(g.events.contains(&GameEvent::BossSlain));
    assert!(g.progress.silver >= COINS_BOSS, "the boss pays out");
    assert!(g.progress.runes >= RUNES_BOSS, "and leaves rune fragments");
    assert!((0..4).any(|t| g.progress.hero(g.hero_kind).relics[t] != 0), "and a relic");
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Shop(0)), "Continue would bring back the old man");
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Shop, "the old man comes after the boss");
    // Leave him: down into the Portals of Hell.
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Ready);
    assert!(g.hub.is_some() && !g.level.data.boss, "the Garden's Cyclops slain, the Portals of Hell open");
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Hub));
}

#[test]
fn a_portal_leads_through_its_realm_and_its_princes_court_and_back_with_the_sins_key() {
    use logic::hub;
    use Sin;
    const STEP: f64 = crate::app::STEP;
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.map_jump(save::Stage::Hub);
    g.enter(Screen::Playing);
    // Into Sloth's portal: up to the doors, pushing on them as they open.
    let greed = hub::gates().into_iter().find(|gt| gt.sin == Sin::Sloth).unwrap();
    g.hero.m.teleport(greed.front());
    g.hero.face(greed.facing.opposite());
    for _ in 0..(1.5 / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!(g.screen, Screen::Ready, "through the portal");
    assert_eq!(
        (g.realm, g.dungeon, g.level.data.boss),
        (Some(Sin::Sloth), 1, false),
        "the realm of Sloth, as hard as the first dungeon after the Garden"
    );
    assert_eq!(g.level.data.name, Sin::Sloth.realm());
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Realm(Sin::Sloth)));
    assert_eq!(g.keys_total, 8, "eight golden keys open its gate");
    assert!(
        g.potions.is_empty() && g.diamonds.iter().all(|d| d.kind == DiamondKind::Angel),
        "gear only from the old man; the archangels stand here too"
    );
    // Its gate leads to the prince's court.
    g.enter(Screen::Playing);
    g.cheat_to_gate();
    let gate = g.gate.unwrap();
    g.hero.m.teleport(P::new(gate.x, gate.y + 1));
    for _ in 0..(3.0 / STEP) as usize {
        g.update(STEP);
        g.hero.invuln = 1.0;
    }
    g.hero.m.teleport(gate);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Cleared);
    for _ in 0..((CLEARED_SECONDS + 0.1) / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!((g.screen, g.realm, g.boss.is_some()), (Screen::Ready, Some(Sin::Sloth), true), "Belphegor's court");
    assert_eq!(g.boss_name(), "BELPHEGOR");
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Court(Sin::Sloth)));
    // The prince falls and drops the key of Greed.
    for _ in 0..((READY_SECONDS + 2.5) / STEP) as usize {
        g.update(STEP);
        g.hero.invuln = 1.0;
    }
    assert_eq!(g.screen, Screen::Playing);
    g.debug_wound_boss(1000);
    g.hero.m.teleport(P::new(ARENA_HERO.0, ARENA_HERO.1));
    for _ in 0..(6.0 / STEP) as usize {
        g.update(STEP);
        g.hero.invuln = 1.0;
    }
    let key = g.red_key.expect("the prince drops the key of Sloth");
    assert_eq!(g.sins_cleared, Sin::Sloth.bit(), "Sloth is conquered");
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::SinShop(Sin::Sloth)));
    g.hero.m.teleport(key);
    g.update(STEP);
    assert!(g.red_key.is_none() && g.gate_open, "the key of Sloth opens the court's gate");
    let gate = g.gate.unwrap();
    g.hero.m.teleport(P::new(gate.x, gate.y + 1));
    for _ in 0..(3.0 / STEP) as usize {
        g.update(STEP);
    }
    g.hero.m.teleport(gate);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Won);
    // The old man, and then back before the portal, now sealed, with the next realm one deeper.
    g.input.inject(Action::Confirm);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Shop);
    g.input.inject(Action::Pause);
    g.update(STEP);
    g.input.inject(Action::Pause);
    g.update(STEP);
    assert_eq!(g.screen, Screen::Ready);
    assert!(g.hub.is_some(), "back in the Portals of Hell");
    assert_eq!(g.hero.occupied(), greed.front(), "before the portal he went through");
    assert_eq!(g.saved_run.as_ref().map(|r| (r.stage, r.sins)), Some((save::Stage::Hub, Sin::Sloth.bit())));
    assert_eq!(g.realm_tier(), 2, "the next realm is one deeper");
    g.enter(Screen::Playing);
    for _ in 0..(1.5 / STEP) as usize {
        g.update(STEP);
    }
    assert_eq!(g.hub.as_ref().unwrap().doors[Sin::Sloth.index()], 0.0, "and Sloth's doors stay shut");
}

#[test]
fn free_movement_lasts_from_dungeon_to_dungeon() {
    let mut g = Game::new();
    g.hero.free_move = true;
    start_from_title(&mut g);
    assert!(g.hero.free_move, "beginning a game keeps it");
    g.load_boss(0);
    g.load_level(1);
    assert!(g.hero.free_move, "and so does every level after");
}

#[test]
fn continue_picks_up_the_saved_stage_and_start_asks_before_wiping_everything() {
    let mut g = Game::new();
    assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit], "nothing to continue yet");
    start_from_title(&mut g);
    assert_eq!(g.saved_run.as_ref().map(|r| r.stage), Some(save::Stage::Dungeon(0)), "an adventure is saved from its start");

    // Mehmed reaches the Cyclops with silver, a score, two lives and two bottles.
    g.hero_kind = HeroKind::Mehmed;
    g.progress.silver = 500;
    g.score.resume(7000, 2);
    g.belt = vec![PotionKind::Health, PotionKind::Life];
    g.checkpoint(save::Stage::Arena(0));
    g.enter(Screen::Title);
    assert_eq!(g.menu_items(), vec![MenuItem::Continue, MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit]);
    g.hero_kind = HeroKind::Ragnar;
    g.score.reset();
    g.belt.clear();
    g.load_level(0);
    // Continue is highlighted first.
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Ready, "no story and no character screen");
    assert!(g.level.data.boss && g.boss.is_some());
    assert_eq!((g.hero_kind, g.hero.kind), (HeroKind::Mehmed, HeroKind::Mehmed));
    assert_eq!((g.score.score, g.score.lives), (7000, 2));
    assert_eq!(g.belt, vec![PotionKind::Health, PotionKind::Life]);

    // Start asks first, with "keep my progress" highlighted: Enter loses nothing.
    g.enter(Screen::Title);
    g.menu_index = 1;
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert!(g.confirm_reset);
    assert_eq!(g.menu_items(), vec![MenuItem::StartOver, MenuItem::KeepProgress]);
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert!(!g.confirm_reset);
    assert_eq!((g.screen, g.progress.silver), (Screen::Title, 500));
    assert!(g.saved_run.is_some());
    // Esc backs out too.
    g.menu_index = 1;
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert!(!g.confirm_reset && g.saved_run.is_some());
    // Start over: everything goes, and the new adventure begins (with the heroes, the story being off in tests).
    g.menu_index = 1;
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    g.menu_index = 0;
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Select);
    assert_eq!(g.progress, Progress::default());
    assert!(g.saved_run.is_none());
    g.enter(Screen::Title);
    assert_eq!(g.menu_items(), vec![MenuItem::Play, MenuItem::Controls, MenuItem::Map, MenuItem::Quit]);
}

#[test]
fn a_foe_across_the_maze_finds_the_hero_rather_than_circling() {
    let mut g = playing();
    g.enemies.clear();
    g.view.clear_enemies();
    let hero = g.hero.occupied();
    // Somewhere well away by the paths.
    let far = pathing::reachable(&g.level.grid, hero, false, false)
        .into_iter()
        .filter(|p| (p.x - hero.x).abs() + (p.y - hero.y).abs() >= 14 && g.level.grid.get(p.x, p.y) == Tile::Open)
        .min_by_key(|p| ((p.x - hero.x).abs() + (p.y - hero.y).abs(), p.y, p.x))
        .unwrap();
    let i = foe_at(&mut g, EnemyKind::Skeleton, far);
    let mut closest = f64::INFINITY;
    for _ in 0..(15.0 / crate::app::STEP) as usize {
        g.hero.invuln = 1.0;
        g.update(crate::app::STEP);
        closest = closest.min(dist_sq(g.enemies[i].pos(), g.hero.pos()).sqrt());
    }
    assert!(closest < 1.5, "it came no closer than {closest}");
}

#[test]
fn lt_and_rt_page_through_the_old_mans_wares() {
    let mut g = playing();
    g.open_shop();
    let first = g.shop.tab;
    g.input.inject(Action::TabNext);
    g.input.inject(Action::Attack); // RT is Attack as well; it must not buy
    g.update(crate::app::STEP);
    let second = g.shop.tab;
    assert_ne!(first, second);
    g.input.inject(Action::TabPrev);
    g.update(crate::app::STEP);
    assert_eq!(g.shop.tab, first);
}

#[test]
fn the_controllers_menu_button_opens_the_pause_menu_and_it_stays_open() {
    let mut g = playing();
    // Menu is Pause and Confirm at once.
    g.input.inject(Action::Pause);
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Paused);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Paused, "not resumed straight away");
    assert!(g.menu_items().contains(&MenuItem::CallOldMan), "the old man's wares from the menu");
    // Menu again resumes.
    g.input.inject(Action::Pause);
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Playing);
}

#[test]
fn the_controller_page_opens_from_the_title_and_the_pause_menu() {
    let mut g = Game::new();
    g.menu_index = g.menu_items().iter().position(|&i| i == MenuItem::Controls).unwrap();
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert!(g.show_controls && g.screen == Screen::Title);
    // B closes it, back on the title with the button still highlighted.
    g.input.inject(Action::Bomb);
    g.update(crate::app::STEP);
    assert!(!g.show_controls && g.screen == Screen::Title);
    assert_eq!(g.menu_items()[g.menu_index], MenuItem::Controls);

    // From the pause menu, Esc closes the page and leaves the game paused.
    let mut g = playing();
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    g.menu_index = g.menu_items().iter().position(|&i| i == MenuItem::Controls).unwrap();
    g.input.inject(Action::Confirm);
    g.update(crate::app::STEP);
    assert!(g.show_controls && g.screen == Screen::Paused);
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert!(!g.show_controls);
    assert_eq!(g.screen, Screen::Paused, "still paused");
}

#[test]
fn the_old_man_sells_each_tree_in_order_and_equips() {
    let mut g = playing();
    g.open_shop();
    g.progress.silver = 5000;
    let press = |g: &mut Game, action: ShopAction| {
        let rows = g.shop_rows();
        let (r, c) = rows
            .iter()
            .enumerate()
            .find_map(|(r, row)| row.buttons.iter().position(|b| b.action == action).map(|c| (r, c)))
            .expect("no such button");
        g.shop.row = r;
        g.shop.col = c;
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
    };
    // Tier 3 cannot be had before tier 2.
    press(&mut g, ShopAction::Unlock(Tree::Armory, 3));
    assert_eq!(g.progress.hero(g.hero_kind).owned(Tree::Armory), 1);
    press(&mut g, ShopAction::Unlock(Tree::Armory, 2));
    assert_eq!(g.hero.blade, 2, "a new weapon goes straight to hand");
    assert_eq!(g.hero.main, WeaponKind::Ranged(RangedKind::Hurled(HeroKind::Ragnar, 2)), "Skofnung is thrown");
    press(&mut g, ShopAction::Wield(1));
    assert_eq!(g.hero.blade, 1);
    assert_eq!(g.hero.main, WeaponKind::Sword);
    assert_eq!(g.progress.silver, 5000 - progression::price(Tree::Armory, 2).silver);
    g.shop.tab = Tree::Path;
    press(&mut g, ShopAction::Unlock(Tree::Path, 1));
    assert_eq!(g.hero.path, 1, "the skill is his at once");
    g.shop.tab = Tree::Shield;
    press(&mut g, ShopAction::Unlock(Tree::Shield, 2));
    assert_eq!(g.hero.shield.unwrap().tier, 2);
    g.shop.tab = Tree::Elixir;
    let before = g.belt.len();
    press(&mut g, ShopAction::Bottle(PotionKind::Health));
    assert_eq!(g.belt.len(), before + 1);
    // An elixir must be unlocked before its bottles are sold.
    assert!(g.shop_rows().iter().flat_map(|r| &r.buttons).all(|b| b.action != ShopAction::Bottle(PotionKind::Strength)));
    // Too poor: nothing changes hands.
    g.progress.silver = 10;
    press(&mut g, ShopAction::Unlock(Tree::Elixir, 2));
    assert_eq!(g.progress.silver, 10);
    assert_eq!(g.progress.hero(g.hero_kind).owned(Tree::Elixir), 1);
}

#[test]
fn the_old_man_can_be_called_from_the_dungeon_and_it_waits() {
    let mut g = playing();
    let at = g.hero.pos();
    g.input.key_down(winit::keyboard::KeyCode::KeyH, false);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Shop);
    assert!(g.shop.called);
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Ready, "back to the same dungeon after a countdown");
    assert_eq!(g.dungeon, 0);
    assert_eq!(g.hero.pos(), at, "where he was");
}

#[test]
fn each_dungeon_hides_a_shrine_with_a_relic() {
    let mut g = playing();
    let t = g.relic_shrine.expect("a hidden shrine");
    assert_eq!(g.level.grid.get(t.x, t.y), Tile::Open);
    let runes = g.progress.runes;
    g.hero.m.teleport(t);
    g.update(crate::app::STEP);
    assert!(g.relic_shrine.is_none());
    assert_eq!(g.progress.runes, runes + RUNES_SHRINE);
    assert!((0..4).any(|i| g.progress.hero(g.hero_kind).relics[i] != 0));
}

#[test]
fn the_path_revives_and_restores_once_per_dungeon() {
    let mut g = playing();
    g.progress.hero_mut(g.hero_kind).owned[1] = 10;
    g.equip_from_progress();
    g.hero.shield = None;
    g.hero.hp = 20;
    g.update(crate::app::STEP);
    assert_eq!(g.hero.hp, g.hero.max_hp, "the World Tree restores him at a quarter health");
    g.hero.hp = 20;
    g.update(crate::app::STEP);
    assert_eq!(g.hero.hp, 20, "only once");
    g.hero.invuln = 0.0;
    g.hurt_hero(500);
    assert!(g.hero.alive, "the Valkyrie raises him");
    assert_eq!(g.hero.hp, g.hero.max_hp / 2);
    g.hero.invuln = 0.0;
    g.hurt_hero(500);
    assert!(!g.hero.alive, "but only once");
}

#[test]
fn the_blades_carry_their_gifts() {
    let mut g = playing();
    // Tier 4 stuns the foes around the blow.
    g.debug_arm(4);
    let hp = g.hero.pos();
    for e in g.enemies.iter_mut() {
        e.b.mode = EnemyMode::Chase;
        e.m.teleport(P::new(hp.x as i32 - 2, hp.y as i32));
    }
    g.hero.facing = Dir::Right;
    g.hero.attack_cooldown = 0.0;
    assert!(g.hero.try_attack());
    g.perform_attack();
    assert!(g.enemies.iter().all(|e| e.is_stunned()), "the oath binds them");
    // Tier 6 leaves a decoy.
    g.debug_arm(6);
    g.hero.attack_cooldown = 0.0;
    g.perform_attack();
    assert!(g.decoy.is_some());
}

/// A game in play as `hero`, with his trees owned up to `tier`.
fn playing_as(hero: HeroKind, tier: u8) -> Game {
    let mut g = playing();
    g.hero_kind = hero;
    g.debug_arm(tier);
    g.hero.hp = g.hero.max_hp;
    g
}

#[test]
fn a_samurai_stands_his_last_and_a_spirit_fights_beside_him() {
    let mut g = playing_as(HeroKind::Miyamoto, 9);
    g.hero.shield = None;
    g.hero.invuln = 0.0;
    g.hurt_hero(500);
    assert!(g.hero.alive, "Seppuku Resolve holds death off");
    assert!(g.hero.last_stand > 0.0);
    assert_eq!(g.hero.blow(2), 6, "and triples his blows");
    g.hero.invuln = 0.0;
    g.hurt_hero(500);
    assert!(g.hero.alive && g.hero.hp == 1, "he cannot die while it lasts");
    // His ancestor strikes down a foe that comes near.
    let hp = g.hero.pos();
    g.enemies[0].b.mode = EnemyMode::Chase;
    g.enemies[0].m.teleport(P::new(hp.x as i32, hp.y as i32));
    for _ in 0..(2.0 / crate::app::STEP) as usize {
        g.update_spirits(crate::app::STEP);
    }
    assert_eq!(g.spirits.len(), 1);
    assert!(!g.enemies[0].is_dangerous(), "the spirit slew it");
}

#[test]
fn the_janissary_has_a_squad_a_pistol_and_a_bow() {
    let mut g = playing_as(HeroKind::Mehmed, 9);
    g.update_spirits(crate::app::STEP);
    assert_eq!(g.spirits.len(), 3, "a squad of three");
    g.debug_arm(5);
    assert_eq!(g.hero.main, WeaponKind::Ranged(RangedKind::Hurled(HeroKind::Mehmed, 5)), "Koroglu's flintlock is shot");
    // The Simurg bow rains arrows on the nearest foes.
    g.debug_arm(8);
    let hp = g.hero.pos();
    for e in g.enemies.iter_mut().take(3) {
        e.b.mode = EnemyMode::Chase;
        e.m.teleport(P::new(hp.x as i32 + 2, hp.y as i32));
    }
    g.perform_attack();
    assert!(g.enemies.iter().take(3).all(|e| !e.is_dangerous()), "every one of them struck");
}

#[test]
fn a_sun_blade_obliterates_the_foes_around_him() {
    for hero in [HeroKind::Miyamoto, HeroKind::Mehmed] {
        let mut g = playing_as(hero, 10);
        let hp = g.hero.pos();
        for (k, e) in g.enemies.iter_mut().enumerate() {
            e.b.mode = EnemyMode::Chase;
            e.m.teleport(P::new(hp.x as i32 + if k % 2 == 0 { 2 } else { -2 }, hp.y as i32));
        }
        g.perform_attack();
        assert!(g.enemies.iter().all(|e| !e.is_dangerous()), "{hero:?}: all gone in the flash");
    }
}

#[test]
fn zero_opens_the_shop_from_anywhere() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
    let mut g = Game::new();
    g.input.key_down(winit::keyboard::KeyCode::Digit0, false);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Shop);
    assert!(g.progress.silver >= CHEAT_COINS);
    let mut g = playing();
    g.input.key_down(winit::keyboard::KeyCode::Digit0, false);
    g.update(crate::app::STEP);
    assert_eq!(g.screen, Screen::Shop);
}

#[test]
fn a_third_quick_kill_is_brutal_and_doubles_points() {
    let mut g = playing();
    let before = g.score.score;
    g.slay_enemy(0);
    g.slay_enemy(1);
    assert_eq!(g.double_points, 0.0);
    g.slay_enemy(2);
    assert!(g.double_points > 0.0, "no BRUTAL on the third kill");
    let call = g.juice.banner.as_ref().map(|b| b.text.clone()).unwrap();
    assert!(STREAK_CALLS[0].contains(&call.as_str()), "{call} is one of the first tier's calls");
    // The third kill already counts double: 100 + 100 + 200.
    assert_eq!(g.score.score - before, 400);
    let mid = g.score.score;
    g.debug_collect_key();
    assert_eq!(g.score.score - mid, POINTS_NODULE * 2, "nuggets count double too");
}

#[test]
fn streak_calls_vary_and_grow_fiercer_as_the_streak_goes_on() {
    assert_eq!((streak_tier(3), streak_tier(6), streak_tier(9), streak_tier(12), streak_tier(30)), (0, 1, 2, 3, 3));
    let mut g = playing();
    let mut heard = Vec::new();
    for _ in 0..40 {
        g.kill_streak = 3;
        g.brutal();
        heard.push(g.juice.banner.as_ref().unwrap().text.clone());
    }
    assert!(heard.windows(2).all(|w| w[0] != w[1]), "never the same call twice running");
    let mut kinds = heard.clone();
    kinds.sort();
    kinds.dedup();
    assert!(kinds.len() >= 3, "more than one word: {kinds:?}");
    // A long streak calls from the top tier, where the hero's war cry is heard too.
    let mut cried = false;
    for _ in 0..60 {
        g.kill_streak = 12;
        g.brutal();
        let call = g.juice.banner.as_ref().unwrap().text.clone();
        assert!(STREAK_CALLS[3].contains(&call.as_str()) || call == war_cry(g.hero_kind), "{call}");
        cried |= call == war_cry(g.hero_kind);
    }
    assert!(cried, "the hero's own cry comes up");
}

#[test]
fn the_first_game_opens_with_a_tour() {
    std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test-tour"));
    let mut g = Game::new();
    g.tour_pending = true;
    start_from_title(&mut g);
    assert_eq!(g.screen, Screen::Tour);
    let stops = g.tour.as_ref().unwrap().stops.len();
    assert!(stops >= 8, "only {stops} stops");
    assert!(g.tour.as_ref().unwrap().stops.iter().any(|s| matches!(s.art, TourArt::Enemy(_))));
    // The camera flies to the first thing.
    for _ in 0..60 {
        g.update(crate::app::STEP);
    }
    assert!(g.view.tour.is_some());
    // Click through every stop; then the countdown.
    for _ in 0..stops {
        for _ in 0..40 {
            g.update(crate::app::STEP);
        }
        g.input.inject(Action::Confirm);
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Ready);
    assert!(!g.tour_pending);
    assert!(tour::tour_seen(), "the tour is remembered");
}

#[test]
fn shields_wear_down_and_break() {
    let mut g = playing();
    g.hero.shield = Some(ShieldGear::new(1));
    let hp = g.hero.hp;
    g.debug_hurt(); // 25: the first shield takes 40%
    assert!(g.hero.hp < hp && g.hero.hp > hp - 25, "shield took no share: {}", g.hero.hp);
    let left = g.hero.shield.unwrap().durability;
    assert!(left < ShieldGear::new(1).max());
    for _ in 0..10 {
        g.debug_hurt();
        g.hero.hp = 100;
    }
    assert!(g.hero.shield.is_none(), "it should have broken");
}

#[test]
fn potions_ride_on_the_belt_until_drunk() {
    let mut g = playing();
    g.belt = vec![PotionKind::Strength, PotionKind::Health];
    g.hero.hp = 40;
    g.input.inject(Action::Slot(1));
    g.update(crate::app::STEP);
    assert_eq!(g.hero.hp, 40 + HEALTH_POTION);
    assert_eq!(g.belt, vec![PotionKind::Strength]);
    g.input.inject(Action::Drink);
    g.update(crate::app::STEP);
    assert!(g.belt.is_empty());
    assert!(g.hero.fx.strength > 0.0);
    assert_eq!(g.hero.blow(1), 3);
}

#[test]
fn right_click_drops_a_bomb_and_left_click_swings() {
    let mut g = playing();
    g.input.mouse_button(winit::event::MouseButton::Right, true);
    g.update(crate::app::STEP);
    assert_eq!(g.bombs.len(), 1);
    g.input.mouse_button(winit::event::MouseButton::Left, true);
    g.update(crate::app::STEP);
    assert!(g.hero.swinging());
}

// ------------------------------------------------------------------ the loadout

/// Clicks the shop button for `action` (the tab it lives on must be open).
fn press_shop(g: &mut Game, action: ShopAction) {
    let rows = g.shop_rows();
    let (r, c) = rows
        .iter()
        .enumerate()
        .find_map(|(r, row)| row.buttons.iter().position(|b| b.action == action).map(|c| (r, c)))
        .expect("no such button");
    let b = crate::render::hud::shop_rects(&rows, g.viewport.0, g.viewport.1)[r][c];
    g.input.click = Some((b[0] + b[2] / 2.0, b[1] + b[3] / 2.0));
    g.update(crate::app::STEP);
}

#[test]
fn the_loadout_tab_swears_an_archangel_whose_power_comes_back_on_its_timer() {
    use logic::angels::{Archangel, Blessing};
    let mut g = playing();
    g.open_shop();
    press_shop(&mut g, ShopAction::Loadout);
    assert!(g.shop.loadout);
    let rows = g.shop_rows();
    assert_eq!(rows.iter().find(|r| r.kind == RowKind::Patron).unwrap().buttons.len(), 4, "the four archangels");
    assert_eq!(rows.iter().find(|r| r.kind == RowKind::Outfit).unwrap().buttons.len(), 4, "four outfits");
    press_shop(&mut g, ShopAction::Patron(Archangel::Michael));
    assert_eq!(g.progress.hero(g.hero_kind).patron, Some(Archangel::Michael));
    press_shop(&mut g, ShopAction::Leave);
    for _ in 0..((READY_SECONDS + 0.1) / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
    }
    assert_eq!(g.screen, Screen::Playing);
    // Michael's fire, then ten seconds before it comes again.
    g.input.inject(Action::Angel);
    g.update(crate::app::STEP);
    assert!(g.projectiles.iter().any(|p| p.kind == RangedKind::Grace), "blue fire");
    assert!((g.patron_wait - Archangel::Michael.recharge()).abs() < 0.05);
    g.projectiles.clear();
    g.input.inject(Action::Angel);
    g.update(crate::app::STEP);
    assert!(g.projectiles.is_empty(), "not yet");
    for _ in 0..(10.1 / crate::app::STEP) as usize {
        g.update(crate::app::STEP);
        g.hero.invuln = 1.0;
    }
    assert_eq!(g.patron_wait, 0.0, "back after ten seconds");
    // A shrine's blessing is used first, and the patron's power waits its turn.
    g.blessing = Some(Blessing::of(Archangel::Gabriel));
    g.input.inject(Action::Angel);
    g.update(crate::app::STEP);
    assert_eq!(g.blessing.map(|b| b.charges), Some(Archangel::Gabriel.charges() - 1));
    assert_eq!(g.patron_wait, 0.0);
    // Each archangel's own timer.
    for a in logic::angels::ARCHANGELS {
        g.progress.hero_mut(g.hero_kind).patron = Some(a);
        g.blessing = None;
        g.patron_wait = 0.0;
        g.input.inject(Action::Angel);
        g.update(crate::app::STEP);
        assert!((g.patron_wait - a.recharge()).abs() < 0.05, "{a:?}");
    }
}

#[test]
fn blades_dull_as_they_strike_bite_the_boss_less_and_are_sharpened_for_gold() {
    let mut g = meadow();
    let k = (g.hero.blade - 1) as usize;
    assert_eq!(g.progress.hero(g.hero_kind).edge[k], 100);
    for _ in 0..100 {
        g.dull_blade();
    }
    let edge = g.progress.hero(g.hero_kind).edge[k];
    assert!((39..=41).contains(&edge), "worn down by about sixty: {edge}");
    assert!((g.hero.edge - edge as f64 / 100.0).abs() < 1e-9, "the hero holds the same blade");
    // A dull blade bites the boss less (into the fight as F1 goes, the Cyclops risen and awake).
    let blow = |edge: f64| {
        let mut b = playing();
        b.input.key_down(winit::keyboard::KeyCode::F1, false);
        b.update(crate::app::STEP);
        for _ in 0..((READY_SECONDS + 2.2) / crate::app::STEP) as usize {
            b.update(crate::app::STEP);
            b.hero.invuln = 1.0;
        }
        let bp = b.boss.as_ref().unwrap().pos;
        let hp = b.boss.as_ref().unwrap().hp;
        b.hero.edge = edge;
        assert!(b.strike_boss(10, bp), "the blow lands");
        hp - b.boss.as_ref().unwrap().hp
    };
    let (keen, dull) = (blow(1.0), blow(0.0));
    assert!(dull < keen, "keen {keen}, rusted {dull}");
    // The old man puts the edge back, for gold.
    g.equip_from_progress();
    g.open_shop();
    g.progress.silver = 1000;
    press_shop(&mut g, ShopAction::Loadout);
    let price = logic::gear::sharpen_price(g.hero.blade, g.hero.edge);
    assert!(price > 0);
    press_shop(&mut g, ShopAction::Sharpen);
    assert_eq!(g.progress.hero(g.hero_kind).edge[k], 100);
    assert_eq!(g.hero.edge, 1.0);
    assert_eq!(g.progress.silver, 1000 - price);
}

#[test]
fn a_rusted_blade_glances_off_some_foes_and_a_keen_one_never() {
    let mut g = meadow();
    let at = P::new(13, 16);
    let mut felled = [0, 0];
    for (k, edge) in [1.0, 0.05].into_iter().enumerate() {
        for _ in 0..30 {
            g.enemies.clear();
            g.view.clear_enemies();
            g.hero.place(P::new(13, 17));
            g.hero.face(Dir::Up);
            let i = foe_at(&mut g, EnemyKind::Bat, at);
            g.hero.edge = edge;
            g.perform_attack();
            felled[k] += usize::from(fell(&g, i));
        }
    }
    assert_eq!(felled[0], 30, "a keen blade fells every one");
    assert!(felled[1] < 30, "a rusted one glances off some: {} of 30 fell", felled[1]);
}

#[test]
fn a_shield_that_breaks_is_gone_until_the_old_man_reforges_it() {
    let mut g = playing();
    let tier = g.progress.hero(g.hero_kind).carry;
    g.hero.shield = Some(ShieldGear { durability: 1, ..ShieldGear::new(tier) });
    g.hero.invuln = 0.0;
    g.hurt_hero(40);
    assert!(g.hero.shield.is_none());
    assert!(g.progress.hero(g.hero_kind).is_broken(tier), "struck off for good");
    // A new adventure does not bring it back.
    g.new_kit();
    assert!(g.hero.shield.is_none());
    // Nor can it be carried; it must be reforged.
    g.open_shop();
    g.progress.silver = 2000;
    press_shop(&mut g, ShopAction::Tab(Tree::Shield));
    assert!(g.shop_rows().iter().all(|r| r.buttons.iter().all(|b| b.action != ShopAction::Carry(tier))), "no carrying a broken shield");
    press_shop(&mut g, ShopAction::Reforge);
    assert_eq!(g.hero.shield.map(|s| s.tier), Some(tier));
    assert!(!g.progress.hero(g.hero_kind).is_broken(tier));
    assert_eq!(g.progress.silver, 2000 - logic::gear::reforge_price(tier));
}

#[test]
fn lb_raises_the_shield_to_take_the_whole_blow_at_a_slower_walk() {
    let mut g = meadow();
    g.hero.shield = Some(ShieldGear::new(1));
    g.update(crate::app::STEP);
    let pace = g.hero.m.speed;
    g.input.inject(Action::Block);
    g.update(crate::app::STEP);
    assert!(g.hero.blocking);
    assert!(g.hero.m.speed < pace * 0.7, "slower: {} from {pace}", g.hero.m.speed);
    let (hp, dur) = (g.hero.hp, g.hero.shield.unwrap().durability);
    g.hero.invuln = 0.0;
    g.hurt_hero(10);
    assert_eq!(g.hero.hp, hp, "the raised shield takes it all");
    assert!(g.hero.shield.unwrap().durability < dur);
    // No swinging behind it.
    g.hero.attack_cooldown = 0.0;
    g.input.inject(Action::Attack);
    g.update(crate::app::STEP);
    assert!(!g.hero.swinging());
    // Lowered, it takes only its share again.
    g.input.inject_release(Action::Block);
    g.update(crate::app::STEP);
    assert!(!g.hero.blocking);
    let hp = g.hero.hp;
    g.hero.invuln = 0.0;
    g.hurt_hero(10);
    assert!(g.hero.hp < hp);
}

#[test]
fn the_controls_page_switches_between_the_controller_and_the_keyboard() {
    let mut g = Game::new();
    g.open_controls();
    assert!(g.show_controls);
    let start = g.controls_keyboard;
    g.input.nav = Some(Dir::Right);
    g.update(crate::app::STEP);
    assert_eq!(g.controls_keyboard, !start);
    assert!(g.show_controls, "still open");
    g.input.inject(Action::TabNext);
    g.update(crate::app::STEP);
    assert_eq!(g.controls_keyboard, start);
    // A click on the keyboard's tab.
    let tabs = crate::render::hud::controls_tab_rects(g.viewport.0, g.viewport.1);
    g.input.click = Some((tabs[1][0] + 5.0, tabs[1][1] + 5.0));
    g.update(crate::app::STEP);
    assert!(g.controls_keyboard && g.show_controls);
    g.input.inject(Action::Pause);
    g.update(crate::app::STEP);
    assert!(!g.show_controls, "Esc puts it away");
}

#[test]
fn an_outfit_and_the_paths_ribbons_dress_the_hero() {
    let mut g = playing();
    g.open_shop();
    press_shop(&mut g, ShopAction::Loadout);
    press_shop(&mut g, ShopAction::Outfit(2));
    assert_eq!((g.progress.hero(g.hero_kind).outfit, g.hero.outfit), (2, 2));
    // A path learned is a ribbon on his arm: the model grows by them.
    let plain = g.view.hero_voxels();
    g.progress.hero_mut(g.hero_kind).owned[Tree::Path.index()] = 4;
    g.equip_from_progress();
    g.update(crate::app::STEP);
    assert!(g.view.hero_voxels() > plain, "ribbons on his arm");
}
