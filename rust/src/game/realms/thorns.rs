//! The Garden of Thorns in play (Lust's realm, REALMS.md §6; laid out in `logic::realms::thorns`).
//!
//! - **Charm** fills while he faces the Bride (within 45 degrees of her bearing) and in the perfume
//!   mist, and drains otherwise; full, it leaves him Enthralled for three seconds, his controls
//!   mirrored and drawn toward her. She stands always ahead on the rose path he walks, so walking
//!   one is facing her; the rose paths heal him for it. The thorn paths bite and slow him, and
//!   walking them turns his back on her.
//! - **Tobit's cure:** a koi caught in a pond and burnt at a censer makes ten seconds of smoke that
//!   blocks charm, turns the facades to glass and lights the honest paths; the golden koi's gall
//!   gives twenty seconds of true sight. Raphael's beams burn straight paths through the hedges.
//! - **The rest of the garden:** the bowers give up the dead grooms' last gear; a karakoncolos's
//!   call draws him toward her rather than to itself; a ring of dancing elf-maids drags him in.
//! - **The wedding night:** at the Black Thorn the gate grinds open over ten seconds and the Bride
//!   walks up behind him. Facing her fills charm three times as fast; if she takes him he is her
//!   eighth groom (a heavy wound and Doom) and wakes in the innermost bower. He survives by keeping
//!   his back to her until the gate is open, or by leaping through her and striking her hollow back
//!   three times, which unmakes her and opens the gate at once.

use super::super::{Banner, Game};
use super::Meter;
use crate::audio::Sfx;
use crate::logic::angels::Archangel;
use crate::logic::combat::ails::DOOM_FRAMES;
use crate::logic::combat::math;
use crate::logic::combat::Phase;
use crate::logic::enemy::EnemyKind;
use crate::logic::gear;
use crate::logic::items::{DiamondKind, ItemKind};
use crate::logic::progression;
use crate::logic::realms::thorns::*;
use crate::logic::realms::{Pack, Run};
use crate::logic::types::{Dir, Tile, P, V};
use crate::logic::vault::DropItem;

/// The realm's rose.
const ROSE: u32 = 0xff4a9a;

/// The Bride, seen from `hero` at `bride`: the way to her.
fn toward(hero: V, bride: V) -> V {
    math::norm(math::sub(bride, hero))
}

impl Game {
    fn thorns(&self) -> Option<&Thorns> {
        match self.run.as_ref() {
            Some(Run::Lust(t)) => Some(t),
            _ => None,
        }
    }

    fn thorns_mut(&mut self) -> Option<&mut Thorns> {
        match self.run.as_mut() {
            Some(Run::Lust(t)) => Some(t),
            _ => None,
        }
    }

    /// As the realm is entered (its packs raised, the camera above).
    pub(in crate::game) fn enter_thorns(&mut self) {
        self.gate_open = false;
    }

    /// The realm's own rules each tick.
    pub(in crate::game) fn update_thorns(&mut self, dt: f64) {
        if self.thorns().is_none() || !self.hero.alive {
            return;
        }
        self.thorns_ground(dt);
        if !self.hero.alive {
            return;
        }
        self.thorns_raphael();
        self.thorns_cure(dt);
        self.thorns_bowers();
        self.thorns_pulls(dt);
        let wedding = self.thorns().is_some_and(|t| t.wedding.is_some());
        if !wedding {
            self.thorns_place_bride(dt);
        }
        self.thorns_charm(dt);
        if wedding {
            self.thorns_wedding(dt);
        } else {
            self.thorns_begin_wedding();
        }
    }

    /// Underfoot: the thorns bite and slow him, the roses heal him.
    fn thorns_ground(&mut self, dt: f64) {
        let tile = self.hero.occupied();
        let air = self.hero.airborne();
        let (hurt, healed) = {
            let full = self.hero.hp >= self.hero.max_hp;
            let Some(t) = self.thorns_mut() else { return };
            let g = if air { Ground::Glade } else { t.ground_at(tile) };
            let mut hurt = 0.0;
            let mut healed = 0;
            if g == Ground::Thorn {
                t.thorn_carry += THORN_BITE * dt;
                if t.thorn_carry >= THORN_BITE - 1e-6 {
                    t.thorn_carry -= THORN_BITE;
                    hurt = THORN_BITE;
                }
            }
            if g == Ground::Rose && !full {
                t.rose_carry += ROSE_HEAL * dt;
                if t.rose_carry >= 1.0 - 1e-6 {
                    t.rose_carry -= 1.0;
                    healed = 1;
                }
            }
            self.hero.pace = if g == Ground::Thorn { THORN_PACE } else { 1.0 };
            (hurt, healed)
        };
        if healed > 0 {
            self.hero.heal(healed);
            if self.hero.hp % 4 == 0 {
                let p = self.hero.pos();
                self.view.sparkle(&self.level.grid, p.x, p.y, 0.5, 0xff9ac8, 3);
            }
        }
        if hurt > 0.0 {
            self.wound_hero(hurt, 0x8a2a3a);
        }
    }

    /// Raphael's beam (his blessing spent, or his power as patron called) burns a straight path
    /// through every hedge it crosses, for good.
    fn thorns_raphael(&mut self) {
        let raphael = self.blessing.filter(|b| b.angel == Archangel::Raphael).map(|b| b.charges).unwrap_or(0);
        let replaced = self.blessing.is_some_and(|b| b.angel != Archangel::Raphael);
        let patron = self.progress.hero(self.hero_kind).patron == Some(Archangel::Raphael);
        let wait = self.patron_wait;
        let Some(t) = self.thorns_mut() else { return };
        let cast = (raphael < t.raphael_seen && !replaced) || (patron && wait > t.patron_seen + 1.0);
        t.raphael_seen = raphael;
        t.patron_seen = wait;
        if cast {
            let (from, aim) = (self.hero.pos(), self.hero.aim());
            self.burn_path(from, aim);
        }
    }

