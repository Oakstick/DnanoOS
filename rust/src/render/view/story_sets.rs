//! Scene I's two sets, built once, in the art direction of the dungeons (stage B): the
//! underworld's gateway is cut from the Bone Vaults' and the Cisterns' stone (`crypt`'s
//! builders), and the graveyard at dusk is the same quality out of doors. The palette is the
//! board's: charcoal, umber, copper, amber, mist teal, pale stone.
//!
//! Each set also carries its light: the practicals (braziers, lanterns) stand here so the
//! story's choreography lights them where they stand, and the set's probes are worked out from
//! these cubes when the story loads (`SetProbes`).

use super::crypt::pal::*;
use super::crypt::{
    arch_cubes, banner_cubes, block, block_m, bone_cubes, bracket_cubes, brazier_cubes, flagstones, glowing_block, jit, lantern_cubes,
    masonry, moss_cap, moss_seam, pillar_cubes, statue_cubes, Arch, Wall, SHADE_BRONZE, SHADE_CLOTH, SHADE_FLAG, SHADE_IRON, SHADE_SOFT,
    SHADE_STONE, SHADE_WOOD,
};
use super::story::GATEWAY;
use super::*;

// ================================================================== the gateway

/// The back wall's face (the door's plane is a touch behind it), the side walls' and the front's.
pub(super) const BACK: f32 = -3.42;
const SIDE: f32 = 4.4;
const FRONT: f32 = 4.0;
/// The walls' height and their courses.
const GATEWAY_H: f32 = 4.8;
const GATEWAY_COURSES: usize = 11;
/// The pillars down each side, carrying the vault's ribs.
const PILLARS: [(f32, f32); 4] = [(-3.35, -1.3), (3.35, -1.3), (-3.35, 2.0), (3.35, 2.0)];
const PILLAR_H: f32 = 3.0;
/// The copper braziers on the floor before the back pillars (their size), lighting the room.
pub(super) const BRAZIERS: [Vec3; 2] = [Vec3::new(-2.6, 0.0, -1.3), Vec3::new(2.6, 0.0, -1.3)];
pub(super) const BRAZIER_SCALE: f32 = 1.25;
/// The lanterns hung under the ribs' crowns, and on brackets from the front pillars: the middle
/// of their glass.
pub(super) const RIB_LANTERNS: [Vec3; 2] = [Vec3::new(0.0, 3.38, -1.3), Vec3::new(0.0, 3.38, 2.0)];
pub(super) const WALL_LANTERNS: [Vec3; 2] = [Vec3::new(-2.6, 1.74, 2.0), Vec3::new(2.6, 1.74, 2.0)];
/// The gate: its towers' middles (x), their tops, and the lock lamps over the arch.
const TOWER_X: f32 = 1.62;
const TOWER_TOP: f32 = 3.4;
pub(super) const LOCK_LAMPS: [Vec3; 5] = [
    Vec3::new(-1.0, 3.86, -3.12),
    Vec3::new(-0.5, 3.92, -3.12),
    Vec3::new(0.0, 3.95, -3.12),
    Vec3::new(0.5, 3.92, -3.12),
    Vec3::new(1.0, 3.86, -3.12),
];
/// The way they came: the stone frame in the front wall, the middle of its opening.
pub(super) const PORTAL: Vec3 = Vec3::new(0.0, 1.32, 3.98);
/// Shallow puddles on the flags (middle, size), where the vault drips.
pub(super) const PUDDLES: [(Vec3, Vec2); 5] = [
    (Vec3::new(-1.2, 0.012, 1.4), Vec2::new(0.9, 0.6)),
    (Vec3::new(2.2, 0.012, 0.4), Vec2::new(0.7, 1.0)),
    (Vec3::new(-0.4, 0.012, -1.6), Vec2::new(1.1, 0.5)),
    (Vec3::new(1.1, 0.012, 2.9), Vec2::new(0.6, 0.7)),
    (Vec3::new(-2.6, 0.012, -0.2), Vec2::new(0.6, 0.9)),
];

