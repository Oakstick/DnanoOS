# Animation by clips: the pipeline, piloted on Ragnar, now all three heroes

How the heroes (and later the cast and the foes) are animated by clips made in Blender, from
motion of any source (keyed by hand or by script, Mixamo, video-to-motion, text-to-motion, paid
services later), instead of only by curves written in code. The pilot is Ragnar: his skeleton, his
first seven clips keyed by script, then twelve clips from Mixamo's sword and shield motion, the
whole chain from Blender to the game, and the old animation beside the new for comparison. Mehmed
and Miyamoto followed: their joint tables, and twelve clips each from Mixamo (§6.5, §6.6).

Status: the pilot is built on `anim-ragnar` (2026-10-05), and the Mixamo step is done (§6): Ragnar
plays by twelve clips retargeted from Mixamo by default; `VH_ANIM=code` gives the old animation
back. The scripted clips of the first pass are kept in the history (and in `block_ragnar.py`) for
comparison, not in the build. Mehmed (the janissary, sabre and kalkan) and Miyamoto (the samurai,
his katana in both fists, his shield slung on his back) play by clips of their own since
`anim-heroes` (2026-10-07). What is left is in §10.

---

## 0. The decisions (they hold for every clip and every hero)

| Decision | What it means |
|---|---|
| **The voxel look stays** | Each voxel follows exactly one bone, rigidly: no weighted skinning, no bending voxels. Fidelity comes from more joints and real clips, not from a new art style. |
| **More joints** | A hierarchy of twenty bones (§1) where the code's rig had six parts. The sculpt is not redone: it is cut into the bones by planes and boxes at the joints, measured on it (§1.3). |
| **Hierarchy in the engine** | Rig parts have parents; their matrices are worked out on the CPU, parents first, and the GPU still gets one matrix a part. A rig without parents (every foe, the story's cast, the heroes animated by the code) multiplies exactly as before: its captures are pixel-identical. |
| **Clips are GLB files** | glTF skeletal animation on that skeleton, embedded in the build (`include_bytes!`: the web build has no file system), read with the pure-Rust `gltf` crate (no file or image loading: it builds for `wasm32-unknown-unknown`), once per hero, with his level, never mid-fight. A sidecar JSON per clip carries its markers (§4), written by Blender's export from the action's markers. |
| **Frame data rules the clip, never the reverse** | COMBAT.md §0 holds: the fight's startups, active frames and recoveries are the truth. A strike's clip is warped piecewise linearly between its markers so its wind-up ends exactly when the move's startup ends, its blow spans the active frames and its follow-through the recovery, at any tempo (haste, a heavy blade, mastery). The run's rate follows his speed over the ground (a stride a cycle), the leap's clip the leap's progress. Nothing in the simulation reads the animation (§9: the harness numbers are unchanged, and a test plays the same bout both ways). |
| **The code's animation stays** | It is the fallback (a missing clip hands the moment back to it) and the comparison: `VH_ANIM=code` forces it, `--anim-compare` renders both side by side (§8). |
| **Blending** | A crossfade of 8 frames between states (4 into a leap, which springs at once), none into a strike (its wind-up must show from its first frame). The guard and a blow's flinch are laid over his upper body, his legs running on beneath (no blow roots him, COMBAT.md §1.2). The juice is added on top: the blade's overshoot after a strike, the shield's jolt, the landing's squash, the hit pause (which holds the fighter's frame, and so the clip). |

---

## 1. The skeleton

### 1.1 The bones

Twenty bones, parents first (`render/anim/skeleton.rs`, `BONES` and `PARENTS`). `.L` and `.R` are
his own left and right, so Blender mirrors a pose across him.

| Bone | Parent | Carries (Ragnar) |
|---|---|---|
| `hips` (the root) | | his belt, the skirt of his tunic, the seax and pouch, the cloak's hem |
| `spine` | hips | the waist up to the chest, the cloak's middle |
| `chest` | spine | the chest, the fur mantle, the top of the cloak |
| `neck` | chest | the column of skin under his head |
| `head` | neck | the head, the helmet and horns, the beard and its braids, the braid down his back |
| `upper_arm.L`, `.R` | chest | the sleeve to its bunch at the elbow |
| `forearm.L`, `.R` | upper arm | the bracer |
| `hand.L`, `.R` | forearm | the fist (and the bracer's cuff) |
| `thigh.L`, `.R` | hips | the cross-gartered trousers |
| `shin.L`, `.R` | thigh | the fur cuff and the boot's shaft |
| `foot.L`, `.R` | shin | the boot below the ankle, and its toe |
| `blade` | hand.R | the weapon of his armory in his right fist (its joint at the grip; the fist modelled round its grip goes to the hand, so a turn of the weapon in his fist turns the weapon alone) |
| `offhand` | hand.L | its twin in his left fist, for a paired weapon (empty otherwise) |
| `shield` | forearm.L | the shield of his tree strapped to his forearm (its joint mid-forearm) |

### 1.2 Axes

The game's and glTF's are the same: **Y up, he faces +Z, +X is his own left, a metre is a tile.**
Every bone rests unturned in the game, so a bone's own axes are the game's. Blender's glTF import
makes that Z up (he faces -Y, his left at +X: a game vector (x, y, z) is Blender's (x, -z, y)), and
its bones come in with those same axes, so a pose bone's rotation in Blender is a turn in the
game's axes relative to its parent. Turns by angle (as `posing.py` and the code both write them):
**x** pitches (a limb hanging down swings back for positive, forward for negative; the spine
leans forward), **y** turns (positive turns his front toward his left), **z** tips (positive takes
a hanging limb toward +X, his left).

### 1.3 The cut, as data

The code's rig (body, two legs, two arms, head, and the weapons and shield layered on) is cut into
the bones by a table per hero (`skeleton::Joints`): each bone's joint, measured on his sculpt in
voxels of his own grid (x as sculpted, before the rig mirrors him), and a list of cuts, each
moving the voxels of one bone that lie in a box to the next bone down the limb, in order. Each old
part starts in its bone (body in the hips, an arm in its upper arm, the head in the head, the
weapons and shield in theirs). Ragnar's (grid 60 × 92 × 36):

| Joint | At (x, y, z) | Cut |
|---|---|---|
| hips | 29.5, 30, 17.5 | |
| spine | 29.5, 35.5, 17 | the hips above y 35.5 (the top of his belt) |
| chest | 29.5, 45.5, 16.5 | the spine above y 45.5 |
| neck | 29.5, 55.5, 16.5 | the head's voxels in the box x 23.5..35.5, y below 60.5, z 10.5..20.5 (the column of skin, not the beard before it) |
| head | 29.5, 60.5, 16 | |
| upper arm | 10.5 / 48.5, 54, 16.5 | (the code's shoulder pivots) |
| forearm | 7.5 / 51.5, 44.5, 17 | the upper arm below y 44.5 (the sleeve's bunch) |
| hand | 7.5 / 51.5, 34.5, 17.2 | the forearm below y 34.5 (the bracer's end) |
| thigh | 21.5 / 37.5, 28, 19.5 | (the code's hip pivots) |
| shin | 21.5 / 37.5, 14.5, 19.3 | the thigh below y 14.5 (the fur cuff rides on the shin) |
| foot | 21.5 / 37.5, 4.5, 19 | the shin below y 4.5, and its toe (z from 24.5, y up to 8.5) |
| blade, offhand | 51.5 / 7.5, 29.8, 17.4 | the grip in each fist |
| shield | 7.5, 40, 17 | the middle of the shield forearm, where it is strapped |

A cut across a limb at a height is **sleeved** (`skeleton::SLEEVE`: the band of the bone above the
joint laid on the bone below as well, so a bend overlaps itself). Where the plane would cross other
parts too, the cut is kept to a box across the limb (`Cut::below_within`, its sleeve with it), and a
joint hidden under a skirt is cut unsleeved (`Cut::bare`: its sleeve would lay a band of the skirt
on the shin). A box cut (the neck, the toe) is never sleeved.

**Mehmed's** (grid 96 × 124 × 40, `heroes::MEHMED_SIZE`; his left at low x):

| Joint | At (x, y, z) | Cut |
|---|---|---|
| hips | 47.5, 30, 19.5 | his sash; the skirt below it goes to the thighs (below) |
| spine | 47.5, 35.5, 19.5 | the hips above y 35.5 (the top of his sash) |
| chest | 47.5, 46, 19.5 | the spine above y 46 (the frogging up, the collar) |
| neck | 47.5, 56.5, 18.5 | the head's voxels in the box x 42.5..52.5, y below 60.5, z 13..19.4: the column of skin behind his beard |
| head | 47.5, 60.5, 18.5 | (the beard, the moustache, the börk and its flap) |
| upper arm | 21.5 / 73.5, 54, 18.5 | (the code's shoulder pivots; the wide sleeve) |
| forearm | 21.5 / 73.5, 41.5, 19.2 | the upper arm below y 41.5; the gold cuff stays on it |
| hand | 21.5 / 73.5, 28.5, 19.5 | the forearm below y 28.5 (the fist) |
| thigh | 39.5 / 55.5, 28, 20.5 | **the skirt's halves**: the hips below y 29.5, cut down the middle (x 47.5), each half to its thigh, sleeved |
| shin | 39.5 / 55.5, 14.5, 21 | the thigh below y 14.5, unsleeved: the boot from the hem down |
| foot | 39.5 / 55.5, 5.5, 21.5 | the shin below y 5.5, and the turned-up toe (z from 27.5, y up to 8.5) |
| blade, offhand | 73.5 / 21.5, 26, 19.5 | the grip in each fist (where `armory::weapon_for` leans the weapon from) |
| shield | 21.5, 36, 19.5 | the middle of the shield forearm, where the kalkan is strapped (`forge::forearm`) |

His kaftan's skirt (and the crimson robe under it, and the sash's hanging end) rides his thighs, half
on each, rather than his hips: from the chase camera his stride shows as the skirt parts and swings
(as the long kaftan does over the janissary's wide trousers), where a skirt kept on his hips would
hide his legs and slide over his boots. Under the skirt only his boots are sculpted, so his knee is
hidden; the knees are cut before the skirt joins the thighs (the cuts apply in order), so the skirt
never goes down to a shin.

**Miyamoto's** (grid 96 × 120 × 44, `heroes::MIYAMOTO_SIZE`):

| Joint | At (x, y, z) | Cut |
|---|---|---|
| hips | 47.5, 30, 21.5 | the obi, the short sword and the top lame of the armoured skirt |
| spine | 47.5, 37.5, 21.5 | the hips above y 37.5 (the top of the obi); the short sword's hilt (x 30..39, y 35..42, z from 32.5) back to the hips |
| chest | 47.5, 48, 21.5 | the spine above y 48 (the dō from the middle of its lames up) |
| neck | 47.5, 54.5, 21 | the head's voxels in the box x 37..58, y below 59.5, z 12..31: the lacquered throat guard |
| head | 47.5, 60, 20.5 | (the mask, the kabuto, its neck guard, turnbacks and crest) |
| upper arm | 21.5 / 73.5, 52, 19.5 | (the code's shoulder pivots) with the **shoulder guards** (sode), as sculpted to swing with the arm |
| forearm | 21.5 / 73.5, 37, 20 | the upper arm below y 37: below the sode, so the elbow's sleeve is two rows of silk and mail, none of the lames |
| hand | 21.5 / 73.5, 29.5, 20.5 | the forearm below y 29.5 (the gauntlet and the fist) |
| thigh | 39.5 / 55.5, 29, 23 | **the armoured skirt's halves**: the hips below y 29.5 (between its top lame and the next), down the middle, each to its thigh, sleeved (the lower lames and the hakama's seat) |
| shin | 39.5 / 55.5, 15.5, 24 | the thigh below y 15.5 within x 7 of the leg (its sleeve kept to the leg): the shin guards and the hakama's legs |
| foot | 39.5 / 55.5, 5.5, 24.5 | the shin below y 5.5: the tabi and sandal |
| blade, offhand | 73.5 / 21.5, 26, 19.5 | the grip in each fist |
| shield | 47.5, 42.5, 10 | **the middle of the shield slung on his back** (`heroes::MIYAMOTO_SHIELD_AT`): his jointed rig keeps it there (`forge::jointed_strap`), and the animator brings it before him to guard (§3, `anim::hold`) |

His kusazuri is cut between its lames, the top lame staying under his obi, the lower ones parting
with his stride like the panels they are; the hakama's wide legs (rows 16 to 19, at the knee) go
with his thighs above the knee and his shins below it. The short sword through his obi stays with
his hips (its hilt cut back from the spine); the scabbard's end, out at the back under the skirt,
rides the left thigh (the middle of it is inside the skirt, so no break shows).

The tests (`skeleton::tests`, for all three) check the cut keeps every voxel exactly once and in its
place, that none lies below the next joint down its limb (but Mehmed's skirt) or beyond its bone's
reach (scaled by the hero's height; the skirts' reach is their own), that the left limbs and the
left half of a skirt stay on his left, and that every sleeve's voxel is a copy of the bone above a
joint laid on its child within a few voxels of the crease (and never a skirt on Mehmed's shin);
`VH_ANIM=bones` paints each bone its own colour in the game, and `import_rig.py` renders the same in
Blender (§8).

---

## 2. The pipeline, step by step

From `rust/`, with `blender` the Blender 4.5 executable (headless; no add-ons, no preferences
changed). Every script is described in `tools/blender/README.md`.

1. **The rig out of the game.**
   `AdventuresOfTheAncientBlades --export-rig ragnar ragnar_rig.glb` writes his jointed rig (his
   first blade, his first shield on his arm): a node per bone placed relative to its parent at
   rest, a mesh per bone (each voxel's open faces, culled as the resident rig culls them, vertex
   colours the voxels' linear colours), every vertex bound wholly to its bone (a skin, its inverse
   bind matrices the joints' rests). About 10 MB, 41 thousand faces.
2. **Into Blender, checked.**
   `blender -b -P tools/blender/import_rig.py -- ragnar_rig.glb OUT [ragnar.blend]`: the bones and
   parents, each mesh weighted 1 to its own bone, colours present; a Workbench render in his
   colours and one in the bones' colours.
3. **The clips.** Keyed by hand in Blender, by script (`block_ragnar.py`), or retargeted from
   outside motion (`retarget.py`, `mixamo_hero.py`, §6): each an action on his armature, with
   its markers as pose markers on the action, `vh_loop` and `vh_stride` as custom properties.
   `blender -b -P tools/blender/block_ragnar.py -- ragnar_rig.glb ragnar.blend` (the armature and
   the scripted clips), then
   `blender -b ragnar.blend -P tools/blender/mixamo_hero.py -- ragnar C:/Users/ulask/Games/mixamo ragnar.blend`
   (the Mixamo clips, replacing the scripted ones of the same names). For Mehmed and Miyamoto, the
   .blend `import_rig.py` saves (`--export-rig mehmed`, `miyamoto`) and `mixamo_hero.py -- mehmed`
   or `-- miyamoto`.
4. **A look before the game.**
   `blender -b ragnar.blend -P tools/blender/preview.py -- OUT [action ...] [--yaw 180 --rise 0.7]`
   renders each action as contact sheets (`--at 0.62 --dist 2.7` for the taller two).
5. **Out to the game.**
   `blender -b ragnar.blend -P tools/blender/export_clips.py -- assets/anim/ragnar` writes
   `<action>.glb` (the armature alone, sampled at 60 fps) and `<action>.json` (markers, loop,
   stride) for each action.
6. **Build.** `build.rs` lists every `assets/anim/<hero>/<clip>.glb` that has its `.json` beside it;
   `render/anim/library.rs` embeds them. Nothing else to register.
7. **Compare.** `python tools/anim_compare.py OUT [--hero K]` (§8); `--anim-bench` and
   `--blade-view` for the numbers (§9).

---

## 3. In the engine

`render/anim/`:

- **`skeleton.rs`**: the bones, the joint tables, `jointed` (the cut), `assign` (which bone each
  voxel goes to), the debug colours.
- **`clip.rs`**: `Clip::from_glb` reads a GLB and its markers. The file's bones are found by name;
  each must hang from the skeleton's parent (or the clip is refused, and the code plays that
  moment). Every track is resampled at 60 Hz (linear and step keys exactly; a cubic spline's keys
  at their values), a bone without a track holding its rest. `Clip::sample(t)` slerps between the
  two frames about `t` (looping clips wrap). `Skeleton::matrices` turns a pose (each bone's
  transform relative to its parent) into each part's matrix in the model: the bone's matrix now,
  times its rest matrix inverted, times the joint's place, so at rest every part sits on its joint
  as the code's pose puts it, whatever rest orientation the file gives its bones. It can also turn
  a bone about its joint in the model's axes (the juice), its children following.
- **`warp.rs`**: `warp(t, game, clip)`, the piecewise-linear map from a move's phase boundaries
  onto a clip's markers.
- **`hero.rs`**: the animator (`Animator::step`), reading only `Drive` (his speed and stride, the
  leap's progress, the strike's time, bounds and side, the guard, the reel, the juice) and handing
  back a pose, or nothing when a clip the moment needs is missing:
  - **On his feet**: `idle` and the gait blended by his stride (`motion`); the gait is `walk` giving
    way to `run` between 2.4 and 4 tiles a second (under his guard he walks), both on one cycle
    that advances by his speed over the ground over their `stride`.
  - **The guard**: `block` over his upper body, its raise (to `raised`) fitted into the guard's
    raise frames (`fsm::GUARD_RAISE`), then `block_idle` looping (or the raise's tail, without
    one); in over 3 frames, out over 8.
  - **A parry**: `parry` over his upper body as his guard turns a blow (the game tells the view,
    `WorldView::hero_parried`, where the parry's sparks and jolt are set off).
  - **A blow taken**: `hit` over his upper body, through the reel's frames (hitstun, a broken
    guard). Blockstun shows on the shield (the jolt).
  - **A strike**: `slash_fore` for the swing that coils to his right first (the view's side -1, as
    the code's), `slash_back` for the other; the swings alternate as they always have; his string's
    finisher (L3) swings `slash_heavy` (`WorldView::sword_finisher`). Its time is
    the fighter's frame where frame data rules him (the Sparring Ring, the realms), or the view's
    own clock with the old swing's proportions elsewhere.
  - **The leap**: `jump` from `takeoff` to `land` by the leap's progress, then its landing to
    `recover_end` in real time (cut short by a strike, the guard or a run). In a fight the leap is
    the evade (the fighter's `Evade` state): `roll`, the same way.
  - **What he carries** (`Drive::two_hands`, `Drive::slung`, laid on the finished pose by
    `hold.rs`): both fists on the grip, his shield slung on his back but to guard (Miyamoto, §6.6).
  - **The juice**: the upper sword arm carried on past the strike's end and back
    (`BLADE_OVERSHOOT`'s spring, about his upright; his chest, with both arms, when he holds his
    weapon two-handed) unless the springs are off; the shield forearm knocked back by
    `shield_tilt` (a slung shield before him, about its own middle); the landing's squash on the
    root.
- **`hold.rs`**: what he carries, laid on the pose after the clips are sampled and blended (so a
  crossfade, the guard laid over his upper body or a flinch never parts his hands from the hilt):
  - `two_hands`: the sword hand keeps the weapon where the clip holds it, turned square round its
    grip; the off hand closes on the grip a fist below it (`FIST_GAP`), turned as the blade is; a
    two-bone solve bends each arm to reach, the elbows on the side the clip bends them. The heroes'
    arms are short against their shoulders, so where the two cannot meet where the clip holds the
    grip (the clip's arms leave it out at his side, where the source's long arms had it in the
    middle), the grip is drawn in toward his middle (`INWARD`) and before his breastplate
    (`BEFORE`), as far toward the middle of his front as it takes, and each shoulder comes forward
    and in, level, up to `REACH_IN`.
  - `sling`: the shield rides his chest on his back (a little lower than it was carved and to his
    shield side, `SLUNG`, so his sword shoulder and his cuts show over it from the chase camera),
    and by the guard's weight (or a parry's) comes round his shield side to stand before him,
    square to the foe, a little before the middle of his shield forearm (`GUARD_OUT`), wherever his
    guard holds that arm.
- **`library.rs`**: the embedded clips, read once per hero (`clips(hero)`, also called from
  `prepare_rigs` so it happens behind the loading screen), and `Mode` (`VH_ANIM`).
- **`export.rs`** (native only): `--export-rig`, `--anim-bench` (all three heroes) and
  `--blade-view` (§9).

In the view: `forge::jointed_rig` cuts the hero's rig each time his kit changes (and the spare for a
broken shield with it), its shield strapped to his arm at its full size (the code's arm carries it at
0.8 and swaps in the shield off his back to block; the clips raise the one on his arm), or for
Miyamoto left on his back as carved (`forge::jointed_strap`); `prepare_rigs` meshes it at every
level of detail with the level; `animate_clips` tells the animator Miyamoto holds his weapon in both
fists (but a pair of blades, a bow or a gun) and slings his shield;
`animation::animate_clips` runs after the code's `animate_hero` (which always runs: it stays the
fallback, and the Hall of Mirrors' reflections still use it) and leaves the parts' matrices in
`hero_mats`; `emit` draws the jointed rig with `RigList::push_matrices` when they are there (and
his haste's afterimages with them), the code's rig otherwise. The hierarchy itself is in
`RigList::push` (a part with a parent rides on its parent's matrix) for any rig that has parents.

---

## 4. Markers

Frames from the clip's first, at 60 Hz (the sidecar's `fps` converts others). Unknown names are
kept and ignored.

| Marker | Clip | Meaning |
|---|---|---|
| `windup_end` | a strike | The wind-up's end: the move's last startup frame lands here. Put it where the blade reaches the foe. |
| `hit` | a strike | The moment of contact (for reference and for effects; the warp uses the next two). |
| `active_end` | a strike | The blow's end: the move's last active frame. |
| `recover_end` | a strike, `jump`, `hit` | The follow-through settled: the move's (or the landing's, or the flinch's) end. |
| `takeoff`, `apex`, `land` | `jump` | The leap: off the floor, at its top, touching down. `takeoff` to `land` fills the leap. |
| `takeoff`, `apex`, `land` | `roll` | The same for the evade's roll: the dive, the tuck, the rise. |
| `raised` | `block`, `parry` | The shield up: the guard's raise ends here (without `block_idle`, the rest loops while he holds it). |
| `impact` | `hit`, `parry` | The blow's moment in the flinch, or on the shield. |
| `foot_l`, `foot_r` | a gait | Each heel strike (for footfalls and phase). |
| `loop` (the sidecar's `"loop": true`) | a cycle | It wraps; its last frame is its first again. |
| `stride` (the sidecar's `"stride"`) | a gait | Metres (tiles) a cycle carries him: the rate is his speed over it. |

A strike without markers is read as 0.3, 0.45 and 1 of its length; a leap as 0, 0.7 and 1.

---

## 5. Adding a clip

1. Make it an action on the hero's armature in his .blend (keyed, scripted or retargeted). Name
   it as the game knows it: `idle`, `walk`, `run`, `slash_fore`, `slash_back`, `slash_heavy`,
   `block`, `block_idle`, `parry`, `jump`, `roll`, `hit` replace his own; a new name is embedded but
   nothing plays it until `anim::hero` asks for it.
2. Add its markers (pose markers in the action editor, §4), and for a cycle `vh_loop` (and for a
   gait `vh_stride`) as custom properties of the action.
3. `export_clips.py` it into `assets/anim/<hero>/`, build, and compare (§8). The tests check every
   clip of each hero reads, has its markers in order and rests on the joints the game cut him at,
   and that no two heroes share an idle or a strike.

A new kind of moment (a sidestep, a death) is a clip and a few lines in `Animator::step` choosing
it from what `Drive` says.

---

## 6. Outside motion: the Mixamo step (done 2026-10-05)

`retarget.py` (see `tools/blender/README.md`): Mixamo FBX, or BVH from any tool, onto a hero's
armature, baked at 60 fps, in place, scaled by leg length, his weapons in his hold. Its self-test on a
synthetic Mixamo-named armature: every mapped limb follows the source's direction to within a
hundredth of a degree, the hips stay over his feet and rise by the source's rise times the leg
scale (0.33); a BVH read lying down is stood up first.

### 6.1 The files

Mixamo's Pro Sword And Shield Pack (51 clips) and "X Bot", and "Dodging Right" and "Stand To Roll",
downloaded as FBX Binary without skin, kept **outside the repository**
(`C:/Users/ulask/Games/mixamo/sword-shield/` and `extra/`): Mixamo's motion may ship in the game,
its files may not be passed on. Only the clips exported from his .blend go into `assets/anim/`. For
Mehmed and Miyamoto the same, and Mixamo's **Great Sword Pack** (51 clips, two-handed:
`great-sword/`) and its "Sword And Shield Pack" (`sword-shield-classic/`: the same clips as the Pro
pack but drawing and sheathing, so not needed).

**Frame rate.** The pack was asked for at 60 fps but came at 30 (so did the Great Sword Pack): its keys fall on whole frames at
30 (one curve of one clip has a few stray keys between them, Mixamo's own). The two extras came at
60. Blender's FBX import sets the scene to each file's own rate and lays the keys out at it;
`retarget.load_source` reads the rate after the import and gives the scene back its 60, and the
bake samples the source at its own rate every 60th of a second (a 30 fps clip's in-between frames
are its keys interpolated). The real data shows the Mixamo name map as written: all 17 of his
mapped bones found as `mixamorig:` names on the 65-bone skeleton, the armature turned and scaled
by the import (centimetres), which the world-space retarget takes as it comes.

### 6.2 The picks, and why

All made by `tools/blender/mixamo_hero.py` (his table is there), from his .blend:
`blender -b ragnar.blend -P tools/blender/mixamo_hero.py -- ragnar C:/Users/ulask/Games/mixamo ragnar.blend`,
then `export_clips.py`. Every candidate was retargeted and looked at as contact sheets from before
him and from behind; the strikes were also measured: the share of their active frames on which the
blade can be seen from the game's chase camera (behind and above him, `camera::chase`), by casting
rays from it in Blender against his voxels.

| Slot | From | Corrections | Why |
|---|---|---|---|
| `idle` | Sword And Shield Idle (4) | stance (§6.3) | A steady guard, shield forward, breathing. Idle (1) turns him round to look behind him; (2) and (3) are 7 and 9 s long and swing the sword about. |
| `walk` | Sword And Shield Walk | stance + 15°, stride 1.3 | Used under the guard, where he walks at about half his pace. |
| `run` | Sword And Shield Run | stride 2.4 | A real run, leaning in, arms pumping; Run (2) is a guarded jog behind the shield, side-on. |
| `slash_fore` | Sword And Shield Slash (5) | sword arm 35° higher | The cleanest cut from high right to low left; the others spin, lunge or combine. Raised, its blade is seen on 69% of its active frames from behind, against 10% for the scripted forehand (the raise was worth 15 to 25 points in the trials). |
| `slash_back` | Sword And Shield Slash (5), **run backward** | sword arm 35° higher | Every Mixamo backhand (Attack (2), (3), Slash (3)) spins him a whole turn: in a 25-frame L2 that is a blur that ends with his back to the foe (tried: the spin cut to 60% and to 25%, still facing away at the blow). Run backward, the forehand is a rising cut from low left to high right, and the pair chains: one's end is the other's start. 69% seen from behind. |
| `slash_heavy` | Sword And Shield Slash (3) | sword arm 15° right, 30° up | The finisher (L3, 36 frames, the slowest): the spinning cut, spin and all, is right for it. 66% seen from behind (its spin turns the blade away for a while). |
| `block` + `block_idle` | Sword And Shield Block, Block Idle | stance | The shield square before him, its centre on his middle (x 0.01 m), his sword drawn back behind it. |
| `parry` | built: the block's raised pose reached in 3 frames, then the Impact's flinch on his upper body at half depth, back by frame 22 | | Mixamo has no parry. |
| `jump` | Sword And Shield Jump | | Off the floor on frame 3 (the game's leap starts at once), tucked at the top, a full landing; it faces forward as he leaps. |
| `roll` | Stand To Roll (extra) | its turning taken out | The evade in a fight: a dive and tuck through the air over the blow, from `takeoff` (his hips sinking into the dive) to `land`. |
| `hit` | Sword And Shield Impact (2) | stance | Thrown back and recovering, upper body only in the game. Impact (1) and (3) twist him 80 and 130 degrees. |

**Not used**: Dodging Right (the game has no sidestep: its evade is the leap, which the roll plays),
the deaths (the game does not show his body once he falls), the crouches, strafes, turns, kick,
casting, power up, drawing and sheathing (nothing in the game asks for them yet).

### 6.3 His build against Mixamo's

Mixamo's bot is 1.8 m with long limbs; Ragnar is stocky, with a big head and horns, short arms and
a wide body. What the retarget does about it (`retarget.Retarget`, per pick in `mixamo_hero.py`):

- **His stance.** Mixamo fights side-on, the shield first, its hips 55 to 67 degrees to the right.
  Every clip of the stance (`idle`, `walk`, `block`, `block_idle`, `hit`, the strikes' starts) is set
  to one heading, 50 degrees (`face`), so the guard laid over the idle sits on the hips it was made
  on (one set at Mixamo's own heading put the shield off at his side) and a strike starts where the
  idle leaves him.
- **Arms out** 6 degrees (8 in the run), so his arms clear his body.
- **The sword arm raised** in the strikes (`sword_turn`): from behind, his helmet, horns and mantle
  hide anything before his chest; a cut 30 to 35 degrees higher passes above his shoulders.
- **Spins taken out** where they would leave him facing away (`yaw_keep`), and the roll's turning.
- **The shield at its full size** on his arm (the code's arm carried it at 0.8): the block covers
  him as the raised shield did.
- His hips' movement is scaled by his leg (0.33 of the bot's), and his travel taken out.

**The blade in his fist.** In the pilot's scripted clips the blade was keyed by the way it should
point, and wrongly: the weapon is modelled leaning forward 23 degrees out of the fist
(`armory::LEAN`), which the keys took for straight up, and several keys pointed it out sideways (the
first build's block held it out through the shield; the run angled it out to his right). It was a
clip problem, not the rig's: the blade's part is the hand's child, its joint at the grip in the
fist. Now the blade follows the source's own fist: Mixamo's skeleton has finger bones, and a fist
holds a grip running from the little finger's knuckle to the forefinger's (the blade leaving on the
thumb's side), its edge toward the knuckles; `retarget.Retarget::grip` turns his blade (its leaning
rest taken into account) onto that line every frame. Measured over each clip, the blade now sits 80
to 110 degrees from the forearm, as a held sword does, in every clip but the strikes (where it
swings through the forearm's line); it rests up before him in the idle and the run, down behind the
shield in the guard. The fist modelled round the weapon's grip was moved onto the hand bone, so
turning the weapon in his hand no longer turns a second fist with it.

### 6.4 Markers, found from the motion

`mixamo_hero.py` finds them, so a pick changed is marked again the same way: a strike's from the
blade's tip speed (`hit` at its fastest, `windup_end` where it first runs at half that, `active_end`
where it falls back under half, `recover_end` at the end; the lead-in before the blade first stirs
cut away), so the first active frame shows the blade already on its way through; a leap's from the
feet (`takeoff` the first frame both are off the floor, `land` back) or for the roll from the hips;
the guard's `raised` where the shield (or, slung, the off hand) stops rising; the flinch's `impact` at the chest's quickest
turn. Loops are closed: their last 8 frames ease onto their first.

**Other motion** (BVH out of video- or text-to-motion tools): `retarget.py --source take.bvh`. The
SMPL names (`pelvis`, `left_hip`, ... `right_wrist`) and the usual BVH names are both mapped. Clean
such takes in Blender before export: foot sliding and jitter are common in them.

### 6.5 Mehmed's picks, and why (2026-10-07)

From the same sword-and-shield pack as Ragnar's (Mixamo's other "Sword And Shield Pack" turned out
to be the same clips), but none of his idle or strikes are Ragnar's, so the two do not move alike;
and his carriage is his own, by the retarget's stance settings: squarer to the foe (his hips 30
degrees to his right where Ragnar's are 50), his back straight and his chest out (`posture`: the
spine and chest straightened 6 and 10 degrees from Mixamo's crouch), the janissary's discipline.
His wide sleeves are carried a little further out from his kaftan (8 degrees). His first blade is
the short yatagan; the kilij and the rest follow in his fist as Ragnar's do.

| Slot | From | Corrections | Why |
|---|---|---|---|
| `idle` | Sword And Shield Idle (3), 4.0 to 7.0 s | his stance and carriage; his head raised (the neck and head 10 and 17 degrees more) | A calm three seconds: the blade turned idly in his fist, his weight settling. Mixamo's soldier looks down at his feet there; Mehmed looks at the foe. Idle (4) is Ragnar's, Idle (1) turns round to look behind him, Idle (2) whirls the blade. |
| `walk` | Walk (2) | stance + 10, carriage, stride 1.3 | Ragnar's is the Walk. |
| `run` | Run (2) | stride 2.4, the spine straightened 4 | The guarded jog behind the shield: the soldier's run, kept in formation (Ragnar's is the open sprint). |
| `slash_fore` | Slash (1) | stance, carriage, sword arm 15 degrees right and 35 up | A diagonal cut from high at his right down across his front, reaching 0.9 m before him. Seen on 66% of its active frames from behind (50% with the arm 20 up: his tall börk hides what passes over his head). |
| `slash_back` | Attack (3) | its whirl taken out (`yaw_keep` 0), sword arm 15 up | The long, low sweep from his left out to his right, the blade a metre out at his right: the longest reach of the pack, the "halberd-tip" blow his mastery is about (WARBAND.md §11.1). Mixamo whirls him round in its wind-up; he stays square, his chest coiling and uncoiling. 85%. |
| `slash_heavy` | Attack (1) | stance, carriage | The blade raised high over his head and brought down in a chop before him: the soldier's heaviest blow. 100%. |
| `block`, `block_idle` | Block (2), Block Idle | stance, carriage | Block Idle is the pack's only held guard (Ragnar's too): the stance and carriage set his apart. |
| `jump` | Jump (2) | arms out 8 | Ragnar's is the Jump. |
| `roll` | Stand To Roll | as Ragnar's | The only roll there is. |
| `hit` | Impact (1) | stance, carriage | A short flinch with little twist (Ragnar's is Impact (2)). |
| `parry` | built, as Ragnar's | | |

Tried and set aside: Attack (4) (an overhead blow that ends 0.2 m before him), Attack (2) and Slash
(4) (a whole turn, then the cut), Slash (2) (a combination of cuts, 3 s long).

### 6.6 Miyamoto's picks, his grip and his shield (2026-10-07)

From the **Great Sword Pack**: a long blade held in both fists, which is kenjutsu's hold (both hands
on the hilt, the guard before him, big diagonal cuts). His stance is near square to the foe (his
hips 15 degrees to his right, as the pack's own guard has them).

| Slot | From | Corrections | Why |
|---|---|---|---|
| `idle` | Great Sword Idle | stance, arms out 4 | A steady guard, the blade raised before him, breathing; two seconds. Idle (2) turns about, (3) to (5) take a hand off the hilt. |
| `walk` | Great Sword Walk | stance, stride 1.3 | |
| `run` | Great Sword Run | stride 3.0 | Running with the blade held across before him; Run (2) runs side-on. |
| `slash_fore` | Great Sword Slash | stance | The diagonal cut (kesa-giri): coiled to his right, then from high at his right down through the foe to low at his left. 78%. |
| `slash_back` | Great Sword Slash (3) | stance | Coiled to his left, then the level cut at the height of his shoulders out to his right. The pair chains: each ends in the guard the other starts from. 84%. |
| `slash_heavy` | Great Sword Jump Attack | its whirls taken out (`yaw_keep` 0), and its travel (`still`) | The leaping cut: he springs where he stands and brings the blade down at his right. Mixamo's turns him round once in the wind-up and once in the recovery. The High Spin Attack cuts first and then spins him a whole turn, slamming down in its recovery: his back to the foe for half a second after the blow, and a second blow where the game has none; the leap keeps his face to the foe through the blow. 80%. |
| `block`, `block_idle` | Great Sword Blocking, Blocking (2) | stance | The blade raised across before him in both fists; the shield comes before him over it (below). |
| `jump` | Great Sword Jump (2) | arms out 4 | The Jump travels 1.7 m; this one leaps where he stands. |
| `roll` | Stand To Roll | as Ragnar's | |
| `hit` | Great Sword Impact (2) | stance | Impact (1) is from the guard's stance, (3) lets go of the hilt, (4) and (5) from crouches. |
| `parry` | built, as Ragnar's | | |

**The two-handed grip, solved in the game** (`anim::hold::two_hands`), not in the retarget: it
holds through everything laid on the clips after they are sampled (a crossfade between two clips, the
guard over his upper body, a flinch, the overshoot, which turns his chest rather than his sword arm
when he holds the blade in both fists), and it is left off for the weapons of his tree not held
that way (the pair of ninjato, the bow, the gun). The clips keep the source's arms, retargeted by
direction. That alone parts his hands: Mixamo's are 0.07 to 0.17 m apart on the hilt, Miyamoto's
come out 0.55 to 0.67 m apart, his fists at his sides, because his arms reach 0.33 m to the grip and
his shoulders are 0.65 m apart (his sode make them wide): from where the clip holds the grip the two
cannot meet at all. So the grip is drawn in toward his middle and out before his breastplate,
keeping its height, as far as it takes for both arms to reach it, his shoulders coming forward and
in up to 9 cm (his upper arms, sode and all, close in over his chest, as a swordsman's do); the
sword hand keeps the clip's turn of the blade, squared round its grip, and the off hand closes on
the grip a fist below it (`FIST_GAP`, 8.4 voxels: the katana's hilt below the sword fist is one fist
long), turned as the blade is. The elbows bend the way the clip bends them.

**The shield slung on his back** (`anim::hold::sling`): his jointed rig keeps the tate on his back
as it was carved (`forge::jointed_strap`), and every frame the animator hangs it from his chest
there, a little lower and to his shield side (`SLUNG`), so his sword shoulder and his cuts show over
it from the chase camera. To guard (the guard's weight, or a parry's), it comes round his shield
side (out 16 cm half way) to stand before him, square to the foe whatever way his chest is turned,
its middle a little before the middle of his shield forearm (`GUARD_OUT`), which carries it as his
guard moves; a blow it takes knocks it back about its own middle. What reads best from behind and
from his side: the samurai's guard, the blade raised across in both fists, with the tate before it
(the shield alone before him, his sword arm free, read as Ragnar's guard; the guard alone, as no
shield at all). The code's animation (`VH_ANIM=code`) keeps it strapped on his arm as before.

---

## 7. Licences

| Source | May its motion ship in the game? |
|---|---|
| Mixamo | Yes: Adobe allows Mixamo's characters and animations in commercial games. **The raw FBX files must not be redistributed**: they stay outside the repository (§6.1; the Great Sword Pack too); only the retargeted clips go into `assets/`. |
| Clips keyed by hand or by script here | Yes: ours. |
| MoMask (text to motion) | **Reference only.** Its models are trained on research-licensed data (HumanML3D, from AMASS): look at its output, key our own. |
| GVHMR (video to motion) | **Reference only.** It outputs SMPL, whose body model's licence is non-commercial. |
| HY-Motion (Tencent) | **Never.** Its licence excludes the EU, and the user is in Denmark. |
| Paid services (Move.ai, Plask, DeepMotion, Rokoko, Cascadeur...) | Check each one's terms for commercial game use before any of its output goes into `assets/`. |

---

## 8. Comparisons

- `python tools/anim_compare.py OUT [EXE] [--also EXE2] [--hero K] [moment ...]` renders 30 moments in the
  Sparring Ring (idle and a breath; the run, and from the chase camera; the walk under the guard;
  the first and second swings and the finisher at wind-up, blow, through and settle; the guard
  rising and held; the evade's roll rising, at its top and landing) and the leap in the Garden,
  close (from before him, three-quarters), from his side, and from the game's chase camera (behind and
  above him, as it is played). Each is one picture: **the code's animation on the left, the
  clips' on the right**, the same frame of the same deterministic run (`--capture ...
  --anim-compare`: the frame is drawn twice, once each way). With `--also EXE2`, a build carrying
  other clips, it is three panels: the code, EXE2's clips, EXE's. The comparisons of §6 were made
  with the build of the scripted clips as EXE2: **the code | the scripted clips | Mixamo's**.
  `--hero K` plays hero K (0 Ragnar, 1 Mehmed, 2 Miyamoto: `hero:K` in the capture's script, which
  any capture can use to show any of the three).
- `--blade-view` reports, for each hero's strikes, the share of their active frames on which the
  blade can be seen from the chase camera at rest, by casting rays from it at points along the blade
  against all his other voxels, the clips posed as the game holds them.
- `VH_ANIM=bones` with any capture paints every voxel its bone's colour (a stranded voxel shows as a
  speck of the wrong colour); `import_rig.py` renders the same split in Blender.
- `VH_ANIM=code` with any capture, run or the capture suite gives the old animation:
  `tools/capture_suite.py` matches the build before the pilot exactly that way (§9).

---

## 9. Numbers (2026-10-05, the development laptop)

Before: the build before the pilot. After: the pilot's first pass (7 scripted clips), then the
Mixamo step (12 clips).

| What | Before | After |
|---|---|---|
| Release exe | 36,553,216 B | 37,304,320 B (+734 KB: 209 KB of clips, the rest the `gltf` and `serde_json` crates and the animator); with Mixamo's 12 clips 37,557,760 B (+1,004 KB in all) |
| Release wasm (the library, before `wasm-bindgen`) | 30,515,196 B | 31,141,990 B (+612 KB: the same 209 KB of clips, 403 KB of code); with Mixamo's 31,396,592 B (+881 KB in all) |
| Embedded clips | | 7 clips, 209 KB of GLB and JSON; with Mixamo's 12 clips, 455 KB |
| Reading Ragnar's clips (once, with the level; release build) | | 1.1 ms; 12 clips 2.3 ms |
| A frame of his animation (sample up to five clips, blend, warp, the juice, the hierarchy of 20 bones) | | 1.2 µs; with the walk and the parry 1.5 µs (`--anim-bench`, release) |
| Resident rig buffers (the jointed rig at every level of detail) | 22.2 MB | 33.1 MB; with the full-size shield 33.3 MB |
| `--perf` fight scenes (battle30, warband, pit, descent, siege, at 60 and 30 FPS) | all within 8 ms but one 16 ms spike in battle30 @60 | all within 8 ms (worst frame 6.7 ms); with Mixamo's, worst 5.7 ms |
| The blade seen from the chase camera on a strike's active frames | the scripted forehand 10%, backhand 100% | Mixamo's forehand and backhand 69%, finisher 66% |
| `--court`, `--pit 2 8 1`, the sixteen `--bout`s | | identical, line for line (but the wall-clock lines) |
| `tools/capture_suite.py` with `VH_ANIM=code` | | MATCH (within its GPU noise) |

**Mehmed and Miyamoto** (2026-10-07, `anim-heroes`; before: the build they branched from, where
both played by the code):

| What | Before | After |
|---|---|---|
| Release exe | | 38,673,408 B (their 24 clips, 994 KB of GLB and JSON: 36 clips, 1,427 KB embedded in all) |
| Their jointed rigs (`--export-rig`) | none | Mehmed 65,449 voxels, 32,728 faces; Miyamoto 68,599 voxels, 40,352 faces (Ragnar 46,233 and 29,440) |
| Reading a hero's clips (once, with the level; release) | | Ragnar 2.4 ms, Mehmed 2.1 ms, Miyamoto 2.0 ms |
| A frame of his animation (`--anim-bench`, release) | | Ragnar 1.8 µs, Mehmed 1.8 µs, Miyamoto 2.7 µs (the grip's solve and the sling) |
| Resident rig buffers of the level (`--perf` battle30) | | 59.6 MB with Ragnar, 82.0 with Mehmed, 85.1 with Miyamoto (their sculpts are bigger, and their jointed rigs are new) |
| `--perf` fight scenes (battle30, warband, pit, descent, siege, at 60 and 30 FPS; `VH_PERF_HERO`) | | all within 8 ms for each hero: worst frame Ragnar 5.4, Mehmed 5.8, Miyamoto 5.5 ms. (The pit scene's own checks, a wound taken and a pack slain, are Ragnar's: they fail for the other two with `VH_ANIM=code` as well.) |
| The blade seen from the chase camera on a strike's active frames (`--blade-view`) | | Mehmed forehand 66%, backhand 85%, finisher 100%; Miyamoto 78%, 84%, 80% (Ragnar's, measured the same way: 72%, 72%, 85%) |
| `--court`, `--pit 2 8 1`, the sixteen `--bout`s (Ragnar's harness) | | identical to the build before, line for line (but the wall-clock lines) |
| A bout in the ring played by the clips and by the code, step for step (`the_clips_leave_the_fight_untouched`) | Ragnar | identical for each of the three |

---

## 9a. Cloth: his cloak (`render/anim/cloth.rs`, 2026-10-06)

**2026-10-06, later: the cloak was taken off Ragnar** (the user's call), so no hero wears a flowing
cloak now (`cloth::spec` gives none); the cloth and its tests (on a test cloak of their own) stay
for the characters that will. In the same change the heroes are sculpted **solid**
(`Sculpt::finish_solid`: the enclosed voxels kept, so a joint that opens shows cloth and flesh, not
a hollow shell) and every joint across a limb has a **sleeve** (`skeleton::SLEEVE`: 2.5 voxels of
the bone above it laid on the bone below too, so a bend overlaps itself). Ragnar: 16,018 voxels
and 43,624 faces before, 46,233 voxels and 29,440 faces after (the hidden ones draw nothing).

What follows describes the cloth as it was built for his cloak.

His red cloak flows. It was carved onto his back as part of his body; now (longer, to his knees)
it is cut out of the jointed rig into a grid of 6 × 6 patches (`cloth::split`, by its colours in
every outfit, `heroes::RAGNAR_CLOAK`, in a box behind him), appended after the skeleton's bones as
parts `cape.N`. A small cloth runs on the grid's 7 × 7 points each frame, in the world, at 120 Hz:

- **Hung** from his chest: the top line follows the chest bone as the clip moves it.
- **Weight and air**: gravity; a little drag in every direction; and the air pushing on the
  cloak's *face* only (`FACE_DRAG`), so it slips through edge-on and trails at a slant (about 33
  degrees behind a sprint) rather than flying flat; a slow breath of wind so it is never still.
- **Inertia** (`INERTIA`): it feels 40% of his travel and turning; a sprint starts at full speed
  in one frame, and the whole jolt flipped it over his head.
- **Kept off him**: every point stays behind his back as far as it hung at rest (measured level,
  not along his leaning back, which pushed it up onto his shoulders), out of his thighs, and never
  lifts past about 60 degrees.
- **Lengths**: along and across the grid, its diagonals, and a little stiffness over two patches
  so it folds rather than crumples; five passes a step.
- Each patch's voxels ride on its four corners (a frame across, up and through them, stretched a
  little with them), so the cloak stays voxels.

A jump of over 1.5 tiles in a frame (a portal) lays it back at rest about him. It is presentation
only and runs on the frame's own time; the code path (`VH_ANIM=code`) keeps the cloak carved on
his back, and `--export-rig` exports the rig before the cut (the cloak on his back as sculpted).
Tests: the cut keeps every cloak voxel (in every outfit), the patches sit where the cloak was at
rest, it never passes through his back while he sprints and settles when he stops, a portal lays
it back at rest.

## 10. What is left

- **Cloth to come**: the tunic's skirt flaps, his beard braids and belt tassels on short chains;
  the same cloth for Mehmed's kaftan and Miyamoto's sleeves, Krampus's fur and chains, the princes'
  robes, the old man's coat.

- **What still reads wrong** (2026-10-05, Mixamo's clips):
  - A blow still hides the blade for about a third of its active frames from behind: Mixamo's cuts
    pass before his chest, and his helmet, horns and mantle are wide. The swing's white arc covers
    the same place, so in play it reads; a cut keyed for his build (or Mixamo's raised higher
    still) would read better.
  - The finisher keeps Mixamo's whole spin: the blade is away from the camera for part of it.
  - His stance is side-on (Mixamo's): from behind the camera sees his shield side more than his
    back. Running and leaping he faces forward; the crossfades turn him between the two.
  - The evade's roll is a dive and tuck in the air (the leap lifts him): it reads as a flip.
  - The parry is built (Mixamo has none); a parry keyed by hand would be sharper.
  - Strikes start from the clip's first frame with no blend (the rule): a strike begun from the
    run snaps from the run's pose to the stance.
- **What still reads wrong for Mehmed and Miyamoto** (2026-10-07):
  - Miyamoto's arms are too short for his shoulders to hold a grip where Mixamo holds it: the grip
    is drawn in before his chest and his shoulders closed in (`anim::hold`), so his cuts travel less
    far out to his sides than the source's, and with the sode on his upper arms his shoulders look
    hunched in the guard. Longer arms in his sculpt would cure it at the root.
  - The tate slung on his back is big: from the chase camera it hides his back and most of his
    arms (slung lower than it was carved, his sword shoulder and cuts show over it).
  - The clips in Blender (and `preview.py`'s sheets) do not show Miyamoto's grip and slung shield:
    they are the game's; Blender's `--measure` is only a guide for him.
  - Mehmed's börk is tall: what passes over his head is hidden from behind (his forehand is the
    least seen of all the strikes, 66%).
  - The pack's only held guard (Block Idle) is Ragnar's too; Mehmed's stance and carriage set it
    apart, not the motion. The roll is the same clip for all three.
  - Mehmed's low sweep (his backhand) drops him into a deep lunge for a moment: it reads as reach,
    and it is his longest, but it is low for a soldier's carriage.
- **More moments**: a death (the game does not show his body after he falls), a sidestep (the
  dodge clip is ready in the pack), turning on the spot.
- **The Hall of Mirrors' reflections** and the select screen still pose him by the code.
- **A lighter reader for the web**: the `gltf` crate and `serde_json` are about 400 KB of the wasm; a
  reader of only what the clips use (nodes, one animation, float accessors) would be a tenth of
  that, if the web build's size comes to matter.
- **Foes and the cast**: the same pipeline; each needs its joint table and clips (their rigs keep no
  parents until then).
