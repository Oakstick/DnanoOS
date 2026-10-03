use super::combat::Duelist;
use super::grid::Grid;
use super::hub::Rect;
use super::mover::Mover;
use super::pathing::{route_toward, step_toward};
use super::rng::Rng;
use super::types::{dist_sq, manhattan, Dir, Tile, CARDINALS, P, V};

/// The foes, stage by stage: the first four haunt the Garden of Krampus, the next six the seven
/// realms; the third stage's five and the seventeen of stages four to seven (Japan's yōkai,
/// Anatolia's night spirits, the legends, Ragnarök) train in the gym for now.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EnemyKind {
    Skeleton,
    Bat,
    Vampire,
    Slime,
    /// The witch: keeps her distance and conjures minions.
    Witch,
    /// Her minion: small, quick, and one blow is enough.
    Minion,
    /// The helmeted skeleton: the first blow rings off its iron cap.
    Helmeted,
    /// The loup-garou: fast, and faster once wounded.
    Werewolf,
    /// The blemmyes: a headless brute, three blows to fell, and his touch hurts twice as much.
    Blemmyes,
    /// The large bat: swerving as it flies.
    LargeBat,
    /// The tengu: a mountain goblin with a katana, quick on his feet.
    Tengu,
    /// The rasetsu: a horned red man-eater with a curved blade in each fist.
    Rasetsu,
    /// The dire wolf: bigger than the loup-garou; it throws its whole body at you.
    DireWolf,
    /// The jorogumo: a spider-woman who hurls fire from afar.
    Jorogumo,
    /// The cobra: coiled, it rears and strikes.
    Cobra,
    // ---- the fourth stage: the yōkai
    /// The kappa: a river imp with a dish of water on its head; it charges like a sumo wrestler,
    /// and bows back (spilling its dish, and its strength) when it is bowed to.
    Kappa,
    /// The rokurokubi: a woman whose neck stretches across the room, her head striking from afar.
    Rokurokubi,
    /// The kitsune: an evil fox that throws foxfire and hides among its illusions.
    Kitsune,
    /// The oni: a red ogre with an iron club that shakes the ground.
    Oni,
    // ---- the fifth stage: Anatolian nights
    /// The tülükuş: an owl demon that dives without a sound; its hoot foretells a death.
    Tulukus,
    /// The albastı: a night hag who pounces with the weight of a nightmare on the chest.
    Albasti,
    /// The cin: a spirit of smokeless fire that strikes out of nowhere, and vanishes at will.
    Cin,
    /// The arçura: a forest spirit that makes roots burst from the ground, and changes its size.
    Arcura,
    /// The karakoncolos: a winter demon that roars its prey's name, dooming it, then strikes.
    Karakoncolos,
    // ---- the sixth stage: the legends
    /// Tepegöz: the one-eyed giant whose body no blade can cut; only his eye can be hurt.
    Tepegoz,
    /// The gashadokuro: a giant skeleton of the starved, heard before it is seen; it crushes.
    Gashadokuro,
    /// The yedi başlı ejder: a seven-headed dragon; struck down, it begs for a second blow.
    Ejder,
    /// The goryō: a wronged noble's ghost that calls down lightning, untouchable but in its wrath.
    Goryo,
    // ---- the seventh stage: Ragnarök
    /// The frost giant: its blows frostbite, and the frost bites on a while.
    FrostGiant,
    /// Surtr: the fire giant whose flaming sword leaves the ground burning.
    Surtr,
    /// Níðhöggr: the dragon under the roots of the world; it burrows, and its bite is venom.
    Nidhogg,
    /// Hel: the half-dead queen of the dead; her bolts drain life, and she raises the dead.
    Hel,
    // ---- the realms' own (REALMS.md), in no stage's cast
    /// The Peerless: his own reflection, stepped out of the mirror at the top of Pride's spire.
    Peerless,
    /// Ganglati and Ganglöt: Hel's shrouded servants in the Grey Mire, snail-slow and never stopping.
    Ganglot,
    /// A gaki: a hungry ghost of the Rotting Feast, all swollen belly and needle throat.
    Gaki,
    /// An ellepige: an elf-maid of the Garden of Thorns, lovely from the front and hollow behind.
    Ellepige,
}

impl EnemyKind {
    pub const STAGE_ONE: [EnemyKind; 4] = [EnemyKind::Skeleton, EnemyKind::Bat, EnemyKind::Vampire, EnemyKind::Slime];
    pub const STAGE_TWO: [EnemyKind; 6] =
        [EnemyKind::Helmeted, EnemyKind::LargeBat, EnemyKind::Witch, EnemyKind::Minion, EnemyKind::Werewolf, EnemyKind::Blemmyes];

