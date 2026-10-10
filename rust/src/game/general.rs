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
//! Every instrument breaks under three of his blows; the generals answer each loss once (a guard to
//! the next instrument, or the muster doubled), announced as any order is.
//!
//! **Three generals a realm, three keys** (BACKLOG B41): each general stands at his own post with
//! his guard, gives one of the three orders, and carries a key. Slain, his order ends, his banner
//! goes to the workshop's wall, and his key falls where he fell: a pickup with a moment. With all
//! three keys the door to the prince's court opens (`gate`); until then it is sealed with a
//! "3 KEYS" lock. In the six realms after the vaults the third general comes out where the realm's
//! climax ends (`climax_survived`): the climax is the road to him. The prince's adds always come
//! (D14); the court reads which generals fell and whether the decree was done (`prince_terms`).

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
use crate::logic::types::{dist_sq, Tile, P, V};
use crate::logic::war::general::{
    self, place, Command, Counter, Cue, General, OrderKind, OrderStage, Posts, PrinceTerms, Records, BAR_FRAMES, CARRIER_PACE, CLIMAX,
    HAZARD_IDS, KINDLE_EVERY, KINDLE_FUSE, KINDLE_MARKS, KINDLE_RADIUS, KINDLE_ZONE, MUSTER, PATROL, PATROL_SIGHT, WHISTLE, WHISTLE_WAKES,
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
/// The keys to the prince's court: one a general.
pub const KEYS: u32 = 3;
/// He takes a fallen general's key this near it (tiles).
pub const KEY_REACH: f64 = 0.8;
/// A general's post is known once he has come this near it (tiles).
const SEEN_AT: f64 = 11.0;
/// His name card is shown when he wakes this near the hero (tiles).
const CARD_AT: f64 = 12.0;
/// Tiles of road between one picket and the next on the way to a general (B42).
const PICKET_EVERY: usize = 14;
/// Seconds after a climax's payoff before its general comes out.
pub const CLIMAX_BREATH: f64 = 2.0;

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
    /// When the last general fell (the realm's clock), and the generals slain, a bit each.
    pub slain_at: Option<f64>,
    pub slain: u8,
    /// Keys taken this visit.
    pub keys: u32,
    /// Each general's fight: when he woke for the hero (the realm's clock) and when he fell.
    pub woke: [Option<f64>; 3],
    pub fell: [Option<f64>; 3],
}

/// One of the realm's three generals in play (BACKLOG B41).
#[derive(Clone, Debug)]
pub struct Lord {
    /// His name (a usurper's, if a rival took his place), his body and his cloak.
    pub name: String,
    pub body: EnemyKind,
    pub cloak: u32,
    pub usurper: Option<RivalId>,
    /// His duelist (none if he fell earlier in the run, or has not come out yet).
    pub id: Option<u32>,
    pub slain: bool,
    /// His guard called at half his health: the frames to its coming.
    pub rally: Option<u32>,
    pub rallied: bool,
    /// His post is known (seen, mapped by a caught herald, named by the old man): on the minimap.
    pub known: bool,
    /// His name card shown this visit.
    pub met: bool,
    /// The climax's general, waiting for the realm's last trap to end before he comes out.
    pub waiting: bool,
    /// His key lying where he fell, and whether it is taken.
    pub key: Option<V>,
    pub key_taken: bool,
}

