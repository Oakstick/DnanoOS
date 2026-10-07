"""The story's cast from Mixamo (ANIMATION.md §12): the four boys' clips (one set, their bodies are
one sculpt) and the old gatekeeper's, each retargeted onto the rig `--export-rig kid0` or `oldman`
wrote (as `import_rig.py` saved it), its markers found from the motion, written as actions into the
.blend (replacing those of the same name). As `mixamo_hero.py`, whose retarget, markers and
trimming it uses.

    blender -b kid0.blend -P tools/blender/mixamo_cast.py -- kid C:/Users/ulask/Games/mixamo kid.blend [slot ...]
    blender -b oldman.blend -P tools/blender/mixamo_cast.py -- oldman C:/Users/ulask/Games/mixamo oldman.blend [slot ...]
    blender -b kid0.blend -P tools/blender/mixamo_cast.py -- kid C:/Users/ulask/Games/mixamo try.blend --try FILE[:OPTS] ...

MIXAMO_DIR holds the cast set as downloaded (`cast/`: FBX Binary, 30 fps, without skin) and the
packs (`melee-axe/` for a plain walk); the FBX files stay outside the repository, only the clips
exported from the .blend go into `assets/anim/kid/` and `assets/anim/oldman/`.

The slots are the names the game's cast animator asks for (`anim::cast`): loops (`loop`) for what
is held (an idle, a run, a fall, the terror), one-shots for the rest, their markers found from the
motion: a kick's (`kind="strike"` on the kicking foot: `hit` its contact), a landing's (`kind="leap"`:
`takeoff`, `apex`, `land`). The boys' clips are not mirrored (they hold nothing); one kicks with his
right foot, one with his left (`mirror`), so the passes differ.

The Scene II hug Mixamo has none of: keyed by script (`block_cast.py`).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import mixamo_hero as mh  # noqa: E402
import vh_blender as vh  # noqa: E402

C = "cast/"
FOOT = dict(kind="strike", weapon="foot.R", tip_rest=(0.0, -0.3, 1.0))
FOOT_L = dict(FOOT, weapon="foot.L")

# The boys: short, a big head; their arms carried a little clear of their round coats.
B = dict(arms_out=4)
KID = {
    # Standing about: three idles, so the four never breathe in step.
    "idle": (C + "Idle.fbx", dict(B, loop=True)),
    "idle_happy": (C + "happy idle 1.fbx", dict(B, loop=True)),
    "idle_happy_2": (C + "happy idle 2.fbx", dict(B, loop=True, mirror=True)),
    # The game: passes and kicks (contact at `hit`), the last one wide (the jog and strike, its run-up kept in place).
    "pass": (C + "Soccer Pass.fbx", dict(B, **FOOT)),
    "pass_left": (C + "Soccer Pass.fbx", dict(B, mirror=True, **FOOT_L)),
    "kick": (C + "Kick Soccerball.fbx", dict(B, **FOOT)),
    "miskick": (C + "Strike Foward Jog.fbx", dict(B, still=True, **FOOT)),
    # The fog, the eyes: looking about, frightened.
    "look": (C + "Look Around.fbx", dict(B, loop=True)),
    "look_nervous": (C + "Nervously Look Around.fbx", dict(B, loop=True)),
    "terrified": (C + "Terrified.fbx", dict(B, loop=True, span=(0.0, 10.0))),
    # "He took him!"
    "yell": (C + "Yelling Out.fbx", dict(B, still=True)),
    "yell_short": (C + "Standing Yell.fbx", dict(B)),
    # After him, and down.
    "run": (C + "Fast Run.fbx", dict(B, loop=True, stride="feet")),
    "sprint": (C + "Sprint.fbx", dict(B, loop=True, stride="feet")),
    "walk": ("melee-axe/unarmed walk forward.fbx", dict(B, loop=True, stride="feet")),
    "walk_back": (C + "Walking Backwards.fbx", dict(B, loop=True, stride="feet")),
    "fall": (C + "Falling.fbx", dict(B, loop=True)),
    "land": (C + "Hard Landing.fbx", dict(B, kind="leap")),
    "fall_land": (C + "Falling To Landing.fbx", dict(B, kind="leap")),
    "get_up": (C + "Getting Up.fbx", dict(B, still=True)),
    # Before the old man: asking, agreeing, nodding, pointing.
    "talk": (C + "talking two hands (boys).fbx", dict(B, loop=True)),
    "talk_m": (C + "talking two hands (boys).fbx", dict(B, loop=True, mirror=True)),
    "agree": (C + "Agreeing.fbx", dict(B, still=True)),
    "nod": (C + "Head Nod Yes.fbx", dict(B)),
    "point": (C + "Pointing Gesture.fbx", dict(B)),
    # Scene II.
    "reach": (C + "Reaching Out.fbx", dict(B)),
    "wave": (C + "Waving.fbx", dict(B, loop=True)),
    "kneel": (C + "Kneeling.fbx", dict(B, loop=True)),
}

# The old gatekeeper: stooped and slow; his staff is the `blade` (upright in his right fist, the game
# keeps it so); his free arm the left.
STAFF = dict(weapon_rest=(0.0, 1.0, 0.0), arms_out=6, face=0.0)
OLDMAN = {
    "sit_idle": (C + "Sitting Idle.fbx", dict(STAFF, loop=True)),
    "sit_talk": (C + "Sitting Talking.fbx", dict(STAFF, loop=True, span=(0.0, 12.0))),
    "sit_talk_2": (C + "talking seated (old man).fbx", dict(STAFF, loop=True)),
    # His free hand out (to the dog, to the door): Mixamo's points with the right, his is the left.
    "sit_point": (C + "Pointing.fbx", dict(STAFF, mirror=True)),
    "sit_to_stand": (C + "Sit To Stand.fbx", dict(STAFF, still=True)),
    "nod": (C + "Thoughtful Head Nod.fbx", dict(STAFF)),
    "idle": (C + "Idle.fbx", dict(STAFF, loop=True)),
    "walk": ("melee-axe/unarmed walk forward.fbx", dict(STAFF, loop=True, stride="feet", slow=1.4)),
    "wave": (C + "Waving.fbx", dict(STAFF, loop=True, mirror=True)),
}

CAST = {"kid": KID, "oldman": OLDMAN}


def label_of(path):
    return os.path.splitext(os.path.basename(path))[0].lower().replace(" ", "_").replace("(", "").replace(")", "")


def main():
    a = vh.args()
    if len(a) < 3:
        sys.exit(__doc__)
    who, root, out, rest = a[0], a[1], a[2], a[3:]
    if who not in CAST:
        sys.exit(f"mixamo_cast: no {who} ({', '.join(CAST)})")
    tgt = vh.armature()
    if tgt.animation_data is None:
        tgt.animation_data_create()
    if rest and rest[0] == "--try":
        for item in rest[1:]:
            path, _, extra = item.partition(":")
            opts = eval(f"dict({extra})") if extra else {}
            mh.make(label_of(path), os.path.join(root, path), opts, tgt, prefix="mixamo_cast")
    else:
        only = set(rest)
        for slot, (path, opts) in CAST[who].items():
            if only and slot not in only:
                continue
            mh.make(slot, os.path.join(root, path), opts, tgt, prefix="mixamo_cast")
    bpy.context.scene.render.fps = vh.FPS
    tgt.animation_data.action = bpy.data.actions.get("idle") or bpy.data.actions.get("sit_idle")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("mixamo_cast: saved", out)


if __name__ == "__main__":
    main()
