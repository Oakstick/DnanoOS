//! Everything drawn flat over the 3D view: the HUD, world labels, and the
//! start, pause, win and game-over screens. `tile` is the layout unit; it
//! scales with the shorter window side so text stays legible from a Steam
//! Deck to 4K.
//!
//! Play's HUD is laid out in `layout` (UI.md §5) and drawn in `style`'s glass by `play` (the
//! corners), `dialogue` (the old man's words, a prince's whisper) and `angel` (the archangels).

use std::sync::OnceLock;

use glam::Vec3;

use super::foes::{bog_slime, draugr, raven};
use super::overlay::{rgba, Align, Overlay, Picture, Style};
use super::view::STEP;
use super::voxels::{sprite_from, Sprite};
use crate::game::{Game, MenuItem, Screen, TourArt, DOUBLE_POINTS_SECONDS, TOUR_STOP_SECONDS};
use crate::input::Action;
use crate::logic::items::{weapon, weapon_label, Offer, WeaponKind};
use crate::logic::progression::{self, Tree};
use crate::render::icons::{icon, Icon};

mod angel;
mod controls;
mod court;
mod dialogue;
mod fullmap;
pub mod glyphs;
mod guide;
pub mod layout;
mod ledger;
pub mod legend;
mod map;
mod mastery;
pub mod minimap;
mod pit;
mod play;
mod ring;
pub mod settings;
mod shop;
pub mod specimen;
pub mod style;
mod war;

pub use controls::controls_tab_rects;
pub use fullmap::{fit as fullmap_fit, layout as fullmap_layout, max_zoom as fullmap_max_zoom, Layout as FullMapLayout};
pub use ledger::draw_ledger_board;
pub use map::{map_cols, map_rects};
mod ending;
mod story;
mod voices;
use crate::logic::angels::Archangel;
use crate::logic::enemy::EnemyKind;
use crate::logic::gear::{BELT_SLOTS, COINS_BOSS, RUNES_BOSS};
use crate::logic::heroes::{HeroKind, ROSTER};
use crate::logic::sins::SINS;
use crate::logic::systems::MAX_LIVES;
use crate::logic::types::{PotionKind, Tile};
use crate::logic::{gear, hub};
use crate::render::view::floor_at;
pub use shop::{shop_rects, shop_row_at};

const TEXT: u32 = 0xe6e0d2;
const TEXT_DIM: u32 = 0xaaa293;
const SAND: u32 = 0xd7ccb8;
// The art direction's UI: charcoal panels with a teal cast and copper edges, amber where the eye
// should go (the board's amber, lifted to read as text), pale stone for quiet lines.
const GOLD: u32 = 0xeab55e;
const INK: [f32; 3] = [12.0 / 255.0, 17.0 / 255.0, 17.0 / 255.0];
const EDGE: [f32; 3] = [0.62, 0.6, 0.54];
const COPPER: u32 = 0xb06a44;
const FRENZIED: u32 = 0x3b5bff;
const FRENZIED_WARN: u32 = 0xe8f0ff;

struct Sprites {
    /// Each hero's face in the corner: Ragnar's pixel model, Mehmed and Miyamoto as they are played.
    portraits: [Sprite; 3],
    /// The fountain's statue, for its tour card.
    cast: Vec<(Sprite, &'static str, u32, &'static str)>,
}

/// A sprite trimmed to what is drawn in it.
fn cropped(s: Sprite) -> Sprite {
    let filled = |x: i32, y: i32| s.pixels[y as usize][x as usize].is_some();
    let xs: Vec<i32> = (0..s.width).filter(|&x| (0..s.height).any(|y| filled(x, y))).collect();
    let ys: Vec<i32> = (0..s.height).filter(|&y| (0..s.width).any(|x| filled(x, y))).collect();
    let (Some(&x0), Some(&x1), Some(&y0), Some(&y1)) = (xs.first(), xs.last(), ys.first(), ys.last()) else { return s };
    let pixels = (y0..=y1).map(|y| (x0..=x1).map(|x| s.pixels[y as usize][x as usize]).collect()).collect();
    Sprite { width: x1 - x0 + 1, height: y1 - y0 + 1, pixels }
}

/// The hero's face for the HUD corner and the tour.
fn portrait_of(hero: HeroKind) -> &'static Sprite {
    use HeroKind;
    &sprites().portraits[match hero {
        HeroKind::Ragnar => 0,
        HeroKind::Mehmed => 1,
        HeroKind::Miyamoto => 2,
    }]
}

fn sprites() -> &'static Sprites {
    static S: OnceLock<Sprites> = OnceLock::new();
    S.get_or_init(|| Sprites {
        portraits: [
            cropped(sprite_from(&super::heroes::ragnar_hd(None))),
            cropped(sprite_from(&super::heroes::mehmed_hd(None))),
            cropped(sprite_from(&super::heroes::miyamoto_hd(None))),
        ],
        cast: vec![
            (sprite_from(&draugr()), "Draugr", 0xe9e9f2, "An undead viking. Follows your footsteps, falls apart, rises again."),
            (sprite_from(&raven(false)), "Raven", 0xc9a0ff, "Fast, and flies over pits and ledges. Its dark kin hunts in pairs."),
            (sprite_from(&bog_slime()), "Bog Slime", 0x5cff6a, "Slow, but its trail slows you too."),
            (sprite_from(&super::foes2::witch()), "Witch", 0xb066ff, "Keeps her distance and conjures minions. Two blows."),
            (sprite_from(&super::foes2::minion()), "Witch Minion", 0xd08cff, "Small and quick; they come in numbers."),
            (
                sprite_from(&super::foes2::helmeted_skeleton()),
                "Helmeted Skeleton",
                0xff5a4a,
                "The first blow rings off its helmet. Rises again.",
            ),
            (sprite_from(&super::foes2::loup_garou()), "Loup-Garou", 0xc0885a, "Fast, and faster once wounded."),
            (sprite_from(&super::foes2::blemmyes()), "Blemmyes", 0xe8b494, "Three blows to fell, and his hand hits twice as hard."),
            (sprite_from(&super::foes2::large_bat()), "Large Bat", 0xc04a70, "Flies over everything, swerving as it comes."),
            (sprite_from(&super::foes3::tengu()), "Tengu", 0xe8503e, "Melee: quick to close in, his katana cuts high and wide."),
            (sprite_from(&super::foes3::rasetsu()), "Rasetsu", 0xd0402e, "Melee: two curved blades, slow to start and hard to stagger."),
            (sprite_from(&super::foes3::dire_wolf()), "Dire Wolf", 0xa4acb8, "Body: it circles, then bites from beside you."),
            (sprite_from(&super::foes3::jorogumo()), "Jorogumo", 0xff8a3a, "Distance: she hurls fire and backs away from you."),
            (sprite_from(&super::foes3::cobra()), "Cobra", 0x9ac84a, "Body: it rears back on its coil, then strikes."),
            (
                sprite_from(&super::foes4::oni()),
                "Oni",
                0xd8402c,
                "Melee: nothing stops the iron club but a parry; the ground shakes where it lands.",
            ),
            (
                sprite_from(&super::foes4::kappa()),
                "Kappa",
                0x5ab07a,
                "Body: a sumo shove. Parry it and it bows, spilling the water in its dish.",
            ),
            (sprite_from(&super::foes4::rokurokubi()), "Rokurokubi", 0xe8d8e8, "Body: up close, her head bows on her long neck and bites."),
            (sprite_from(&super::foes4::kitsune()), "Kitsune", 0xff8a3a, "Distance: foxfire in threes, and illusions of itself."),
            (sprite_from(&super::foes5::karakoncolos()), "Karakoncolos", 0x8ec8ff, "Melee: it roars your name, and the roar dooms you."),
            (sprite_from(&super::foes5::tulukus()), "Tülükuş", 0xd09a5a, "Body: up on its wings, then talons; its cry dooms you."),
            (sprite_from(&super::foes5::albasti()), "Albastı", 0xc0283a, "Body: a shadow until she falls on you with all her weight."),
            (sprite_from(&super::foes5::cin()), "Cin", 0xffe6a0, "Distance: smokeless fire out of nowhere; vanishes."),
            (sprite_from(&super::foes5::arcura()), "Arçura", 0x6ad050, "Distance: thorny root darts that burst into roots."),
            (
                sprite_from(&super::foes7::frost_giant()),
                "Frost Giant",
                0xa8dcff,
                "Melee: its blows frostbite through your shield, and the frost bites on.",
            ),
            (sprite_from(&super::foes7::surtr()), "Surtr", 0xff6a1a, "Melee: a flaming sword that leaves the floor burning."),
            (sprite_from(&super::foes7::nidhogg()), "Nidhogg", 0x8ad04a, "Body: it burrows and bursts up beneath you; venom."),
            (sprite_from(&super::foes7::hel()), "Hel", 0x7aa0e8, "Distance: bolts that drain your life; she raises the dead."),
            (
                sprite_from(&super::foes6::tepegoz()),
                "Tepegöz",
                0xe8a070,
                "Melee: no blade cuts him; strike his one eye when it comes within reach.",
            ),
            (
                sprite_from(&super::foes6::gashadokuro()),
                "Gashadokuro",
                0xdcd8b8,
                "Melee: a ringing, then its hands crush you. Parry or leap.",
            ),
            (
                sprite_from(&super::foes6::ejder()),
                "Yedi Başlı Ejder",
                0x7ac04a,
                "Body: seven heads strike in turn. Felled, it begs a second blow: deny it.",
            ),
            (sprite_from(&super::foes6::goryo()), "Goryō", 0x8ab0ff, "Distance: bolts of lightning; untouchable but in its wrath."),
            (sprite_from(&super::foes_feast::gaki()), "Gaki", 0xc8b49a, "Body: hungry ghosts; they swarm whoever smells of food."),
            (
                sprite_from(&super::foes_thorns::ellepige()),
                "Ellepige",
                0xf0e0ea,
                "Melee: lovely from the front, hollow behind. Strike her back.",
            ),
        ],
    })
}

