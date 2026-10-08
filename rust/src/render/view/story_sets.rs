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
use super::story::{GATEWAY, GRAVEYARD};
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
    let gate_gap = [(0.0, 2.04, 0.0, TOWER_TOP + 0.16, 0.0), (0.0, 1.3, 0.0, TOWER_TOP + 1.22, 0.0)];
    masonry(&mut out, &Wall { openings: &gate_gap, ..wall(at(0.0, 0.0, BACK - 0.46), (0, 1), 4.62, 11.0) });
    for side in [-1.0f32, 1.0] {
        masonry(&mut out, &wall(at(side * (SIDE + 0.46), 0.0, 0.3), (-side as i32, 0), 3.75, 23.0 + side as f64 * 5.0));
    }
    // The front wall, with the way they came in it.
    let portal_gap = [(0.0, 1.14, 0.0, PORTAL.y + 0.28, 1.14)];
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

/// The old man's stool (gateway coordinates), in his corner beside where he stands: a prop.
pub(super) const STOOL: Vec3 = Vec3::new(2.05, 0.0, -2.75);

/// The old man's corner, right of the gate: a three-legged stool, a crate he uses for a table
/// with a mug and a heel of bread, his pallet of straw under a folded blanket, a pail.
fn old_mans_corner(out: &mut Vec<Instance>) {
    let o = GATEWAY;
    let wood = lin(0x5e4430);
    let st = o + STOOL;
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

// ================================================================= the graveyard

/// The churchyard wall's line, its height, and the gate piers either side of the path.
const YARD_WALL: f32 = -2.0;
const YARD_WALL_H: f32 = 1.0;
const PIER_X: f32 = 1.34;
/// The lantern on its post beside the lane outside the gate: the middle of its glass.
pub(super) const LANE_LANTERN: Vec3 = Vec3::new(1.45, 1.72, 4.4);
/// The chapel beyond the graves: where it stands, and its lit windows (middles).
const CHAPEL: Vec3 = Vec3::new(2.0, 0.0, -17.5);
pub(super) const CHAPEL_WINDOWS: [Vec3; 2] = [Vec3::new(0.4, 1.9, -15.58), Vec3::new(3.6, 1.9, -15.58)];

/// Grass in three tones of the board's moss and olive, and the earth under it.
const TURF: [u32; 3] = [0x4a5434, 0x56603a, 0x3e4a30];
const EARTH: [u32; 2] = [0x4e4032, 0x5a4a38];
const LEAVES: [u32; 3] = [0x8a4a22, 0x9a6a2a, 0x5e3a1e];
const BARK: u32 = 0x2e2620;
/// Wrought iron, rusted and, here and there, green.
const RUST: u32 = 0x5a3a26;

/// Dusk over the graveyard (the meadow's dusk, in the board's palette): a low amber sun behind the
/// graves, copper and mist-teal grey above it; it dims (`darken`) and the fog closes in (`fog`)
/// when Krampus comes.
pub(super) fn dusk(darken: f32, fog: f32) -> Atmosphere {
    let mix = |a: f32, b: f32, t: f32| a + (b - a) * t;
    Atmosphere {
        background: 0x58544c,
        fog_near: mix(12.0, 1.5, fog),
        fog_far: mix(36.0, 5.0, fog),
        hemi_sky: lin(0x8aa6a0) * 0.5 * (1.0 - 0.35 * darken),
        hemi_ground: lin(0x4a4230) * 0.45,
        ambient: lin(0x6e7470) * 0.22,
        sun_dir: Vec3::new(-0.72, 0.3, -0.62).normalize(),
        sun: lin(0xffa062) * 2.1 * (1.0 - 0.55 * darken),
        exposure: 1.2,
        night_sky: 2.0,
        bloom: 0.8,
        moon_view: Vec3::new(-0.62, 0.1, -0.78).normalize(),
        cloud_tint: lin(0x3e3a36),
    }
}

/// The graveyard's ground haze: low and thick among the graves (RP19), thickening as the fog comes.
pub(super) fn dusk_haze(fog: f32) -> Haze {
    Haze { density: 0.028 + 0.25 * fog, floor: 0.0, falloff: 0.5 + 0.6 * fog }
}

/// How high the ground stands at (x, z): uneven, but level where the story plays (the path, the
/// friends' game, the way Krampus comes and goes), and settled into the churchyard's slope.
fn ground(x: f32, z: f32) -> f32 {
    let wave = (x * 0.9 + z * 0.4).sin() * 0.5 + (x * 0.35 - z * 1.1).sin() * 0.3 + ((x + z) * 1.7).sin() * 0.2;
    let busy_x = smooth(2.2, 4.0, x.abs());
    let busy_z = if (-10.0..4.0).contains(&z) { 1.0 } else { 0.0 };
    let quiet = 1.0 - (1.0 - busy_x) * busy_z;
    // The wall's foot and the path stay level.
    let wall = smooth(0.2, 1.0, (z - YARD_WALL).abs());
    wave * 0.07 * quiet * wall
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The edge of an old graveyard at dusk, everything that stays put: uneven turf with a trodden
/// path and a lane, the churchyard wall of dressed stone with its rusted railings, the gate's
/// piers, the graves (headstones, crosses, obelisks, chest tombs, sunk slabs, a railed family
/// plot), a mausoleum, the chapel beyond with its lit windows, bare trees, and outside the wall the
/// friends' patch of trodden grass, old graves at the yard's edge, a wayside cross, a field wall
/// and the lantern on its post.
pub(super) fn graveyard_set() -> Vec<Instance> {
    let mut out = Vec::new();
    let o = GRAVEYARD;
    let at = |x: f32, y: f32, z: f32| o + Vec3::new(x, y, z);

    // ---- the ground in half-unit cells: turf, the path's earth, the friends' trodden patch
    for gx in -36..=36 {
        for gz in -40..=36 {
            let (x, z) = (gx as f32 * 0.5, gz as f32 * 0.5);
            let k = hash(gx as f64 * 1.3 + 0.5, gz as f64 * 2.1 + 3.0) as f32;
            let y = ground(x, z);
            // The path: up to the gate and on to the chapel; the lane outside bends off east.
            let lane_x = if z > 5.0 { (z - 5.0).powi(2) * 0.12 } else { 0.0 };
            let path = ((x - lane_x).abs() < 0.55 && z > -16.0) || (z > 9.5 && (z - 9.5) < 0.8 && x > 0.0);
            let worn = ((x + 0.25).powi(2) / 1.8 + (z - 1.45).powi(2) / 1.4) < 1.0;
            let (c, shade) = if path {
                (jit(EARTH[(k * 2.0) as usize % 2], k as f64, 0.12), SHADE_LIT)
            } else if worn {
                (jit(TURF[1], k as f64, 0.1).lerp(lin(EARTH[1]), 0.45 + 0.3 * k), SHADE_GRASS)
            } else {
                let inside = z < YARD_WALL;
                let t = jit(TURF[(k * 3.0) as usize % 3], k as f64, 0.14);
                (if inside { t * 0.9 } else { t }, SHADE_GRASS)
            };
            block(&mut out, at(x, y - 0.15, z), Vec3::new(0.5, 0.3, 0.5), 0.0, c, shade);
        }
    }
    // Beyond, out to the fog, the fields in coarser cells, falling away a little.
    for gx in -20..=20 {
        for gz in -20..=16 {
            let (x, z) = (gx as f32 * 2.0, gz as f32 * 2.0);
            if x.abs() < 18.5 && (-20.5..18.5).contains(&z) {
                continue;
            }
            let k = hash(gx as f64 * 3.3 + 1.0, gz as f64 * 1.9 + 7.0) as f32;
            let fall = ((x.abs() - 18.0).max(0.0) + (z.abs() - 19.0).max(0.0)) * 0.03;
            let c = jit(TURF[(k * 3.0) as usize % 3], k as f64, 0.15) * 0.85;
            block(&mut out, at(x, -0.15 - fall + (k - 0.5) * 0.06, z), Vec3::new(2.0, 0.3, 2.0), 0.0, c, SHADE_GRASS);
        }
    }
    // Flat stones sunk along the path, and gravel.
    for i in 0..44 {
        let (a, b) = (hash(i as f64, 7.0) as f32, hash(i as f64, 13.0) as f32);
        let z = 8.6 - i as f32 * 0.55;
        if (-2.4..-1.6).contains(&z) {
            continue;
        }
        let c = jit(PALE_STONE[i % 2], b as f64, 0.08) * 0.75;
        block(
            &mut out,
            at((a - 0.5) * 0.6, 0.012, z + (b - 0.5) * 0.2),
            Vec3::new(0.3 + a * 0.22, 0.04, 0.26 + b * 0.14),
            a * 3.0,
            c,
            SHADE_STONE,
        );
    }
    for i in 0..160 {
        let (a, b) = (hash(i as f64, 17.0) as f32, hash(i as f64, 19.0) as f32);
        let z = 9.0 - b * 24.0;
        block(
            &mut out,
            at((a - 0.5) * 1.0, 0.006, z),
            Vec3::new(0.05, 0.02, 0.04),
            a * 9.0,
            jit(PALE_STONE[1], a as f64, 0.15) * 0.6,
            SHADE_STONE,
        );
    }

    // ---- the churchyard wall: dressed stone both faces, a pale coping, rusted railings on it
    for side in [-1.0f32, 1.0] {
        let (x0, x1) = (1.64f32, 12.4f32);
        let mid = side * (x0 + x1) / 2.0;
        let half = (x1 - x0) / 2.0;
        block(
            &mut out,
            at(mid, YARD_WALL_H / 2.0, YARD_WALL),
            Vec3::new(x1 - x0, YARD_WALL_H, 0.3),
            0.0,
            jit(CHARCOAL[1], 0.4, 0.0) * 0.6,
            SHADE_STONE,
        );
        for (dir, seed) in [(1, 3.0), (-1, 9.0)] {
            masonry(
                &mut out,
                &Wall {
                    centre: at(mid, 0.0, YARD_WALL),
                    dir: (0, dir),
                    half,
                    base: o.y,
                    height: YARD_WALL_H,
                    courses: 3,
                    seed: (seed + side as f64 * 5.0, 2.0),
                    len: (0.32, 0.3),
                    depth: (0.05, 0.06),
                    face: 0.14,
                    wet: false,
                    ruined: false,
                    foot: Some(o.y),
                    openings: &[],
                },
            );
        }
        // The coping: pale slabs, a touch askew, moss gathered on some.
        let mut x = x0;
        let mut k = 0;
        while x < x1 {
            let h = hash(x as f64 * 3.1, side as f64 + 4.0);
            let len = 0.5 + h as f32 * 0.3;
            let w = (x1 - x).min(len);
            block(
                &mut out,
                at(side * (x + w / 2.0), YARD_WALL_H + 0.05, YARD_WALL),
                Vec3::new(w - 0.02, 0.1, 0.5),
                (h as f32 - 0.5) * 0.03,
                jit(PALE_STONE[k % 2], h, 0.06) * 0.78,
                SHADE_STONE,
            );
            if h > 0.6 {
                moss_cap(&mut out, at(side * (x + w / 2.0), YARD_WALL_H + 0.1, YARD_WALL), 0.4, (x as f64 * 2.0, side as f64));
            }
            x += len;
            k += 1;
        }
        railings(&mut out, side, x0 + 0.1, x1 - 0.1);
    }
    // ---- the gate's piers: dressed stone in courses on a pale plinth, a moulded cap and a ball
    for side in [-1.0f32, 1.0] {
        pier(&mut out, at(side * PIER_X, 0.0, YARD_WALL), side as f64);
    }
    // The iron arch over the gate, scrolled, with a cross at its crown.
    iron_arch(&mut out);
    // The lantern's bracket, out from the right pier over the path.
    let iron = lin(IRON);
    block(&mut out, at(PIER_X, 1.95, -1.55), Vec3::new(0.05, 0.05, 0.5), 0.0, iron, SHADE_IRON);
    block_m(
        &mut out,
        Mat4::from_scale_rotation_translation(Vec3::new(0.03, 0.22, 0.03), Quat::from_rotation_x(0.6), at(PIER_X, 1.83, -1.33)),
        iron,
        SHADE_IRON,
    );

    // ---- the graves: rows of them with a walk up the middle, a railed family plot, tombs
    for row in 0..8 {
        let z = -3.2 - row as f32 * 1.3;
        let mut x = -10.4 + (row % 2) as f32 * 0.7;
        while x < 10.4 {
            let k = hash(x as f64 * 2.3 + row as f64, row as f64 * 7.1) as f32;
            let (px, pz) = (x + (k - 0.5) * 0.4, z + (hash(x as f64, row as f64 + 50.0) as f32 - 0.5) * 0.35);
            x += 1.25 + k * 0.45;
            if px.abs() < 1.0 && row < 7 {
                continue;
            }
            // (The mausoleum's ground, and the plot's.)
            if (-6.2..-2.6).contains(&px) && (-9.8..-6.6).contains(&pz) || (3.6..6.6).contains(&px) && (-8.4..-6.0).contains(&pz) {
                continue;
            }
            grave(&mut out, at(px, ground(px, pz), pz), k, hash(px as f64 * 9.0, pz as f64 * 3.0) as f32);
        }
    }
    family_plot(&mut out, at(5.1, 0.0, -7.2));
    mausoleum(&mut out, at(-4.4, 0.0, -8.2));
    chapel(&mut out, o + CHAPEL);

    // ---- bare trees in the churchyard and outside it
    for (i, &(x, z, height)) in [
        (-5.4f32, -3.9f32, 3.4f32),
        (5.1, -5.7, 3.9),
        (-8.3, -10.6, 4.4),
        (7.8, -11.8, 4.1),
        (-3.9, 1.2, 4.6),
        (9.2, 0.4, 3.6),
        (-11.5, 6.5, 5.0),
        (12.0, 8.0, 4.4),
        (-7.0, 12.5, 5.2),
        (4.5, 14.0, 4.8),
    ]
    .iter()
    .enumerate()
    {
        dead_tree(&mut out, at(x, ground(x, z), z), height, i as f64 * 7.3 + 1.0);
    }

    // ---- outside: old graves at the yard's edge, a wayside cross, a field wall, the lantern's post
    for (i, &(x, z)) in [
        (-5.6f32, 2.6f32),
        (-6.8, 3.4),
        (-4.9, 4.4),
        (-7.6, 1.9),
        (6.0, 3.0),
        (7.1, 2.2),
        (5.2, 4.3),
        (-9.0, 4.6),
        (8.6, 4.8),
        (-2.2, 3.7),
        (2.7, 5.3),
        (-3.3, 6.3),
        (3.6, 3.4),
    ]
    .iter()
    .enumerate()
    {
        let k = hash(i as f64 * 3.7, 41.0) as f32;
        // (Old and sunk: slabs and leaning headstones, no crosses.)
        let k = if (0.4..0.8).contains(&k) { k * 0.3 } else { k };
        grave(&mut out, at(x, ground(x, z), z), k, 0.7 + 0.3 * hash(i as f64, 5.0) as f32);
    }
    wayside_cross(&mut out, at(-2.6, ground(-2.6, 5.4), 5.4));
    field_wall(&mut out);
    lantern_post(&mut out, o + Vec3::new(LANE_LANTERN.x, 0.0, LANE_LANTERN.z));
    // A bench by the lane, under the lantern, its planks grey with weather.
    let bench = at(2.25, 0.0, 4.1);
    let wood = lin(0x5a4a3c);
    for (k, z) in [-0.12f32, 0.0, 0.12].into_iter().enumerate() {
        block(
            &mut out,
            bench + Vec3::new(0.0, 0.42, z),
            Vec3::new(1.3, 0.04, 0.11),
            0.02 * k as f32,
            wood * (0.85 + 0.08 * k as f32),
            SHADE_WOOD,
        );
    }
    for x in [-0.55f32, 0.55] {
        block(&mut out, bench + Vec3::new(x, 0.2, 0.0), Vec3::new(0.08, 0.4, 0.36), 0.0, wood * 0.75, SHADE_WOOD);
    }
    block(&mut out, bench + Vec3::new(0.0, 0.68, -0.2), Vec3::new(1.3, 0.16, 0.05), 0.0, wood * 0.8, SHADE_WOOD);

    // ---- tussocks of long grass, and dead leaves blown against everything
    for i in 0..520 {
        let (a, b) = (hash(i as f64, 3.3) as f32, hash(i as f64, 5.9) as f32);
        let (x, z) = ((a - 0.5) * 30.0, -15.0 + b * 30.0);
        if x.abs() < 0.8 || (x.abs() < 2.0 && (-0.2..2.8).contains(&z)) || (z - YARD_WALL).abs() < 0.35 {
            continue;
        }
        let y = ground(x, z);
        let tone = jit(TURF[i % 3], b as f64, 0.1) * 1.25 + lin(0x6a6030) * 0.2;
        for k in 0..5 {
            let lean = Quat::from_rotation_z((hash(i as f64, k as f64) as f32 - 0.5) * 0.8)
                * Quat::from_rotation_x((hash(k as f64, i as f64) as f32 - 0.5) * 0.7);
            let tall = 0.12 + hash(i as f64 * 3.0, k as f64) as f32 * 0.22;
            let p = at(x + (k as f32 - 2.0) * 0.03, y + tall / 2.0, z + ((k * 7) % 5) as f32 * 0.012);
            block_m(
                &mut out,
                Mat4::from_scale_rotation_translation(Vec3::new(0.02, tall, 0.02), lean, p),
                tone * (0.8 + 0.1 * k as f32),
                SHADE_SOFT,
            );
        }
    }
    for i in 0..700 {
        let (a, b) = (hash(i as f64, 71.0) as f32, hash(i as f64, 73.0) as f32);
        // Thickest by the wall and round the trees; a scatter everywhere.
        let (x, z) = if i % 3 == 0 { ((a - 0.5) * 24.0, YARD_WALL + 0.3 + b * 0.6) } else { ((a - 0.5) * 22.0, -12.0 + b * 20.0) };
        let c = jit(LEAVES[i % 3], b as f64, 0.15);
        let m = Mat4::from_scale_rotation_translation(
            Vec3::new(0.07, 0.012, 0.05),
            Quat::from_rotation_y(a * 9.0) * Quat::from_rotation_x((b - 0.5) * 0.6),
            at(x, ground(x, z) + 0.01, z),
        );
        block_m(&mut out, m, c, SHADE_SOFT);
    }
    out
}

/// A gate pier at `p` (its foot on the wall's line): a pale plinth, a shaft of dressed stone in
/// courses (each course two blocks, the joint wandering), a moulded cap, and a stone ball on it.
fn pier(out: &mut Vec<Instance>, p: Vec3, seed: f64) {
    block(out, p + Vec3::Y * 0.15, Vec3::new(0.72, 0.3, 0.72), 0.0, jit(PALE_STONE[1], seed, 0.04) * 0.75, SHADE_STONE);
    block(out, p + Vec3::Y * 0.33, Vec3::new(0.64, 0.06, 0.64), 0.0, jit(PALE_STONE[0], seed, 0.04) * 0.7, SHADE_STONE);
    let course_h = 0.34;
    for k in 0..6 {
        let h = hash(seed * 3.0 + 7.0, k as f64 * 1.7);
        let c = if h < 0.25 {
            jit(UMBER[k % 2], h, 0.08)
        } else if k == 3 {
            jit(PALE_STONE[0], h, 0.05) * 0.72
        } else {
            jit(CHARCOAL[k % 3], h, 0.1) * 1.15
        };
        let split = (h as f32 - 0.5) * 0.25;
        let y = 0.36 + k as f32 * course_h + course_h / 2.0;
        // Alternate courses split across x and across z, as a mason lays them.
        for (a, b) in [(-0.27f32, split), (split, 0.27)] {
            let hk = hash(h * 9.0 + a as f64, seed);
            let along_x = k % 2 == 0;
            let (cx, sx, sz) = if along_x { ((a + b) / 2.0, b - a - 0.02, 0.54) } else { (0.0, 0.54, b - a - 0.02) };
            let cz = if along_x { 0.0 } else { (a + b) / 2.0 };
            block(
                out,
                p + Vec3::new(cx, y - (hk as f32) * 0.01, cz),
                Vec3::new(sx + hk as f32 * 0.02, course_h - 0.025, sz + hk as f32 * 0.02),
                (hk as f32 - 0.5) * 0.03,
                c * (0.95 + hk as f32 * 0.1),
                SHADE_STONE,
            );
        }
    }
    block(out, p + Vec3::Y * 2.4, Vec3::new(0.74, 0.12, 0.74), 0.0, jit(PALE_STONE[0], seed + 1.0, 0.04) * 0.8, SHADE_STONE);
    block(out, p + Vec3::Y * 2.5, Vec3::new(0.52, 0.08, 0.52), 0.0, jit(PALE_STONE[1], seed + 2.0, 0.04) * 0.75, SHADE_STONE);
    // The ball, cut in facets, mossed on top.
    let ball = p + Vec3::Y * 2.73;
    let stone = jit(PALE_STONE[0], seed + 3.0, 0.05) * 0.72;
    for r in [
        Quat::IDENTITY,
        Quat::from_rotation_y(PI / 4.0) * Quat::from_rotation_x(PI / 4.0),
        Quat::from_rotation_z(PI / 4.0) * Quat::from_rotation_y(PI / 4.0),
    ] {
        block_m(out, Mat4::from_scale_rotation_translation(Vec3::splat(0.3), r, ball), stone, SHADE_STONE);
    }
    block(out, ball + Vec3::Y * 0.17, Vec3::new(0.16, 0.03, 0.14), seed as f32, lin(MOSS[1]), SHADE_SOFT);
    // Lichen down the shaft's weather side.
    for k in 0..5 {
        let h = hash(seed * 5.0, k as f64) as f32;
        block(
            out,
            p + Vec3::new((h - 0.5) * 0.4, 0.6 + h * 1.5, 0.285),
            Vec3::new(0.08, 0.06, 0.01),
            0.0,
            jit(VERDIGRIS, h as f64, 0.15) * 1.2,
            SHADE_SOFT,
        );
    }
}

/// The railings along the wall's top on `side`, from `x0` to `x1` out from the gate: spear-headed
/// bars between two rails, rusted, a few bent, one or two green.
fn railings(out: &mut Vec<Instance>, side: f32, x0: f32, x1: f32) {
    let at = |x: f32, y: f32| GRAVEYARD + Vec3::new(side * x, y, YARD_WALL);
    let top = YARD_WALL_H + 0.1;
    let mut x = x0;
    let mut i = 0;
    while x < x1 {
        let h = hash(x as f64 * 5.0, side as f64);
        let c = lin(IRON).lerp(lin(RUST), (h as f32 * 1.6).min(1.0) * 0.45);
        let bent = h > 0.96;
        let tilt = if bent { Quat::from_rotation_z(side * 0.12) } else { Quat::IDENTITY };
        let len = 0.7;
        block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(0.035, len, 0.035), tilt, at(x, top + len / 2.0)), c, SHADE_IRON);
        let tip = at(x, top + len + 0.02) + tilt * Vec3::new(0.0, 0.0, 0.0);
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.06, 0.06, 0.03), tilt * Quat::from_rotation_z(PI / 4.0), tip),
            c,
            SHADE_IRON,
        );
        // A collar on every fourth bar.
        if i % 4 == 0 {
            block(out, at(x, top + 0.45), Vec3::new(0.06, 0.04, 0.06), 0.0, c, SHADE_IRON);
        }
        x += 0.16;
        i += 1;
    }
    for y in [top + 0.15, top + 0.62] {
        block(out, at((x0 + x1) / 2.0, y), Vec3::new(x1 - x0, 0.035, 0.045), 0.0, lin(IRON).lerp(lin(RUST), 0.4), SHADE_IRON);
    }
}

