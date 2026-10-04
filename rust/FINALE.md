# The finale: the Pit, Krampus, and the end

Phase 10 of ROADMAP.md, redesigned by the user on 2026-10-04: **there is no Devil.** The last level
is inspired by the Spanish film *El hoyo* (*The Platform*, 2019): a vertical prison where a stone
platform sinks one level at a time. The hero rides it down through seven levels, each holding the
foes of one stage, and at the bottom Krampus waits with the stolen friend in a cage. The hero
kills him after an epic fight, frees the child, and they rise together in white light to angelic
voices. Then the words: there was never a devil.

This replaces the old phase 10 (Krampus at the door, Satan in the volcano). Satan is cut from the
game and the story everywhere.

## 0. Rules it keeps

Everything in COMBAT.md §0 and PRINCES.md §0 still holds: every move readable (30 frames or more
of startup for Krampus, a stage's own numbers for its foes), punishable, no two landings within 18
frames (the threat manager plans them), nothing slows or holds the hero, determinism in the sim.
"Many enemies" means many on the floor, not many striking at once: the threat manager's tokens
keep the fight readable however many stand round him. The old man always leads to good.

## 1. The way in

With all seven keys the causeways rise and the door of the House of the Devil opens (already
built). Inside there is no throne and no devil: a square shaft drops into the dark, and a stone
platform waits at its lip, a long table on it set with a feast. A banner: **THERE IS NO DEVIL HERE.
ONLY THE WAY DOWN.** The hero jumps onto the platform, and it begins to sink.

## 2. The Pit

**One level of the Pit.** A square stone cell about 25 tiles across with a square hole in its
middle, and the platform (about 11 by 11) filling the hole exactly. The cell's floor is a ledge
round the platform, five or six tiles wide; its walls are rough concrete-grey stone with the
level's number carved huge on one wall, iron doors and barred cells in them, from which the
level's inmates come. Above and below, the shaft: the same cells stacked, dark, seen through the
hole as the platform sinks.

**Seven levels, seven stages.** Level *n* (1 to 7) holds the foes of stage *n*
(`EnemyKind::of_stage(n)`), many of them: about 20 to 30 over the level, coming out of the cells
in waves (four or so), never more than about ten standing at once, in packs with roles as the
Warband's packs are (WARBAND.md). Each level's light and colour follows its stage.

**The timer.** Every level lasts **two minutes**, shown large at the top of the screen with the
level's number. The waves come over the first hundred seconds or so; the last twenty are for
clearing the platform. In the last fifteen seconds a bell tolls each second, the platform's edge
glows, and a banner says **THE PLATFORM DESCENDS. CLEAR IT.**

**What rides down.** At zero the platform sinks to the next level, and **every foe standing on the
platform goes down with it**, to join the next level's inmates (a floater counts them: "4 RIDE
DOWN WITH YOU"). What stands on the ledge stays behind. The hero always goes down: if he stands on
the ledge as it sinks, he leaps down after it and lands hurt (a tenth of his health), never
stranded.

**The descent.** About five seconds: the cell's walls slide up past the camera, the shaft's dark
with the numbers of the levels passing, the light turning toward the next stage's colour; the
fight is still (no one strikes while the platform moves). The feast on the platform is picked
cleaner at every level, down to bones and an overturned cup at the seventh (the film's nod).

**Falls and checkpoints.** A green cross on the platform as each level begins. Each level is a
checkpoint: if the hero falls, he begins that level again, its riders as they came down with him.

## 3. The bottom: Krampus's lair

After the seventh level the platform sinks once more and comes to rest at the bottom of the Pit:
a frozen hollow under the earth, its floor dusted with snow over black stone, icicles from the
dark above, iron chains hanging with bells on them, bundles of birch switches, his sacks and
basket, broken toys in the snow. At its north end, an iron cage hangs from a great chain: in it,
the friend Krampus took at the graveyard (the boy with the red bobble cap and scarf, from Scene
I's cast).

**Krampus** (Alpine folklore: the horned companion of St Nicholas who punishes children on
Krampusnacht, 5 December; black fur, goat's horns and hooves, a long red tongue, chains and bells,
birch switches, a basket on his back to carry children off) fights as the last boss: a fighter on
the Prince archetype, larger and longer than any prince, with his own sculpt built from Scene I's
Krampus (his look is fixed by the cutscene). Proposal for his fight (the builder tunes it):

| Phase | Title | Moves | What else |
|---|---|---|---|
| 1 (100-70%) | THE BELLS | Birch Lash (a wide cone), Chain Sweep (a long sweep), Bell Toll (a quake round him) | |
| 2 (70-40%) | THE CHAINS | Chain Lash (a line, `Shape::Beam`), Hoof Charge (`Shape::Charge`, dazed on stone), Birch Lash | his Perchten come (two masked companions; existing kinds dressed as them, or his own) |
| 3 (40-0%) | KRAMPUSNACHT | Coal Fire (`TrapLine`, lingering), Sack Swing (a quake), Chain Lash, Hoof Charge | quicker; he goes to the cage to rattle it and taunt the child, and stands open while he does |

He has a hide like a prince's. His health and damage are measured for the Pit's end (all seven
keys held). His death is slow: the bells on his chains fall silent one by one.

## 4. The end

When Krampus falls, the cage's lock breaks and the cage comes down. The friend climbs out and runs
to the hero; the hero kneels, and they hold each other. Then (Scene II, in the story-cutscene
system with Scene I's cast and camera):

1. A white light falls down the shaft from far above. Angelic voices: a choir swelling (white,
   pure tones, bells), the first gentle sound in the game since the old man's door.
2. The platform rises with the two of them on it, up through the seven levels, every cell now
   empty and lit warm, into the white.
3. White, then the words, one line at a time:

   > There was never a devil.
   >
   > Only the dark we carry, and the light we choose.
   >
   > Every monster in these depths was made of us,
   > and so was every hand that reached down to lift another up.
   >
   > The good and the bad were never below.
   > They were always within.

4. The credits, then the title screen. The save remembers that the game was finished.

## 5. The story without Satan

Scene I's last line, the route and the old man's words no longer lead to Satan. The trail leads
down to the House of the Devil, which the old man says nobody has ever seen the inside of. Wrath's
prince no longer keeps the sin "for Satan". Krampus is simply Krampus, who takes children down to
the bottom of the world.

## 6. Build

Three parts built in parallel, each in its own files, joined at two seams:

| Part | Owns | Ends by calling |
|---|---|---|
| **The Pit** | `logic/pit.rs`, `game/pit.rs`, `render/view/pit.rs`, `render/hud/pit.rs`, `Theme::Pit`, `Stage::Pit(level)`, `GuidePlace::Pit(level)`, the House's door in the hub, `--pit` harness | `pit_reaches_bottom()` (wired to the lair) |
| **Krampus** | `logic/lair.rs`, `game/lair.rs`, `render/view/lair.rs`, `render/foes_lair.rs`, `Theme::Lair`, `Stage::Lair`, `GuidePlace::Lair`, `EnemyKind::Krampus` (and his Perchten), the cage and the freed child in play, `--lair` harness | `lair_won()` (wired to the ending) |
| **The end** | `game/ending.rs`, Scene II in `logic/story.rs` and `render/view/story.rs`, the choir, the words, the credits, the save's "finished", Satan cut from the story and the old man's words | |

## Build notes

### The Pit

(to come)

### Krampus

(to come)

### The end

(to come)