pub struct OverlayLine {
    pub text: String,
    pub size: f32,
    pub color: u32,
}

fn line(text: impl Into<String>, size: f32, color: u32) -> OverlayLine {
    OverlayLine { text: text.into(), size, color }
}

thread_local! {
    /// The settings the HUD reads as it draws (`crate::settings::UiPrefs`), set by the game as
    /// they change. Per thread: the game and its HUD run on one, and each test on its own.
    static PREFS: std::cell::Cell<crate::settings::UiPrefs> = std::cell::Cell::new(crate::settings::UiPrefs::default());
}

/// The UI's scale, the prose's size, the colour-blind palette, the backing and the flashes.
pub fn prefs() -> crate::settings::UiPrefs {
    PREFS.with(|p| p.get())
}

pub fn set_prefs(p: crate::settings::UiPrefs) {
    PREFS.with(|c| c.set(p));
}

/// A flash's strength as the player has it: dimmed to a third with reduce flashing on.
pub fn flash(k: f32) -> f32 {
    if prefs().reduce_flashing {
        k * 0.33
    } else {
        k
    }
}

/// The HUD's unit for a `w` by `h` screen, with the player's UI scale (`style::unit`).
pub fn tile_for(w: f32, h: f32) -> f32 {
    style::unit(w, h)
}

/// Builds the whole overlay for this frame.
/// Makes, as a level loads (behind its loading screen), the pictures the HUD makes the first
/// time it shows them: the heroes' sprites and the foes' cast, the icons, and the portraits on the
/// Ledger's cards of `kinds` (the level's foes). (The old man's and the hero's live portraits are
/// made by the renderer behind the loading screen: `draw_loading`.)
pub fn warm(kinds: &[crate::logic::enemy::EnemyKind]) {
    let _ = sprites();
    let _ = icon(Icon::Lock);
    ledger::warm(kinds);
}

pub fn draw(o: &mut Overlay, g: &Game, fps: u32) {
    let (w, h) = (o.width, o.height);
    let t = tile_for(w, h);
    // Every piece of text gets a drop shadow so it reads over bright floors and lights.
    o.shadow = (t * 0.08).max(1.5);
    if g.screen == Screen::Tour {
        draw_tour(o, g, t);
        return;
    }
    if g.screen == Screen::Shop {
        shop::draw_shop(o, g, t);
        if g.juice.fade > 0.0 {
            o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, g.juice.fade.min(1.0)]);
        }
        return;
    }
    if g.screen == Screen::Story {
        story::draw_story(o, g, t);
        return;
    }
    if g.screen == Screen::Select {
        draw_select(o, g, t);
        if g.juice.fade > 0.0 {
            o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, g.juice.fade.min(1.0)]);
        }
        return;
    }
    // (The camera choice keeps the world clear of the HUD, as the tour does.)
    if !matches!(g.screen, Screen::Title | Screen::View) {
        draw_world_overlays(o, g, t);
        voices::draw_signs(o, g, t);
        ledger::draw_rival_card(o, g, t);
        war::draw_war(o, g, t);
        ring::draw_ring(o, g, t);
        mastery::draw_mastery_toast(o, g, t);
        angel::draw_arrows(o, g);
        draw_hud(o, g, if g.show_fps { Some(fps) } else { None });
        minimap::draw(o, g, t);
        draw_feedback(o, g, t);
        dialogue::draw(o, g);
    }
    match g.screen {
        Screen::Title => {
            draw_title(o, g, t);
            if g.show_controls {
                controls::draw_controls(o, g, t);
            }
            if g.show_map {
                map::draw_map(o, g, t);
            }
            if g.warband.board_open {
                ledger::draw_ledger_screen(o, g, t);
            }
            if g.settings_page.open {
                settings::draw_settings(o, g, t);
            }
        }
        Screen::Ready => draw_countdown(o, g, t),
        Screen::Paused if g.guide_open => guide::draw_guide(o, g, t),
        Screen::Paused => {
            o.rect(0.0, 0.0, o.width, o.height, [INK[0], INK[1], INK[2], 0.65]);
            let top = menu_rects(Screen::Paused, &g.menu_items(), o.width, o.height)[0][1];
            o.text("PAUSED", o.width / 2.0, top - t * 2.6, t * 1.45, rgba(TEXT, 1.0), Align::Center, true);
            glyphs::prompt_line(o, &g.input, "{Pause} resume", o.width / 2.0, top - t * 1.1, t * 0.6, rgba(SAND, 1.0), Align::Center);
            draw_menu(o, g, t);
            // Where the story stands: the road down after Krampus, below the last button.
            let rects = menu_rects(Screen::Paused, &g.menu_items(), o.width, o.height);
            let y = rects.last().map(|r| r[1] + r[3]).unwrap_or(o.height / 2.0) + t * 1.4;
            o.text("THE ROAD DOWN", o.width / 2.0, y, t * 0.55, rgba(GOLD, 0.9), Align::Center, true);
            o.text(&where_line(g), o.width / 2.0, y + t * 0.8, t * 0.5, rgba(SAND, 0.9), Align::Center, false);
            let below = o.wrap_text(
                crate::logic::story::objective(),
                o.width / 2.0,
                y + t * 1.7,
                (t * 26.0).min(o.width - t * 2.0),
                t * 0.75,
                t * 0.5,
                rgba(TEXT_DIM, 1.0),
            );
            play::draw_pause_summary(o, g, t, below + t * 0.5);
            fullmap::draw(o, g, t);
            if g.show_controls {
                controls::draw_controls(o, g, t);
            }
            if g.show_map {
                map::draw_map(o, g, t);
            }
            if g.warband.board_open {
                ledger::draw_ledger_screen(o, g, t);
            }
            if g.settings_page.open {
                settings::draw_settings(o, g, t);
            }
        }
        Screen::Won => draw_screen(
            o,
            &g.input,
            t,
            &[
                line(format!("{} IS SLAIN", g.boss_name()), 2.0, GOLD),
                line(
                    match g.realm {
                        Some(sin) => format!("The key of {} is {}'s. Back to the Portals of Hell.", sin.name(), g.hero_kind.name()),
                        None if g.in_lair() => format!("The cage is open. {} and his friend are together again.", g.hero_kind.name()),
                        None if g.dungeon == 0 => format!("The Portals of Hell open below. {} goes down.", g.hero_kind.name()),
                        None => format!("The way down lies open. {} goes deeper.", g.hero_kind.name()),
                    },
                    0.9,
                    SAND,
                ),
                line(format!("Gold {}   Lives left {}", g.score.score, g.score.lives), 0.9, TEXT),
                line(format!("Best {}", g.score.high), 0.8, SAND),
                line(
                    format!(
                        "+{} {}   +{} {}   and a relic of the old heroes",
                        COINS_BOSS + 100 * g.dungeon as u32,
                        progression::coin(g.hero_kind).to_lowercase(),
                        RUNES_BOSS + g.dungeon as u32,
                        progression::gems(g.hero_kind, 2).to_lowercase()
                    ),
                    0.9,
                    GOLD,
                ),
                line("{Confirm} or a click: the old gatekeeper's wares", 0.8, TEXT_DIM),
            ],
            0.7,
        ),
        Screen::GameOver => draw_screen(
            o,
            &g.input,
            t,
            &[
                line(format!("{} HAS FALLEN", g.hero_kind.name().to_uppercase()), 1.6, 0xd8262e),
                line(format!("Gold {}", g.score.score), 0.9, TEXT),
                line(format!("Best {}", g.score.high), 0.8, SAND),
                line("{Confirm} or a click to try again", 0.8, TEXT_DIM),
            ],
            0.65,
        ),
        Screen::Toll => draw_toll(o, g, t),
        Screen::View => draw_view_choice(o, g, t),
        Screen::Cleared => {
            // The fanfare's last moments fade to black on the way to the boss.
            let k = (1.0 - g.screen_timer as f32 / 0.9).clamp(0.0, 1.0);
            o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, k]);
        }
        _ => {}
    }
    if g.juice.fade > 0.0 {
        o.rect(0.0, 0.0, o.width, o.height, [0.0, 0.0, 0.0, g.juice.fade.min(1.0)]);
    }
    draw_loading(o, g, t);
}

/// Seconds a level may take to make ready before the loading screen's words come up: a shorter
/// wait (the Garden's cold start, about 0.2 s; a portal's, under 0.15 s) is a plain cut through
/// black, not a card that flashes up and is gone.
const LOADING_WORDS_AFTER: f32 = 0.6;
/// Seconds the words take to come up once they do.
const LOADING_WORDS_FADE: f32 = 0.3;

/// How far the loading screen's words have come up, `age` seconds into the wait.
fn loading_words(age: f32) -> f32 {
    ((age - LOADING_WORDS_AFTER) / LOADING_WORDS_FADE).clamp(0.0, 1.0)
}

/// The loading screen (RP03): over the black, the level's name and a pulse, once the wait is long
/// enough to need saying. The HUD under it is drawn all the same, so its sprites are ready on reveal.
fn draw_loading(o: &mut Overlay, g: &Game, t: f32) {
    let Some(l) = &g.loading else { return };
    // The portraits the level will show, made behind the loading screen (their first making costs
    // the most: the meshes go up and the driver readies the pipeline), drawn unseen.
    use crate::render::portrait::{PortraitId, Pose};
    let hud = layout::HudLayout::of(o.width, o.height);
    let (hero_px, face) = (hud.t * 3.2, hud.t * dialogue::FACE);
    o.portrait(PortraitId::Hero(g.hero_kind), Pose::live(g.time as f32, 1.0), 0.0, 0.0, hero_px, hero_px, false, [0.0; 4]);
    o.portrait(PortraitId::OldMan, Pose::live(g.time as f32, 1.0), 0.0, 0.0, face, face, false, [0.0; 4]);
    // The new level's minimap image, built and sent up behind the black too.
    minimap::prepare(o, g);
    let show = loading_words(l.age as f32);
    if show <= 0.0 {
        return;
    }
    // The card: the controller legend, the level's name and a tip from the old man (`legend`).
    legend::draw_loading_card(o, g, l, t, show);
}