/// The gateway's light: the vaults' cold teal dark, a faint light from shafts far above, the
/// braziers and lanterns doing the rest (they are the story's own lights).
pub(super) fn gateway_atmosphere() -> Atmosphere {
    Atmosphere {
        background: DARK,
        fog_near: 6.0,
        fog_far: 22.0,
        hemi_sky: lin(MIST_TEAL) * 0.42,
        hemi_ground: lin(0x3a2e27) * 0.14,
        ambient: lin(0x2a3634) * 0.9,
        sun_dir: Vec3::new(0.3, 1.0, 0.35).normalize(),
        sun: lin(0x9ec4bc) * 0.4,
        exposure: 1.35,
        night_sky: 0.0,
        bloom: 1.1,
        moon_view: Vec3::Y,
        cloud_tint: lin(DARK),
    }
}

/// The gateway's ground haze (RP19): the vaults' cold mist, lying low.
pub(super) fn gateway_haze() -> Haze {
    Haze { density: 0.05, floor: 0.0, falloff: 1.0 }
}

/// The dungeon gateway at the top of the underworld, everything that stays put: wet flagstones,
/// walls of chunky dressed stone, pillars and ribs, the crypt gate round the iron door, the old
/// man's corner and Quitmer's, braziers and lanterns, the stone frame of the way they came, and
/// the stair down behind the door.
pub(super) fn gateway_set() -> Vec<Instance> {
    let mut out = Vec::new();
    let o = GATEWAY;
    let at = |x: f32, y: f32, z: f32| o + Vec3::new(x, y, z);

    // ---- the floor: flagstones in their grout over a dark bed, moss in some seams
    block(&mut out, at(0.0, -0.3, 0.3), Vec3::new(10.0, 0.4, 9.0), 0.0, lin(GROUT), SHADE_LIT);
    for gx in -4..=4 {
        for gz in -3..=4 {
            let (hx, hy) = (gx as f64 + 211.0, gz as f64 + 97.0);
            let p = at(gx as f32, 0.0, gz as f32);
            flagstones(&mut out, p, (hx, hy));
            moss_seam(&mut out, p, (hx, hy));
        }
    }

    // ---- the walls, whole, each face in chunky dressed courses with fallen blocks at its foot
    let wall = |centre: Vec3, dir: (i32, i32), half: f32, seed: f64| Wall {
        centre,
        dir,
        half,
        base: 0.0,
        height: GATEWAY_H,
        courses: GATEWAY_COURSES,
        seed: (seed, seed * 1.7 + 3.0),
        len: (0.5, 0.45),
        depth: (0.1, 0.12),
        face: 0.46,
        wet: true,
        ruined: false,
        foot: Some(o.y),
        openings: &[],
    };
    // The back wall: the gate stands in it (its towers, arch and cornice take that stretch).
    let gate_gap = [(0.0, 2.04, TOWER_TOP + 0.16, 0.0), (0.0, 1.3, TOWER_TOP + 1.22, 0.0)];
    masonry(&mut out, &Wall { openings: &gate_gap, ..wall(at(0.0, 0.0, BACK - 0.46), (0, 1), 4.62, 11.0) });
    for side in [-1.0f32, 1.0] {
        masonry(&mut out, &wall(at(side * (SIDE + 0.46), 0.0, 0.3), (-side as i32, 0), 3.75, 23.0 + side as f64 * 5.0));
    }
    // The front wall, with the way they came in it.
    let portal_gap = [(0.0, 1.14, PORTAL.y + 0.28, 1.14)];
    masonry(&mut out, &Wall { openings: &portal_gap, ..wall(at(0.0, 0.0, FRONT + 0.46), (0, -1), 4.62, 41.0) });
    // The cores behind the faces (open for the door and the way they came), and a coping of
    // dark slabs along their tops.
    let core = jit(CHARCOAL[1], 0.4, 0.0) * 0.6;
    for (z, hole) in [(BACK - 0.28, 2.75f32), (FRONT + 0.28, 2.75)] {
        for side in [-1.0f32, 1.0] {
            block(&mut out, at(side * 3.0, GATEWAY_H / 2.0, z), Vec3::new(4.0, GATEWAY_H, 0.6), 0.0, core, SHADE_STONE);
        }
        block(&mut out, at(0.0, (hole + GATEWAY_H) / 2.0, z), Vec3::new(2.0, GATEWAY_H - hole, 0.6), 0.0, core, SHADE_STONE);
    }
    for side in [-1.0f32, 1.0] {
        block(&mut out, at(side * (SIDE + 0.28), GATEWAY_H / 2.0, 0.3), Vec3::new(0.6, GATEWAY_H, 8.6), 0.0, core, SHADE_STONE);
    }
    for k in 0..14 {
        let hk = hash(k as f64 * 3.1, 7.0);
        let x = -4.6 + k as f32 * 0.7;
        for z in [BACK - 0.5, FRONT + 0.5] {
            block(
                &mut out,
                at(x + 0.35, GATEWAY_H + 0.06, z),
                Vec3::new(0.68, 0.12, 1.1),
                (hk as f32 - 0.5) * 0.03,
                jit(CHARCOAL[k % 3], hk, 0.08) * 0.8,
                SHADE_STONE,
            );
        }
    }

    // ---- the pillars, the vault's ribs across between them, blind arches along the side walls
    for (i, &(x, z)) in PILLARS.iter().enumerate() {
        pillar_cubes(&mut out, at(x, 0.0, z), PILLAR_H, 7, 1.3, (i as f64 * 5.3 + 1.0, z as f64));
        moss_cap(&mut out, at(x, PILLAR_H, z), 0.5, (i as f64 * 2.0 + 0.3, 1.1));
    }
    for (k, z) in [-1.3f32, 2.0].into_iter().enumerate() {
        arch_cubes(
            &mut out,
            &Arch {
                centre: at(0.0, 0.0, z),
                across_x: true,
                r: 3.35,
                rise: 1.2,
                spring: PILLAR_H - 0.1,
                depth: 0.52,
                piers: false,
                lintel: false,
                seed: (k as f64 * 7.0 + 3.0, 5.0),
            },
        );
    }
    for side in [-1.0f32, 1.0] {
        // Between the back wall and the first pillar, the pillars, and the second and the front wall.
        for (k, &(z, r)) in [(-2.56f32, 0.82f32), (0.35, 1.24), (3.2, 0.78)].iter().enumerate() {
            let x = side * (SIDE - 0.26);
            arch_cubes(
                &mut out,
                &Arch {
                    centre: at(x, 0.0, z),
                    across_x: false,
                    r,
                    rise: r * 0.8,
                    spring: 2.0,
                    depth: 0.32,
                    piers: true,
                    lintel: true,
                    seed: (side as f64 * 9.0 + k as f64, 13.0 + k as f64),
                },
            );
            // In each a niche, its back dark: a skull in the outer ones, the middle one hung with a banner.
            let inward = -side;
            if k == 1 {
                banner_cubes(&mut out, at(side * (SIDE + 0.27), 0.0, z), (inward as i32, 0), 0.75, (side as f64 * 3.0, 4.0));
            } else {
                block(
                    &mut out,
                    at(x + inward * 0.05, 1.25, z),
                    Vec3::new(0.04, 0.6, r * 0.9),
                    0.0,
                    jit(CHARCOAL[0], 0.3, 0.0) * 0.3,
                    SHADE_STONE,
                );
                block(
                    &mut out,
                    at(x + inward * 0.12, 0.98, z),
                    Vec3::new(0.24, 0.05, r * 1.2),
                    0.0,
                    jit(PALE_STONE[1], 0.3, 0.0) * 0.7,
                    SHADE_STONE,
                );
                bone_cubes(&mut out, at(x + inward * 0.16, 1.0, z), 0.3 + k as f32 * 0.4 + side * 0.1);
            }
        }
    }
    // A hooded knight keeping the corner by Quitmer's straw.
    statue_cubes(&mut out, at(-3.82, 0.0, -2.75), 0.9, 0.82, (16.0, 6.0));

    // ---- the gate in the back wall: squat towers, a round arch on pale piers under a skull
    // keystone, a fixed iron grille over the door, a cornice with the lock lamps, braziers
    gate(&mut out);

    // ---- braziers and lanterns: the room's practical light (their flames are the story's)
    for (i, &b) in BRAZIERS.iter().enumerate() {
        brazier_cubes(&mut out, o + b, BRAZIER_SCALE, false);
        // A deeper bowl: copper sides round the coals, a heap of them glowing in it.
        let s = BRAZIER_SCALE;
        for k in 0..8 {
            let a = k as f32 * TAU / 8.0;
            let side = Vec3::new(a.cos(), 0.0, a.sin());
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(0.05, 0.16, 0.2) * s,
                Quat::from_rotation_y(-a) * Quat::from_rotation_z(-0.45),
                o + b + Vec3::Y * 0.82 * s + side * 0.24 * s,
            );
            block_m(&mut out, m, lin(COPPER) * (0.8 + 0.1 * (k % 2) as f32), SHADE_BRONZE);
        }
        for k in 0..5 {
            let (u, v) = (hash(k as f64, i as f64 + 3.0) as f32 - 0.5, hash(i as f64 + 7.0, k as f64) as f32 - 0.5);
            glowing_block(
                &mut out,
                o + b + Vec3::new(u * 0.3, 0.82, v * 0.3) * s,
                Vec3::splat(0.08 * s),
                lin(0x2a1408),
                lin(EMBER) * (0.9 + u),
            );
        }
        // A pale stone slab under each, scorched.
        block(
            &mut out,
            o + b + Vec3::Y * 0.02,
            Vec3::new(0.7, 0.04, 0.7),
            0.3 + i as f32,
            jit(PALE_STONE[1], 0.2, 0.0) * 0.55,
            SHADE_STONE,
        );
    }
    for &l in &RIB_LANTERNS {
        // Hung from the rib's crown on a long chain.
        let crown = Vec3::new(l.x, PILLAR_H + 1.2, l.z);
        lantern_cubes(&mut out, o + crown, crown.y - l.y - 0.13);
    }
    for &l in &WALL_LANTERNS {
        let dir = Vec3::new(l.x.signum(), 0.0, 0.0) * -1.0;
        let face = Vec3::new(l.x.signum() * (3.35 - 0.42), l.y + 0.2, l.z);
        let hook = bracket_cubes(&mut out, o + face, dir);
        lantern_cubes(&mut out, hook, 0.06);
    }

    // ---- the way they came: a stone frame in the front wall, the rent sealed deep inside it
    arch_cubes(
        &mut out,
        &Arch {
            centre: o + Vec3::new(PORTAL.x, 0.0, FRONT + 0.04),
            across_x: true,
            r: 1.14,
            rise: 1.14,
            spring: PORTAL.y + 0.18,
            depth: 0.56,
            piers: true,
            lintel: true,
            seed: (31.0, 17.0),
        },
    );
    block(&mut out, o + PORTAL + Vec3::new(0.0, 0.25, 0.42), Vec3::new(2.3, 3.0, 0.1), 0.0, lin(0x0a0c0b), SHADE_LIT);
    // A worn step before it.
    block(&mut out, at(0.0, 0.03, FRONT - 0.2), Vec3::new(2.4, 0.08, 0.42), 0.0, jit(PALE_STONE[0], 0.6, 0.0) * 0.7, SHADE_STONE);

    // ---- the old man's corner: his stool, a crate for a table, his pallet of straw and blanket
    old_mans_corner(&mut out);
    // ---- Quitmer's corner: straw, his bowl with a bone, the ring in the wall for his chain
    dogs_corner(&mut out);

    // ---- chains hung on the side walls; bones swept to the walls' feet; moss on the coping
    for &(x, z) in &[(-4.25f32, -0.9f32), (4.25, -0.3), (4.25, 1.6)] {
        for k in 0..12 {
            let y = 3.4 - k as f32 * 0.12;
            let r = if k % 2 == 0 { 0.0 } else { PI / 2.0 };
            block(&mut out, at(x, y, z), Vec3::new(0.02, 0.1, 0.06), r, lin(IRON), SHADE_IRON);
        }
    }
    for (i, &(x, z)) in [(-4.0f32, 0.9f32), (3.9, 2.9), (-3.7, 3.6), (4.05, -1.9), (-1.0, -3.0)].iter().enumerate() {
        bone_cubes(&mut out, at(x, 0.0, z), i as f32 * 0.37 + 0.2);
    }
    for k in 0..6 {
        moss_cap(&mut out, at(-4.0 + k as f32 * 1.6, GATEWAY_H + 0.12, BACK - 0.5), 0.9, (k as f64 * 3.3, 2.0));
    }

    // ---- behind the door: a stair of worn stone going down into red light
    stair(&mut out);
    out
}

