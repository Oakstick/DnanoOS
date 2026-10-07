//! The courts' own cut into their bones (ANIMATION.md §11.9): the seven princes and what they raise,
//! court by court in the game's order. The princes on the heroes' bones where they stand as men do,
//! their weapons their `offhand` (in the fist at the sculpt's high x, their own left) cut out of
//! their arms by what they are made of; the beasts on skeletons of their own.

use super::foe_build::Build;
use super::skeleton::{Bone, Cut, Joints};
use crate::render::characters::mat;

const INF: f32 = f32::INFINITY;

fn mirror(p: &[[f32; 3]], cx: f32) -> Vec<[f32; 3]> {
    p.iter().map(|q| [2.0 * cx - q[0], q[1], q[2]]).collect()
}

/// The hips, spine and chest at their joints, the neck and the head; cut level at the spine's and
/// the chest's heights.
fn torso(b: &mut Build, hips: [f32; 3], spine: [f32; 3], chest: [f32; 3], neck: [f32; 3], head: [f32; 3]) {
    b.at(Bone::Hips, hips).at(Bone::Spine, spine).at(Bone::Chest, chest).at(Bone::Neck, neck).at(Bone::Head, head);
    b.cut(Cut::up(Bone::Hips.index(), Bone::Spine.index(), spine[1])).cut(Cut::up(Bone::Spine.index(), Bone::Chest.index(), chest[1]));
}

/// Both arms, cut at each bend; the empty weapon bones in the fists.
fn arms(b: &mut Build, arm_l: &[[f32; 3]], arm_r: &[[f32; 3]]) {
    b.chain(&[Bone::UpperArmL, Bone::ForearmL, Bone::HandL].map(Bone::index), arm_l, None);
    b.chain(&[Bone::UpperArmR, Bone::ForearmR, Bone::HandR].map(Bone::index), arm_r, None);
    b.at(Bone::Offhand, arm_l[2]).at(Bone::Blade, arm_r[2]).at(Bone::Shield, arm_r[1]);
}

/// Both legs (the `.L` one given, the other mirrored about `cx`), cut at each bend.
fn legs(b: &mut Build, cx: f32, leg_l: &[[f32; 3]; 4]) {
    b.chain(&[Bone::ThighL, Bone::ShinL, Bone::FootL].map(Bone::index), leg_l, None);
    b.chain(&[Bone::ThighR, Bone::ShinR, Bone::FootR].map(Bone::index), &mirror(leg_l, cx), None);
}

// ------------------------------------------------------------------ Greed

/**
 * Mammon (grid 100 × 124 × 80, `foes_court::MAMMON_SIZE`; x 50 and z 38 his middle): his umber robe,
 * his belly and his sack on his hips; his chest and the mantle of gold chain; his arms in wide
 * sleeves, the hide below sewn with coins; his left an open claw (the sculpt's low x), his right
 * fist round the sceptre. The sceptre (its gold shaft, its foot and its spiked disc) is his
 * `offhand`, cut from his arm by its gold below his fist, above his wrist and over his shoulder.
 */