    pub const STAGE_THREE: [EnemyKind; 5] =
        [EnemyKind::Tengu, EnemyKind::Rasetsu, EnemyKind::DireWolf, EnemyKind::Jorogumo, EnemyKind::Cobra];
    pub const STAGE_FOUR: [EnemyKind; 4] = [EnemyKind::Oni, EnemyKind::Kappa, EnemyKind::Rokurokubi, EnemyKind::Kitsune];
    pub const STAGE_FIVE: [EnemyKind; 5] =
        [EnemyKind::Karakoncolos, EnemyKind::Tulukus, EnemyKind::Albasti, EnemyKind::Cin, EnemyKind::Arcura];
    pub const STAGE_SIX: [EnemyKind; 4] = [EnemyKind::Tepegoz, EnemyKind::Gashadokuro, EnemyKind::Ejder, EnemyKind::Goryo];
    pub const STAGE_SEVEN: [EnemyKind; 4] = [EnemyKind::FrostGiant, EnemyKind::Surtr, EnemyKind::Nidhogg, EnemyKind::Hel];
    /// The last stage.
    pub const STAGES: u8 = 7;
    /// The realms' own new foes, which belong to no stage.
    pub const REALMS_OWN: [EnemyKind; 4] = [EnemyKind::Peerless, EnemyKind::Ganglot, EnemyKind::Gaki, EnemyKind::Ellepige];

    /// One of the realms' own new foes.
    pub fn realm_own(self) -> bool {
        Self::REALMS_OWN.contains(&self)
    }

    /// The cast of a stage: 1 for the Garden, 2 for the realms, 3 to 7 for the stages after them.
    pub fn of_stage(stage: u8) -> &'static [EnemyKind] {
        match stage {
            0 | 1 => &Self::STAGE_ONE,
            2 => &Self::STAGE_TWO,
            3 => &Self::STAGE_THREE,
            4 => &Self::STAGE_FOUR,
            5 => &Self::STAGE_FIVE,
            6 => &Self::STAGE_SIX,
            _ => &Self::STAGE_SEVEN,
        }
    }

    /// The stage it belongs to.
    pub fn stage(self) -> u8 {
        (1..Self::STAGES).find(|&s| Self::of_stage(s).contains(&self)).unwrap_or(Self::STAGES)
    }

    /// Flies over pits and ledges.
    pub fn flies(self) -> bool {
        matches!(self, EnemyKind::Bat | EnemyKind::Vampire | EnemyKind::LargeBat)
    }

    /// Falls to pieces when slain, and pulls itself together again; hunts by ear.
    pub fn reassembles(self) -> bool {
        matches!(self, EnemyKind::Skeleton | EnemyKind::Helmeted)
    }

    /// Blows of the hero's it takes to fell.
    pub fn toughness(self) -> u8 {
        if self.stage() >= 4 {
            return 3;
        }
        match self {
            EnemyKind::Blemmyes | EnemyKind::Rasetsu => 3,
            EnemyKind::Witch
            | EnemyKind::Helmeted
            | EnemyKind::Werewolf
            | EnemyKind::Tengu
            | EnemyKind::DireWolf
            | EnemyKind::Jorogumo
            | EnemyKind::Cobra => 2,
            _ => 1,
        }
    }

    /// How many ordinary bites its touch is worth.
    pub fn bite(self) -> i32 {
        match self {
            EnemyKind::Blemmyes | EnemyKind::Rasetsu | EnemyKind::Cobra => 2,
            _ if self.stage() >= 6 => 3,
            _ if self.stage() >= 4 => 2,
            _ => 1,
        }
    }
}

/**
 * `Frenzied` is the stunned state. `Scattered` is a slain skeleton lying in
 * pieces before it reassembles.
 */
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyMode {
    Nest,
    Leaving,
    Scatter,
    Chase,
    Frenzied,
    Eaten,
    Scattered,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GlobalMode {
    Scatter,
    Chase,
}

impl GlobalMode {
    pub fn mode(self) -> EnemyMode {
        match self {
            GlobalMode::Scatter => EnemyMode::Scatter,
            GlobalMode::Chase => EnemyMode::Chase,
        }
    }
}

/// What an enemy may know about the hero this tick.
#[derive(Clone, Copy, Debug)]
pub struct HeroView {
    pub pos: V,
    pub dir: Dir,
    pub heard_pos: V,
    pub heard_age: f64,
}

pub struct EnemyContext<'a> {
    pub grid: &'a Grid,
    pub hero: HeroView,
    pub skeleton_pos: Option<V>,
    pub nest_center: P,
    pub nest_exit: P,
    pub global_mode: GlobalMode,
    pub frenzy_active: bool,
    /// The hero is invisible: nobody can sense him, everyone falls back to scattering.
    pub hero_hidden: bool,
    /// 0..1 how enraged the dungeon is (it rises with every key taken): faster, keener hunters.
    pub rage: f64,
    /// A decoy (Loki's trick) the hunters chase in the hero's place.
    pub decoy: Option<V>,
    /// For a foe turned to the hero's side: the nearest foe it hunts (none: it follows the hero).
    pub quarry: Option<V>,
    pub rng: &'a mut Rng,
}

#[derive(Clone, Debug)]
pub struct EnemySpec {
    pub kind: EnemyKind,
    pub name: String,
    pub color: u32,
    pub scatter_target: P,
    pub exit_delay: f64,
    pub speed_factor: f64,
}

