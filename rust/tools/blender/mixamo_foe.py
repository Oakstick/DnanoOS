"""The humanoid foes' clips from Mixamo (ANIMATION.md §11): the picks for each (a table per foe), each
retargeted onto its armature with the corrections its build needs, its markers found from the motion,
written as actions into its .blend (replacing those of the same name). As `mixamo_hero.py`, whose
retarget, markers and trimming it uses.

    blender -b FOE.blend -P tools/blender/mixamo_foe.py -- FOE MIXAMO_DIR OUT.blend [slot ...]
    blender -b FOE.blend -P tools/blender/mixamo_foe.py -- FOE MIXAMO_DIR OUT.blend --try FILE[:OPTS] ...

FOE is a humanoid foe's rig as `--export-rig` names it (`draugr`, `helmeted`, `blemmyes`, `tengu`,
`rasetsu`, `witch`, `jorogumo`); FOE.blend its rig as `import_rig.py` saves it. MIXAMO_DIR holds the
packs as downloaded (`zombie/`, `melee-axe/`, `magic/`, `mutant/`, `foe-extra/`, `sword-shield/`,
`great-sword/`); the FBX files stay outside the repository, only the clips exported from OUT.blend
go into `assets/anim/<foe>/`.

The foes hold their weapons in their own left fists (their sculpts are not mirrored as the heroes'
are: `anim::foe_rigs`), so a pick made for a right hand is mirrored (`mirror`): its left takes the
source's right, turned through the mirror, and the weapon (the `offhand` bone) is held as the
source's right fist holds it, the way it points at rest given (`weapon_rest`, game axes).

The slots are the game's (`anim::foe`): `idle`, `walk`, `run`; each move's clip by the move
(`slam`, `sweep`, `hex`, `rally`); the reels `hit` (laid over the upper body), `stagger` and
`deflected` (whole). A move's markers are found from its weapon's tip as a strike's are (`weapon`,
the bone that strikes; the head for a roar).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import mixamo_hero as mh  # noqa: E402
import vh_blender as vh  # noqa: E402

Z = "zombie/zombie "
SS = "sword-shield/sword and shield "
GS = "great-sword/great sword "
FX = "foe-extra/"
AX = "melee-axe/standing "

# ------------------------------------------------------------------ the draugr

# His notched sword hangs point down from his bony fist at rest, leaning forward a little
# (`foe_rigs::FoeRig::weapon_rest`).
DRAUGR_SWORD = dict(mirror=True, weapon="offhand", weapon_rest=(0.0, -0.975, 0.22))
# The dead man's carriage: the zombie's own, a little hunched; his arms carried clear of his mail.
DRAUGR = {
    "idle": (Z + "idle.fbx", dict(DRAUGR_SWORD, loop=True, arms_out=5)),
    "walk": (Z + "walk.fbx", dict(DRAUGR_SWORD, loop=True, stride="feet", arms_out=5)),
    "run": (Z + "run.fbx", dict(DRAUGR_SWORD, loop=True, stride="feet", arms_out=5)),
    "slam": (AX + "melee attack downward.fbx", dict(DRAUGR_SWORD, kind="strike")),
    "sweep": (SS + "attack (2).fbx", dict(DRAUGR_SWORD, kind="strike", yaw_keep=0.0, face=0.0, still=True)),
    "rally": (Z + "scream.fbx", dict(DRAUGR_SWORD, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (FX + "zombie reaction hit flinch.fbx", dict(DRAUGR_SWORD, kind="flinch", cut=60)),
    "stagger": (FX + "zombie reaction hit stumble back.fbx", dict(DRAUGR_SWORD, kind="flinch", cut=100)),
}

FOES = {"draugr": DRAUGR}


def label_of(path):
    base = os.path.splitext(os.path.basename(path))[0]
    for long, short in (("sword and shield ", "ss "), ("great sword ", "gs "), ("zombie ", "z "), ("standing ", "")):
        base = base.replace(long, short)
    return base.replace(" ", "_")


def deflected(tgt, src="stagger", frames=24):
    """The recoil of a parried blow: the first part of the stagger (thrown back), eased home by
    `frames`."""
    old = bpy.data.actions.get("deflected")
    if old:
        bpy.data.actions.remove(old)
    act_src = bpy.data.actions.get(src)
    if act_src is None:
        return
    scene = bpy.context.scene
    names = vh.bones_of(tgt)
    root = vh.root_of(tgt)

    def grab(act, f):
        tgt.animation_data.action = act
        scene.frame_set(f)
        return {b: (tgt.pose.bones[b].rotation_quaternion.copy(), tgt.pose.bones[b].location.copy()) for b in names}

    # (The stagger's turn at its deepest in its first third.)
    n = int(act_src.frame_range[1])
    rest = grab(act_src, 0)
    deep = grab(act_src, max(4, n // 4))
    act = vh.new_action(tgt, "deflected")
    for f, k in ((0, 0.0), (frames // 4, 1.0), (frames // 2, 0.8), (frames, 0.0)):
        for b in names:
            pb = tgt.pose.bones[b]
            pb.rotation_mode = "QUATERNION"
            pb.rotation_quaternion = rest[b][0].slerp(deep[b][0], k)
            pb.keyframe_insert("rotation_quaternion", frame=f)
            if b == root:
                pb.location = rest[b][1].lerp(deep[b][1], k)
                pb.keyframe_insert("location", frame=f)
    act.frame_range = (0, frames)
    act.use_frame_range = True
    act.pose_markers.new("recover_end").frame = frames
    print(f"mixamo_foe: deflected    from the stagger's first quarter, {frames} frames")


def main():
    a = vh.args()
    if len(a) < 3:
        sys.exit(__doc__)
    foe, root, out, rest = a[0], a[1], a[2], a[3:]
    if foe not in FOES:
        sys.exit(f"mixamo_foe: no foe {foe} ({', '.join(FOES)})")
    tgt = vh.armature()
    if tgt.animation_data is None:
        tgt.animation_data_create()
    if rest and rest[0] == "--try":
        for item in rest[1:]:
            path, _, extra = item.partition(":")
            opts = eval(f"dict({extra})") if extra else {}
            mh.make(label_of(path), os.path.join(root, path), opts, tgt, prefix="mixamo_foe")
    else:
        only = set(rest)
        for slot, (path, opts) in FOES[foe].items():
            if only and slot not in only:
                continue
            mh.make(slot, os.path.join(root, path), opts, tgt, prefix="mixamo_foe")
        if (not only or "deflected" in only) and "stagger" in bpy.data.actions and "deflected" not in FOES[foe]:
            deflected(tgt)
    bpy.context.scene.render.fps = vh.FPS
    tgt.animation_data.action = bpy.data.actions.get("idle")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("mixamo_foe: saved", out)


if __name__ == "__main__":
    main()
