//! Voxel models authored as text, and the cutting of characters into rigged parts.
//!
//! `layers` run from the bottom of the model to the top; each layer is a list
//! of rows from back (-z) to front (+z); each row is a string of palette keys
//! along x. '.' is empty. Models face +z.

use glam::Vec3;

pub struct VoxelModel {
    pub palette: &'static [(char, u32)],
    pub layers: &'static [&'static [&'static str]],
}

#[derive(Clone, Copy, Debug)]
pub struct Voxel {
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub c: u32,
}

pub struct Collected {
    pub voxels: Vec<Voxel>,
    pub width: i32,
    pub depth: i32,
    pub height: i32,
}

pub fn collect(model: &VoxelModel) -> Collected {
    let mut voxels = Vec::new();
    let mut width = 0;
    let mut depth = 0;
    for (y, rows) in model.layers.iter().enumerate() {
        depth = depth.max(rows.len() as i32);
        for (z, row) in rows.iter().enumerate() {
            width = width.max(row.chars().count() as i32);
            for (x, key) in row.chars().enumerate() {
                if key == '.' || key == ' ' {
                    continue;
                }
                let c = model
                    .palette
                    .iter()
                    .find(|(k, _)| *k == key)
                    .map(|(_, c)| *c)
                    .unwrap_or_else(|| panic!("voxel palette missing key {key:?}"));
                voxels.push(Voxel { x: x as i32, y: y as i32, z: z as i32, c });
            }
        }
    }
    Collected { voxels, width, depth, height: model.layers.len() as i32 }
}

/// sRGB hex to linear RGB, as Three.js stores colors.
pub fn lin(hex: u32) -> Vec3 {
    let f = |c: u32| {
        let c = c as f32 / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    Vec3::new(f((hex >> 16) & 255), f((hex >> 8) & 255), f(hex & 255))
}

/// A model's cubes centered on x/z with its base at y = 0, in world units.
pub struct Mesh {
    pub size: f32,
    pub cubes: Vec<(Vec3, Vec3)>,
}

pub fn build_mesh(model: &VoxelModel, size: f32) -> Mesh {
    let c = collect(model);
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    let cubes = c
        .voxels
        .iter()
        .map(|v| {
            (
                Vec3::new((v.x as f32 - ox) * size, (v.y as f32 + 0.5) * size, (v.z as f32 - oz) * size),
                lin(v.c),
            )
        })
        .collect();
    Mesh { size, cubes }
}

/// The same mesh with one palette key recolored (potion liquids).
pub fn build_mesh_recolored(model: &VoxelModel, size: f32, key_color: u32, to: u32) -> Mesh {
    let mut m = build_mesh(model, size);
    let from = lin(key_color);
    let to = lin(to);
    for (_, c) in m.cubes.iter_mut() {
        if (*c - from).length() < 1e-4 {
            *c = to;
        }
    }
    m
}

// ---------------------------------------------------------------- sprites

/// Front view of a model as a 2D sprite: for each column and height, the
/// frontmost voxel's color. Rows run from the top of the model down.
pub struct Sprite {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<Vec<Option<u32>>>,
}

pub fn sprite_from_model(model: &VoxelModel) -> Sprite {
    let c = collect(model);
    let mut pixels = Vec::new();
    for y in (0..c.height).rev() {
        let mut row = Vec::new();
        for x in 0..c.width {
            let best = c.voxels.iter().filter(|v| v.x == x && v.y == y).max_by_key(|v| v.z);
            row.push(best.map(|v| v.c));
        }
        pixels.push(row);
    }
    Sprite { width: c.width, height: c.height, pixels }
}

// ------------------------------------------------------------------- rigs

/// Inclusive voxel box.
pub type VoxelBox = ([i32; 3], [i32; 3]);

/// A named, independently rotating part of a model, cut from its voxels by boxes.
pub struct PartDef {
    pub name: &'static str,
    pub boxes: &'static [VoxelBox],
    /// Rotation pivot in voxel units of the model grid.
    pub pivot: [f32; 3],
}

pub struct RigPart {
    pub name: &'static str,
    /// Rest position of the joint, relative to the model root.
    pub rest: Vec3,
    /// Cubes relative to the joint.
    pub cubes: Vec<(Vec3, Vec3)>,
    /// Lowest cube face below the joint (negative when the part hangs below it).
    pub min_y: f32,
}

pub struct Rig {
    pub size: f32,
    /// 'body' first, then the named parts in definition order.
    pub parts: Vec<RigPart>,
}

impl Rig {
    pub fn index(&self, name: &str) -> Option<usize> {
        self.parts.iter().position(|p| p.name == name)
    }
}

fn in_box(v: &Voxel, b: &VoxelBox) -> bool {
    v.x >= b.0[0] && v.x <= b.1[0] && v.y >= b.0[1] && v.y <= b.1[1] && v.z >= b.0[2] && v.z <= b.1[2]
}

/// Splits a model into pivoted parts; rotating a part swings it around its joint.
pub fn build_rig(model: &VoxelModel, defs: &[PartDef], size: f32) -> Rig {
    let c = collect(model);
    let ox = (c.width - 1) as f32 / 2.0;
    let oz = (c.depth - 1) as f32 / 2.0;
    let mut buckets: Vec<Vec<Voxel>> = vec![Vec::new(); defs.len()];
    let mut body = Vec::new();
    for v in &c.voxels {
        match defs.iter().position(|d| d.boxes.iter().any(|b| in_box(v, b))) {
            Some(i) => buckets[i].push(*v),
            None => body.push(*v),
        }
    }
    let make = |name: &'static str, list: &[Voxel], pivot: [f32; 3]| {
        let rest = Vec3::new((pivot[0] - ox) * size, pivot[1] * size, (pivot[2] - oz) * size);
        let cubes: Vec<(Vec3, Vec3)> = list
            .iter()
            .map(|v| {
                (
                    Vec3::new(
                        (v.x as f32 - pivot[0]) * size,
                        (v.y as f32 + 0.5 - pivot[1]) * size,
                        (v.z as f32 - pivot[2]) * size,
                    ),
                    lin(v.c),
                )
            })
            .collect();
        let min_y = cubes.iter().map(|(p, _)| p.y - size / 2.0).fold(f32::INFINITY, f32::min);
        RigPart { name, rest, cubes, min_y: if min_y.is_finite() { min_y } else { 0.0 } }
    };
    let mut parts = vec![make("body", &body, [ox, 0.0, oz])];
    for (i, d) in defs.iter().enumerate() {
        parts.push(make(d.name, &buckets[i], d.pivot));
    }
    Rig { size, parts }
}

// ------------------------------------------------------------------ models

mod pal {
    pub const RED: u32 = 0xd8262e;
    pub const DARK_RED: u32 = 0x8f1a1f;
    pub const BLUE: u32 = 0x2b4fd6;
    pub const SKIN: u32 = 0xf2c9a0;
    pub const WHITE: u32 = 0xf4f4f8;
    pub const GOLD: u32 = 0xf2c53d;
    pub const CYAN: u32 = 0x8fe7ff;
    pub const STEEL: u32 = 0xb8c0cc;
    pub const DARK: u32 = 0x1a1a24;
    pub const BROWN: u32 = 0x6b3f1d;
    pub const PURPLE: u32 = 0x7a3fe0;
    pub const DARK_PURPLE: u32 = 0x3d1f78;
    pub const BONE: u32 = 0xe0d3ba;
    pub const BONE_SHADE: u32 = 0xb3a288;
    pub const CAVITY: u32 = 0x241d17;
    pub const EYE_RED: u32 = 0xff2a2a;
    pub const GREEN: u32 = 0x3fe05a;
    pub const LIGHT_GREEN: u32 = 0x9dffab;
    pub const DARK_GREEN: u32 = 0x1c7a2c;
    pub const ORANGE: u32 = 0xff8a2a;
    pub const YELLOW: u32 = 0xffe066;
}
use pal::*;

/// Popolon: 9 wide, 5 deep, 14 tall. Sword raised on his right (+x), shield on his left.
pub const POPOLON: VoxelModel = VoxelModel {
    palette: &[
        ('R', RED), ('r', DARK_RED), ('B', BLUE), ('S', SKIN), ('W', WHITE), ('Y', GOLD),
        ('C', CYAN), ('G', STEEL), ('K', DARK), ('D', BROWN),
    ],
    layers: &[
        // 0-1 boots
        &[".........", "..DD.DD..", "..DD.DD..", "..DD.DD..", "........."],
        &[".........", "..DD.DD..", "..DD.DD..", "..DD.DD..", "........."],
        // 2-4 legs
        &[".........", "..BB.BB..", "..BB.BB..", "..BB.BB..", "........."],
        &[".........", "..BB.BB..", "..BB.BB..", "..BB.BB..", "........."],
        &[".........", "..BBBBB..", "..BBBBB..", "..BBBBB..", "........."],
        // 5-8 torso, arms, shield (x=0), sword hilt (x=8)
        &[".........", ".SRRRRRS.", "GSRRRRRSY", ".SRRRRRS.", "G........"],
        &[".........", ".RRRRRRR.", "GRRRRRRRC", ".RRRRRRR.", "G........"],
        &[".........", ".RRrRrRR.", "GRRRRRRRC", ".RRRRRRR.", "G........"],
        &[".........", ".RRRRRRR.", "GRRRRRRRC", ".RRRRRRR.", "G........"],
        // 9-11 head with eyes on the front row
        &[".........", "...SSS...", "...SSS..C", "...SSS...", "........."],
        &[".........", "...SSS...", "...SSS..C", "...KSK...", "........."],
        &[".........", "...SSS...", "...SSS..C", "...SSS...", "........."],
        // 12-13 helmet and horns, blade tip
        &[".........", "..RRRRR..", ".YRRRRRYC", "..RRRRR..", "........."],
        &[".........", "...RRR...", ".Y.RRR.YC", "...RRR...", "........."],
    ],
};

/// Popolon's joints: legs at the hip, arms at the shoulder, head at the neck.
pub const POPOLON_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[([2, 0, 0], [3, 3, 4])], pivot: [2.5, 4.0, 2.0] },
    PartDef { name: "rightLeg", boxes: &[([5, 0, 0], [6, 3, 4])], pivot: [5.5, 4.0, 2.0] },
    PartDef { name: "shieldArm", boxes: &[([0, 5, 0], [1, 8, 4])], pivot: [1.0, 8.5, 2.0] },
    PartDef { name: "swordArm", boxes: &[([7, 5, 0], [7, 8, 4]), ([8, 5, 0], [8, 13, 4])], pivot: [7.0, 8.5, 2.0] },
    PartDef { name: "head", boxes: &[([1, 9, 0], [7, 13, 4])], pivot: [4.0, 9.0, 2.0] },
];