/// The wrought-iron arch over the gate between the piers, scrolled, with a cross at its crown.
fn iron_arch(out: &mut Vec<Instance>) {
    let at = |x: f32, y: f32| GRAVEYARD + Vec3::new(x, y, YARD_WALL);
    let c = lin(IRON).lerp(lin(RUST), 0.35);
    for i in 0..=40 {
        let t = i as f32 / 40.0;
        let x = -1.08 + 2.16 * t;
        let y = 2.28 + (t * PI).sin() * 0.5;
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.07, 0.045, 0.045), Quat::from_rotation_z((0.5 - t) * 1.2), at(x, y)),
            c,
            SHADE_IRON,
        );
        if i % 5 == 0 && i > 0 && i < 40 {
            block(out, at(x, (2.24 + y) / 2.0), Vec3::new(0.025, y - 2.24, 0.025), 0.0, c, SHADE_IRON);
        }
    }
    block(out, at(0.0, 2.2), Vec3::new(2.2, 0.04, 0.045), 0.0, c, SHADE_IRON);
    for k in 0..10 {
        let a = k as f32 / 10.0 * TAU;
        for side in [-1.0f32, 1.0] {
            block(out, at(side * 0.45 + a.cos() * 0.1, 2.45 + a.sin() * 0.1), Vec3::splat(0.032), 0.0, c, SHADE_IRON);
        }
    }
    block(out, at(0.0, 3.0), Vec3::new(0.045, 0.4, 0.045), 0.0, c, SHADE_IRON);
    block(out, at(0.0, 3.08), Vec3::new(0.22, 0.045, 0.045), 0.0, c, SHADE_IRON);
}

