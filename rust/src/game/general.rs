//! The generals in play (WARBAND.md §9): the general of the realm at his command post with his
//! guard, and his orders run through the instruments the hero can break (`logic::war::general`
//! holds his mind and his realm's plan).
//!
//! * **Watch**: the patrol asleep at the watch-fire rises and walks its round; if it sees him, its
//!   whistler whistles for a second, and the dens near wake (unless the whistler falls first).
//! * **Muster**: a horn (or bell) for four seconds, the gate nearest him glows, and a pack marches
//!   in through it; standing at the gate a second and a half bars it for good.
//! * **Kindle**: the realm's hazard wakes on the floor about him, each mark lying three seconds
//!   before it bursts, every burst planned by the threat manager as one strike; it never slows him.
//!   Turned (F at the bellows), it burns the foes instead.
//! * **Decree**: carriers walk from his post to the court's portal; whatever reaches it, the
//!   prince fights with. Kill them or cut the chain.
//!
//! Every instrument breaks under three of his blows; the general answers each loss once (a guard to
//! the next instrument, or the muster doubled), announced as any order is. Slain, the general's
//! orders end, his banner goes to the workshop's wall, and the prince is weakened.

use super::workshop::{GeneralCard, Trophy};
use super::{Banner, Game};
use crate::audio::Sfx;
use crate::input::Action;
use crate::logic::combat::duelist::{Den, Lore, HERO_RADIUS};
use crate::logic::combat::fsm::Hit;
use crate::logic::combat::threat::Request;
use crate::logic::combat::{math, Breed, Duelist, MoveId, Phase};
use crate::logic::enemy::{spec_of, Enemy, EnemyKind, EnemyMode};
use crate::logic::juice::Trauma;
use crate::logic::realms::{Dealer, Run};
use crate::logic::sins::Sin;
use crate::logic::types::{dist_sq, P, V};
use crate::logic::war::general::{
    self, place, Command, Counter, Cue, General, OrderKind, OrderStage, Posts, PrinceTerms, Records, BAR_FRAMES, CARRIER_PACE, HAZARD_IDS,
    KINDLE_EVERY, KINDLE_FUSE, KINDLE_MARKS, KINDLE_RADIUS, KINDLE_ZONE, MUSTER, PATROL, PATROL_SIGHT, WHISTLE, WHISTLE_WAKES,
};
use crate::logic::war::ledger::RivalId;
use crate::logic::war::WarEvent;

/// How long an order's card stays up (seconds).
pub const CARD_SECONDS: f64 = 3.0;
/// How near (tiles) he must be to turn the bellows, and to stand at a gate to bar it.
const TURN_REACH: f64 = 1.4;
const BAR_REACH: f64 = 1.3;
/// A marching pack is let loose this near him (tiles).
const LOOSE_AT: f64 = 9.0;
/// A kindled burst's damage, its poise on a foe it burns, and what it does on a turned bellows.
const BURST_DAMAGE: f64 = 22.0;
const BURST_POISE: f64 = 35.0;
/// The general's own second wind: at this share of his health he calls his guard, and they come
/// this long after the horn.
const RALLY_AT: f64 = 0.5;
const RALLY_LEAD: u32 = 90;
/// A muster waits while more than this many would hunt him with it.
const MUSTER_ROOM: usize = 6;
/// Runes the patrol's leader drops if he is ambushed before the whistle.
const AMBUSH_RUNES: u32 = 2;

/// The slowest a herald runs (tiles a second), however long his muster's horn.
pub const HERALD_PACE: f64 = 3.2;

/// A puppet on its way: which duelist, the tiles of its way, how far along (tiles), and what it
/// does at the end.
#[derive(Clone, Debug, PartialEq)]
pub struct Walker {
    pub id: u32,
    pub path: Vec<P>,
    pub at: f64,
    pub pace: f64,
    pub then: Then,
    /// Its health when last seen (struck on the way, it turns on him).
    hp: f64,
}

/// What a walker does when its way runs out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Then {
    /// A mustered pack's foe: it hunts him.
    Hunt,
    /// A carrier: into the court's portal with what it carries.
    Court,
    /// The watch: round again while the order stands, else asleep at the fire.
    Round,
    /// A guard sent to an instrument: asleep there, its den about it.
    Hold(P),
    /// The herald carrying muster `order` from the horn to its gate (WARBAND.md §9.1): caught
    /// before it bites, the muster fails, and he drops a map of the general's post.
    Herald(usize),
}

/// A kindled volley: its marks on the floor, the frame they burst on (once planned), whether the
/// bellows burns the foes with it, and its frames since it burst.
#[derive(Clone, Debug, PartialEq)]
pub struct Volley {
    pub id: u32,
    pub marks: Vec<V>,
    pub due: Option<u32>,
    pub planned_at: u32,
    pub turned: bool,
    pub burst: Option<u32>,
}

impl Volley {
    /// The tell's fill, 0 when marked to 1 as it bursts.
    pub fn fill(&self, now: u32) -> f32 {
        match self.due {
            Some(d) => 1.0 - (d.saturating_sub(now) as f32 / KINDLE_FUSE as f32).clamp(0.0, 1.0),
            None => 0.0,
        }
    }
}

/// An order's card on the HUD: its words, colour and when it went up.
#[derive(Clone, Debug, PartialEq)]
pub struct OrderCard {
    pub text: String,
    pub sub: String,
    pub color: u32,
    pub since: f64,
    /// The order it is about (its countdown is read from the general's mind), if any.
    pub order: Option<usize>,
}

/// What the harness reads of a general's realm.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneralTally {
    pub announced: u32,
    pub bitten: u32,
    pub failed: u32,
    pub answered: u32,
    pub broken: u32,
    pub barred: u32,
    pub turned: u32,
    pub whistles: u32,
    pub bursts: u32,
    /// Bursts that met him.
    pub burned: u32,
    pub decrees: u32,
    /// Heralds caught before their muster bit.
    pub heralds: u32,
    pub slain_at: Option<f64>,
}

/// A realm's general at run time.
#[derive(Clone, Debug)]
pub struct GeneralRun {
    pub sin: Sin,
    pub command: Command,
    pub posts: Posts,
    /// His name (a usurper's, if a rival took his place) and his body.
    pub name: String,
    pub body: EnemyKind,
    pub usurper: Option<RivalId>,
    /// His duelist (none if he fell earlier in the run), and the watch.
    pub id: Option<u32>,
    pub patrol: Vec<u32>,
    /// Everything his orders brought into the realm (cleared when the hero falls).
    pub brought: Vec<u32>,
    pub walkers: Vec<Walker>,
    /// The watch: its whistler and the frames left of the whistle, and whether it has whistled
    /// (or been ambushed) this round.
    pub whistle: Option<(u32, u32)>,
    pub alarmed: bool,
    /// The gate glowing for the muster announced, the gates barred, and how long he has stood at it.
    pub gate: Option<usize>,
    pub barred: Vec<bool>,
    pub bar: u32,
    /// The kindling: its zone's middle, its volleys, frames to the next.
    pub zone: V,
    pub volleys: Vec<Volley>,
    pub next_volley: u32,
    pub volley_no: u32,
    /// The decree's carriers that reached the court, and those still on the way.
    pub arrived: u32,
    pub carriers: Vec<u32>,
    /// The instruments his present swing has struck, and the swing's last frame seen.
    struck: [bool; 3],
    swing: Option<(MoveId, u16)>,
    /// His guard called at half his health: the frames to its coming.
    pub rally: Option<u32>,
    pub rallied: bool,
    /// The herald on his way with the muster, and whether a caught herald's map has marked the
    /// general's post.
    pub herald: Option<u32>,
    pub post_known: bool,
    /// Foes dealt for his orders, so far.
    dealt: usize,
    pub cards: Vec<OrderCard>,
    pub tally: GeneralTally,
    /// The threat manager's frame last seen, and now (the brawl's frames: the marks' fuses count them).
    seen: u32,
    pub now: u32,
    /// The realm's ground as laid out (the ways his orders walk are found on it).
    pub layout: crate::logic::grid::Grid,
}

impl GeneralRun {
    /// The general's mind.
    pub fn general(&self) -> &General {
        &self.command.general
    }
    /// Instrument `k`'s place in the world.
    pub fn instrument_at(&self, k: usize) -> V {
        self.command.instruments[k].tile.v()
    }
    /// The gate glowing now, and how far it is barred (0..1).
    pub fn glowing(&self) -> Option<(V, f32)> {
        self.gate.map(|g| (self.posts.gates[g].v(), self.bar as f32 / BAR_FRAMES as f32))
    }
}

fn off() -> bool {
    static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *OFF.get_or_init(|| std::env::var_os("VH_NO_GENERAL").is_some())
}

impl Game {
    /// What the run keeps of the generals.
    pub fn general_records(&self) -> Records {
        Records::decode(&self.warband.save.generals)
    }

