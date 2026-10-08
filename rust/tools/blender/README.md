# The Blender side of the animation pipeline

Python scripts for Blender 4.5, run headless (`blender -b [file.blend] -P script.py -- args`). They
need no add-ons and change no preferences. `vh_blender.py` holds what they share (the skeleton's
bones, the axes, keying, rendering) and `posing.py` the posing helpers that `block_ragnar.py` keys
with. The whole pipeline, its decisions and its vocabulary are in `rust/ANIMATION.md`; this is
the reference for each script.

On this machine Blender is `C:\Program Files\Blender Foundation\Blender 4.5\blender.exe`; below it
is `blender`, and the paths are from `rust/`.

## import_rig.py: the rig into Blender, checked

```
AdventuresOfTheAncientBlades --export-rig ragnar ragnar_rig.glb
blender -b -P tools/blender/import_rig.py -- ragnar_rig.glb OUT_DIR [ragnar.blend]
```

Imports the game's jointed rig (glTF: armature, rest pose, a mesh per bone bound wholly to it,
vertex colours) and checks it: all twenty bones with the skeleton's parents, every voxel mesh bound
to the armature, each vertex weighted 1 to its own bone and nothing else, colours present. Renders
`rig_colours.png` (his own colours), `rig_bones.png` and `rig_bones_back.png` (each bone's voxels
in that bone's colour, as `VH_ANIM=bones` paints them in the game) with Workbench. Saves the
.blend if asked. Exits 1 if a check fails.

## block_ragnar.py: his first clips, keyed by script

```
blender -b -P tools/blender/block_ragnar.py -- ragnar_rig.glb ragnar.blend
```

Imports the rig and keys seven actions on it: `idle` (2 s, looping: a breath, a shift of weight,
a glance), `run` (26 frames a cycle, looping, stride 2.2 m), `slash_fore` and `slash_back` (40
frames each as keyed: the guard, the wind-up, the strike, the follow-through, the settle), `block`
(raised in 6 frames, held a second that loops), `jump` (25 frames in the air and 12 of landing)
and `hit` (a flinch of the upper body, 24 frames). Markers go on each action as pose markers; loops
and strides as the custom properties `vh_loop` and `vh_stride`. Then saves the .blend: open it in
Blender to change the clips by hand, or edit the script and run it again.

The poses are built with `posing.py`: the torso by angles, the legs by where each ankle goes (a
two-bone IK, the knee forward, the foot flat unless told), the arms and the weapon by the way they
point. Angles and directions are in the game's axes (X his left, Y up, Z forward).

## preview.py: contact sheets

```
blender -b ragnar.blend -P tools/blender/preview.py -- OUT_DIR [action ...] [--yaw DEG] [--rise R] [--frames N]
    [--at HEIGHT] [--dist D]
```

Renders each action (all, or those named) as rows of frames, its markers among them, to
`OUT_DIR/<action>.png`: a quick look without building the game. `--yaw` turns the camera round
from his front (180 is behind him, as the game's camera mostly is), `--rise` lifts it, `--at` sets
the height it looks at and `--dist` its distance (Mehmed and Miyamoto are taller: `--at 0.62 --dist
2.7`). It shows the clips as Blender has them: Miyamoto's off hand and slung shield are the game's
(`anim::hold`), so look at him in the game (`anim_compare.py --hero 2`).

## export_clips.py: the clips out to the game

```
blender -b ragnar.blend -P tools/blender/export_clips.py -- assets/anim/ragnar [action ...]
```

Writes each action (all, or those named) as `<action>.glb`: the armature alone (no meshes), the
action sampled every frame at 60 fps, slid to start at 0. Beside it `<action>.json`: its frame
count, whether it loops, a gait's stride, and its markers in frames from its start (the action's
pose markers, or failing those the scene's timeline markers within its range). The game embeds
every pair under `assets/anim/<hero>/` at the next build.

## retarget.py: outside motion onto his skeleton

```
blender -b ragnar.blend -P tools/blender/retarget.py -- --source run.fbx --name run_mixamo
    [--loop] [--stride 2.2] [--marker foot_l=0 ...] [--keep-travel] [--save OUT.blend]
blender -b -P tools/blender/retarget.py -- --selftest ragnar_rig.glb [OUT_DIR]
```

Reads a Mixamo FBX or a BVH (any humanoid: motion capture, video- or text-to-motion output) and
bakes it onto his armature as a new action at 60 fps. The bone map is chosen by names (Mixamo's
`mixamorig:` names first, the SMPL body model's 22 joints second, the usual BVH names third, with
their aliases); his weapons and shield keep his hold. Each bone's direction at rest is turned onto
the source bone's (a T-pose's level arms onto his hanging ones) about the way both face, then the
source's turns away from its rest are laid on in the world, so chains of other lengths come out
right. The hips' movement is scaled by his leg over the source's, his travel over the ground is
taken out (the drift from first frame to last; the sway and the bob stay; `--keep-travel` keeps it).
Saves the .blend (the one opened, unless `--save`). Add markers with `--marker`, then
`export_clips.py`.

The options for his build (Mixamo's bot is tall and thin, he is stocky): `--face DEG` sets the way
his hips face at the first frame (or `--turn DEG` turns the whole clip), `--yaw-keep K` keeps only a
share of the source's turning about the upright (a spin becomes a cut struck where he stands),
`--arms-out DEG` carries his arms out from his body, `--sword-turn YAW,PITCH` swings the sword arm
round his shoulder (toward his right, then up), `--hold DEG` sets the blade in his fist. From a
script, `Retarget(posture=...)` straightens his torso (degrees back from the source's lean for the
spine, chest, neck and head, each carrying those above it; his arms keep the source's aim), and
`bake(still=True)` takes out all his movement over the ground (a leaping blow struck where he
stands), the rise and fall kept. An FBX's
frame rate is read from the file (Mixamo's sword and shield pack is at 30 fps, whatever was asked
for); a loop's last 8 frames are eased onto its first.

`--selftest` builds a Mixamo-named armature in a T-pose, animates it at 30 fps (arms down and
bent, a leg lifted, the spine turned, the hips carried a metre forward and up), retargets it and
checks every mapped limb points the way the source's does on every sixth frame (within 4 degrees),
that his hips stay over his feet and rise by the source's rise times the leg scale. It prints PASS
or FAIL (exit 1) and renders `retarget_selftest.png` (him and the source's bones, last frame).

## mixamo_hero.py: each hero's clips from Mixamo

```
blender -b HERO.blend -P tools/blender/mixamo_hero.py -- HERO C:/Users/ulask/Games/mixamo OUT.blend [slot ...]
blender -b HERO.blend -P tools/blender/mixamo_hero.py -- HERO C:/Users/ulask/Games/mixamo OUT.blend --try FILE[:OPTS] ...
blender -b HERO.blend -P tools/blender/mixamo_hero.py -- HERO --measure [action ...]
```

HERO is `ragnar`, `mehmed` or `miyamoto`; HERO.blend is his rig as `import_rig.py` saves it. The
picks (a table for each hero at its top: slot, file, corrections) are retargeted from Mixamo's packs
(`sword-shield/` for Ragnar and Mehmed, `great-sword/` for Miyamoto, and the two extras) into his
.blend, each as the action the game knows by that name: `idle`, `walk`, `run`, `slash_fore`,
`slash_back`, `slash_heavy`, `block`, `block_idle`, `jump`, `roll`, `hit`, and `parry` built from
the block and the flinch. Their markers are found from the motion (a strike's from the blade tip's
speed, a leap's from the feet, the guard's from the shield or, where it is slung, the off hand, the
flinch's from the chest) and the lead-in before a strike's blade stirs is cut away. A pick can keep
only some seconds of its source (`span`), play it backward (`reverse`), straighten him (`posture`)
or keep him where he stands (`still`). The FBX files stay outside the repository; then
`export_clips.py` the .blend. Why each pick is in `rust/ANIMATION.md` §6.

`--try` retargets candidates instead (each an action named after its file, with a pick's options
after a colon as Python keywords: `"great-sword/great sword slash.fbx:kind='strike',face=-15"`), to
look at with `preview.py` before choosing. `--measure` reports a strike's blade as the chase camera
sees it on its active frames, by rays against his voxels in Blender; the game's own
`--blade-view` does the same with the clips as the game holds them (Miyamoto's fists on the grip,
his shield slung), and is the one to trust.

## The foes (ANIMATION.md §11)

The scripts above take any skeleton: a foe's rig (`--export-rig draugr`, `bat`, `cobra`, ... the
names of `anim::foe_rigs::FoeRig::dir`) imports and exports as a hero's does (`import_rig.py` checks a
humanoid's bones against the heroes' and any rig's binding; `export_clips.py` takes the actions of
the armature's own root bone). `preview.py` wants `--at` and `--dist` for the foe's size (a raven:
`--at 0.15 --dist 1.5 --rise 0.5`; the rasetsu: `--at 0.75 --dist 3.2`).

### block_foe.py: the foes Mixamo cannot drive, keyed by script

```
AdventuresOfTheAncientBlades --export-rig cobra cobra_rig.glb
blender -b -P tools/blender/block_foe.py -- cobra cobra_rig.glb cobra.blend
blender -b cobra.blend -P tools/blender/export_clips.py -- assets/anim/cobra
```

FOE is `bat`, `vampire`, `large_bat`, `werewolf`, `dire_wolf`, `cobra`, `slime`, `minion` or
`jorogumo`. Every clip is a function of the frame (a class per kind of body: `Flyer`, `Wolf`, `Cobra`,
`Slime`, `Minion`, `Jorogumo`, their numbers per foe in `FOES`), baked on every frame at 60 fps:
the stand and the gait (with their strides), the moves at their frame data's base tempo with their
markers, the rally, and the reels. Angles in degrees about the game's axes, relative to the parent;
`Pose.scale` keys a bone's scale (the slime's squash, the cobra's hood).

### mixamo_foe.py: the humanoid foes from Mixamo

```
blender -b draugr.blend -P tools/blender/mixamo_foe.py -- draugr C:/Users/ulask/Games/mixamo draugr_m.blend [slot ...]
blender -b draugr.blend -P tools/blender/mixamo_foe.py -- draugr C:/Users/ulask/Games/mixamo try.blend --try FILE[:OPTS] ...
```

As `mixamo_hero.py` (its retarget, markers and trimming), a table per foe (`DRAUGR`, `HELMETED`,
`WITCH`, `BLEMMYES`, `TENGU`, `RASETSU`, `JOROGUMO`). A foe's weapon is in its own left fist (its
sculpt is not mirrored), so its picks are mirrored (`mirror`), the weapon the `offhand`, held as the
source's right fist holds its grip, the way it points at rest given (`weapon_rest`, or one per
weapon for a pair). `weapon` and `tip_rest` name the bone whose tip gives a move's markers (the
head for a roar). `shorten` cuts a long settle against the move's recovery, a reel's against its
impact. The jorogumo's keyed legs (`block_foe.Jorogumo`) are laid under each of her clips
(`lay_legs`). `deflected` is built from the stagger.

The retarget (`retarget.py`) now also takes an armature with only some of the heroes' bones (her
legs are her own: no hips' travel then), a weapon in each fist each its own way (`weapon_rest` as a
dict), and mirrors a foe's grip and sword arm; `mixamo_hero.make` takes `stride="feet"` (the stride
measured from the feet) and `extra_marks`.