pub fn mammon() -> &'static Joints {
    let (cx, cz) = (50.0, 38.0);
    let mut b = Build::humanoid();
    const GOLD: &[u32] = &[mat::GOLD];
    let (up, staff) = (Bone::UpperArmL.index(), Bone::Offhand.index());
    b.cuts_first(&[
        // (The shaft below his fist and its foot.)
        Cut::cube(up, staff, [cx + 20.0, -INF, -INF], [INF, 33.5, INF]).only(GOLD),
        // (The shaft above his wrist, clear of the coins on his forearm and the trim of his sleeve.)
        Cut::cube(up, staff, [cx + 32.5, 49.0, cz + 17.8], [cx + 40.5, INF, INF]).only(GOLD),
        // (The disc and its spikes, over his shoulder.)
        Cut::cube(up, staff, [cx + 20.0, 76.0, cz + 14.0], [INF; 3]).only(GOLD),
    ]);
    torso(&mut b, [cx, 34.0, cz], [cx, 62.0, cz + 2.0], [cx, 76.0, cz + 2.0], [cx, 88.0, cz + 4.0], [cx, 92.0, cz + 4.0]);
    let arm_l = [[cx + 30.0, 82.0, cz], [cx + 37.0, 62.0, cz + 6.0], [cx + 36.0, 46.0, cz + 16.0], [cx + 36.5, 34.0, cz + 20.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    b.at(Bone::Offhand, [cx + 36.5, 40.0, cz + 22.5]);
    legs(&mut b, cx, &[[cx + 11.0, 34.0, cz], [cx + 11.0, 21.0, cz + 1.0], [cx + 11.0, 8.0, cz + 2.0], [cx + 11.0, 1.0, cz + 18.0]]);
    b.finish("mammon", false)
}

/**
 * A gold golem (grid 60 × 84 × 48, `foes_court::GOLEM_SIZE`; x 30 and z 24 its middle): its hunched
 * heap of a body, its stumps of legs, its arms of stacked gold bars with fists of coin, its head sunk
 * between its shoulders.
 */
pub fn gold_golem() -> &'static Joints {
    let (cx, cz) = (30.0, 24.0);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 26.0, cz], [cx, 42.0, cz], [cx, 50.0, cz - 1.0], [cx, 63.0, cz + 2.0], [cx, 66.0, cz + 3.0]);
    let arm_l = [[cx + 18.0, 58.0, cz], [cx + 20.0, 45.0, cz + 2.0], [cx + 22.0, 33.0, cz + 4.5], [cx + 22.0, 21.0, cz + 6.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    legs(&mut b, cx, &[[cx + 8.0, 26.0, cz], [cx + 8.0, 14.0, cz + 1.0], [cx + 8.5, 6.0, cz + 2.0], [cx + 8.5, 1.0, cz + 10.0]]);
    b.finish("gold_golem", false)
}

// ------------------------------------------------------------------ Pride

/// Lucifer's middle (x, z) on his grid (`foes_court::pride`), his Reflections' too.
const LX: f32 = 50.0;
const LZ: f32 = 46.0;

/**
 * Lucifer (grid 100 × 124 × 80, `foes_court::LUCIFER_SIZE`): in white plate, his fauld and tabard on
 * his hips; the sword of light held point-down in his fist at the sculpt's high x (his own left), its
 * hilt, guard, gems and blade of light his `offhand`, cut out of his gauntlet below his wrist's cuff
 * (and its pommel above it); his six white wings either side each a bone on his chest (`wings.L`,
 * `wings.R`), and the burnt six that take their place when he falls (`burnt.L`, `burnt.R`). His
 * Reflection is his mirror image (`mirrored`): the sword in its fist at the sculpt's low x (its own
 * right), the `blade`.
 */
fn pride_body(name: &'static str, reflection: bool) -> &'static Joints {
    let s = if reflection { -1.0 } else { 1.0 };
    let x = |dx: f32| LX + s * dx;
    let mut b = Build::humanoid();
    const LIT: &[u32] = &[mat::GOLD, mat::GLOW];
    let (up, sword) = if reflection { (Bone::UpperArmR, Bone::Blade) } else { (Bone::UpperArmL, Bone::Offhand) };
    let (x0, x1) = lo_hi(reflection, LX + 19.0, INF);
    b.cuts_first(&[
        Cut::cube(up.index(), sword.index(), [x0, -INF, -INF], [x1, 49.4, INF]).only(LIT),
        Cut::cube(up.index(), sword.index(), [x0, 51.6, -INF], [x1, 58.0, LZ + 11.0]).only(LIT),
    ]);
    torso(&mut b, [LX, 52.0, LZ], [LX, 64.0, LZ], [LX, 74.0, LZ], [LX, 85.0, LZ + 1.0], [LX, 91.0, LZ + 1.0]);
    let arm = |sd: f32| {
        [[LX + sd * 23.0, 85.0, LZ], [LX + sd * 27.0, 65.0, LZ + 2.0], [LX + sd * 27.0, 50.0, LZ + 10.0], [LX + sd * 27.0, 38.0, LZ + 14.0]]
    };
    arms(&mut b, &arm(1.0), &arm(-1.0));
    b.at(sword, [x(27.0), 45.0, LZ + 12.0]);
    legs(&mut b, LX, &[[LX + 9.0, 52.0, LZ], [LX + 10.0, 28.0, LZ + 2.0], [LX + 10.0, 8.0, LZ + 1.0], [LX + 10.0, 1.0, LZ + 17.0]]);
    let chest = Bone::Chest.index();
    for (bone, part, side) in
        [("wings.L", "rightWings", 1.0), ("wings.R", "leftWings", -1.0), ("burnt.L", "rightBurnt", 1.0), ("burnt.R", "leftBurnt", -1.0)]
    {
        if reflection && part.ends_with("Burnt") {
            continue;
        }
        let k = b.bone(bone, chest, [LX + side * 7.0, 82.0, LZ - 12.0]);
        b.part(part, k);
    }
    b.finish(name, false)
}

/// The span `[a, b)` in x, or its mirror image about Lucifer's middle.
fn lo_hi(mirrored: bool, a: f32, b: f32) -> (f32, f32) {
    if mirrored {
        (2.0 * LX - b, 2.0 * LX - a)
    } else {
        (a, b)
    }
}

pub fn lucifer() -> &'static Joints {
    pride_body("lucifer", false)
}

pub fn reflection() -> &'static Joints {
    pride_body("reflection", true)
}