/// The crypt gate round the iron door (the door's leaves move: `gateway_door` draws them).
fn gate(out: &mut Vec<Instance>) {
    let o = GATEWAY;
    let at = |x: f32, y: f32, z: f32| o + Vec3::new(x, y, z);
    // A pale threshold under the door.
    block(out, at(0.0, 0.02, BACK + 0.05), Vec3::new(2.2, 0.06, 0.55), 0.0, jit(PALE_STONE[1], 0.3, 0.0) * 0.75, SHADE_STONE);
    // The towers: courses of dressed stone, one pale band, a heavy cap, a watcher on each.
    let course_h = 0.4;
    let courses = 8;
    for side in [-1.0f32, 1.0] {
        let t = at(side * TOWER_X, 0.0, BACK + 0.06);
        block(out, t + Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.92, 0.24, 0.8), 0.0, jit(PALE_STONE[1], 0.4, 0.04) * 0.8, SHADE_STONE);
        for k in 0..courses {
            let h = hash(side as f64 * 3.0 + 5.0, k as f64 * 1.7);
            let c = if k == 4 {
                jit(PALE_STONE[0], h, 0.05) * 0.75
            } else if h < 0.3 {
                jit(UMBER[k % 2], h, 0.08)
            } else {
                jit(CHARCOAL[k % 3], h, 0.1)
            };
            let w = 0.8 + (h as f32 - 0.5) * 0.05;
            let y0 = 0.2 + k as f32 * course_h;
            // Each course split in two blocks, the joint wandering.
            let split = (h as f32 - 0.5) * 0.3;
            for (a, b) in [(-w / 2.0, split), (split, w / 2.0)] {
                let hk = hash(h * 7.0 + a as f64, k as f64);
                let depth = 0.66 + hk as f32 * 0.06;
                block(
                    out,
                    t + Vec3::new((a + b) / 2.0, y0 + course_h / 2.0, (depth - 0.66) / 2.0),
                    Vec3::new(b - a - 0.025, course_h - 0.03, depth),
                    (hk as f32 - 0.5) * 0.04,
                    c * (0.95 + hk as f32 * 0.1),
                    SHADE_STONE,
                );
            }
        }
        block(
            out,
            t + Vec3::new(0.0, TOWER_TOP + 0.08, 0.02),
            Vec3::new(0.98, 0.16, 0.86),
            0.0,
            jit(CHARCOAL[0], 0.5, 0.05) * 0.8,
            SHADE_STONE,
        );
        // Moss on the cap.
        moss_cap(out, t + Vec3::Y * (TOWER_TOP + 0.16), 0.7, (side as f64 * 2.0 + 0.2, 0.3));
        statue_cubes(out, t + Vec3::new(0.0, TOWER_TOP + 0.16, 0.05), 0.0, 0.78, (side as f64 * 7.0 + 3.0, 2.0));
    }
    // The arch on pale piers, a round head of wedge stones.
    let crown = arch_cubes(
        out,
        &Arch {
            centre: at(0.0, 0.0, BACK + 0.06),
            across_x: true,
            r: 1.1,
            rise: 0.75,
            spring: 2.35,
            depth: 0.56,
            piers: true,
            lintel: false,
            seed: (3.0, 7.0),
        },
    );
    // The spandrels over the arch, up to the cornice, and the cornice: a pale band, a frieze,
    // a stepped pale coping.
    for side in [-1.0f32, 1.0] {
        block(out, at(side * 0.92, 3.05, BACK - 0.04), Vec3::new(0.5, 1.4, 0.4), 0.0, jit(CHARCOAL[1], 0.5, 0.05) * 0.85, SHADE_STONE);
    }
    block(out, at(0.0, crown.y - o.y + 0.32, BACK - 0.06), Vec3::new(0.9, 0.3, 0.36), 0.0, jit(UMBER[0], 0.4, 0.05), SHADE_STONE);
    block(out, at(0.0, TOWER_TOP + 0.1, BACK + 0.1), Vec3::new(2.5, 0.18, 0.5), 0.0, jit(PALE_STONE[0], 0.3, 0.04) * 0.78, SHADE_STONE);
    block(out, at(0.0, TOWER_TOP + 0.48, BACK - 0.02), Vec3::new(2.3, 0.58, 0.32), 0.0, jit(CHARCOAL[2], 0.6, 0.05) * 0.8, SHADE_STONE);
    for (k, w) in [2.6f32, 2.0, 1.2].into_iter().enumerate() {
        let c = jit(PALE_STONE[k % 2], k as f64 * 0.3, 0.05) * (0.8 - k as f32 * 0.04);
        block(
            out,
            at(0.0, TOWER_TOP + 0.84 + k as f32 * 0.14, BACK + 0.04 - k as f32 * 0.04),
            Vec3::new(w, 0.14, 0.48 - k as f32 * 0.06),
            0.0,
            c,
            SHADE_STONE,
        );
    }
    block(out, at(0.5, TOWER_TOP + 1.18, BACK + 0.0), Vec3::new(0.5, 0.04, 0.2), 0.2, lin(MOSS[2]), SHADE_SOFT);
    // Iron sockets for the lock lamps along the frieze (their light is the story's).
    for &s in &LOCK_LAMPS {
        block(out, o + s - Vec3::new(0.0, 0.0, 0.05), Vec3::new(0.13, 0.13, 0.05), PI / 4.0, lin(IRON), SHADE_IRON);
    }
    // The skull carved in the keystone, its jaw, embers burning in its sockets.
    let sk = crown + Vec3::new(0.0, 0.02, 0.32);
    block(out, sk, Vec3::new(0.34, 0.3, 0.14), 0.0, lin(BONE) * 0.78, SHADE_STONE);
    block(out, sk + Vec3::new(0.0, -0.19, -0.01), Vec3::new(0.22, 0.1, 0.12), 0.0, lin(BONE) * 0.7, SHADE_STONE);
    block(out, sk + Vec3::new(0.0, -0.06, 0.075), Vec3::new(0.04, 0.05, 0.02), 0.0, lin(0x0e0c0a), SHADE_LIT);
    for side in [-1.0f32, 1.0] {
        glowing_block(out, sk + Vec3::new(side * 0.075, 0.02, 0.07), Vec3::new(0.08, 0.07, 0.02), lin(0x300808), lin(0xff2a10) * 1.6);
    }
    // The fixed iron grille filling the arch's head over the door leaves.
    let rail = 2.44;
    block(out, at(0.0, rail, BACK - 0.08), Vec3::new(1.98, 0.06, 0.08), 0.0, lin(IRON), SHADE_IRON);
    for i in 0..9 {
        let x = -0.84 + i as f32 * 0.21;
        let top = 2.35 + 0.1 + ((1.0 - (x / 0.98).powi(2)).max(0.0)).sqrt() * 0.75 - 0.1;
        block(out, at(x, (rail + top) / 2.0, BACK - 0.08), Vec3::new(0.035, top - rail, 0.035), 0.0, lin(IRON), SHADE_IRON);
    }
    for k in 0..7 {
        let a = PI * (k as f32 + 1.0) / 8.0;
        let p = at(-a.cos() * 0.5, rail + 0.04 + a.sin() * 0.36, BACK - 0.08);
        block(out, p, Vec3::new(0.05, 0.05, 0.05), PI / 4.0, lin(IRON), SHADE_IRON);
    }
}