fn bar(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, frac: f32, color: u32) {
    o.rect(x - 1.0, y - 1.0, w + 2.0, h + 2.0, [0.0, 0.0, 0.0, 0.6]);
    o.rect(x, y, w, h, [1.0, 1.0, 1.0, 0.12]);
    o.rect(x, y, w * frac.clamp(0.0, 1.0), h, rgba(color, 1.0));
}

/// The health bar's colour, high to low (the colour-blind palette's when it is on).
fn hp_color(frac: f32) -> u32 {
    super::palette::health(prefs().colour_blind, frac)
}

fn draw_sprite(o: &mut Overlay, s: &Sprite, x: f32, y: f32, scale: f32, tint: Option<u32>, alpha: f32) {
    for (py, row) in s.pixels.iter().enumerate() {
        for (px, c) in row.iter().enumerate() {
            if let Some(c) = c {
                let color = rgba(tint.unwrap_or(*c), alpha);
                o.rect(x + px as f32 * scale, y + py as f32 * scale, scale + 0.5, scale + 0.5, color);
            }
        }
    }
}

// ---------------------------------------------------------------------- HUD

/// A panel of the HUD's glass (`style::panel_glass`): its shadow, the glass, the copper rule, the
/// notched corners.
fn panel(o: &mut Overlay, x: f32, y: f32, w: f32, h: f32, t: f32) {
    style::panel_glass(o, [x, y, w, h], t, 1.0);
}

/// Where flying keys land: the row of keys in the purse under the minimap.
pub fn key_target(w: f32, h: f32) -> (f32, f32) {
    play::key_target(w, h)
}

/// Play's HUD (UI.md §5): the corners (`play`), the archangel (`angel`); the prince's bar and the
/// Pit's clock in the top centre.
fn draw_hud(o: &mut Overlay, g: &Game, fps: Option<u32>) {
    play::draw(o, g, fps);
    angel::draw(o, g);
    court::draw_prince_bar(o, g);
    pit::draw_clock(o, g);
}