/// A realm's generals at run time.
#[derive(Clone, Debug)]
pub struct GeneralRun {
    pub sin: Sin,
    pub command: Command,
    pub posts: Posts,
    /// The three, general `k` giving order `k`.
    pub lords: [Lord; 3],
    /// The realm has a climax that leads to the third (the six realms after the vaults), and it
    /// has been survived.
    pub climax: bool,
    pub climax_done: bool,
    /// When the third comes out (the realm's clock), a breath after the climax's own payoff.
    pub climax_due: Option<f64>,
    /// The watch.
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
    /// The herald on his way with the muster.
    pub herald: Option<u32>,
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
    /// The realm's command.
    pub fn general(&self) -> &General {
        &self.command.general
    }
    /// General `k` in play.
    pub fn lord(&self, k: usize) -> &Lord {
        &self.lords[k.min(2)]
    }
    /// Which general duelist `id` is.
    pub fn lord_of(&self, id: u32) -> Option<usize> {
        (0..3).find(|&k| self.lords[k].id == Some(id))
    }
    /// The keys taken.
    pub fn keys(&self) -> u32 {
        self.lords.iter().filter(|l| l.key_taken).count() as u32
    }
    /// The generals still standing (or still to come out).
    pub fn standing(&self) -> usize {
        self.lords.iter().filter(|l| !l.slain).count()
    }
    /// The card's line for order `k`, in its general's name as he is now ("THE ASSESSOR MUSTERS").
    pub fn card(&self, k: usize) -> String {
        format!("{} {}", self.lord(k).name, self.general().orders[k.min(2)].verb())
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

    fn keep_record(&mut self) {
        let Some(g) = self.general.as_ref() else { return };
        let mut all = self.general_records();
        let mut r = g.command.record();
        r.keys = (0..3).filter(|&k| g.lords[k].key_taken).fold(0u8, |a, k| a | 1 << k);
        all.set(g.sin, r);
        self.warband.save.generals = all.encode();
    }

    /// The prince of `sin`'s fight, by what his generals' realm came to this run: which of them fell
    /// and whether the decree was done (his adds always come, D14).
    pub fn prince_terms(&self, sin: Sin) -> PrinceTerms {
        let r = self.general_records().get(sin);
        general::terms(sin, r.decree_done, r.slain)
    }

    /// The door to the prince's court may open: all three keys held (or the realm has no generals,
    /// `VH_NO_GENERAL`).
    pub fn court_keys_held(&self) -> bool {
        self.general.as_ref().is_none_or(|g| g.keys() >= KEYS)
    }

    /// Where the third general comes out as the realm's climax ends: a few steps before the court's
    /// portal, on its floor.
    fn climax_spot(grid: &crate::logic::grid::Grid, exit: crate::logic::hub::Gate) -> P {
        let (dx, dy) = exit.facing.vec();
        Self::near_floor(grid, P::new(exit.tile.x + dx * 3, exit.tile.y + dy * 3), Some(exit.front()))
    }

    /// The floor tile nearest `want` (within five), not `not`.
    fn near_floor(grid: &crate::logic::grid::Grid, want: P, not: Option<P>) -> P {
        let mut best = want;
        let mut d = i32::MAX;
        for y in -5..=5 {
            for x in -5..=5 {
                let p = P::new(want.x + x, want.y + y);
                let dd = (p.x - want.x).abs() + (p.y - want.y).abs();
                if grid.is_walkable(p.x, p.y, false) && dd < d && Some(p) != not {
                    d = dd;
                    best = p;
                }
            }
        }
        best
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
        let (wh, wd) = self.ng_foe(if self.in_vault() {
            (super::brawl::VAULT_HP, 1.0)
        } else {
            crate::logic::realms::weight(if kind.realm_own() { 0 } else { stage }, tier)
        });
        d.max_hp *= wh * health;
        d.hp = d.max_hp;
        d.breed.damage *= wd * self.realm_blows();
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
     * The realm's three generals take their posts as the level is laid out (the vaults, or one of
     * the six realms): placed by the realm's seed, their instruments standing, each general's guard
     * asleep about him, the watch asleep at its fire; as the run left them (instruments broken stay
     * broken, a slain general stays slain and his key, taken, stays his). In a realm after the
     * vaults the third waits for its climax to end. The war table learns of them.
     */
    pub(super) fn enter_general(&mut self) {
        self.general = None;
        if off() {
            return;
        }
        let tier = self.realm_tier();
        // (Placed on the realm's ground as it was laid out, before its rules flood or hide any of it.)
        let (sin, arrival, court, rooms, layout, climax) = if let Some(v) = &self.vault {
            (Sin::Greed, v.arrival, P::new(v.exit_gate.x - 1, v.exit_gate.y), general::vault_rooms(v), self.level.grid.clone(), None)
        } else if let Some(r) = &self.run {
            let b = r.base();
            let layout = crate::logic::level::parse_level(&b.level(&self.level.data)).grid;
            // (The Rotting Feast's third stands at the table at the Stomach's heart, the gullet his
            // road; every other realm's comes out where its climax ends, before the court's portal.)
            let spot = match r {
                Run::Gluttony(f) => Self::near_floor(&layout, P::new(f.dome.x, f.dome.y + 3), Some(f.dome)),
                _ => Self::climax_spot(&layout, b.exit),
            };
            (Run::sin(r), b.arrival, b.exit.front(), general::realm_rooms(b), layout, Some(spot))
        } else {
            return;
        };
        let waits = climax.is_some() && !matches!(self.run.as_ref(), Some(Run::Gluttony(_)));
        // The realm's foes' posts, on its ground as it was laid out (the generals take some of them).
        let foes: Vec<P> = self
            .enemies
            .iter()
            .filter_map(|e| e.duel.as_deref().and_then(|d| d.den).map(|den| P::new(den.home.x.round() as i32, den.home.y.round() as i32)))
            .collect();
        // (The Rotting Feast's seams are no posts: a general in a gatehouse would be shut in its
        // teeth. Its garrisons and their posts are left out.)
        let (rooms, foes) = match self.run.as_ref() {
            Some(Run::Gluttony(f)) => {
                let seams: Vec<crate::logic::hub::Rect> = f
                    .seams
                    .iter()
                    .map(|s| crate::logic::hub::Rect { x0: 0, y0: s.bottom.min(s.top) - 1, x1: layout.width, y1: s.bottom.max(s.top) + 1 })
                    .collect();
                let clear = |r: &crate::logic::hub::Rect| !seams.iter().any(|s| r.y0 <= s.y1 && s.y0 <= r.y1);
                let rooms: Vec<_> = rooms.into_iter().filter(clear).collect();
                let foes: Vec<P> = foes.into_iter().filter(|p| !seams.iter().any(|s| s.contains(*p))).collect();
                (rooms, foes)
            }
            _ => (rooms, foes),
        };
        let gen = General::of(sin);
        let Some(posts) = place(&layout, arrival, court, &rooms, &foes, gen.orders, general::seed_of(sin, tier), climax) else { return };
        let mut command = Command::new(sin, posts.instruments);
        command.decree_frames = ((posts.court.len() as f64 / CARRIER_PACE) * 60.0) as u32 + 120;
        let record = self.general_records().get(sin);
        command.restore(record);
        // A rival of rank four or more may have taken the place of the first general still standing.
        let usurped = (0..3).find(|&k| record.slain & (1 << k) == 0 && k != CLIMAX);
        let usurper = usurped.and(self.ledger().usurper());
        let lords: [Lord; 3] = std::array::from_fn(|k| {
            let c = gen.captains[k];
            let rival = if Some(k) == usurped {
                usurper.and_then(|id| self.ledger().get(id).map(|r| (id, r.name.to_uppercase(), r.kind)))
            } else {
                None
            };
            let slain = record.slain & (1 << k) != 0;
            Lord {
                name: rival.as_ref().map(|r| r.1.clone()).unwrap_or_else(|| c.name.to_string()),
                body: rival.as_ref().map(|r| r.2).unwrap_or(c.body),
                cloak: c.cloak,
                usurper: rival.map(|r| r.0),
                id: None,
                slain,
                rally: None,
                rallied: false,
                known: false,
                met: false,
                waiting: waits && k == CLIMAX && !slain,
                key: None,
                key_taken: record.keys & (1 << k) != 0,
            }
        });
        self.general = Some(GeneralRun {
            sin,
            command,
            lords,
            climax: waits,
            climax_done: false,
            climax_due: None,
            patrol: Vec::new(),
            brought: Vec::new(),
            walkers: Vec::new(),
            whistle: None,
            alarmed: false,
            gate: None,
            barred: vec![false; posts.gates.len()],
            bar: 0,
            zone: posts.posts[0].v(),
            volleys: Vec::new(),
            next_volley: 0,
            volley_no: 0,
            arrived: 0,
            carriers: Vec::new(),
            struck: [false; 3],
            swing: None,
            herald: None,
            dealt: 0,
            cards: Vec::new(),
            tally: GeneralTally::default(),
            seen: 0,
            now: 0,
            posts,
            layout,
        });
        // A key that fell and was not taken (he left the realm before) lies where its general stood.
        for k in 0..3 {
            let lying = self.general.as_ref().is_some_and(|g| g.lords[k].slain && !g.lords[k].key_taken);
            if lying {
                let at = self.general.as_ref().map(|g| g.posts.post(k).v()).unwrap_or_default();
                self.drop_key(k, at, false);
            }
        }
        for k in 0..3 {
            let up = self.general.as_ref().is_some_and(|g| !g.lords[k].slain && !g.lords[k].waiting);
            if up {
                self.raise_lord(k);
            }
        }
        self.raise_watch();
        self.raise_pickets(arrival);
        // The court's door: three keys (B41).
        self.keys_total = KEYS;
        self.keys_got = self.general.as_ref().map(|g| g.keys()).unwrap_or(0);
        self.know_general(!record.met);
        self.keep_record();
        // (Every key already his, from an earlier visit: the door stands unsealed.)
        self.check_court_door();
    }

    /// The realm's climax has been survived this visit (with no generals, `VH_NO_GENERAL`: its door
    /// open, as before B41).
    pub fn realm_climax_done(&self) -> bool {
        match self.general.as_ref() {
            Some(g) => g.climax_done,
            None => self.gate_open,
        }
    }

    /// General `k` and his guard at his post (asleep in its room, its pack his to lead).
    fn raise_lord(&mut self, k: usize) {
        let Some(g) = self.general.as_ref() else { return };
        let (room, post, body) = (g.posts.rooms[k], g.posts.post(k), g.lords[k].body);
        let mut den = Den {
            lo: V::new(room.x0 as f64 - 0.5, room.y0 as f64 - 0.5),
            hi: V::new(room.x1 as f64 + 0.5, room.y1 as f64 + 0.5),
            home: post.v(),
            awake: false,
            returning: false,
        };
        // He takes the post of the room's foe that stood there: its pack is his guard. (Where none
        // stood, as where a climax ends, two of the realm's foes are his guard.)
        let mut guard = true;
        if let Some(i) = self.enemies.iter().position(|e| e.duel.as_deref().and_then(|d| d.den).is_some_and(|d| d.home == post.v())) {
            if let Some(theirs) = self.enemies[i].duel.as_deref().and_then(|d| d.den) {
                den = Den { home: post.v(), ..theirs };
            }
            self.enemies.remove(i);
            guard = false;
        }
        // (His room's pack is raised half as large again as laid out, B42: the post is reinforced.)
        if guard {
            den = Self::den_at(post, 3.0);
            for n in 0..2 {
                let (kind, stage) = self.deal();
                let at = self.open_near(post, n + 2);
                self.raise(kind, stage, at, Some(den), false, 1.0);
            }
        }
        let id =
            self.raise(body, crate::logic::realms::deepest_stage(self.realm_tier()), post, Some(den), false, general::general_health(body));
        // A usurper is a rival of the Ledger still: his lesson, his weakness and his nature go
        // with him into the fight (WARBAND.md §8-§9), and his card when he is met.
        if let (Some(fid), Some(rival)) = (id, self.general.as_ref().and_then(|g| g.lords[k].usurper)) {
            self.war.grudges.seated.push((fid, rival));
        }
        if let (Some(i), Some(g)) = (id.and_then(|id| self.foe_at(id)), self.general.as_ref()) {
            self.enemies[i].b.name = g.lords[k].name.clone();
        }
        if let Some(g) = self.general.as_mut() {
            g.lords[k].id = id;
        }
    }

    /**
     * Pickets on the roads to the generals (BACKLOG B42: more foes met a minute): along the way from
     * where he arrives to each general's post, every `PICKET_EVERY` tiles of open road clear of the
     * dens and of every foe, two of the realm's foes asleep in a small den of their own. Not in the
     * Rotting Feast, whose gullet is one road already full, nor with `VH_THIN`.
     */
    fn raise_pickets(&mut self, arrival: P) {
        if std::env::var_os("VH_THIN").is_some() || matches!(self.run.as_ref(), Some(Run::Gluttony(_))) {
            return;
        }
        let Some(g) = self.general.as_ref() else { return };
        let mut roads: Vec<Vec<P>> = Vec::new();
        for k in 0..3 {
            if g.lords[k].slain || g.lords[k].waiting {
                continue;
            }
            if let Some(w) = general::walk(&g.layout, arrival, g.posts.post(k)) {
                roads.push(w);
            }
        }
        let dens: Vec<crate::logic::hub::Rect> = match (&self.vault, &self.run) {
            (Some(v), _) => v.packs.iter().map(|p| p.den).collect(),
            (_, Some(r)) => r.base().packs.iter().map(|p| p.den).collect(),
            _ => Vec::new(),
        };
        let mut placed: Vec<P> = Vec::new();
        for road in roads {
            let mut i = PICKET_EVERY;
            while i + 8 < road.len() {
                let p = road[i];
                i += 1;
                let clear = !dens.iter().any(|d| d.contains(p))
                    && self.enemies.iter().all(|e| crate::logic::types::dist_sq(e.pos(), p.v()) > 5.0 * 5.0)
                    && placed.iter().all(|q| (q.x - p.x).abs() + (q.y - p.y).abs() > PICKET_EVERY as i32 / 2)
                    && self.level.grid.is_walkable(p.x, p.y, false);
                if !clear {
                    continue;
                }
                let den = Self::den_at(p, 2.5);
                for n in 0..2 {
                    let (kind, stage) = self.deal();
                    let at = if n == 0 { p } else { self.open_near(p, 1) };
                    self.raise(kind, stage, at, Some(den), false, 1.0);
                }
                placed.push(p);
                i += PICKET_EVERY;
            }
        }
    }

    /// The watch asleep at its fire (its fire whole, its general standing).
    fn raise_watch(&mut self) {
        let watch = self
            .general
            .as_ref()
            .and_then(|g| g.general().index_of(OrderKind::Watch).filter(|&w| !g.command.instruments[w].broken && !g.lords[w].slain));
        let Some(w) = watch else { return };
        let fire = self.general.as_ref().map(|g| g.command.instruments[w].tile).unwrap_or_default();
        let mut patrol = Vec::new();
        for k in 0..PATROL {
            let (kind, stage) = self.deal();
            let at = self.open_near(fire, k as i32 + 1);
            if let Some(pid) = self.raise(kind, stage, at, Some(Self::den_at(at, 2.0)), false, 1.0) {
                patrol.push(pid);
            }
        }
        if let Some(g) = self.general.as_mut() {
            g.patrol = patrol;
        }
    }

    /**
     * The realm's climax is survived (the Peerless shattered, the bell lifted and dived, the truce
     * held, the Long Sleep outlasted, the Last Course left lean, the wedding night endured): its
     * general comes out before the court's portal with his guard, the third key on him (B41). Where
     * the realm has no generals (`VH_NO_GENERAL`) the portal simply opens, as before B41.
     */
    pub(super) fn climax_survived(&mut self) {
        let Some(g) = self.general.as_mut() else {
            self.gate_open = true;
            return;
        };
        if g.climax_done {
            return;
        }
        g.climax_done = true;
        if g.lords[CLIMAX].waiting {
            g.climax_due = Some(self.time + CLIMAX_BREATH);
            g.lords[CLIMAX].known = true;
        }
        self.keep_record();
        self.check_court_door();
    }

    /// The climax's general comes out at his post, awake, with his guard: his entrance.
    pub(super) fn bring_out_climax_general(&mut self) {
        let Some(g) = self.general.as_mut() else { return };
        g.climax_due = None;
        if !g.lords[CLIMAX].waiting {
            return;
        }
        g.lords[CLIMAX].waiting = false;
        self.raise_lord(CLIMAX);
        let post = self.general.as_ref().map(|g| g.posts.post(CLIMAX)).unwrap_or_default();
        // He comes for the hero at once: his den awake.
        for e in self.enemies.iter_mut() {
            if let Some(d) = e.duel.as_deref_mut() {
                if let Some(den) = d.den.as_mut().filter(|den| {
                    den.home.x.round() as i32 - post.x <= 3
                        && post.x - den.home.x.round() as i32 <= 3
                        && (den.home.y.round() as i32 - post.y).abs() <= 3
                }) {
                    den.awake = true;
                }
            }
        }
        let (name, line, cloak) = self
            .general
            .as_ref()
            .map(|g| (g.lords[CLIMAX].name.clone(), g.general().captains[CLIMAX].line, g.lords[CLIMAX].cloak))
            .unwrap_or_default();
        self.juice.banner = Some(Banner {
            text: format!("{name} COMES OUT"),
            sub: format!("{line}   HE CARRIES THE LAST KEY"),
            color: cloak,
            life: 3.4,
            max: 3.4,
        });
        if let Some(g) = self.general.as_mut() {
            g.lords[CLIMAX].met = true;
        }
        self.audio.play(Sfx::Stinger);
        self.view.trauma(Trauma::Horn);
        let p = post.v();
        self.view.sparkle(&self.level.grid, p.x, p.y, 1.6, cloak, 40);
        self.view.dust(&self.level.grid, p.x, p.y, 0x8a7a68, 30, 2.2);
        self.refresh_foe_views();
    }

    /// The war table's card for the realm's generals, and the old man's news of them the first time.
    fn know_general(&mut self, first: bool) {
        let Some(g) = self.general.as_ref() else { return };
        let gen = *g.general();
        let mut known: Vec<String> = Vec::new();
        for k in 0..3 {
            let i = g.command.instruments[k];
            let state = if g.lords[k].slain {
                "slain"
            } else if i.broken {
                "broken"
            } else if i.turned {
                "turned"
            } else {
                "standing"
            };
            known.push(format!(
                "{} {} by {} ({state})",
                title_case(&g.lords[k].name),
                gen.orders[k].verb().to_lowercase(),
                title_case(gen.instruments[k])
            ));
        }
        let keys = g.keys();
        let note = format!("{}. Keys to {}'s court: {keys} of {KEYS}.", known.join("; "), gen.sin.prince());
        let names = g.lords.iter().map(|l| title_case(&l.name)).collect::<Vec<_>>().join(", ");
        let sin = g.sin;
        let first_name = title_case(&g.lords[0].name);
        let climax = g.climax;
        let last = title_case(&g.lords[CLIMAX].name);
        self.workshop.generals[sin.index()] = Some(GeneralCard { sin, name: names, note });
        if first {
            let line = format!(
                "Three generals hold {} for {}, and each carries a key to his court. {} {} by {}: break it, and he cannot.{}",
                sin.realm(),
                sin.prince(),
                first_name,
                gen.orders[0].verb().to_lowercase(),
                title_case(gen.instruments[0]).to_lowercase(),
                if climax { format!(" {last} waits at the end of the realm's trap.") } else { String::new() }
            );
            self.workshop.news.push(line);
        }
    }

    /// The hero fell and the realm begins again: what the orders brought is gone, the order under
    /// way is called off, the kindled floor is cold; the generals and their guards are home (as the
    /// dens are); broken instruments stay broken, the slain stay slain, their keys stay his.
    pub(super) fn reset_general(&mut self) {
        let Some(g) = self.general.as_mut() else { return };
        let gone: Vec<u32> = g.brought.drain(..).chain(g.carriers.drain(..)).collect();
        g.walkers.clear();
        g.whistle = None;
        g.alarmed = false;
        g.gate = None;
        g.bar = 0;
        g.volleys.clear();
        for l in g.lords.iter_mut() {
            l.rally = None;
        }
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
        // them without the generals: each takes his post again.)
        for k in 0..3 {
            let lost = self.general.as_ref().is_some_and(|g| {
                let l = &g.lords[k];
                !l.slain && !l.waiting && l.id.is_none_or(|id| self.foe(id).is_none())
            });
            if lost {
                self.raise_lord(k);
            }
        }
        self.refresh_foe_views();
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

    /// One frame of the general's realm (after the brawl's).
    pub(super) fn update_general(&mut self, dt: f64) {
        if self.general.as_ref().and_then(|g| g.climax_due).is_some_and(|t| self.time >= t) {
            self.bring_out_climax_general();
        }
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

    /// Each general's death, his half-health horn, his name card when he wakes, his post seen; the
    /// keys lying about; the carriers' fate.
    fn watch_the_general(&mut self) {
        let hp = self.hero.pos();
        for k in 0..3 {
            let Some(g) = self.general.as_ref() else { return };
            let l = &g.lords[k];
            if l.slain {
                continue;
            }
            // His post is known once he has come near it (or near him).
            if !l.known && dist_sq(g.posts.post(k).v(), hp) <= SEEN_AT * SEEN_AT {
                if let Some(g) = self.general.as_mut() {
                    g.lords[k].known = true;
                }
            }
            let Some(g) = self.general.as_ref() else { return };
            let Some(id) = g.lords[k].id else { continue };
            // (Slain this frame, as the war log has it: the vaults sweep the fallen away at once.)
            let fell = self.war.log.iter().find_map(|e| match *e {
                WarEvent::Slain { foe, at, .. } if foe == id => Some(at),
                _ => None,
            });
            if let Some(at) = fell {
                self.general_slain(k, at);
                continue;
            }
            let rallied = g.lords[k].rallied;
            let met = g.lords[k].met;
            match self.foe(id).map(|d| (d.alive(), d.hp / d.max_hp.max(1.0), d.pos, d.engaged())) {
                Some((true, share, at, _)) if share <= RALLY_AT && !rallied => {
                    let name = g.lords[k].name.clone();
                    if let Some(g) = self.general.as_mut() {
                        g.lords[k].rallied = true;
                        g.lords[k].rally = Some(RALLY_LEAD);
                    }
                    self.horn(at, k, false);
                    self.order_card(
                        format!("{name} SOUNDS HIS HORN - 1"),
                        "HIS GUARD STANDS FAST: STRIKE NOW OR WAIT IT OUT".into(),
                        0xff8a3a,
                        None,
                    );
                }
                Some((true, _, at, true)) if !met && dist_sq(at, hp) <= CARD_AT * CARD_AT => self.general_met(k, at),
                Some((true, ..)) => {}
                Some((false, _, at, _)) => self.general_slain(k, at),
                // (Gone from the field without falling: a realm raised its packs afresh. He takes his
                // post again when the hero next rises.)
                None => {}
            }
        }
        self.take_keys();
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

    /// General `k` wakes and comes for him: his name card (WARBAND.md §8.4's beat for a rival), a
    /// moment's slowing, his cry.
    fn general_met(&mut self, k: usize, at: V) {
        let t = self.time;
        let Some(g) = self.general.as_mut() else { return };
        g.lords[k].met = true;
        g.lords[k].known = true;
        g.tally.woke[k].get_or_insert(t);
        let (name, body, cloak) = (g.lords[k].name.clone(), g.lords[k].body, g.lords[k].cloak);
        let line = if g.lords[k].usurper.is_some() { "A RIVAL IN A GENERAL'S PLACE" } else { g.general().captains[k].line };
        let order = g.general().orders[k];
        let instrument = g.general().instruments[k];
        self.juice.banner = Some(Banner {
            text: name,
            sub: format!("{line}   {} BY {instrument}   HE CARRIES A KEY", order.verb()),
            color: cloak,
            life: 3.0,
            max: 3.0,
        });
        self.dilate(crate::logic::juice::DilationKind::RivalMet);
        self.audio.foe_cry(body, crate::audio::Cry::Alert, at);
    }

    /// What the general's mind says, done in the world.
    fn general_cue(&mut self, cue: Cue) {
        let Some(g) = self.general.as_ref() else { return };
        let gen = *g.general();
        let sin = g.sin;
        match cue {
            Cue::Announce { order, lead } => {
                let kind = gen.orders[order];
                let hp = self.hero.pos();
                let step = self.input.prompt(crate::input::Action::Step).unwrap_or_default().to_uppercase();
                let mut sub = format!("{} - {}", title_case(gen.instruments[order]).to_uppercase(), cue_line(&gen, kind, &step));
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
                self.horn(at, order, kind == OrderKind::Muster);
                let (card, cloak) = self.general.as_ref().map(|g| (g.card(order), g.lords[order].cloak)).unwrap_or_default();
                self.order_card(card, sub, cloak, Some(order));
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
                            color: gen.captain(order).cloak,
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
                let name = self
                    .general
                    .as_ref()
                    .map(|g| g.lords.iter().find(|l| !l.slain).map(|l| l.name.clone()).unwrap_or_default())
                    .unwrap_or_default();
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
    fn horn(&mut self, at: V, k: usize, muster: bool) {
        let Some((bell, body, cloak)) =
            self.general.as_ref().map(|g| (g.general().bell(), g.lords[k.min(2)].body, g.lords[k.min(2)].cloak))
        else {
            return;
        };
        if muster && bell {
            self.audio.play_at(Sfx::Bells, at);
        } else {
            let listener = self.audio.listener();
            self.audio.tell(body, MoveId::Rally, at, listener);
        }
        if muster {
            self.view.trauma(Trauma::Horn);
        }
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.6, cloak, 18);
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
                g.lords[order.min(2)].known = true;
                g.tally.heralds += 1;
                g.walkers.retain(|w| w.id != id);
                (g.command.fail(order), g.lords[order.min(2)].name.clone())
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
        // (A gate the realm has shut for good since, a seam's teeth closed on it: none come through.)
        if g.layout.get(gate.x, gate.y) == Tile::Rock {
            if let Some(r) = self.general.as_mut() {
                r.gate = None;
                r.bar = 0;
            }
            return;
        }
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
        let post = g.general().index_of(OrderKind::Decree).map(|k| g.posts.post(k)).unwrap_or(g.posts.post(0));
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
        // (From the post of the general nearest the instrument still standing.)
        let from = (0..3)
            .filter(|&k| !g.lords[k].slain && !g.lords[k].waiting)
            .map(|k| g.posts.post(k))
            .min_by_key(|p| (p.x - to.x).abs() + (p.y - to.y).abs())
            .unwrap_or(g.posts.post(0));
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
        // At half his health a general's horn steadies his guard: their poise whole again.
        for k in 0..3 {
            let due = self.general.as_mut().and_then(|g| {
                let l = &mut g.lords[k];
                let r = l.rally.as_mut()?;
                *r = r.saturating_sub(1);
                (*r == 0).then(|| {
                    l.rally = None;
                    l.id
                })
            });
            self.rally_guard(k, due.flatten());
        }
    }

    /// General `k`'s horn has sounded out: his guard stands fast.
    fn rally_guard(&mut self, k: usize, id: Option<u32>) {
        if let Some(id) = id {
            if let Some(at) = self.foe(id).filter(|d| d.alive()).map(|d| d.pos) {
                let pack = self.general_pack(k);
                for d in self.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()) {
                    if pack.contains(&d.id) && d.alive() && dist_sq(d.pos, at) <= 36.0 && d.f.reeling().is_none() {
                        d.f.poise = d.f.max_poise;
                    }
                }
                let color = self.general.as_ref().map(|g| g.lords[k].cloak).unwrap_or(0xffc040);
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
            let name = self
                .general
                .as_ref()
                .and_then(|g| g.general().index_of(OrderKind::Watch).map(|w| g.lords[w].name.clone()))
                .unwrap_or_default();
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

    /// The Step's button (Left Shift / LT) at the bellows (whole, not yet turned): it burns his foes from now on.
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

    /**
     * The realm has shut `tiles` for good (the Rotting Feast's teeth closed on a seam): the general's
     * map of the ground learns it, and nothing of his walks through them from now on. A puppet whose
     * way crosses them gives the way up where it stands (one of a muster hunts him from there by its
     * own way; the rest stand guard there), and a watch whose round crosses them sets out no more
     * (QA.md Q3: the Steward's watch walked its round through the closed gatehouse, and his muster
     * marched in through the teeth from the digested zone).
     */
    pub(super) fn general_ground_shut(&mut self, tiles: &[P]) {
        let Some(g) = self.general.as_mut() else { return };
        for p in tiles {
            g.layout.set(p.x, p.y, Tile::Rock);
        }
        let crosses = |path: &[P]| path.iter().any(|p| tiles.contains(p));
        if crosses(&g.posts.round) {
            g.posts.round.clear();
        }
        // A muster's gate shut, or cut off from where he is: barred for good, as if he had barred
        // it (the muster glowing there fails).
        let mut failed = None;
        for i in 0..g.posts.gates.len() {
            let gate = g.posts.gates[i];
            let cut = tiles.contains(&gate) || !(0..3).any(|k| general::walk(&g.layout, gate, g.posts.post(k)).is_some());
            if cut && !g.barred[i] {
                g.barred[i] = true;
                if g.gate == Some(i) {
                    g.gate = None;
                    g.bar = 0;
                    failed = g.general().index_of(OrderKind::Muster).and_then(|k| g.command.fail(k));
                }
            }
        }
        let mut released: Vec<(u32, bool)> = Vec::new();
        g.walkers.retain(|w| {
            if matches!(w.then, Then::Herald(_)) || !crosses(&w.path) {
                return true;
            }
            released.push((w.id, w.then == Then::Hunt));
            false
        });
        for (id, hunt) in released {
            if let Some(d) = self.foe_mut(id) {
                d.puppet = false;
                let at = P::new(d.pos.x.round() as i32, d.pos.y.round() as i32);
                d.den = if hunt { None } else { Some(Self::den_at(at, 2.0)) };
            }
        }
        if let Some(c) = failed {
            self.general_cue(c);
        }
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
                    // (Where its post has turned to rock since, it holds where it came to.)
                    let open = self.level.grid.get(at.x, at.y) != Tile::Rock;
                    if let Some(d) = self.foe_mut(w.id) {
                        d.puppet = false;
                        let at = if open { at } else { P::new(d.pos.x.round() as i32, d.pos.y.round() as i32) };
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

    /// General `k` falls: his order ends, his banner is the hero's, his key falls where he fell.
    fn general_slain(&mut self, k: usize, at: V) {
        let Some(g) = self.general.as_mut() else { return };
        if g.lords[k].slain {
            return;
        }
        let cues = g.command.slay(k);
        g.lords[k].slain = true;
        g.lords[k].id = None;
        g.lords[k].rally = None;
        g.lords[k].known = true;
        let t = self.time;
        g.tally.slain_at = Some(t);
        g.tally.slain |= 1 << k;
        g.tally.fell[k] = Some(t);
        let gen = *g.general();
        let name = g.lords[k].name.clone();
        let cloak = g.lords[k].cloak;
        let usurper = g.lords[k].usurper;
        let order = gen.orders[k];
        let all = g.command.slain;
        // His kindling dies with him.
        let pending: Vec<u32> = if order == OrderKind::Kindle || all { g.volleys.iter().map(|v| v.id).collect() } else { Vec::new() };
        g.volleys.retain(|v| !pending.contains(&v.id));
        if let Some(b) = self.brawl.as_mut() {
            for id in pending {
                b.threat.release(id);
            }
        }
        for c in cues {
            self.general_cue(c);
        }
        if order == OrderKind::Decree || all {
            self.drop_the_decree();
        }
        if order == OrderKind::Watch || all {
            self.stand_down_the_watch();
        }
        if let Some(r) = usurper.and_then(|id| self.warband.save.ledger.slain(id)) {
            self.progress.runes += r.runes;
            self.progress.silver += r.silver;
        }
        self.workshop.trophies.push(banner_of(&name, cloak, gen.sin));
        let left = self.general.as_ref().map(|g| g.standing()).unwrap_or(0);
        self.juice.banner = Some(Banner {
            text: format!("{name} IS SLAIN"),
            sub: if left == 0 {
                "THE LAST OF THE THREE   TAKE HIS KEY".to_string()
            } else {
                format!("{} DIES WITH HIM   HIS KEY FALLS   {left} GENERAL{} LEFT", gen.instruments[k], if left == 1 { "" } else { "S" })
            },
            color: cloak,
            life: 3.2,
            max: 3.2,
        });
        self.floater(at.x, at.y - 2.0, "HIS BANNER IS YOURS", 1.8, Some(cloak));
        self.view.trauma(Trauma::Trophy);
        self.audio.play(Sfx::Milestone);
        self.drop_key(k, at, true);
        self.keep_record();
        self.know_general(false);
    }

    /// General `k`'s key falls at `at` (on the nearest open floor): a key in his cloak's colour,
    /// glinting, his for the walking over.
    fn drop_key(&mut self, k: usize, at: V, fresh: bool) {
        let grid = &self.level.grid;
        let t = P::new(at.x.round() as i32, at.y.round() as i32);
        // (On floor he can walk to from where he stands: a general digested behind the Feast's
        // teeth, or fallen off a slab, leaves his key where the hero can have it.)
        // (A key left lying on an earlier visit lies where its general stood.)
        let reach = if fresh { crate::logic::pathing::reachable(grid, self.hero.occupied(), false, false) } else { Default::default() };
        let mut best = if fresh { self.hero.occupied() } else { t };
        let mut d = i32::MAX;
        for y in -6..=6 {
            for x in -6..=6 {
                let p = P::new(t.x + x, t.y + y);
                let dd = x * x + y * y;
                if (reach.contains(&p) || !fresh && grid.is_walkable(p.x, p.y, false)) && dd < d && crate::game::bot::Bot::floor(self, p) {
                    d = dd;
                    best = p;
                }
            }
        }
        if fresh && d == i32::MAX {
            // (Nothing he can walk to near: the nearest floor he can walk to.)
            if let Some(p) = reach
                .iter()
                .copied()
                .filter(|p| crate::game::bot::Bot::floor(self, *p))
                .min_by_key(|p| ((p.x - t.x).pow(2) + (p.y - t.y).pow(2), p.y, p.x))
            {
                best = p;
            }
        }
        let spot = best.v();
        let Some(g) = self.general.as_mut() else { return };
        if g.lords[k].key_taken {
            return;
        }
        g.lords[k].key = Some(spot);
        let cloak = g.lords[k].cloak;
        if fresh {
            self.view.sparkle(&self.level.grid, spot.x, spot.y, 1.2, cloak, 40);
            self.audio.play_at(Sfx::Nodule, spot);
        }
    }

    /// Keys lying where their generals fell: walked over, each is his, with a moment (a flash, a
    /// chime, a beat of slowing, the count on the banner). The third opens the court's door.
    fn take_keys(&mut self) {
        if !self.hero.alive || self.hero.airborne() {
            return;
        }
        let hp = self.hero.pos();
        let Some(g) = self.general.as_ref() else { return };
        let Some(k) = (0..3).find(|&k| g.lords[k].key.is_some_and(|at| dist_sq(at, hp) <= KEY_REACH * KEY_REACH)) else { return };
        let at = g.lords[k].key.unwrap_or(hp);
        let name = g.lords[k].name.clone();
        let cloak = g.lords[k].cloak;
        let prince = g.sin.prince().to_uppercase();
        if let Some(g) = self.general.as_mut() {
            g.lords[k].key = None;
            g.lords[k].key_taken = true;
            g.tally.keys += 1;
        }
        let held = self.general.as_ref().map(|g| g.keys()).unwrap_or(0);
        self.keys_got = held;
        self.view.sparkle(&self.level.grid, at.x, at.y, 1.0, cloak, 50);
        self.view.flash_light(&self.level.grid, at, cloak);
        self.audio.play(Sfx::Milestone);
        self.juice.coin_punch = 1.0;
        self.dilate(crate::logic::juice::DilationKind::RivalMet);
        self.floater(at.x, at.y - 0.8, format!("{name}'S KEY"), 2.0, Some(cloak));
        self.juice.banner = Some(Banner {
            text: format!("KEY {held} OF {KEYS}"),
            sub: if held >= KEYS {
                format!("THE DOOR TO {prince}'S COURT OPENS")
            } else {
                format!("{} MORE TO OPEN {prince}'S DOOR", KEYS - held)
            },
            color: cloak,
            life: 2.6,
            max: 2.6,
        });
        self.keep_record();
        self.save_warband();
        self.know_general(false);
        self.check_court_door();
    }

    /// With every key held the court's door is unsealed (the realm's far portal, the vaults' door);
    /// the Rotting Feast's pylorus still parts only on its own beat, for the lean.
    pub(super) fn check_court_door(&mut self) {
        if !self.court_keys_held() || self.gate_open {
            return;
        }
        if !self.door_may_open() {
            return;
        }
        if matches!(self.run.as_ref(), Some(Run::Gluttony(_))) {
            return;
        }
        if self.run.is_some() || self.vault.is_some() {
            self.gate_open = true;
            self.audio.play(Sfx::Win);
            if let Some(g) = self.gate {
                self.view.sparkle(&self.level.grid, g.x as f64, g.y as f64, 1.4, 0xf2c53d, 50);
            }
        }
    }

    /**
     * Between "all three keys held" and "the court's door opens": the place for a choice offered at
     * the door (the backlog's temptation at a realm's end may go here; it would return false while
     * it asks, and call `check_court_door` again once answered). Today the door simply opens.
     */
    fn door_may_open(&mut self) -> bool {
        true
    }

    /// The banners of the generals the run has slain on the trophy wall (a run continued from its
    /// save hangs them again: the save keeps the deed, the wall is drawn from it).
    pub fn hang_banners(&mut self) {
        let records = self.general_records();
        for sin in crate::logic::sins::SINS {
            let r = records.get(sin);
            for (k, c) in General::of(sin).captains.iter().enumerate() {
                if r.slain & (1 << k) == 0 {
                    continue;
                }
                let name = format!("{}'S BANNER", c.name);
                if !self.workshop.trophies.iter().any(|t| t.name == name) {
                    self.workshop.trophies.push(banner_of(c.name, c.cloak, sin));
                }
            }
        }
    }

    /// General `k`'s command post, for the Map and the `general:S` capture: set down before his post
    /// (the third, in a realm with a climax, at the place he comes out).
    pub fn jump_to_post(&mut self, k: usize) {
        let Some(g) = self.general.as_ref() else { return };
        let post = g.posts.post(k);
        let room = g.posts.rooms[k.min(2)];
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

    /// The Map's and the `general:S:K` capture's stop: before general `k`'s post (the climax's
    /// general brought out first, as if the realm's last trap had been survived).
    pub fn debug_general(&mut self, k: usize) {
        if k >= CLIMAX && self.general.as_ref().is_some_and(|g| g.climax) {
            self.climax_survived();
            self.bring_out_climax_general();
        }
        self.jump_to_post(k);
    }

    /// Fires order `k` of the realm's generals at once (the `order:K` capture): announced now.
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

    /// The Rotting Feast's teeth close at row `top`: a key lying behind them (south) comes through to
    /// `to`, so none is ever shut in a digested zone.
    pub(super) fn keys_come_through(&mut self, top: i32, to: P) {
        let Some(g) = self.general.as_mut() else { return };
        for l in g.lords.iter_mut() {
            if l.key.is_some_and(|k| k.y.round() as i32 >= top) {
                l.key = Some(to.v());
            }
        }
    }

    /// General `k` falls at once (the `slaygeneral:K` capture): his key drops where he stood.
    pub fn debug_slay_general(&mut self, k: usize) {
        let Some(id) = self.general.as_ref().and_then(|g| g.lords[k.min(2)].id) else { return };
        if let Some(d) = self.foe_mut(id) {
            d.f.kill();
        }
    }

    /// Every general of the realm counted slain and his key taken (the tests, a testing cheat).
    pub fn debug_all_keys(&mut self) {
        let Some(g) = self.general.as_mut() else { return };
        for l in g.lords.iter_mut() {
            (l.slain, l.key_taken, l.key, l.waiting) = (true, true, None, false);
        }
        g.command.slay_all();
        self.keys_got = KEYS;
        self.keep_record();
        self.check_court_door();
    }

    /// The whole instruments and the gate glowing, for the bot.
    pub fn general_targets(&self) -> (Vec<(usize, V)>, Option<V>) {
        let Some(g) = self.general.as_ref() else { return (Vec::new(), None) };
        let whole = (0..3).filter(|&k| !g.command.instruments[k].broken).map(|k| (k, g.instrument_at(k))).collect();
        (whole, g.glowing().map(|x| x.0))
    }

    /// The tile before the court's door (the realm's far portal, the vaults' east door).
    pub fn court_door_front(&self) -> Option<P> {
        if let Some(v) = &self.vault {
            return Some(P::new(v.exit_gate.x - 1, v.exit_gate.y));
        }
        self.run.as_ref().map(|r| r.base().exit.front())
    }

    /// The bot's errand in a realm with generals (B41): a key lying about, else (with every key) the
    /// court's door, else the nearest general standing at his post (him, or his post if he is not
    /// on the field), else the climax's way (none: the realm's own errand).
    pub fn general_errand_goal(&self) -> Option<V> {
        let g = self.general.as_ref()?;
        let hp = self.hero.pos();
        let near = |a: &V, b: &V| dist_sq(*a, hp).total_cmp(&dist_sq(*b, hp));
        if let Some(k) = g.lords.iter().filter_map(|l| l.key).min_by(near) {
            return Some(k);
        }
        if g.keys() >= KEYS {
            // (The Rotting Feast's way out is its own errand: the dome, then the pylorus on its beat.)
            if matches!(self.run.as_ref(), Some(Run::Gluttony(_))) {
                return None;
            }
            return self.gate.map(|p| p.v());
        }
        let standing = g
            .lords
            .iter()
            .enumerate()
            .filter(|(_, l)| !l.slain && !l.waiting)
            .map(|(k, l)| l.id.and_then(|id| self.foe(id)).map(|d| d.pos).unwrap_or(g.posts.post(k).v()))
            .min_by(near);
        if standing.is_some() {
            return standing;
        }
        // (The other two down, their keys taken: on to the realm's climax, the road to the third.)
        if g.lords[CLIMAX].waiting && !g.climax_done {
            return self.climax_road();
        }
        None
    }

    /// Where the realm's climax is sprung: the throne's dais, the bell's pool, the horn on the
    /// field (then the truce stone), Hel's bed, the Black Thorn's gate.
    pub fn climax_road(&self) -> Option<V> {
        Some(match self.run.as_ref()? {
            Run::Pride(m) => m.throne.v(),
            Run::Envy(gr) => gr.bell.pool.first()?.v(),
            Run::Wrath(f) => {
                if f.called.is_some() {
                    f.stone.v()
                } else {
                    f.horn.v()
                }
            }
            Run::Sloth(m) => m.to_world(m.bed).v(),
            Run::Lust(t) => t.base.exit.front().v(),
            Run::Gluttony(_) => return None,
        })
    }

    /// The duelists of general `k`'s own pack (him first, then his guard and his room's foes).
    pub fn general_pack(&self, k: usize) -> Vec<u32> {
        let Some(g) = self.general.as_ref() else { return Vec::new() };
        let Some(id) = g.lords[k.min(2)].id else { return Vec::new() };
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
fn banner_of(name: &str, color: u32, sin: Sin) -> Trophy {
    Trophy {
        name: format!("{name}'S BANNER"),
        line: format!("{} ran {} against you. His banner hangs here now.", title_case(name), sin.realm()),
        color,
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

/// The card's second line for an order: what to do about it (`step` names the button that turns the bellows).
fn cue_line(gen: &General, kind: OrderKind, step: &str) -> String {
    match kind {
        OrderKind::Watch => "BREAK THE FIRE, OR STRIKE THE WATCH BEFORE IT WHISTLES".into(),
        OrderKind::Muster => "BREAK IT, OR BAR THE GATE".into(),
        OrderKind::Kindle => format!("{} - STEP OFF THE MARKS, OR TURN THE BELLOWS ({step})", gen.hazard),
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
        assert_eq!(run.lords[0].usurper, Some(rival));
        let id = run.lords[0].id.expect("at his post");
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
        assert!(r.lords[m].known, "his map marks his general's post");
        assert_eq!(r.command.stages[m], OrderStage::Ready, "the muster failed");
        assert!(g.workshop.news.iter().any(|n| n.contains("herald")));
        frames(&mut g, 400);
        let after = g.enemies.iter().filter(|e| e.duel.as_ref().is_some_and(|d| d.alive())).count();
        assert!(after < before, "nobody marched in");
    }

    /// BACKLOG B41: every realm has three generals, each in his own room with his guard asleep
    /// about him, wearing his own body; the third of a realm after the vaults waits for its climax
    /// to end, and comes out before the court's portal. The court's door is sealed: three keys.
    #[test]
    fn every_realm_has_its_three_generals_at_their_posts_with_their_guards_asleep() {
        for sin in crate::logic::sins::SINS {
            let mut g = at_the_post(sin);
            let r = g.general.as_ref().unwrap_or_else(|| panic!("{sin:?}: no general"));
            assert_eq!(r.sin, sin);
            assert_eq!((g.keys_total, g.keys_got), (KEYS, 0), "{sin:?}: three keys to the court");
            assert!(!g.gate_open, "{sin:?}: the court's door sealed");
            let climax = r.climax;
            assert_eq!(climax, sin != Sin::Greed && sin != Sin::Gluttony, "{sin:?}: the third waits for the climax");
            let posts = r.posts.posts;
            assert!(posts[0] != posts[1] && posts[1] != posts[2] && posts[0] != posts[2], "{sin:?}: {posts:?}");
            if climax {
                assert!(r.lords[CLIMAX].waiting && r.lords[CLIMAX].id.is_none(), "{sin:?}: not out before the climax");
                g.debug_general(CLIMAX);
            }
            let r = g.general.as_ref().unwrap();
            for k in 0..3 {
                let id = r.lords[k].id.unwrap_or_else(|| panic!("{sin:?}: general {k} stands"));
                let d = g
                    .enemies
                    .iter()
                    .filter_map(|e| e.duel.as_deref())
                    .find(|d| d.id == id)
                    .unwrap_or_else(|| panic!("{sin:?}: general {k} is not among the foes"));
                assert_eq!(d.breed.kind, General::of(sin).captains[k].body);
                if k != CLIMAX || !climax {
                    assert!(d.den.is_some_and(|den| !den.awake), "{sin:?} {k}: asleep at his post");
                } else {
                    assert!(d.den.is_some_and(|den| den.awake), "{sin:?}: come out, he comes for him");
                }
                // (At least his breed's health under the realm's weight; never past a lesser prince's.)
                assert!(d.max_hp >= Breed::of(d.breed.kind).stats().hp * 0.8, "{sin:?}: a general is as tough as his breed at least");
                assert!(g.general_pack(k).len() >= 2, "{sin:?} {k}: a guard about him");
            }
            assert!(g.workshop.generals[sin.index()].is_some(), "the war table knows them");
        }
    }

    /// A general slain: his order ends, his key falls (a pickup with a moment), and with the third
    /// key taken the court's door opens. The prince's adds still come (D14); the court reads which
    /// fell.
    #[test]
    fn slaying_a_general_ends_his_order_drops_his_key_and_three_keys_open_the_door() {
        let mut g = at_the_post(Sin::Greed);
        g.immortal = true;
        let d = General::of(Sin::Greed).index_of(OrderKind::Decree).unwrap();
        g.debug_order(d);
        frames(&mut g, 200);
        assert!(!g.general.as_ref().unwrap().carriers.is_empty(), "the carriers set out");
        let kill = |g: &mut Game, k: usize| {
            let id = g.general.as_ref().unwrap().lords[k].id.unwrap();
            if let Some(du) = g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|x| x.id == id) {
                du.f.kill();
            }
            frames(g, 2);
        };
        kill(&mut g, d);
        let r = g.general.as_ref().unwrap();
        assert!(r.lords[d].slain && !r.command.slain, "one of three");
        assert!(r.carriers.is_empty() && r.command.stages[d] == OrderStage::Broken, "his decree dies with him");
        assert!(g.workshop.trophies.iter().any(|t| t.name.contains("HOARD-HOUND'S BANNER")));
        let key = r.lords[d].key.expect("his key falls");
        assert!(g.level.grid.is_walkable(key.x as i32, key.y as i32, false));
        // Taken: one of three, and the door still sealed.
        g.hero.place(P::new(key.x as i32, key.y as i32));
        frames(&mut g, 2);
        assert_eq!(g.keys_got, 1);
        assert!(!g.gate_open);
        assert_eq!(g.general_records().get(Sin::Greed).keys, 1 << d, "the run keeps it");
        for k in [0, 2] {
            kill(&mut g, k);
            let key = g.general.as_ref().unwrap().lords[k].key.expect("his key");
            g.hero.place(P::new(key.x as i32, key.y as i32));
            frames(&mut g, 2);
        }
        assert_eq!(g.keys_got, KEYS);
        assert!(g.gate_open, "three keys: the door to Mammon's court opens");
        assert!(g.general.as_ref().unwrap().command.slain);
        let t = g.prince_terms(Sin::Greed);
        assert!(t.phase_two_adds && !t.adds_at_start && t.slain == 0b111, "{t:?}");
        // Mammon's court: his golems still come at his second phase (D14).
        g.map_jump(Stage::Court(Sin::Greed));
        let m = g.prince().unwrap();
        assert!(!m.lore.summoned, "his phase-two adds still come");
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

    /// The realm's climax is the road to its third general: survived, he comes out before the
    /// court's portal a breath later, with his guard, awake.
    #[test]
    fn the_climax_survived_brings_out_its_general() {
        let mut g = at_the_post(Sin::Lust);
        g.immortal = true;
        assert!(g.general.as_ref().unwrap().lords[CLIMAX].waiting);
        g.climax_survived();
        assert!(g.general.as_ref().unwrap().lords[CLIMAX].waiting, "a breath first");
        frames(&mut g, (CLIMAX_BREATH * 60.0) as u32 + 10);
        let r = g.general.as_ref().unwrap();
        assert!(!r.lords[CLIMAX].waiting);
        let id = r.lords[CLIMAX].id.expect("out");
        let exit = g.run.as_ref().unwrap().base().exit;
        let d = g.enemies.iter().filter_map(|e| e.duel.as_deref()).find(|d| d.id == id).unwrap();
        assert!((d.pos.x - exit.tile.x as f64).hypot(d.pos.y - exit.tile.y as f64) < 9.0, "before the court's portal: {:?}", d.pos);
        assert!(!g.gate_open, "the door waits for the keys");
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
        // A general slain, his key left lying.
        let id = g.general.as_ref().unwrap().lords[0].id.unwrap();
        if let Some(du) = g.enemies.iter_mut().filter_map(|e| e.duel.as_deref_mut()).find(|x| x.id == id) {
            du.f.kill();
        }
        frames(&mut g, 2);
        // Back into the realm (Continue): the chain is still broken, the general still slain, and
        // his key lies where he stood.
        g.map_jump(Stage::Realm(Sin::Wrath));
        let r = g.general.as_ref().unwrap();
        assert!(r.command.instruments[1].broken && r.command.stages[1] == OrderStage::Broken);
        let l = &r.lords[0];
        assert!(l.slain && l.id.is_none() && l.key.is_some(), "{:?} {:?} {:?} {}", l.slain, l.id, l.key, l.key_taken);
        assert_eq!(r.command.stages[0], OrderStage::Broken, "his order with him");
    }

    /**
     * The Rotting Feast's teeth close on a seam: nothing of the Steward's walks through them after
     * (QA.md Q3). A puppet whose way crosses them gives it up where it stands, his map of the ground
     * learns the rock (so no muster marches in through a gate behind them), and a watch whose round
     * crossed them sets out no more.
     */
    #[test]
    fn the_teeth_shut_the_generals_ways_too() {
        let mut g = at_the_post(Sin::Gluttony);
        frames(&mut g, 5);
        let Some(Run::Gluttony(f)) = g.run.as_ref() else { panic!("the Rotting Feast") };
        let seam = f.seams[0].clone();
        let shut: Vec<P> = seam.narrow.tiles.iter().chain(&seam.wide.tiles).copied().collect();
        let layout = g.general.as_ref().expect("the Steward").layout.clone();
        let way = general::walk(&layout, seam.wide.south[0], seam.wide.north[0]).expect("a way through the gatehouse");
        assert!(way.iter().any(|p| shut.contains(p)));
        let id = g.raise(EnemyKind::Skeleton, 1, way[0], None, true, 1.0).expect("raised");
        let hp = g.foe(id).unwrap().hp;
        g.general.as_mut().unwrap().walkers.push(Walker { id, path: way, at: 0.0, pace: 1.0, then: Then::Hold(seam.wide.north[0]), hp });
        let gate = g.general.as_ref().unwrap().posts.gates[0];
        g.general_ground_shut(&shut);
        let r = g.general.as_ref().unwrap();
        assert!(r.walkers.iter().all(|w| w.id != id), "its way given up");
        assert!(shut.iter().all(|p| r.layout.get(p.x, p.y) == Tile::Rock), "his map knows the rock");
        assert!(r.posts.round.iter().all(|p| !shut.contains(p)), "no round through the teeth");
        let d = g.foe(id).unwrap();
        assert!(!d.puppet && d.den.is_some(), "it stands guard where it was");
        // A muster through a gate shut since brings no one.
        g.general.as_mut().unwrap().layout.set(gate.x, gate.y, Tile::Rock);
        g.general.as_mut().unwrap().gate = Some(0);
        let before = g.enemies.len();
        g.muster(0);
        assert_eq!(g.enemies.len(), before, "none come through the teeth");
    }
}
