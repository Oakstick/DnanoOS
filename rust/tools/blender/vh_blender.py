"""Shared helpers for the Blender side of the animation pipeline (ANIMATION.md).

Run inside Blender (4.5, headless: `blender -b -P script.py -- args`); every script here puts this
folder on `sys.path` and imports what it needs from this module.

Axes. The game and glTF agree: Y up, the hero faces +Z, +X is his own left, a metre is a tile.
Blender's glTF import turns that into Z up (he faces -Y, his left at +X): a game vector (x, y, z) is
Blender's (x, -z, y). Ragnar's bones all rest unturned in the game, so in Blender each bone's own
axes are the game's (X his left, Y up, Z forward) and a pose bone's rotation is a turn in the game's
axes relative to its parent: the same numbers the engine reads. The helpers below still go through
each bone's rest matrix, so an armature imported another way keys the same.
"""
import math
import os
import sys

import bpy
from mathutils import Matrix, Quaternion, Vector

FPS = 60

# The skeleton, parents first (render/anim/skeleton.rs `BONES`, `PARENTS`).
BONES = [
    "hips", "spine", "chest", "neck", "head",
    "upper_arm.L", "forearm.L", "hand.L", "upper_arm.R", "forearm.R", "hand.R",
    "thigh.L", "shin.L", "foot.L", "thigh.R", "shin.R", "foot.R",
    "blade", "offhand", "shield",
]
PARENTS = {
    "spine": "hips", "chest": "spine", "neck": "chest", "head": "neck",
    "upper_arm.L": "chest", "forearm.L": "upper_arm.L", "hand.L": "forearm.L",
    "upper_arm.R": "chest", "forearm.R": "upper_arm.R", "hand.R": "forearm.R",
    "thigh.L": "hips", "shin.L": "thigh.L", "foot.L": "shin.L",
    "thigh.R": "hips", "shin.R": "thigh.R", "foot.R": "shin.R",
    "blade": "hand.R", "offhand": "hand.L", "shield": "forearm.L",
}

# The way a held weapon points out of his fist at rest, in the game's axes (the bone's own): up, and
# leaning forward 0.42 voxels a voxel (`armory::LEAN`: held at the ready, clear of his arm).
WEAPON_REST = Vector((0.0, 1.0, 0.42)).normalized()

# Game axes to Blender's, as a rotation.
G2B = Matrix(((1, 0, 0), (0, 0, -1), (0, 1, 0)))


def args():
    """The script's own arguments (after `--`)."""
    return sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []


def g2b(v):
    """A game vector in Blender's axes."""
    return G2B @ Vector(v)


def b2g(v):
    """A Blender vector in the game's axes."""
    return G2B.transposed() @ Vector(v)


def quat_g(x=0.0, y=0.0, z=0.0):
    """A turn of x, y, z degrees about the game's axes, composed as the engine's `EulerRot::XYZ`
    (about X, then Y, then Z, each about the axes as turned by the one before)."""
    q = Quaternion((1, 0, 0), math.radians(x))
    q = q @ Quaternion((0, 1, 0), math.radians(y))
    return q @ Quaternion((0, 0, 1), math.radians(z))


def axis_quat(axis, deg):
    return Quaternion(Vector(axis).normalized(), math.radians(deg))


def fresh_scene():
    bpy.ops.wm.read_factory_settings(use_empty=True)
    scene = bpy.context.scene
    scene.render.fps = FPS
    scene.render.fps_base = 1.0
    return scene


def import_rig(path):
    """Imports the rig `--export-rig` wrote; returns its armature."""
    bpy.ops.import_scene.gltf(filepath=path)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    # The importer's display shape for the bones: not needed.
    for o in [o for o in bpy.data.objects if o.type == "MESH" and o.parent is None and not o.name.endswith("_voxels")]:
        bpy.data.objects.remove(o)
    return arm


def armature():
    return next(o for o in bpy.data.objects if o.type == "ARMATURE")


def joint(arm, bone):
    """A bone's joint at rest, in the game's axes (metres)."""
    return b2g(arm.data.bones[bone].head_local)


def set_local(arm, bone, q_game, loc_game=None):
    """Poses `bone` turned by `q_game` (a turn in the game's axes relative to its parent at rest),
    and for the root moved by `loc_game` (game axes, metres)."""
    pb = arm.pose.bones[bone]
    rest = arm.data.bones[bone].matrix_local.to_quaternion()
    qb = G2B.to_quaternion() @ q_game @ G2B.to_quaternion().inverted()
    pb.rotation_mode = "QUATERNION"
    pb.rotation_quaternion = rest.inverted() @ qb @ rest
    if loc_game is not None:
        pb.location = rest.inverted() @ (G2B @ Vector(loc_game))