/// One grave at `p`, its kind and weathering picked by `k` and `j`: a rounded headstone, a cross
/// (sometimes ringed), an obelisk, a chest tomb, or an old slab sunk and broken.
pub(super) fn grave(out: &mut Vec<Instance>, p: Vec3, k: f32, j: f32) {
    let stone = if j > 0.5 {
        jit(PALE_STONE[(k * 7.0) as usize % 2], j as f64, 0.08) * (0.62 + 0.15 * j)
    } else {
        jit(CHARCOAL[(k * 9.0) as usize % 3], j as f64, 0.1) * 1.25
    };
    let lean = Quat::from_rotation_z((k - 0.5) * 0.25) * Quat::from_rotation_x((j - 0.5) * 0.22);
    let put = |out: &mut Vec<Instance>, off: Vec3, s: Vec3, c: Vec3| {
        block_m(out, Mat4::from_scale_rotation_translation(s, lean, p + lean * off), c, SHADE_STONE);
    };
    let moss = |out: &mut Vec<Instance>, off: Vec3, w: f32| {
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(w, 0.03, 0.12), lean, p + lean * off),
            jit(MOSS[(j * 7.0) as usize % 3], k as f64, 0.1),
            SHADE_SOFT,
        );
    };
    // A low mound of earth before most, grassed over.
    if j > 0.25 {
        block(
            out,
            p + Vec3::new(0.0, 0.03, 0.5),
            Vec3::new(0.46, 0.09, 0.8),
            (k - 0.5) * 0.2,
            jit(TURF[2], k as f64, 0.1) * 0.85,
            SHADE_GRASS,
        );
    }
    match (k * 6.0) as i32 {
        0 | 1 => {
            // A rounded headstone: stepped to its curve, a worn inscription, lichen and moss.
            put(out, Vec3::new(0.0, 0.3, 0.0), Vec3::new(0.48, 0.6, 0.11), stone);
            for (i, w) in [0.4f32, 0.3, 0.18].into_iter().enumerate() {
                put(out, Vec3::new(0.0, 0.625 + i as f32 * 0.045, 0.0), Vec3::new(w, 0.05, 0.11), stone);
            }
            for r in 0..3 {
                put(out, Vec3::new(0.0, 0.46 - r as f32 * 0.09, 0.056), Vec3::new(0.26 - r as f32 * 0.04, 0.022, 0.006), stone * 0.45);
            }
            if j > 0.55 {
                moss(out, Vec3::new(0.0, 0.73, 0.0), 0.2);
                put(out, Vec3::new(0.12, 0.2, 0.057), Vec3::new(0.1, 0.08, 0.004), jit(VERDIGRIS, j as f64, 0.1) * 1.3);
            }
        }
        2 => {
            // A cross on a stepped plinth; the better kept ringed.
            put(out, Vec3::new(0.0, 0.06, 0.0), Vec3::new(0.38, 0.12, 0.3), stone * 0.9);
            put(out, Vec3::new(0.0, 0.16, 0.0), Vec3::new(0.26, 0.08, 0.2), stone * 0.95);
            put(out, Vec3::new(0.0, 0.58, 0.0), Vec3::new(0.09, 0.84, 0.09), stone);
            put(out, Vec3::new(0.0, 0.74, 0.0), Vec3::new(0.44, 0.09, 0.09), stone);
            if j > 0.45 {
                for a in 0..10 {
                    let t = a as f32 / 10.0 * TAU;
                    put(out, Vec3::new(t.cos() * 0.14, 0.74 + t.sin() * 0.14, 0.0), Vec3::new(0.05, 0.05, 0.06), stone);
                }
            }
            moss(out, Vec3::new(0.0, 0.215, 0.0), 0.22);
        }
        3 => {
            // An obelisk on a plinth.
            put(out, Vec3::new(0.0, 0.1, 0.0), Vec3::new(0.4, 0.2, 0.4), stone * 0.9);
            for (i, (w, hgt)) in [(0.26f32, 0.42f32), (0.21, 0.4), (0.16, 0.34)].into_iter().enumerate() {
                put(out, Vec3::new(0.0, 0.2 + i as f32 * 0.39 + hgt / 2.0, 0.0), Vec3::new(w, hgt, w), stone);
            }
            put(out, Vec3::new(0.0, 1.4, 0.0), Vec3::new(0.1, 0.1, 0.1), stone);
            moss(out, Vec3::new(0.0, 0.205, 0.0), 0.3);
        }
        4 => {
            // A chest tomb: a box on feet under a heavy lid, a panel on its side.
            let s = stone * 0.95;
            for (x, z) in [(-0.3f32, -0.5f32), (0.3, -0.5), (-0.3, 0.5), (0.3, 0.5)] {
                put(out, Vec3::new(x, 0.05, z + 0.3), Vec3::new(0.14, 0.1, 0.14), s * 0.85);
            }
            put(out, Vec3::new(0.0, 0.32, 0.3), Vec3::new(0.66, 0.44, 1.14), s);
            put(out, Vec3::new(0.0, 0.58, 0.3), Vec3::new(0.76, 0.08, 1.26), s * 1.08);
            put(out, Vec3::new(0.335, 0.32, 0.3), Vec3::new(0.01, 0.26, 0.7), s * 0.7);
            moss(out, Vec3::new(-0.1, 0.63, 0.1), 0.34);
        }
        _ => {
            // An old slab, sunk and tilted, its top broken off and lying before it.
            let tilt = Quat::from_rotation_x(-0.35 - j * 0.3) * lean;
            block_m(
                out,
                Mat4::from_scale_rotation_translation(Vec3::new(0.46, 0.4, 0.1), tilt, p + Vec3::new(0.0, 0.17, 0.0)),
                stone * 0.85,
                SHADE_STONE,
            );
            block_m(
                out,
                Mat4::from_scale_rotation_translation(
                    Vec3::new(0.2, 0.06, 0.13),
                    Quat::from_rotation_y(k * 4.0),
                    p + Vec3::new(0.18, 0.03, 0.3),
                ),
                stone * 0.8,
                SHADE_STONE,
            );
            moss(out, Vec3::new(0.0, 0.05, 0.08), 0.3);
        }
    }
}