/// Seconds a skeleton keeps homing after the hero stops making footsteps.
pub const HEARING_MEMORY: f64 = 1.5;
/// How much faster than usual a fully enraged foe runs.
pub const RAGE_SPEED: f64 = 0.35;
/// Past this much rage every foe hunts the hero himself instead of its own trick.
pub const RAGE_DIRECT: f64 = 0.6;
/// Seconds a slain skeleton lies in pieces before pulling itself back together.
pub const REASSEMBLE_SECONDS: f64 = 3.0;
/// A wounded loup-garou runs this much faster, for this long.
pub const WOLF_FURY_SPEED: f64 = 1.45;
pub const WOLF_FURY_SECONDS: f64 = 4.0;
/// A witch conjures her first minion this long after she starts hunting, then one every so often,
/// as long as the hero is within reach.
pub const WITCH_CAST_FIRST: f64 = 5.0;
pub const WITCH_CAST_EVERY: f64 = 9.0;
pub const WITCH_CAST_RANGE: f64 = 12.0;
/// How often the large bat swerves off its line at a crossing.
const BAT_SWERVE: f64 = 0.25;
/// A foe kept to its platform (the gym) only wanders it, at this share of its pace, and mostly straight on.
const KEEP_PACE: f64 = 0.7;
const KEEP_STRAIGHT: f64 = 0.7;

/// Everything but the mover, so the step callback can think while the mover moves.
#[derive(Clone, Debug)]
pub struct Brain {
    pub kind: EnemyKind,
    pub name: String,
    pub color: u32,
    pub scatter_target: P,
    pub speed_factor: f64,
    pub mode: EnemyMode,
    pub base_speed: f64,
    pub exit_timer: f64,
    pub scatter_timer: f64,
    pub target: V,
    initial_exit_delay: f64,
    /// Seconds left stunned or fleeing in panic, whatever the rest are doing.
    pub dread: f64,
    /// Seconds left wandering at random, still dangerous (Loki's trick).
    pub confused: f64,
    /// Seconds left fighting on the hero's side, turned by Michael's fire (0: a foe).
    pub ally: f64,
    /// Blows it can still take before it falls.
    pub hits: u8,
    /// Seconds of a wounded loup-garou's fury.
    pub fury: f64,
    /// Seconds before a witch conjures again.
    pub cast: f64,
    /// Ground it never leaves (the gym's platforms: only the hero crosses the bridges).
    pub keep: Option<Rect>,
    /// The room it holds, in the Vaults of Mammon: it idles there (`keep`) until the hero comes near,
    /// hunts him while he stays close, and goes back when he is gone.
    pub den: Option<Rect>,
    /// On its way back to its den.
    pub returning: bool,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub m: Mover,
    pub b: Brain,
    /// Where it stands in the gym, and rises again when slain (none: it comes from the hollow).
    pub post: Option<P>,
    /// A duelist of the Sparring Ring: it fights by frame data, walks freely, and its mind is
    /// its own (the grid brain above sits idle).
    pub duel: Option<Box<Duelist>>,
}

impl Enemy {
    pub fn new(spec: &EnemySpec, start: P, base_speed: f64) -> Self {
        let mut m = Mover::new(start, base_speed);
        m.flying = spec.kind.flies();
        Self {
            m,
            post: None,
            duel: None,
            b: Brain {
                kind: spec.kind,
                name: spec.name.clone(),
                color: spec.color,
                scatter_target: spec.scatter_target,
                speed_factor: spec.speed_factor,
                mode: EnemyMode::Nest,
                base_speed,
                exit_timer: spec.exit_delay,
                scatter_timer: 0.0,
                target: spec.scatter_target.v(),
                initial_exit_delay: spec.exit_delay,
                dread: 0.0,
                confused: 0.0,
                ally: 0.0,
                hits: spec.kind.toughness(),
                fury: 0.0,
                cast: WITCH_CAST_FIRST,
                keep: None,
                den: None,
                returning: false,
            },
        }
    }

    /// Sets its pace before its first step (a kept foe's easy pace included), so nothing catches it
    /// at the mover's bare speed.
    pub fn prime(&mut self) {
        self.m.speed = self.b.base_speed * self.b.speed_factor * if self.b.keep.is_some() { KEEP_PACE } else { 1.0 };
    }

    /// Blows it can still take.
    pub fn hits_left(&self) -> u8 {
        self.b.hits
    }

    /// A blow of the hero's lands: it falls, unless it is tough enough to take another (a
    /// loup-garou only grows furious). True when this one fells it.
    pub fn wound(&mut self) -> bool {
        if self.b.hits > 1 {
            self.b.hits -= 1;
            if self.b.kind == EnemyKind::Werewolf {
                self.b.fury = WOLF_FURY_SECONDS;
            }
            return false;
        }
        self.kill();
        true
    }

    pub fn kind(&self) -> EnemyKind {
        self.b.kind
    }
    pub fn mode(&self) -> EnemyMode {
        self.b.mode
    }
    pub fn pos(&self) -> V {
        match &self.duel {
            Some(d) => d.pos,
            None => self.m.pos(),
        }
    }
    pub fn occupied(&self) -> P {
        match &self.duel {
            Some(d) => P::new(d.pos.x.round() as i32, d.pos.y.round() as i32),
            None => self.m.occupied(),
        }
    }

    /// Idling in its den, not yet roused (the Vaults of Mammon).
    pub fn lurking(&self) -> bool {
        self.b.den.is_some() && self.b.keep.is_some()
    }