fn mix_rgb(a: u32, b: u32, k: f32) -> u32 {
    let ch = |s: u32| {
        let x = ((a >> s) & 255) as f32;
        let y = ((b >> s) & 255) as f32;
        ((x + (y - x) * k.clamp(0.0, 1.0)) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// Screen-wide feedback drawn over the HUD: flying nuggets, banners, GO!, the hurt vignette.
fn draw_feedback(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let j = &g.juice;

    // BRUTAL: a hot flash that burns off from the middle out.
    if j.brutal > 0.0 {
        o.rect(0.0, 0.0, w, h, [1.0, 0.35, 0.15, flash(0.2 * j.brutal * j.brutal)]);
        style::vignette(o, t * 3.6, rgba(0xff5a1a, flash(0.6 * j.brutal)));
    }
    // Red at the screen's edges when hurt.
    if j.hurt > 0.0 {
        style::vignette(o, t * 3.0, rgba(0xd8262e, flash(0.55 * j.hurt)));
    }

    // Nuggets flying to the counter along an arc, with a short trail.
    let (tx, ty) = key_target(w, h);
    for c in &j.coins {
        for k in 0..4 {
            let u = (c.t - k as f32 * 0.05).clamp(0.0, 1.0);
            let e = u * u * (3.0 - 2.0 * u);
            let ctrl = (c.from.0 + (tx - c.from.0) * 0.2, c.from.1.min(ty) - t * 3.0);
            let bez = |a: f32, b: f32, m: f32| (1.0 - e) * (1.0 - e) * a + 2.0 * (1.0 - e) * e * m + e * e * b;
            let x = bez(c.from.0, tx, ctrl.0);
            let y = bez(c.from.1, ty, ctrl.1);
            let r = t * (0.32 - k as f32 * 0.06) * (1.0 - 0.35 * e);
            let a = 1.0 - k as f32 * 0.25;
            o.circle(x, y, r, rgba(GOLD, a));
            if k == 0 {
                o.circle(x - r * 0.25, y - r * 0.25, r * 0.45, rgba(0xfff1a8, a));
            }
        }
    }

    // A milestone's banner: a band across the screen, the call in the display serif popping in.
    if let Some(b) = &j.banner {
        let age = b.max - b.life;
        let big = b.text == "BRUTAL!";
        let pop = if big { 1.0 + 1.6 * (1.0 - (age / 0.25).min(1.0)).powi(3) } else { 1.0 + 0.55 * (1.0 - (age / 0.2).min(1.0)).powi(2) };
        let pop = if big { pop * 1.7 } else { pop };
        let alpha = (b.life / 0.4).min(1.0) * (age / 0.12).min(1.0);
        let y = h * 0.27;
        style::band(o, y, t * 2.9, t, b.color, alpha);
        let title = style::name(t, 1.4 * pop, rgba(b.color, alpha)).centered().glow(t * 0.3, rgba(b.color, 0.35 * alpha));
        o.text_styled(&b.text, w / 2.0, y - t * 0.3, &title);
        o.text_styled(&b.sub, w / 2.0, y + t * 0.85, &style::label(t, style::size::LABEL, rgba(SAND, alpha)).centered());
    }
    angel::draw_banner(o, g, t);

    // GO!
    if j.go > 0.0 {
        let k = j.go / 0.8;
        let units = 4.5 * (1.0 + (1.0 - k) * 0.7);
        let a = k.min(1.0);
        let st = style::name(t, units, rgba(0x9dffb4, a)).centered().glow(t * 0.5, rgba(0x2a8a4a, 0.6 * a));
        o.text_styled("GO!", w / 2.0, h * 0.42, &st);
    }
}

/// Before play: READY lands big and settles, breathes while the level's name is read, and gathers
/// itself in as GO! comes (`juice.go`).
fn draw_countdown(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.3]);
    let left = g.screen_timer.max(0.0) as f32;
    let age = crate::game::READY_SECONDS as f32 - left;
    let land = 1.0 + 0.7 * (1.0 - (age / 0.3).min(1.0)).powi(4);
    let breathe = 1.0 + 0.03 * (age * 5.0).sin();
    let gather = (left / 0.2).min(1.0);
    let alpha = (age / 0.12).min(1.0) * gather;
    let size = t * 4.0 * land * breathe * (0.85 + 0.15 * gather);
    let y = h * 0.42;
    o.circle(w / 2.0, y, t * 4.2 * land, [1.0, 1.0, 1.0, 0.05 * alpha]);
    o.text("READY", w / 2.0, y, size, rgba(GOLD, alpha), Align::Center, true);
    if g.arena() {
        o.text(&format!("{} AWAITS", g.boss_name()), w / 2.0, y + t * 3.2, t * 0.9, rgba(0xff8a2a, 1.0), Align::Center, true);
    }
    // The level's name, large and gold across the top, as the count begins.
    let name = g.level.data.name.to_uppercase();
    let fit = w * 0.86 / (name.chars().count().max(1) as f32 * o.measure("M", 1.0));
    let ny = h * 0.2;
    o.text(&where_line(g), w / 2.0, ny - t * 1.9, t * 0.55, rgba(SAND, 1.0), Align::Center, true);
    draw_logo_word(o, &name, w / 2.0, ny, (t * 2.0).min(fit), g.time as f32);
    draw_ornament(o, w / 2.0, ny + t * 1.55, t * 12.0, t, g.time as f32);
}

// ------------------------------------------------------------------- tour

/**
 * The first-run tour's overlay: letterbox bars slide in, a pulsing frame
 * locks onto what the camera is showing, and a see-through card names it
 * and explains it, with a picture, progress dots and how to move on.
 */
fn draw_tour(o: &mut Overlay, g: &Game, t: f32) {
    let Some(tour) = &g.tour else { return };
    let (w, h) = (o.width, o.height);
    let stop = tour.stop();
    let time = g.time as f32;
    let k = (tour.age as f32 / 0.35).min(1.0);
    let bar_h = h * 0.1 * (1.0 - (1.0 - k).powi(3));
    o.rect(0.0, 0.0, w, bar_h, [0.0, 0.0, 0.0, 0.92]);
    o.rect(0.0, h - bar_h, w, bar_h, [0.0, 0.0, 0.0, 0.92]);

    // Lock-on frame around the target, tightening as the camera arrives.
    let st = tour.t as f32;
    let settle = (st / 0.7).min(1.0);
    if let Some((x, y)) = g.view.project(stop.target, w, h) {
        let size = t * (2.4 + 3.0 * (1.0 - settle)) * (1.0 + (time * 4.0).sin() * 0.04);
        let arm = size * 0.35;
        let lw = (t * 0.1).max(2.0);
        let c = rgba(stop.color, 0.9 * settle.max(0.3));
        for (sx, sy) in [(-1.0f32, -1.0f32), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
            let (cx, cy) = (x + sx * size, y + sy * size);
            o.line([cx, cy], [cx - sx * arm, cy], lw, c);
            o.line([cx, cy], [cx, cy - sy * arm], lw, c);
        }
    }

    // The card: a see-through panel above the lower bar.
    let fade = (st / 0.25).min(1.0);
    let cw = (w * 0.42).max(t * 26.0).min(w - t * 1.6);
    let chh = t * 6.4;
    let cx = t * 0.8;
    let cy = h - bar_h - chh - t * 0.6;
    o.rect(cx, cy, cw, chh, [0.02, 0.03, 0.05, 0.5 * fade]);
    o.rect(cx, cy, t * 0.14, chh, rgba(stop.color, 0.9 * fade));
    o.rect_outline(cx, cy, cw, chh, 1.0, [1.0, 1.0, 1.0, 0.18 * fade]);
    let art = t * 4.2;
    let ax = cx + t * 0.6;
    let ay = cy + (chh - art) / 2.0;
    draw_tour_art(o, g.hero_kind, stop.art, ax, ay, art, fade);
    let tx = ax + art + t * 0.7;
    // The title types itself out.
    let shown = ((st * 60.0) as usize).min(stop.title.len());
    o.text(&stop.title[..shown], tx, cy + t * 1.0, t * 1.0, rgba(stop.color, fade), Align::Left, true);
    wrap_left(o, &stop.body, tx, cy + t * 2.1, cw - (tx - cx) - t * 0.6, t * 0.62, t * 0.5, rgba(TEXT, fade));

    // Progress dots and the way on.
    let n = tour.stops.len();
    let dy = cy + chh - t * 0.55;
    for i in 0..n {
        let on = i == tour.index;
        let c = if on {
            rgba(stop.color, fade)
        } else if i < tour.index {
            [1.0, 1.0, 1.0, 0.5 * fade]
        } else {
            [1.0, 1.0, 1.0, 0.18 * fade]
        };
        let r = if on { t * 0.16 } else { t * 0.11 };
        o.circle(tx + i as f32 * t * 0.5, dy, r, c);
    }
    let left = (TOUR_STOP_SECONDS - tour.t).max(0.0) as f32 / TOUR_STOP_SECONDS as f32;
    o.rect(cx, cy + chh - t * 0.08, cw * left, t * 0.08, rgba(stop.color, 0.6 * fade));
    glyphs::prompt_line(
        o,
        &g.input,
        "{Confirm} next     {Pause} skip the tour",
        w - t * 0.8,
        h - bar_h / 2.0,
        t * 0.5,
        rgba(TEXT_DIM, k),
        Align::Right,
    );
    o.text("ADVENTURES OF THE ANCIENT BLADES", t * 0.8, bar_h / 2.0, t * 0.5, rgba(SAND, k * 0.8), Align::Left, false);
    o.text(&format!("{} / {}", tour.index + 1, n), w - t * 0.8, bar_h / 2.0, t * 0.5, rgba(SAND, k * 0.8), Align::Right, false);
}

/// Where the road down stands, for the countdown and the pause screen.
fn where_line(g: &Game) -> String {
    if g.hub.is_some() {
        "DEPTH II: SEVEN DOORS, SEVEN SINS".to_string()
    } else if g.in_vault() {
        "THE VAULTS OF MAMMON: TAKE ALL YOU CAN CARRY, AND SEE WHAT IT COSTS YOU AT THE FAR PORTAL".to_string()
    } else if g.theme().is_gym() {
        "ONE OF EVERY FOE, STAGE BY STAGE: THEY RISE AGAIN WHEN THEY FALL".to_string()
    } else if let Some((level, ..)) = g.pit_status() {
        format!("THE PIT, LEVEL {level} OF 7: RIDE THE PLATFORM DOWN, AND LEAVE WHAT YOU CAN ON THE LEDGE")
    } else if let Some(line) = g.run_where().filter(|_| !g.arena()) {
        line
    } else if let Some(sin) = g.realm {
        if g.arena() {
            format!("THE REALM OF {}: {} GUARDS ITS KEY", sin.title(), sin.prince().to_uppercase())
        } else {
            format!("THE REALM OF {}: EIGHT KEYS OPEN THE WAY TO ITS PRINCE", sin.title())
        }
    } else {
        crate::logic::story::depth_line(g.dungeon, g.arena())
    }
}

/// A doorway in `color`: a dark arch with a swirl in it.
fn draw_portal_icon(o: &mut Overlay, x: f32, y: f32, size: f32, color: u32, alpha: f32) {
    let (cx, w) = (x + size / 2.0, size * 0.5);
    o.rect(cx - w / 2.0 - size * 0.08, y + size * 0.2, size * 0.08, size * 0.66, rgba(0x2e2638, alpha));
    o.rect(cx + w / 2.0, y + size * 0.2, size * 0.08, size * 0.66, rgba(0x2e2638, alpha));
    o.rect(cx - w / 2.0 - size * 0.12, y + size * 0.12, w + size * 0.24, size * 0.1, rgba(0x2e2638, alpha));
    o.rect(cx - w / 2.0, y + size * 0.22, w, size * 0.64, rgba(0x0a0608, alpha));
    for k in 0..3 {
        let r = size * (0.08 + k as f32 * 0.07);
        o.arc(cx, y + size * 0.54, r, k as f32, k as f32 + 4.2, (size * 0.03).max(1.5), rgba(color, alpha * (1.0 - k as f32 * 0.2)));
    }
    o.circle(cx, y + size * 0.54, size * 0.04, rgba(0xffffff, alpha));
}

/// The picture on a tour card: a portrait of a foe, or an icon of a thing.
fn draw_tour_art(o: &mut Overlay, hero: HeroKind, art: TourArt, x: f32, y: f32, size: f32, alpha: f32) {
    use EnemyKind;
    use TourArt;
    o.rect(x, y, size, size, [1.0, 1.0, 1.0, 0.06 * alpha]);
    let sp = sprites();
    let fit = |o: &mut Overlay, s: &Sprite| {
        let scale = (size * 0.9) / s.width.max(s.height) as f32;
        let (sw, sh) = (s.width as f32 * scale, s.height as f32 * scale);
        draw_sprite(o, s, x + (size - sw) / 2.0, y + (size - sh) / 2.0, scale, None, alpha);
    };
    match art {
        TourArt::Hero => fit(o, portrait_of(hero)),
        TourArt::Enemy(kind) => {
            let i = match kind {
                EnemyKind::Skeleton => 0,
                EnemyKind::Bat | EnemyKind::Vampire => 1,
                EnemyKind::Slime => 2,
                EnemyKind::Witch => 3,
                EnemyKind::Minion => 4,
                EnemyKind::Helmeted => 5,
                EnemyKind::Werewolf => 6,
                EnemyKind::Blemmyes => 7,
                EnemyKind::LargeBat => 8,
                EnemyKind::Tengu => 9,
                EnemyKind::Rasetsu => 10,
                EnemyKind::DireWolf => 11,
                EnemyKind::Jorogumo => 12,
                EnemyKind::Cobra => 13,
                EnemyKind::Oni => 14,
                EnemyKind::Kappa => 15,
                EnemyKind::Rokurokubi => 16,
                EnemyKind::Kitsune => 17,
                EnemyKind::Karakoncolos => 18,
                EnemyKind::Tulukus => 19,
                EnemyKind::Albasti => 20,
                EnemyKind::Cin => 21,
                EnemyKind::Arcura => 22,
                EnemyKind::FrostGiant => 23,
                EnemyKind::Surtr => 24,
                EnemyKind::Nidhogg => 25,
                EnemyKind::Hel => 26,
                EnemyKind::Tepegoz => 27,
                EnemyKind::Gashadokuro => 28,
                EnemyKind::Ejder => 29,
                EnemyKind::Goryo => 30,
                // (The realms' own, for now in their kin's likeness.)
                EnemyKind::Peerless => 5,
                EnemyKind::Ganglot => 0,
                EnemyKind::Gaki => sp.cast.iter().position(|c| c.1 == "Gaki").unwrap_or(7),
                EnemyKind::Ellepige => sp.cast.iter().position(|c| c.1 == "Ellepige").unwrap_or(3),
                // (The courts' own have no card of their own yet: the princes are drawn as the
                // blemmyes.)
                EnemyKind::Mammon
                | EnemyKind::GoldGolem
                | EnemyKind::Lucifer
                | EnemyKind::Reflection
                | EnemyKind::Leviathan
                | EnemyKind::Serpentling
                | EnemyKind::Amon
                | EnemyKind::Raven
                | EnemyKind::Belphegor
                | EnemyKind::Nightmare
                | EnemyKind::Beelzebub
                | EnemyKind::FlySwarm
                | EnemyKind::Asmodeus
                | EnemyKind::Krampus
                | EnemyKind::Percht => 7,
            };
            let tint = if kind == EnemyKind::Vampire { Some(0x9a3060) } else { None };
            let s = &sp.cast[i].0;
            let scale = (size * 0.9) / s.width.max(s.height) as f32;
            let (sw, sh) = (s.width as f32 * scale, s.height as f32 * scale);
            draw_sprite(o, s, x + (size - sw) / 2.0, y + (size - sh) / 2.0, scale, tint, alpha);
        }
        TourArt::Nugget => fit(o, icon(Icon::Key)),
        TourArt::Gate => fit(o, icon(Icon::Lock)),
        TourArt::Portal(color) => draw_portal_icon(o, x, y, size, color, alpha),
        TourArt::Seal => {
            // Seven sockets round a seal stone, each in its sin's colour.
            let (cx, cy) = (x + size / 2.0, y + size / 2.0);
            o.circle(cx, cy, size * 0.36, rgba(0x2e2638, alpha));
            o.circle(cx, cy, size * 0.12, rgba(0x6a1a10, alpha));
            for (i, sin) in SINS.iter().enumerate() {
                let a = i as f32 / 7.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                o.circle(cx + a.cos() * size * 0.25, cy + a.sin() * size * 0.25, size * 0.055, rgba(sin.color(), alpha));
            }
        }
        TourArt::Volcano => {
            let (cx, base) = (x + size / 2.0, y + size * 0.85);
            for k in 0..8 {
                let t = k as f32 / 8.0;
                let w = size * (0.8 - t * 0.6);
                o.rect(cx - w / 2.0, base - (k + 1) as f32 * size * 0.07, w, size * 0.075, rgba(0x2e2a2c, alpha));
            }
            o.rect(cx - size * 0.05, base - size * 0.56, size * 0.1, size * 0.5, rgba(0xff5a1a, alpha * 0.8));
            o.circle(cx, base - size * 0.62, size * 0.1, rgba(0xff8a2a, alpha));
            o.rect(cx - size * 0.12, base - size * 0.3, size * 0.24, size * 0.3, rgba(0x1c1622, alpha));
            o.rect(cx - size * 0.04, base - size * 0.16, size * 0.08, size * 0.16, rgba(0xff3010, alpha));
        }
    }
}

// ------------------------------------------------------------- world labels

/// Health bar over Ragnar, shrine labels, and floating text projected onto the overlay.
fn draw_world_overlays(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let grid = &g.level.grid;
    let view = &g.view;
    if g.hero.alive && g.screen == Screen::Playing {
        let hp = g.hero.pos();
        let p = Vec3::new(hp.x as f32, g.hero.altitude as f32 * STEP + 1.45, hp.y as f32);
        if let Some((x, y)) = view.project(p, w, h) {
            let bw = t * 1.8;
            let bh = (t * 0.22).max(5.0);
            let frac = g.hero.hp as f32 / g.hero.max_hp as f32;
            let trail = g.juice.hp_trail / g.hero.max_hp.max(1) as f32;
            style::bar(o, [x - bw / 2.0, y, bw, bh], frac, trail, hp_color(frac), 1.0);
        }
    }
    if g.hero.sees() && g.screen == Screen::Playing {
        // Seer's Sight: every key, bottle, cross and the hidden shrine marked, walls or not.
        let pulse = 0.75 + (g.time as f32 * 5.0).sin() * 0.25;
        let fade = 0.8;
        let mark = |o: &mut Overlay, x: i32, y: i32, color: u32| {
            let (fx, fy) = (x as f32, y as f32);
            if let Some((sx, sy)) = view.project(Vec3::new(fx, floor_at(grid, fx, fy) + 0.9, fy), w, h) {
                let r = t * 0.32 * pulse;
                o.line([sx, sy - r], [sx + r, sy], 2.0, rgba(color, fade));
                o.line([sx + r, sy], [sx, sy + r], 2.0, rgba(color, fade));
                o.line([sx, sy + r], [sx - r, sy], 2.0, rgba(color, fade));
                o.line([sx - r, sy], [sx, sy - r], 2.0, rgba(color, fade));
                o.line([sx, sy + r], [sx, sy + r * 2.2], 1.5, rgba(color, fade * 0.6));
            }
        };
        for (x, y, tile) in grid.cells() {
            match tile {
                Tile::Nodule => mark(o, x, y, GOLD),
                Tile::Charge => mark(o, x, y, 0x5cff6a),
                _ => {}
            }
        }
        for p in g.potions.iter().filter(|p| !p.taken) {
            mark(o, p.tile.x, p.tile.y, p.def(g.hero_kind).color);
        }
        if let Some(r) = g.relic_shrine {
            mark(o, r.x, r.y, 0x9ad1ff);
        }
    }
    // The Portals of Hell: each gate's sin named over it as he comes near; a conquered one says so.
    if g.hub.is_some() && matches!(g.screen, Screen::Playing | Screen::Ready) {
        let hp = g.hero.pos();
        for gate in hub::gates() {
            let (tx, ty) = (gate.tile.x as f32, gate.tile.y as f32);
            let d = ((hp.x as f32 - tx).powi(2) + (hp.y as f32 - ty).powi(2)).sqrt();
            if d > 11.0 {
                continue;
            }
            let fade = (1.0 - (d - 7.0) / 4.0).clamp(0.0, 1.0);
            if let Some((x, y)) = view.project(Vec3::new(tx, 4.7, ty), w, h) {
                let sin = gate.sin;
                o.text(sin.title(), x, y, t * 0.8, rgba(sin.color(), fade), Align::Center, true);
                let conquered = g.sins_cleared & sin.bit() != 0;
                let sub = if conquered { "CONQUERED".to_string() } else { sin.realm().to_uppercase() };
                o.text(&sub, x, y + t * 0.75, t * 0.45, rgba(if conquered { 0xffc860 } else { SAND }, fade), Align::Center, false);
            }
        }
    }
    for d in &g.diamonds {
        if !d.active {
            continue;
        }
        let (tx, ty) = (d.tile.x as f32, d.tile.y as f32);
        if let Some((x, y)) = view.project(Vec3::new(tx, floor_at(grid, tx, ty) + 1.25, ty), w, h) {
            let offer = d.current();
            let spent = match offer {
                Offer::Angel(a) => g.angels_spent()[a.index()].then_some(a),
                _ => None,
            };
            // (A name that would sit under the HUD's panels fades out rather than being cut by them.)
            let under = clear_of_hud(&layout::HudLayout::of(w, h), x, y, t * 3.2, t * 0.5);
            if under <= 0.0 {
                continue;
            }
            let keep = o.alpha;
            o.alpha *= under;
            if let Some(a) = spent {
                // One who has given this level (or Uriel resting): the name greyed; Uriel's rest counted down.
                let text = if a == Archangel::Uriel && !g.angels_used[a.index()] {
                    let left = g.uriel_rest.ceil() as u32;
                    format!("URIEL {}:{:02}", left / 60, left % 60)
                } else {
                    a.name().to_uppercase()
                };
                o.text_styled(&text, x, y, &style::name(t, 0.58, rgba(0x8a8a90, 1.0)).centered());
            } else {
                // A name in the display serif, lit in its colour.
                let c = offer.color();
                let st = style::name(t, 0.64, rgba(mix_rgb(c, 0xffffff, 0.25), 1.0)).centered().glow(t * 0.22, rgba(c, 0.45));
                o.text_styled(&offer.name().to_uppercase(), x, y, &st);
            }
            o.alpha = keep;
        }
    }
    for f in &g.floaters {
        let (fx, fy) = (f.x as f32, f.y as f32);
        let lift = floor_at(grid, fx, fy) + 0.9 + (1.0 - f.life as f32) * 0.6;
        if let Some((x, y)) = view.project(Vec3::new(fx, lift, fy), w, h) {
            let alpha = (f.life as f32 * 2.0).min(1.0);
            // Pop in big, settle, then drift up and fade.
            let age = (f.max_life - f.life) as f32;
            let pop = 1.0 + 0.8 * (1.0 - (age / 0.15).min(1.0)).powi(2);
            let st = Style::body(t * 0.68 * pop, rgba(f.color.unwrap_or(GOLD), alpha))
                .weight(0.45)
                .outline((t * 0.06).max(1.0), [0.0, 0.0, 0.0, 0.7 * alpha])
                .centered();
            o.text_styled(&f.text, x, y, &st);
        }
    }
}

// ------------------------------------------------------------------ screens

/// Button rectangles `[x, y, w, h]` for a menu, in logical pixels. The title
/// menu is a row along the bottom; the pause menu is a centered column.
pub fn menu_rects(screen: Screen, items: &[MenuItem], w: f32, h: f32) -> Vec<[f32; 4]> {
    let t = tile_for(w, h);
    let bh = t * 1.3;
    let count = items.len();
    match screen {
        Screen::View => {
            // Two cards side by side, a picture in each.
            let [cw, ch] = view_card(w, h);
            let gap = t * 1.2;
            let x0 = w / 2.0 - (count as f32 * cw + (count as f32 - 1.0) * gap) / 2.0;
            (0..count).map(|i| [x0 + i as f32 * (cw + gap), h / 2.0 - ch / 2.0 + t * 0.3, cw, ch]).collect()
        }
        Screen::Title if items.contains(&MenuItem::StartOver) => {
            // The two answers side by side at the foot of the "start over?" panel.
            let [px, py, pw, ph] = reset_panel(w, h);
            let (bw, bh, gap) = ((pw - t * 2.2) / 2.0, t * 1.4, t * 0.6);
            let y = py + ph - bh - t * 0.8;
            (0..count).map(|i| [px + (pw - 2.0 * bw - gap) / 2.0 + i as f32 * (bw + gap), y, bw, bh]).collect()
        }
        Screen::Title => {
            // Continue and Start side by side in the middle below the line-up; the controller page,
            // the settings (and the developer's map) small in the bottom-left corner; Quit (desktop)
            // small in the other.
            let (bw, bh, gap) = (t * 9.0, t * 1.9, t * 0.9);
            let small = |i: &MenuItem| matches!(i, MenuItem::Controls | MenuItem::Settings | MenuItem::Map | MenuItem::DevMap);
            let big = items.iter().filter(|i| !matches!(i, MenuItem::Quit) && !small(i)).count();
            let x0 = w / 2.0 - (big as f32 * bw + (big as f32 - 1.0) * gap) / 2.0;
            let (cw, ch) = (t * 4.6, t * 1.1);
            let mut corner = 0.0;
            items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    if *item == MenuItem::Quit {
                        [w - cw - t * 0.6, h - ch - t * 0.6, cw, ch]
                    } else if small(item) {
                        let x = t * 0.6 + corner * (cw + t * 0.4);
                        corner += 1.0;
                        [x, h - ch - t * 0.6, cw, ch]
                    } else {
                        [x0 + i as f32 * (bw + gap), h * 0.875 - bh / 2.0, bw, bh]
                    }
                })
                .collect()
        }
        _ => {
            let bw = t * 9.0;
            // (A longer list, the pause menu with the feel's settings, packs its buttons closer.)
            let tight = (7.0 / count.max(7) as f32).max(0.75);
            let (bh, gap) = (bh * tight, t * 0.45 * tight);
            // (A list longer than seven is centred as a whole, so what is written under it, the
            // road down in the pause menu, stays clear of the belt at the bottom.)
            let total = count as f32 * (bh + gap) - gap;
            let y0 = if count > 7 { (h / 2.0 - total / 2.0 - t * 1.2).max(t * 4.2) } else { h / 2.0 - t * 0.6 };
            (0..count).map(|i| [w / 2.0 - bw / 2.0, y0 + i as f32 * (bh + gap), bw, bh]).collect()
        }
    }
}