// ------------------------------------------------------------------ Envy

/**
 * Leviathan (grid 100 × 124 × 80, `foes_court::envy::LEVIATHAN_SIZE`; x 50 and z 40 his middle): his
 * coil on the floor and the front third rising off it his `body` (the root, at his middle on the
 * floor, as the code turns him); his neck from where it leaves the body (y 47) in three, cut square
 * to each bend of its S, to his long finned head; his jaw on its hinge; his tail in two where it
 * leaves the coil; his pectoral fins.
 */
pub fn leviathan() -> &'static Joints {
    let (cx, cz) = (50.0, 40.0);
    let at = |x: f32, y: f32, z: f32| [cx + x, y, cz + z];
    let mut b = Build::new("body", at(0.0, 0.0, 0.0));
    let n1 = b.bone("neck.1", 0, [0.0; 3]);
    let n2 = b.bone("neck.2", n1, [0.0; 3]);
    let n3 = b.bone("neck.3", n2, [0.0; 3]);
    let head = b.bone("head", n3, at(0.0, 92.0, 4.0));
    let jaw = b.bone("jaw", head, at(0.0, 92.8, 5.6));
    let t1 = b.bone("tail.1", 0, [0.0; 3]);
    let t2 = b.bone("tail.2", t1, [0.0; 3]);
    let fin_l = b.bone("fin.L", 0, at(11.0, 36.0, 2.0));
    let fin_r = b.bone("fin.R", 0, at(-11.0, 36.0, 2.0));
    b.part("neck", n1).part("head", head).part("jaw", jaw).part("tail", t1).part("rightFin", fin_l).part("leftFin", fin_r);
    b.chain(&[n1, n2, n3], &[at(0.0, 47.0, -4.5), at(0.0, 62.0, -8.0), at(0.0, 76.0, -6.5), at(0.0, 94.0, 2.0)], None);
    b.chain(&[t1, t2], &[[30.0, 8.5, cz + 14.0], [40.0, 11.0, cz + 30.0], [50.0, 22.0, cz + 32.0]], None);
    b.legs(&[]);
    b.finish("leviathan", false)
}

/**
 * A serpentling (grid 44 × 62 × 44, `foes_court::envy::SERPENTLING_SIZE`; x and z 22 its middle): a
 * sea krait reared off its coil. The coil the `coil` (the root, at its middle on the floor), its
 * neck rising off it in three to its small flat head, its paddle of a tail.
 */
