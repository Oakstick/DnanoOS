"""Puts a source animation onto Ragnar's armature: Mixamo FBX first, BVH (from video- or text-to-motion
tools, motion capture, any humanoid) second, baked to keys at 60 fps as a new action of his.

    blender -b ragnar.blend -P tools/blender/retarget.py -- --source run.fbx --name run_mixamo
        [--loop] [--stride 2.2] [--marker foot_l=0 ...] [--keep-travel] [--save OUT.blend]
    blender -b -P tools/blender/retarget.py -- --selftest ragnar_rig.glb [OUT_DIR]

How it goes:

* **The bone map.** Each of his bones takes one of the source's: Mixamo's `mixamorig:` names (any
  prefix up to a colon is dropped), then the SMPL body model's 22 joints (`pelvis`, `left_hip`,
  `spine3`...: what most video- and text-to-motion tools write), then the usual BVH names
  (`Hips`, `LeftUpLeg`, `LeftForeArm`, CMU's `LowerBack`...). Whichever map names the most of the
  source's bones is used. His weapons and shield are not in any: they keep his hold.
* **The rest poses aligned.** Each bone's direction at rest (from its joint to the next joint down
  the limb) is turned onto the source bone's, about the way both face (a T-pose's level arms
  onto his hanging ones), once; every frame the source bone's turn away from its own rest is
  then laid on top, in the world. Chains of other lengths (Mixamo's three spine bones on his two)
  come out right, as each of his bones takes its source bone's turn in the world.
* **Scale from leg length.** The hips' movement is scaled by his leg (hip to ankle) over the
  source's.
* **In place.** His travel over the ground is taken out (the game moves him): the drift from the
  first frame to the last, leaving the sway; `--keep-travel` keeps it. The rise and fall stay.
* **Baked** at 60 fps whatever the source's rate, every bone keyed every frame.

The self-test builds a Mixamo-named armature in a T-pose, animates it (arms down and bent, a leg
lifted, the spine turned, the hips carried forward and up, at 30 fps), retargets it onto Ragnar
and checks every mapped limb points the way the source's does, frame by frame, and that the hips
stay over his feet: it prints PASS or FAIL (exit 1), and renders the two side by side.
"""
import math
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import vh_blender as vh  # noqa: E402
from mathutils import Matrix, Quaternion, Vector  # noqa: E402

# His bones and their source bones, by convention. (Neck and spine where a source has fewer bones
# map to the nearest it has.)
MIXAMO = {
    "hips": "Hips", "spine": "Spine", "chest": "Spine2", "neck": "Neck", "head": "Head",
    "upper_arm.L": "LeftArm", "forearm.L": "LeftForeArm", "hand.L": "LeftHand",
    "upper_arm.R": "RightArm", "forearm.R": "RightForeArm", "hand.R": "RightHand",
    "thigh.L": "LeftUpLeg", "shin.L": "LeftLeg", "foot.L": "LeftFoot",
    "thigh.R": "RightUpLeg", "shin.R": "RightLeg", "foot.R": "RightFoot",
}
SMPL = {
    "hips": "pelvis", "spine": "spine1", "chest": "spine3", "neck": "neck", "head": "head",
    "upper_arm.L": "left_shoulder", "forearm.L": "left_elbow", "hand.L": "left_wrist",
    "upper_arm.R": "right_shoulder", "forearm.R": "right_elbow", "hand.R": "right_wrist",
    "thigh.L": "left_hip", "shin.L": "left_knee", "foot.L": "left_ankle",
    "thigh.R": "right_hip", "shin.R": "right_knee", "foot.R": "right_ankle",
}
BVH = {
    "hips": "Hips", "spine": "Spine", "chest": "Spine1", "neck": "Neck", "head": "Head",
    "upper_arm.L": "LeftArm", "forearm.L": "LeftForeArm", "hand.L": "LeftHand",
    "upper_arm.R": "RightArm", "forearm.R": "RightForeArm", "hand.R": "RightHand",
    "thigh.L": "LeftUpLeg", "shin.L": "LeftLeg", "foot.L": "LeftFoot",
    "thigh.R": "RightUpLeg", "shin.R": "RightLeg", "foot.R": "RightFoot",
}
# Other names the same joints go by in BVH files (CMU's, and a few tools').
ALIASES = {
    "Spine": ["LowerBack", "spine", "Chest0"], "Spine1": ["Spine2", "Chest", "spine1", "UpperChest"],
    "LeftArm": ["LeftUpArm", "LeftShoulder2"], "RightArm": ["RightUpArm", "RightShoulder2"],
    "LeftUpLeg": ["LeftHip", "LHip", "LeftThigh"], "RightUpLeg": ["RightHip", "RHip", "RightThigh"],
    "LeftLeg": ["LeftKnee", "LeftShin"], "RightLeg": ["RightKnee", "RightShin"],
    "LeftFoot": ["LeftAnkle"], "RightFoot": ["RightAnkle"], "LeftForeArm": ["LeftElbow"], "RightForeArm": ["RightElbow"],
    "LeftHand": ["LeftWrist"], "RightHand": ["RightWrist"],
}
MAPS = [("Mixamo", MIXAMO), ("SMPL", SMPL), ("BVH", BVH)]

