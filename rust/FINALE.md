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

Built 2026-10-04 in `logic/lair.rs` (the layout), `logic/combat/prince/krampus.rs` (his tables,
pace, thresholds and numbers), `game/lair.rs` (the fight, the cage, the boy, the way in and out),
`render/foes_lair.rs` (Krampus and the Percht sculpted) and `render/view/lair.rs` (the hollow's
art, his poses, his coals, his death, the cage and the boy).

**The lair** (`logic::lair`, `Theme::Lair`, "Krampus's Lair"): a round hollow 29 tiles across
(33 grid, radius 14.2). The platform (11 × 11, `PLATFORM`, its top flush with the floor) lies in
the south, the hero in its middle at (16, 23); Krampus at (16, 9); the cage (3 × 3, solid) hangs
low at the north end over a ring of iron in the floor, his stand to rattle it at (16, 6). Four
columns of ice two tiles by two (`COLUMNS`, 7.8-8.5 tiles from the middle, open ground round each)
are the stone he is baited into; two braziers of his coal flank the cage; his basket and three
sacks stand against the wall (solid); five bundles of birch and six broken toys lie in the snow,
ten chains with bells hang out of the dark near the wall, and two clefts in the wall west and east
are where his Perchten come out. No gate: the way out is up. Pinned by
`the_lair_is_a_round_hollow_with_the_platform_in_the_south_and_the_cage_in_the_north`.

**His fight.** `EnemyKind::Krampus` on `Archetype::Prince`, `Power::Pelt` (a hide as Mammon's: 30%
of a blow outside his recoveries); HP 1.35 × the Prince's 900, poise 1.3 ×, radius 1.0 (a tile of
arm), then the courts' weight for tier 8 (`court::weight`: health × 3.1, blows × 1.55): 3766
health at the bottom of the Pit. His phases fall at 70% and 40% (`krampus::PHASE_AT`, through the
new `prince::phase_for`), so Krampusnacht is the longest.

| Phase | Title | Moves (preferred first) | Pace, rest | What else |
|---|---|---|---|---|
| 1 (100-70%) | THE BELLS | Bell Toll, Chain Sweep, Birch Lash | 1.0, 1.0 | |
| 2 (70-40%) | THE CHAINS | Chain Lash, Hoof Charge, Chain Sweep, Birch Lash | 1.1, 0.85 | two Perchten out of the clefts, once |
| 3 (40-0%) | KRAMPUSNACHT | Coal Fire, Hoof Charge, Chain Lash, Sack Swing | 1.3, 0.62 | the cage taunt, up to three times |

| Move | Startup | Active | Recovery | Damage | Shape | Cooldown |
|---|---|---|---|---|---|---|
| Birch Lash | 32 | 8 | 44 | 26 | cone 1.3 (+1.0), ±75° | |
| Chain Sweep | 40 | 10 | 48 | 24 | cone 2.6 (+1.0), ±110°, from 1.2 | 210 |
| Bell Toll (`TollOfBells`) | 48 | 6 | 52 | 28 | cone 1.2 (+1.0), ±50°; quake 2.2 (+1.0) at 60% | 300 |
| Chain Lash | 50 | 8 | 46 | 26 | beam 9 × 1.0 along his aim, locked 24 f before it flies, from 2.4-8.5 | 240 |
| Hoof Charge | 42 | 46 | 46 | 28 | charge 7.5 tiles at 10.5 t/s, aim locked 14 f before he sets off; stone dazes him 150 f | 300 |
| Coal Fire | 40 | 1 | 42 | 22 | a row of six coals 0.7 wide, 1.25 apart, fuse 50, burning on 150 f | 300 |
| Sack Swing | 46 | 6 | 54 | 32 | cone 1.4 (+1.0), ±60°; quake 2.2 (+1.0) at 50% (his main move in Krampusnacht) | |
| Cage Rattle | 36 | 1 | 180 | — | a spell: no blow; he stands open all its recovery | |

