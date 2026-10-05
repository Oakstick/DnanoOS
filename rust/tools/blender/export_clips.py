"""Writes every action of a .blend with Ragnar's armature as a game clip: `<action>.glb` (the
armature alone, no meshes, the action sampled at 60 fps) and `<action>.json` beside it (its
markers, from the action's pose markers, or failing those the scene's timeline markers within its
frame range; whether it loops; a gait's stride).

    blender -b ragnar.blend -P tools/blender/export_clips.py -- assets/anim/ragnar [action ...]

Custom properties on an action: `vh_loop` (true for a cycle) and `vh_stride` (metres a cycle of
a gait carries him). Marker frames are counted from the action's first frame.
"""
import json
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import vh_blender as vh  # noqa: E402


def markers(act, scene):
    f0, f1 = act.frame_range
    marks = {m.name: m.frame for m in act.pose_markers}
    if not marks:
        marks = {m.name: m.frame for m in scene.timeline_markers if f0 <= m.frame <= f1}
    return {k: int(round(f - f0)) for k, f in sorted(marks.items(), key=lambda kv: kv[1])}


def main():
    a = vh.args()
    if not a:
        sys.exit(__doc__)
    out, only = a[0], set(a[1:])
    os.makedirs(out, exist_ok=True)
    scene = bpy.context.scene
    scene.render.fps, scene.render.fps_base = vh.FPS, 1.0
    arm = vh.armature()
    # The armature alone goes out: select it and nothing else.
    for o in bpy.data.objects:
        o.select_set(o == arm)
    bpy.context.view_layer.objects.active = arm
    for act in list(bpy.data.actions):
        if only and act.name not in only:
            continue
        # (Only actions of his bones: an imported source's own action, left in the file, is not his.)
        if not any('pose.bones["hips"]' in fc.data_path for fc in vh.fcurves(act)):
            continue
        arm.animation_data.action = act
        f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
        scene.frame_start, scene.frame_end = f0, f1
        scene.frame_set(f0)
        path = os.path.abspath(os.path.join(out, act.name + ".glb"))
        bpy.ops.export_scene.gltf(
            filepath=path,
            export_format="GLB",
            use_selection=True,
            export_animations=True,
            export_animation_mode="ACTIVE_ACTIONS",
            export_frame_range=True,
            export_force_sampling=True,
            export_frame_step=1,
            export_anim_slide_to_zero=True,
            export_optimize_animation_size=True,
            export_skins=False,
            export_def_bones=False,
            export_leaf_bone=False,
            export_reset_pose_bones=True,
            export_current_frame=False,
            export_rest_position_armature=True,
            export_yup=True,
            export_materials="NONE",
            export_extras=False,
        )
        info = {
            "clip": act.name,
            "fps": vh.FPS,
            "frames": f1 - f0,
            "loop": bool(act.get("vh_loop", False)),
            "markers": markers(act, scene),
        }
        if "vh_stride" in act:
            info["stride"] = float(act["vh_stride"])
        with open(os.path.join(out, act.name + ".json"), "w") as f:
            json.dump(info, f, indent=1)
            f.write("\n")
        print(f"export_clips: {act.name}: {f1 - f0} frames, {os.path.getsize(path) // 1024} KB, markers {info['markers']}")


main()