# The next joint down each of his limbs (a bone's direction runs from its joint to that one's).
CHILD = {
    "hips": "spine", "spine": "chest", "chest": "neck", "neck": "head",
    "upper_arm.L": "forearm.L", "forearm.L": "hand.L", "upper_arm.R": "forearm.R", "forearm.R": "hand.R",
    "thigh.L": "shin.L", "shin.L": "foot.L", "thigh.R": "shin.R", "shin.R": "foot.R",
}
# The hold of his weapons in his fists where the source has none (degrees about his hand's x).
HOLD = {"blade": 90.0, "offhand": 90.0, "shield": 0.0}


def short(name):
    return name.split(":")[-1]


def pick_map(src):
    names = {short(b.name): b.name for b in src.data.bones}

    def find(n):
        for cand in [n] + ALIASES.get(n, []):
            if cand in names:
                return names[cand]
        return None

    best = None
    for label, m in MAPS:
        found = {t: find(s) for t, s in m.items()}
        found = {t: s for t, s in found.items() if s}
        if best is None or len(found) > len(best[1]):
            best = (label, found)
    return best


def world_rot(obj, mat):
    return (obj.matrix_world.to_3x3() @ mat.to_3x3()).to_quaternion()


def heads(obj, posed):
    """Every bone's joint in the world, at rest or as posed now."""
    mw = obj.matrix_world
    if posed:
        return {pb.name: mw @ pb.head for pb in obj.pose.bones}
    return {b.name: mw @ b.head_local for b in obj.data.bones}


def basis(p, s):
    p = p.normalized()
    s = s - p * s.dot(p)
    s.normalize()
    return Matrix((p, s, p.cross(s))).transposed()


def align(d_from, d_to, up):
    """The turn taking direction `d_from` onto `d_to`, keeping `up` (the way both face) as it can."""
    return (basis(d_to, up) @ basis(d_from, up).transposed()).to_quaternion()


def direction(h, bone, child_of, parent_dir=None, fallback=None):
    c = child_of.get(bone)
    if c and c in h and bone in h and (h[c] - h[bone]).length > 1e-6:
        return (h[c] - h[bone]).normalized()
    return fallback if fallback is not None else parent_dir


