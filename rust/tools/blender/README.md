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
```

Renders each action (all, or those named) as rows of frames, its markers among them, to
`OUT_DIR/<action>.png`: a quick look without building the game. `--yaw` turns the camera round
from his front (180 is behind him, as the game's camera mostly is), `--rise` lifts it.

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
round his shoulder (toward his right, then up), `--hold DEG` sets the blade in his fist. An FBX's
frame rate is read from the file (Mixamo's sword and shield pack is at 30 fps, whatever was asked
for); a loop's last 8 frames are eased onto its first.

`--selftest` builds a Mixamo-named armature in a T-pose, animates it at 30 fps (arms down and
bent, a leg lifted, the spine turned, the hips carried a metre forward and up), retargets it and
checks every mapped limb points the way the source's does on every sixth frame (within 4 degrees),
that his hips stay over his feet and rise by the source's rise times the leg scale. It prints PASS
or FAIL (exit 1) and renders `retarget_selftest.png` (him and the source's bones, last frame).

## mixamo_ragnar.py: his clips from Mixamo

```
blender -b ragnar.blend -P tools/blender/mixamo_ragnar.py -- C:/Users/ulask/Games/mixamo ragnar.blend [slot ...]
```

The picks (a table at its top: slot, file, corrections) retargeted from Mixamo's Pro Sword And
Shield Pack and two extras into his .blend, each as the action the game knows by that name
(replacing the scripted one): `idle`, `walk`, `run`, `slash_fore`, `slash_back` (the forehand run
backward), `slash_heavy`, `block`, `block_idle`, `jump`, `roll`, `hit`, and `parry` built from the
block and the flinch. Their markers are found from the motion (a strike's from the blade tip's
speed, a leap's from the feet, the guard's from the shield, the flinch's from the chest) and the
lead-in before a strike's blade stirs is cut away. The FBX files stay outside the repository; then
`export_clips.py` the .blend. Why each pick is in `rust/ANIMATION.md` §6.