/// The panel that asks whether to start over, in the middle of the title.
fn reset_panel(w: f32, h: f32) -> [f32; 4] {
    let t = tile_for(w, h);
    let (pw, ph) = ((t * 22.0).min(w - t * 1.2), t * 7.4);
    [w / 2.0 - pw / 2.0, h / 2.0 - ph / 2.0, pw, ph]
}

/// The current screen's buttons, with the highlighted one outlined in gold.
fn draw_menu(o: &mut Overlay, g: &Game, t: f32) {
    let items = g.menu_items();
    let rects = menu_rects(g.screen, &items, o.width, o.height);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    for (i, (item, r)) in items.iter().zip(rects).enumerate() {
        let selected = i == g.menu_index;
        let quit = matches!(item, MenuItem::Quit | MenuItem::QuitToTitle | MenuItem::StartOver);
        let bg = if selected { rgba(COPPER, 0.24) } else { [1.0, 1.0, 1.0, 0.03] };
        o.rect(r[0], r[1], r[2], r[3], [INK[0], INK[1], INK[2], 0.88]);
        o.rect(r[0], r[1], r[2], r[3], bg);
        let edge = if selected { rgba(GOLD, pulse) } else { rgba(COPPER, 0.6) };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), edge);
        let color = if selected {
            if quit {
                0xff8a8a
            } else {
                GOLD
            }
        } else {
            TEXT_DIM
        };
        let big = matches!(item, MenuItem::Play | MenuItem::Continue);
        if big && selected {
            // The highlighted big button glows gold and breathes.
            o.rect_outline(
                r[0] - t * 0.12,
                r[1] - t * 0.12,
                r[2] + t * 0.24,
                r[3] + t * 0.24,
                (t * 0.05).max(1.0),
                rgba(GOLD, 0.35 * pulse),
            );
        }
        let color = if big && !selected { SAND } else { color };
        o.text(
            item.label(),
            r[0] + r[2] / 2.0,
            r[1] + r[3] / 2.0,
            t * if big { 1.15 } else { 0.55 },
            rgba(color, 1.0),
            Align::Center,
            true,
        );
    }
}

