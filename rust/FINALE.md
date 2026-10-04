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

Built on branch `finale-pit`. Files: `logic/pit.rs` (the layout, the levels, the rules as pure
functions), `game/pit.rs` (the rules in play), `render/view/pit.rs` (the art), `render/hud/pit.rs`
(the clock), and the bot's `pit_board` and `pit_ride` in `game/bot.rs`.

**The way in.** With all seven keys the House's door stands open (a banner says so as he comes up to
it); walking into it calls `enter_pit()`: the banner THERE IS NO DEVIL HERE. ONLY THE WAY DOWN., the
countdown, and he leaps from the lip onto the platform as play begins (`Hero::leap_to`, the same
arc as his own leap). Each level is a checkpoint (`Stage::Pit(n)`, saved as `p n`); Continue and the
map's seven stops (THE PIT - LEVEL n) load the level fresh, with nothing riding down, and every key
held (so `realm_tier()` is 8, and the packs' affixes are the deepest realm's).

**One level** (`logic::pit`): a cell 25 tiles across, walls included, the platform 11 by 11 in its
middle and a ledge six tiles wide round it; a long table of seven tiles across the platform's middle
is solid (rock in the grid), so the platform is two halves joined at the table's ends. Two iron doors
in each wall (eight), four barred cells in each (scenery). The grid is the same at every level, so a
descent changes no level: only the fight's state and the art's.

**The rules** (`game::pit`). The clock runs 120 seconds of play. Each level's foes are its own stage's
(`EnemyKind::of_stage(n)`, no witch's minions), 24 of them in four waves at 3, 26, 50 and 74
seconds, each wave out of two doors on facing walls, three to a door, dealt by turns so every pack
fights with its body, in melee and from afar. They fight as the realms' packs do (`raise_packs`,
`Field::Vault`, `logic::realms::weight`), awake from the first moment, each door's pack its own den
(so the Warband's packs, roles and affixes compose over them; `disband_packs` after each arrival
recomposes them). At most ten stand at once (riders included): a pack whose time has come waits
behind its door until there is room, and at 100 seconds the doors shut on whoever is left in. The
last fifteen seconds: a toll each second, the banner THE PLATFORM DESCENDS. CLEAR IT., the platform's
iron rim glowing with the bell, the screen's edge reddening, the HUD's count of who stands on it. At
zero who rides down is settled (every foe counted standing on the platform, `pit::on_platform`); the
fight stops (bolts, hazards and tokens cleared, every move broken off); a hero on the ledge leaps
after the platform and lands with a tenth of his health gone (`leap_cost`, never his last point).
Five seconds of descent hold the tick (`pit_holds`): nothing fights or walks. Then the ledge's foes
are gone, the riders stand where they stood, hunting, the next level's clock begins with its green
cross north of the table and the banner LEVEL n with its stage's name and how many rode down. A fall
begins the level again with its riders as they came (kind, place and health) and its inmates back
behind their doors. After the seventh level's descent, `pit_reaches_bottom()`: for now the Won screen
(THE BOTTOM OF THE PIT), and the title after it; at the merge it leads into the lair.

**The art** (`render::view::pit`). The level he stands on and the one above are static scenery (the
upper one's floor is this one's ceiling, its hole over the hole); the level below is a set the GPU
keeps. Each level's shell: concrete slabs with stains and cracks round an iron lip; rough grey walls
in uneven courses, a dark plinth, rust bleeding under the lamps; iron doors whose leaves rise into
the wall as their packs come out, a red lamp blinking over each open one; barred cells with a cot, a
chain and a skull; three caged lamps a wall, lit in the stage's colour (white tinged with it). The
level's number is carved in the north wall's middle, three tiles wide and six tall (a three-by-five
chisel font, the grooves dark with a pale lip above, old paint of the level's colour in them). The
platform is a stone slab paved in blocks inside a riveted iron frame (corner plates, guide shoes),
glowing red with the bell. The table: oak on stone trestles, a cloth that stains and slides askew,
and the feast by `feast_left` (1 at the first level, 0 at the seventh): the roast pig on its
platter, a fowl, a three-tier cake, loaves, cheese, bowls of fruit, a jug, goblets, two
candelabra; eaten to a haunch and bare ribs, then to a skull and a spine, the cake to a slice and
crumbs, the candles burnt down and out (smoke from the wicks), bones dropped round the table and
flies over what is left; at the seventh an overturned cup, its wine across the cloth and dripping off
the edge. The overhead light is harsh and cold straight down the holes; the ledges are lit by
their lamps; dust drifts down the light. On the way down the platform, the hero and its riders
sink (the view's `pit_sink`), the chase camera lifts its pitch a little (`pit_tilt`) to see the walls
slide up past it and the next number come up, the top camera moves over the hole to look down the
shaft (`pit_overhead`), the lamps below brighten and the light turns to the next stage's colour.

**The HUD and the guide.** Top middle, in the charcoal and copper: LEVEL n, the stage's name, the
clock large (red and beating with the bell at the end), the two minutes running out as a thin bar,
and the warning with how many stand on the platform; on the way down, the next level's name rising.
Under the score: ON THE PLATFORM n, and how many are behind the doors and how many rode down. The old
man's page (`GuidePlace::Pit(n)`): no devil, only the way down; the clock, what rides down and what
stays, the leap and the cross; each level's foes and the one thing to know of each; the cap and the
doors. His words "now" say the seconds left and who stands on the platform.

**Bot and harness.** `--pit SKILL TIER SEED [--pit-level N]` rides from level 1 (or N) and prints each
level's seconds, kills, riders in and out, foes left on the ledge, damage taken, falls and whether it
had to leap, then the run's end and the sim's cost per step. The adept and the expert walk onto the
platform 4 and 7 seconds before it sinks (`Bot::pit_board`); while the bell tolls the expert strikes
first at what stands on the platform (`pit_targets`); the novice (a masher) never looks at the clock.

Balance, Ragnar's kit at TIER, seeds 1 to 3 (lives lost; the run is three lives and game over):

| Skill | Tier 5 | Tier 7 | Tier 8 |
|---|---|---|---|
| Expert | the bottom, 0 / 0 / 0 | the bottom, 0 / 1 / 0 | the bottom, 0 / 0 / 0 |
| Adept | game over at level 2 to 3 | game over at level 3 to 5 | the bottom, 1 / 0 / 1 |
| Novice | game over at level 2 | game over at level 2 | game over at level 2 |

The expert kills 100 to 140 of the 168 foes, takes 40 to 290 damage over the seven levels, and lets
0 to 2 ride down a level; the cap holds at ten from the doors (the peak of 14 to 18 counts what the
deeper stages raise: Hel's dead, the kitsune's illusions). Cost: the sim at 0.02 ms a step with eight
or more standing (slowest step, the level's load, 43 ms); drawn at 1920 by 1080 on the RTX 5070 Ti
laptop, level 7 with ten foes, Hel's dead and Surtr's fire: 6.6 ms a frame from the chase camera
(GPU 3.9 ms), 7.5 ms from the top; 5.4k static cubes, about 1.1k drawn each frame.

**Changed from the design, and why.** The ledge is six tiles (25 - 11 over two, less the wall). The
table on the platform is solid, so the fight goes round it. The doors shut at 100 seconds (the last
twenty are for clearing the platform, as designed), and a wave that cannot come out for the cap
waits rather than being lost, until then. The tier is the realms' (8) and was not tuned further: the
measured spread above already meets the targets. Continue does not remember riders (it loads the
level fresh). Summoned foes (raised dead, illusions) may lift the count past ten for a moment.

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