    /// Roused: it leaves its idling and hunts (nothing changes for a foe without a den).
    pub fn rouse(&mut self) {
        if self.b.den.is_some() {
            self.b.keep = None;
            self.b.returning = false;
            if matches!(self.b.mode, EnemyMode::Scatter | EnemyMode::Chase) {
                self.b.mode = EnemyMode::Chase;
            }
        }
    }

    /// Gives up the hunt and makes for its den; back in it, it idles again.
    pub fn go_home(&mut self) {
        if self.b.den.is_some() && self.b.keep.is_none() {
            self.b.returning = true;
        }
    }

    /// Free-roaming and able to hurt the hero (not one turned to his side).
    pub fn is_dangerous(&self) -> bool {
        matches!(self.b.mode, EnemyMode::Scatter | EnemyMode::Chase) && self.b.ally <= 0.0
    }

    /// Turned by Michael's fire: it hunts the other foes, and nothing of the hero's hurts it.
    pub fn is_ally(&self) -> bool {
        self.b.ally > 0.0
    }

    /// Michael's fire strikes it: for `seconds` it fights on the hero's side. (A duelist's mind is
    /// its own: these brain-level spells pass it by, and the game shakes its poise instead.)
    pub fn turn(&mut self, seconds: f64) {
        if self.duel.is_some() {
            return;
        }
        self.b.ally = seconds;
        self.b.mode = EnemyMode::Chase;
        self.b.dread = 0.0;
        self.b.confused = 0.0;
    }

    /// Stunned: harmless and worth chain points.
    pub fn is_stunned(&self) -> bool {
        self.b.mode == EnemyMode::Frenzied
    }

    /// Can be hit by the sword, a shot, or a blast.
    pub fn is_targetable(&self) -> bool {
        self.is_dangerous() || self.is_stunned()
    }

    /// 0..1 through the bone-pile phase; 0 when not scattered.
    pub fn scatter_progress(&self) -> f64 {
        if self.b.mode == EnemyMode::Scattered {
            1.0 - self.b.scatter_timer.max(0.0) / REASSEMBLE_SECONDS
        } else {
            0.0
        }
    }

    pub fn reset(&mut self, at: P) {
        self.m.teleport(at);
        self.b.mode = EnemyMode::Nest;
        self.b.exit_timer = self.b.initial_exit_delay;
        self.b.scatter_timer = 0.0;
        self.b.hits = self.b.kind.toughness();
        self.b.fury = 0.0;
        self.b.cast = WITCH_CAST_FIRST;
    }

    /// Stands at `at`, whole and on the hunt (as a level begins: no foe lies there in pieces).
    pub fn stand_at(&mut self, at: P) {
        self.m.teleport(at);
        self.b.mode = EnemyMode::Chase;
        self.b.scatter_timer = 0.0;
        self.b.hits = self.b.kind.toughness();
        self.b.fury = 0.0;
        self.b.cast = WITCH_CAST_FIRST;
    }

    /// Climbs up out of the ground at `at`: in pieces at first (as a slain draugr lies), whole after `seconds`.
    pub fn rise_at(&mut self, at: P, seconds: f64) {
        self.m.teleport(at);
        self.b.mode = EnemyMode::Scattered;
        self.b.scatter_timer = seconds;
    }

    /// Enemies in or leaving the nest are unaffected.
    pub fn enter_frenzy(&mut self, grid: &Grid) {
        if !self.is_dangerous() || self.duel.is_some() {
            return;
        }
        self.b.mode = EnemyMode::Frenzied;
        self.m.reverse(grid);
    }

    pub fn exit_frenzy(&mut self, global: GlobalMode) {
        if self.b.mode == EnemyMode::Frenzied && self.b.dread <= 0.0 {
            self.b.mode = global.mode();
        }
    }

    /// Stunned by a blow, or sent fleeing in panic by a kill: harmless and slow for `seconds`.
    pub fn scare(&mut self, grid: &Grid, seconds: f64) {
        if self.duel.is_some() {
            return;
        }
        if self.is_dangerous() {
            self.b.mode = EnemyMode::Frenzied;
            self.m.reverse(grid);
        } else if !self.is_stunned() {
            return;
        }
        self.b.dread = self.b.dread.max(seconds);
    }

    /// Befuddled by a trick: it wanders at random for `seconds`, though it still bites.
    pub fn confuse(&mut self, seconds: f64) {
        if self.is_dangerous() && self.duel.is_none() {
            self.b.confused = self.b.confused.max(seconds);
        }
    }

    pub fn is_confused(&self) -> bool {
        self.b.confused > 0.0 && self.is_dangerous()
    }

    pub fn reverse(&mut self, grid: &Grid) {
        if self.duel.is_some() {
            return;
        }
        self.m.reverse(grid);
    }

    /**
     * Slain by the hero. Skeletons collapse in place and reassemble after a
     * few seconds; everything else returns to the crypt to respawn.
     */
    pub fn kill(&mut self) {
        // Whole again when it comes back.
        self.b.hits = self.b.kind.toughness();
        self.b.fury = 0.0;
        if self.b.kind.reassembles() {
            let at = self.m.occupied();
            self.m.teleport(at);
            self.b.mode = EnemyMode::Scattered;
            self.b.scatter_timer = REASSEMBLE_SECONDS;
            return;
        }
        self.b.mode = EnemyMode::Eaten;
    }