    /**
     * The prince's crooked bargain is taken (WARBAND.md §10.3): its unnamed price is the
     * general's decree, done as if his carriers had reached the court; the prince fights by it.
     * Returns what the decree did (the card's words), or None if no general stands here to have it.
     */
    pub(super) fn bargain_completes_the_decree(&mut self) -> Option<&'static str> {
        let g = self.general.as_mut().filter(|g| !g.command.slain && !g.command.decree_done)?;
        g.command.decree_done = true;
        let done = g.command.general.decree_done;
        self.keep_record();
        self.save_warband();
        Some(done)
    }

    fn keep_record(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let mut all = self.general_records();
        all.set(g.sin, g.command.record());
        self.warband.save.generals = all.encode();
    }

    /// The prince of `sin`'s fight, by what his general's realm came to this run.
    pub fn prince_terms(&self, sin: Sin) -> PrinceTerms {
        let r = self.general_records().get(sin);
        general::terms(sin, r.decree_done, r.slain)
    }

    /// Duelist `id`'s index in `self.enemies`.
    fn foe_at(&self, id: u32) -> Option<usize> {
        self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == id))
    }

    fn foe(&self, id: u32) -> Option<&Duelist> {
        self.foe_at(id).and_then(|i| self.enemies[i].duel.as_deref())
    }

    fn foe_mut(&mut self, id: u32) -> Option<&mut Duelist> {
        self.foe_at(id).and_then(|i| self.enemies[i].duel.as_deref_mut())
    }

    /// The next of the realm's foes for his orders, dealt as the realm deals them.
    fn deal(&mut self) -> (EnemyKind, u8) {
        let tier = self.realm_tier();
        let n = self.general.as_ref().map(|g| g.dealt).unwrap_or(0);
        let mut d = Dealer::new(tier);
        let mut out = d.draw();
        for _ in 0..n {
            out = d.draw();
        }
        if let Some(g) = self.general.as_mut() {
            g.dealt += 1;
        }
        out
    }

    /// A foe of `kind` (stage `stage`) raised at `at` for the general: weighed as the realm's own,
    /// asleep in `den` if given, else a puppet (set going by `walk`) or a hunter.
    fn raise(&mut self, kind: EnemyKind, stage: u8, at: P, den: Option<Den>, puppet: bool, health: f64) -> Option<u32> {
        let tier = self.realm_tier();
        let id = self.brawl.as_mut()?.next_id();
        let mut spec = spec_of(&self.level.grid, kind);
        spec.name = format!("{} {}", spec.name, self.enemies.len() + 1);
        let mut e = Enemy::new(&spec, at, self.level.data.enemy_speed);
        e.post = Some(at);
        e.b.mode = EnemyMode::Chase;
        let hp = self.hero.pos();
        let facing = math::norm(V::new(hp.x - at.x as f64 + 0.01, hp.y - at.y as f64));
        let mut d = Duelist::bred(id, Breed::of(kind), at.v(), facing);
        let (wh, wd) = if self.in_vault() {
            (super::brawl::VAULT_HP, 1.0)
        } else {
            crate::logic::realms::weight(if kind.realm_own() { 0 } else { stage }, tier)
        };
        d.max_hp *= wh * health;
        d.hp = d.max_hp;
        d.breed.damage *= wd;
        // (A puppet on its way is asleep to the fight, not hunting him, until he comes near it.)
        d.den = if puppet { den.or(Some(Self::den_at(at, 0.6))) } else { den };
        d.puppet = puppet;
        d.rest = 45;
        d.lore = Lore::default();
        e.duel = Some(Box::new(d));
        self.view.add_enemy(&e);
        self.enemies.push(e);
        Some(id)
    }

    /// A small den about `at` (radius `r` tiles), asleep.
    fn den_at(at: P, r: f64) -> Den {
        Den {
            lo: V::new(at.x as f64 - r, at.y as f64 - r),
            hi: V::new(at.x as f64 + r, at.y as f64 + r),
            home: at.v(),
            awake: false,
            returning: false,
        }
    }

    /**
     * The realm's general takes his post as the level is laid out (the vaults, or one of the six
     * realms): placed by the realm's seed, his instruments standing, his guard asleep about him,
     * the watch asleep at its fire; as the run left him (instruments broken stay broken, a slain
     * general stays slain). The war table learns of him.
     */
    pub(super) fn enter_general(&mut self) {
        self.general = None;
        if off() {
            return;
        }
        let tier = self.realm_tier();
        // (Placed on the realm's ground as it was laid out, before its rules flood or hide any of it.)
        let (sin, arrival, court, rooms, layout) = if let Some(v) = &self.vault {
            (Sin::Greed, v.arrival, P::new(v.exit_gate.x - 1, v.exit_gate.y), general::vault_rooms(v), self.level.grid.clone())
        } else if let Some(r) = &self.run {
            let b = r.base();
            let layout = crate::logic::level::parse_level(&b.level(&self.level.data)).grid;
            (Run::sin(r), b.arrival, b.exit.front(), general::realm_rooms(b), layout)
        } else {
            return;
        };
        // The realm's foes' posts, on its ground as it was laid out (he takes one of them).
        let foes: Vec<P> = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref().and_then(|d| d.den).map(|den| P::new(den.home.x.round() as i32, den.home.y.round() as i32)))
            .collect();
        let gen = General::of(sin);
        let Some(posts) = place(&layout, arrival, court, &rooms, &foes, gen.orders, general::seed_of(sin, tier)) else { return };
        let mut command = Command::new(sin, posts.instruments);
        command.decree_frames = ((posts.court.len() as f64 / CARRIER_PACE) * 60.0) as u32 + 120;
        let record = self.general_records().get(sin);
        command.restore(record);
        // A rival of rank four or more may have taken his place.
        let usurper = if record.slain { None } else { self.ledger().usurper() };
        let (name, body) = match usurper.and_then(|id| self.ledger().get(id)) {
            Some(r) => (r.name.to_uppercase(), r.kind),
            None => (gen.name.to_string(), gen.body),
        };
        self.general = Some(GeneralRun {
            sin,
            command,
            name,
            body,
            usurper,
            id: None,
            patrol: Vec::new(),
            brought: Vec::new(),
            walkers: Vec::new(),
            whistle: None,
            alarmed: false,
            gate: None,
            barred: vec![false; posts.gates.len()],
            bar: 0,
            zone: posts.post.v(),
            volleys: Vec::new(),
            next_volley: 0,
            volley_no: 0,
            arrived: 0,
            carriers: Vec::new(),
            struck: [false; 3],
            swing: None,
            rally: None,
            rallied: false,
            herald: None,
            post_known: false,
            dealt: 0,
            cards: Vec::new(),
            tally: GeneralTally::default(),
            seen: 0,
            now: 0,
            posts,
            layout,
        });
        if !record.slain {
            self.raise_general();
        }
        self.know_general(!record.met);
        self.keep_record();
    }

    /// The general and his guard at his post (asleep in its room, his pack to be), and the watch
    /// at its fire.
    fn raise_general(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let (room, post, body) = (g.posts.room, g.posts.post, g.body);
        let den = Den {
            lo: V::new(room.x0 as f64 - 0.5, room.y0 as f64 - 0.5),
            hi: V::new(room.x1 as f64 + 0.5, room.y1 as f64 + 0.5),
            home: post.v(),
            awake: false,
            returning: false,
        };
        // He takes the post of the room's foe that stood there: its pack is his guard.
        let mut den = den;
        if let Some(i) = self.enemies.iter().position(|e| e.duel.as_deref().and_then(|d| d.den).is_some_and(|d| d.home == post.v())) {
            if let Some(theirs) = self.enemies[i].duel.as_deref().and_then(|d| d.den) {
                den = Den { home: post.v(), ..theirs };
            }
            self.enemies.remove(i);
        }
        let id =
            self.raise(body, crate::logic::realms::deepest_stage(self.realm_tier()), post, Some(den), false, general::general_health(body));
        let mut patrol = Vec::new();
        let watch =
            self.general.as_ref().and_then(|g| g.general().index_of(OrderKind::Watch).filter(|&w| !g.command.instruments[w].broken));
        if let Some(w) = watch {
            let fire = self.general.as_ref().map(|g| g.command.instruments[w].tile).unwrap_or(post);
            for k in 0..PATROL {
                let (kind, stage) = self.deal();
                let at = self.open_near(fire, k as i32 + 1);
                if let Some(pid) = self.raise(kind, stage, at, Some(Self::den_at(at, 2.0)), false, 1.0) {
                    patrol.push(pid);
                }
            }
        }
        // A usurper is a rival of the Ledger still: his lesson, his weakness and his nature go
        // with him into the fight (WARBAND.md §8-§9), and his card when he is met.
        if let (Some(fid), Some(rival)) = (id, self.general.as_ref().and_then(|g| g.usurper)) {
            self.war.grudges.seated.push((fid, rival));
        }
        if let Some(g) = self.general.as_mut() {
            g.id = id;
            g.patrol = patrol;
        }
    }

    /// An open tile about `at`, `k` steps round it.
    fn open_near(&self, at: P, k: i32) -> P {
        let ring = [(1, 0), (0, 1), (-1, 0), (0, -1), (1, 1), (-1, -1), (1, -1), (-1, 1), (2, 0), (0, 2)];
        let g = &self.level.grid;
        ring.iter()
            .cycle()
            .skip(k as usize)
            .take(ring.len())
            .map(|&(dx, dy)| P::new(at.x + dx, at.y + dy))
            .find(|p| g.is_walkable(p.x, p.y, false))
            .unwrap_or(at)
    }

    /// The war table's card for him, and the old man's news of him the first time.
    fn know_general(&mut self, first: bool) {
        let Some(g) = self.general.as_ref() else { return };
        let gen = *g.general();
        let mut known: Vec<String> = Vec::new();
        for k in 0..3 {
            let i = g.command.instruments[k];
            let state = if i.broken {
                "broken"
            } else if i.turned {
                "turned"
            } else {
                "standing"
            };
            known.push(format!("{} by {} ({state})", gen.orders[k].verb().to_lowercase(), title_case(gen.instruments[k])));
        }
        let note = if g.command.slain {
            format!(
                "Slain. His banner hangs on your wall. {}",
                if crate::logic::court::built(gen.sin) {
                    format!("{} fights without his {}.", gen.sin.prince(), crate::logic::court::adds(gen.sin))
                } else {
                    "His prince is weaker for it.".to_string()
                }
            )
        } else {
            format!("He {}.", known.join("; he "))
        };
        let name = g.name.clone();
        let sin = g.sin;
        self.workshop.generals[sin.index()] = Some(GeneralCard { sin, name: title_case(&name), note });
        if first {
            let line = format!(
                "{} holds {} for {}. He {} by {}: break it, and he cannot.",
                title_case(&name),
                sin.realm(),
                sin.prince(),
                gen.orders[0].verb().to_lowercase(),
                title_case(gen.instruments[0]).to_lowercase()
            );
            self.workshop.news.push(line);
        }
    }

    /// The hero fell and the realm begins again: what the orders brought is gone, the order under
    /// way is called off, the kindled floor is cold; the general and his guard are home (as the
    /// dens are); broken instruments stay broken.
    pub(super) fn reset_general(&mut self) {
        let Some(g) = self.general.as_mut() else { return };
        let gone: Vec<u32> = g.brought.drain(..).chain(g.carriers.drain(..)).collect();
        g.walkers.clear();
        g.whistle = None;
        g.alarmed = false;
        g.gate = None;
        g.bar = 0;
        g.volleys.clear();
        g.rally = None;
        g.cards.clear();
        g.seen = 0;
        if let Some(k) = g.command.running {
            g.command.fail(k);
        }
        g.command.answer = None;
        g.command.wait = general::FIRST_ORDER;
        let patrol = g.patrol.clone();
        self.enemies.retain(|e| !e.duel.as_ref().is_some_and(|d| gone.contains(&d.id)));
        // The watch back asleep at its fire.
        for id in patrol {
            if let Some(d) = self.foe_mut(id) {
                d.puppet = false;
                if let Some(den) = d.den.as_mut() {
                    d.pos = den.home;
                    den.awake = false;
                } else {
                    let home = d.pos;
                    d.den = Some(Self::den_at(P::new(home.x.round() as i32, home.y.round() as i32), 2.0));
                }
            }
        }
        // (A realm that raises its packs afresh after a fall, as the Burning Field does, raises
        // them without him: he takes his post again.)
        let lost = self.general.as_ref().is_some_and(|g| !g.command.slain && g.id.is_some_and(|id| self.foe(id).is_none()));
        if lost {
            self.raise_general();
        }
        self.refresh_foe_views();
    }

    /// One frame of the general's realm (after the brawl's).
    pub(super) fn update_general(&mut self, dt: f64) {
        if self.general.is_none() || !self.brawling() {
            return;
        }
        let now = self.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
        let frames = {
            let g = self.general.as_mut().unwrap();
            let n = now.saturating_sub(g.seen);
            g.seen = now;
            g.now = now;
            if n > 4 {
                1
            } else {
                n
            }
        };
        self.watch_the_general();
        for _ in 0..frames {
            let build = self.drum_bounds().orders;
            let room = self.room_for_orders();
            let cues = self.general.as_mut().map(|g| g.command.step_with(build, room)).unwrap_or_default();
            for c in cues {
                self.general_cue(c);
            }
            self.general_frame(now);
            if !self.hero.alive {
                return;
            }
        }
        self.strike_instruments();
        self.turn_bellows();
        self.walk_puppets(dt);
        let t = self.time;
        if let Some(g) = self.general.as_mut() {
            g.cards.retain(|c| t - c.since < CARD_SECONDS);
        }
    }

    /// Which of his orders the field has room for: a muster only while what hunts him and what it
    /// would bring stay inside the fight's bounds (a pack at a time), a watch only with a watch.
    fn room_for_orders(&self) -> [bool; 3] {
        let Some(g) = self.general.as_ref() else { return [false; 3] };
        let hunting = self.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.engaged() && !d.puppet).count();
        let brought = g.brought.iter().filter(|&&id| self.foe(id).is_some_and(|d| d.alive())).count();
        std::array::from_fn(|k| match g.general().orders[k] {
            OrderKind::Muster => hunting + MUSTER + g.command.muster_extra <= MUSTER_ROOM && brought < MUSTER_ROOM,
            OrderKind::Watch => g.patrol.iter().any(|&id| self.foe(id).is_some_and(|d| d.alive() && d.den.is_some_and(|den| !den.awake))),
            _ => true,
        })
    }

    /// His death, his half-health horn, the watch's ambush, the carriers' fate.
    fn watch_the_general(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        // (Slain this frame, as the war log has it: the vaults sweep the fallen away at once.)
        let fell = g.id.and_then(|id| {
            self.war.log.iter().find_map(|e| match *e {
                WarEvent::Slain { foe, at, .. } if foe == id => Some(at),
                _ => None,
            })
        });
        if let Some(at) = fell.filter(|_| !g.command.slain) {
            self.general_slain(at);
            return;
        }
        let Some(g) = self.general.as_ref() else { return };
        if let Some(id) = g.id.filter(|_| !g.command.slain) {
            match self.foe(id).map(|d| (d.alive(), d.hp / d.max_hp.max(1.0), d.pos)) {
                Some((true, share, _)) if share <= RALLY_AT && !g.rallied => {
                    let name = g.name.clone();
                    let gen = g.sin;
                    if let Some(g) = self.general.as_mut() {
                        g.rallied = true;
                        g.rally = Some(RALLY_LEAD);
                    }
                    let at = self.foe(id).map(|d| d.pos).unwrap_or_default();
                    self.horn(at, gen, false);
                    self.order_card(
                        format!("{name} SOUNDS HIS HORN - 1"),
                        "HIS GUARD STANDS FAST: STRIKE NOW OR WAIT IT OUT".into(),
                        0xff8a3a,
                        None,
                    );
                }
                Some((true, ..)) => {}
                Some((false, _, at)) => self.general_slain(at),
                // (Gone from the field without falling: a realm raised its packs afresh. He takes his
                // post again when the hero next rises.)
                None => {}
            }
        }
        // The decree's carriers: all cut down before any reached the court, the decree fails.
        let Some(g) = self.general.as_ref() else { return };
        if let Some(k) = g.general().index_of(OrderKind::Decree) {
            if matches!(g.command.stages[k], OrderStage::Live { .. }) && !g.carriers.is_empty() {
                let alive = g.carriers.iter().filter(|&&id| self.foe(id).is_some_and(|d| d.alive())).count();
                let arrived = g.arrived;
                if alive == 0 {
                    let cue = self.general.as_mut().and_then(|g| {
                        g.carriers.clear();
                        if arrived > 0 {
                            g.command.finish(k)
                        } else {
                            g.tally.failed += 1;
                            g.command.fail(k)
                        }
                    });
                    if let Some(c) = cue {
                        self.general_cue(c);
                    }
                }
            }
        }
    }

    /// What the general's mind says, done in the world.
    fn general_cue(&mut self, cue: Cue) {
        let Some(g) = self.general.as_ref() else { return };
        let gen = *g.general();
        let name = g.name.clone();
        let sin = g.sin;
        match cue {
            Cue::Announce { order, lead } => {
                let kind = gen.orders[order];
                let hp = self.hero.pos();
                let mut sub = format!("{} - {}", title_case(gen.instruments[order]).to_uppercase(), cue_line(&gen, kind));
                match kind {
                    OrderKind::Muster => {
                        // The gate nearest him (not on top of him) glows.
                        let gate = self.general.as_ref().and_then(|g| {
                            g.posts
                                .gates
                                .iter()
                                .enumerate()
                                .filter(|(i, _)| !g.barred[*i])
                                .map(|(i, p)| (i, dist_sq(p.v(), hp)))
                                .filter(|&(_, d)| d >= 8.0 * 8.0)
                                .min_by(|a, b| a.1.total_cmp(&b.1))
                                .or_else(|| {
                                    g.posts
                                        .gates
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| !g.barred[*i])
                                        .map(|(i, p)| (i, dist_sq(p.v(), hp)))
                                        .next()
                                })
                                .map(|(i, _)| i)
                        });
                        match gate {
                            Some(i) => {
                                if let Some(g) = self.general.as_mut() {
                                    g.gate = Some(i);
                                    g.bar = 0;
                                }
                                sub = "A GATE GLOWS - BAR IT, OR CATCH HIS HERALD".into();
                                self.send_herald(order, i, lead);
                            }
                            None => {
                                // Every gate barred: the horn sounds for nothing.
                                let cue = self.general.as_mut().and_then(|g| g.command.fail(order));
                                if let Some(c) = cue {
                                    self.general_cue(c);
                                }
                                return;
                            }
                        }
                    }
                    OrderKind::Kindle => {
                        if let Some(g) = self.general.as_mut() {
                            g.zone = hp;
                        }
                    }
                    _ => {}
                }
                let at = self.general.as_ref().map(|g| g.instrument_at(order)).unwrap_or(hp);
                self.horn(at, sin, kind == OrderKind::Muster);
                self.order_card(gen.card(kind).replace(gen.name, &name), sub, gen.cloak, Some(order));
                self.war_note(WarEvent::Order { general: sin, order: kind, stage: OrderStage::Announced { left: lead } });
                if let Some(g) = self.general.as_mut() {
                    g.tally.announced += 1;
                }
            }
            Cue::Bite { order } => {
                let kind = gen.orders[order];
                if let Some(g) = self.general.as_mut() {
                    g.tally.bitten += 1;
                }
                self.war_note(WarEvent::Order { general: sin, order: kind, stage: OrderStage::Live { left: 0 } });
                match kind {
                    OrderKind::Watch => self.set_the_watch(),
                    OrderKind::Muster => {
                        self.herald_arrives();
                        self.muster(order);
                    }
                    OrderKind::Kindle => {
                        if let Some(g) = self.general.as_mut() {
                            g.next_volley = 0;
                        }
                    }
                    OrderKind::Decree => self.send_carriers(),
                }
            }
            Cue::Over { order, stage } => {
                let kind = gen.orders[order];
                self.war_note(WarEvent::Order { general: sin, order: kind, stage });
                if let Some(g) = self.general.as_mut() {
                    if kind == OrderKind::Muster {
                        g.gate = None;
                        g.bar = 0;
                    }
                }
                if kind == OrderKind::Muster {
                    // However it ended, the herald's errand is over.
                    self.herald_arrives();
                }
                match (kind, stage) {
                    (OrderKind::Decree, OrderStage::Done) => {
                        // The rest of the carriers go into the court with what they carry.
                        let carriers = self.general.as_mut().map(|g| std::mem::take(&mut g.carriers)).unwrap_or_default();
                        for id in carriers {
                            if let Some(i) = self.foe_at(id) {
                                self.enemies[i].b.mode = EnemyMode::Eaten;
                            }
                        }
                        if let Some(g) = self.general.as_mut() {
                            g.tally.decrees += 1;
                            g.walkers.retain(|w| w.then != Then::Court);
                        }
                        self.juice.banner = Some(Banner {
                            text: format!("{} IS DONE", gen.decree),
                            sub: gen.decree_done.to_string(),
                            color: gen.cloak,
                            life: 3.0,
                            max: 3.0,
                        });
                        self.audio.play(Sfx::Toll);
                        self.keep_record();
                    }
                    (OrderKind::Decree, _) => self.drop_the_decree(),
                    // (The watch's round walked, or its fire broken: it lies down where it stands.)
                    (OrderKind::Watch, _) => self.stand_down_the_watch(),
                    _ => {}
                }
            }
            Cue::Answer { counter, lead: _ } => {
                let line = match counter {
                    Counter::GuardTo(j) => format!("GUARDS TO {}", gen.instruments[j]),
                    Counter::DoubleMuster => "THE MUSTER IS DOUBLED".to_string(),
                };
                self.order_card(format!("{name} ANSWERS"), line, 0xff6a3a, None);
                self.audio.play(Sfx::Stinger);
            }
            Cue::Answered { counter } => {
                if let Some(g) = self.general.as_mut() {
                    g.tally.answered += 1;
                }
                if let Counter::GuardTo(j) = counter {
                    self.send_guards(j);
                }
            }
        }
    }

    /// A horn (or the counting-bell) from `at`, on the music's bar line if it is near.
    fn horn(&mut self, at: V, sin: Sin, muster: bool) {
        let gen = General::of(sin);
        if muster && gen.bell() {
            self.audio.play_at(Sfx::Bells, at);
        } else {
            let listener = self.audio.listener();
            self.audio.tell(gen.body, MoveId::Rally, at, listener);
        }
        if muster {
            self.view.trauma(Trauma::Horn);
        }
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.6, gen.cloak, 18);
    }

    /// An order's card goes up under the banner.
    fn order_card(&mut self, text: String, sub: String, color: u32, order: Option<usize>) {
        let since = self.time;
        if let Some(g) = self.general.as_mut() {
            g.cards.retain(|c| c.order.is_none() || c.order != order);
            g.cards.push(OrderCard { text, sub, color, since, order });
            if g.cards.len() > 3 {
                g.cards.remove(0);
            }
        }
    }

    /// The watch rises from its fire and walks its round.
    fn set_the_watch(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let round = g.posts.round.clone();
        let patrol: Vec<u32> = g.patrol.iter().copied().filter(|&id| self.foe(id).is_some_and(|d| d.alive())).collect();
        if round.is_empty() || patrol.is_empty() {
            return;
        }
        for (k, id) in patrol.iter().enumerate() {
            let hp = self.foe(*id).map(|d| d.hp).unwrap_or(0.0);
            // (It takes up the round where it lies nearest, one behind another.)
            let here = self.foe(*id).map(|d| d.pos).unwrap_or_default();
            let start = round
                .iter()
                .enumerate()
                .min_by(|a, b| dist_sq(a.1.v(), here).total_cmp(&dist_sq(b.1.v(), here)))
                .map(|(i, _)| i)
                .unwrap_or(0);
            if let Some(d) = self.foe_mut(*id) {
                if d.den.is_some_and(|den| den.awake) {
                    continue; // already hunting him
                }
                let at = P::new(d.pos.x.round() as i32, d.pos.y.round() as i32);
                d.den = Some(Self::den_at(at, 0.6));
                d.puppet = true;
                d.token = false;
            }
            if let Some(g) = self.general.as_mut() {
                g.walkers.retain(|w| w.id != *id);
                g.walkers.push(Walker {
                    id: *id,
                    path: round.clone(),
                    at: start as f64 - k as f64 * 0.01,
                    pace: 1.4,
                    then: Then::Round,
                    hp,
                });
            }
        }
        if let Some(g) = self.general.as_mut() {
            g.alarmed = false;
            g.whistle = None;
        }
    }

    /// The watch is called off (its fire broken, or its round walked): each lies down where it stands.
    fn stand_down_the_watch(&mut self) {
        let Some(g) = self.general.as_mut() else { return };
        let ids: Vec<u32> = g.walkers.iter().filter(|w| w.then == Then::Round).map(|w| w.id).collect();
        g.walkers.retain(|w| w.then != Then::Round);
        g.whistle = None;
        for id in ids {
            if let Some(d) = self.foe_mut(id) {
                d.puppet = false;
                let at = P::new(d.pos.x.round() as i32, d.pos.y.round() as i32);
                d.den = Some(Self::den_at(at, 2.0));
            }
        }
    }

    /// The muster's herald sets out from the horn for gate `gate`, to reach it as the muster bites
    /// (`lead` frames): quick, slight, and running on when struck.
    fn send_herald(&mut self, order: usize, gate: usize, lead: u32) {
        let Some(g) = self.general.as_ref() else { return };
        let from = g.command.instruments[order].tile;
        let to = g.posts.gates[gate];
        let Some(way) = general::walk(&self.level.grid, from, to).or_else(|| general::walk(&g.layout, from, to)) else { return };
        let at = self.open_near(from, 1);
        let Some(id) = self.raise(EnemyKind::Minion, 1, at, None, true, 1.0) else { return };
        let hp = self.foe(id).map(|d| d.hp).unwrap_or(0.0);
        let pace = ((way.len() as f64) / (lead as f64 / 60.0 * 0.9)).max(HERALD_PACE);
        if let Some(i) = self.foe_at(id) {
            self.enemies[i].b.name = "THE HERALD".into();
        }
        if let Some(g) = self.general.as_mut() {
            g.herald = Some(id);
            g.walkers.push(Walker { id, path: way, at: 0.0, pace, then: Then::Herald(order), hp });
        }
        let p = at.v();
        self.floater(p.x, p.y - 1.4, "THE HERALD RUNS WITH THE ORDER", 1.6, Some(0xffc08a));
    }

    /// The herald is cut down before the muster bit: it fails, and the map he carried marks his
    /// general's post (a ping on the screen, a line in the old man's news).
    fn herald_caught(&mut self, order: usize, id: u32) {
        let at = self.foe(id).map(|d| d.pos).unwrap_or(self.hero.pos());
        let (cue, name) = match self.general.as_mut() {
            Some(g) => {
                g.herald = None;
                g.post_known = true;
                g.tally.heralds += 1;
                g.walkers.retain(|w| w.id != id);
                (g.command.fail(order), g.name.clone())
            }
            None => return,
        };
        if let Some(c) = cue {
            self.general_cue(c);
        }
        self.floater(at.x, at.y - 1.6, "THE HERALD FALLS: THE MUSTER FAILS", 2.0, Some(0x9ad1ff));
        self.floater(at.x, at.y - 1.0, format!("HIS MAP MARKS {name}'S POST"), 2.0, Some(0xffc040));
        self.workshop.news.push(format!("You caught {}'s herald. His map shows where the general keeps his post.", title_case(&name)));
        self.audio.play_at(Sfx::Clang, at);
    }

    /// The herald reached his gate (or the muster bit): his errand is done, and he is gone.
    fn herald_arrives(&mut self) {
        let Some(id) = self.general.as_mut().and_then(|g| g.herald.take()) else { return };
        if let Some(g) = self.general.as_mut() {
            g.walkers.retain(|w| w.id != id);
        }
        if let Some(i) = self.foe_at(id).filter(|&i| self.enemies[i].duel.as_ref().is_some_and(|d| d.alive())) {
            let p = self.enemies[i].pos();
            self.enemies[i].b.mode = EnemyMode::Eaten;
            self.view.dust(&self.level.grid, p.x, p.y, 0xd8c8a0, 10, 1.2);
        }
    }

    /// The muster bites: unless its gate was barred, a pack marches in through it.
    fn muster(&mut self, order: usize) {
        let Some(g) = self.general.as_ref() else { return };
        let Some(gi) = g.gate else { return };
        let gate = g.posts.gates[gi];
        let n = MUSTER + g.command.muster_extra;
        let hp = self.hero.pos();
        let goal = P::new(hp.x.round() as i32, hp.y.round() as i32);
        let way =
            general::walk(&self.level.grid, gate, goal).or_else(|| general::walk(&g.layout, gate, goal)).unwrap_or_else(|| vec![gate]);
        for k in 0..n {
            let (kind, stage) = self.deal();
            let at = self.open_near(gate, k as i32);
            let Some(id) = self.raise(kind, stage, at, None, true, 1.0) else { continue };
            let hp = self.foe(id).map(|d| d.hp).unwrap_or(0.0);
            if let Some(g) = self.general.as_mut() {
                g.brought.push(id);
                g.walkers.push(Walker { id, path: way.clone(), at: -(k as f64) * 1.2, pace: 2.4, then: Then::Hunt, hp });
            }
        }
        let g = gate.v();
        self.view.dust(&self.level.grid, g.x, g.y, 0x8a7a68, 30, 2.2);
        self.floater(g.x, g.y - 1.5, "THEY MARCH IN", 1.6, Some(0xff8a3a));
        self.audio.play_at(Sfx::DoorGrind, g);
        if let Some(r) = self.general.as_mut() {
            r.gate = None;
            r.bar = 0;
        }
        let _ = order;
    }

    /// The decree: carriers set out from his post for the court's portal.
    fn send_carriers(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let way = g.posts.court.clone();
        let post = g.posts.post;
        for k in 0..3 {
            let at = self.open_near(post, k + 3);
            let (kind, stage) = self.deal();
            let Some(id) = self.raise(kind, stage, at, None, true, 1.0) else { continue };
            let hp = self.foe(id).map(|d| d.hp).unwrap_or(0.0);
            if let Some(g) = self.general.as_mut() {
                g.carriers.push(id);
                g.walkers.push(Walker { id, path: way.clone(), at: -(k as f64) * 1.5, pace: CARRIER_PACE, then: Then::Court, hp });
            }
        }
        if let Some(g) = self.general.as_mut() {
            g.arrived = 0;
        }
        let p = post.v();
        self.floater(p.x, p.y - 1.6, "THE CARRIERS SET OUT FOR THE COURT", 2.0, Some(0xf2c53d));
        self.audio.play_at(Sfx::Chain, p);
    }

    /// The decree falls short (the chain cut, the general slain): the carriers drop their loads and flee.
    fn drop_the_decree(&mut self) {
        let carriers = self.general.as_mut().map(|g| std::mem::take(&mut g.carriers)).unwrap_or_default();
        if let Some(g) = self.general.as_mut() {
            g.walkers.retain(|w| w.then != Then::Court);
        }
        for id in carriers {
            if let Some(i) = self.foe_at(id).filter(|&i| self.enemies[i].duel.as_ref().is_some_and(|d| d.alive())) {
                let p = self.enemies[i].pos();
                self.enemies[i].b.mode = EnemyMode::Eaten;
                self.view.dust(&self.level.grid, p.x, p.y, 0xd8c8a0, 12, 1.4);
                self.floater(p.x, p.y - 1.0, "THEY DROP THEIR LOADS AND FLEE", 1.2, Some(0xd8c8a0));
            }
        }
    }

    /// A counter-order: two of the realm's foes go from his post to instrument `j`, and hold it.
    fn send_guards(&mut self, j: usize) {
        let Some(g) = self.general.as_ref() else { return };
        let to = g.command.instruments[j].tile;
        let from = g.posts.post;
        let Some(way) = general::walk(&g.layout, from, to) else { return };
        for k in 0..2 {
            let (kind, stage) = self.deal();
            let at = self.open_near(from, k + 5);
            let Some(id) = self.raise(kind, stage, at, None, true, 1.0) else { continue };
            let hp = self.foe(id).map(|d| d.hp).unwrap_or(0.0);
            let hold = self.open_near(to, k + 1);
            if let Some(g) = self.general.as_mut() {
                g.brought.push(id);
                g.walkers.push(Walker { id, path: way.clone(), at: -(k as f64) * 1.4, pace: 2.6, then: Then::Hold(hold), hp });
            }
        }
    }

    /// One frame of the orders under way: the gate being barred, the whistle, the kindled floor,
    /// his guard's coming.
    fn general_frame(&mut self, now: u32) {
        let hp = self.hero.pos();
        // The gate: he stands at it, and it is barred.
        if let Some((gate, _)) = self.general.as_ref().and_then(|g| g.glowing()) {
            let at = dist_sq(gate, hp) <= BAR_REACH * BAR_REACH;
            let barred = self.general.as_mut().map(|g| {
                g.bar = if at { g.bar + 1 } else { g.bar.saturating_sub(2) };
                g.bar >= BAR_FRAMES
            });
            if barred == Some(true) {
                let cue = self.general.as_mut().and_then(|g| {
                    if let Some(i) = g.gate.take() {
                        g.barred[i] = true;
                    }
                    g.bar = 0;
                    g.tally.barred += 1;
                    let k = g.general().index_of(OrderKind::Muster)?;
                    g.command.fail(k)
                });
                self.floater(gate.x, gate.y - 1.4, "THE GATE IS BARRED", 1.6, Some(0x9ad1ff));
                self.audio.play_at(Sfx::DoorGrind, gate);
                self.view.dust(&self.level.grid, gate.x, gate.y, 0x9a8a78, 20, 1.6);
                if let Some(c) = cue {
                    self.general_cue(c);
                }
            }
        }
        self.update_whistle();
        self.update_kindling(now);
        // At half his health his horn steadies his guard: their poise whole again.
        let due = self.general.as_mut().and_then(|g| {
            let r = g.rally.as_mut()?;
            *r = r.saturating_sub(1);
            (*r == 0).then(|| {
                g.rally = None;
                g.id
            })
        });
        if let Some(Some(id)) = due {
            if let Some(at) = self.foe(id).filter(|d| d.alive()).map(|d| d.pos) {
                let pack = self.general_pack();
                for d in self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()) {
                    if pack.contains(&d.id) && d.alive() && dist_sq(d.pos, at) <= 36.0 && d.f.reeling().is_none() {
                        d.f.poise = d.f.max_poise;
                    }
                }
                let color = self.general.as_ref().map(|g| g.general().cloak).unwrap_or(0xffc040);
                self.floater(at.x, at.y - 1.8, "HIS GUARD STANDS FAST", 1.4, Some(color));
                self.view.sparkle(&self.level.grid, at.x, at.y, 1.4, color, 24);
            }
        }
    }

    /// The watch: a walker that sees him stops and whistles; a second later the dens near wake,
    /// unless the whistler has fallen. A walker struck on its round turns on him (an ambush: its
    /// leader drops runes when it falls before the whistle).
    fn update_whistle(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let hp = self.hero.pos();
        if let Some((whistler, left)) = g.whistle {
            let alive = self.foe(whistler).is_some_and(|d| d.alive());
            if !alive {
                if let Some(g) = self.general.as_mut() {
                    g.whistle = None;
                }
                self.floater(hp.x, hp.y - 1.6, "THE WHISTLE DIES WITH HIM", 1.4, Some(0x9ad1ff));
                self.loose_the_watch();
                return;
            }
            if left > 1 {
                if let Some(g) = self.general.as_mut() {
                    g.whistle = Some((whistler, left - 1));
                }
                return;
            }
            // The whistle: every den within its reach wakes.
            let at = self.foe(whistler).map(|d| d.pos).unwrap_or(hp);
            for e in self.enemies.iter_mut() {
                if let Some(d) = e.duel.as_deref_mut() {
                    if let Some(den) = d.den.as_mut().filter(|den| !den.awake && dist_sq(den.home, at) <= WHISTLE_WAKES * WHISTLE_WAKES) {
                        den.awake = true;
                    }
                }
            }
            if let Some(g) = self.general.as_mut() {
                g.whistle = None;
                g.tally.whistles += 1;
            }
            self.floater(at.x, at.y - 1.6, "THE WATCH CALLS THE DENS", 1.6, Some(0xff8a3a));
            self.audio.play_at(Sfx::Shriek, at);
            self.loose_the_watch();
            return;
        }
        if g.alarmed {
            return;
        }
        let walking: Vec<u32> = g.walkers.iter().filter(|w| w.then == Then::Round && w.at >= 0.0).map(|w| w.id).collect();
        let seen = walking
            .iter()
            .copied()
            .find(|&id| self.foe(id).is_some_and(|d| d.alive() && dist_sq(d.pos, hp) <= PATROL_SIGHT * PATROL_SIGHT));
        if let Some(id) = seen {
            let at = self.foe(id).map(|d| d.pos).unwrap_or(hp);
            if let Some(g) = self.general.as_mut() {
                g.alarmed = true;
                g.whistle = Some((id, WHISTLE));
                g.walkers.retain(|w| w.then != Then::Round);
            }
            self.floater(at.x, at.y - 1.6, "! THE WATCH SEES YOU: STRIKE THE WHISTLER", 1.2, Some(0xffd040));
            self.audio.play_at(Sfx::Shriek, at);
            let name = self.general.as_ref().map(|g| g.name.clone()).unwrap_or_default();
            self.order_card(format!("{name}'S WATCH WHISTLES - 1"), "KILL THE WHISTLER BEFORE THE DENS WAKE".into(), 0xffd040, None);
        }
    }

    /// The watch has seen him (or been struck): it hunts him.
    fn loose_the_watch(&mut self) {
        let Some(g) = self.general.as_mut() else { return };
        g.alarmed = true;
        g.walkers.retain(|w| w.then != Then::Round);
        let patrol = g.patrol.clone();
        for id in patrol {
            if let Some(d) = self.foe_mut(id).filter(|d| d.alive()) {
                d.puppet = false;
                d.den = None;
            }
        }
    }

    /// The kindling: each volley is marked on the floor about him once the threat manager has
    /// planned its burst clear of every other strike, then bursts three seconds later: one strike
    /// on him if he stands on a mark (and is not in the air), or, with the bellows turned, on the
    /// foes standing on them.
    fn update_kindling(&mut self, now: u32) {
        let Some(g) = self.general.as_ref() else { return };
        let k = g.general().index_of(OrderKind::Kindle);
        let live = k.is_some_and(|k| matches!(g.command.stages[k], OrderStage::Live { left } if left > KINDLE_FUSE / 2));
        let turned = k.is_some_and(|k| g.command.instruments[k].turned);
        // A new volley, every so often while the kindling is live.
        if live {
            let due = self.general.as_mut().map(|g| {
                g.next_volley = g.next_volley.saturating_sub(1);
                g.next_volley == 0
            });
            if due == Some(true) && !self.general.as_ref().is_some_and(|g| g.volleys.iter().any(|v| v.due.is_none())) {
                let marks = self.kindle_marks();
                if let Some(g) = self.general.as_mut() {
                    g.volley_no += 1;
                    let id = HAZARD_IDS + g.volley_no;
                    g.volleys.push(Volley { id, marks, due: None, planned_at: now, turned, burst: None });
                    g.next_volley = KINDLE_EVERY;
                }
            }
        }
        // Plans: through the threat manager (a turned volley hurts only the foes, and needs none).
        let unplanned: Vec<(u32, bool)> = self
            .general
            .as_ref()
            .map(|g| g.volleys.iter().filter(|v| v.due.is_none()).map(|v| (v.id, v.turned)).collect())
            .unwrap_or_default();
        for (id, turned) in unplanned {
            let planned = turned || self.plan_hazard(id, KINDLE_FUSE);
            if planned {
                if let Some(v) = self.general.as_mut().and_then(|g| g.volleys.iter_mut().find(|v| v.id == id)) {
                    v.due = Some(now + KINDLE_FUSE);
                    v.planned_at = now;
                }
                let at = self.general.as_ref().and_then(|g| g.volleys.iter().find(|v| v.id == id)).and_then(|v| v.marks.first().copied());
                if let Some(at) = at {
                    self.audio.play_at(Sfx::Fuse, at);
                }
            }
        }
        // Bursts.
        let due: Vec<Volley> = self
            .general
            .as_ref()
            .map(|g| g.volleys.iter().filter(|v| v.burst.is_none() && v.due.is_some_and(|d| now >= d)).cloned().collect())
            .unwrap_or_default();
        let color = self.general.as_ref().map(|g| g.general().hazard_color).unwrap_or(0xffd860);
        for v in due {
            if let Some(w) = self.general.as_mut().and_then(|g| g.volleys.iter_mut().find(|w| w.id == v.id)) {
                w.burst = Some(0);
            }
            if let Some(g) = self.general.as_mut() {
                g.tally.bursts += 1;
            }
            for m in &v.marks {
                self.view.sparkle(&self.level.grid, m.x, m.y, 0.8, color, 12);
            }
            self.audio.play_at(Sfx::Clang, v.marks.first().copied().unwrap_or(self.hero.pos()));
            if let Some(b) = self.brawl.as_mut() {
                b.threat.release(v.id);
            }
            if v.turned {
                self.burn_foes(&v.marks);
                continue;
            }
            let hp = self.hero.pos();
            let reach = KINDLE_RADIUS + HERO_RADIUS;
            let on = v.marks.iter().copied().find(|m| dist_sq(*m, hp) <= reach * reach);
            if let (Some(m), false) = (on, self.hero.airborne()) {
                if let Some(g) = self.general.as_mut() {
                    g.tally.burned += 1;
                }
                let damage = BURST_DAMAGE * self.brawl.as_ref().map(|b| b.foe_damage).unwrap_or(1.0);
                self.general_hazard_blow(m, damage, v.id);
                if !self.hero.alive {
                    return;
                }
            }
        }
        if let Some(g) = self.general.as_mut() {
            for v in g.volleys.iter_mut() {
                if let Some(b) = v.burst.as_mut() {
                    *b += 1;
                }
            }
            g.volleys.retain(|v| v.burst.is_none_or(|b| b < 30));
        }
    }

    /// A volley's marks: about the zone's middle, one where he stands if he is in it, on open floor.
    fn kindle_marks(&mut self) -> Vec<V> {
        let Some(zone) = self.general.as_ref().map(|g| g.zone) else { return Vec::new() };
        let hp = self.hero.pos();
        let mut marks: Vec<V> = Vec::new();
        if dist_sq(hp, zone) <= KINDLE_ZONE * KINDLE_ZONE {
            marks.push(V::new(hp.x.round(), hp.y.round()));
        }
        let mut tries = 0;
        while marks.len() < KINDLE_MARKS && tries < 40 {
            tries += 1;
            let a = self.rng.next() * std::f64::consts::TAU;
            let r = KINDLE_ZONE * self.rng.next().sqrt();
            let p = V::new((zone.x + a.cos() * r).round(), (zone.y + a.sin() * r).round());
            let open = self.level.grid.is_walkable(p.x as i32, p.y as i32, false);
            if open && marks.iter().all(|m| dist_sq(*m, p) >= 2.4 * 2.4) {
                marks.push(p);
            }
        }
        marks
    }

    /// A turned volley burns the foes standing on its marks.
    fn burn_foes(&mut self, marks: &[V]) {
        let reach = KINDLE_RADIUS + 0.4;
        let hit: Vec<usize> = (0..self.enemies.len())
            .filter(|&i| {
                self.enemies[i].duel.as_deref().is_some_and(|d| d.alive() && marks.iter().any(|m| dist_sq(*m, d.pos) <= reach * reach))
            })
            .collect();
        let hp = self.hero.pos();
        for i in hit {
            self.duel_hit(i, Hit { damage: BURST_DAMAGE * 1.5, poise: BURST_POISE, hitstun: 14, ..Default::default() }, 3, hp);
            let p = self.enemies[i].pos();
            self.floater(p.x, p.y - 1.0, "BURNED BY HIS OWN", 0.9, Some(0xffa040));
        }
    }

    /// A hazard of the general's asks the threat manager for a strike landing `lands` frames from
    /// now, clear of every other planned strike (a token of no cost, taken and committed at once).
    fn plan_hazard(&mut self, id: u32, lands: u32) -> bool {
        let Some(b) = self.brawl.as_mut() else { return false };
        let ask = Request { id, cost: 0, eta: lands, dist: 0.0, front: 1.0, wait: 0, bias: 10.0 };
        if b.threat.grant(&[ask]) != Some(id) {
            return false;
        }
        b.threat.commit(id, lands as u16);
        true
    }

    /// His blows on the instruments: on his swing's active frames, each one in its cone that this
    /// swing has not struck takes a blow; three break it.
    fn strike_instruments(&mut self) {
        let Some(b) = self.brawl.as_ref() else { return };
        let attack = b.hero.attack();
        let aim = b.aim;
        let hp = self.hero.pos();
        let Some(g) = self.general.as_mut() else { return };
        // A new swing (or none) clears what the last one struck.
        let fresh = match (attack, g.swing) {
            (Some((mv, _, f)), Some((last, lf))) => mv != last || f < lf,
            (Some(_), None) => true,
            (None, _) => {
                g.swing = None;
                return;
            }
        };
        let Some((mv, phase, frame)) = attack else { return };
        if fresh {
            g.struck = [false; 3];
        }
        g.swing = Some((mv, frame));
        if phase != Phase::Active {
            return;
        }
        let d = b.hero.data(mv);
        if d.projectile {
            return;
        }
        let targets: Vec<usize> = (0..3)
            .filter(|&k| {
                let i = g.command.instruments[k];
                !i.broken && !g.struck[k] && math::in_cone(hp, aim, i.tile.v(), 0.45, d.reach, d.half_angle)
            })
            .collect();
        for k in targets {
            if let Some(g) = self.general.as_mut() {
                g.struck[k] = true;
            }
            self.strike_instrument(k);
        }
    }

    /// One blow on instrument `k` (his swing, or the F11 cheat).
    pub(super) fn strike_instrument(&mut self, k: usize) {
        let Some(g) = self.general.as_mut() else { return };
        let cues = g.command.strike(k);
        let i = g.command.instruments[k];
        let sin = g.sin;
        let gen = *g.general();
        let at = i.tile.v();
        self.war_note(WarEvent::Instrument { general: sin, id: i.id, broken: i.broken });
        self.view.sparkle(&self.level.grid, at.x, at.y, 0.9, 0xffc060, 10);
        if i.broken {
            if let Some(g) = self.general.as_mut() {
                g.tally.broken += 1;
            }
            self.audio.play_at(Sfx::ShieldBreak, at);
            self.view.dust(&self.level.grid, at.x, at.y, 0x6a5040, 26, 2.0);
            self.view.shake(0.18);
            self.floater(at.x, at.y - 1.4, format!("{} IS BROKEN", gen.instruments[k]), 1.8, Some(0xffc85a));
            self.keep_record();
            self.know_general(false);
        } else {
            self.audio.play_at(Sfx::Anvil, at);
            self.floater(at.x, at.y - 1.1, format!("{} MORE", i.blows), 0.7, Some(0xffc85a));
        }
        for c in cues {
            self.general_cue(c);
        }
    }

    /// F at the bellows (whole, not yet turned): it burns his foes from now on.
    fn turn_bellows(&mut self) {
        if !self.input.was_pressed(&[Action::Step]) {
            return;
        }
        let hp = self.hero.pos();
        let Some(g) = self.general.as_mut() else { return };
        let Some(k) = g.general().index_of(OrderKind::Kindle) else { return };
        let at = g.command.instruments[k].tile.v();
        if dist_sq(at, hp) > TURN_REACH * TURN_REACH || !g.command.turn(k) {
            return;
        }
        g.tally.turned += 1;
        for v in g.volleys.iter_mut().filter(|v| v.due.is_none()) {
            v.turned = true;
        }
        self.floater(at.x, at.y - 1.4, "THE BELLOWS TURNED: IT BURNS HIS OWN", 1.8, Some(0xffa040));
        self.audio.play_at(Sfx::Chain, at);
        self.keep_record();
        self.know_general(false);
    }

    /// The puppets on their ways: the watch on its round, the mustered on the march, the carriers,
    /// the guards sent to an instrument. A walker struck on the way turns on him (but a carrier).
    fn walk_puppets(&mut self, dt: f64) {
        let Some(g) = self.general.as_mut() else { return };
        let mut walkers = std::mem::take(&mut g.walkers);
        let mut done: Vec<(Walker, bool)> = Vec::new();
        let hp = self.hero.pos();
        walkers.retain_mut(|w| {
            let Some(i) = self.enemies.iter().position(|e| e.duel.as_ref().is_some_and(|d| d.id == w.id)) else { return false };
            let Some(d) = self.enemies[i].duel.as_deref_mut() else { return false };
            if !d.alive() {
                if matches!(w.then, Then::Herald(_)) {
                    done.push((w.clone(), true));
                }
                return false;
            }
            let struck = d.hp < w.hp - 0.5;
            w.hp = d.hp;
            if struck && !matches!(w.then, Then::Court | Then::Herald(_)) {
                done.push((w.clone(), true));
                return false;
            }
            if d.f.reeling().is_some() {
                return true;
            }
            w.at += w.pace * dt;
            let n = w.path.len();
            if n == 0 {
                return false;
            }
            let last = (n - 1) as f64;
            if w.then == Then::Round && w.at > last {
                w.at -= last.max(1.0);
            }
            if w.then == Then::Hunt && dist_sq(d.pos, hp) <= LOOSE_AT * LOOSE_AT && w.at > 0.0 {
                done.push((w.clone(), false));
                return false;
            }
            if w.at >= last && w.then != Then::Round {
                d.pos = w.path[n - 1].v();
                done.push((w.clone(), false));
                return false;
            }
            let at = w.at.max(0.0);
            let k = at.floor() as usize;
            let (a, b) = (w.path[k.min(n - 1)].v(), w.path[(k + 1).min(n - 1)].v());
            let u = at - k as f64;
            let p = V::new(a.x + (b.x - a.x) * u, a.y + (b.y - a.y) * u);
            let to = math::sub(b, a);
            if math::len(to) > 0.01 {
                d.facing = math::norm(to);
            }
            d.pos = p;
            d.puppet = true;
            self.enemies[i].m.tile = P::new(p.x.round() as i32, p.y.round() as i32);
            true
        });
        if let Some(g) = self.general.as_mut() {
            g.walkers = walkers;
        }
        for (w, struck) in done {
            match (w.then, struck) {
                (Then::Herald(order), true) => self.herald_caught(order, w.id),
                (Then::Herald(_), false) => self.herald_arrives(),
                (Then::Round, true) => {
                    // Ambushed on its round, before any whistle: the watch turns on him.
                    let first = self.general.as_ref().is_some_and(|g| !g.alarmed);
                    if first {
                        if let Some(at) = self.foe(w.id).map(|d| d.pos) {
                            self.floater(at.x, at.y - 1.4, "AMBUSHED BEFORE THE WHISTLE", 1.2, Some(0x9ad1ff));
                        }
                        self.progress.runes += AMBUSH_RUNES;
                    }
                    self.loose_the_watch();
                }
                (Then::Court, _) => {
                    if let Some(i) = self.foe_at(w.id) {
                        let p = self.enemies[i].pos();
                        self.enemies[i].b.mode = EnemyMode::Eaten;
                        self.floater(p.x, p.y - 1.2, "CARRIED INTO THE COURT", 1.2, Some(0xf2c53d));
                        self.view.sparkle(&self.level.grid, p.x, p.y, 1.0, 0xf2c53d, 16);
                    }
                    if let Some(g) = self.general.as_mut() {
                        g.arrived += 1;
                        g.carriers.retain(|&c| c != w.id);
                    }
                    let k = self.general.as_ref().and_then(|g| g.general().index_of(OrderKind::Decree));
                    let none_left = self.general.as_ref().is_some_and(|g| g.carriers.is_empty());
                    if let (Some(k), true) = (k, none_left) {
                        let cue = self.general.as_mut().and_then(|g| g.command.finish(k));
                        if let Some(c) = cue {
                            self.general_cue(c);
                        }
                    }
                }
                (Then::Hold(at), false) => {
                    if let Some(d) = self.foe_mut(w.id) {
                        d.puppet = false;
                        d.pos = at.v();
                        d.den = Some(Self::den_at(at, 3.0));
                    }
                }
                _ => {
                    if let Some(d) = self.foe_mut(w.id) {
                        d.puppet = false;
                        d.den = None;
                    }
                }
            }
        }
    }

    /// The general falls: his orders end, his banner is the hero's, his prince is weakened.
    fn general_slain(&mut self, at: V) {
        let Some(g) = self.general.as_mut() else { return };
        if g.command.slain {
            return;
        }
        let cues = g.command.slay();
        g.id = None;
        g.rally = None;
        let t = self.time;
        g.tally.slain_at = Some(t);
        let gen = *g.general();
        let name = g.name.clone();
        let usurper = g.usurper;
        let pending: Vec<u32> = g.volleys.iter().map(|v| v.id).collect();
        g.volleys.clear();
        if let Some(b) = self.brawl.as_mut() {
            for id in pending {
                b.threat.release(id);
            }
        }
        for c in cues {
            self.general_cue(c);
        }
        self.drop_the_decree();
        self.stand_down_the_watch();
        if let Some(r) = usurper.and_then(|id| self.warband.save.ledger.slain(id)) {
            self.progress.runes += r.runes;
            self.progress.silver += r.silver;
        }
        self.workshop.trophies.push(banner_of(&name, gen.sin));
        let weaker = if gen.sin == Sin::Greed { "MAMMON LOSES HIS GOLEMS" } else { "HIS PRINCE IS WEAKER" };
        self.juice.banner = Some(Banner {
            text: format!("{name} IS SLAIN"),
            sub: format!("HIS ORDERS DIE WITH HIM   {weaker}"),
            color: gen.cloak,
            life: 3.2,
            max: 3.2,
        });
        self.floater(at.x, at.y - 2.0, "HIS BANNER IS YOURS", 1.8, Some(gen.cloak));
        self.view.trauma(Trauma::Trophy);
        self.audio.play(Sfx::Milestone);
        self.keep_record();
        self.know_general(false);
    }

    /// The banners of the generals the run has slain on the trophy wall (a run continued from its
    /// save hangs them again: the save keeps the deed, the wall is drawn from it).
    pub fn hang_banners(&mut self) {
        let records = self.general_records();
        for sin in crate::logic::sins::SINS {
            if !records.get(sin).slain {
                continue;
            }
            let color = General::of(sin).cloak;
            if !self.workshop.trophies.iter().any(|t| t.name.ends_with("'S BANNER") && t.color == color) {
                self.workshop.trophies.push(banner_of(General::of(sin).name, sin));
            }
        }
    }

    /// The general's command post, for the Map and the `general:S` capture: into the realm of
    /// `sin`, and set down before his post.
    pub fn jump_to_post(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let post = g.posts.post;
        let room = g.posts.room;
        // Just inside his room's door, facing him: the room's edge on the way he came.
        let way = general::walk(&g.layout, self.level.hero_spawn, post).unwrap_or_default();
        // (Where no floor leads there from where he arrives, as to a gallery of Pride's spire, a few
        // steps from the general on the floor about him.)
        let near = || {
            let g = &self.level.grid;
            let mut ring: Vec<P> = (-4..=4)
                .flat_map(|dy| (-4..=4).map(move |dx| P::new(post.x + dx, post.y + dy)))
                .filter(|p| g.is_walkable(p.x, p.y, false) && *p != post)
                .collect();
            ring.sort_by_key(|p| (((p.x - post.x).pow(2) + (p.y - post.y).pow(2) - 9).abs(), p.y, p.x));
            ring.first().copied()
        };
        let stand =
            way.iter().rev().find(|p| !room.contains(**p)).copied().or_else(|| way.get(way.len().saturating_sub(8)).copied()).or_else(near);
        if let Some(p) = stand {
            self.level.hero_spawn = p;
            self.hero.place(p);
        }
    }

    /// Fires order `k` of the realm's general at once (the `order:K` capture): announced now.
    pub fn debug_order(&mut self, k: usize) {
        let Some(g) = self.general.as_mut() else { return };
        let k = k.min(2);
        if let Some(r) = g.command.running {
            g.command.fail(r);
        }
        if !g.command.can_give(k) {
            return;
        }
        let lead = g.command.lead(k);
        g.command.stages[k] = OrderStage::Announced { left: lead };
        g.command.running = Some(k);
        g.command.given[k] += 1;
        self.general_cue(Cue::Announce { order: k, lead });
    }

    /// The nearest whole instrument within `reach` tiles, the gate glowing (and the general's
    /// pack), for the bot.
    pub fn general_targets(&self) -> (Vec<(usize, V)>, Option<V>) {
        let Some(g) = self.general.as_ref() else { return (Vec::new(), None) };
        let whole = (0..3).filter(|&k| !g.command.instruments[k].broken).map(|k| (k, g.instrument_at(k))).collect();
        (whole, g.glowing().map(|x| x.0))
    }

    /// The duelists of the general's own pack (him first, then his guard and his room's foes).
    pub fn general_pack(&self) -> Vec<u32> {
        let Some(g) = self.general.as_ref() else { return Vec::new() };
        let Some(id) = g.id else { return Vec::new() };
        let den = self.foe(id).and_then(|d| d.den);
        let mates = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref())
            .filter(|d| d.id != id && den.is_some_and(|n| d.den.is_some_and(|m| m.lo == n.lo && m.hi == n.hi)));
        std::iter::once(id).chain(mates.map(|d| d.id)).collect()
    }
}