pub fn serpentling() -> &'static Joints {
    let (cx, cz) = (22.0, 22.0);
    let at = |x: f32, y: f32, z: f32| [cx + x, y, cz + z];
    let mut b = Build::new("coil", at(0.0, 0.0, 0.0));
    let n1 = b.bone("neck.1", 0, [0.0; 3]);
    let n2 = b.bone("neck.2", n1, [0.0; 3]);
    let n3 = b.bone("neck.3", n2, [0.0; 3]);
    let head = b.bone("head", n3, at(0.0, 50.0, 3.0));
    let tail = b.bone("tail", 0, at(-10.0, 3.0, 8.0));
    b.part("neck", n1).part("head", head).part("tail", tail);
    b.chain(&[n1, n2, n3], &[at(0.0, 8.0, -1.5), at(0.0, 22.0, -4.0), at(0.0, 36.0, -2.5), at(0.0, 49.0, 2.0)], None);
    b.legs(&[]);
    b.finish("serpentling", false)
}

// ------------------------------------------------------------------ Wrath

/**
 * Amon (grid 100 × 124 × 80, `foes_court::wrath::AMON_SIZE`; x 50 and z 42 his middle): a wolf's legs
 * (the knee forward, the hock back) on paws; his kilt and his skull-hung belt on his hips; his chest
 * like a bull's under the wolf's pelt; his raven's head; his huge bare arms in iron bracers; his
 * serpent's tail coiling behind him on the floor, in two. His great axe in his fist at the sculpt's
 * high x (his own left), its haft and crescent blade his `offhand`: cut out of his arm by its wood
 * at the fist, by all that is not his hide below the fist and above the elbow's band, by its iron at
 * the spike behind the blade.
 */
pub fn amon() -> &'static Joints {
    let (cx, cz) = (50.0, 42.0);
    let mut b = Build::humanoid();
    let (up, axe) = (Bone::UpperArmL.index(), Bone::Offhand.index());
    const HIDE: &[u32] = &[mat::SKIN, mat::IVORY];
    b.cuts_first(&[
        Cut::cube(up, axe, [cx + 25.0, -INF, -INF], [INF, 34.0, INF]).except(HIDE),
        Cut::cube(up, axe, [cx + 25.0, 66.0, cz + 12.5], [INF; 3]).except(HIDE),
        Cut::cube(up, axe, [cx + 25.0, 34.0, cz + 14.5], [INF, 66.0, INF]).only(&[mat::WOOD]),
        Cut::cube(up, axe, [cx + 33.0, 78.0, cz + 2.0], [cx + 46.0, 95.0, cz + 12.5]).only(&[mat::METAL]),
    ]);
    torso(&mut b, [cx, 40.0, cz], [cx, 56.0, cz], [cx, 70.0, cz], [cx, 88.0, cz + 3.0], [cx, 92.0, cz + 4.0]);
    let arm_l = [[cx + 32.0, 84.0, cz], [cx + 39.0, 64.0, cz + 4.0], [cx + 38.0, 47.0, cz + 10.0], [cx + 38.5, 33.0, cz + 14.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    b.at(Bone::Offhand, [cx + 39.5, 41.0, cz + 17.5]);
    legs(&mut b, cx, &[[cx + 12.0, 40.0, cz], [cx + 13.5, 23.0, cz + 5.0], [cx + 13.0, 9.0, cz - 2.0], [cx + 13.0, 1.0, cz + 10.0]]);
    let t1 = b.bone("tail.1", Bone::Hips.index(), [0.0; 3]);
    let t2 = b.bone("tail.2", t1, [0.0; 3]);
    b.part("tail", t1);
    b.chain(&[t1, t2], &[[cx, 34.0, cz - 12.0], [cx + 9.0, 7.0, cz - 31.0], [cx + 35.0, 5.0, cz - 21.0]], None);
    b.finish("amon", false)
}

/**
 * A raven of Amon (grid 80 × 64 × 54, `foes_court::wrath::RAVEN_SIZE`; its body at x 40, y 44, z 26):
 * a great black bird hovering, wings spread, on the night raven's bones (`foe_beasts::FLYER`, so the
 * flyers' keyed clips drive it): its body, its tail feathers behind z 15, its head before z 35 and
 * the lower half of its beak its jaw, each wing hinged at its root and at the wrist, its talons
 * hanging.
 */
pub fn raven() -> &'static Joints {
    let (rx, ry, rz) = (40.0, 44.0, 26.0);
    let mut b = Build::new("body", [rx, ry, rz - 1.0]);
    let tail = b.bone("tail", 0, [rx, ry - 1.0, rz - 11.0]);
    let head = b.bone("head", 0, [rx, ry + 3.0, rz + 9.0]);
    let jaw = b.bone("jaw", head, [rx, ry + 2.6, rz + 17.0]);
    let wing_l = b.bone("wing.L", 0, [rx + 6.0, ry + 2.0, rz]);
    let hand_l = b.bone("hand.L", wing_l, [rx + 25.2, ry + 6.2, rz + 3.0]);
    let wing_r = b.bone("wing.R", 0, [rx - 6.0, ry + 2.0, rz]);
    let hand_r = b.bone("hand.R", wing_r, [rx - 25.2, ry + 6.2, rz + 3.0]);
    let foot_l = b.bone("foot.L", 0, [rx + 3.0, ry - 5.0, rz + 1.0]);
    let foot_r = b.bone("foot.R", 0, [rx - 3.0, ry - 5.0, rz + 1.0]);
    b.part("rightWing", wing_l).part("leftWing", wing_r);
    const FEET: &[u32] = &[mat::LEATHER, mat::IVORY];
    b.cut(Cut::cube(0, foot_l, [rx + 0.5, -INF, rz - 4.0], [INF, ry - 5.0, rz + 8.0]).only(FEET))
        .cut(Cut::cube(0, foot_r, [-INF, -INF, rz - 4.0], [rx - 0.5, ry - 5.0, rz + 8.0]).only(FEET))
        .cut(Cut::plane(0, tail, 2, rz - 11.0, true))
        .cut(Cut::plane(0, head, 2, rz + 9.0, false))
        .cut(Cut::cube(head, jaw, [rx - 2.5, -INF, rz + 16.5], [rx + 2.5, ry + 3.3, INF]).only(&[mat::LEATHER]))
        .cut(Cut::plane(wing_l, hand_l, 0, rx + 25.2, false))
        .cut(Cut::plane(wing_r, hand_r, 0, rx - 25.2, true));
    b.legs(&[]);
    b.finish("raven", false)
}

