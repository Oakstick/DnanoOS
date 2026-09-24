use super::grid::Grid;
use super::mover::Mover;
use super::pathing::step_toward;
use super::rng::Rng;
use super::types::{dist_sq, manhattan, Dir, Tile, CARDINALS, P, V};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EnemyKind {
    Skeleton,
    Bat,
    Vampire,
    Slime,
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
/// Seconds a slain skeleton lies in pieces before pulling itself back together.
pub const REASSEMBLE_SECONDS: f64 = 3.0;

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
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub m: Mover,
    pub b: Brain,
}

impl Enemy {
    pub fn new(spec: &EnemySpec, start: P, base_speed: f64) -> Self {
        let mut m = Mover::new(start, base_speed);
        m.flying = matches!(spec.kind, EnemyKind::Bat | EnemyKind::Vampire);
        Self {
            m,
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
            },
        }
    }

    pub fn kind(&self) -> EnemyKind {
        self.b.kind
    }
    pub fn mode(&self) -> EnemyMode {
        self.b.mode
    }
    pub fn pos(&self) -> V {
        self.m.pos()
    }
    pub fn occupied(&self) -> P {
        self.m.occupied()
    }

    /// Free-roaming and able to hurt the hero.
    pub fn is_dangerous(&self) -> bool {
        matches!(self.b.mode, EnemyMode::Scatter | EnemyMode::Chase)
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
    }

    /// Enemies in or leaving the nest are unaffected.
    pub fn enter_frenzy(&mut self, grid: &Grid) {
        if !self.is_dangerous() {
            return;
        }
        self.b.mode = EnemyMode::Frenzied;
        self.m.reverse(grid);
    }

    pub fn exit_frenzy(&mut self, global: GlobalMode) {
        if self.b.mode == EnemyMode::Frenzied {
            self.b.mode = global.mode();
        }
    }

    pub fn reverse(&mut self, grid: &Grid) {
        self.m.reverse(grid);
    }

    /**
     * Slain by the hero. Skeletons collapse in place and reassemble after a
     * few seconds; everything else returns to the crypt to respawn.
     */
    pub fn kill(&mut self) {
        if self.b.kind == EnemyKind::Skeleton {
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
        let grid = ctx.grid;
        let b = &mut self.b;

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
            b.mode = ctx.global_mode.mode();
        }

        let own_pos = self.m.pos();
        b.target = b.compute_target(ctx, own_pos);
        let mult = b.speed_multiplier(grid, self.m.tile);
        self.m.speed = b.base_speed * b.speed_factor * mult;
        let allow_nest = matches!(b.mode, EnemyMode::Eaten | EnemyMode::Leaving);
        let flying = self.m.flying;
        self.m.step(
            dt,
            grid,
            |tile, current| b.decide_at_center(tile, current, ctx, allow_nest, flying, own_pos),
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
        own_pos: V,
    ) -> Dir {
        let grid = ctx.grid;
        let mut may_reverse = false;

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

        let mut options: Vec<Dir> =
            CARDINALS.iter().copied().filter(|&d| grid.can_move(tile, d, allow_nest, flying)).collect();
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
            return current;
        }
        if self.mode == EnemyMode::Frenzied {
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
        if ctx.hero_hidden {
            return self.scatter_target.v();
        }
        let hp = ctx.hero.pos;
        match self.kind {
            // Eyeless but sharp-eared: homes on the last place it heard footsteps.
            EnemyKind::Skeleton => {
                if ctx.hero.heard_age < HEARING_MEMORY {
                    ctx.hero.heard_pos
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

pub fn default_enemy_specs(grid: &Grid) -> Vec<EnemySpec> {
    let w = grid.width - 1;
    let h = grid.height - 1;
    vec![
        EnemySpec {
            kind: EnemyKind::Skeleton, name: "Skeleton".into(), color: 0xe9e9f2,
            scatter_target: P::new(w - 2, 0), exit_delay: 0.0, speed_factor: 1.0,
        },
        EnemySpec {
            kind: EnemyKind::Bat, name: "Bat".into(), color: 0x9a5cff,
            scatter_target: P::new(2, 0), exit_delay: 2.5, speed_factor: 1.12,
        },
        EnemySpec {
            kind: EnemyKind::Vampire, name: "Vampire Bat".into(), color: 0x4f6dff,
            scatter_target: P::new(w, h), exit_delay: 5.0, speed_factor: 1.05,
        },
        EnemySpec {
            kind: EnemyKind::Slime, name: "Slime".into(), color: 0x5cff6a,
            scatter_target: P::new(0, h), exit_delay: 7.5, speed_factor: 0.8,
        },
    ]
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