    /// Elevation under the enemy, blended while crossing tiles.
    pub fn elevation_on(&mut self, grid: &Grid) -> f64 {
        self.m.elevation_on(grid)
    }

    pub fn update(&mut self, dt: f64, ctx: &mut EnemyContext) {
        if self.duel.is_some() {
            return; // the Sparring Ring runs its duelists
        }
        let grid = ctx.grid;
        let b = &mut self.b;
        b.confused = (b.confused - dt).max(0.0);
        b.fury = (b.fury - dt).max(0.0);
        if b.dread > 0.0 {
            b.dread -= dt;
            if b.dread <= 0.0 && b.mode == EnemyMode::Frenzied && !ctx.frenzy_active {
                b.mode = ctx.global_mode.mode();
            }
        }

        if b.mode == EnemyMode::Scattered {
            b.scatter_timer -= dt;
            if b.scatter_timer <= 0.0 {
                b.mode = if ctx.frenzy_active { EnemyMode::Frenzied } else { ctx.global_mode.mode() };
            }
            return;
        }
        if b.mode == EnemyMode::Nest {
            b.exit_timer -= dt;
            if b.exit_timer > 0.0 {
                return;
            }
            b.mode = EnemyMode::Leaving;
        }
        if matches!(b.mode, EnemyMode::Scatter | EnemyMode::Chase) {
            // One turned to the hero's side is always on the hunt.
            b.mode = if b.ally > 0.0 { EnemyMode::Chase } else { ctx.global_mode.mode() };
        }

        let own_pos = self.m.pos();
        b.target = b.compute_target(ctx, own_pos);
        let mut mult = b.speed_multiplier(grid, self.m.tile);
        if matches!(b.mode, EnemyMode::Scatter | EnemyMode::Chase) {
            // Enraged, they run up to a third faster.
            mult *= 1.0 + RAGE_SPEED * ctx.rage;
        }
        if b.fury > 0.0 && b.mode == EnemyMode::Chase {
            mult *= WOLF_FURY_SPEED;
        }
        if b.keep.is_some() {
            mult *= KEEP_PACE;
        }
        self.m.speed = b.base_speed * b.speed_factor * mult;
        let allow_nest = matches!(b.mode, EnemyMode::Eaten | EnemyMode::Leaving);
        let flying = self.m.flying;
        // Slimes shun bridges; only a slain one's spirit drifts home over them.
        let avoid_bridges = b.kind == EnemyKind::Slime && b.mode != EnemyMode::Eaten;
        self.m.avoids_bridges = avoid_bridges;
        self.m.step(
            dt,
            grid,
            |tile, current| b.decide_at_center(tile, current, ctx, allow_nest, flying, avoid_bridges, own_pos),
            allow_nest,
            false,
            false,
        );
    }
}

impl Brain {
    /// Runs at each tile center: applies nest transitions, then picks a direction.
    fn decide_at_center(
        &mut self,
        tile: P,
        current: Dir,
        ctx: &mut EnemyContext,
        mut allow_nest: bool,
        flying: bool,
        avoid_bridges: bool,
        own_pos: V,
    ) -> Dir {
        let grid = ctx.grid;
        let mut may_reverse = false;

        // Home again: it settles back to idling in its den.
        if self.returning && self.den.is_some_and(|d| d.contains(tile)) {
            self.returning = false;
            self.keep = self.den;
        }
        if self.mode == EnemyMode::Leaving && tile == ctx.nest_exit {
            self.mode = if ctx.frenzy_active { EnemyMode::Frenzied } else { ctx.global_mode.mode() };
            self.target = self.compute_target(ctx, own_pos);
            allow_nest = false;
        } else if self.mode == EnemyMode::Eaten && grid.get(tile.x, tile.y) == Tile::Nest {
            self.mode = EnemyMode::Leaving;
            self.target = ctx.nest_exit.v();
            may_reverse = true;
        }

        // Going home is the one trip that must always succeed, so it is routed
        // rather than eyeballed: the hollow has a single mouth.
        if self.mode == EnemyMode::Eaten {
            let goal = P::new(self.target.x as i32, self.target.y as i32);
            if let Some(step) = step_toward(grid, tile, goal, flying, true) {
                return step;
            }
        }

        let mut options: Vec<Dir> = CARDINALS
            .iter()
            .copied()
            .filter(|&d| grid.can_move(tile, d, allow_nest, flying))
            .filter(|&d| {
                let n = grid.neighbor(tile, d);
                !(avoid_bridges && grid.is_bridge(n.x, n.y)) && self.keep.is_none_or(|k| k.contains(n))
            })
            .collect();
        // Kept to a platform, it does not hunt: it wanders, keeping on mostly, turning now and then.
        if self.keep.is_some() && matches!(self.mode, EnemyMode::Chase | EnemyMode::Scatter) {
            if options.contains(&current) && ctx.rng.next() < KEEP_STRAIGHT {
                return current;
            }
            let ahead: Vec<Dir> = options.iter().copied().filter(|&d| d != current.opposite()).collect();
            let pick = if ahead.is_empty() { &options } else { &ahead };
            return if pick.is_empty() { Dir::None } else { ctx.rng.pick(pick) };
        }
        // Hunting (and leaving the hollow), they take the shortest way through the maze, turning
        // back if that is the way: greedy steps toward the target only circle the blocks.
        if matches!(self.mode, EnemyMode::Chase | EnemyMode::Scatter | EnemyMode::Leaving) && self.confused <= 0.0 {
            if let Some(step) = route_toward(grid, tile, self.target, flying, allow_nest, avoid_bridges) {
                if options.contains(&step) {
                    return step;
                }
            }
        }
        if !may_reverse {
            let no_reverse: Vec<Dir> = options.iter().copied().filter(|&d| d != current.opposite()).collect();
            if !no_reverse.is_empty() {
                options = no_reverse;
            }
        }
        if !allow_nest {
            options.retain(|&d| {
                let n = grid.neighbor(tile, d);
                grid.get(n.x, n.y) != Tile::Nest
            });
        }
        if options.is_empty() {
            // Nowhere to go within its keep: it stands (elsewhere, it keeps on, into a wall or a turn).
            return if self.keep.is_some() { Dir::None } else { current };
        }
        if self.mode == EnemyMode::Frenzied || self.confused > 0.0 {
            return ctx.rng.pick(&options);
        }
        // The large bat swerves: now and then it takes any way at all.
        if self.kind == EnemyKind::LargeBat && self.mode == EnemyMode::Chase && ctx.rng.next() < BAT_SWERVE {
            return ctx.rng.pick(&options);
        }

        let mut best = options[0];
        let mut best_d = f64::INFINITY;
        for &d in &options {
            let n = grid.neighbor(tile, d);
            let dist = dist_sq(n.v(), self.target);
            if dist < best_d {
                best_d = dist;
                best = d;
            }
        }
        best
    }