// ------------------------------------------------------------------ Sloth

/**
 * Belphegor (grid 100 × 124 × 80, `foes_court::sloth::BELPHEGOR_SIZE`; x 50 and z 34 his middle): his
 * night-robe over his belly on his hips, the beard's fall on his chest; his thick legs under the robe
 * on bare grey feet; his arms in wide striped sleeves to the elbow, grey forearms, great long-nailed
 * hands; his heavy head in its nightcap. His hourglass hangs from his fist at the sculpt's low x (his
 * own right): the `blade`, all of it below his fingers but their nails.
 */
pub fn belphegor() -> &'static Joints {
    let (cx, cz) = (50.0, 34.0);
    let mut b = Build::humanoid();
    b.cuts_first(&[
        Cut::cube(Bone::UpperArmR.index(), Bone::Blade.index(), [-INF; 3], [cx - 28.0, 39.6, INF]).except(&[mat::SKIN, mat::IVORY])
    ]);
    torso(&mut b, [cx, 30.0, cz], [cx, 54.0, cz + 2.0], [cx, 70.0, cz + 2.0], [cx, 86.0, cz + 5.0], [cx, 90.0, cz + 6.0]);
    let arm_l = [[cx + 31.0, 82.0, cz], [cx + 39.0, 62.0, cz + 6.0], [cx + 38.0, 46.0, cz + 14.0], [cx + 38.5, 32.0, cz + 18.0]];
    let arm_r = [[cx - 31.0, 82.0, cz], [cx - 39.0, 62.0, cz + 6.0], [cx - 40.0, 50.0, cz + 18.0], [cx - 40.5, 39.0, cz + 21.0]];
    arms(&mut b, &arm_l, &arm_r);
    b.at(Bone::Blade, [cx - 40.5, 39.0, cz + 21.0]);
    legs(&mut b, cx, &[[cx + 12.0, 30.0, cz], [cx + 12.0, 18.0, cz + 1.0], [cx + 12.5, 7.0, cz + 2.0], [cx + 13.0, 1.0, cz + 18.0]]);
    b.finish("belphegor", false)
}

