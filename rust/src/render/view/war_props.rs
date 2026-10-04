//! The generals in the world (WARBAND.md §9): their instruments as small props of blocks (the
//! watch-fire's brazier, the war horn on its post, the counting-bell in its frame, the bellows, the
//! chain on its winch, and each as wreckage once broken), the gate glowing for a muster, the
//! kindled marks on the floor and their bursts, and the general dressed: his banner on its pole at
//! his back and his cloak in his colour. A few dozen cubes a frame, drawn with the rest.

use super::*;
use crate::game::general::GeneralRun;
use crate::logic::war::general::{Instrument, InstrumentKind, OrderStage, KINDLE_RADIUS};

const WOOD: u32 = 0x5a3a22;
const WOOD_DARK: u32 = 0x3a2616;
const IRON: u32 = 0x3a3a40;
const STONE: u32 = 0x6a6258;
const BRASS: u32 = 0xc8a040;
const BONE: u32 = 0xe0d4b8;
const LEATHER: u32 = 0x5a3220;
const LEATHER_LIGHT: u32 = 0x7a4a2c;
const WOOD_LIGHT: u32 = 0x9a6a3a;
const PLINTH: u32 = 0x9a9080;
const CHAIN: u32 = 0x7a7a84;
const FIRE: u32 = 0xff8a2a;

/// One block: centred at `at`, `size` across, turned `turn` about the vertical.
fn block(f: &mut Frame, at: Vec3, size: Vec3, turn: f32, color: u32, glow: f32) {
    let m = Mat4::from_translation(at) * Mat4::from_rotation_y(turn) * Mat4::from_scale(size);
    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(color), 1.0, lin(color) * glow, SHADE_LIT));
}

/// A block turned about all three axes (a fallen piece, a swinging bell).
fn tilted(f: &mut Frame, at: Vec3, size: Vec3, rot: Quat, color: u32, glow: f32) {
    let m = Mat4::from_scale_rotation_translation(size, rot, at);
    f.opaque[MeshKind::Cube as usize].push(Instance::new(m, lin(color), 1.0, lin(color) * glow, SHADE_LIT));
}

/// A soft glow (additive) at `at`.
fn glow(f: &mut Frame, at: Vec3, size: f32, color: u32, k: f32) {
    let m = Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(size));
    f.additive.push(Instance::new(m, lin(color) * k, 0.9, Vec3::ZERO, SHADE_UNLIT));
}

impl WorldView {
    /// The general's realm: his instruments, the glowing gate, the kindled floor, his dress.
    pub(super) fn emit_war(&self, s: &SceneState, f: &mut Frame) {
        let Some(r) = s.general else { return };
        let time = s.time as f32;
        for k in 0..3 {
            let i = r.command.instruments[k];
            let busy = matches!(r.command.stages[k], OrderStage::Announced { .. } | OrderStage::Live { .. });
            let live = matches!(r.command.stages[k], OrderStage::Live { .. });
            self.emit_instrument(s, f, &i, busy, live, r.general().cloak, r.general().hazard_color, time);
        }
        self.emit_gate(s, f, r, time);
        self.emit_kindling(s, f, r, time);
        self.emit_dress(s, f, r, time);
    }