/// A family's plot: a low kerb of pale stone with rusted railings, two headstones and a tomb in it.
fn family_plot(out: &mut Vec<Instance>, c: Vec3) {
    let (hw, hd) = (1.3f32, 1.0f32);
    for (x, z, w, d) in [(0.0, -hd, hw * 2.0, 0.14), (0.0, hd, hw * 2.0, 0.14), (-hw, 0.0, 0.14, hd * 2.0), (hw, 0.0, 0.14, hd * 2.0)] {
        block(out, c + Vec3::new(x, 0.07, z), Vec3::new(w, 0.14, d), 0.0, jit(PALE_STONE[1], 0.4, 0.04) * 0.7, SHADE_STONE);
    }
    // Railings on the kerb, gapped at the front for a little gate.
    let iron = lin(IRON).lerp(lin(RUST), 0.5);
    let mut post = |p: Vec3| {
        block(out, p + Vec3::Y * 0.4, Vec3::new(0.03, 0.55, 0.03), 0.0, iron, SHADE_IRON);
        block_m(
            out,
            Mat4::from_scale_rotation_translation(Vec3::new(0.05, 0.05, 0.03), Quat::from_rotation_z(PI / 4.0), p + Vec3::Y * 0.7),
            iron,
            SHADE_IRON,
        );
    };
    let n = 16;
    for i in 0..=n {
        let u = i as f32 / n as f32;
        post(c + Vec3::new(-hw + 2.0 * hw * u, 0.0, -hd));
        if (u - 0.5).abs() > 0.12 {
            post(c + Vec3::new(-hw + 2.0 * hw * u, 0.0, hd));
        }
    }
    for i in 1..12 {
        let u = i as f32 / 12.0;
        for x in [-hw, hw] {
            post(c + Vec3::new(x, 0.0, -hd + 2.0 * hd * u));
        }
    }
    for (x, z, w, d) in [(0.0, -hd, hw * 2.0, 0.035), (0.0, hd, hw * 2.0, 0.035), (-hw, 0.0, 0.035, hd * 2.0), (hw, 0.0, 0.035, hd * 2.0)] {
        block(out, c + Vec3::new(x, 0.62, z), Vec3::new(w, 0.03, d), 0.0, iron, SHADE_IRON);
    }
    grave(out, c + Vec3::new(-0.6, 0.0, -0.75), 0.1, 0.8);
    grave(out, c + Vec3::new(0.6, 0.0, -0.75), 0.4, 0.6);
    grave(out, c + Vec3::new(0.0, 0.0, -0.55), 0.7, 0.9);
}