/**
 * A nightmare (grid 52 × 76 × 44, `foes_court::sloth::NIGHTMARE_SIZE`; x 26 and z 20 its middle): a
 * hunched shroud, its two wisps trailing below it where legs would be (on the legs' bones, each in
 * three down its length), its thin bony arms out of tattered sleeves, the mare's skull in its hood.
 */
pub fn nightmare() -> &'static Joints {
    let (cx, cz) = (26.0, 20.0);
    let mut b = Build::humanoid();
    torso(&mut b, [cx, 34.0, cz], [cx, 42.0, cz], [cx, 48.0, cz], [cx, 56.0, cz + 1.0], [cx, 58.0, cz + 1.0]);
    let arm_l = [[cx + 11.0, 53.0, cz - 1.0], [cx + 15.0, 42.0, cz + 5.0], [cx + 15.5, 34.0, cz + 11.0], [cx + 16.0, 28.0, cz + 14.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    legs(&mut b, cx, &[[cx + 5.0, 32.0, cz - 1.0], [cx + 6.5, 24.0, cz - 3.0], [cx + 5.0, 14.0, cz - 6.0], [cx + 9.0, 1.0, cz - 11.0]]);
    b.finish("nightmare", false)
}

// ------------------------------------------------------------------ Gluttony

/**
 * Beelzebub (grid 100 × 124 × 80, `foes_court::gluttony::BEELZEBUB_SIZE`; x 50 and z 40 his middle):
 * his rotten crimson robe over his vast belly on his hips, his napkin and the ruff of bristles on his
 * chest; a fly's legs under the robe; his sleeves to the elbow and a fly's leg below, hooked claws
 * round a gnawed drumstick (his own right) and a butcher's cleaver (his own left): the cleaver his
 * `offhand`, cut out of his claws by its wood, gold and steel, all of its blade beyond them; his fly's
 * head; his two great glassy wings each a bone on his chest.
 */
pub fn beelzebub() -> &'static Joints {
    let (cx, cz) = (50.0, 40.0);
    let mut b = Build::humanoid();
    let (up, cleaver) = (Bone::UpperArmL.index(), Bone::Offhand.index());
    b.cuts_first(&[
        Cut::cube(up, cleaver, [cx + 30.0, -INF, cz + 21.0], [INF, 46.0, cz + 31.5]).only(&[mat::WOOD, mat::GOLD, mat::METAL]),
        Cut::cube(up, cleaver, [cx + 30.0, -INF, cz + 31.5], [INF, 46.0, INF]),
    ]);
    torso(&mut b, [cx, 34.0, cz], [cx, 58.0, cz + 2.0], [cx, 72.0, cz + 2.0], [cx, 86.0, cz + 4.0], [cx, 90.0, cz + 4.0]);
    let arm_l = [[cx + 31.0, 82.0, cz], [cx + 38.0, 62.0, cz + 7.0], [cx + 36.0, 47.0, cz + 18.0], [cx + 36.0, 38.0, cz + 24.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    b.at(Bone::Offhand, [cx + 36.0, 42.0, cz + 25.0]);
    legs(&mut b, cx, &[[cx + 12.0, 34.0, cz], [cx + 12.0, 21.0, cz + 1.5], [cx + 12.0, 9.0, cz + 3.0], [cx + 12.0, 1.0, cz + 20.0]]);
    let chest = Bone::Chest.index();
    for (bone, part, side) in [("wing.L", "rightWing", 1.0), ("wing.R", "leftWing", -1.0)] {
        let k = b.bone(bone, chest, [cx + side * 9.0, 88.0, cz - 14.0]);
        b.part(part, k);
    }
    b.finish("beelzebub", false)
}

/**
 * A fly swarm (grid 44 × 64 × 44, `foes_court::gluttony::FLY_SWARM_SIZE`): its heart of big flies
 * about a dark knot the root (at its middle, as the code turns its clouds about it), and its four
 * clouds of small flies each a bone on it, at the heart.
 */
pub fn fly_swarm() -> &'static Joints {
    let at = [22.0, 36.0, 22.0];
    let mut b = Build::new("heart", at);
    for (bone, part) in [("cloud.1", "cloud1"), ("cloud.2", "cloud2"), ("cloud.3", "cloud3"), ("cloud.4", "cloud4")] {
        let k = b.bone(bone, 0, at);
        b.part(part, k);
    }
    b.legs(&[]);
    b.finish("fly_swarm", false)
}

// ------------------------------------------------------------------ Lust

/**
 * Asmodeus (grid 100 × 124 × 80, `foes_court::lust::ASMODEUS_SIZE`; x 50 and z 34 his middle): his
 * coat of rose silk on his hips, his doublet and the fan of his collar on his chest; his legs in hose
 * on goose's feet; his three heads together on his neck; his serpent's tail in two, coiled on the floor
 * to his left. His left arm (his own right, the sculpt's low x) raised, the hand open at his lips; his
 * right low before him with the lance in his fist: the lance his `offhand`, its gold and steel beyond
 * his fist along it, its banded butt behind his fist, and its bands and pennon well out before him.
 */
pub fn asmodeus() -> &'static Joints {
    let (cx, cz) = (50.0, 34.0);
    let mut b = Build::humanoid();
    let (up, lance) = (Bone::UpperArmL.index(), Bone::Offhand.index());
    let hand = [cx + 24.76, 46.6, cz + 15.2];
    let a = 55.0f32.to_radians();
    let dir = [0.0, a.sin(), a.cos()];
    b.cuts_first(&[
        Cut::slanted(up, lance, hand, dir, 3.0, false).bare().only(&[mat::GOLD, mat::METAL]),
        Cut::slanted(up, lance, hand, dir, -3.5, true).bare().except(&[mat::SKIN]),
        Cut::cube(up, lance, [-INF, -INF, cz + 20.0], [INF; 3]).only(&[mat::CLOTH]),
    ]);
    torso(&mut b, [cx, 40.0, cz], [cx, 54.0, cz], [cx, 68.0, cz], [cx, 86.0, cz], [cx, 90.0, cz]);
    let arm_l = [[cx + 22.0, 81.0, cz], [cx + 26.0, 64.0, cz + 4.0], [cx + 25.0, 50.0, cz + 13.0], [cx + 24.7, 45.0, cz + 16.0]];
    let arm_r = [[cx - 22.0, 81.0, cz], [cx - 27.0, 66.0, cz + 6.0], [cx - 18.0, 76.0, cz + 17.0], [cx - 15.6, 78.7, cz + 20.0]];
    arms(&mut b, &arm_l, &arm_r);
    b.at(Bone::Offhand, hand);
    legs(&mut b, cx, &[[cx + 8.0, 40.0, cz], [cx + 8.0, 22.0, cz + 1.0], [cx + 8.0, 6.0, cz], [cx + 8.0, 1.0, cz + 14.0]]);
    let t1 = b.bone("tail.1", Bone::Hips.index(), [0.0; 3]);
    let t2 = b.bone("tail.2", t1, [0.0; 3]);
    b.part("tail", t1);
    b.chain(&[t1, t2], &[[cx, 26.0, cz - 10.0], [cx - 6.0, 5.0, cz - 25.0], [cx - 34.0, 3.5, cz - 8.0]], None);
    b.finish("asmodeus", false)
}

