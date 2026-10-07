"""Puts a source animation onto Ragnar's armature: Mixamo FBX first, BVH (from video- or text-to-motion
tools, motion capture, any humanoid) second, baked to keys at 60 fps as a new action of his.

    blender -b ragnar.blend -P tools/blender/retarget.py -- --source run.fbx --name run_mixamo
        [--loop] [--stride 2.2] [--marker foot_l=0 ...] [--keep-travel] [--save OUT.blend]
        [--face DEG | --turn DEG] [--yaw-keep K] [--arms-out DEG] [--sword-turn YAW,PITCH] [--hold DEG]
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
* **Baked** at 60 fps whatever the source's rate (an FBX's own, read from the file: Mixamo's sword
  and shield pack came at 30 whatever was asked; a BVH's from its frame time), every bone keyed
  every frame; a loop's last frames eased onto its first.
* **His build.** `--face` sets the way his hips face at the first frame (or `--turn` turns the whole
  clip), `--yaw-keep` keeps only a share of the source's turning (a spin becomes a cut struck where
  he stands), `--arms-out` carries his short arms out from his wide body, `--sword-turn` swings the
  sword arm round his shoulder (toward his right, then up) so a cut passes where the camera behind
  him sees it, `--hold` sets the blade in his fist.

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
# The bones of his torso, which his carriage (`Retarget.posture`) straightens.
TORSO = ("spine", "chest", "neck", "head")
# His weapons, and the source's fist each is held in (its finger bones show how: `Retarget.grip`).
GRIPS = {"blade": "Right", "offhand": "Left"}
# The hold of his weapons in his fists where the source has no fingers (degrees about his hand's x).
HOLD = {"blade": 90.0, "offhand": 90.0, "shield": 0.0}


def swap(bone):
    """His bone on the other side (`upper_arm.L` for `upper_arm.R`), or the same one."""
    if bone.endswith(".L"):
        return bone[:-2] + ".R"
    if bone.endswith(".R"):
        return bone[:-2] + ".L"
    return bone


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
    def __init__(self, src, tgt, facing=Vector((0, -1, 0)), mirror=False, arms_out=0.0, hold=None, turn=0.0, yaw_keep=1.0, sword_turn=(0.0, 0.0), face=None, posture=None,
                 weapon_rest=None):
        self.src, self.tgt = src, tgt
        # Mirrored (a foe holding its weapon in its left fist: `mixamo_foe.py`), its left takes the
        # source's right: the sword arm is its left, and its weapon the `offhand`.
        self.mirror, self.arms_out = mirror, arms_out
        side = "L" if mirror else "R"
        self.sword_arm = (f"upper_arm.{side}", f"forearm.{side}", f"hand.{side}")
        self.sword_bone = "offhand" if mirror else "blade"
        # The sword arm swung round its shoulder as one, `sword_turn` (degrees toward the sword
        # side about the upright, then up), so its cuts pass out where they are seen past the body.
        yaw, pitch = sword_turn
        self.sword_q = Quaternion(Vector((0, 0, 1)), math.radians(-yaw)) @ Quaternion(Vector((1, 0, 0)), math.radians(-pitch))
        if mirror:
            q = self.sword_q
            self.sword_q = Quaternion((q.w, q.x, -q.y, -q.z))
        # The way its weapon points out of its fist at rest (game axes): the heroes' lean up and
        # forward (`vh.WEAPON_REST`); the draugr's sword hangs point down. (A bone's own axes are the
        # game's: it rests unturned.)
        self.weapon_rest = Vector(weapon_rest).normalized() if weapon_rest is not None else vh.WEAPON_REST.copy()
        # A turn of the whole of him (degrees, toward his left), and how much of the source's own
        # turning about the upright to keep (1 all, 0 none: a spinning blow becomes one struck where
        # he stands, his chest still twisting against his hips).
        self.turn, self.yaw_keep = turn, yaw_keep
        # Or his hips set to face `face` degrees (toward his left) at the first frame, whatever the
        # source's heading: clips laid over one another (the guard on the idle) then agree.
        self.face = face
        # His carriage: his torso's bones straightened (degrees, back from the source's lean, each
        # bone carrying those above it), his arms keeping the source's aim.
        self.posture = posture or {}
        self.yaw0 = self.yaw_last = None
        self.hold = dict(HOLD, **(hold or {}))
        self.label, self.map = pick_map(src)
        # The heroes' bones this armature has (the jorogumo has a woman's upper body only, on her
        # spider's carapace: her legs are her own, `block_foe.py`).
        self.bones = [b for b in vh.BONES if b in tgt.data.bones]
        self.map = {t: s for t, s in self.map.items() if t in self.bones}
        self.legged = all(b in self.bones for b in ("thigh.L", "shin.L", "foot.L"))
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
        self.rest_tgt = {b: world_rot(tgt, tgt.data.bones[b].matrix_local) for b in self.bones}
        self.rest_arm = {b: tgt.data.bones[b].matrix_local.to_quaternion() for b in self.bones}
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
        if self.legged:
            self.leg_src = sum((hs[self.map[a]] - hs[self.map[b]]).length for a, b in (("thigh.L", "shin.L"), ("shin.L", "foot.L")))
            self.leg_tgt = sum((ht[a] - ht[b]).length for a, b in (("thigh.L", "shin.L"), ("shin.L", "foot.L")))
            self.scale = self.leg_tgt / self.leg_src
        else:
            # (No legs: its waist stays where its body carries it.)
            self.scale = 0.0
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
        corr = Quaternion()
        upright = {}
        for b in self.bones:
            rest = self.rest_arm[b]
            parent = vh.PARENTS.get(b)
            # Where the bone would turn to with no turn of its own: carried by its parent.
            carried = posed[parent] @ self.rest_arm[parent].inverted() @ rest if parent else rest
            if b in self.map:
                # (Mirrored, his left takes the source's right, turned through the mirror.)
                s = self.map[swap(b)] if self.mirror and swap(b) in self.map else self.map[b]
                d = self.g @ world_rot(src, src.pose.bones[s].matrix) @ self.rest_src[s].inverted()
                if self.mirror:
                    d = Quaternion((d.w, d.x, -d.y, -d.z))
                want = d @ self.k[b] @ (tw @ rest)
                if b == "hips":
                    corr = self.yaw_correction(want @ (tw @ rest).inverted())
                want = corr @ want
                if b == "hips":
                    # (His left, as his hips turn now: the axis his torso straightens about.)
                    side = (want @ (tw @ rest).inverted()) @ (tw @ Vector((1, 0, 0)))
                if b in TORSO:
                    upright[b] = upright.get(vh.PARENTS.get(b), Quaternion()) @ Quaternion(side, math.radians(-self.posture.get(b, 0.0)))
                    want = upright[b] @ want
                if b in self.sword_arm:
                    want = corr @ self.sword_q @ corr.inverted() @ want
                if b.startswith("upper_arm") and self.arms_out:
                    # His short arms carried out from his wide body (about his forward axis).
                    sign = 1.0 if b.endswith(".L") else -1.0
                    want = Quaternion(self.facing, sign * math.radians(self.arms_out)) @ want
                want = tw.inverted() @ want
                basis_q = carried.inverted() @ want
            elif b in GRIPS and self.grip(b) is not None:
                # The weapon held as the source's fist holds it (`grip`).
                grip, knuckles = self.grip(b)
                r0 = tw @ rest
                want = (basis(grip, knuckles) @ basis(r0 @ self.weapon_rest, r0 @ Vector((0, 0, 1))).transposed()).to_quaternion() @ r0
                want = corr @ want
                if b == self.sword_bone:
                    want = corr @ self.sword_q @ corr.inverted() @ want
                want = tw.inverted() @ want
                basis_q = carried.inverted() @ want
            else:
                basis_q = rest.inverted() @ g2b @ vh.quat_g(self.hold.get(b, 0.0)) @ g2b.inverted() @ rest
                want = carried @ basis_q
            posed[b] = want
            out[b] = basis_q
        hips_now = self.g @ (src.matrix_world @ src.pose.bones[self.map["hips"]].head)
        off = (hips_now - self.hips_rest) * self.scale
        if self.mirror:
            off.x = -off.x
        return out, corr @ off

    def grip(self, bone):
        """Where the source's fist points a weapon held in it, in the world: along the grip, from the
        little finger's knuckle to the forefinger's (the blade leaves the fist on the thumb's side),
        its flat toward the knuckles (as his sculpted fist holds it). None without finger bones."""
        side = GRIPS[bone]
        if self.mirror:
            side = "Left" if side == "Right" else "Right"
        names = {short(b.name): b.name for b in self.src.data.bones}
        need = [side + "HandIndex1", side + "HandPinky1", side + "HandMiddle1", side + "Hand"]
        if not all(n in names for n in need):
            return None
        at = {n: self.g @ (self.src.matrix_world @ self.src.pose.bones[names[n]].head) for n in need}
        grip = at[side + "HandIndex1"] - at[side + "HandPinky1"]
        knuckles = at[side + "HandMiddle1"] - at[side + "Hand"]
        if self.mirror:
            # (The other fist, seen in the mirror.)
            grip, knuckles = Vector((-grip.x, grip.y, grip.z)), Vector((-knuckles.x, knuckles.y, knuckles.z))
        if grip.length < 1e-6 or knuckles.length < 1e-6:
            return None
        return grip.normalized(), knuckles.normalized()

    def yaw_correction(self, hips_turn):
        """The turn about the upright that brings his hips' heading (`hips_turn` from rest) to the
        source's first heading plus `yaw_keep` of its turning since, plus `turn`."""
        f = hips_turn @ self.facing
        yaw = math.atan2(f.x, -f.y)
        if self.yaw_last is not None:
            while yaw - self.yaw_last > math.pi:
                yaw -= 2 * math.pi
            while yaw - self.yaw_last < -math.pi:
                yaw += 2 * math.pi
        if self.yaw0 is None:
            self.yaw0 = yaw
        self.yaw_last = yaw
        start = math.radians(self.face) if self.face is not None else self.yaw0 + math.radians(self.turn)
        want = start + self.yaw_keep * (yaw - self.yaw0)
        return Quaternion(Vector((0, 0, 1)), want - yaw)


def bake(rt, name, keep_travel=False, loop=False, stride=None, markers=None, frames=None, fps_src=None, close=8, still=False):
    """Keys the source's action (its frames at `fps_src`, the scene's rate unless given) onto his
    armature as action `name`, at 60 fps. A loop is closed: its last `close` frames ease onto its
    first, so it wraps without a hitch. `still` takes out all his movement over the ground (a leaping
    blow struck where he stands), the rise and fall kept."""
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
    if stride == "auto":
        # (A gait's stride is the ground its cycle covers in the source, scaled to him: none if the
        # source walks in place.)
        drift = last - first
        stride = math.hypot(drift.x, drift.y) or None
        print(f"retarget: {name}: its cycle covers {stride or 0.0:.2f} m")
    if loop and close and n > 2 * close:
        rots0, off0 = samples[0]
        for k in range(1, close + 1):
            f = n - close + k
            u = k / close
            u = u * u * (3 - 2 * u)
            rots, off = samples[f]
            samples[f] = ({b: rots[b].slerp(rots0[b], u) for b in rots}, off.lerp(off0 + (last - first) * (f / n), u))
    # Each bone's turns kept on one side of the sphere, frame to frame (Blender eases between keys by component).
    for f in range(1, len(samples)):
        prev, cur = samples[f - 1][0], samples[f][0]
        for b in cur:
            if prev[b].dot(cur[b]) < 0:
                cur[b] = -cur[b]
    act = vh.new_action(tgt, name)
    tgt.animation_data.action = act
    hips_rest = tgt.data.bones["hips"].matrix_local.to_quaternion()
    tw = tgt.matrix_world.to_quaternion()
    for f, (rots, off) in enumerate(samples):
        off = off.copy()
        if still:
            off.x, off.y = 0.0, 0.0
        elif not keep_travel:
            u = f / max(1, n)
            drift = first.lerp(last, u)
            off.x -= drift.x
            off.y -= drift.y
        for b in rt.bones:
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
    print(f"retarget: {name}: the source's hips face {math.degrees(rt.yaw0 or 0.0):.0f} degrees at its start")
    print(f"retarget: {name}: {n + 1} frames at 60 fps from {s1 - s0} at {fps_src:g} ({rt.label} names, {len(rt.map)} bones), scale {rt.scale:.3f}")
    return act


def load_source(path):
    """Imports the source; returns its armature and the rate its frames run at: a BVH's from its
    frame time (Blender lays its frames one to a frame of the scene), an FBX's from the file (Mixamo
    writes 30 or 60 fps whatever was asked for: the sword and shield pack came at 30, the dodge at 60)."""
    before = set(bpy.data.objects)
    scene = bpy.context.scene
    fps = scene.render.fps / scene.render.fps_base
    ext = os.path.splitext(path)[1].lower()
    if ext == ".fbx":
        # (The importer sets the scene to the file's own rate and lays its keys out at it: read the
        # rate, and give the scene back its 60.)
        keep = (scene.render.fps, scene.render.fps_base)
        bpy.ops.import_scene.fbx(filepath=path, automatic_bone_orientation=False, use_anim=True)
        fps = scene.render.fps / scene.render.fps_base
        scene.render.fps, scene.render.fps_base = keep
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
    opts = {"--source": None, "--name": None, "--save": None, "--stride": None, "--turn": "0", "--face": None, "--yaw-keep": "1",
            "--arms-out": "0", "--hold": None, "--sword-turn": "0,0"}
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
    rt = Retarget(src, tgt, arms_out=float(opts["--arms-out"]), turn=float(opts["--turn"]), yaw_keep=float(opts["--yaw-keep"]),
                  face=float(opts["--face"]) if opts["--face"] else None, hold={"blade": float(opts["--hold"])} if opts["--hold"] else None,
                  sword_turn=tuple(float(x) for x in opts["--sword-turn"].split(",")))
    bake(rt, opts["--name"], keep, loop, float(opts["--stride"]) if opts["--stride"] else None, markers, fps_src=fps)
    bpy.data.objects.remove(src)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(opts["--save"] or bpy.data.filepath))


if __name__ == "__main__":
    main()