/// The mausoleum among the graves, at `c` (facing the walk, +z): walls of dressed stone on a
/// stepped base, two columns either side of an arched iron door, a pediment, a roof of copper
/// gone green.
fn mausoleum(out: &mut Vec<Instance>, c: Vec3) {
    let at = |x: f32, y: f32, z: f32| c + Vec3::new(x, y, z);
    for (k, (w, d)) in [(2.6f32, 2.8f32), (2.4, 2.6)].into_iter().enumerate() {
        block(out, at(0.0, 0.08 + k as f32 * 0.14, 0.0), Vec3::new(w, 0.16, d), 0.0, jit(PALE_STONE[k], 0.3, 0.04) * 0.72, SHADE_STONE);
    }
    let base = 0.28;
    let (hw, hd, h) = (1.0f32, 1.1f32, 1.6f32);
    block(
        out,
        at(0.0, base + h / 2.0, -0.05),
        Vec3::new(hw * 2.0 - 0.1, h, hd * 2.0 - 0.1),
        0.0,
        jit(CHARCOAL[1], 0.3, 0.0) * 0.6,
        SHADE_STONE,
    );
    let door = [(0.0, 0.42, 0.0, 0.92, 0.4)];
    for (dir, centre, half, openings) in [
        ((0, 1), at(0.0, 0.0, hd - 0.55), hw, &door[..]),
        ((0, -1), at(0.0, 0.0, -hd + 0.45), hw, &[][..]),
        ((1, 0), at(hw - 0.5, 0.0, -0.05), hd, &[][..]),
        ((-1, 0), at(-hw + 0.5, 0.0, -0.05), hd, &[][..]),
    ] {
        masonry(
            out,
            &Wall {
                centre,
                dir,
                half,
                base: c.y + base,
                height: h,
                courses: 5,
                seed: (dir.0 as f64 * 3.0 + 41.0, dir.1 as f64 + 17.0),
                len: (0.3, 0.25),
                depth: (0.04, 0.05),
                face: 0.46,
                wet: false,
                ruined: false,
                foot: None,
                openings,
            },
        );
    }
    // The door, its arch and the columns either side.
    block(out, at(0.0, base + 0.42, hd - 0.12), Vec3::new(0.82, 0.86, 0.06), 0.0, lin(IRON) * 0.8, SHADE_IRON);
    for k in 0..3 {
        block(out, at(0.0, base + 0.15 + k as f32 * 0.28, hd - 0.08), Vec3::new(0.84, 0.04, 0.03), 0.0, lin(RUST) * 0.8, SHADE_IRON);
    }
    arch_cubes(
        out,
        &Arch {
            centre: at(0.0, base, hd - 0.06),
            across_x: true,
            r: 0.48,
            rise: 0.4,
            spring: 0.82,
            depth: 0.2,
            piers: true,
            lintel: false,
            seed: (5.0, 9.0),
        },
    );
    for side in [-1.0f32, 1.0] {
        pillar_cubes(out, at(side * 0.8, base, hd + 0.08), h + 0.05, 4, 0.5, (side as f64 + 9.0, 3.0));
    }
    // The pediment: an entablature, then a low gable of pale stone.
    let top = base + h;
    block(
        out,
        at(0.0, top + 0.08, 0.05),
        Vec3::new(hw * 2.0 + 0.2, 0.16, hd * 2.0 + 0.3),
        0.0,
        jit(PALE_STONE[0], 0.4, 0.04) * 0.75,
        SHADE_STONE,
    );
    for k in 0..5 {
        let w = hw * 2.0 + 0.1 - k as f32 * 0.42;
        block(
            out,
            at(0.0, top + 0.22 + k as f32 * 0.1, hd + 0.1),
            Vec3::new(w, 0.1, 0.1),
            0.0,
            jit(PALE_STONE[k % 2], 0.5, 0.04) * 0.72,
            SHADE_STONE,
        );
    }
    // The roof: copper sheets gone to verdigris, either side of a ridge.
    for side in [-1.0f32, 1.0] {
        for k in 0..6 {
            let z = -hd + 0.2 + k as f32 * 0.4;
            let hk = hash(k as f64, side as f64 + 2.0) as f32;
            let m = Mat4::from_scale_rotation_translation(
                Vec3::new(1.25, 0.06, 0.38),
                Quat::from_rotation_z(side * -0.42),
                at(side * 0.56, top + 0.42, z),
            );
            block_m(out, m, lin(VERDIGRIS).lerp(lin(COPPER), hk * 0.4) * (0.85 + hk * 0.2), SHADE_BRONZE);
        }
    }
    block(out, at(0.0, top + 0.68, 0.0), Vec3::new(0.1, 0.08, hd * 2.2), 0.0, lin(VERDIGRIS) * 0.8, SHADE_BRONZE);
    moss_cap(out, at(-0.4, top + 0.17, hd), 0.5, (3.0, 4.0));
}