class Retarget:
    def __init__(self, src, tgt, facing=Vector((0, -1, 0))):
        self.src, self.tgt = src, tgt
        self.label, self.map = pick_map(src)
        self.facing = facing
        # The source's chain: the source bone of each of his bones' next joint down.
        self.src_child = {}
        for t, s in self.map.items():
            c = CHILD.get(t)
            if c in self.map:
                self.src_child[s] = self.map[c]
        hs, ht = heads(src, False), heads(tgt, False)
        up = Vector((0, 0, 1))
        # A source standing other than upright (a BVH read with the wrong axes): stood up first.
        top = self.map.get("head") or self.map.get("neck") or self.map.get("chest")
        rise = (hs[top] - hs[self.map["hips"]]).normalized()
        self.g = rise.rotation_difference(up) if rise.dot(up) < 0.9 else Quaternion()
        hs = {k: self.g @ h for k, h in hs.items()}
        self.rest_src = {s: self.g @ world_rot(src, src.data.bones[s].matrix_local) for s in self.map.values()}
        self.rest_tgt = {b: world_rot(tgt, tgt.data.bones[b].matrix_local) for b in vh.BONES}
        self.rest_arm = {b: tgt.data.bones[b].matrix_local.to_quaternion() for b in vh.BONES}
        self.k = {}
        for t, s in self.map.items():
            if t.startswith("foot"):
                # A foot points forward, level: his and the source's (its toe, where it has one).
                toe = next((b.name for b in src.data.bones if b.parent and b.parent.name == s), None)
                ds = (hs[toe] - hs[s]) if toe else self.facing.copy()
                ds.z = 0.0
                ds = ds.normalized() if ds.length > 1e-6 else self.facing.copy()
                dt = self.facing.copy()
            elif t == "head" or t.startswith("hand"):
                parent_t = vh.PARENTS[t]
                ps = self.map.get(parent_t)
                dt = direction(ht, parent_t, CHILD) if t != "head" else up
                ds = (hs[s] - hs[ps]).normalized() if t != "head" and ps else up
            else:
                dt = direction(ht, t, CHILD)
                ds = direction(hs, s, self.src_child, fallback=up)
            ref = self.facing if abs(dt.dot(self.facing)) < 0.95 else up
            self.k[t] = align(dt, ds, ref)
        self.leg_src = sum((hs[self.map[a]] - hs[self.map[b]]).length for a, b in (("thigh.L", "shin.L"), ("shin.L", "foot.L")))
        self.leg_tgt = sum((ht[a] - ht[b]).length for a, b in (("thigh.L", "shin.L"), ("shin.L", "foot.L")))
        self.scale = self.leg_tgt / self.leg_src
        self.hips_rest = hs[self.map["hips"]]
        if self.g != Quaternion():
            print("retarget: the source lies; stood up by", [round(math.degrees(x)) for x in self.g.to_euler()])

    def pose(self):
        """His pose now, from the source's pose now: each bone's pose turn (relative to its rest, as
        Blender keys it) and his hips' offset in the world."""
        src, tgt = self.src, self.tgt
        tw = tgt.matrix_world.to_quaternion()
        g2b = vh.G2B.to_quaternion()
        posed, out = {}, {}
        for b in vh.BONES:
            rest = self.rest_arm[b]
            parent = vh.PARENTS.get(b)
            # Where the bone would turn to with no turn of its own: carried by its parent.
            carried = posed[parent] @ self.rest_arm[parent].inverted() @ rest if parent else rest
            if b in self.map:
                s = self.map[b]
                d = self.g @ world_rot(src, src.pose.bones[s].matrix) @ self.rest_src[s].inverted()
                want = tw.inverted() @ (d @ self.k[b] @ (tw @ rest))
                basis_q = carried.inverted() @ want
            else:
                basis_q = rest.inverted() @ g2b @ vh.quat_g(HOLD.get(b, 0.0)) @ g2b.inverted() @ rest
                want = carried @ basis_q
            posed[b] = want
            out[b] = basis_q
        hips_now = self.g @ (src.matrix_world @ src.pose.bones[self.map["hips"]].head)
        return out, (hips_now - self.hips_rest) * self.scale