    fn speed_multiplier(&self, grid: &Grid, tile: P) -> f64 {
        match self.mode {
            EnemyMode::Eaten => 1.7,
            EnemyMode::Frenzied => 0.55,
            EnemyMode::Leaving => 0.7,
            _ if grid.get(tile.x, tile.y) == Tile::Tunnel => 0.55,
            _ => 1.0,
        }
    }

    fn compute_target(&self, ctx: &EnemyContext, own_pos: V) -> V {
        match self.mode {
            EnemyMode::Eaten => return ctx.nest_center.v(),
            EnemyMode::Leaving => return ctx.nest_exit.v(),
            EnemyMode::Chase => {}
            _ => return self.scatter_target.v(),
        }
        if self.ally > 0.0 {
            return ctx.quarry.unwrap_or(ctx.hero.pos);
        }
        if self.returning {
            if let Some(d) = self.den {
                let (cx, cy) = d.center();
                return V::new(cx as f64, cy as f64);
            }
        }
        if ctx.hero_hidden {
            return self.scatter_target.v();
        }
        if let Some(d) = ctx.decoy {
            // A decoy draws every hunter, the eyeless ones by its noise.
            return d;
        }
        let hp = ctx.hero.pos;
        if ctx.rage >= RAGE_DIRECT && !self.kind.reassembles() {
            // Enraged: no more flanking or ambushes, straight for him.
            return hp;
        }
        match self.kind {
            // Eyeless but sharp-eared: makes for the last place it heard footsteps, and searches on from there.
            EnemyKind::Skeleton | EnemyKind::Helmeted => ctx.hero.heard_pos,
            // Straight for him.
            EnemyKind::Minion
            | EnemyKind::Werewolf
            | EnemyKind::Blemmyes
            | EnemyKind::Tengu
            | EnemyKind::Rasetsu
            | EnemyKind::DireWolf
            | EnemyKind::Cobra
            | EnemyKind::Kappa
            | EnemyKind::Rokurokubi
            | EnemyKind::Oni
            | EnemyKind::Tulukus
            | EnemyKind::Albasti
            | EnemyKind::Karakoncolos
            | EnemyKind::Tepegoz
            | EnemyKind::Gashadokuro
            | EnemyKind::Ejder
            | EnemyKind::FrostGiant
            | EnemyKind::Surtr
            | EnemyKind::Nidhogg
            | EnemyKind::Peerless
            | EnemyKind::Ganglot
            | EnemyKind::Gaki
            | EnemyKind::Ellepige => hp,
            // Swoops at where he is heading, but short of it, and swerves on the way.
            EnemyKind::LargeBat => {
                let v = ctx.hero.dir.vecf();
                V::new(hp.x + v.x * 3.0, hp.y + v.y * 3.0)
            }
            // Keeps her distance: comes on when far, backs off to her corner when close.
            EnemyKind::Witch
            | EnemyKind::Jorogumo
            | EnemyKind::Kitsune
            | EnemyKind::Cin
            | EnemyKind::Arcura
            | EnemyKind::Goryo
            | EnemyKind::Hel => {
                if manhattan(own_pos, hp) > 7.0 {
                    hp
                } else {
                    self.scatter_target.v()
                }
            }
            // Swoops at where the hero is heading.
            EnemyKind::Bat => {
                let v = ctx.hero.dir.vecf();
                V::new(hp.x + v.x * 4.0, hp.y + v.y * 4.0)
            }
            // Flanks: mirrors the skeleton around a point ahead of the hero.
            EnemyKind::Vampire => {
                let anchor = ctx.skeleton_pos.unwrap_or(own_pos);
                let v = ctx.hero.dir.vecf();
                let ahead = V::new(hp.x + v.x * 2.0, hp.y + v.y * 2.0);
                V::new(ahead.x + (ahead.x - anchor.x), ahead.y + (ahead.y - anchor.y))
            }
            // Oozes toward the hero when far, slinks back to its corner when close.
            EnemyKind::Slime => {
                if manhattan(own_pos, hp) > 8.0 {
                    hp
                } else {
                    self.scatter_target.v()
                }
            }
        }
    }
}