/// The old man's corner, right of the gate: a three-legged stool, a crate he uses for a table
/// with a mug and a heel of bread, his pallet of straw under a folded blanket, a pail.
fn old_mans_corner(out: &mut Vec<Instance>) {
    let o = GATEWAY;
    let wood = lin(0x5e4430);
    let st = o + Vec3::new(2.05, 0.0, -2.75);
    let q = Quat::from_rotation_y(0.3);
    block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(0.4, 0.06, 0.4), q, st + Vec3::Y * 0.42), wood, SHADE_WOOD);
    for k in 0..3 {
        let a = k as f32 / 3.0 * TAU + 0.3;
        let m = Mat4::from_scale_rotation_translation(
            Vec3::new(0.05, 0.42, 0.05),
            Quat::from_rotation_z(a.cos() * 0.12) * Quat::from_rotation_x(-a.sin() * 0.12),
            st + Vec3::new(a.cos() * 0.13, 0.2, a.sin() * 0.13),
        );
        block_m(out, m, wood * 0.85, SHADE_WOOD);
    }
    // A cushion of old cloth on it.
    block(out, st + Vec3::Y * 0.47, Vec3::new(0.34, 0.04, 0.3), 0.2, lin(BANNER[1]), SHADE_CLOTH);
    // The crate, its planks and a mug and bread on it.
    let cr = o + Vec3::new(2.95, 0.0, -2.95);
    for k in 0..3 {
        block(
            out,
            cr + Vec3::new(0.0, 0.08 + k as f32 * 0.15, 0.0),
            Vec3::new(0.5, 0.13, 0.4),
            0.15,
            wood * (0.8 + k as f32 * 0.08),
            SHADE_WOOD,
        );
    }
    block(out, cr + Vec3::new(0.08, 0.5, 0.02), Vec3::new(0.08, 0.1, 0.08), 0.0, lin(COPPER) * 0.8, SHADE_BRONZE);
    block(out, cr + Vec3::new(-0.1, 0.48, -0.04), Vec3::new(0.14, 0.06, 0.09), 0.5, lin(0x8a6034), SHADE_SOFT);
    // The pallet of straw along the wall, a blanket folded at its foot.
    let pal = o + Vec3::new(3.65, 0.0, -2.5);
    block(out, pal + Vec3::Y * 0.06, Vec3::new(0.8, 0.12, 1.3), 0.0, lin(0x8a7444) * 0.7, SHADE_SOFT);
    for i in 0..40 {
        let (a, b) = (hash(i as f64, 61.0) as f32, hash(i as f64, 67.0) as f32);
        block(
            out,
            pal + Vec3::new((a - 0.5) * 0.85, 0.12 + b * 0.03, (b - 0.5) * 1.3),
            Vec3::new(0.22, 0.012, 0.025),
            a * 7.0,
            lin(0xb09a50) * (0.55 + 0.4 * b),
            SHADE_SOFT,
        );
    }
    block(out, pal + Vec3::new(0.0, 0.17, 0.4), Vec3::new(0.78, 0.08, 0.45), 0.05, lin(BANNER[0]) * 0.85, SHADE_CLOTH);
    block(out, pal + Vec3::new(0.0, 0.215, 0.4), Vec3::new(0.7, 0.02, 0.06), 0.05, lin(AMBER) * 0.6, SHADE_CLOTH);
    // A wooden pail with an iron hoop.
    let pail = o + Vec3::new(2.5, 0.0, -1.85);
    block(out, pail + Vec3::Y * 0.12, Vec3::new(0.2, 0.24, 0.2), 0.4, wood * 0.9, SHADE_WOOD);
    block(out, pail + Vec3::Y * 0.17, Vec3::new(0.215, 0.03, 0.215), 0.4, lin(IRON), SHADE_IRON);
}

