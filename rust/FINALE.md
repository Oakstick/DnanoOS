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

**Built (finale-end).** `Game::begin_ending()` (`game/ending.rs`) plays **Scene II, "The Way Up"**,
in the story's own player: `logic::story` now has two scenes (`Scene::One`, `Scene::Two`, each
with its beats; `Story` and `StoryView` carry which), so Scene II is staged, timed, captioned and
skipped exactly as Scene I is. Its beats, all action (`SCENE_TWO`, times in `story::ascent`):

| Beat | Seconds | What happens |
|---|---|---|
| Embrace | 8 | up out of black at the bottom of the Pit; the boy (red bobble cap) runs from the broken cage across the snow, the hero kneels, they hold each other (card: THE BOTTOM OF THE PIT) |
| Light | 7 | the shaft seen from below, its levels stacked up into a light that comes down it; the first chord of the hymn at 1.6 s, the light on them at 3 s; they look up; he stands, the boy's hand in his |
| Rise | 15 | the platform rises through the seven levels (cells empty, lamps lit warm), from above and then from below them; the white floods in from 10.8 s |
| White | 3 | nothing but white |
| Words | 34 | the poem of §4, exactly, line by line on the white turning to cream; held, then fading down into the dark |
| Credits | 44 | the roll (title; the three warriors; the old man and Quitmer; and the friend who was taken; the Pit and Krampus; the seven princes; the archangels, "who sent help"; Made with Claude), then THE END; then the title |

Esc skips (to the title, the hymn let go over a second and a half); Enter during the credits lets
them go. The save's `vh.finished` remembers the game was finished (`Game::finished`, read at
start; the Map's Scene II stop does not set it); the title shows a gold diamond and THE TALE IS
TOLD in its top-left corner once it is.

**The set** (`render/view/ascent.rs`) stands on its own at (-240, 0, -40), so it needs neither the
Pit's nor the lair's code: a frozen hollow at the bottom (black stone, drifted snow, icicles, the
fallen cage burst open with its door in the snow and its chain coiled by it, still bells on hanging
chains, Krampus's sack, basket and birch switches, a wooden horse, a split drum, a doll), then
seven levels six units apart, each a 25-wide cell round an 11-wide hole: a ledge of flagstones,
stone pillars and lintels, barred cells with a straw pallet and a lamp lit warm in each, an iron
door, and the level's number carved huge on the north wall. The platform (11 by 11, dressed
stone, its edge glowing faintly warmer as it rises) carries the long table with the feast picked
to the bones and the cup on its side. The hero (whichever of the three is played) has laid his
weapon on the stone; he kneels with his legs folded under him and holds the boy (`Act::Hug`,
`Act::Gaze`, `Act::HandUp` are new for the boy). The light is the shaft's sky going white far up
(the fog fades the upper levels into it), a soft column of glow and faint rays coming down, the
sun turning from cold to white on them, and motes of light; the cold blue of the bottom gives way
to it.

**The hymn** (`audio/hymn.rs`): 21 bars of 4.8 s in D major (D, G/D, Bm, Gadd9, Em7, Asus4, A),
a choir of five parts, each two singers a little apart in pitch and vibrato through the two
formants of "oo" or "ah", a pure tone under each, a breath of high air over them, slow to swell
(1.5 s) and slow to die (2.8 s; the last chord 7 s), and high celesta-like bells. It swells through
the rise, opens into the white on bar 4 (a falling cascade of bells), lies softly under the words
on "oo", sings under the credits on "ah". The game sounds it bar by bar (`ending::bar_between`), so
Esc releases it (`Synth::release_group`); the game's music is hushed for it. At most about 90
voices sound at once, inside the mix's budget.

**Satan is cut** (§5): Scene I's narrator now ends "The trail led past the House of the Devil,
which no one has ever seen inside... for Krampus takes the children he steals to the bottom of the
world" (the beats and timing unchanged); `ROUTE`, the objective and the module's words follow it;
the old man's Garden page says the same; the hub page no longer says Wrath was Satan's or that
Krampus waits with his father (no one has seen inside the House; the trail runs through it, down
to the bottom of the world); Wrath's page says Amon keeps it; the tour's stop and the banner at the
House's door lose "his father". A test greps the story's and the guide's sources and everything
they show for the name and for "his father".

**Seams and shared files.** `begin_ending()` is the seam; `lair_won()` should call it once the
child has reached the hero. Touched outside my own files: `logic/story.rs` (Scene II, the cut),
`render/view/story.rs` (`StoryView::scene`, the boy's three new acts, Place::Pit routed to
`ascent`), `game/story.rs` (both scenes), `render/hud/story.rs` (the letterbox names the scene;
Scene II's overlay), `game.rs` (`mod ending`, the `finished` field), `game/flow.rs` (the view's
scene), `game/map.rs` (a stop's scene is now `(Scene, beat)`; SCENE II - THE WAY UP added last),
`game/tests.rs` (the two map tests: the stop count, and the scene stops), `render/view.rs` (`mod ascent`, the set), `render/hud.rs`
(`mod ending`, the title's mark), `audio/mod.rs` (`mod hymn`), `audio/synth.rs`
(`release_group`), `capture.rs` (`ending` / `ending:T`, after the last command), `logic/guide.rs`,
`game/hub.rs` (the House banner's sub-line: the Pit builder may rewrite that banner), `game/tour.rs`.

**Captures:** `--script "0.1:ending:T" --time 0.2` shows Scene II T seconds in (0 the embrace, 8
the light, 15 the rise, 30 the white, 33 the words, 67 the credits).