/// A realm's gauges (`Game::run_meters`), right-aligned at `right` from `y` down: a label and a bar,
/// or a label and a number; a warning pulses.
fn draw_run_meters(o: &mut Overlay, g: &Game, t: f32, right: f32, y: f32, small: f32) {
    let bar_w = t * 4.2;
    let bar_h = small * 0.55;
    let pulse = 0.75 + (g.time as f32 * 8.0).sin().abs() * 0.25;
    for (i, m) in g.run_meters().iter().enumerate() {
        let y = y + i as f32 * small * 1.9;
        let a = if m.warn { pulse } else { 1.0 };
        if m.count {
            let n = format!("{}", m.value.round() as i64);
            let ns = small * 1.25;
            o.text(&n, right, y, ns, rgba(m.color, a), Align::Right, true);
            o.text(&m.label, right - o.measure(&n, ns) - t * 0.3, y, small, rgba(m.color, a), Align::Right, true);
            continue;
        }
        let x = right - bar_w;
        let k = if m.max > 0.0 { (m.value / m.max).clamp(0.0, 1.0) } else { 0.0 };
        o.rect(x, y - bar_h / 2.0, bar_w, bar_h, [0.0, 0.0, 0.0, 0.55]);
        o.rect(x, y - bar_h / 2.0, bar_w * k, bar_h, rgba(m.color, 0.9 * a));
        o.rect_outline(x, y - bar_h / 2.0, bar_w, bar_h, t * 0.04, rgba(m.color, 0.6));
        o.text(&m.label, x - t * 0.3, y, small, rgba(m.color, a), Align::Right, true);
    }
}

/// A prince's offer in one of the realms after the vaults: its title, what is said, and the answers.
fn draw_offer(o: &mut Overlay, g: &Game, t: f32) {
    let Some(offer) = g.run_offer() else { return };
    o.rect(0.0, 0.0, o.width, o.height, [0.02, 0.02, 0.04, 0.72]);
    let top = menu_rects(Screen::Toll, &offer.choices, o.width, o.height).first().map(|r| r[1]).unwrap_or(o.height * 0.6);
    let pulse = 0.9 + (g.time as f32 * 3.0).sin() * 0.1;
    let n = offer.lines.len() as f32;
    o.text(&offer.title, o.width / 2.0, top - t * (2.6 + n), t * 1.45, rgba(offer.color, pulse), Align::Center, true);
    for (i, line) in offer.lines.iter().enumerate() {
        let y = top - t * (1.5 + n - i as f32);
        o.text(
            line,
            o.width / 2.0,
            y,
            t * 0.66,
            rgba(if i == 0 { mix_rgb(offer.color, 0xffffff, 0.5) } else { SAND }, 1.0),
            Align::Center,
            true,
        );
    }
    draw_menu(o, g, t);
}

/// A camera card's width and height: a picture of 16 by 9 over its name and a line.
fn view_card(w: f32, h: f32) -> [f32; 2] {
    let t = tile_for(w, h);
    let cw = (w * 0.4).min(t * 16.0).min((h * 0.55 - t * 3.2) * 16.0 / 9.0 + t * 0.8);
    [cw, (cw - t * 0.8) * 9.0 / 16.0 + t * 3.6]
}

/// Just before play: the two cameras, each a card with a picture of how it sees the Garden, its
/// name and a line; the highlighted card in gold. The world behind is seen as it would be.
fn draw_view_choice(o: &mut Overlay, g: &Game, t: f32) {
    o.rect(0.0, 0.0, o.width, o.height, [INK[0], INK[1], INK[2], 0.55]);
    let items = g.menu_items();
    let rects = menu_rects(Screen::View, &items, o.width, o.height);
    let Some(first) = rects.first() else { return };
    o.text("CHOOSE YOUR VIEW", o.width / 2.0, first[1] - t * 1.9, t * 1.45, rgba(TEXT, 1.0), Align::Center, true);
    let pulse = 0.75 + (g.time as f32 * 4.0).sin() * 0.25;
    let pad = t * 0.4;
    for (i, (item, r)) in items.iter().zip(&rects).enumerate() {
        let selected = i == g.menu_index;
        let (picture, name, what) = match item {
            MenuItem::ViewTop => (Picture::ViewTop, "TOP-DOWN", "From above: the whole ground and every foe in sight."),
            _ => (Picture::ViewFollow, "FOLLOW", "Over his shoulder: up close, the mouse turns the view."),
        };
        o.rect(r[0], r[1], r[2], r[3], [INK[0], INK[1], INK[2], 0.9]);
        let pw = r[2] - pad * 2.0;
        let ph = pw * 9.0 / 16.0;
        o.picture(picture, r[0] + pad, r[1] + pad, pw, ph, if selected { 1.0 } else { 0.6 });
        let edge = if selected { rgba(GOLD, pulse) } else { [EDGE[0], EDGE[1], EDGE[2], 0.32] };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * if selected { 0.14 } else { 0.08 }).max(1.0), edge);
        let y = r[1] + pad + ph + t * 0.95;
        o.text(name, r[0] + r[2] / 2.0, y, t * 0.95, rgba(if selected { GOLD } else { TEXT_DIM }, 1.0), Align::Center, true);
        o.wrap_text(
            what,
            r[0] + r[2] / 2.0,
            y + t * 1.05,
            r[2] - pad * 2.0,
            t * 0.62,
            t * 0.48,
            rgba(SAND, if selected { 1.0 } else { 0.7 }),
        );
    }
    let foot = rects.iter().map(|r| r[1] + r[3]).fold(0.0, f32::max) + t * 1.2;
    glyphs::prompt_line(o, &g.input, "{NavLR} choose    {Confirm} play", o.width / 2.0, foot, t * 0.6, rgba(TEXT, 0.9), Align::Center);
    glyphs::prompt_line(
        o,
        &g.input,
        "Switch at any time with {Camera}, or from the pause menu",
        o.width / 2.0,
        foot + t * 0.85,
        t * 0.5,
        rgba(SAND, 0.85),
        Align::Center,
    );
}

/// Mammon's toll, before the far portal of his vaults: what he carries, and the choice.
fn draw_toll(o: &mut Overlay, g: &Game, t: f32) {
    if !g.in_vault() || g.deceit_offer().is_some() {
        draw_offer(o, g, t);
        return;
    }
    o.rect(0.0, 0.0, o.width, o.height, [0.06, 0.03, 0.0, 0.72]);
    let top = menu_rects(Screen::Toll, &[MenuItem::KeepAll], o.width, o.height)[0][1];
    let pulse = 0.9 + (g.time as f32 * 3.0).sin() * 0.1;
    o.text("MAMMON'S TOLL", o.width / 2.0, top - t * 4.6, t * 1.45, rgba(GOLD, pulse), Align::Center, true);
    let h = g.hoard;
    let carried = if h.is_empty() {
        "You carry nothing out of the vaults.".to_string()
    } else {
        format!(
            "You carry {} gold: {} coins, {} heaps, {} treasures, {} chests, {} gems.",
            h.gold, h.coins, h.piles, h.treasures, h.chests, h.gems
        )
    };
    o.text(&carried, o.width / 2.0, top - t * 3.4, t * 0.62, rgba(SAND, 1.0), Align::Center, true);
    // His offer, and nothing of what keeping costs: there is no chance, and keeping always begins the vaults again.
    o.text(
        "There is a chance for you to keep the gold.",
        o.width / 2.0,
        top - t * 2.45,
        t * 0.7,
        rgba(0xffe08a, pulse),
        Align::Center,
        true,
    );
    o.text("Keep it, or leave it.", o.width / 2.0, top - t * 1.5, t * 0.55, rgba(TEXT_DIM, 1.0), Align::Center, false);
    draw_menu(o, g, t);
}