/// Bat: 11 wide, 3 deep, 4 tall.
pub const BAT: VoxelModel = VoxelModel {
    palette: &[('P', PURPLE), ('p', DARK_PURPLE), ('K', DARK), ('Y', YELLOW)],
    layers: &[
        &["...........", "....pKp....", "..........."],
        &["P.........P", "PPPPpppPPPP", "....pYp...."],
        &["PP.......PP", ".PPPpppPPP.", "....ppp...."],
        &["...........", "..PP...PP..", "..........."],
    ],
};

/// Bat wings hinge where they meet the body.
pub const BAT_RIG: &[PartDef] = &[
    PartDef { name: "leftWing", boxes: &[([0, 0, 0], [3, 3, 2])], pivot: [4.0, 1.5, 1.0] },
    PartDef { name: "rightWing", boxes: &[([7, 0, 0], [10, 3, 2])], pivot: [6.0, 1.5, 1.0] },
];

/// Skeleton: 9 wide, 5 deep, 15 tall. Ribbed cage with a hollow chest, red eye sockets.
pub const SKELETON: VoxelModel = VoxelModel {
    palette: &[('W', BONE), ('w', BONE_SHADE), ('K', CAVITY), ('E', EYE_RED)],
    layers: &[
        // 0 feet
        &[".........", ".........", "...W.W...", "...W.W...", "........."],
        // 1-4 leg bones
        &[".........", ".........", "...W.W...", ".........", "........."],
        &[".........", ".........", "...W.W...", ".........", "........."],
        &[".........", ".........", "...W.W...", ".........", "........."],
        &[".........", ".........", "...W.W...", ".........", "........."],
        // 5 pelvis, hands at the sides
        &["....w....", "...WWW...", "W..WWW..W", "...WWW...", "....W...."],
        // 6-9 the cage: solid rib bands alternating with gaps that show the hollow
        &["...www...", "...WKW...", "W..WKW..W", "...WKW...", "...WWW..."],
        &["...www...", "..WKKKW..", "W.WKKKW.W", "..WKKKW..", "........."],
        &["...www...", "..WKKKW..", "W.WKKKW.W", "..WKKKW..", "...WWW..."],
        &["...www...", "..WKKKW..", "W.WKKKW.W", "..WKKKW..", "........."],
        // 10 shoulders and collarbone
        &["...www...", "..WWWWW..", "WWWWWWWWW", "..WWWWW..", "...WWW..."],
        // 11 neck
        &[".........", "....w....", "....W....", "....W....", "........."],
        // 12-14 jaw and skull, eyes on the front face
        &[".........", "...www...", "...WWW...", "...WWW...", "...WWW..."],
        &["...www...", "..WWWWW..", "..WWWWW..", "..WWWWW..", "..KEWEK.."],
        &[".........", "...www...", "...WWW...", "...WWW...", "........."],
    ],
};