/// The chapel beyond the graves, at `c`: walls of dressed stone, a pitched roof of slates, a bell
/// tower with a spire, pointed windows lit warm (`CHAPEL_WINDOWS`), a door under an arch.
fn chapel(out: &mut Vec<Instance>, c: Vec3) {
    let at = |x: f32, y: f32, z: f32| c + Vec3::new(x, y, z);
    let (hw, hd, h) = (2.7f32, 1.9f32, 3.0f32);
    block(out, at(0.0, h / 2.0, 0.0), Vec3::new(hw * 2.0 - 0.2, h, hd * 2.0 - 0.2), 0.0, jit(CHARCOAL[1], 0.3, 0.0) * 0.55, SHADE_STONE);
    let windows = [(-1.6, 0.26, 1.35, 2.4, 0.32), (0.0, 0.42, 0.0, 1.35, 0.42), (1.6, 0.26, 1.35, 2.4, 0.32)];
    for (dir, centre, half, openings) in [
        ((0, 1), at(0.0, 0.0, hd - 0.56), hw, &windows[..]),
        ((0, -1), at(0.0, 0.0, -hd + 0.56), hw, &[][..]),
        ((1, 0), at(hw - 0.56, 0.0, 0.0), hd, &[][..]),
        ((-1, 0), at(-hw + 0.56, 0.0, 0.0), hd, &[][..]),
    ] {
        masonry(
            out,
            &Wall {
                centre,
                dir,
                half,
                base: c.y,
                height: h,
                courses: 8,
                seed: (dir.0 as f64 * 5.0 + 61.0, dir.1 as f64 * 3.0 + 7.0),
                len: (0.4, 0.35),
                depth: (0.05, 0.07),
                face: 0.46,
                wet: false,
                ruined: false,
                foot: None,
                openings,
            },
        );
    }
    // The windows: deep reveals, the glass lit from within, a pointed head and a sill.
    for &w in &CHAPEL_WINDOWS {
        let p = GRAVEYARD + w;
        glowing_block(out, p + Vec3::new(0.0, 0.0, -0.08), Vec3::new(0.46, 1.0, 0.04), lin(0xffc070), lin(0xffa050) * 1.3);
        for (k, s) in [(0, 0.4f32), (1, 0.26), (2, 0.12)] {
            glowing_block(
                out,
                p + Vec3::new(0.0, 0.52 + k as f32 * 0.07, -0.08),
                Vec3::new(s, 0.07, 0.04),
                lin(0xffc070),
                lin(0xffa050) * 1.3,
            );
        }
        block(out, p + Vec3::new(0.0, -0.55, 0.06), Vec3::new(0.62, 0.08, 0.2), 0.0, jit(PALE_STONE[0], 0.5, 0.04) * 0.75, SHADE_STONE);
        // Leading across the glass.
        for y in [-0.25f32, 0.1, 0.4] {
            block(out, p + Vec3::new(0.0, y, -0.055), Vec3::new(0.46, 0.02, 0.01), 0.0, lin(IRON), SHADE_IRON);
        }
        block(out, p + Vec3::new(0.0, 0.1, -0.055), Vec3::new(0.02, 1.0, 0.01), 0.0, lin(IRON), SHADE_IRON);
    }
    // The door, under a round arch, with steps up to it.
    block(out, at(0.0, 0.62, hd - 0.1), Vec3::new(0.8, 1.24, 0.06), 0.0, lin(0x3a2a20), SHADE_WOOD);
    arch_cubes(
        out,
        &Arch {
            centre: at(0.0, 0.0, hd - 0.04),
            across_x: true,
            r: 0.48,
            rise: 0.48,
            spring: 1.25,
            depth: 0.22,
            piers: true,
            lintel: false,
            seed: (2.0, 2.0),
        },
    );
    block(out, at(0.0, 0.05, hd + 0.3), Vec3::new(1.4, 0.1, 0.6), 0.0, jit(PALE_STONE[1], 0.2, 0.04) * 0.7, SHADE_STONE);
    // The roof: slates in rows either side of the ridge, a few slipped.
    for side in [-1.0f32, 1.0] {
        for row in 0..6 {
            let t = (row as f32 + 0.5) / 6.0;
            let y = h + 0.05 + (1.0 - t) * 1.5;
            let z = side * (0.12 + t * (hd + 0.25));
            for k in 0..10 {
                let hk = hash(row as f64 * 3.0 + side as f64, k as f64) as f32;
                let x = -hw - 0.15 + (k as f32 + 0.5) * (hw * 2.0 + 0.3) / 10.0;
                let m = Mat4::from_scale_rotation_translation(
                    Vec3::new((hw * 2.0 + 0.3) / 10.0 - 0.02, 0.06, (hd + 0.4) / 6.0 + 0.06),
                    Quat::from_rotation_x(side * 0.68 + (hk - 0.5) * 0.04),
                    at(x, y + (hk - 0.5) * 0.02, z),
                );
                block_m(out, m, jit(CHARCOAL[k % 3], hk as f64, 0.12) * 0.55, SHADE_STONE);
            }
        }
    }
    block(out, at(0.0, h + 1.62, 0.0), Vec3::new(hw * 2.0 + 0.3, 0.1, 0.16), 0.0, jit(CHARCOAL[0], 0.3, 0.05) * 0.5, SHADE_STONE);
    // Gables of stone under the roof's ends.
    for side in [-1.0f32, 1.0] {
        for k in 0..5 {
            let w = (hd * 2.0 - 0.2) * (1.0 - k as f32 / 5.0);
            block(
                out,
                at(side * (hw - 0.3), h + 0.15 + k as f32 * 0.3, 0.0),
                Vec3::new(0.3, 0.3, w),
                0.0,
                jit(CHARCOAL[k % 3], 0.4, 0.1) * 0.9,
                SHADE_STONE,
            );
        }
    }
    // The bell tower at the east end, a belfry open on its sides, a spire of slates and a cross.
    let t = at(hw + 0.7, 0.0, 0.3);
    block(out, t + Vec3::Y * 2.4, Vec3::new(1.3, 4.8, 1.3), 0.0, jit(CHARCOAL[1], 0.3, 0.0) * 0.6, SHADE_STONE);
    for (dir, off) in [
        ((0, 1), Vec3::new(0.0, 0.0, -0.04)),
        ((1, 0), Vec3::new(-0.04, 0.0, 0.0)),
        ((-1, 0), Vec3::new(0.04, 0.0, 0.0)),
        ((0, -1), Vec3::new(0.0, 0.0, 0.04)),
    ] {
        let belfry = [(0.0, 0.22, 3.9, 4.55, 0.22)];
        masonry(
            out,
            &Wall {
                centre: t + off,
                dir,
                half: 0.66,
                base: c.y,
                height: 5.2,
                courses: 13,
                seed: (dir.0 as f64 * 7.0 + 3.0, dir.1 as f64 * 11.0 + 5.0),
                len: (0.3, 0.25),
                depth: (0.04, 0.05),
                face: 0.64,
                wet: false,
                ruined: false,
                foot: None,
                openings: &belfry,
            },
        );
    }
    block(out, t + Vec3::Y * 4.45, Vec3::new(0.5, 0.7, 0.5), 0.0, lin(0x0c0a0a), SHADE_LIT);
    block(out, t + Vec3::Y * 5.26, Vec3::new(1.5, 0.12, 1.5), 0.0, jit(PALE_STONE[0], 0.4, 0.04) * 0.7, SHADE_STONE);
    for k in 0..7 {
        let w = 1.3 - k as f32 * 0.18;
        block(
            out,
            t + Vec3::Y * (5.45 + k as f32 * 0.32),
            Vec3::new(w, 0.32, w),
            PI / 4.0,
            jit(CHARCOAL[k % 3], 0.3, 0.08) * 0.45,
            SHADE_STONE,
        );
    }
    let iron = lin(IRON);
    block(out, t + Vec3::Y * 7.9, Vec3::new(0.05, 0.5, 0.05), 0.0, iron, SHADE_IRON);
    block(out, t + Vec3::Y * 7.98, Vec3::new(0.26, 0.05, 0.05), 0.0, iron, SHADE_IRON);
}