### The later stages, the realms, the courts and the lair

The same two scripts take every foe since (`FoeRig::dir` names them all: `kappa` ... `hel`, `ganglot`,
`gaki`, `ellepige`, `mammon`, `gold_golem`, `lucifer`, `reflection`, `leviathan`, `serpentling`,
`amon`, `raven`, `belphegor`, `nightmare`, `beelzebub`, `fly_swarm`, `asmodeus`, `krampus`, `percht`).
`block_foe.py` finds the keyed ones in its own `FOES` and in the modules after it: `block_stage4.py`
to `block_stage7.py`, `block_courts.py` (the ravens; and the keyed bones laid under princes' clips),
`block_envy.py` (Leviathan, the serpentlings), `block_feast.py` (the fly swarm; the charges and
Beelzebub's belly roll, baked onto a prince's Mixamo .blend), `block_realms.py` (the ganglöt's staff
arm).

```
blender -b mammon.blend -P tools/blender/mixamo_foe.py -- mammon C:/Users/ulask/Games/mixamo mammon_m.blend
blender -b -P tools/blender/block_foe.py -- amon amon_rig.glb amon_m.blend --onto amon_m.blend
blender -b mammon_m.blend -P tools/blender/mixamo_foe.py -- mammon C:/Users/ulask/Games/mixamo mammon_m.blend coin_rain
```

The second bakes a foe's keyed clips onto its Mixamo .blend (Amon's fiery charge over his Mixamo
moves); the third retargets only the slots named, into the .blend it opens (the others kept).