/// Skeleton joints: bare arms from the collarbone ends, legs at the pelvis, skull at the neck.
pub const SKELETON_RIG: &[PartDef] = &[
    PartDef { name: "leftLeg", boxes: &[([3, 0, 0], [3, 4, 4])], pivot: [3.0, 5.0, 2.0] },
    PartDef { name: "rightLeg", boxes: &[([5, 0, 0], [5, 4, 4])], pivot: [5.0, 5.0, 2.0] },
    PartDef { name: "leftArm", boxes: &[([0, 5, 0], [0, 9, 4])], pivot: [0.0, 10.0, 2.0] },
    PartDef { name: "rightArm", boxes: &[([8, 5, 0], [8, 9, 4])], pivot: [8.0, 10.0, 2.0] },
    PartDef { name: "head", boxes: &[([2, 12, 0], [6, 14, 4])], pivot: [4.0, 11.5, 2.0] },
];

/// Slime: 7 wide, 7 deep, 5 tall.
pub const SLIME: VoxelModel = VoxelModel {
    palette: &[('G', GREEN), ('L', LIGHT_GREEN), ('g', DARK_GREEN), ('K', DARK)],
    layers: &[
        &[".ggggg.", "ggggggg", "ggggggg", "ggggggg", "ggggggg", "ggggggg", ".ggggg."],
        &[".GGGGG.", "GGGGGGG", "GGGGGGG", "GGGGGGG", "GGGGGGG", "GGGGGGG", ".GGGGG."],
        &["..GGG..", ".GGGGG.", "GGGGGGG", "GGGGGGG", "GGGGGGG", ".GKGKG.", "..GGG.."],
        &[".......", "..GGG..", ".GGGGG.", ".GGLGG.", ".GGGGG.", "..GGG..", "......."],
        &[".......", ".......", "..LLL..", "..LLL..", "..LLL..", ".......", "......."],
    ],
};