/// Centered overlay for ready, pause, win, and game over screens.
fn draw_screen(o: &mut Overlay, input: &crate::input::Input, t: f32, lines: &[OverlayLine], dim: f32) {
    let (w, h) = (o.width, o.height);
    o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], dim]);
    let total: f32 = lines.iter().map(|l| l.size * t * 1.4).sum();
    let mut y = h / 2.0 - total / 2.0;
    for l in lines {
        let size = l.size * t * 0.9;
        y += size * 0.7;
        if l.text.contains('{') {
            glyphs::prompt_line(o, input, &l.text, w / 2.0, y, size, rgba(l.color, 1.0), Align::Center);
        } else {
            o.text(&l.text, w / 2.0, y, size, rgba(l.color, 1.0), Align::Center, true);
        }
        y += size * 0.85;
    }
}

/// Start screen. The real 3D Ragnar stands in the middle while the camera
/// circles him; the logo, the quest, the foes and the controls frame him.
/// A smooth top-to-bottom shade over the whole screen in `rgb`, opacity `alpha(v)` at the height
/// `v` (0 top, 1 bottom): rows a few pixels high meeting exactly, so no seam is shaded twice.
fn shade_rows(o: &mut Overlay, w: f32, h: f32, rgb: [f32; 3], alpha: impl Fn(f32) -> f32) {
    let rows = (h / 3.0).ceil().max(1.0) as usize;
    for i in 0..rows {
        let (y0, y1) = ((h * i as f32 / rows as f32).round(), (h * (i + 1) as f32 / rows as f32).round());
        let v = (i as f32 + 0.5) / rows as f32;
        o.rect(0.0, y0, w, y1 - y0, [rgb[0], rgb[1], rgb[2], alpha(v)]);
    }
}

fn draw_title(o: &mut Overlay, g: &Game, t: f32) {
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;

    // Shade the scene top and bottom, keep the middle clear for Ragnar.
    shade_rows(o, w, h, INK, |v| {
        if v < 0.3 {
            0.8 - v / 0.3 * 0.62
        } else if v < 0.76 {
            0.18
        } else {
            0.18 + (v - 0.76) / 0.24 * 0.6
        }
    });
    // Side vignettes behind the panels.
    for i in 0..10 {
        let a = 0.05 * (10 - i) as f32 / 10.0;
        o.rect(0.0, 0.0, w * 0.035 * (i + 1) as f32, h, [0.0, 0.0, 0.0, a]);
        o.rect(w - w * 0.035 * (i + 1) as f32, 0.0, w * 0.035 * (i + 1) as f32, h, [0.0, 0.0, 0.0, a]);
    }

    // ---- logo
    o.text("ADVENTURES OF THE", w / 2.0, h * 0.075, t * 0.95, rgba(SAND, 1.0), Align::Center, true);
    draw_logo_word(o, "ANCIENT BLADES", w / 2.0, h * 0.075 + t * 2.6, t * 2.9, time);
    let orn_y = h * 0.075 + t * 4.55;
    draw_ornament(o, w / 2.0, orn_y, t * 15.0, t, time);
    // The three heroes' names, small gold diamonds between them.
    {
        let (y, size) = (orn_y + t * 0.95, t * 0.6);
        let gap = t * 1.4;
        let names = ["Mehmed", "Ragnar", "Miyamoto"];
        let total: f32 = names.iter().map(|n| o.measure(n, size)).sum::<f32>() + gap * 2.0;
        let mut x = w / 2.0 - total / 2.0;
        for (i, n) in names.iter().enumerate() {
            o.text(n, x, y, size, rgba(0xcfc2a0, 1.0), Align::Left, false);
            x += o.measure(n, size);
            if i < 2 {
                let (cx, r, lw) = (x + gap / 2.0, t * 0.16, (t * 0.07).max(1.2));
                let c = rgba(GOLD, 0.9);
                o.line([cx - r, y], [cx, y - r], lw, c);
                o.line([cx, y - r], [cx + r, y], lw, c);
                o.line([cx + r, y], [cx, y + r], lw, c);
                o.line([cx, y + r], [cx - r, y], lw, c);
                x += gap;
            }
        }
    }

    // The release's version, small in the top left corner.
    o.text(concat!("v", env!("CARGO_PKG_VERSION")), t * 0.6, t * 0.9, t * 0.45, rgba(TEXT_DIM, 0.8), Align::Left, false);

    // One line on how to play, tucked under the button.
    glyphs::prompt_line(
        o,
        &g.input,
        "{Move} run   {Attack} attack   {Jump} jump   {Block} shield   {Step} step   {Angel} archangel   {Guide} his word   {Call} call the old man   {Pause} pause",
        w / 2.0,
        h * 0.965,
        t * 0.45,
        rgba(TEXT_DIM, 0.9),
        Align::Center,
    );

    // Under the buttons: where Continue leads, and that Start begins anew.
    if !g.confirm_reset && g.can_continue() {
        let items = g.menu_items();
        let rects = menu_rects(g.screen, &items, w, h);
        for (item, r) in items.iter().zip(&rects) {
            let note = match item {
                MenuItem::Continue => g.continue_caption().unwrap_or_else(|| "Choose a hero and keep what is earned".to_string()),
                MenuItem::Play => "A new adventure, all reset".to_string(),
                _ => continue,
            };
            o.wrap_text(&note, r[0] + r[2] / 2.0, r[1] + r[3] + t * 0.5, r[2], t * 0.55, t * 0.42, rgba(TEXT_DIM, 1.0));
        }
    }
    if g.confirm_reset {
        o.rect(0.0, 0.0, w, h, [INK[0], INK[1], INK[2], 0.6]);
        let [px, py, pw, ph] = reset_panel(w, h);
        panel(o, px, py, pw, ph, t);
        o.text("START OVER?", w / 2.0, py + t * 1.35, t * 1.1, rgba(GOLD, 1.0), Align::Center, true);
        o.wrap_text(
            "Every hero's weapons, shields, paths and elixirs, the silver, the runes and the adventure under way will be lost.",
            w / 2.0,
            py + t * 2.6,
            pw - t * 2.4,
            t * 0.72,
            t * 0.5,
            rgba(SAND, 1.0),
        );
    }
    draw_menu(o, g, t);
    o.text(&format!("Best {}", g.score.high), w - t * 0.6, h * 0.035, t * 0.5, rgba(TEXT_DIM, 1.0), Align::Right, false);
    ending::draw_finished_mark(o, g, t);
}

/// The title word in the display serif: a dark outline, a bronze bevel below, gold on top, and a
/// shimmer of light sweeping through the letters, each letter bobbing on its own (set by the
/// face's own advances, so the word keeps its spacing).
fn draw_logo_word(o: &mut Overlay, word: &str, cx: f32, cy: f32, size: f32, time: f32) {
    let style = Style::display(size, rgba(GOLD, 1.0)).tracking(0.04);
    let n = word.chars().count() as f32;
    let (pens, width) = o.pens(word, &style);
    let x0 = cx - width / 2.0;
    let sweep = ((time * 0.35) % 1.6) * (n + 6.0) - 3.0;
    let d = (size * 0.05).max(2.0);
    for (i, (ch, pen)) in word.chars().zip(pens).enumerate() {
        let x = x0 + pen;
        let s = ch.to_string();
        let bounce = (time * 2.2 + i as f32 * 0.5).sin() * size * 0.015;
        let y = cy + bounce;
        let edge = style.outline(d, [0.0, 0.0, 0.0, 0.85]).shadow(d * 0.6, d);
        o.text_styled(&s, x, y + d, &edge.color([0.0, 0.0, 0.0, 0.85]));
        o.text_styled(&s, x, y + d * 1.2, &style.color(rgba(0x8a5a14, 1.0)));
        let glint = (1.0 - ((i as f32 - sweep).abs() / 1.6)).clamp(0.0, 1.0);
        let color = mix_rgb(GOLD, 0xfff6d0, glint * 0.85);
        o.text_styled(&s, x, y, &style.color(rgba(color, 1.0)));
    }
}

// ---------------------------------------------------------- character screen

/// A rectangle on screen: x, y, width, height.
#[derive(Clone, Copy, Debug)]
pub struct Rect(pub [f32; 4]);

impl Rect {
    pub fn contains(&self, p: (f32, f32)) -> bool {
        let r = self.0;
        p.0 >= r[0] && p.0 <= r[0] + r[2] && p.1 >= r[1] && p.1 <= r[1] + r[3]
    }
}

/// Where the character screen's parts sit: a third of the screen for each hero, and the buttons.
pub struct SelectLayout {
    pub heroes: [Rect; 3],
    pub back: Rect,
    pub begin: Rect,
}

impl SelectLayout {
    pub fn hero_at(&self, p: (f32, f32)) -> Option<usize> {
        self.heroes.iter().position(|r| r.contains(p))
    }
}