/// Where Quitmer's chain is fixed in the wall, left of the gate.
pub(super) const CHAIN_RING: Vec3 = Vec3::new(-2.35, 0.62, BACK + 0.22);

/// Quitmer's corner, left of the gate: his straw, a battered iron bowl with a bone in it, and
/// the ring in the wall his chain hangs from.
fn dogs_corner(out: &mut Vec<Instance>) {
    let o = GATEWAY;
    let at = |x: f32, y: f32, z: f32| o + Vec3::new(x, y, z);
    block(out, at(-1.95, 0.025, -2.55), Vec3::new(1.2, 0.05, 0.9), 0.1, lin(0x8a7444) * 0.6, SHADE_SOFT);
    for i in 0..80 {
        let (a, b) = (hash(i as f64, 5.0) as f32, hash(i as f64, 8.0) as f32);
        let p = at(-1.95 + (a - 0.5) * 1.3, 0.05 + b * 0.04, -2.55 + (b - 0.5) * 1.0);
        block(out, p, Vec3::new(0.24, 0.015, 0.025), a * 7.0, lin(0xb09a50) * (0.55 + 0.45 * b), SHADE_SOFT);
    }
    let bowl = at(-0.95, 0.0, -2.85);
    block(out, bowl + Vec3::Y * 0.05, Vec3::new(0.32, 0.1, 0.32), 0.0, lin(IRON) * 1.4, SHADE_IRON);
    block(out, bowl + Vec3::Y * 0.095, Vec3::new(0.24, 0.012, 0.24), 0.0, lin(0x1a1614), SHADE_LIT);
    let rot = 0.6;
    block(out, bowl + Vec3::new(0.05, 0.12, 0.02), Vec3::new(0.22, 0.04, 0.04), rot, lin(BONE) * 0.9, SHADE_STONE);
    for side in [-1.0f32, 1.0] {
        let end = bowl + Vec3::new(0.05, 0.12, 0.02) + Quat::from_rotation_y(rot) * Vec3::new(side * 0.12, 0.0, 0.0);
        block(out, end, Vec3::splat(0.06), 0.0, lin(BONE), SHADE_STONE);
    }
    // The ring, on an iron plate in the stone.
    let ring = o + CHAIN_RING;
    block(out, ring + Vec3::new(0.0, 0.1, -0.04), Vec3::new(0.14, 0.14, 0.04), 0.0, lin(IRON), SHADE_IRON);
    for k in 0..12 {
        let a = k as f32 / 12.0 * TAU;
        block(out, ring + Vec3::new(a.cos() * 0.09, a.sin() * 0.09, 0.0), Vec3::splat(0.035), 0.0, lin(IRON) * 1.3, SHADE_IRON);
    }
}