/// A slain general's banner, for the trophy wall.
fn banner_of(name: &str, sin: Sin) -> Trophy {
    Trophy {
        name: format!("{name}'S BANNER"),
        line: format!("{} ran {} against you. His banner hangs here now.", title_case(name), sin.realm()),
        color: General::of(sin).cloak,
    }
}

/// "THE ASSESSOR" as "The Assessor".
pub fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let lower = w.to_lowercase();
            if matches!(lower.as_str(), "of" | "the" | "a") {
                return lower;
            }
            let mut c = lower.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
        .split_inclusive(". ")
        .map(|s| {
            let mut c = s.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// The card's second line for an order: what to do about it.
fn cue_line(gen: &General, kind: OrderKind) -> String {
    match kind {
        OrderKind::Watch => "BREAK THE FIRE, OR STRIKE THE WATCH BEFORE IT WHISTLES".into(),
        OrderKind::Muster => "BREAK IT, OR BAR THE GATE".into(),
        OrderKind::Kindle => format!("{} - STEP OFF THE MARKS, OR TURN THE BELLOWS (F)", gen.hazard),
        OrderKind::Decree => "KILL THE CARRIERS OR CUT THE CHAIN".into(),
    }
}

#[cfg(test)]
mod tests {

    /// A rival of the fourth rank usurps the realm's general (WARBAND.md §9.2): his name and body
    /// at the post, and in the fight he is the rival still (seated in the grudges, his lesson and
    /// traits with him), offered nowhere else.
    #[test]
    fn a_usurper_holds_the_post_and_fights_as_the_rival_he_is() {
        use crate::logic::war::names::Deed;
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = Game::new();
        let rival = g.promote_rival(EnemyKind::Werewolf, Deed::KillingBlow);
        for _ in 0..3 {
            g.warband.save.ledger.rank_up(rival);
        }
        g.map_jump(crate::game::save::Stage::Realm(Sin::Greed));
        g.enter(crate::game::Screen::Playing);
        for _ in 0..10 {
            g.update(crate::app::STEP);
        }
        let run = g.general.as_ref().expect("a general");
        assert_eq!(run.usurper, Some(rival));
        let id = run.id.expect("at his post");
        let seated: Vec<_> = g.war.grudges.seated.iter().filter(|s| s.1 == rival).collect();
        assert_eq!(seated.len(), 1, "seated once: {seated:?}");
        assert_eq!(seated[0].0, id, "as the general");
    }
    use super::*;
    use crate::game::save::Stage;
    use crate::game::Screen;

    /// Into the realm of `sin`, playing, before its general's post.
    pub(crate) fn at_the_post(sin: Sin) -> Game {
        std::env::set_var("VH_SAVE_DIR", std::env::temp_dir().join("vh-test"));
        let mut g = super::super::bot::fresh_game(1);
        g.map_jump(Stage::Realm(sin));
        g.enter(Screen::Playing);
        g.hero.free_move = true;
        g
    }

    /// `n` of the brawl's frames (60 a second) of play.
    fn frames(g: &mut Game, n: u32) {
        let at = |g: &Game| g.brawl.as_ref().map(|b| b.threat.frame).unwrap_or(0);
        let goal = at(g) + n;
        let mut guard = 0;
        while at(g) < goal && guard < n * 4 + 8 {
            guard += 1;
            g.update(crate::app::STEP);
            if g.screen == Screen::Ready {
                g.enter(Screen::Playing);
            }
        }
    }

    #[test]
    fn every_realm_has_its_general_at_his_post_with_his_guard_asleep() {
        for sin in crate::logic::sins::SINS {
            let g = at_the_post(sin);
            let r = g.general.as_ref().unwrap_or_else(|| panic!("{sin:?}: no general"));
            assert_eq!(r.sin, sin);
            let id = r.id.expect("he stands");
            let d = g
                .enemies
                .iter()
                .filter_map(|e| e.duel.as_deref())
                .find(|d| d.id == id)
                .unwrap_or_else(|| panic!("{sin:?}: he is not among the foes"));
            assert_eq!(d.breed.kind, General::of(sin).body);
            assert!(d.den.is_some_and(|den| !den.awake), "asleep at his post");
            // (At least his breed's health under the realm's weight; never past a lesser prince's.)
            assert!(d.max_hp >= Breed::of(d.breed.kind).stats().hp * 0.8, "{sin:?}: a general is as tough as his breed at least");
            assert!(general::general_health(d.breed.kind) <= general::GENERAL_HEALTH);
            // (His room's own pack is his guard, he its leader to be.)
            assert!(g.general_pack().len() >= 2, "{sin:?}: a guard about him");
            assert!(g.workshop.generals[sin.index()].is_some(), "the war table knows him");
        }
    }

    #[test]
    fn no_order_slows_or_holds_the_hero() {
        // Every order of every general fired in turn about him: his pace never changes, his feet
        // are never held, and every hazard that reaches him lands as one planned strike.
        for sin in crate::logic::sins::SINS {
            let mut g = at_the_post(sin);
            g.immortal = true;
            let speed = g.hero.base_speed;
            for k in 0..3 {
                g.debug_order(k);
                for _ in 0..900 {
                    frames(&mut g, 1);
                    assert!(g.brawl.as_ref().is_some_and(|b| b.held.is_none()), "{sin:?}: held");
                    assert_eq!(g.hero.base_speed, speed, "{sin:?}: slowed");
                    assert!(g.hero.slow_timer <= 0.0, "{sin:?}: slowed");
                    assert!(g.hero.root > 0.0, "{sin:?}: rooted");
                }
            }
            let s = g.brawl.as_ref().unwrap().stats;
            assert_eq!(s.overlaps, 0, "{sin:?}: {s:?}");
        }
    }

    #[test]
    fn the_kindled_floor_is_marked_three_seconds_and_bursts_as_planned() {
        let mut g = at_the_post(Sin::Greed);
        g.immortal = true;
        let k = General::of(Sin::Greed).index_of(OrderKind::Kindle).unwrap();
        g.debug_order(k);
        let mut seen: Vec<(u32, u32)> = Vec::new();
        for _ in 0..900 {
            frames(&mut g, 1);
            let now = g.brawl.as_ref().unwrap().threat.frame;
            for v in &g.general.as_ref().unwrap().volleys {
                if let Some(d) = v.due {
                    if !seen.iter().any(|s| s.0 == v.id) {
                        seen.push((v.id, now));
                        assert!(d >= now + KINDLE_FUSE - 1, "marked three seconds ahead");
                    }
                }
            }
        }
        let burst = g.general.as_ref().unwrap().tally.bursts;
        assert!(seen.len() >= 2 && burst >= 2, "{seen:?} {burst}");
        assert_eq!(g.brawl.as_ref().unwrap().stats.overlaps, 0);
    }

    #[test]
    fn a_broken_horn_ends_the_muster_and_a_barred_gate_turns_it_back() {
        let mut g = at_the_post(Sin::Greed);
        g.immortal = true;
        let m = General::of(Sin::Greed).index_of(OrderKind::Muster).unwrap();
        // Barred: he stands at the glowing gate.
        g.debug_order(m);
        let (gate, _) = g.general.as_ref().unwrap().glowing().expect("a gate glows");
        let before = g.enemies.len();
        g.hero.place(P::new(gate.x.round() as i32, gate.y.round() as i32));
        frames(&mut g, BAR_FRAMES + 2);
        let r = g.general.as_ref().unwrap();
        assert_eq!(r.tally.barred, 1);
        assert_eq!(r.command.stages[m], OrderStage::Ready);
        frames(&mut g, 300);
        assert!(g.enemies.len() <= before, "nobody marched in (the herald gone with his muster)");
        // Broken: three blows on the bell.
        for _ in 0..3 {
            g.strike_instrument(m);
        }
        let r = g.general.as_ref().unwrap();
        assert!(r.command.instruments[m].broken && r.command.stages[m] == OrderStage::Broken);
        assert!(g.general_records().get(Sin::Greed).broken & 1 != 0, "the run keeps it");
        // He answers: a guard to the next whole instrument, announced first.
        frames(&mut g, general::COUNTER_LEAD + 2);
        assert_eq!(g.general.as_ref().unwrap().tally.answered, 1);
    }

    /// The muster's herald runs from the horn to the gate; cut down before the muster bites, it
    /// fails, nobody marches in, and his map marks the general's post (WARBAND.md §9.1).
    #[test]
    fn a_herald_caught_fails_the_muster_and_his_map_marks_the_post() {
        let mut g = at_the_post(Sin::Greed);
        g.immortal = true;
        let m = General::of(Sin::Greed).index_of(OrderKind::Muster).unwrap();
        g.debug_order(m);
        let herald = g.general.as_ref().unwrap().herald.expect("the herald sets out");
        frames(&mut g, 30);
        let before = g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.alive())).count();
        if let Some(d) = g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|d| d.id == herald) {
            d.f.kill();
        }
        frames(&mut g, 2);
        let r = g.general.as_ref().unwrap();
        assert_eq!(r.tally.heralds, 1);
        assert!(r.post_known, "his map marks the post");
        assert_eq!(r.command.stages[m], OrderStage::Ready, "the muster failed");
        assert!(g.workshop.news.iter().any(|n| n.contains("herald")));
        frames(&mut g, 400);
        let after = g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.alive())).count();
        assert!(after < before, "nobody marched in");
    }

    #[test]
    fn slaying_the_general_ends_his_orders_hangs_his_banner_and_weakens_the_prince() {
        let mut g = at_the_post(Sin::Greed);
        g.immortal = true;
        let d = General::of(Sin::Greed).index_of(OrderKind::Decree).unwrap();
        g.debug_order(d);
        frames(&mut g, 200);
        assert!(!g.general.as_ref().unwrap().carriers.is_empty(), "the carriers set out");
        let id = g.general.as_ref().unwrap().id.unwrap();
        if let Some(du) = g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|x| x.id == id) {
            du.f.kill();
        }
        frames(&mut g, 2);
        let r = g.general.as_ref().unwrap();
        assert!(r.command.slain && r.carriers.is_empty() && r.command.running.is_none());
        assert!(g.workshop.trophies.iter().any(|t| t.name.contains("BANNER")));
        let t = g.prince_terms(Sin::Greed);
        assert!(!t.phase_two_adds && !t.adds_at_start);
        // Mammon's court: no golems at his second phase.
        g.map_jump(Stage::Court(Sin::Greed));
        let m = g.prince().unwrap();
        assert!(m.lore.summoned, "his phase-two adds are lost");
        // And a decree done brings them at once.
        let mut g = at_the_post(Sin::Greed);
        let mut all = g.general_records();
        all.set(Sin::Greed, general::Record { met: true, decree_done: true, ..Default::default() });
        g.warband.save.generals = all.encode();
        g.map_jump(Stage::Court(Sin::Greed));
        g.enter(Screen::Playing);
        frames(&mut g, 3);
        let golems = g.enemies.iter().filter_map(|e| e.duel.as_deref()).filter(|d| d.breed.kind == EnemyKind::GoldGolem).count();
        assert_eq!(golems, crate::logic::combat::prince::GOLEMS as usize, "the treasurers' decree: the golems come at once");
    }

    #[test]
    fn the_generals_realm_survives_the_save() {
        let mut g = at_the_post(Sin::Wrath);
        for _ in 0..3 {
            g.strike_instrument(1);
        }
        let text = g.warband.save.encode();
        let back = crate::logic::war::save::WarSave::decode(&text).unwrap();
        assert_eq!(Records::decode(&back.generals).get(Sin::Wrath).broken, 0b010);
        // Back into the realm (Continue): the sluice is still broken.
        g.map_jump(Stage::Realm(Sin::Wrath));
        let r = g.general.as_ref().unwrap();
        assert!(r.command.instruments[1].broken && r.command.stages[1] == OrderStage::Broken);
    }
}