def bake(rt, name, keep_travel=False, loop=False, stride=None, markers=None, frames=None, fps_src=None):
    """Keys the source's action (its frames at `fps_src`, the scene's rate unless given) onto his
    armature as action `name`, at 60 fps."""
    scene = bpy.context.scene
    src, tgt = rt.src, rt.tgt
    act_src = src.animation_data.action
    s0, s1 = frames or (int(act_src.frame_range[0]), int(act_src.frame_range[1]))
    fps_src = fps_src or scene.render.fps / scene.render.fps_base
    n = max(1, round((s1 - s0) / fps_src * vh.FPS))
    samples = []
    for f in range(n + 1):
        t = s0 + f / vh.FPS * fps_src
        scene.frame_set(int(math.floor(t)), subframe=t - math.floor(t))
        samples.append(rt.pose())
    # In place: the drift over the ground from first frame to last taken out (the sway stays).
    first, last = samples[0][1], samples[-1][1]
    act = vh.new_action(tgt, name)
    tgt.animation_data.action = act
    hips_rest = tgt.data.bones["hips"].matrix_local.to_quaternion()
    tw = tgt.matrix_world.to_quaternion()
    for f, (rots, off) in enumerate(samples):
        off = off.copy()
        if not keep_travel:
            u = f / max(1, n)
            drift = first.lerp(last, u)
            off.x -= drift.x
            off.y -= drift.y
        for b in vh.BONES:
            pb = tgt.pose.bones[b]
            pb.rotation_mode = "QUATERNION"
            pb.rotation_quaternion = rots[b]
            pb.keyframe_insert("rotation_quaternion", frame=f)
        hb = tgt.pose.bones["hips"]
        hb.location = hips_rest.inverted() @ (tw.inverted() @ off)
        hb.keyframe_insert("location", frame=f)
    for fc in vh.fcurves(act):
        for kp in fc.keyframe_points:
            kp.interpolation = "LINEAR"
    act.frame_range = (0, n)
    act.use_frame_range = True
    act["vh_loop"] = loop
    if stride:
        act["vh_stride"] = stride
    for k, v in (markers or {}).items():
        act.pose_markers.new(k).frame = int(v)
    print(f"retarget: {name}: {n + 1} frames at 60 fps from {s1 - s0} at {fps_src:g} ({rt.label} names, {len(rt.map)} bones), scale {rt.scale:.3f}")
    return act


def load_source(path):
    """Imports the source; returns its armature and the rate its frames run at (a BVH's own, from
    its frame time: Blender lays a BVH's frames one to a frame of the scene; an FBX's are already
    at the scene's)."""
    before = set(bpy.data.objects)
    scene = bpy.context.scene
    fps = scene.render.fps / scene.render.fps_base
    ext = os.path.splitext(path)[1].lower()
    if ext == ".fbx":
        bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=False, use_anim=True)
    elif ext == ".bvh":
        bpy.ops.import_anim.bvh(filepath=path, update_scene_fps=False, rotate_mode="NATIVE")
        with open(path) as f:
            for line in f:
                if line.strip().lower().startswith("frame time:"):
                    fps = 1.0 / float(line.split(":")[1])
                    break
    else:
        sys.exit(f"retarget: cannot read {path} (FBX or BVH)")
    new = [o for o in bpy.data.objects if o not in before and o.type == "ARMATURE"]
    if not new:
        sys.exit(f"retarget: no armature in {path}")
    return new[0], fps


# ------------------------------------------------------------------ the self-test

def mixamo_armature():
    """A Mixamo-named armature in a T-pose, 1.8 m, facing -Y (as Mixamo's FBX comes into Blender)."""
    data = bpy.data.armatures.new("mixamo")
    obj = bpy.data.objects.new("mixamo", data)
    bpy.context.scene.collection.objects.link(obj)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    bones = {}

    def bone(name, head, tail, parent=None):
        b = data.edit_bones.new("mixamorig:" + name)
        b.head, b.tail = Vector(head), Vector(tail)
        if parent:
            b.parent = bones[parent]
            b.use_connect = False
        bones[name] = b

    bone("Hips", (0, 0, 1.0), (0, 0, 1.1))
    bone("Spine", (0, 0, 1.1), (0, 0, 1.2), "Hips")
    bone("Spine1", (0, 0, 1.2), (0, 0, 1.3), "Spine")
    bone("Spine2", (0, 0, 1.3), (0, 0, 1.45), "Spine1")
    bone("Neck", (0, 0, 1.45), (0, 0, 1.55), "Spine2")
    bone("Head", (0, 0, 1.55), (0, 0, 1.75), "Neck")
    for side, sx in (("Left", 1), ("Right", -1)):
        bone(side + "Shoulder", (0.05 * sx, 0, 1.42), (0.18 * sx, 0, 1.42), "Spine2")
        bone(side + "Arm", (0.18 * sx, 0, 1.42), (0.45 * sx, 0, 1.42), side + "Shoulder")
        bone(side + "ForeArm", (0.45 * sx, 0, 1.42), (0.72 * sx, 0, 1.42), side + "Arm")
        bone(side + "Hand", (0.72 * sx, 0, 1.42), (0.82 * sx, 0, 1.42), side + "ForeArm")
        bone(side + "UpLeg", (0.1 * sx, 0, 1.0), (0.1 * sx, 0, 0.55), "Hips")
        bone(side + "Leg", (0.1 * sx, 0, 0.55), (0.1 * sx, 0, 0.1), side + "UpLeg")
        bone(side + "Foot", (0.1 * sx, 0, 0.1), (0.1 * sx, -0.12, 0.02), side + "Leg")
        bone(side + "ToeBase", (0.1 * sx, -0.12, 0.02), (0.1 * sx, -0.2, 0.02), side + "Foot")
    bpy.ops.object.mode_set(mode="OBJECT")
    obj.location = (1.2, 0, 0)
    return obj