/// Behind the door: a stair of worn stone between walls of the same masonry, going down into red light.
fn stair(out: &mut Vec<Instance>) {
    let o = GATEWAY;
    let at = |x: f32, y: f32, z: f32| o + Vec3::new(x, y, z);
    for k in 0..10 {
        let z = -3.95 - k as f32 * 0.42;
        let y = -0.12 - k as f32 * 0.26;
        let hk = hash(k as f64, 3.0);
        block(
            out,
            at(0.0, y, z),
            Vec3::new(1.9, 0.26, 0.44),
            (hk as f32 - 0.5) * 0.02,
            jit(PALE_STONE[k % 2], hk, 0.06) * 0.6,
            SHADE_STONE,
        );
        // Worn hollow in the middle of each tread.
        block(out, at(0.0, y + 0.131, z + 0.05), Vec3::new(0.7, 0.01, 0.2), 0.0, jit(FLAG[1], hk, 0.05) * 0.7, SHADE_FLAG);
        for side in [-1.0f32, 1.0] {
            for c in 0..5 {
                let h = hash(k as f64 * 3.0 + side as f64, c as f64);
                let bh = 0.56;
                block(
                    out,
                    at(side * (1.15 + h as f32 * 0.04), y + 0.2 + c as f32 * bh + bh / 2.0, z),
                    Vec3::new(0.4, bh - 0.03, 0.42),
                    (h as f32 - 0.5) * 0.04,
                    jit(CHARCOAL[(k + c) % 3], h, 0.12) * 0.75,
                    SHADE_STONE,
                );
            }
        }
        block(out, at(0.0, y + 3.1, z), Vec3::new(2.3, 0.3, 0.44), 0.0, lin(0x0e0a09), SHADE_LIT);
    }
    glowing_block(out, at(0.0, -2.9, -8.4), Vec3::new(1.9, 0.1, 1.4), lin(0x2a0400), lin(0xff3a0a) * 2.2);
}