pub fn select_layout(w: f32, h: f32) -> SelectLayout {
    let t = tile_for(w, h);
    let (bw, bh, gap) = (t * 6.0, t * 1.3, t * 0.8);
    let y = h * 0.93 - bh / 2.0;
    SelectLayout {
        heroes: [0, 1, 2].map(|i| Rect([w * i as f32 / 3.0, h * 0.18, w / 3.0, h * 0.62])),
        back: Rect([w / 2.0 - gap / 2.0 - bw, y, bw, bh]),
        begin: Rect([w / 2.0 + gap / 2.0, y, bw, bh]),
    }
}

/**
 * The character screen over its 3D cavern: the title, a name plate under
 * each hero (grey with a lock for those not yet unlocked, gold for the one
 * picked), and Back and Begin.
 */
fn draw_select(o: &mut Overlay, g: &Game, t: f32) {
    use ROSTER;
    let (w, h) = (o.width, o.height);
    let time = g.time as f32;
    // Darken the top and bottom so the words read over the cavern.
    shade_rows(o, w, h, INK, |v| {
        if v < 0.2 {
            0.78 - v / 0.2 * 0.6
        } else if v < 0.7 {
            0.18
        } else {
            0.18 + (v - 0.7) / 0.3 * 0.62
        }
    });

    // ---- title
    o.text("ADVENTURES OF THE ANCIENT BLADES", w / 2.0, h * 0.045, t * 0.55, rgba(SAND, 0.9), Align::Center, false);
    draw_logo_word(o, "CHOOSE YOUR HERO", w / 2.0, h * 0.045 + t * 1.55, t * 1.55, time);
    draw_ornament(o, w / 2.0, h * 0.045 + t * 2.85, t * 11.0, t, time);

    // ---- a plate under each hero's feet
    let hero = g.select_hero();
    for (i, &kind) in ROSTER.iter().enumerate() {
        let picked = i == g.select_index;
        let Some((sx, sy)) = g.view.project(g.view.select_feet(i), w, h) else { continue };
        let shake = if picked && g.select_denied > 0.0 { (g.select_denied as f32 * 70.0).sin() * t * 0.3 } else { 0.0 };
        let (pw, ph) = (t * 6.4, t * 1.9);
        let (px, py) = (sx - pw / 2.0 + shake, sy + t * 0.35);
        let open = kind.unlocked();
        o.rect(px, py, pw, ph, [INK[0], INK[1], INK[2], if picked { 0.88 } else { 0.7 }]);
        let pulse = 0.7 + (time * 4.0).sin() * 0.3;
        let edge = match (picked, open) {
            (true, true) => rgba(GOLD, pulse),
            (true, false) => rgba(if g.select_denied > 0.0 { 0xff5a5a } else { 0x9aa4b4 }, 0.9),
            _ => [1.0, 1.0, 1.0, 0.15],
        };
        o.rect_outline(px, py, pw, ph, (t * if picked { 0.1 } else { 0.06 }).max(1.0), edge);
        let name_color = match (open, picked) {
            (true, true) => GOLD,
            (true, false) => SAND,
            (false, _) => 0x8c929c,
        };
        let cx = px + pw / 2.0;
        o.text(
            &kind.name().to_uppercase(),
            cx,
            py + ph * 0.36,
            t * if picked { 0.95 } else { 0.8 },
            rgba(name_color, 1.0),
            Align::Center,
            true,
        );
        o.text(kind.title(), cx, py + ph * 0.74, t * 0.5, rgba(if open { TEXT_DIM } else { 0x6c727c }, 1.0), Align::Center, false);
        if !open {
            // A padlock on the plate's shoulder, and the word under it.
            let lock = icon(Icon::Lock);
            let scale = (t * 1.1) / lock.height.max(1) as f32;
            draw_sprite(o, lock, px - lock.width as f32 * scale * 0.4, py - lock.height as f32 * scale * 0.45, scale, Some(0xb4bcc8), 1.0);
            o.text(
                "LOCKED",
                cx,
                py + ph + t * 0.45,
                t * 0.48,
                rgba(if picked && g.select_denied > 0.0 { 0xff6a6a } else { 0x8c929c }, 1.0),
                Align::Center,
                true,
            );
        }
    }

    // ---- buttons: Back, and Begin (dimmed while a locked hero is picked)
    let layout = select_layout(w, h);
    for (r, label, live) in [(layout.back, "BACK", true), (layout.begin, "BEGIN", hero.unlocked())] {
        let r = r.0;
        let hover = g.input.cursor.is_some_and(|p| Rect(r).contains(p));
        o.rect(r[0], r[1], r[2], r[3], [INK[0], INK[1], INK[2], 0.85]);
        let main = label == "BEGIN";
        let edge = if main && live {
            rgba(GOLD, 0.75 + (time * 4.0).sin() * 0.25)
        } else if hover {
            [1.0, 1.0, 1.0, 0.4]
        } else {
            [EDGE[0], EDGE[1], EDGE[2], 0.32]
        };
        o.rect_outline(r[0], r[1], r[2], r[3], (t * 0.08).max(1.0), edge);
        let color = if !live {
            0x5c626c
        } else if main {
            GOLD
        } else {
            TEXT_DIM
        };
        o.text(label, r[0] + r[2] / 2.0, r[1] + r[3] / 2.0, t * 0.7, rgba(color, 1.0), Align::Center, true);
    }
    glyphs::prompt_line(
        o,
        &g.input,
        "{NavLR} choose     {Confirm} begin     {Back} back",
        w / 2.0,
        h * 0.985,
        t * 0.45,
        rgba(TEXT_DIM, 1.0),
        Align::Center,
    );
}

/// A knotwork-style rule: a line with diamonds at the ends and a gem in the middle.
fn draw_ornament(o: &mut Overlay, cx: f32, cy: f32, width: f32, t: f32, time: f32) {
    let lw = (t * 0.08).max(1.5);
    let c = rgba(GOLD, 0.85);
    let half = width / 2.0;
    o.line([cx - half, cy], [cx - t * 0.8, cy], lw, c);
    o.line([cx + t * 0.8, cy], [cx + half, cy], lw, c);
    let diamond = |o: &mut Overlay, x: f32, r: f32, color: [f32; 4]| {
        o.line([x - r, cy], [x, cy - r], lw, color);
        o.line([x, cy - r], [x + r, cy], lw, color);
        o.line([x + r, cy], [x, cy + r], lw, color);
        o.line([x, cy + r], [x - r, cy], lw, color);
    };
    diamond(o, cx - half, t * 0.3, c);
    diamond(o, cx + half, t * 0.3, c);
    diamond(o, cx, t * 0.5, c);
    let pulse = 0.6 + (time * 3.0).sin() * 0.4;
    o.circle(cx, cy, t * 0.2, rgba(0xff8a2a, pulse));
    for k in [-1.0, 1.0] {
        diamond(o, cx + k * half * 0.5, t * 0.18, rgba(GOLD, 0.6));
    }
}

/// Left-aligned word wrap. Returns the y just below the last line.
#[allow(clippy::too_many_arguments)]
fn wrap_left(o: &mut Overlay, text: &str, x: f32, mut y: f32, max_width: f32, line_height: f32, size: f32, color: [f32; 4]) -> f32 {
    let mut line = String::new();
    for word in text.split(' ') {
        let trial = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
        if o.measure(&trial, size) > max_width && !line.is_empty() {
            o.text(&line, x, y, size, color, Align::Left, false);
            y += line_height;
            line = word.to_string();
        } else {
            line = trial;
        }
    }
    if !line.is_empty() {
        o.text(&line, x, y, size, color, Align::Left, false);
        y += line_height;
    }
    y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quick_load_is_a_cut_through_black_and_a_slow_one_says_so() {
        // The Garden's cold start (about 0.2 s, one long frame in it) shows no words at all.
        for age in [0.0, 0.12, 0.2, 0.35, 0.5, LOADING_WORDS_AFTER] {
            assert_eq!(loading_words(age), 0.0, "{age} s");
        }
        assert!(loading_words(LOADING_WORDS_AFTER + 0.1) > 0.0);
        assert_eq!(loading_words(LOADING_WORDS_AFTER + LOADING_WORDS_FADE), 1.0);
    }
}

/// How far a label of half-size `hw` × `hh` centred at (`x`, `y`) is clear of play's HUD panels:
/// 1 when it is a unit clear of them all, falling to 0 as it reaches one (and 0 under one).
fn clear_of_hud(l: &layout::HudLayout, x: f32, y: f32, hw: f32, hh: f32) -> f32 {
    let margin = l.t * 0.6;
    [l.health, l.minimap, l.purse, l.top, l.loadout, l.angel, l.location]
        .iter()
        .filter(|r| r[2] > 0.0 && r[3] > 0.0)
        .map(|r| {
            // The gap between the label's box and the panel's (negative when they overlap).
            let gx = (r[0] - (x + hw)).max(x - hw - (r[0] + r[2]));
            let gy = (r[1] - (y + hh)).max(y - hh - (r[1] + r[3]));
            (gx.max(gy) / margin).clamp(0.0, 1.0)
        })
        .fold(1.0, f32::min)
}

#[cfg(test)]
mod hud_label_tests {
    use super::*;

    #[test]
    fn a_world_label_fades_out_under_the_hud_panels() {
        let l = layout::HudLayout::of(1920.0, 1080.0);
        let (cx, cy) = (l.health[0] + l.health[2] / 2.0, l.health[1] + l.health[3] / 2.0);
        assert_eq!(clear_of_hud(&l, cx, cy, 40.0, 10.0), 0.0, "under the health panel");
        assert_eq!(clear_of_hud(&l, 960.0, 540.0, 40.0, 10.0), 1.0, "mid-screen");
        let edge = clear_of_hud(&l, l.health[0] + l.health[2] + 40.0 + l.t * 0.3, cy, 40.0, 10.0);
        assert!(edge > 0.0 && edge < 1.0, "fading near it: {edge}");
    }
}