/// Gold nugget: a lumpy chunk, 4 wide, 3 deep, 3 tall, brighter on top.
pub const NUGGET: VoxelModel = VoxelModel {
    palette: &[('Y', GOLD), ('y', YELLOW), ('d', 0xb8902a)],
    layers: &[&[".dYd", "dYYY", ".dY."], &["..Yy", ".YyY", "..Y."], &["....", "..y.", "...."]],
};

/// Remedy: an upright green cross, 5 wide, 3 deep, 5 tall. Restores health.
pub const MEDKIT: VoxelModel = VoxelModel {
    palette: &[('G', GREEN), ('L', LIGHT_GREEN)],
    layers: &[
        &["..G..", "..G..", "..G.."],
        &["..G..", "..L..", "..G.."],
        &["GGGGG", "GLLLG", "GGGGG"],
        &["..G..", "..L..", "..G.."],
        &["..G..", "..G..", "..G.."],
    ],
};

/// TNT-style bomb: 5 wide, 5 deep, 6 tall with a fuse on top.
pub const BOMB: VoxelModel = VoxelModel {
    palette: &[('R', RED), ('r', DARK_RED), ('W', WHITE), ('K', DARK), ('G', STEEL)],
    layers: &[
        &["rRRRr", "RRRRR", "RRRRR", "RRRRR", "rRRRr"],
        &["RRRRR", "RRRRR", "RRRRR", "RRRRR", "RRRRR"],
        &["WWWWW", "WWKWW", "WKKKW", "WWKWW", "WWWWW"],
        &["RRRRR", "RRRRR", "RRRRR", "RRRRR", "RRRRR"],
        &["rRRRr", "RRRRR", "RRRRR", "RRRRR", "rRRRr"],
        &[".....", ".....", "..G..", ".....", "....."],
    ],
};

/// Arrow lying flat: 1 wide, 7 deep, 1 tall. Steel tip at the front (+z), feathers at the back.
pub const ARROW: VoxelModel = VoxelModel {
    palette: &[('D', BROWN), ('G', STEEL), ('W', WHITE)],
    layers: &[&["W", "W", "D", "D", "D", "D", "G"]],
};