* **The cage taunt** (`game::lair::cage_taunt`): 4 s into Krampusnacht, then 15 s after each,
  between his moves, he walks to the cage (a puppet of the lair, his token given back), grips the
  bars and rattles them (`CageRattle`): three seconds open, the boy crying ("HE RATTLES THE CAGE:
  STRIKE HIM!"). A stagger breaks it off; a walk of more than 8 s is given up.
* **The charge's daze**: as Amon's (a charge that crashes into stone reels 150 f, staggered); the
  ice columns, the cage, the braziers, the basket and the wall all stop him.
* **The Perchten** (`EnemyKind::Percht`, a kind of their own): Brutes with a birch switch (the
  archetype's slam and sweep), 0.8 of a Brute's health, cost 1, weighted as he is (their blows at
  0.8); they rise out of the clefts as the Chains begin and flee into the dark when he falls.
* **His death** (`krampus_slain`, `lair_aftermath`): the freeze of a trophy, time slowed, the
  screen shaken; he sinks to his knees in the snow and his seven bells fall silent one every 0.6 s
  (a toll each, the last "THE LAST BELL FALLS SILENT"); at 5 s the cage's lock breaks, the cage is
  down 0.7 s later, its door opens at 6.3 s, the boy climbs out (0.8 s) and runs to the hero at
  3.8 t/s (round the columns by the flow field); once he is in his arms ("TOGETHER AGAIN") they
  hold each other 2 s, and `lair_won()` is called (for now the Won screen, whose Confirm goes back
  to the title).
* **Nothing hurts the child**: he has no health and is no target; the cage is stone to every
  blow, coal and step (no trap lands on a solid tile), and nothing he does is aimed at anything
  but the hero. Pinned by `nothing_in_the_fight_touches_the_child_or_the_cage`.

**Why a module of its own, beside the courts.** A `Court` is keyed by its sin throughout (its
hooks, its colour, its general's terms, its key, the gate and the old man's wares after it), and
Krampus has none of them. So the lair keeps its own state (`game::lair::Lair`) and the court's
shared pieces learn of it in a few small, additive places: `prince()` and `prince_bar()` find
Krampus too (the bar notched at his own thresholds: `PrinceBar::notches`), `prince_phase` hands
his phases to `krampus_phase`, `brawl::slay` hands his fall to `krampus_slain`, and
`brawl::first_active` his bells, chain and sack to `krampus_strikes`.

**What changed from the design, and why.**

| Change | Why |
|---|---|
| Bell Toll is `MoveId::TollOfBells` | `BellToll` is Leviathan's already |
| Perchten are a kind of their own, light Brutes | no existing kind looks like a masked, horned, belled Percht; a Brute's wait-and-punish suits a beast with a switch, and they are weighted under him so two never decide the fight |
| The taunt is a spell with a 180 f recovery | his pelt opens in his recoveries already: the rule stays the princes' own, and the bots and players read it as every opening |
| The Chain Lash and the Hoof Charge are answered by the bots as Lucifer's lance and Amon's charge (`Bot::lucifer_dodges`, `Bot::amon_court` now also in the lair) | the same shapes, the same answers |
| His coals' lingering stings are not landings | as Amon's lava (`brawl::update_traps`): the floor he chose to stand on |
| Entering the lair gives all seven keys (`load_lair`) | no one reaches the bottom without them, and the map's jump must meet him at tier 8 |
| HP 1.35 (from a first 1.6) | at 1.6 the expert took 205 s at tier 8 and 256 s at tier 7; at 1.35 both sit in 150-250 s |

**His body** (`render::foes_lair::krampus`, 100 × 130 × 96 at 0.025, 3.25 tiles tall): Scene I's
Krampus as he fights, with the birch bundle bound in red ribbon in his right fist, the chain and its
great bell hanging from his left, a sack of coals (a part of its own, `sack`) at his hip that he
takes in his fist for the Sack Swing, his eyes glowing; rig parts head, armR, armL, legR, legL,
sack. His poses (`WorldView::pose_krampus`) for every move and his daze; his walk a hunched stalk
(`animation.rs`). The Percht (48 × 82 × 40 at 0.025): pale sheepskin to the knees, a belt of
cowbells, a carved red mask grinning with its tongue out, two pairs of horns, a switch.

**The art** (`render::view::lair`): cold blue-white light from the shaft above on the platform and
over the cage, rays of it and snow drifting down through it; black stone slabs dusted with snow,
drifts against the wall, frost in the cracks; rock walls glazed with ice and hung with icicles; the
ice columns; the braziers' red coal and its light; the chains swaying with their bells; the cage
(its bars, its dome, its lock, its door on a hinge) swaying on its great chain, shaking when he
rattles it, coming down when the lock breaks; the boy in it (Scene I's cast, `kid(0)`), frightened,
crying when it is rattled, then climbing out and running. His coals (`emit_coals`) arc from his
hand, glow hotter as their fuse burns in a ring of melted snow, burst into flame and burn on as a
bed of embers; his chain cracks out along its line; his death sinks him into the snow in black
smoke, a glint on each bell still ringing.

**His voice and sounds** (`audio/voices.rs`): bells for his signature (`krampus_bells`), a goat's
bleat torn into a roar, a slow laugh through his teeth, iron chain, the birch's swish, the child's
whimper (in the cage rattle's sound), each move its own tell and swing; the Percht a few small
bells and a growl. The phases step the music's tempo and roar (`BossRoar`, `Bells`); the lair plays
a boss's track.

**The old man's guide**: `GuidePlace::Lair`, "KRAMPUS'S LAIR": Krampusnacht, St Nicholas's dark
companion, the birch and the basket, the Perchten; what to do; what to watch for.

**Harness and results** (`--lair SKILL TIER SEED`, Ragnar's kit at the tier, all seven keys held,
360 s limit):

| Skill | Tier | Seed 1 | Seed 2 | Seed 3 |
|---|---|---|---|---|
| expert | 8 | slain 168.4 s, no fall, took 0 | slain 181.7 s, no fall, took 83 | slain 165.3 s, no fall, took 59 |
| expert | 7 | slain 249.0 s, no fall, took 59 | slain 241.6 s, no fall, took 0 | slain 221.9 s, no fall, took 78 |
| adept | 8 | timed out in the Chains, 2 falls | | |
| adept | 7 | lost every life in the Bells, 83.6 s | | |
| novice | 8 | lost every life in the Bells, 37.8 s | | |
| novice | 7 | | lost every life in the Bells, 34.4 s | |

No two blows inside 18 frames in any of them; 16-25 charges a fight, 3-13 of them dazed in stone,
three taunts. Pinned by `the_expert_beats_krampus_in_his_lair_and_a_masher_does_not`.

**Shared files touched.** `logic/enemy.rs` (Krampus, Percht; `LAIR_OWN`), `logic/combat/archetype.rs`
(their rows; `Power::Pelt`), `logic/combat/frames.rs` (his MoveIds), `logic/combat/prince/mod.rs`
(`krampus`, `phase_for`), `logic/level.rs` (`Theme::Lair`), `logic/guide.rs`, `logic/juice.rs`,
`logic/war/{counsel,ledger,names}.rs` (their matter, trophy, culture, names), `game.rs` (`lair`),
`game/court/mod.rs` (the prince found, the bar's notches, the phase handed on, `drop_breather`
shared), `game/brawl.rs` (`phase_for`, his fall, his strikes), `game/level.rs` (the lair set up
and reset), `game/save.rs` (`Stage::Lair`), `game/map.rs`, `game/realm.rs` (his name), `game/flow.rs`
(the scene's lair; Won → title from the lair), `game/guide.rs`, `game/tour.rs`, `game/bot.rs`
(`lair_fight`; the beam and charge answers in the lair), `main.rs` (`--lair`), `capture.rs`
(`lair`, `taunt`, `fell`), `audio/voices.rs`, `render/cast.rs` (`link`, `bell`, `birch`, `horn`
shared), `render/hud.rs`, `render/hud/{court,ledger}.rs`, `render/view.rs`, `render/view/{emit,
duel,animation,models}.rs`.

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