def turn(obj, name, axis, deg):
    """Turns source bone `name` by `deg` about `axis` (the armature's axes) from its rest."""
    pb = obj.pose.bones["mixamorig:" + name]
    rest = obj.data.bones[pb.name].matrix_local.to_quaternion()
    pb.rotation_mode = "QUATERNION"
    pb.rotation_quaternion = rest.inverted() @ Quaternion(Vector(axis), math.radians(deg)) @ rest


def selftest(rig_path, out):
    scene = vh.fresh_scene()
    tgt = vh.import_rig(rig_path)
    src = mixamo_armature()
    scene.render.fps = 30
    act = vh.new_action(src, "source")
    for f, k in ((1, 0.0), (16, 0.5), (31, 1.0)):
        for b in src.pose.bones:
            b.rotation_mode = "QUATERNION"
            b.rotation_quaternion = Quaternion()
        # Arms down and the elbows bent forward; the right leg lifted, its knee bent; the spine turned.
        turn(src, "LeftArm", (0, 1, 0), 80 * k)
        turn(src, "LeftForeArm", (0, 0, 1), 70 * k)
        turn(src, "RightArm", (0, 1, 0), -60 * k)
        turn(src, "RightUpLeg", (1, 0, 0), 50 * k)
        turn(src, "RightLeg", (1, 0, 0), -60 * k)
        turn(src, "Spine1", (0, 0, 1), 25 * k)
        hb = src.pose.bones["mixamorig:Hips"]
        hb.location = Quaternion() @ Vector((0, 0, 0))
        rest = src.data.bones[hb.name].matrix_local.to_quaternion()
        hb.location = rest.inverted() @ Vector((0.0, -1.0 * k, 0.05 * k))
        for b in src.pose.bones:
            b.keyframe_insert("rotation_quaternion", frame=f)
        hb.keyframe_insert("location", frame=f)
    act.frame_range = (1, 31)
    # (Carried forward at an even pace, so all of the travel is drift and none of it sway.)
    for fc in vh.fcurves(act):
        for kp in fc.keyframe_points:
            kp.interpolation = "LINEAR"
    rt = Retarget(src, tgt)
    baked = bake(rt, "selftest")
    tgt.animation_data.action = baked
    scene.render.fps = 60
    failures = []
    worst = 0.0
    for f in range(0, 61, 6):
        # The source at its own frame (30 fps), then him at the matching one of his (60 fps).
        scene.frame_set(1 + f // 2, subframe=(f % 2) / 2.0)
        hs = heads(src, True)
        scene.frame_set(f)
        ht = heads(tgt, True)
        for t, s, sc in (("upper_arm.L", "LeftArm", "LeftForeArm"), ("forearm.L", "LeftForeArm", "LeftHand"),
                         ("upper_arm.R", "RightArm", "RightForeArm"), ("thigh.R", "RightUpLeg", "RightLeg"),
                         ("shin.R", "RightLeg", "RightFoot"), ("thigh.L", "LeftUpLeg", "LeftLeg")):
            ds = (hs["mixamorig:" + sc] - hs["mixamorig:" + s]).normalized()
            dt = (ht[CHILD[t]] - ht[t]).normalized()
            ang = math.degrees(ds.angle(dt))
            worst = max(worst, ang)
            if ang > 4.0:
                failures.append(f"frame {f}: {t} is {ang:.1f} degrees off {s}")
        hips = ht["hips"] - tgt.matrix_world @ tgt.data.bones["hips"].head_local
        if abs(hips.x) > 1e-3 or abs(hips.y) > 1e-3:
            failures.append(f"frame {f}: his hips travel ({hips.x:.4f}, {hips.y:.4f})")
        rise = 0.05 * (f / 60.0) * rt.scale
        if abs(hips.z - rise) > 2e-3:
            failures.append(f"frame {f}: his hips at {hips.z:.4f}, not {rise:.4f}")
    print(f"retarget: self-test: worst limb {worst:.2f} degrees off the source's; leg scale {rt.scale:.3f}")
    if out:
        os.makedirs(out, exist_ok=True)
        # (The last frame: the source at 31, him at 60; shown together, the source's pose at his.)
        scene.frame_set(60)
        src.animation_data.action = None
        for b in src.pose.bones:
            b.rotation_quaternion = Quaternion()
        turn(src, "LeftArm", (0, 1, 0), 80)
        turn(src, "LeftForeArm", (0, 0, 1), 70)
        turn(src, "RightArm", (0, 1, 0), -60)
        turn(src, "RightUpLeg", (1, 0, 0), 50)
        turn(src, "RightLeg", (1, 0, 0), -60)
        turn(src, "Spine1", (0, 0, 1), 25)
        src.pose.bones["mixamorig:Hips"].location = Vector((0, 0, 0))
        bpy.context.view_layer.update()
        vh.setup_render(scene, 900, 700)
        cam = vh.camera(scene, at=(0.7, 0.85, 0.0), yaw=12.0, dist=4.6, rise=0.12)
        cam.data.lens = 40
        # (A stick figure for the source: its bones' envelopes shown in the render.)
        src.data.display_type = "ENVELOPE"
        src.show_in_front = True
        for b in src.data.bones:
            b.envelope_distance = 0.03
            b.head_radius = b.tail_radius = 0.03
        mesh = stick(src)
        vh.render(scene, os.path.join(out, "retarget_selftest.png"))
        bpy.data.objects.remove(mesh)
    for msg in failures:
        print("retarget: FAIL", msg)
    print("retarget: self-test", "FAIL" if failures else "PASS")
    return not failures


def stick(arm):
    """A mesh of tubes along an armature's posed bones, so a render shows it."""
    import bmesh
    bm = bmesh.new()
    for pb in arm.pose.bones:
        a, b = arm.matrix_world @ pb.head, arm.matrix_world @ pb.tail
        d = b - a
        if d.length < 1e-4:
            continue
        m = Matrix.Translation((a + b) / 2) @ d.to_track_quat("Z", "Y").to_matrix().to_4x4() @ Matrix.Diagonal((0.04, 0.04, d.length, 1))
        bmesh.ops.create_cube(bm, size=1.0, matrix=m)
    me = bpy.data.meshes.new("stick")
    bm.to_mesh(me)
    obj = bpy.data.objects.new("stick", me)
    obj.color = (0.9, 0.9, 0.95, 1)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def main():
    a = vh.args()
    if "--selftest" in a:
        i = a.index("--selftest")
        ok = selftest(a[i + 1], a[i + 2] if len(a) > i + 2 else None)
        sys.exit(0 if ok else 1)
    opts = {"--source": None, "--name": None, "--save": None, "--stride": None}
    markers, loop, keep = {}, False, False
    i = 0
    while i < len(a):
        if a[i] in opts:
            opts[a[i]] = a[i + 1]
            i += 2
        elif a[i] == "--marker":
            k, v = a[i + 1].split("=")
            markers[k] = int(v)
            i += 2
        elif a[i] == "--loop":
            loop, i = True, i + 1
        elif a[i] == "--keep-travel":
            keep, i = True, i + 1
        else:
            sys.exit(__doc__)
    if not opts["--source"] or not opts["--name"]:
        sys.exit(__doc__)
    tgt = vh.armature()
    src, fps = load_source(opts["--source"])
    if tgt.animation_data is None:
        tgt.animation_data_create()
    rt = Retarget(src, tgt)
    bake(rt, opts["--name"], keep, loop, float(opts["--stride"]) if opts["--stride"] else None, markers, fps_src=fps)
    bpy.data.objects.remove(src)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(opts["--save"] or bpy.data.filepath))


main()