/// Boomerang lying flat: an L of wood with pale tips.
pub const BOOMERANG: VoxelModel = VoxelModel {
    palette: &[('D', BROWN), ('W', WHITE)],
    layers: &[&["W....", "D....", "D....", "D....", "WDDDW"]],
};

/// Fireball: 3x3x3 with a bright core.
pub const FIREBALL: VoxelModel = VoxelModel {
    palette: &[('O', ORANGE), ('Y', YELLOW)],
    layers: &[&[".O.", "OOO", ".O."], &["OOO", "OYO", "OOO"], &[".O.", "OOO", ".O."]],
};

/// Crescent blade lying flat: a wide arc.
pub const CRESCENT: VoxelModel = VoxelModel {
    palette: &[('C', CYAN), ('W', WHITE)],
    layers: &[&["C.....C", "CC...CC", ".CCWCC."]],
};

/// Placeholder liquid color in the bottle, recolored per potion.
pub const BOTTLE_LIQUID: u32 = 0x010203;

/// Potion bottle: 3 wide, 3 deep, 6 tall.
pub const BOTTLE: VoxelModel = VoxelModel {
    palette: &[('L', BOTTLE_LIQUID), ('G', 0xcfe8f0), ('K', BROWN)],
    layers: &[
        &[".L.", "LLL", ".L."],
        &["LLL", "LLL", "LLL"],
        &["LLL", "LLL", "LLL"],
        &[".G.", "GLG", ".G."],
        &["...", ".G.", "..."],
        &["...", ".K.", "..."],
    ],
};

/// Shrine crystal: an 8-tall diamond, 5 wide and deep at the waist.
pub const DIAMOND: VoxelModel = VoxelModel {
    palette: &[('W', WHITE), ('L', CYAN)],
    layers: &[
        &[".....", ".....", "..W..", ".....", "....."],
        &[".....", "..L..", ".LWL.", "..L..", "....."],
        &["..L..", ".LLL.", "LLWLL", ".LLL.", "..L.."],
        &[".LLL.", "LLLLL", "LLWLL", "LLLLL", ".LLL."],
        &["..L..", ".LLL.", "LLWLL", ".LLL.", "..L.."],
        &[".....", "..L..", ".LWL.", "..L..", "....."],
        &[".....", ".....", "..W..", ".....", "....."],
    ],
};

/// Hanging lantern on a post: glowing cage on top.
pub const LANTERN: VoxelModel = VoxelModel {
    palette: &[('D', BROWN), ('G', 0xd6ff5a), ('K', DARK)],
    layers: &[&["D"], &["D"], &["D"], &["D"], &["G"], &["K"]],
};

/// Lean forest tree for dense woods: 3 wide, 3 deep, 6 tall, 22 voxels.
pub const TREE_LOW: VoxelModel = VoxelModel {
    palette: &[('T', 0x4a2f16), ('G', 0x2f6b2a), ('g', 0x24521f), ('L', 0x4c8f3c)],
    layers: &[
        &["...", ".T.", "..."],
        &["...", ".T.", "..."],
        &[".g.", "gGg", ".g."],
        &["gGg", "GGG", "gGg"],
        &[".G.", "GLG", ".G."],
        &["...", ".L.", "..."],
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn models_are_rectangular_and_colored() {
        for m in [&POPOLON, &BAT, &SKELETON, &SLIME, &NUGGET, &MEDKIT, &BOMB, &DIAMOND, &TREE_LOW, &BOTTLE] {
            let c = collect(m);
            assert!(!c.voxels.is_empty());
            for layer in m.layers {
                assert_eq!(layer.len() as i32, c.depth);
                for row in layer.iter() {
                    assert_eq!(row.len() as i32, c.width);
                }
            }
        }
    }

    #[test]
    fn rigs_claim_the_named_parts() {
        let rig = build_rig(&POPOLON, POPOLON_RIG, 0.075);
        assert_eq!(rig.parts.len(), 6);
        assert!(rig.parts.iter().all(|p| !p.cubes.is_empty()));
        let total: usize = rig.parts.iter().map(|p| p.cubes.len()).sum();
        assert_eq!(total, collect(&POPOLON).voxels.len());
    }

    #[test]
    fn portrait_is_the_front_view() {
        let s = sprite_from_model(&POPOLON);
        assert_eq!(s.height, 14);
        assert_eq!(s.width, 9);
    }
}