// ------------------------------------------------------------------ the lair (FINALE.md §3)

/**
 * Krampus as he fights (grid 100 × 130 × 96, `foes_lair::KRAMPUS_SIZE`; x 50 his middle): hunched
 * forward, his belly and chest, his chains and bells, the basket on his back his hips, spine and
 * chest; his head thrust forward and low; his goat's legs on the heroes' leg bones (the thigh
 * forward to the hock, the shank back down to the cloven hoof); his sack of coals at his hip its own
 * bone on his hips. The birch bundle and its ribbon in his right fist (the sculpt's low x) the
 * `blade`; the chain and its great bell from his left the `offhand`.
 */
pub fn krampus() -> &'static Joints {
    let cx = 50.0;
    let mut b = Build::humanoid();
    const BODY: &[u32] = &[mat::FUR, mat::SKIN, mat::IVORY];
    b.cuts_first(&[
        Cut::cube(Bone::UpperArmR.index(), Bone::Blade.index(), [-INF; 3], [INF; 3]).except(BODY),
        Cut::cube(Bone::UpperArmL.index(), Bone::Offhand.index(), [-INF; 3], [INF; 3]).except(BODY),
    ]);
    b.part("armR", Bone::UpperArmR.index()).part("armL", Bone::UpperArmL.index());
    b.part("legR", Bone::ThighR.index()).part("legL", Bone::ThighL.index());
    torso(&mut b, [cx, 44.0, 38.0], [cx, 58.0, 39.0], [cx, 70.0, 42.0], [cx, 80.0, 46.0], [cx, 82.0, 48.0]);
    let arm_l = [[cx + 21.0, 81.0, 44.0], [cx + 26.0, 66.0, 52.0], [cx + 27.0, 52.0, 60.0], [cx + 27.0, 44.0, 64.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    b.at(Bone::Offhand, [cx + 27.0, 46.0, 63.0]).at(Bone::Blade, [cx - 27.0, 48.4, 62.0]);
    legs(&mut b, cx, &[[cx + 10.0, 45.0, 40.0], [cx + 10.0, 25.0, 32.5], [cx + 10.0, 6.0, 43.0], [cx + 10.0, 1.0, 50.0]]);
    let sack = b.bone("sack", Bone::Hips.index(), [cx - 17.0, 57.0, 38.0]);
    b.part("sack", sack);
    b.finish("krampus", false)
}

/**
 * A Percht (grid 48 × 82 × 40, `foes_lair::PERCHT_SIZE`; x 24 and z 20 its middle): its shaggy
 * sheepskin pelt and belt of cowbells on its hips, its legs fur to the knee and booted, its arms in
 * shaggy sleeves, its carved red mask and horns its head. (Its code parts are named as it faces
 * the hero: `leftLeg` and `leftArm` at the sculpt's high x.) The birch switch in its right fist (the
 * sculpt's low x) the `blade`.
 */
pub fn percht() -> &'static Joints {
    let (cx, cz) = (24.0, 20.0);
    let mut b = Build::humanoid();
    b.part("leftLeg", Bone::ThighL.index()).part("rightLeg", Bone::ThighR.index());
    b.part("leftArm", Bone::UpperArmL.index()).part("rightArm", Bone::UpperArmR.index());
    b.cuts_first(&[Cut::cube(Bone::UpperArmR.index(), Bone::Blade.index(), [-INF; 3], [INF; 3]).except(&[mat::FUR, mat::LEATHER])]);
    torso(&mut b, [cx, 30.0, cz], [cx, 40.0, cz], [cx, 47.0, cz], [cx, 55.0, cz + 1.0], [cx, 56.0, cz + 1.0]);
    let arm_l = [[cx + 11.0, 52.0, cz], [cx + 13.6, 41.0, cz + 3.0], [cx + 13.0, 33.0, cz + 6.0], [cx + 13.0, 28.0, cz + 8.0]];
    arms(&mut b, &arm_l, &mirror(&arm_l, cx));
    b.at(Bone::Blade, [cx - 13.0, 30.0, cz + 8.0]);
    legs(&mut b, cx, &[[cx + 5.0, 30.0, cz], [cx + 5.0, 14.0, cz + 0.6], [cx + 5.0, 3.0, cz + 1.0], [cx + 5.0, 1.0, cz + 6.0]]);
    b.finish("percht", false)
}