/// Low mist over the gateway's floor: sheets whose drifting noise the shader draws.
pub(super) fn gateway_mist() -> Vec<Instance> {
    mist_sheets(GATEWAY, (-4, 4), (-3, 4), &[(0.06, 0.28), (0.2, 0.18)], lin(MIST_TEAL) * 0.8, |_x, _z| 1.0)
}

/// Low mist over a set: sheets whose drifting noise the shader draws.
pub(super) fn mist_sheets(
    origin: Vec3,
    x: (i32, i32),
    z: (i32, i32),
    layers: &[(f32, f32)],
    colour: Vec3,
    thick: impl Fn(i32, i32) -> f32,
) -> Vec<Instance> {
    let mut out = Vec::new();
    for gx in x.0..=x.1 {
        for gz in z.0..=z.1 {
            let k = thick(gx, gz);
            if k <= 0.0 {
                continue;
            }
            for (i, &(lift, alpha)) in layers.iter().enumerate() {
                let m =
                    Mat4::from_translation(origin + Vec3::new(gx as f32, lift, gz as f32)) * Mat4::from_scale(Vec3::new(1.0, 0.002, 1.0));
                out.push(Instance::new(m, colour, alpha * k, Vec3::new(20.0 + i as f32, lift, 0.0), SHADE_CLOUD));
            }
        }
    }
    out
}