/// A leafless tree: a crooked trunk splitting into branches and twigs, its bark dark umber.
pub(super) fn dead_tree(out: &mut Vec<Instance>, base: Vec3, height: f32, seed: f64) {
    #[allow(clippy::too_many_arguments)]
    fn branch(out: &mut Vec<Instance>, from: Vec3, dir: Vec3, len: f32, thick: f32, depth: u32, seed: f64) {
        let segs = if depth >= 3 { 4 } else { 3 };
        let mut p = from;
        let mut d = dir.normalize();
        for i in 0..segs {
            let k = hash(seed + i as f64 * 1.7, depth as f64) as f32;
            d = (d + Vec3::new((k - 0.5) * 0.5, 0.06, (hash(seed, i as f64 + 9.0) as f32 - 0.5) * 0.5)).normalize();
            let l = len / segs as f32;
            let q = p + d * l;
            let w = thick * (1.0 - i as f32 / segs as f32 * 0.4);
            let rot = Quat::from_rotation_arc(Vec3::Y, d);
            let c = jit(BARK, k as f64, 0.15) * if depth == 0 { 0.8 } else { 1.0 };
            block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(w, l + w * 0.5, w), rot, (p + q) * 0.5), c, SHADE_WOOD);
            // Bark: a ridge now and then on the thick parts, and lichen on the old.
            if thick > 0.12 && k > 0.5 {
                let side = rot * Vec3::new(w * 0.5, 0.0, 0.0);
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(Vec3::new(w * 0.25, l * 0.7, w * 0.6), rot, (p + q) * 0.5 + side),
                    c * 0.8,
                    SHADE_WOOD,
                );
                if k > 0.8 {
                    block_m(
                        out,
                        Mat4::from_scale_rotation_translation(Vec3::new(w * 0.4, l * 0.3, w * 1.05), rot, (p + q) * 0.5),
                        jit(VERDIGRIS, k as f64, 0.1) * 1.1,
                        SHADE_SOFT,
                    );
                }
            }
            p = q;
        }
        if depth == 0 || thick < 0.018 {
            return;
        }
        let n = if depth >= 2 { 3 } else { 2 };
        for b in 0..n {
            let k = hash(seed * 1.3 + b as f64, depth as f64 * 3.1) as f32;
            let a = k * TAU + b as f32 * 2.1;
            let nd = (d * 0.9 + Vec3::new(a.cos(), 0.35 + k * 0.4, a.sin()) * 0.9).normalize();
            branch(out, p, nd, len * (0.55 + k * 0.2), thick * 0.55, depth - 1, seed * 1.7 + b as f64 * 11.0);
        }
    }
    let lean = Vec3::new((hash(seed, 1.0) as f32 - 0.5) * 0.35, 1.0, (hash(seed, 2.0) as f32 - 0.5) * 0.35);
    branch(out, base, lean, height * 0.5, 0.24, 4, seed);
    // Roots gripping the ground.
    for r in 0..5 {
        let a = r as f32 / 5.0 * TAU + seed as f32;
        let d = Vec3::new(a.cos(), -0.3, a.sin());
        let m = Mat4::from_scale_rotation_translation(
            Vec3::new(0.1, 0.45, 0.1),
            Quat::from_rotation_arc(Vec3::Y, d),
            base + d * 0.2 + Vec3::Y * 0.06,
        );
        block_m(out, m, jit(BARK, r as f64, 0.1), SHADE_WOOD);
    }
}