A table's options beyond the heroes': `slow` (the source played that much longer: a giant's
weight), `lay=(module, class, method, bones, frames, loop)` (bones keyed by script laid under the
clip: a staff or a sceptre held upright as sculpted where the source's arm swings empty, a tail, a
jaw, wings; a loop gets whole cycles, a move fitted to its markers by `RECOVERY_FD`), `face` and
`yaw_keep` (the source's turning taken out), `mirror=False` per slot (a throw from the prince's own
right). A prince's `roar` plays as each of his phases begins.

`export_clips.py` thins each clip as it writes it (`thin_clips.py`, also standalone over written
clips): a track keeps only the keys a straight line between its neighbours does not give back.

## The story's cast (ANIMATION.md §12)

`--export-rig kid0` (to `kid3`), `oldman`, `dog`, `krampus_shade` writes the cast's rigs; `import_rig.py`
takes them into Blender as any rig.

### mixamo_cast.py: the boys' and the old man's clips from Mixamo

```
blender -b kid0.blend -P tools/blender/mixamo_cast.py -- kid C:/Users/ulask/Games/mixamo kid.blend [slot ...]
blender -b oldman.blend -P tools/blender/mixamo_cast.py -- oldman C:/Users/ulask/Games/mixamo om.blend [slot ...]
```

As `mixamo_hero.py` (its retarget, markers and trimming), a table for the boys (one set for the four:
their bodies are one sculpt) and one for the old man, from the cast set in `cast/` (and the axe pack's
plain walk). A kick's contact (`hit`) is found from the kicking foot (`kind="strike"`,
`weapon="foot.R"`), a landing's from the feet (`kind="leap"`). `--try` as the others.

### block_cast.py: what Mixamo has none of, keyed by script

```
blender -b -P tools/blender/block_cast.py -- dog dog_rig.glb dog.blend
blender -b -P tools/blender/block_cast.py -- kid kid0_rig.glb kid.blend --onto kid.blend
blender -b -P tools/blender/block_cast.py -- krampus_shade krampus_shade_rig.glb kr.blend
```

Quitmer's clips (on the wolves' `Wolf`: asleep, lying and watching, his tail going, rising, straining
and snapping at his chain, lying down), Scene II's hug, and Scene I's Krampus's grab and his arm held
up with his catch (the rest of his are the lair's). `--onto` adds them to a .blend that has the Mixamo
clips already.