    #[allow(clippy::too_many_arguments)]
    fn emit_instrument(&self, s: &SceneState, f: &mut Frame, i: &Instrument, busy: bool, live: bool, cloak: u32, hazard: u32, time: f32) {
        let (x, z) = (i.tile.x as f32, i.tile.y as f32);
        let base = Vec3::new(x, floor_at(s.grid, x, z), z);
        let seed = (x * 1.7 + z * 2.3).sin() * 3.0;
        if i.broken {
            // Wreckage: its pieces lying where it stood.
            for k in 0..6 {
                let a = seed + k as f32 * 1.1;
                let r = 0.15 + 0.08 * k as f32;
                let at = base + Vec3::new(a.cos() * r, 0.04 + 0.02 * (k % 2) as f32, a.sin() * r);
                let rot = Quat::from_rotation_y(a) * Quat::from_rotation_z(0.2 * (k as f32 - 2.5));
                let c = match (i.kind, k % 3) {
                    (InstrumentKind::Bell, 0) | (InstrumentKind::WarHorn, 0) => BRASS,
                    (InstrumentKind::WatchFire, 0) | (InstrumentKind::Chain, _) => IRON,
                    _ => WOOD_DARK,
                };
                tilted(f, at, Vec3::new(0.32, 0.06, 0.1), rot, c, 0.0);
            }
            return;
        }
        // A beam of his colour over it while its order is under way: the ping, seen from afar.
        if busy {
            let pulse = 0.5 + 0.5 * (time * 6.0).sin();
            let m = Mat4::from_translation(base + Vec3::new(0.0, 2.6, 0.0)) * Mat4::from_scale(Vec3::new(0.06, 4.0, 0.06));
            f.additive.push(Instance::new(m, lin(cloak) * (1.2 + pulse), 0.5, Vec3::ZERO, SHADE_UNLIT));
        }
        match i.kind {
            InstrumentKind::WatchFire => {
                block(f, base + Vec3::new(0.0, 0.35, 0.0), Vec3::new(0.34, 0.7, 0.34), seed, STONE, 0.0);
                for k in 0..6 {
                    let a = k as f32 * std::f32::consts::TAU / 6.0 + seed;
                    block(f, base + Vec3::new(a.cos() * 0.24, 0.82, a.sin() * 0.24), Vec3::new(0.05, 0.26, 0.05), a, IRON, 0.0);
                }
                block(f, base + Vec3::new(0.0, 0.72, 0.0), Vec3::new(0.42, 0.05, 0.42), seed, IRON, 0.0);
                let flick = 0.8 + 0.25 * (time * 13.0 + seed).sin() + 0.15 * (time * 29.0).sin();
                for k in 0..4 {
                    let a = k as f32 * 1.7 + time * 3.0;
                    glow(
                        f,
                        base + Vec3::new(a.cos() * 0.08, 0.9 + 0.09 * k as f32, a.sin() * 0.08),
                        0.14 - 0.02 * k as f32,
                        FIRE,
                        2.2 * flick,
                    );
                }
                f.lights.push(Light { pos: base + Vec3::new(0.0, 1.2, 0.0), color: lin(FIRE) * 3.0 * flick, distance: 5.0 });
            }
            InstrumentKind::WarHorn => {
                block(f, base + Vec3::new(0.0, 0.7, 0.0), Vec3::new(0.12, 1.4, 0.12), seed, WOOD, 0.0);
                block(f, base + Vec3::new(0.0, 1.25, 0.0), Vec3::new(0.6, 0.08, 0.08), seed, WOOD_DARK, 0.0);
                // The horn: a curve of bone swelling to its bell, brass at the mouth.
                let shake = if busy { 0.04 * (time * 30.0).sin() } else { 0.0 };
                for k in 0..6 {
                    let u = k as f32 / 5.0;
                    let at = base + Vec3::new(-0.28 + 0.56 * u, 1.38 + 0.18 * (u * 3.0).sin() + shake, 0.0);
                    let c = if k == 5 { BRASS } else { BONE };
                    block(f, at, Vec3::splat(0.06 + 0.07 * u), seed, c, if busy && k == 5 { 0.4 } else { 0.0 });
                }
            }
            InstrumentKind::Bell => {
                for sx in [-0.42f32, 0.42] {
                    block(f, base + Vec3::new(sx, 0.06, 0.0), Vec3::new(0.22, 0.12, 0.22), 0.0, PLINTH, 0.0);
                    block(f, base + Vec3::new(sx, 0.7, 0.0), Vec3::new(0.1, 1.3, 0.1), 0.0, WOOD_LIGHT, 0.0);
                }
                block(f, base + Vec3::new(0.0, 1.36, 0.0), Vec3::new(1.05, 0.1, 0.14), 0.0, WOOD, 0.0);
                let swing = if busy { 0.45 * (time * 5.0).sin() } else { 0.05 * (time * 0.8 + seed).sin() };
                let pivot = base + Vec3::new(0.0, 1.27, 0.0);
                let rot = Quat::from_rotation_z(swing);
                for (k, (w, y)) in [(0.16f32, 0.1f32), (0.26, 0.24), (0.34, 0.38), (0.42, 0.48)].iter().enumerate() {
                    let at = pivot + rot * Vec3::new(0.0, -y, 0.0);
                    let tall = if k == 3 { 0.05 } else { 0.14 };
                    tilted(f, at, Vec3::new(*w, tall, *w), rot, BRASS, if busy { 0.25 } else { 0.05 * k as f32 });
                }
                tilted(f, pivot + rot * Vec3::new(0.0, -0.55, 0.0), Vec3::splat(0.08), rot, IRON, 0.0);
                if busy {
                    glow(f, pivot + Vec3::new(0.0, -0.3, 0.0), 0.35, BRASS, 0.6 + 0.6 * (time * 10.0).sin().abs());
                }
            }
            InstrumentKind::Bellows => {
                // A forge's bellows on a stone plinth, its nozzle toward the floor it kindles
                // (turned about, it faces the other way and glows cold).
                let turned = i.turned;
                let pump = if live { 0.05 * (time * 7.0).sin() } else { 0.0 };
                let q = Quat::from_rotation_y(seed + if turned { std::f32::consts::PI } else { 0.0 });
                let at = |o: Vec3| base + q * o;
                tilted(f, at(Vec3::new(0.0, 0.09, 0.0)), Vec3::new(0.78, 0.18, 0.56), q, PLINTH, 0.0);
                tilted(f, at(Vec3::new(0.0, 0.21, 0.0)), Vec3::new(0.7, 0.05, 0.46), q, WOOD_LIGHT, 0.0);
                for k in 0..3 {
                    let y = 0.27 + k as f32 * (0.06 + pump * 0.6);
                    let w = 0.56 - 0.05 * k as f32;
                    tilted(
                        f,
                        at(Vec3::new(-0.04 * k as f32, y, 0.0)),
                        Vec3::new(w, 0.06, 0.4 - 0.03 * k as f32),
                        q,
                        if k % 2 == 0 { LEATHER } else { LEATHER_LIGHT },
                        0.0,
                    );
                }
                let lid = q * Quat::from_rotation_z(-0.16 - pump);
                tilted(f, at(Vec3::new(-0.04, 0.47 + pump * 2.0, 0.0)), Vec3::new(0.7, 0.05, 0.46), lid, WOOD_LIGHT, 0.0);
                for y in [0.21f32, 0.5 + pump * 2.0] {
                    tilted(f, at(Vec3::new(-0.5, y, 0.0)), Vec3::new(0.26, 0.05, 0.07), q, WOOD, 0.0);
                }
                tilted(f, at(Vec3::new(0.44, 0.27, 0.0)), Vec3::new(0.24, 0.08, 0.08), q, BRASS, 0.15);
                let tint = if turned { 0x7ad8ff } else { hazard };
                let k = if live { 1.8 + 0.8 * (time * 9.0).sin().abs() } else { 0.6 };
                glow(f, at(Vec3::new(0.6, 0.27, 0.0)), 0.11, tint, k);
                if live {
                    f.lights.push(Light { pos: base + Vec3::new(0.0, 0.8, 0.0), color: lin(tint) * 2.0, distance: 4.0 });
                }
            }
            InstrumentKind::Chain => {
                // A winch: its drum between two posts, a crank, and the chain running down from it
                // to a gold lock on a ring in the floor (cut it, and the decree goes nowhere).
                let q = Quat::from_rotation_y(seed);
                let at = |o: Vec3| base + q * o;
                for sx in [-0.38f32, 0.38] {
                    tilted(f, at(Vec3::new(sx, 0.4, 0.0)), Vec3::new(0.1, 0.8, 0.12), q, WOOD, 0.0);
                }
                let spin = if live { time * 2.5 } else { 0.0 };
                let axle = q * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2) * Quat::from_rotation_y(spin);
                // (The cylinder mesh is a puck, 0.37 across its radius and 0.12 high.)
                let drum = Mat4::from_scale_rotation_translation(Vec3::new(0.46, 5.0, 0.46), axle, at(Vec3::new(0.0, 0.64, 0.0)));
                f.opaque[MeshKind::Cylinder as usize].push(Instance::new(drum, lin(WOOD_LIGHT), 1.0, Vec3::ZERO, SHADE_LIT));
                for bx in [-0.22f32, 0.22] {
                    let band = Mat4::from_scale_rotation_translation(Vec3::new(0.54, 0.5, 0.54), axle, at(Vec3::new(bx, 0.64, 0.0)));
                    f.opaque[MeshKind::Cylinder as usize].push(Instance::new(band, lin(CHAIN), 1.0, Vec3::ZERO, SHADE_LIT));
                }
                let crank = q * Quat::from_rotation_x(spin);
                tilted(f, at(Vec3::new(0.48, 0.64, 0.0)), Vec3::new(0.05, 0.05, 0.05), q, CHAIN, 0.0);
                tilted(f, at(Vec3::new(0.5, 0.64, 0.0)) + crank * Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.04, 0.22, 0.04), crank, CHAIN, 0.0);
                for k in 0..8 {
                    let u = k as f32 / 7.0;
                    let o = Vec3::new(0.0, 0.5 * (1.0 - u) + 0.06, 0.12 + 0.64 * u);
                    let w = if k % 2 == 0 { Vec3::new(0.1, 0.06, 0.03) } else { Vec3::new(0.03, 0.06, 0.1) };
                    tilted(f, at(o), w, q, CHAIN, 0.05);
                }
                tilted(f, at(Vec3::new(0.0, 0.03, 0.8)), Vec3::new(0.22, 0.05, 0.22), q, CHAIN, 0.0);
                tilted(f, at(Vec3::new(0.0, 0.12, 0.8)), Vec3::new(0.12, 0.12, 0.06), q, BRASS, 0.25);
            }
        }
    }

    /// The gate glowing for a muster: a ring of fire on the floor and its posts lit, filling blue
    /// as he bars it.
    fn emit_gate(&self, s: &SceneState, f: &mut Frame, r: &GeneralRun, time: f32) {
        let Some((at, barred)) = r.glowing() else { return };
        let (x, z) = (at.x as f32, at.y as f32);
        let base = Vec3::new(x, floor_at(s.grid, x, z), z);
        let pulse = 0.6 + 0.4 * (time * 6.0).sin();
        let m = Mat4::from_translation(base + Vec3::new(0.0, 0.04, 0.0))
            * Mat4::from_rotation_x(-PI / 2.0)
            * Mat4::from_scale(Vec3::new(1.1, 1.1, 1.0));
        f.transparent[MeshKind::Annulus as usize].push(Instance::new(m, lin(0xff8a3a) * (1.4 + pulse), 0.7, Vec3::ZERO, SHADE_UNLIT));
        for k in 0..2 {
            let side = if k == 0 { -0.9 } else { 0.9 };
            block(f, base + Vec3::new(side, 0.6, 0.0), Vec3::new(0.16, 1.2, 0.16), 0.0, STONE, 0.0);
            glow(f, base + Vec3::new(side, 1.3, 0.0), 0.16, 0xff8a3a, 1.6 * pulse);
        }
        if barred > 0.0 {
            let m = Mat4::from_translation(base + Vec3::new(0.0, 0.05, 0.0))
                * Mat4::from_rotation_x(-PI / 2.0)
                * Mat4::from_scale(Vec3::new(1.1 * barred, 1.1 * barred, 1.0));
            f.transparent[MeshKind::Disc as usize].push(Instance::new(m, lin(0x9ad1ff) * 1.4, 0.35, Vec3::ZERO, SHADE_UNLIT));
        }
        f.lights.push(Light { pos: base + Vec3::new(0.0, 1.0, 0.0), color: lin(0xff8a3a) * 3.0 * pulse, distance: 5.0 });
    }

    /// The kindled marks: a ring in the hazard's colour, filling in from its edge over its three
    /// seconds and flashing as it comes due; then spikes thrust up out of the floor.
    fn emit_kindling(&self, s: &SceneState, f: &mut Frame, r: &GeneralRun, time: f32) {
        let color = r.general().hazard_color;
        let now = r.now;
        for v in &r.volleys {
            let tint = if v.turned { 0x7ad8ff } else { color };
            for m in &v.marks {
                let (x, z) = (m.x as f32, m.y as f32);
                let at = Vec3::new(x, floor_at(s.grid, x, z), z);
                if let Some(b) = v.burst {
                    let u = b as f32 / 30.0;
                    let rise = if u < 0.15 { u / 0.15 } else { 1.0 - ((u - 0.15) / 0.85).powi(2) };
                    for k in 0..7 {
                        let a = k as f32 * 2.39 + x * 0.7;
                        let rr = if k == 0 { 0.0 } else { KINDLE_RADIUS as f32 * (0.3 + 0.6 * ((k * 37 % 10) as f32 / 10.0)) };
                        let tall = (0.5 + 0.4 * ((k * 13 % 7) as f32 / 7.0)) * rise;
                        let p = at + Vec3::new(a.cos() * rr, tall * 0.5, a.sin() * rr);
                        block(f, p, Vec3::new(0.09, tall.max(0.01), 0.09), a, tint, 0.4);
                    }
                    f.lights.push(Light { pos: at + Vec3::new(0.0, 0.6, 0.0), color: lin(tint) * 3.0 * (1.0 - u), distance: 3.0 });
                    continue;
                }
                let Some(_) = v.due else { continue };
                let fill = v.fill(now);
                let late = fill > 0.92;
                let blink = if late { 0.6 + (time * 40.0).sin().abs() * 0.8 } else { 0.0 };
                let rad = KINDLE_RADIUS as f32;
                let n = ((std::f32::consts::TAU * rad) / 0.22).ceil() as i32;
                for k in 0..n {
                    let a = k as f32 * std::f32::consts::TAU / n as f32;
                    let p = at + Vec3::new(a.cos() * rad, 0.03, a.sin() * rad);
                    let mm = Mat4::from_translation(p) * Mat4::from_scale(Vec3::new(0.08, 0.012, 0.08));
                    f.additive.push(Instance::new(mm, lin(tint) * (0.9 + blink), 0.7, Vec3::ZERO, SHADE_UNLIT));
                }
                let inner = rad * (1.0 - fill);
                let mm = Mat4::from_translation(at + Vec3::new(0.0, 0.025, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(rad, rad, 1.0));
                f.transparent[MeshKind::Disc as usize].push(Instance::new(
                    mm,
                    lin(tint) * (0.4 + blink * 0.6),
                    0.08 + 0.22 * fill,
                    Vec3::ZERO,
                    SHADE_UNLIT,
                ));
                let mm = Mat4::from_translation(at + Vec3::new(0.0, 0.03, 0.0))
                    * Mat4::from_rotation_x(-PI / 2.0)
                    * Mat4::from_scale(Vec3::new(inner + 0.05, inner + 0.05, 1.0));
                f.transparent[MeshKind::Annulus as usize].push(Instance::new(mm, lin(tint) * 1.2, 0.5, Vec3::ZERO, SHADE_UNLIT));
            }
        }
    }

    /// The general dressed: a cloak of his colour hanging at his back, and his banner on its pole
    /// above it, his sin's sigil on the cloth.
    fn emit_dress(&self, s: &SceneState, f: &mut Frame, r: &GeneralRun, time: f32) {
        let Some(id) = r.id else { return };
        let Some((i, d)) =
            s.enemies.iter().enumerate().find_map(|(i, e)| e.duel.as_deref().filter(|d| d.id == id && d.alive()).map(|d| (i, d)))
        else {
            return;
        };
        let Some(v) = self.enemy_views.get(i).filter(|v| v.visible) else { return };
        let gen = r.general();
        let top = crown(d.breed.kind).unwrap_or(1.2);
        let back = Vec3::new(-d.facing.x as f32, 0.0, -d.facing.y as f32).normalize_or_zero();
        let side = Vec3::new(-back.z, 0.0, back.x);
        let turn = (-back.z).atan2(back.x);
        let at = v.group_pos;
        // The cloak: two panels hanging from his shoulders, swaying as he goes.
        let sway = 0.06 * (time * 3.0 + at.x).sin();
        for k in 0..3 {
            let y = top * (0.75 - 0.22 * k as f32);
            let p = at + back * (0.26 + 0.05 * k as f32 + sway * k as f32) + Vec3::new(0.0, y, 0.0);
            block(f, p, Vec3::new(0.05, top * 0.24, 0.5 + 0.06 * k as f32), turn, gen.cloak, 0.05);
        }
        // The banner: its pole from his back to over his head, the cloth, the sigil.
        let pole = at + back * 0.32 + side * 0.18;
        let tall = top + 0.9;
        block(f, pole + Vec3::new(0.0, tall / 2.0, 0.0), Vec3::new(0.05, tall, 0.05), turn, WOOD_DARK, 0.0);
        block(f, pole + Vec3::new(0.0, tall, 0.0), Vec3::new(0.07, 0.07, 0.07), turn, BRASS, 0.3);
        let flap = 0.15 * (time * 4.0 + at.z).sin();
        let cloth = pole + Vec3::new(0.0, tall - 0.28, 0.0) + side * 0.26;
        let rot = Quat::from_rotation_y(turn + flap);
        tilted(f, cloth, Vec3::new(0.03, 0.44, 0.5), rot, gen.cloak, 0.1);
        tilted(f, cloth + side * 0.02 + Vec3::new(0.0, 0.02, 0.0), Vec3::new(0.035, 0.16, 0.16), rot, r.sin.color(), 0.6);
        f.lights.push(Light { pos: at + Vec3::new(0.0, top + 0.6, 0.0), color: lin(gen.cloak) * 1.4, distance: 3.5 });
    }
}