def bones_of(arm):
    """The armature's bones, parents first (as the game lists them: the import keeps its order)."""
    return [b.name for b in arm.data.bones]


def root_of(arm):
    """The armature's root bone (the hips for a humanoid, the body or the carapace for the others)."""
    return next(b.name for b in arm.data.bones if b.parent is None)


def parents_of(arm):
    return {b.name: (b.parent.name if b.parent else None) for b in arm.data.bones}


def key_pose(arm, frame, pose, interp="BEZIER", easing="AUTO"):
    """Keys every bone at `frame` from `pose` (a dict of bone to game quaternion, and "loc" for the
    root's offset); bones it leaves out rest. Returns the frame."""
    root = root_of(arm)
    for b in bones_of(arm):
        set_local(arm, b, pose.get(b, Quaternion()), pose.get("loc", (0, 0, 0)) if b == root else None)
        pb = arm.pose.bones[b]
        pb.keyframe_insert("rotation_quaternion", frame=frame)
        if b == root:
            pb.keyframe_insert("location", frame=frame)
    act = arm.animation_data.action
    for fc in fcurves(act):
        for kp in fc.keyframe_points:
            if abs(kp.co.x - frame) < 1e-3:
                kp.interpolation = interp
                kp.easing = easing
    return frame


def fcurves(action):
    """An action's F-curves (Blender 4.4 and on keep them in layers and slots)."""
    if hasattr(action, "fcurves") and len(action.fcurves):
        return list(action.fcurves)
    out = []
    for layer in getattr(action, "layers", []):
        for strip in layer.strips:
            for bag in getattr(strip, "channelbags", []):
                out.extend(bag.fcurves)
    return out


def new_action(arm, name):
    if arm.animation_data is None:
        arm.animation_data_create()
    act = bpy.data.actions.new(name)
    act.use_fake_user = True
    arm.animation_data.action = act
    return act


def setup_render(scene, w=800, h=1000, color="VERTEX"):
    """Workbench, studio light, the voxels' own colours (or each object's, `color="OBJECT"`)."""
    scene.render.engine = "BLENDER_WORKBENCH"
    scene.render.resolution_x = w
    scene.render.resolution_y = h
    scene.render.resolution_percentage = 100
    scene.render.film_transparent = False
    sh = scene.display.shading
    sh.light = "STUDIO"
    sh.color_type = color
    sh.show_cavity = True
    sh.cavity_type = "WORLD"
    sh.show_shadows = True
    scene.display.shadow_focus = 0.9
    scene.world = scene.world or bpy.data.worlds.new("World")
    scene.world.color = (0.18, 0.19, 0.22)
    scene.view_settings.view_transform = "Standard"


def camera(scene, at=(0.0, 0.55, 0.0), yaw=30.0, dist=2.6, rise=0.35, lens=50):
    """A camera looking at `at` (game axes) from `yaw` degrees round from his front, `dist` away,
    `rise` times the distance up."""
    cam = scene.camera
    if cam is None:
        data = bpy.data.cameras.new("cam")
        cam = bpy.data.objects.new("cam", data)
        scene.collection.objects.link(cam)
        scene.camera = cam
    cam.data.lens = lens
    a = math.radians(yaw)
    eye = Vector((math.sin(a) * dist, at[1] + rise * dist, math.cos(a) * dist))
    target = Vector(at)
    cam.location = g2b(eye)
    d = (g2b(target) - g2b(eye)).normalized()
    cam.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    return cam


def render(scene, path):
    scene.render.filepath = os.path.abspath(path)
    bpy.ops.render.render(write_still=True)


def bone_colors(names=None):
    """A colour for each bone (as `anim::skeleton::bone_color`), sRGB 0..1: the heroes' bones, or
    `names` in their order (past twenty, the colours again, darker)."""
    hexes = [0xffffff, 0xffd400, 0xff7a00, 0x8a5a2a, 0xff2a8a, 0x2a6aff, 0x2affd4, 0x9a2aff, 0xff2a2a, 0xffb0b0,
             0x7a0000, 0x2aff2a, 0x007a2a, 0xd4ff8a, 0x00d4ff, 0x005a8a, 0xb0e8ff, 0x404040, 0xa0a0a0, 0x5a2a00]
    out = {}
    for i, b in enumerate(names or BONES):
        h, k = hexes[i % 20], (1.0 if (i // 20) % 2 == 0 else 0.55)
        out[b] = (((h >> 16) & 255) / 255 * k, ((h >> 8) & 255) / 255 * k, (h & 255) / 255 * k, 1.0)
    return out
