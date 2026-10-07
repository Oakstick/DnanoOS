"""The realms' own keyed bones (`mixamo_foe.py` lays them under their Mixamo clips, `lay`): the
ganglöt's staff arm. The zombie's arms reach and sway; its staff fist keeps the staff upright, planted
with its steps as an old man's walking stick (the code swings it a little with each step), and the
lantern on its crook hangs steady.
"""
import math

from block_foe import TAU, E, Pose


class GanglotStaff:
    """The staff arm (`upper_arm.L`, `forearm.L`, `hand.L` and the staff, `offhand`) as sculpted,
    swinging a little with its gait."""

    def held(self, swing):
        p = Pose()
        p.turn("upper_arm.L", E(-swing, 0, 0))
        p.turn("forearm.L", E(-0.4 * swing, 0, 0))
        # (The fist keeps the staff upright as the arm swings.)
        p.turn("hand.L", E(1.4 * swing, 0, 0))
        return p

    def idle(self, f):
        return self.held(1.5 * math.sin(TAU * f / 240))

    def walk(self, f):
        # (One swing a clip: the clip is one cycle of the shuffle.)
        return self.held(7.0 * math.sin(TAU * f / 1000))


STAFF = ["upper_arm.L", "forearm.L", "hand.L", "offhand"]
FOES = {}
