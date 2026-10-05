# Animation by clips: the pipeline, piloted on Ragnar

How the heroes (and later the cast and the foes) are animated by clips made in Blender, from
motion of any source (keyed by hand or by script, Mixamo, video-to-motion, text-to-motion, paid
services later), instead of only by curves written in code. The pilot is Ragnar: his skeleton, his
first seven clips keyed by script, the whole chain from Blender to the game, and the old animation
beside the new for comparison, so the user can judge it before paying for any tool.

Status: the pilot is built on `anim-ragnar` (2026-10-05). Ragnar plays by his clips by default;
`VH_ANIM=code` gives the old animation back. Mehmed and Miyamoto play as before until their joint
tables (§1.3) and clips exist. What is left is in §10.

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
| `blade` | hand.R | the weapon of his armory in his right fist (its joint at the grip) |
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

Mehmed and Miyamoto follow with tables of their own in `skeleton::joints` (their grids are
`heroes::MEHMED_SIZE` and `MIYAMOTO_SIZE`; their parts are tagged as Ragnar's are). Until then they
have no jointed rig and play by the code. The tests check the cut keeps every voxel exactly once
and in its place, that none lies below the next joint down its limb or beyond its bone's reach,
and that the left limbs stay on his left; `VH_ANIM=bones` paints each bone its own colour in the
game, and `import_rig.py` renders the same in Blender (§8).

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
3. **The clips.** Keyed by hand in Blender, by script (`block_ragnar.py`, §3), or retargeted from
   outside motion (`retarget.py`, §6): each an action on his armature, with its markers as pose
   markers on the action, `vh_loop` and `vh_stride` as custom properties.
   `blender -b -P tools/blender/block_ragnar.py -- ragnar_rig.glb ragnar.blend`
4. **A look before the game.**
   `blender -b ragnar.blend -P tools/blender/preview.py -- OUT [action ...] [--yaw 180 --rise 0.7]`
   renders each action as contact sheets.
5. **Out to the game.**
   `blender -b ragnar.blend -P tools/blender/export_clips.py -- assets/anim/ragnar` writes
   `<action>.glb` (the armature alone, sampled at 60 fps) and `<action>.json` (markers, loop,
   stride) for each action.
6. **Build.** `build.rs` lists every `assets/anim/<hero>/<clip>.glb` that has its `.json` beside it;
   `render/anim/library.rs` embeds them. Nothing else to register.
7. **Compare.** `python tools/anim_compare.py OUT` (§8); `--anim-bench` for the numbers (§9).

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
  - **On his feet**: `idle` and `run` blended by his stride (`motion`); the run's cycle advances by
    his speed over the ground over its `stride`.
  - **The guard**: `block` over his upper body, its raise (to `raised`) fitted into the guard's
    raise frames (`fsm::GUARD_RAISE`), then its hold looping; in over 3 frames, out over 8.
  - **A blow taken**: `hit` over his upper body, through the reel's frames (hitstun, a broken
    guard). Blockstun shows on the shield (the jolt).
  - **A strike**: `slash_fore` for the swing that coils to his right first (the view's side -1, as
    the code's), `slash_back` for the other; the swings alternate as they always have. Its time is
    the fighter's frame where frame data rules him (the Sparring Ring, the realms), or the view's
    own clock with the old swing's proportions elsewhere.
  - **The leap**: `jump` from `takeoff` to `land` by the leap's progress, then its landing to
    `recover_end` in real time (cut short by a strike, the guard or a run).
  - **The juice**: the upper sword arm carried on past the strike's end and back
    (`BLADE_OVERSHOOT`'s spring, about his upright) unless the springs are off; the shield forearm
    knocked back by `shield_tilt`; the landing's squash on the root.
- **`library.rs`**: the embedded clips, read once per hero (`clips(hero)`, also called from
  `prepare_rigs` so it happens behind the loading screen), and `Mode` (`VH_ANIM`).
- **`export.rs`** (native only): `--export-rig` and `--anim-bench`.

In the view: `forge::jointed_of` cuts the hero's rig each time his kit changes (and the spare for a
broken shield with it); `prepare_rigs` meshes it at every level of detail with the level;
`animation::animate_clips` runs after the code's `animate_hero` (which always runs: it stays the
fallback, and the Hall of Mirrors' reflections still use it) and leaves the parts' matrices in `hero_mats`; `emit` draws the jointed rig with
`RigList::push_matrices` when they are there (and his haste's afterimages with them), the code's
rig otherwise. The hierarchy itself is in
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
| `raised` | `block` | The shield up: the guard's raise ends here; from here to the end loops while he holds it. |
| `impact` | `hit` | The blow's moment in the flinch. |
| `foot_l`, `foot_r` | a gait | Each heel strike (for footfalls and phase). |
| `loop` (the sidecar's `"loop": true`) | a cycle | It wraps; its last frame is its first again. |
| `stride` (the sidecar's `"stride"`) | a gait | Metres (tiles) a cycle carries him: the rate is his speed over it. |

A strike without markers is read as 0.3, 0.45 and 1 of its length; a leap as 0, 0.7 and 1.

---

## 5. Adding a clip

1. Make it an action on the hero's armature in his .blend (keyed, scripted or retargeted). Name
   it as the game knows it: `idle`, `run`, `slash_fore`, `slash_back`, `block`, `jump`, `hit` replace
   his own; a new name is embedded but nothing plays it until `anim::hero` asks for it.
2. Add its markers (pose markers in the action editor, §4), and for a cycle `vh_loop` (and for a
   gait `vh_stride`) as custom properties of the action.
3. `export_clips.py` it into `assets/anim/<hero>/`, build, and compare (§8). The tests check every
   clip of Ragnar reads, has its markers in order and rests on the joints the game cut him at.

A new kind of moment (a dodge apart from the leap, a heavy finisher, a death) is a clip and a few
lines in `Animator::step` choosing it from what `Drive` says.

---

## 6. Outside motion

`retarget.py` (see `tools/blender/README.md`): Mixamo FBX, or BVH from any tool, onto his armature,
baked at 60 fps, in place, scaled by leg length, his weapons in his hold. It is tested on a
synthetic Mixamo-named armature (no Mixamo files are here yet): every mapped limb follows the
source's direction to within a hundredth of a degree, the hips stay over his feet and rise by the
source's rise times the leg scale (0.33); a BVH read lying down is stood up first.

**What to download from Mixamo** (mixamo.com, free with an Adobe account; the "Sword and Shield
Pack" holds most of them; numbering of the variants may differ: search "sword and shield"):

| For | Mixamo clips |
|---|---|
| `idle` | Sword And Shield Idle (and its variants (2) to (4) for a choice) |
| `run` | Sword And Shield Run (with **In Place** ticked), Sword And Shield Walk for a later walk |
| `slash_fore`, `slash_back` | Sword And Shield Slash and Slash (2) to (5); Sword And Shield Attack (1) to (4) for heavier blows |
| `block` | Sword And Shield Block, Sword And Shield Block Idle (the hold) |
| `jump` | Sword And Shield Jump (and Jump (2)) |
| a dodge | Standing Dodge Backward, Standing Dodge Left, Standing Dodge Right |
| `hit` | Sword And Shield Impact, Impact (2), Impact (3); Sword And Shield Death for a later death |

Download each as **FBX Binary, Without Skin, 30 fps, no keyframe reduction**, on the default
character (Y Bot), locomotion **In Place**. Then, per clip:
`blender -b ragnar.blend -P tools/blender/retarget.py -- --source Sword_And_Shield_Run.fbx --name run --loop --stride 2.2 --marker foot_l=0 --marker foot_r=13`,
check its markers in Blender (a strike needs `windup_end`, `active_end`, `recover_end`), preview,
export. Mixamo's skeletons are human-sized and their swings human-paced: expect to retime (the warp
does most of it for strikes) and to raise his arms a little where his wide body hides them.

**Video and text to motion** (BVH out): the same command with `--source take.bvh`. The SMPL names
(`pelvis`, `left_hip`, ... `right_wrist`) and the usual BVH names are both mapped. Clean such
takes in Blender before export: foot sliding and jitter are common in them.

---

## 7. Licences

| Source | May its motion ship in the game? |
|---|---|
| Mixamo | Yes: Adobe allows Mixamo's characters and animations in commercial games (not the raw files redistributed on their own). |
| Clips keyed by hand or by script here | Yes: ours. |
| MoMask (text to motion) | **Reference only.** Its models are trained on research-licensed data (HumanML3D, from AMASS): look at its output, key our own. |
| GVHMR (video to motion) | **Reference only.** It outputs SMPL, whose body model's licence is non-commercial. |
| HY-Motion (Tencent) | **Never.** Its licence excludes the EU, and the user is in Denmark. |
| Paid services (Move.ai, Plask, DeepMotion, Rokoko, Cascadeur...) | Check each one's terms for commercial game use before any of its output goes into `assets/`. |

---

## 8. Comparisons

- `python tools/anim_compare.py OUT [EXE] [moment ...]` renders twenty moments in the Sparring Ring
  (idle and a breath; three moments of a run, and one from behind; the first and second swings at
  wind-up, blow, through and settle, and one from behind; the guard rising and held; the leap
  rising, at its top, falling and landing), each as one picture: **the code's animation on the
  left, the clips' on the right**, the same frame of the same deterministic run
  (`--capture ... --anim-compare`: the frame is drawn twice, once each way).
- `VH_ANIM=bones` with any capture paints every voxel its bone's colour (a stranded voxel shows as a
  speck of the wrong colour); `import_rig.py` renders the same split in Blender.
- `VH_ANIM=code` with any capture, run or the capture suite gives the old animation:
  `tools/capture_suite.py` matches the build before the pilot exactly that way (§9).

---

## 9. Numbers (2026-10-05, the development laptop)

| What | Before | After |
|---|---|---|
| Release exe | 36,553,216 B | 37,304,320 B (+734 KB: 209 KB of clips, the rest the `gltf` and `serde_json` crates and the animator) |
| Release wasm (the library, before `wasm-bindgen`) | 30,515,196 B | 31,141,990 B (+612 KB: the same 209 KB of clips, 403 KB of code) |
| Embedded clips | | 7 clips, 209 KB of GLB and JSON |
| Reading Ragnar's clips (once, with the level; release build) | | 1.1 ms |
| A frame of his animation (sample up to four clips, blend, warp, the juice, the hierarchy of 20 bones) | | 1.2 µs (`--anim-bench`, release) |
| Resident rig buffers (the jointed rig at every level of detail) | 22.2 MB | 33.1 MB |
| `--perf` fight scenes (battle30, warband, pit, descent, siege, at 60 and 30 FPS) | all within 8 ms but one 16 ms spike in battle30 @60 | all within 8 ms (worst frame 6.7 ms) |
| `--court`, `--pit 2 8 1`, the sixteen `--bout`s | | identical, line for line (but the wall-clock lines) |
| `tools/capture_suite.py` with `VH_ANIM=code` | | MATCH (within its GPU noise) |

---

## 10. What is left

- **The clips by hand.** The scripted clips show the chain; an animator's pass in Blender (or
  Mixamo's sword-and-shield set retargeted, §6) is the next step up in quality. The swings in
  particular should be judged from the game's camera, behind him: his wide body hides a blade in
  front of his chest.
- **Mehmed and Miyamoto**: their joint tables (`skeleton::joints`), then their clips.
- **More moments**: a dodge apart from the leap, L3 as a heavier finisher of its own, a death, the
  walk under the guard, turning on the spot.
- **The Hall of Mirrors' reflections** and the select screen still pose him by the code.
- **A lighter reader for the web**: the `gltf` crate and `serde_json` are about 400 KB of the wasm; a
  reader of only what the clips use (nodes, one animation, float accessors) would be a tenth of
  that, if the web build's size comes to matter.
- **Foes and the cast**: the same pipeline; each needs its joint table and clips (their rigs keep no
  parents until then).