/// A wayside cross of weathered stone on a stepped base, at `p`.
fn wayside_cross(out: &mut Vec<Instance>, p: Vec3) {
    let stone = jit(PALE_STONE[0], 0.6, 0.05) * 0.7;
    for (k, w) in [0.9f32, 0.6].into_iter().enumerate() {
        block(out, p + Vec3::Y * (0.08 + k as f32 * 0.16), Vec3::new(w, 0.16, w), 0.1, stone * (0.9 + 0.05 * k as f32), SHADE_STONE);
    }
    let lean = Quat::from_rotation_z(0.06) * Quat::from_rotation_y(0.1);
    block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(0.14, 1.5, 0.14), lean, p + Vec3::Y * 1.05), stone, SHADE_STONE);
    block_m(
        out,
        Mat4::from_scale_rotation_translation(Vec3::new(0.6, 0.13, 0.14), lean, p + lean * Vec3::Y * 1.25 + Vec3::Y * 0.02),
        stone,
        SHADE_STONE,
    );
    for a in 0..12 {
        let t = a as f32 / 12.0 * TAU;
        let q = p + Vec3::Y * 0.02 + lean * Vec3::new(t.cos() * 0.2, 1.25 + t.sin() * 0.2, 0.0);
        block_m(out, Mat4::from_scale_rotation_translation(Vec3::new(0.07, 0.07, 0.09), lean, q), stone, SHADE_STONE);
    }
    block(out, p + Vec3::new(0.0, 0.33, 0.0), Vec3::new(0.4, 0.03, 0.3), 0.3, lin(MOSS[0]), SHADE_SOFT);
}

/// A dry-stone wall bounding the field outside, across the lane's far end and gapped for it.
fn field_wall(out: &mut Vec<Instance>) {
    let z0 = 7.4;
    let mut x = -17.0f32;
    let mut i = 0;
    while x < 17.0 {
        let h = hash(i as f64 * 1.9, 3.0) as f32;
        let lane = (x - 0.0).abs() < 1.6;
        let w = 0.3 + h * 0.3;
        if !lane {
            for row in 0..3 {
                let hk = hash(i as f64 + row as f64 * 7.0, 11.0) as f32;
                let s = Vec3::new(w * (0.8 + hk * 0.3), 0.18 + hk * 0.08, 0.36 + hk * 0.1);
                let z = z0 + (x * 0.15).sin() * 0.4;
                let y = ground(x, z) + 0.1 + row as f32 * 0.2;
                let c = jit(if hk > 0.6 { UMBER[row % 2] } else { CHARCOAL[(i + row) % 3] }, hk as f64, 0.12) * 1.1;
                block_m(
                    out,
                    Mat4::from_scale_rotation_translation(
                        s,
                        Quat::from_rotation_y((hk - 0.5) * 0.4) * Quat::from_rotation_z((h - 0.5) * 0.2),
                        GRAVEYARD + Vec3::new(x + (row % 2) as f32 * 0.12, y, z),
                    ),
                    c,
                    SHADE_STONE,
                );
            }
            if h > 0.55 {
                let z = z0 + (x * 0.15).sin() * 0.4;
                block(
                    out,
                    GRAVEYARD + Vec3::new(x, ground(x, z) + 0.66, z),
                    Vec3::new(w * 0.8, 0.04, 0.3),
                    h,
                    jit(MOSS[i % 3], h as f64, 0.1),
                    SHADE_SOFT,
                );
            }
        }
        x += w;
        i += 1;
    }
}

/// The lantern on its post by the lane (its light is the story's): an iron post on a stone foot,
/// an arm, and the lantern of copper and amber glass hung from it.
fn lantern_post(out: &mut Vec<Instance>, p: Vec3) {
    block(out, p + Vec3::Y * 0.1, Vec3::new(0.26, 0.2, 0.26), 0.3, jit(PALE_STONE[1], 0.3, 0.04) * 0.7, SHADE_STONE);
    let iron = lin(IRON).lerp(lin(RUST), 0.3);
    block(out, p + Vec3::Y * 1.1, Vec3::new(0.07, 1.9, 0.07), 0.0, iron, SHADE_IRON);
    block(out, p + Vec3::Y * 0.3, Vec3::new(0.11, 0.1, 0.11), 0.0, iron, SHADE_IRON);
    block(out, p + Vec3::new(-0.16, 2.05, 0.0), Vec3::new(0.38, 0.04, 0.04), 0.0, iron, SHADE_IRON);
    block_m(
        out,
        Mat4::from_scale_rotation_translation(Vec3::new(0.03, 0.26, 0.03), Quat::from_rotation_z(-0.7), p + Vec3::new(-0.1, 1.92, 0.0)),
        iron,
        SHADE_IRON,
    );
    let hook = p + Vec3::new(-0.3, 2.03, 0.0);
    lantern_cubes(out, hook, 0.06);
}

/// Low mist among the graves: thick there, thinning out past the wall, in the board's mist teal.
pub(super) fn graveyard_mist() -> Vec<Instance> {
    mist_sheets(
        GRAVEYARD,
        (-14, 14),
        (-18, 10),
        &[(0.05, 0.42), (0.16, 0.34), (0.32, 0.26), (0.55, 0.17), (0.9, 0.09)],
        lin(0x9aaaa4),
        // Thick among the graves, thin outside the wall, thinning toward the edges.
        |x, z| {
            let edge = (1.0 - (x.abs() as f32 - 9.0).max(0.0) / 5.0) * (1.0 - (-z as f32 - 13.0).max(0.0) / 5.0);
            edge.max(0.0) * if z <= -2 { 0.8 } else { (0.8 - (z as f32 + 2.0) / 8.0).max(0.12) }
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sets_stay_within_their_cube_budget() {
        // Each set is drawn whole every frame (and into the shadow maps): keep it in bounds.
        let (graveyard, gateway) = (graveyard_set().len(), gateway_set().len());
        eprintln!("story sets: graveyard {graveyard} cubes, gateway {gateway}");
        assert!(graveyard < 30_000 && gateway < 12_000, "graveyard {graveyard}, gateway {gateway}");
    }
}