/// The foes of a stage: the Garden's four, the realms' six, the third stage's five, or a later one's.
pub fn enemy_specs(grid: &Grid, stage: u8) -> Vec<EnemySpec> {
    if stage <= 1 {
        return default_enemy_specs(grid);
    }
    if stage == 3 {
        return third_stage_specs(grid);
    }
    if stage >= 4 {
        return later_stage_specs(grid, stage);
    }
    let w = grid.width - 1;
    let h = grid.height - 1;
    vec![
        EnemySpec {
            kind: EnemyKind::Helmeted,
            name: "Helmeted Skeleton".into(),
            color: 0xff5a4a,
            scatter_target: P::new(w - 2, 0),
            exit_delay: 0.0,
            speed_factor: 1.05,
        },
        EnemySpec {
            kind: EnemyKind::LargeBat,
            name: "Large Bat".into(),
            color: 0xc04a70,
            scatter_target: P::new(2, 0),
            exit_delay: 2.0,
            speed_factor: 1.2,
        },
        EnemySpec {
            kind: EnemyKind::Witch,
            name: "Witch".into(),
            color: 0xb066ff,
            scatter_target: P::new(w, h),
            exit_delay: 4.0,
            speed_factor: 0.95,
        },
        EnemySpec {
            kind: EnemyKind::Minion,
            name: "Witch Minion".into(),
            color: 0xd08cff,
            scatter_target: P::new(0, h),
            exit_delay: 5.5,
            speed_factor: 1.25,
        },
        EnemySpec {
            kind: EnemyKind::Werewolf,
            name: "Loup-Garou".into(),
            color: 0xc0885a,
            scatter_target: P::new(w, h / 2),
            exit_delay: 7.0,
            speed_factor: 1.3,
        },
        EnemySpec {
            kind: EnemyKind::Blemmyes,
            name: "Blemmyes".into(),
            color: 0xe8b494,
            scatter_target: P::new(0, h / 2),
            exit_delay: 9.0,
            speed_factor: 0.72,
        },
    ]
}

/// The Garden's foes (the first stage).
pub fn default_enemy_specs(grid: &Grid) -> Vec<EnemySpec> {
    let w = grid.width - 1;
    let h = grid.height - 1;
    vec![
        EnemySpec {
            kind: EnemyKind::Skeleton,
            name: "Draugr".into(),
            color: 0xe9e9f2,
            scatter_target: P::new(w - 2, 0),
            exit_delay: 0.0,
            speed_factor: 1.0,
        },
        EnemySpec {
            kind: EnemyKind::Bat,
            name: "Raven".into(),
            color: 0x9a5cff,
            scatter_target: P::new(2, 0),
            exit_delay: 2.5,
            speed_factor: 1.12,
        },
        EnemySpec {
            kind: EnemyKind::Vampire,
            name: "Night Raven".into(),
            color: 0x4f6dff,
            scatter_target: P::new(w, h),
            exit_delay: 5.0,
            speed_factor: 1.05,
        },
        EnemySpec {
            kind: EnemyKind::Slime,
            name: "Bog Slime".into(),
            color: 0x5cff6a,
            scatter_target: P::new(0, h),
            exit_delay: 7.5,
            speed_factor: 0.8,
        },
    ]
}

/// The third stage's foes.
pub fn third_stage_specs(grid: &Grid) -> Vec<EnemySpec> {
    let w = grid.width - 1;
    let h = grid.height - 1;
    vec![
        EnemySpec {
            kind: EnemyKind::Tengu,
            name: "Tengu".into(),
            color: 0xe8503e,
            scatter_target: P::new(w - 2, 0),
            exit_delay: 0.0,
            speed_factor: 1.15,
        },
        EnemySpec {
            kind: EnemyKind::Rasetsu,
            name: "Rasetsu".into(),
            color: 0xd0402e,
            scatter_target: P::new(2, 0),
            exit_delay: 2.0,
            speed_factor: 0.8,
        },
        EnemySpec {
            kind: EnemyKind::DireWolf,
            name: "Dire Wolf".into(),
            color: 0xa4acb8,
            scatter_target: P::new(w, h),
            exit_delay: 4.0,
            speed_factor: 1.35,
        },
        EnemySpec {
            kind: EnemyKind::Jorogumo,
            name: "Jorogumo".into(),
            color: 0xff8a3a,
            scatter_target: P::new(0, h),
            exit_delay: 5.5,
            speed_factor: 0.95,
        },
        EnemySpec {
            kind: EnemyKind::Cobra,
            name: "Cobra".into(),
            color: 0x9ac84a,
            scatter_target: P::new(w, h / 2),
            exit_delay: 7.0,
            speed_factor: 0.85,
        },
    ]
}