    /// Burns the hedge along a beam from `from` along `aim`: every hedge tile within its width.
    pub(in crate::game) fn burn_path(&mut self, from: V, aim: V) {
        let reach = crate::logic::angels::BEAM_REACH;
        let mut burnt = Vec::new();
        {
            let Some(t) = self.thorns_mut() else { return };
            let mut d = 0.5;
            while d <= reach {
                let at = V::new(from.x + aim.x * d, from.y + aim.y * d);
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        let p = P::new(at.x.round() as i32 + dx, at.y.round() as i32 + dy);
                        let off = math::len(math::sub(p.v(), at));
                        if off > 0.72 || burnt.contains(&p) {
                            continue;
                        }
                        let facade = t.facades.contains(&p);
                        if t.burnable(p) || facade {
                            burnt.push(p);
                        }
                    }
                }
                d += 0.25;
            }
            for p in &burnt {
                t.ground[(p.y * W + p.x) as usize] = Ground::Ash;
                t.facades.remove(p);
                t.burnt.push(*p);
            }
        }
        if burnt.is_empty() {
            return;
        }
        for p in &burnt {
            self.level.grid.set(p.x, p.y, Tile::Open);
            if burnt.len() < 40 || (p.x + p.y) % 3 == 0 {
                self.view.dust(&self.level.grid, p.x as f64, p.y as f64, 0xffb070, 4, 1.6);
            }
        }
        let hp = self.hero.pos();
        self.floater(hp.x, hp.y - 1.6, "RAPHAEL BURNS A WAY THROUGH THE HEDGE", 1.8, Some(Archangel::Raphael.color()));
    }

    /// Tobit's cure: koi caught in the ponds, burnt at the censers; the smoke and the true sight wane.
    fn thorns_cure(&mut self, dt: f64) {
        let tile = self.hero.occupied();
        let hp = self.hero.pos();
        let air = self.hero.airborne();
        enum Got {
            Nothing,
            Koi(u32),
            Golden,
            Smoke(V),
            Hint(&'static str),
        }
        let got = {
            let Some(t) = self.thorns_mut() else { return };
            for pd in t.ponds.iter_mut() {
                pd.empty = (pd.empty - dt).max(0.0);
            }
            if let Some(s) = t.smoke.as_mut() {
                s.left -= dt;
            }
            if t.smoke.is_some_and(|s| s.left <= 0.0) {
                t.smoke = None;
            }
            t.sight = (t.sight - dt).max(0.0);
            let mut got = Got::Nothing;
            match t.pond_at(tile).filter(|_| !air) {
                Some(i) if t.ponds[i].empty <= 0.0 && (t.koi < KOI_MAX || (t.ponds[i].golden && !t.golden_taken)) => {
                    t.fishing += dt;
                    if t.fishing >= KOI_CATCH {
                        t.fishing = 0.0;
                        t.ponds[i].empty = KOI_RESTOCK;
                        if t.ponds[i].golden && !t.golden_taken {
                            t.golden_taken = true;
                            t.sight = SIGHT_SECONDS;
                            got = Got::Golden;
                        } else {
                            t.koi += 1;
                            got = Got::Koi(t.koi);
                        }
                    } else if t.hints & 1 == 0 {
                        t.hints |= 1;
                        got = Got::Hint("STAND STILL IN THE POND TO CATCH A KOI");
                    }
                }
                _ => t.fishing = 0.0,
            }
            if let Some(c) = t.censer_near(hp) {
                if t.koi > 0 && t.smoke.is_none() {
                    t.koi -= 1;
                    t.hints |= 2;
                    let at = t.censers[c].v();
                    t.smoke = Some(Smoke { at, left: SMOKE_SECONDS });
                    got = Got::Smoke(at);
                } else if t.koi == 0 && t.hints & 2 == 0 {
                    t.hints |= 2;
                    got = Got::Hint("A CENSER: BURN A KOI HERE, AS TOBIAS BURNT THE FISH");
                }
            }
            got
        };
        match got {
            Got::Nothing => {}
            Got::Hint(text) => self.floater(hp.x, hp.y - 1.2, text, 2.4, Some(0xf0d8e8)),
            Got::Koi(n) => {
                self.audio.play(Sfx::Drip);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.3, 0xff8a3a, 14);
                self.floater(hp.x, hp.y - 1.0, format!("A KOI ({n})"), 1.2, Some(0xff9a4a));
            }
            Got::Golden => {
                self.audio.play(Sfx::Milestone);
                self.view.sparkle(&self.level.grid, hp.x, hp.y, 0.6, 0xffd84a, 40);
                self.juice.banner = Some(Banner {
                    text: "THE GOLDEN KOI'S GALL".into(),
                    sub: "TRUE SIGHT: SEE HER AS SHE IS".into(),
                    color: 0xffd84a,
                    life: 2.6,
                    max: 2.6,
                });
            }
            Got::Smoke(at) => {
                self.audio.play(Sfx::Whoosh);
                self.view.dust(&self.level.grid, at.x, at.y, 0xd8d0e0, 30, 1.4);
                self.floater(at.x, at.y - 1.2, "TOBIT'S SMOKE: THE FACADES TURN TO GLASS", 2.0, Some(0xe8e0f0));
            }
        }
    }

    /// A bridal bower walked into: the dead groom's last gear is his.
    fn thorns_bowers(&mut self) {
        if self.hero.airborne() {
            return;
        }
        let tile = self.hero.occupied();
        let Some(t) = self.thorns_mut() else { return };
        let Some(b) = t.bowers.iter_mut().find(|b| b.at == tile && !b.taken) else { return };
        b.taken = true;
        let (item, rarity) = (b.item, b.rarity);
        let (x, y) = (tile.x as f64, tile.y as f64);
        self.view.sparkle(&self.level.grid, x, y, 0.6, rarity.color(), 24);
        match item {
            DropItem::Potion(kind) => self.take_potion(kind, tile),
            DropItem::Weapon(w) => self.hero.equip(w),
            DropItem::Boon(ItemKind::Shield) => self.hero.grant_shield(),
            DropItem::Boon(ItemKind::Invincible) => self.hero.grant_invincibility(),
            DropItem::Boon(ItemKind::Speed) => self.hero.grant_speed(),
            DropItem::Runes(n) => {
                self.progress.runes += n;
                self.floater(x, y - 0.4, progression::gems(self.hero_kind, n), 1.6, Some(0x9ad1ff));
            }
            DropItem::Bombs(n) => self.bombs_left += n,
        }
        self.audio.play(Sfx::Bells);
        let label = item.label(|p| gear::potion(self.hero_kind, p).name);
        self.floater(x, y - 1.0, format!("A DEAD GROOM'S LAST GEAR: {label}"), 2.2, Some(rarity.color()));
    }

    /// What draws him: a karakoncolos's call (turned toward her) and the rings of dancing elf-maids.
    fn thorns_pulls(&mut self, dt: f64) {
        // A karakoncolos calls him by name in a loved one's voice, and the voice comes from her.
        let caller = self.brawl.as_ref().and_then(|b| b.ails.pull);
        if let Some((id, left)) = caller {
            let karakoncolos =
                self.enemies.iter().any(|e| e.kind() == EnemyKind::Karakoncolos && e.duel.as_ref().is_some_and(|d| d.id == id));
            if karakoncolos {
                if let Some(b) = self.brawl.as_mut() {
                    b.ails.pull = None;
                }
                if let Some(t) = self.thorns_mut() {
                    t.call = left.max(45);
                }
            }
        }
        let hp = self.hero.pos();
        let air = self.hero.airborne();
        let mut shove = V::new(0.0, 0.0);
        let mut hint = false;
        if let Some(t) = self.thorns_mut() {
            if t.call > 0 {
                t.call = t.call.saturating_sub((dt * 60.0).round().max(1.0) as u16);
                shove = math::add(shove, math::scale(toward(hp, t.bride), crate::logic::combat::ails::PULL_SPEED * dt));
            }
            for &c in &t.dances {
                let d = math::len(math::sub(c, hp));
                if d < DANCE_REACH && d > 0.4 {
                    shove = math::add(shove, math::scale(toward(hp, c), DANCE_PULL * dt));
                    if t.hints & 4 == 0 {
                        t.hints |= 4;
                        hint = true;
                    }
                }
            }
        }
        if !air && (shove.x != 0.0 || shove.y != 0.0) {
            self.hero.shove(&self.level.grid, shove);
        }
        if hint {
            self.floater(hp.x, hp.y - 1.3, "THE ELF-KING'S DAUGHTERS DRAW YOU INTO THEIR DANCE", 2.2, Some(0xf0e0ea));
        }
    }

    /// The Bride in the garden: always ahead on the rose path he walks, gone on whenever he comes
    /// near her or turns onto another; on her pavilion while he is in the heart; elsewhere on the
    /// hedge beside the spiral, level with him.
    fn thorns_place_bride(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let tile = self.hero.occupied();
        let Some(t) = self.thorns_mut() else { return };
        t.bride_moved += dt;
        let want = match t.path_at(tile) {
            Some(i) => {
                let ahead = t.paths[i].ahead.vecf();
                let still_ahead = math::dot(math::sub(t.bride, hp), ahead) >= NEAR_HER;
                (t.bride_path != Some(i) || !still_ahead).then(|| (t.focus(i, hp), Some(i)))
            }
            None => {
                let at = t.lure(hp);
                (t.bride_path.is_some() || math::len(math::sub(at, t.bride)) > 5.0).then_some((at, None))
            }
        };
        if let Some((at, path)) = want {
            if math::len(math::sub(at, t.bride)) > 1.5 {
                t.bride_from = t.bride;
                t.bride = at;
                t.bride_moved = 0.0;
            }
            t.bride_path = path;
        }
        // (She is built to face the camera: her lovely front is all the garden shows of her.)
        t.bride_facing = V::new(0.0, 1.0);
    }

    /// Charm: filled by facing her and by the perfume, drained otherwise; full, it enthrals him.
    fn thorns_charm(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let tile = self.hero.occupied();
        let mut enthralled_now = false;
        let mut pull = V::new(0.0, 0.0);
        {
            let Some(t) = self.thorns_mut() else { return };
            if t.enthralled > 0.0 {
                t.enthralled -= dt;
                pull = math::scale(toward(hp, t.bride), ENTHRAL_PULL * dt);
                if t.enthralled <= 0.0 {
                    t.enthralled = 0.0;
                    t.charm = CHARM_AFTER;
                }
            } else {
                let wedding = t.wedding.is_some();
                let blocked = t.smoked(hp) || t.sight > 0.0 || t.unmade;
                // Her hollow back does not charm.
                let faces = facing(hp, aim, t.bride) && !(wedding && t.behind_her(hp));
                let dancing = t.dances.iter().any(|c| math::len(math::sub(*c, hp)) < 1.3);
                let mut rate = 0.0;
                if !blocked {
                    if faces {
                        rate += CHARM_FACING * if wedding { WEDDING_CHARM } else { 1.0 };
                    }
                    if t.mist.contains(&tile) {
                        rate += CHARM_MIST;
                    }
                    if dancing {
                        rate += DANCE_CHARM;
                    }
                }
                if rate == 0.0 {
                    rate = -CHARM_DRAIN;
                }
                t.charm = (t.charm + rate * dt).clamp(0.0, CHARM_MAX);
                if t.charm >= CHARM_MAX {
                    t.enthralled = ENTHRAL_SECONDS;
                    enthralled_now = true;
                }
            }
        }
        if pull.x != 0.0 || pull.y != 0.0 {
            self.hero.shove(&self.level.grid, pull);
        }
        if enthralled_now {
            self.audio.play(Sfx::Bells);
            self.view.shake(0.25);
            self.view.sparkle(&self.level.grid, hp.x, hp.y, 1.0, ROSE, 30);
            self.floater(hp.x, hp.y - 1.5, "ENTHRALLED", 1.6, Some(ROSE));
        }
    }

    /// He reaches the Black Thorn: the wedding night begins.
    fn thorns_begin_wedding(&mut self) {
        if self.gate_open {
            return;
        }
        let hp = self.hero.pos();
        let near = self.thorns().is_some_and(|t| !t.unmade && math::len(math::sub(t.base.exit.front().v(), hp)) <= WEDDING_REACH);
        if near {
            self.begin_wedding();
        }
    }

    /// The gate grinds; the song stops; the Bride leaves her pavilion and walks up behind him.
    fn begin_wedding(&mut self) {
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let tier = self.realm_tier();
        let pack = {
            let Some(t) = self.thorns_mut() else { return };
            let behind = V::new((hp.x - aim.x * 7.0).clamp(1.0, W as f64 - 2.0), (hp.y - aim.y * 7.0).clamp(1.0, H as f64 - 2.0));
            t.bride_from = t.bride;
            t.bride = behind;
            t.bride_moved = 0.0;
            t.bride_path = None;
            t.bride_facing = toward(behind, hp);
            t.enthralled = 0.0;
            let first = !t.brides_raised;
            t.brides_raised = true;
            t.wedding = Some(Wedding { t: 0.0, back_blows: 0, swung: false, passed: false });
            let g = t.thorn_glade;
            // Brides close in from either side.
            first.then(|| Pack {
                den: g,
                foes: vec![(EnemyKind::Ellepige, 0, P::new(g.x0 + 1, g.y1 - 2)), (EnemyKind::Ellepige, 0, P::new(g.x1 - 1, g.y1 - 2))],
            })
        };
        if let (Some(pack), Some(mut b)) = (pack, self.brawl.take()) {
            let mut raised = self.raise_packs(&[pack], tier, &mut b);
            self.brawl = Some(b);
            for e in raised.iter_mut() {
                if let Some(den) = e.duel.as_mut().and_then(|d| d.den.as_mut()) {
                    den.awake = true;
                }
            }
            self.enemies.extend(raised);
            self.refresh_foe_views();
        }
        self.audio.hush();
        self.audio.play(Sfx::DoorGrind);
        self.audio.play(Sfx::Stinger);
        self.view.shake(0.4);
        self.juice.banner = Some(Banner {
            text: "\"TURN AROUND.\"".into(),
            sub: "THE GATE GRINDS OPEN   THE SONG HAS STOPPED   SHE IS BEHIND YOU".into(),
            color: ROSE,
            life: 3.2,
            max: 3.2,
        });
    }

    /// The wedding night, one tick: the gate grinds, she walks up and turns to him; she takes him,
    /// or he outlasts her, or he unmakes her.
    fn thorns_wedding(&mut self, dt: f64) {
        let hp = self.hero.pos();
        let aim = self.hero.aim();
        let air = self.hero.airborne();
        let swinging = self
            .brawl
            .as_ref()
            .and_then(|b| b.hero.attack().map(|(mv, ph, _)| (b.hero.data(mv).projectile, ph)))
            .is_some_and(|(thrown, ph)| ph == Phase::Active && !thrown);
        enum End {
            Going,
            Taken,
            Survived,
            Unmade,
            Blow(u32),
            Face,
        }
        let end = {
            let Some(t) = self.thorns_mut() else { return };
            let Some(mut w) = t.wedding else { return };
            w.t += dt;
            t.bride_moved += dt;
            let to = math::sub(hp, t.bride);
            let dist = math::len(to);
            // She turns toward him, and walks only once she faces him.
            if dist > 1e-3 {
                let want = math::norm(to);
                let (a, b) = (t.bride_facing.y.atan2(t.bride_facing.x), want.y.atan2(want.x));
                let mut d = b - a;
                while d > std::f64::consts::PI {
                    d -= std::f64::consts::TAU;
                }
                while d < -std::f64::consts::PI {
                    d += std::f64::consts::TAU;
                }
                let turn = d.clamp(-BRIDE_TURN * dt, BRIDE_TURN * dt);
                let na = a + turn;
                t.bride_facing = V::new(na.cos(), na.sin());
                if math::dot(t.bride_facing, want) > 0.6 && dist > EMBRACE * 0.85 {
                    let step = (BRIDE_WALK * dt).min(dist - EMBRACE * 0.85);
                    t.bride = math::add(t.bride, math::scale(t.bride_facing, step));
                }
            }
            let dist = math::len(math::sub(hp, t.bride));
            if air && dist < EMBRACE {
                w.passed = true;
            }
            let behind = t.behind_her(hp);
            let mut end = End::Going;
            if !air && dist < EMBRACE && (t.enthralled > 0.0 || (facing(hp, aim, t.bride) && !behind)) {
                end = End::Taken;
            } else if swinging && !w.swung {
                w.swung = true;
                let off = math::dot(math::norm(math::sub(t.bride, hp)), aim);
                if dist <= 2.2 && off >= 0.25 {
                    if behind {
                        w.back_blows += 1;
                        end = if w.back_blows >= BACK_BLOWS { End::Unmade } else { End::Blow(w.back_blows) };
                    } else {
                        end = End::Face;
                    }
                }
            }
            if !swinging {
                w.swung = false;
            }
            if matches!(end, End::Going | End::Face | End::Blow(_)) && w.t >= WEDDING_SECONDS {
                end = End::Survived;
            }
            t.wedding = Some(w);
            end
        };
        match end {
            End::Going => {}
            End::Face => {
                let b = self.thorns().map(|t| t.bride).unwrap_or(hp);
                self.floater(b.x, b.y - 2.6, "NO BLOW TELLS ON HER FACE", 1.0, Some(0xf0e0ea));
                self.audio.play(Sfx::Denied);
            }
            End::Blow(n) => {
                let b = self.thorns().map(|t| t.bride).unwrap_or(hp);
                self.floater(b.x, b.y - 2.6, format!("HER HOLLOW BACK {n}/{BACK_BLOWS}"), 1.2, Some(0xffe0f0));
                self.audio.play(Sfx::Shriek);
                self.view.shake(0.3);
                self.view.dust(&self.level.grid, b.x, b.y, 0x5a3a2a, 18, 2.0);
            }
            End::Taken => self.eighth_groom(),
            End::Survived => self.end_wedding(false),
            End::Unmade => self.end_wedding(true),
        }
    }

    /// The music comes back as it was.
    fn thorns_music(&mut self) {
        self.audio.set_soundtrack(self.level_track);
    }

    /// She has him: a heavy wound and Doom, and he wakes in the innermost bower, her eighth groom.
    fn eighth_groom(&mut self) {
        let inner = {
            let Some(t) = self.thorns_mut() else { return };
            t.wedding = None;
            t.grooms += 1;
            t.charm = 0.0;
            t.enthralled = 0.0;
            t.bride_from = t.bride;
            t.bride = t.home;
            t.bride_path = None;
            t.bride_moved = 0.0;
            t.inner
        };
        let loss = (self.hero.max_hp as f64 * GROOM_WOUND).round().min((self.hero.hp - 1) as f64).max(0.0);
        self.wound_hero(loss, ROSE);
        if let Some(b) = self.brawl.as_mut() {
            b.ails.doom = DOOM_FRAMES;
        }
        self.hero.place(inner);
        self.hero.face(Dir::Up);
        self.view.aim_along(Dir::Up);
        self.thorns_music();
        self.audio.play(Sfx::Shriek);
        self.view.shake(0.7);
        self.juice.fade = 1.0;
        self.juice.banner = Some(Banner {
            text: "THE EIGHTH GROOM".into(),
            sub: "SHE TOOK YOU AS SHE TOOK THE SEVEN   YOU WAKE IN THE INNERMOST BOWER, DOOMED".into(),
            color: ROSE,
            life: 4.0,
            max: 4.0,
        });
    }

    /// The wedding night is over: he outlasted her, or unmade her; the gate stands open.
    fn end_wedding(&mut self, unmade: bool) {
        let b = {
            let Some(t) = self.thorns_mut() else { return };
            t.wedding = None;
            t.enthralled = 0.0;
            t.charm = 0.0;
            t.unmade |= unmade;
            t.bride_from = t.bride;
            if !unmade {
                t.bride = t.home;
            }
            t.bride_path = None;
            t.bride_moved = 0.0;
            t.bride_from
        };
        self.gate_open = true;
        self.thorns_music();
        self.audio.play(Sfx::Win);
        self.view.sparkle(&self.level.grid, b.x, b.y, 2.0, 0xffd0e8, 70);
        self.view.dust(&self.level.grid, b.x, b.y, 0xf4e8ee, 40, 2.4);
        self.juice.banner = Some(if unmade {
            Banner {
                text: "THE BRIDE IS UNMADE".into(),
                sub: "HOLLOW AS A KNEADING TROUGH   THE GATE TO ASMODEUS' COURT OPENS".into(),
                color: 0xffe0f0,
                life: 4.0,
                max: 4.0,
            }
        } else {
            Banner {
                text: "YOU NEVER TURNED AROUND".into(),
                sub: "THE GATE TO ASMODEUS' COURT STANDS OPEN".into(),
                color: 0xffe0f0,
                life: 4.0,
                max: 4.0,
            }
        });
    }

    /// The controls as the realm leaves them (`Game::steer_run`): mirrored while he is enthralled.
    pub(in crate::game) fn steer_thorns(&mut self, wanted: Dir) -> Dir {
        if self.thorns().is_some_and(|t| t.enthralled > 0.0) {
            self.hero.wish = V::new(-self.hero.wish.x, -self.hero.wish.y);
            return wanted.opposite();
        }
        wanted
    }

    /// After a fall, as he rises at the checkpoint (and as the realm is first entered): the spell
    /// broken, the smoke gone, the koi lost, the ponds full, the wedding night undone; what Raphael
    /// burnt and what the bowers gave stay so. Raphael waits on his shrine.
    pub(in crate::game) fn reset_thorns(&mut self) {
        let raphael = self.blessing.filter(|b| b.angel == Archangel::Raphael).map(|b| b.charges).unwrap_or(0);
        let wait = self.patron_wait;
        let mut music = false;
        if let Some(t) = self.thorns_mut() {
            music = t.wedding.is_some();
            t.charm = 0.0;
            t.enthralled = 0.0;
            t.koi = 0;
            t.fishing = 0.0;
            t.smoke = None;
            t.sight = 0.0;
            t.wedding = None;
            t.call = 0;
            t.thorn_carry = 0.0;
            t.rose_carry = 0.0;
            t.golden_taken = false;
            for pd in t.ponds.iter_mut() {
                pd.empty = 0.0;
            }
            t.bride_path = None;
            t.raphael_seen = raphael;
            t.patron_seen = wait;
        }
        self.hero.pace = 1.0;
        for d in self.diamonds.iter_mut().filter(|d| d.kind == DiamondKind::Angel) {
            d.index = Archangel::Raphael.index();
        }
        if music {
            self.thorns_music();
        }
    }

    /// F2: straight to the wedding night, before the gate at the Black Thorn's foot.
    pub(in crate::game) fn cheat_thorns(&mut self) {
        self.cheat_to_far_portal();
        if self.thorns().is_some_and(|t| t.wedding.is_none() && !t.unmade) && !self.gate_open {
            self.begin_wedding();
        }
    }

    /// What the HUD shows of the realm's rules: charm (pulsing past 70 and while enthralled), the
    /// koi he carries, the smoke and true sight while they last, and the gate grinding open.
    pub(in crate::game) fn thorns_meters(&self) -> Vec<Meter> {
        let Some(t) = self.thorns() else { return Vec::new() };
        let mut out = vec![Meter::bar("CHARM", t.charm as f32, CHARM_MAX as f32, ROSE).warning(t.charm >= 70.0 || t.enthralled > 0.0)];
        if t.enthralled > 0.0 {
            out[0].label = "ENTHRALLED".into();
        }
        out.push(Meter::count("KOI", t.koi, 0xff9a4a));
        if let Some(s) = t.smoke {
            out.push(Meter::bar("SMOKE", s.left as f32, SMOKE_SECONDS as f32, 0xd8d0e8));
        }
        if t.sight > 0.0 {
            out.push(Meter::bar("TRUE SIGHT", t.sight as f32, SIGHT_SECONDS as f32, 0xffd84a));
        }
        if let Some(w) = t.wedding {
            out.push(Meter::bar("THE GATE", w.t as f32, WEDDING_SECONDS as f32, 0xb8a0b0));
            if w.back_blows > 0 {
                out.push(Meter::count("HER BACK", w.back_blows, 0xffe0f0));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::super::tests::{in_realm, STEP};
    use super::*;
    use crate::game::Screen;
    use crate::input::Action;
    use crate::logic::sins::Sin;

    fn garden() -> Game {
        let mut g = in_realm(Sin::Lust);
        g.hero.invuln = 0.0;
        g
    }

    fn t(g: &Game) -> &Thorns {
        g.thorns().unwrap()
    }

    fn t_mut(g: &mut Game) -> &mut Thorns {
        g.thorns_mut().unwrap()
    }

    /// Runs the game for `secs`.
    fn run(g: &mut Game, secs: f64) {
        for _ in 0..(secs / STEP).round() as usize {
            g.update(STEP);
        }
    }

    /// A tile of the first ring's west side, about halfway up it.
    fn west_side(g: &Game) -> (P, usize) {
        let (x0, y0, _, y1) = ring_bounds(0);
        let p = P::new(x0, (y0 + y1) / 2);
        (p, t(g).path_at(p).unwrap())
    }

    #[test]
    fn the_garden_is_entered_sealed_with_the_bride_beyond_the_rose_arch() {
        let g = garden();
        assert!(!g.gate_open, "sealed until the wedding night is survived");
        let th = t(&g);
        assert_eq!(g.hero.occupied(), th.base.arrival);
        // (He arrives facing her: the charm has already begun.)
        assert!(th.charm < 5.0 && th.wedding.is_none());
        assert!(g.diamonds.iter().any(|d| d.kind == DiamondKind::Angel && d.index == Archangel::Raphael.index()), "Raphael on his shrine");
        // She waits on her dais at the court's far end, before the arch into the spiral.
        assert_eq!(th.bride, th.court_dais());
        assert!(th.bride.x < g.hero.pos().x - 4.0);
    }

    #[test]
    fn facing_the_bride_fills_charm_and_facing_away_drains_it() {
        let mut g = garden();
        let (p, _) = west_side(&g);
        g.hero.place(p);
        g.hero.face(Dir::Up);
        run(&mut g, 1.0);
        let her = t(&g).bride;
        assert!(her.y < p.y as f64 && her.x < p.x as f64, "ahead of him up the west side, on the outer hedge: {her:?}");
        let c = t(&g).charm;
        assert!((15.0..=25.0).contains(&c), "a second facing her: {c}");
        g.hero.face(Dir::Down);
        run(&mut g, 1.0);
        let d = t(&g).charm;
        assert!(d < c - 10.0, "a second with his back to her drains it: {c} -> {d}");
    }

    #[test]
    fn full_charm_enthrals_him_mirrors_his_steering_and_draws_him_to_her() {
        let mut g = garden();
        g.hero.free_move = true;
        let (p, _) = west_side(&g);
        g.hero.place(p);
        g.hero.face(Dir::Up);
        g.update(STEP);
        t_mut(&mut g).charm = CHARM_MAX - 0.1;
        g.update(STEP);
        assert!(t(&g).enthralled > 0.0, "enthralled");
        assert!(g.floaters.iter().any(|f| f.text == "ENTHRALLED"));
        assert_eq!(g.steer_thorns(Dir::Left), Dir::Right, "his controls mirrored");
        g.hero.wish = V::new(1.0, 0.0);
        g.steer_thorns(Dir::Right);
        assert_eq!(g.hero.wish, V::new(-1.0, 0.0));
        // Drawn toward her (she stands up the path, on its outer hedge).
        let her = t(&g).bride;
        let before = math::len(math::sub(g.hero.pos(), her));
        run(&mut g, 1.0);
        let after = math::len(math::sub(g.hero.pos(), her));
        assert!(after < before - 1.5, "pulled toward her: {before} -> {after}");
        // The spell lasts three seconds, then breaks.
        run(&mut g, 2.2);
        assert_eq!(t(&g).enthralled, 0.0);
        assert_eq!(g.steer_thorns(Dir::Left), Dir::Left);
    }

    #[test]
    fn the_thorns_hurt_and_slow_him_and_the_roses_heal() {
        let mut g = garden();
        g.enemies.clear();
        g.refresh_foe_views();
        let th = t(&g).clone();
        let thorn = P::new(THORN_X, th.court.y0 - 3);
        assert_eq!(th.ground_at(thorn), Ground::Thorn);
        g.hero.place(thorn);
        g.hero.face(Dir::Right);
        let hp = g.hero.hp;
        run(&mut g, 2.0);
        assert!((3..=5).contains(&(hp - g.hero.hp)), "two health a second: {} -> {}", hp, g.hero.hp);
        assert_eq!(g.hero.pace, THORN_PACE, "slowed by a fifth");
        // On a rose path, healed; the pace back.
        let (rose, _) = west_side(&g);
        g.hero.place(rose);
        g.hero.face(Dir::Down);
        g.hero.hp = 50;
        run(&mut g, 2.0);
        assert!((53..=55).contains(&g.hero.hp), "healed two a second: {}", g.hero.hp);
        assert_eq!(g.hero.pace, 1.0);
        // In the air, the thorns cannot bite.
        g.hero.place(thorn);
        let before = g.hero.hp;
        t_mut(&mut g).thorn_carry = THORN_BITE - 0.01;
        g.hero.free_move = true;
        assert!(g.hero.try_jump(&g.level.grid));
        g.update(STEP);
        assert_eq!(g.hero.hp, before);
    }

    #[test]
    fn a_koi_burnt_at_a_censer_makes_smoke_that_blocks_charm() {
        let mut g = garden();
        let th = t(&g).clone();
        let pond = th.ponds.iter().find(|p| !p.golden && th.court.contains(p.tiles[0])).unwrap().clone();
        g.hero.place(pond.tiles[0]);
        run(&mut g, KOI_CATCH + 0.2);
        assert_eq!(t(&g).koi, 1, "a koi from the court's pond");
        // Beside the court's censer: burnt, and the smoke rises.
        let censer = th.censers[0];
        g.hero.place(P::new(censer.x - 1, censer.y));
        g.update(STEP);
        assert_eq!(t(&g).koi, 0);
        let s = t(&g).smoke.expect("smoke");
        assert!((s.at.x - censer.x as f64).abs() < 0.01);
        // In it, facing her charms nothing; out of it, it does.
        t_mut(&mut g).charm = 40.0;
        let her = P::new(censer.x - 3, censer.y);
        t_mut(&mut g).bride = her.v();
        t_mut(&mut g).bride_path = None;
        g.hero.face(Dir::Left);
        let c0 = t(&g).charm;
        for _ in 0..30 {
            t_mut(&mut g).bride = her.v();
            g.update(STEP);
        }
        assert!(t(&g).charm < c0, "the smoke blocks charm: {c0} -> {}", t(&g).charm);
        // Ten seconds, and it is gone.
        run(&mut g, SMOKE_SECONDS);
        assert!(t(&g).smoke.is_none());
    }

    #[test]
    fn the_golden_koi_gives_true_sight_and_the_bowers_their_grooms_gear() {
        let mut g = garden();
        let th = t(&g).clone();
        let golden = th.ponds.iter().find(|p| p.golden).unwrap().clone();
        g.hero.place(golden.tiles[0]);
        run(&mut g, KOI_CATCH + 0.2);
        assert!(t(&g).golden_taken && t(&g).sight > SIGHT_SECONDS - 1.0);
        let b = th.bowers[1];
        let runes = g.progress.runes;
        t_mut(&mut g).bowers[1].item = DropItem::Runes(3);
        g.hero.place(b.at);
        g.update(STEP);
        assert!(t(&g).bowers[1].taken);
        assert_eq!(g.progress.runes, runes + 3, "the groom's last gear");
    }

    #[test]
    fn raphaels_beam_burns_a_straight_path_through_the_hedges() {
        let mut g = garden();
        // From the first ring's west side, east through the hedge into the second ring.
        let (x0, y0, _, y1) = ring_bounds(0);
        let at = P::new(x0 + 1, (y0 + y1) / 2);
        g.hero.place(at);
        g.hero.face(Dir::Right);
        g.blessing = Some(crate::logic::angels::Blessing::of(Archangel::Raphael));
        g.update(STEP);
        let wall = P::new(x0 + 2, at.y);
        assert!(!g.level.grid.is_walkable(wall.x, wall.y, false));
        g.input.inject(Action::Angel);
        g.update(STEP);
        assert!(g.level.grid.is_walkable(wall.x, wall.y, false), "the hedge burnt away");
        assert_eq!(t(&g).ground_at(wall), Ground::Ash);
        assert!(t(&g).burnt.len() >= 4, "{} tiles burnt", t(&g).burnt.len());
        assert!(t(&g).burnt.iter().all(|p| p.y == at.y), "straight along the beam");
    }

    #[test]
    fn a_karakoncolos_calls_him_toward_her_and_the_elf_maids_draw_him_into_their_dance() {
        let mut g = garden();
        g.hero.free_move = true;
        g.enemies.clear();
        // A karakoncolos in the first clearing, its call just heard.
        let th = t(&g).clone();
        let den = th.clearings[0];
        let post = P::new(den.x0 + 1, den.y0 + 1);
        let mut b = g.brawl.take().unwrap();
        let raised = g.raise_packs(&[Pack { den, foes: vec![(EnemyKind::Karakoncolos, 5, post)] }], 1, &mut b);
        g.brawl = Some(b);
        g.enemies = raised;
        g.refresh_foe_views();
        let id = g.enemies[0].duel.as_ref().unwrap().id;
        let at = P::new(THORN_X, den.y0 - 3);
        g.hero.place(at);
        g.update(STEP);
        let her = t(&g).bride;
        let before = math::len(math::sub(g.hero.pos(), her));
        g.brawl.as_mut().unwrap().ails.pull = Some((id, 40));
        g.update(STEP);
        assert!(g.brawl.as_ref().unwrap().ails.pull.is_none(), "the call is hers, not its own");
        assert!(t(&g).call > 0);
        for _ in 0..20 {
            g.update(STEP);
        }
        let after = math::len(math::sub(g.hero.pos(), t(&g).bride));
        assert!(after < before - 0.2, "drawn toward her: {before} -> {after}");
        // The dancing ring in the last clearing drags a passer-by in.
        let c = th.dances[0];
        g.hero.place(P::new(c.x.round() as i32, (c.y + 2.6).round() as i32));
        g.update(STEP);
        let d0 = math::len(math::sub(g.hero.pos(), c));
        run(&mut g, 0.5);
        let d1 = math::len(math::sub(g.hero.pos(), c));
        assert!(d1 < d0 - 0.4, "into the dance: {d0} -> {d1}");
    }

    /// The wedding night begun by the F2 cheat: before the gate, facing it, the Bride behind him.
    fn wedding() -> Game {
        let mut g = garden();
        let n = g.enemies.len();
        g.cheat_thorns();
        assert!(t(&g).wedding.is_some(), "the wedding night");
        assert_eq!(g.enemies.len(), n + 2, "brides close in from either side");
        assert!(g.enemies[n..]
            .iter()
            .all(|e| e.kind() == EnemyKind::Ellepige && e.duel.as_ref().is_some_and(|d| d.den.is_some_and(|d| d.awake))));
        // (The rest is between him and her.)
        g.enemies.clear();
        g.refresh_foe_views();
        g
    }

    #[test]
    fn the_wedding_night_with_his_back_turned_ends_with_the_gate_open() {
        let mut g = wedding();
        let th = t(&g).clone();
        assert!(th.bride.y > g.hero.pos().y + 5.0, "she walks up behind him");
        assert!(g.juice.banner.as_ref().is_some_and(|b| b.text.contains("TURN AROUND")));
        for _ in 0..((WEDDING_SECONDS - 0.5) / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
            g.hero.hp = g.hero.max_hp;
        }
        assert!(!g.gate_open, "not yet");
        let d = math::len(math::sub(t(&g).bride, g.hero.pos()));
        assert!(d < EMBRACE * 1.2, "she stands at his back: {d}");
        run(&mut g, 0.7);
        assert!(g.gate_open, "ten seconds with his back to her: the gate opens");
        assert!(t(&g).wedding.is_none() && t(&g).grooms == 0);
    }

    #[test]
    fn facing_her_on_the_wedding_night_fills_charm_three_times_as_fast() {
        let mut g = wedding();
        t_mut(&mut g).charm = 0.0;
        g.hero.face(Dir::Down);
        run(&mut g, 0.5);
        let c = t(&g).charm;
        assert!((25.0..=35.0).contains(&c), "half a second facing her: {c}");
    }

    #[test]
    fn enthralled_and_drawn_into_her_he_becomes_the_eighth_groom() {
        let mut g = wedding();
        g.hero.free_move = true;
        g.hero.face(Dir::Down);
        let max = g.hero.max_hp;
        for _ in 0..(8.0 / STEP) as usize {
            g.update(STEP);
            if t(&g).wedding.is_none() {
                break;
            }
        }
        let th = t(&g);
        assert_eq!(th.grooms, 1, "taken");
        assert_eq!(g.hero.occupied(), th.inner, "he wakes in the innermost bower");
        assert!(g.hero.hp <= max - (max as f64 * GROOM_WOUND) as i32 + 1 && g.hero.hp > 0, "a heavy blow: {}", g.hero.hp);
        assert_eq!(g.brawl.as_ref().unwrap().ails.doom, DOOM_FRAMES, "and Doom");
        assert!(!g.gate_open);
        assert_eq!(g.screen, Screen::Playing);
    }

    #[test]
    fn leaping_through_her_and_three_blows_on_her_back_unmake_her() {
        let mut g = wedding();
        g.hero.free_move = true;
        // Let her come close behind him.
        for _ in 0..(6.0 / STEP) as usize {
            g.update(STEP);
            g.hero.invuln = 1.0;
            if math::len(math::sub(t(&g).bride, g.hero.pos())) < 2.0 {
                break;
            }
        }
        // Turn and leap through her: she cannot take him in the air.
        g.hero.face(Dir::Down);
        t_mut(&mut g).charm = 0.0;
        g.input.inject(Action::Jump);
        for _ in 0..(0.6 / STEP) as usize {
            g.update(STEP);
        }
        assert!(t(&g).wedding.is_some_and(|w| w.passed), "through her");
        assert_eq!(t(&g).grooms, 0, "not taken");
        assert!(t(&g).behind_her(g.hero.pos()), "behind her now");
        // Face her hollow back and strike it three times.
        g.hero.face(Dir::Up);
        for _ in 0..12 {
            if g.gate_open {
                break;
            }
            let her = t(&g).bride;
            let to = math::norm(math::sub(her, g.hero.pos()));
            g.hero.heading = to.y.atan2(to.x);
            g.input.inject(Action::Attack);
            run(&mut g, 0.45);
        }
        assert!(t(&g).unmade, "unmade: {:?}", t(&g).wedding);
        assert!(g.gate_open, "the gate opens at once");
    }

    #[test]
    fn a_fall_undoes_the_spell_and_the_wedding_night_but_not_the_burnt_paths() {
        let mut g = wedding();
        t_mut(&mut g).charm = 80.0;
        t_mut(&mut g).koi = 2;
        t_mut(&mut g).burnt.push(P::new(10, 10));
        g.reset_thorns();
        let th = t(&g);
        assert!(th.wedding.is_none() && th.charm == 0.0 && th.koi == 0);
        assert_eq!(th.burnt.len(), 1);
        assert_eq!(g.thorns_meters()[0].label, "CHARM");
        // The wedding night again: the brides from the sides came once, and do not come again.
        let n = g.enemies.len();
        g.cheat_thorns();
        assert!(t(&g).wedding.is_some());
        assert_eq!(g.enemies.len(), n);
    }
}
