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

MU = "mutant/"
MG = "magic/"
GS = "great-sword/great sword "

# ------------------------------------------------------------------ the helmeted skeleton

# His spiked mace hangs head down from his bony fist (his own left: mirrored).
HELM = dict(mirror=True, weapon="offhand", weapon_rest=(0.04, -0.997, 0.06))
# Mixamo's melee axe pack: a soldier of the dead, square and heavy.
HELMETED = {
    "idle": (AX + "idle.fbx", dict(HELM, loop=True)),
    "walk": (AX + "walk forward.fbx", dict(HELM, loop=True, stride="feet")),
    "run": (AX + "run forward.fbx", dict(HELM, loop=True, stride="feet")),
    # The slam: the leap and the mace brought down on him with both hands, struck where he stands
    # (its run-up cut away).
    "slam": (AX + "melee run jump attack.fbx", dict(HELM, kind="strike", still=True, span=(0.37, 3.45))),
    "sweep": (AX + "melee attack horizontal.fbx", dict(HELM, kind="strike")),
    "rally": (AX + "taunt battlecry.fbx", dict(HELM, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (AX + "react large gut.fbx", dict(HELM, kind="flinch")),
    "stagger": (AX + "react large from left.fbx", dict(HELM, kind="flinch")),
}

# ------------------------------------------------------------------ the blemmyes

# His knotted club hangs down and forward from his fist (his own left: mirrored).
BLEM = dict(mirror=True, weapon="offhand", weapon_rest=(0.06, -0.96, 0.26))
# Mixamo's creature pack: a hunched brute, his face in his chest.
BLEMMYES = {
    "idle": (MU + "mutant breathing idle.fbx", dict(BLEM, loop=True)),
    "walk": (MU + "mutant walking.fbx", dict(BLEM, loop=True, stride="feet")),
    "run": (MU + "mutant run.fbx", dict(BLEM, loop=True, stride="feet")),
    # The leap and the club brought down, its spins taken out.
    "slam": (MU + "mutant jump attack.fbx", dict(BLEM, kind="strike", still=True, yaw_keep=0.0, face=0.0)),
    "sweep": (MU + "mutant swiping.fbx", dict(BLEM, kind="strike")),
    "rally": (MU + "mutant roaring.fbx", dict(BLEM, kind="strike", weapon="chest", tip_rest=(0.0, 1.0, 0.3))),
    "hit": (AX + "react large from right.fbx", dict(BLEM, kind="flinch")),
    "stagger": (MG + "Standing React Large From Left.fbx", dict(BLEM, kind="flinch")),
}

# ------------------------------------------------------------------ the rasetsu

# A curved blade in each fist, hanging down and out: neither mirrored, each held as the source's
# own fist holds it.
RAS = dict(weapon="offhand", weapon_rest={"offhand": (0.49, -0.86, 0.09), "blade": (-0.49, -0.86, 0.09)})
RASETSU = {
    "idle": (MU + "mutant idle.fbx", dict(RAS, loop=True)),
    "walk": (MG + "Standing Walk Forward.fbx", dict(RAS, loop=True, stride="feet")),
    "run": ("melee-axe/unarmed run forward.fbx", dict(RAS, loop=True, stride="feet")),
    # The twin blades: the first cuts of the dual weapon combo, its whirl mostly taken out.
    "slam": (FX + "dual weapon combo.fbx", dict(RAS, kind="strike", yaw_keep=0.3, cut=110)),
    "sweep": (AX + "melee attack 360 low.fbx", dict(RAS, kind="strike", yaw_keep=0.0, face=0.0)),
    "rally": (MU + "mutant flexing muscles.fbx", dict(RAS, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (MG + "Standing React Small From Left.fbx", dict(RAS, kind="flinch")),
    "stagger": (MG + "Standing React Large From Back.fbx", dict(RAS, kind="flinch")),
}

# ------------------------------------------------------------------ the tengu

# His katana, held low in his fist (his own left: mirrored), down and forward past his foot. The
# great sword pack, picks none of the heroes uses: one-handed where it can, the other hand free.
TENG = dict(mirror=True, weapon="offhand", weapon_rest=(0.14, -0.84, 0.52))
TENGU = {
    "idle": (GS + "idle (4).fbx", dict(TENG, loop=True)),
    "walk": ("melee-axe/unarmed walk forward.fbx", dict(TENG, loop=True, stride="feet")),
    "run": (GS + "run (2).fbx", dict(TENG, loop=True, stride="feet")),
    "slam": (GS + "slash (4).fbx", dict(TENG, kind="strike")),
    "sweep": (GS + "slash (5).fbx", dict(TENG, kind="strike", yaw_keep=0.0, face=0.0)),
    "rally": (GS + "power up.fbx", dict(TENG, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (GS + "impact (3).fbx", dict(TENG, kind="flinch")),
    "stagger": (GS + "impact (4).fbx", dict(TENG, kind="flinch")),
}

# ------------------------------------------------------------------ the witch

# Her staff, upright in her fist (her own left: mirrored). Mixamo's magic pack; she creeps hunched.
WITCH_W = dict(mirror=True, weapon="offhand", weapon_rest=(0.0, 1.0, 0.0))
WITCH = {
    "idle": (MG + "standing idle 02.fbx", dict(WITCH_W, loop=True)),
    "walk": (MG + "Crouch Walk Forward.fbx", dict(WITCH_W, loop=True, stride="feet")),
    "run": (MG + "Standing Run Forward.fbx", dict(WITCH_W, loop=True, stride="feet")),
    "hex": (MG + "Standing 2H Magic Attack 01.fbx", dict(WITCH_W, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "rally": (MG + "Standing 2H Magic Area Attack 01.fbx", dict(WITCH_W, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "hit": (MG + "Standing React Small From Right.fbx", dict(WITCH_W, kind="flinch")),
    "stagger": (MG + "Standing React Large From Right.fbx", dict(WITCH_W, kind="flinch")),
}

# ------------------------------------------------------------------ the oni

# His kanabo hangs head down from his fist (his own left: mirrored). A brute twice a man's height:
# the creature pack's lumber, the great sword's spinning smash brought down where he stands.
ONI_W = dict(mirror=True, weapon="offhand", weapon_rest=(0.06, -1.0, 0.18))
ONI = {
    "idle": (MU + "mutant idle (2).fbx", dict(ONI_W, loop=True)),
    "walk": (MU + "mutant walking.fbx", dict(ONI_W, loop=True, stride="feet")),
    "run": (MU + "mutant run.fbx", dict(ONI_W, loop=True, stride="feet")),
    # The kanabo: whirled up over his head and brought down (its spin taken out).
    "kanabo": (GS + "high spin attack.fbx", dict(ONI_W, kind="strike", yaw_keep=0.0, face=0.0)),
    "sweep": (AX + "melee attack 360 high.fbx", dict(ONI_W, kind="strike", yaw_keep=0.0, face=0.0)),
    "rally": (AX + "taunt chest thump.fbx", dict(ONI_W, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (AX + "react large from right.fbx", dict(ONI_W, kind="flinch")),
    "stagger": (MU + "mutant dying.fbx", dict(ONI_W, kind="flinch", cut=70)),
}

# ------------------------------------------------------------------ the fifth stage

# The albasti: her own moves are keyed (`block_stage5.Albasti`: the smother); her stand, creep and
# reels from the magic pack. Her arms hang empty.
ALB = dict(face=0.0)
ALBASTI = {
    "idle": (MG + "Standing Idle 04.fbx", dict(ALB, loop=True)),
    "walk": (MG + "Crouch Walk Forward.fbx", dict(ALB, loop=True, stride="feet")),
    "run": (MG + "Standing Sprint Forward.fbx", dict(ALB, loop=True, stride="feet")),
    "hit": (MG + "Standing React Small From Back.fbx", dict(ALB, kind="flinch")),
    "stagger": (MG + "Standing React Large From Front.fbx", dict(ALB, kind="flinch")),
}
# The cin: his crooked staff in his right fist (not mirrored), its fire flung from it.
CIN_W = dict(weapon="blade", weapon_rest=(0.0, 1.0, 0.0))
CIN = {
    "idle": (MG + "Standing Idle 03.fbx", dict(CIN_W, loop=True, span=(0.0, 4.0))),
    "walk": (MG + "Standing Walk Forward.fbx", dict(CIN_W, loop=True, stride="feet")),
    "run": (MG + "Standing Run Forward.fbx", dict(CIN_W, loop=True, stride="feet")),
    "smokeless": (MG + "Standing 1H Magic Attack 02.fbx", dict(CIN_W, kind="strike", weapon="hand.R", tip_rest=(0, 0, 1))),
    "rally": (MG + "Standing 2H Cast Spell 01.fbx", dict(CIN_W, kind="strike", weapon="hand.R", tip_rest=(0, 0, 1))),
    "hit": (MG + "Standing React Small From Left.fbx", dict(CIN_W, kind="flinch")),
    "stagger": (MG + "Standing React Large From Left.fbx", dict(CIN_W, kind="flinch")),
}
# The arcura: its roots called up with both arms raised (the growth is the code's), on goat's legs.
ARCURA = {
    "idle": (MG + "Standing Idle 03.fbx", dict(loop=True, span=(4.0, 8.0))),
    "walk": (MU + "mutant walking.fbx", dict(loop=True, stride="feet")),
    "run": (MU + "mutant run.fbx", dict(loop=True, stride="feet")),
    "roots": (MG + "Standing 2H Magic Area Attack 02.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, 1, 0))),
    "rally": (MU + "mutant roaring.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (MG + "Standing React Large From Front.fbx", dict(kind="flinch")),
    "stagger": (MG + "Standing React Large From Back.fbx", dict(kind="flinch")),
}
# The karakoncolos: an ape's lumber and its blows, its call the creature's roar, head thrown back.
KARAKONCOLOS = {
    "idle": (MU + "mutant idle.fbx", dict(loop=True, span=(0.0, 5.0))),
    "walk": (MU + "mutant walking.fbx", dict(loop=True, stride="feet")),
    "run": (MU + "mutant run.fbx", dict(loop=True, stride="feet")),
    "call": (MU + "mutant roaring.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "slam": (MU + "jump attack.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0), still=True, yaw_keep=0.0, face=0.0)),
    "sweep": (MU + "mutant punch.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0))),
    "hit": (MU + "mutant dying.fbx", dict(kind="flinch", cut=40)),
    "stagger": (AX + "react large from left.fbx", dict(kind="flinch")),
}

# ------------------------------------------------------------------ the sixth stage

# Tepegoz: the creature pack slowed for a giant (half again as long), his feet planted.
TEPEGOZ = {
    "idle": (MU + "mutant breathing idle.fbx", dict(loop=True, slow=1.5)),
    "walk": (MU + "mutant walking.fbx", dict(loop=True, stride="feet", slow=1.5)),
    "slam": (MU + "mutant jump attack.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0), still=True, yaw_keep=0.0, face=0.0, slow=1.3)),
    "sweep": (MU + "mutant swiping.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0), slow=1.3)),
    "rally": (MU + "mutant roaring.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4), slow=1.3)),
    "hit": (AX + "react large gut.fbx", dict(kind="flinch", slow=1.4)),
    "stagger": (AX + "react large from right.fbx", dict(kind="flinch", slow=1.4)),
}
# The gashadokuro: the zombie's two-handed seizing for its grab, its jaw keyed (`block_stage6`).
GJAW = ("block_stage6", "GashaJaw")
GASHADOKURO = {
    "idle": (Z + "idle.fbx", dict(loop=True, slow=1.5, lay=GJAW + ("idle", ["jaw"], 120, True))),
    "walk": (Z + "walk.fbx", dict(loop=True, stride="feet", slow=1.5, lay=GJAW + ("idle", ["jaw"], 120, True))),
    "grab": (Z + "attack.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, 0, 1), slow=1.2, lay=GJAW + ("grab", ["jaw"], 92, False))),
    "slam": (MU + "jump attack.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, 0, 1), still=True, yaw_keep=0.0, face=0.0, slow=1.3,
                                          lay=GJAW + ("slam", ["jaw"], 96, False))),
    "sweep": (AX + "melee attack 360 low.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, 0, 1), yaw_keep=0.0, face=0.0, slow=1.3,
                                                    lay=GJAW + ("slam", ["jaw"], 96, False))),
    "rally": (Z + "scream.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 0.0, 1.0), slow=1.3, lay=GJAW + ("rally", ["jaw"], 77, False))),
    "hit": (FX + "zombie reaction hit flinch.fbx", dict(kind="flinch", cut=60, slow=1.3)),
    "stagger": (FX + "zombie reaction hit stumble back.fbx", dict(kind="flinch", cut=100, slow=1.3)),
}
# The goryo: a ghost, its arms held out before it (the magic pack's guard), the storm called down
# with both hands.
GORYO = {
    "idle": (MG + "Standing Block Idle.fbx", dict(loop=True)),
    "walk": (MG + "Standing Block Idle.fbx", dict(loop=True, stride=1.0)),
    "thunder": (MG + "Standing 2H Magic Attack 03.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "rally": (MG + "Standing 2H Magic Area Attack 02.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "hit": (MG + "Standing React Small From Front.fbx", dict(kind="flinch")),
    "stagger": (MG + "Standing React Large From Back.fbx", dict(kind="flinch")),
}

# ------------------------------------------------------------------ the seventh stage

# The frost giant: the creature pack slowed for a giant; his frost slam both fists brought down.
FROST_GIANT = {
    "idle": (MU + "mutant idle (2).fbx", dict(loop=True, slow=1.5)),
    "walk": (MU + "mutant walking.fbx", dict(loop=True, stride="feet", slow=1.5)),
    "frost_slam": (MU + "mutant jump attack.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0), still=True, yaw_keep=0.0, face=0.0, slow=1.3)),
    "slam": (MU + "jump attack.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0), still=True, yaw_keep=0.0, face=0.0, slow=1.3)),
    "sweep": (MU + "mutant swiping.fbx", dict(kind="strike", weapon="hand.L", tip_rest=(0, -1, 0), slow=1.3)),
    "rally": (MU + "mutant roaring.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4), slow=1.3)),
    "hit": (AX + "react large gut.fbx", dict(kind="flinch", slow=1.4)),
    "stagger": (AX + "react large from left.fbx", dict(kind="flinch", slow=1.4)),
}
# Surtr: his flaming sword (his own left: mirrored) swung as the great sword pack swings, slowed,
# picks none of the heroes uses.
SUR = dict(mirror=True, weapon="offhand", weapon_rest=(0.1, -0.87, 0.48))
SURTR = {
    "idle": (GS + "idle (3).fbx", dict(SUR, loop=True, slow=1.5, span=(0.0, 3.0))),
    "walk": (AX + "walk forward.fbx", dict(SUR, loop=True, stride="feet", slow=1.5)),
    "slam": (GS + "attack.fbx", dict(SUR, kind="strike", slow=1.4)),
    "sweep": (AX + "melee attack backhand.fbx", dict(SUR, kind="strike", slow=1.3)),
    "rally": (GS + "casting.fbx", dict(SUR, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4), slow=1.3)),
    "hit": (GS + "impact (5).fbx", dict(SUR, kind="flinch", slow=1.4)),
    "stagger": (MU + "mutant dying.fbx", dict(SUR, kind="flinch", cut=70, slow=1.3)),
}
# Hel: the queen of the dead; her staff (her own left: mirrored) raised to call them up, her bolt
# flung from it.
HEL_W = dict(mirror=True, weapon="offhand", weapon_rest=(0.0, 1.0, 0.0))
HEL = {
    "idle": (MG + "Standing Idle 03.fbx", dict(HEL_W, loop=True, span=(8.0, 12.0))),
    "walk": (MG + "Standing Walk Forward.fbx", dict(HEL_W, loop=True, stride="feet")),
    "raise": (MG + "Standing 2H Magic Area Attack 01.fbx", dict(HEL_W, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "hex": (MG + "Standing 1H Magic Attack 03.fbx", dict(HEL_W, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "rally": (MG + "Standing 2H Cast Spell 01.fbx", dict(HEL_W, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "hit": (MG + "Standing React Small From Right.fbx", dict(HEL_W, kind="flinch")),
    "stagger": (MG + "Standing React Large From Back.fbx", dict(HEL_W, kind="flinch")),
}

# ------------------------------------------------------------------ the realms' own

# The ganglot: Hel's slow servant, the zombie's shuffle slowed; its staff (its own left: mirrored),
# upright in its fist with the lantern on its crook, held as sculpted while it stands, shuffles and
# reels (`block_realms.GanglotStaff` laid over the zombie's arm), swung as the axe pack swings for
# its blows.
GANG = dict(mirror=True, weapon="offhand", weapon_rest=(0.0, 1.0, 0.0))
GSTAFF = ("block_realms", "GanglotStaff")
GSTAFF_BONES = ["upper_arm.L", "forearm.L", "hand.L", "offhand"]
GANGLOT = {
    "idle": (Z + "idle.fbx", dict(GANG, loop=True, slow=1.3, lay=GSTAFF + ("idle", GSTAFF_BONES, 240, True))),
    "walk": (Z + "walk.fbx", dict(GANG, loop=True, stride="feet", slow=1.4, lay=GSTAFF + ("walk", GSTAFF_BONES, 1000, True))),
    "slam": (AX + "melee attack downward.fbx", dict(GANG, kind="strike", slow=1.2)),
    "sweep": (AX + "melee attack horizontal.fbx", dict(GANG, kind="strike", slow=1.2)),
    "rally": (Z + "scream.fbx", dict(GANG, kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4), lay=GSTAFF + ("idle", GSTAFF_BONES, 240, False))),
    "hit": (FX + "zombie reaction hit flinch.fbx", dict(GANG, kind="flinch", cut=60, lay=GSTAFF + ("idle", GSTAFF_BONES, 240, False))),
    "stagger": (FX + "zombie reaction hit stumble back.fbx", dict(GANG, kind="flinch", cut=100, lay=GSTAFF + ("idle", GSTAFF_BONES, 240, False))),
}
# The gaki: hungry ghosts, the zombie's shamble and its lunge to bite, their arms reaching.
GAKI = {
    "idle": (Z + "idle.fbx", dict(loop=True)),
    "walk": (Z + "walk.fbx", dict(loop=True, stride="feet")),
    "run": (Z + "run.fbx", dict(loop=True, stride="feet")),
    "bite": (Z + "attack.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 0.0, 1.0))),
    "rally": (MU + "mutant roaring.fbx", dict(kind="strike", weapon="head", tip_rest=(0.0, 1.0, 0.4))),
    "hit": (FX + "zombie reaction hit flinch.fbx", dict(kind="flinch", cut=60)),
    "stagger": (FX + "zombie reaction hit stumble back.fbx", dict(kind="flinch", cut=100)),
}
# The ellepige: an elf-maid, light on her feet; her blow a quick spell thrown from her hand.
ELLE = dict(face=0.0)
ELLEPIGE = {
    "idle": (MG + "standing idle.fbx", dict(ELLE, loop=True)),
    "walk": (MG + "Standing Walk Forward.fbx", dict(ELLE, loop=True, stride="feet")),
    "run": (MG + "Standing Run Forward.fbx", dict(ELLE, loop=True, stride="feet")),
    "bite": (MG + "Standing 1H Magic Attack 01.fbx", dict(ELLE, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "rally": (MG + "standing 1H cast spell 01.fbx", dict(ELLE, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1))),
    "hit": (MG + "Standing React Small From Front.fbx", dict(ELLE, kind="flinch")),
    "stagger": (MG + "Standing React Large From Right.fbx", dict(ELLE, kind="flinch")),
}

# ------------------------------------------------------------------ the jorogumo

MG = "magic/"
# Her upper body from Mixamo's magic pack, faced to the foe whatever the source's turning (her
# spider's body steers her); the sickle in her right fist held as the source's fist holds its grip
# (it rises from her fist, leaning forward). Her legs are keyed (`block_foe.Jorogumo`, `legs`) and
# laid in under each clip, fitted to its markers.
JORO = dict(yaw_keep=0.0, face=0.0, weapon_rest=(0.0, 0.975, 0.22))
JOROGUMO = {
    "idle": (MG + "standing idle.fbx", dict(JORO, loop=True, legs="idle")),
    # (Her walk is her legs': above them she stands as she does, poised.)
    "walk": (MG + "standing idle.fbx", dict(JORO, loop=True, legs="walk")),
    # The fire flung from her sickle hand: the sickle thrust out at him as it leaves.
    "hex": (MG + "Standing 1H Magic Attack 01.fbx", dict(JORO, kind="strike", weapon="hand.R", tip_rest=(0, 0, 1), legs="hex")),
    "rally": (MG + "Standing 2H Cast Spell 01.fbx", dict(JORO, kind="strike", weapon="hand.L", tip_rest=(0, 0, 1), legs="rally")),
    "hit": (MG + "Standing React Small From Front.fbx", dict(JORO, kind="flinch", legs="hit")),
    "stagger": (MG + "Standing React Large From Front.fbx", dict(JORO, kind="flinch", legs="stagger")),
}

FOES = {"draugr": DRAUGR, "helmeted": HELMETED, "blemmyes": BLEMMYES, "rasetsu": RASETSU, "tengu": TENGU, "witch": WITCH, "jorogumo": JOROGUMO, "oni": ONI,
        "albasti": ALBASTI, "cin": CIN, "arcura": ARCURA, "karakoncolos": KARAKONCOLOS,
        "tepegoz": TEPEGOZ, "gashadokuro": GASHADOKURO, "goryo": GORYO,
        "frost_giant": FROST_GIANT, "surtr": SURTR, "hel": HEL,
        "ganglot": GANGLOT, "gaki": GAKI, "ellepige": ELLEPIGE}

# The bones her legs' clips key (her spider's body); the rest are Mixamo's.
SPIDER = ("carapace", "abdomen") + tuple(f"leg{i}.{k}" for i in range(8) for k in "abc")


# The recovery each move has in the game at its base tempo (frames: logic/combat/frames.rs), and
# the longest a clip's follow-through is kept against it: the rest of a long Mixamo settle is cut
# off (the game would play it several times too fast), and the foe's feet take over in a crossfade.
RECOVERY = {"slam": 48, "sweep": 36, "hex": 30, "rally": 36, "bite": 24, "kanabo": 52, "smokeless": 30, "roots": 30, "call": 12,
            "grab": 44, "thunder": 32, "frost_slam": 44, "raise": 40}
TAIL = 2.2
# The moves' frame data at the base tempo (startup, active, recovery), for fitting keyed bones to a
# clip's markers.
RECOVERY_FD = {"slam": (42, 6, 48), "sweep": (30, 8, 36), "grab": (40, 8, 44), "thunder": (40, 1, 32), "rally": (40, 1, 36)}
# The longest a reel's clip runs past its impact (a reel lasts 14 to 50 frames in the game).
REEL_TAIL = 40


def shorten(slot, act):
    """Cuts a long follow-through or settle off a move or a reel, its markers kept in order."""
    marks = {m.name: m for m in act.pose_markers}
    n = int(act.frame_range[1])
    if slot in RECOVERY and "active_end" in marks:
        end = int(marks["active_end"].frame + RECOVERY[slot] * TAIL)
    elif "impact" in marks and slot in ("hit", "stagger"):
        end = int(marks["impact"].frame + REEL_TAIL * (1.6 if slot == "stagger" else 1.0))
    else:
        return
    if end >= n:
        return
    mh.trim(act, 0, end)
    marks["recover_end"].frame = end
    print(f"mixamo_foe: {slot}: cut to {end} frames (from {n})")


def lay_legs(tgt, act, which):
    """The jorogumo's keyed legs (`block_foe.Jorogumo.<which>`) laid under `act` (`lay`)."""
    import block_foe as bf

    keyed = {"idle": (120, True), "walk": (40, True), "hex": (sum(bf.HEX), False), "rally": (sum(bf.RALLY), False),
             "hit": (24, False), "stagger": (48, False)}
    n_k, loop = keyed[which]
    lay(tgt, act, bf.Jorogumo(), which, SPIDER, n_k, loop, {"hex": bf.HEX, "rally": bf.RALLY}.get(which), stride_per_cycle=1.0 if which == "walk" else None)


def lay(tgt, act, keyed, which, bones, n_k, loop, fd=None, stride_per_cycle=None):
    """Lays bones keyed by script (`getattr(keyed, which)(frame)`, a pose of `n_k` frames) under
    Mixamo's clip `act`: a loop as many whole cycles of them as fit its length (its stride theirs
    times as many), a move or a reel fitted piecewise to its markers (the wind-up's end to the wind-up's
    end, and so on, where `fd` gives the move's frames), else stretched to its length. Only `bones`
    are keyed (their root's place too, if it is among them)."""
    import block_foe as bf

    n = int(act.frame_range[1])
    marks = {m.name: m.frame for m in act.pose_markers}
    if loop:
        cycles = max(1, round(n / n_k))
        at = lambda f: (f * n_k * cycles / n) % n_k  # noqa: E731
        if stride_per_cycle:
            act["vh_stride"] = stride_per_cycle * cycles
    elif fd and "windup_end" in marks:
        src = [0, marks["windup_end"], marks["active_end"], n]
        dst = [0, fd[0], fd[0] + fd[1], n_k]

        def at(f):
            for k in range(3):
                if f <= src[k + 1]:
                    u = (f - src[k]) / max(1e-6, src[k + 1] - src[k])
                    return dst[k] + (dst[k + 1] - dst[k]) * u
            return n_k
    else:
        at = lambda f: f * n_k / n  # noqa: E731
    root = vh.root_of(tgt)
    tgt.animation_data.action = act
    for f in range(n + 1):
        pose = getattr(keyed, which)(at(f))
        for b in bones:
            vh.set_local(tgt, b, pose.get(b, bf.Quaternion()), pose.get("loc", (0, 0, 0)) if b == root else None)
            pb = tgt.pose.bones[b]
            pb.keyframe_insert("rotation_quaternion", frame=f)
            if b == root:
                pb.keyframe_insert("location", frame=f)
    for fc in vh.fcurves(act):
        for kp in fc.keyframe_points:
            kp.interpolation = "LINEAR"
    print(f"mixamo_foe: {act.name}: {', '.join(bones)} laid in from {type(keyed).__name__}.{which}")


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
            act = mh.make(slot, os.path.join(root, path), opts, tgt, prefix="mixamo_foe")
            shorten(slot, act)
            if opts.get("legs"):
                lay_legs(tgt, act, opts["legs"])
            if opts.get("lay"):
                # (Bones the source has none of, keyed by script: `(module, class, method, bones, frames, loop)`.)
                mod, cls, which, bones, n_k, loop = opts["lay"]
                keyed = getattr(__import__(mod), cls)()
                lay(tgt, act, keyed, which, bones, n_k, loop, RECOVERY_FD.get(slot))
        if (not only or "deflected" in only) and "stagger" in bpy.data.actions and "deflected" not in FOES[foe]:
            deflected(tgt)
    bpy.context.scene.render.fps = vh.FPS
    tgt.animation_data.action = bpy.data.actions.get("idle")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out))
    print("mixamo_foe: saved", out)


if __name__ == "__main__":
    main()