/// The foes of stages four to seven: each kind's name, colour and pace (their scatter corners and
/// exits in turn, as the earlier casts have theirs).
pub fn later_stage_specs(grid: &Grid, stage: u8) -> Vec<EnemySpec> {
    let w = grid.width - 1;
    let h = grid.height - 1;
    let corners = [P::new(w - 2, 0), P::new(2, 0), P::new(w, h), P::new(0, h), P::new(w, h / 2)];
    EnemyKind::of_stage(stage)
        .iter()
        .enumerate()
        .map(|(i, &kind)| {
            let (name, color, speed_factor) = match kind {
                EnemyKind::Oni => ("Oni", 0xd8402c, 0.8),
                EnemyKind::Kappa => ("Kappa", 0x5ab07a, 1.1),
                EnemyKind::Rokurokubi => ("Rokurokubi", 0xe8d8e8, 0.85),
                EnemyKind::Kitsune => ("Kitsune", 0xff8a3a, 1.25),
                EnemyKind::Karakoncolos => ("Karakoncolos", 0x8ec8ff, 0.95),
                EnemyKind::Tulukus => ("Tülükuş", 0xd09a5a, 1.2),
                EnemyKind::Albasti => ("Albastı", 0xc0283a, 1.1),
                EnemyKind::Cin => ("Cin", 0xffe6a0, 0.95),
                EnemyKind::Arcura => ("Arçura", 0x6ad050, 0.9),
                EnemyKind::Tepegoz => ("Tepegöz", 0xe8a070, 0.75),
                EnemyKind::Gashadokuro => ("Gashadokuro", 0xdcd8b8, 0.75),
                EnemyKind::Ejder => ("Yedi Başlı Ejder", 0x7ac04a, 0.8),
                EnemyKind::Goryo => ("Goryō", 0x8ab0ff, 0.95),
                EnemyKind::FrostGiant => ("Frost Giant", 0xa8dcff, 0.75),
                EnemyKind::Surtr => ("Surtr", 0xff6a1a, 0.75),
                EnemyKind::Nidhogg => ("Nidhogg", 0x8ad04a, 0.85),
                EnemyKind::Hel => ("Hel", 0x7aa0e8, 0.9),
                _ => unreachable!("an earlier stage's kind"),
            };
            EnemySpec {
                kind,
                name: name.into(),
                color,
                scatter_target: corners[i % corners.len()],
                exit_delay: i as f64 * 2.0,
                speed_factor,
            }
        })
        .collect()
}

/// The spec of any kind, from its stage's (or the realms' own).
pub fn spec_of(grid: &Grid, kind: EnemyKind) -> EnemySpec {
    if let Some(spec) = realm_foe_spec(kind) {
        return spec;
    }
    enemy_specs(grid, kind.stage()).into_iter().find(|s| s.kind == kind).expect("every kind has its stage's spec")
}

/// The realms' own foes: each kind's name, colour and pace (none for any other kind).
pub fn realm_foe_spec(kind: EnemyKind) -> Option<EnemySpec> {
    let (name, color, speed_factor) = match kind {
        EnemyKind::Peerless => ("The Peerless", 0xd8c8ff, 1.0),
        EnemyKind::Ganglot => ("Ganglöt", 0x9aa4b0, 0.45),
        EnemyKind::Gaki => ("Gaki", 0xc8b49a, 1.05),
        EnemyKind::Ellepige => ("Ellepige", 0xf0e0ea, 1.0),
        _ => return None,
    };
    Some(EnemySpec { kind, name: name.into(), color, scatter_target: P::new(0, 0), exit_delay: 0.0, speed_factor })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::level::parse_level;
    use crate::logic::levels::level_for;

    #[test]
    fn slain_enemies_find_their_way_home_and_out() {
        let level = parse_level(&level_for(0));
        let specs = default_enemy_specs(&level.grid);
        let mut bat = Enemy::new(&specs[1], level.hero_spawn, 4.32);
        bat.b.mode = EnemyMode::Eaten;
        let mut rng = Rng::new(3);
        let mut left_home = false;
        for _ in 0..(120 * 30) {
            let mut ctx = EnemyContext {
                grid: &level.grid,
                hero: HeroView { pos: V::new(1.0, 1.0), dir: Dir::None, heard_pos: V::new(1.0, 1.0), heard_age: 9.0 },
                skeleton_pos: None,
                nest_center: level.nest_center,
                nest_exit: level.enemy_spawn,
                global_mode: GlobalMode::Scatter,
                frenzy_active: false,
                hero_hidden: false,
                rage: 0.0,
                decoy: None,
                quarry: None,
                rng: &mut rng,
            };
            bat.update(1.0 / 120.0, &mut ctx);
            if bat.mode() == EnemyMode::Scatter {
                left_home = true;
                break;
            }
        }
        assert!(left_home, "the bat never made it back out of the hollow");
    }

    #[test]
    fn skeletons_scatter_then_reassemble() {
        let level = parse_level(&level_for(0));
        let specs = default_enemy_specs(&level.grid);
        let mut s = Enemy::new(&specs[0], level.enemy_spawn, 4.32);
        s.b.mode = EnemyMode::Chase;
        s.kill();
        assert_eq!(s.mode(), EnemyMode::Scattered);
        assert!(!s.is_targetable());
    }
}
