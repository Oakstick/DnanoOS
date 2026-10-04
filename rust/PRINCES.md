# The six princes: their courts and their fights

Greed's prince, Mammon, fights in his own court (the Counting House; COMBAT.md §13). This file is
the design for the other six, built in parallel on a shared scaffold, each the same way as
Mammon: a duelist of the fourth archetype, the Prince, fought by frame data in a court built to
its own plan. Status: designed 2026-10-04; each section's builder writes its build notes and
results under its own heading at the end.

## 0. Rules every prince keeps

These come from COMBAT.md §0 and §13 and WARBAND.md, and they are not bent:

* **Readable.** Every move he makes starts up for 30 frames or more (the reaction floor is 18;
  a prince is slower to read than a man). A move that lands from a distance (a trap, a mark, a
  beam) shows where it will land before it does, and lets the hero step out with a fifth of the
  time to spare (as `a_coin_on_the_floor_bursts_late_enough_to_step_off_it`).
* **Punishable.** Every move ends in a recovery of 30 frames or more in which his hide is open.
* **Nothing slows the hero, holds him, or turns his controls.** Mammon's grasp is the one hold in
  the game. No pool of slowness, no charm that flips the stick, no root. A push (`Rider::Shove`)
  is allowed; so is taking the hero's guard away for a moment, as long as it is telegraphed and
  leapable.
* **The threat manager plans every landing.** No two landings within 18 frames; the prince's
  token is three points; what he raises waits while he strikes (court pool 3, a grant every 45).
* **Phases by health** (`prince::PHASE_AT`, two thirds and one third): each change gives his roar,
  a banner with his phase's title and a line of what to do, and a green cross for a breather.
* **A hide** that turns most of a blow outside his recoveries (`HIDE_SHARE` 0.3), unless his
  design says otherwise; a parry still deflects him.
* **His general weighs on him** (`PrinceTerms`): his general slain, he fights without his adds;
  his general's decree done, his adds come at once as the fight begins (and the design below
  may name one more thing the decree changes).
* **He drops his sin's key.** The way on is as from Mammon: the gate behind him, the old man's
  wares, the Portals of Hell with the sin sealed. A fall brings him back whole before his throne;
  Continue comes back to his court.
* **Balance targets**, played by the bots in his court (`--court SIN SKILL TIER SEED`): the expert
  beats him at tiers 1 and 2 without losing all three lives, in about 90 to 150 seconds; a masher
  (novice) cannot. Pinned by a test as Mammon's is.
* **Determinism.** The sim uses its own seeded rolls; the view and the sound have their own.
* **Art.** The prince is a voxel sculpt in parts its rig swings, about two and a half times the
  hero's height (as Mammon, 100 × 124 × 80 at 0.025), built at startup, so no larger than his.
  His court is a hall about thirty tiles across with its own art module, lit and coloured to the
  realm he rules. Each has his own voice and attack sounds (the per-breed `VoiceSet`).
* **The old man's guide** (`logic/guide.rs`, `GuidePlace::Court(sin)`) is rewritten for the court
  as built: the tale of the prince, what to do, what to watch for, in the old man's voice. The
  old man always leads to good.

**Shared pieces** (built once, in the scaffold, for any prince to use): `Shape::Beam` (a line from
him along his locked aim, stopped by rock), `Shape::Charge` (he runs along his locked aim and may
crash into stone, `Duelist.crashed`), `Shape::TrapLine` (traps in a row toward the hero), and
ground that keeps hurting for a while after a trap bursts (`Trap.linger`). Each prince's court,
fight, sculpt and view live in their own files: `logic/court/<sin>.rs`,
`logic/combat/prince/<sin>.rs`, `game/court/<sin>.rs`, `render/foes_court/<sin>.rs`,
`render/view/court_<sin>.rs`.

---

## 1. Lucifer, Pride: the Throne of the Morning Star

*"How art thou fallen from heaven, O Lucifer, son of the morning!"* (Isaiah 14:12). The light-
bearer, the most beautiful of the angels, who would set his throne above the stars and fell.
Binsfeld's prince of pride. The realm before him is the Hall of Mirrors, whose lie is *"You stand
above them all. Take your throne."*

**The court.** A round hall of white marble veined with gold, its floor a mirror-polish that
reflects him. Seven tall mirrors in gilt frames stand round the wall; in the north, high steps up
to an empty throne under a great window shaped like a star. Shafts of light fall from it.

**Him.** A tall angel, terribly beautiful: white-gold armour, a face like a statue's, a crown of
light, a sword of light. Six wings, white in his first phases, burnt black and smoking in his
third.

| Phase | Title | Moves (in the order he prefers them) | What else |
|---|---|---|---|
| 1 (100-66%) | THE MORNING STAR | Lance of Dawn (a beam), Radiant Cut | |
| 2 (66-33%) | THE MIRROR | Mirror Step, Lance of Dawn, Radiant Cut | two Reflections step out of the mirrors |
| 3 (< 33%) | THE FALLEN | Starfall, Pride's Fall, Radiant Cut | his wings burn; quicker |

* **Radiant Cut**: his sword swept wide (a cone like the sceptre).
* **Lance of Dawn**: a beam of light along a line from him toward the hero: the line glows on the
  floor through his startup and stops tracking before it fires (a new `Shape::Beam`).
* **Mirror Step**: he steps into one mirror and out of another near the hero, with a long
  recovery as he comes out (no blow; a spell).
* **Starfall**: stars fall round him and burst (traps, like the coin rain).
* **Pride's Fall**: he rises on his wings and crashes down on a mark that follows the hero
  through the tracking frames, then holds still: a quake.
* **His hide, Vainglory**: Pride never looks behind him. His light turns most of a blow outside
  his recoveries, but a blow from behind him (more than 100° from where he faces) lands whole.
* **The Reflections** (the add): cold silver copies of him, left-handed, with no shadow; they
  strike with his Radiant Cut at half his damage, and shatter at one good blow. The real one casts
  a shadow and his halo is gold. Only he bleeds.
* **His general, the Chamberlain of Glass**: slain, no reflections; decree done (the throne-mirror
  polished), the reflections stand at his side from the start.

## 2. Leviathan, Envy: the Drowned Bell

*"Leviathan the piercing serpent, even leviathan that crooked serpent; and he shall slay the
dragon that is in the sea"* (Isaiah 27:1). Binsfeld's prince of envy. And Kiyohime of Dōjō-ji,
whose jealousy turned her into a serpent that coiled round the temple bell and burned the man
hiding in it. The realm before him is the Serpent Grotto: *"All you want lies below."*

**The court.** A drowned temple: a round hall of green-black stone, a ring of shallow green water
round a central floor (the water is only looks: it never slows the hero), and in the middle a
great bronze bell hung from a broken beam, its mouth an arm's height above the floor. Serpent
bones in the walls; blue *nazar* beads hung on cords.

**Him.** A sea serpent's front third risen out of the water: a great finned head with a crest of
spines, slit-pupilled eyes, green-black scales with a pale belly, the rest of his body implied in
coils behind him. He moves over the floor as a duelist like every prince.

| Phase | Title | Moves | What else |
|---|---|---|---|
| 1 | THE CROOKED SERPENT | Brine Spit, Coil Lash, Dive | |
| 2 | THE EVIL EYE | Covet, Brine Spit, Coil Lash | two Serpentlings rise from the water |
| 3 | THE BELL | Bell Toll, Burning Coil, Coil Lash | he coils round the bell; quicker |

* **Coil Lash**: his tail swept wide round him (a cone of 120° or more).
* **Brine Spit**: a fan of three bolts.
* **Dive**: he sinks out of sight (no blow lands on him, and none of his) and comes up under the
  hero: a mark that follows the hero, then erupts, and he stands there in a long recovery.
* **Covet** (Envy's own): he copies the archangel's power the hero last used, as a telegraphed
  move of his own in green: Michael's four bolts of fire become a volley of four; Gabriel's
  thunder, three marks round the hero; Raphael's beams, a beam; Uriel's wave, a ring of the
  floor shaking round him. If the hero has used none, he copies Raphael. The banner tells it:
  "HE COVETS YOUR <ANGEL>".
* **Bell Toll**: he strikes the bell; the floor shakes in a ring round it (a quake).
* **Burning Coil**: fire bursts at points round the bell (traps).
* **His hide, Scales**: as Mammon's; in his third phase, coiled round the hot bell, his hide is
  burnt off on the side he turns to the bell (as Lucifer's back, but by place, not facing).
* **The Serpentlings** (the add): sea snakes, skirmishers.
* **His general, the Bell-Warden**: slain, no serpentlings; decree done (the bell lowered), the
  serpentlings come at once.

## 3. Amon, Wrath: the Anvil of Wrath

Amon (the Goetia): a great marquis, a wolf with a serpent's tail who vomits flame; at the
conjurer's word, a man with a raven's head and a dog's teeth. Wrath's prince here (Satan is the
last boss). The realm before him is the Burning Field: *"Sound the horn and the battle ends."*

**The court.** A ring of black basalt cracked with glowing seams, broken weapons stuck in the
ground, a great anvil on a dais in the north with a burning throne behind it, war banners on
fire. Four broken stone pillars stand round the floor: they matter (below).

**Him.** A raven-headed demon warrior in a wolf's pelt, a serpent's tail behind him, fire in his
throat, a great iron axe. Scarred, huge-shouldered.

| Phase | Title | Moves | What else |
|---|---|---|---|
| 1 | THE WOLF | Fiery Charge, Flame Breath, Axe Cleave | |
| 2 | THE RAVEN | Rift, Fiery Charge, Flame Breath | his ravens come |
| 3 | THE FURY | Fiery Charge (twice), Eruption, Flame Breath | his rage |

* **Fiery Charge**: he lowers his head and charges in a straight line, locked as his startup
  ends, up to seven tiles; what is in his path is struck. If he runs into a pillar or the wall he
  is dazed (a long recovery, his hide open): the hero baits him into stone.
* **Flame Breath**: a cone of fire, longer than an arm (2.2 tiles, ±35°).
* **Axe Cleave**: an overhead blow, narrow and far.
* **Rift**: he splits the floor in a line toward the hero; the seam glows, then bursts into lava
  that burns for a few seconds (stood in, it hurts; it never slows).
* **Eruption**: the floor round him bursts (a quake).
* **His rage, Fury**: Wrath has no hide. Every blow he takes makes him a little quicker (rage, up
  to five steps); a charge that ends dazed against stone empties it. His health is a little more
  than the others' to make up for the open hide.
* **The Ravens** (the add): flyers that harry from the air (skirmishers).
* **His general, Marshal Gorebrand**: slain, no ravens; decree done (the bridges burnt), the
  ravens come at once.

## 4. Belphegor, Sloth: the Hall of Stopped Clocks

Belphegor, the Moabite Baal of Peor, whom Binsfeld named prince of sloth; the demon of
discoveries and ingenious inventions who makes men rich and idle. The realm before him is the
Grey Mire: *"Lie down. The door opens when you wake."*

**The court.** A grey bedchamber the size of a hall, full of fog that lies on the floor. Stopped
clocks of every size stand round the walls; a giant hourglass, stopped, stands on the floor before
his great chair; cobwebs hang from the beams; a drift of sand.

**Him.** A huge, bloated, bearded demon with ram's horns and long nails, in a grey night-robe,
heavy-lidded. He sits in his great chair as the fight begins.

| Phase | Title | Moves | What else |
|---|---|---|---|
| 0 | ASLEEP | (none) | he sleeps until struck: the first blow lands whole |
| 1 | THE SLEEPER | Yawn, Swat, Sand of Hours | |
| 2 | THE DREAMER | (he lies down to sleep again) | two Nightmares come out of his dream |
| 3 | THE WAKING | Hourglass Slam, Clock Hands, Yawn, Swat | he stands up; slow, vast reach |

* **Yawn**: a great breath pushes the hero back (`Rider::Shove`) in a cone; little damage;
  leapable.
* **Swat**: a slow, heavy, wide blow of his hand.
* **Sand of Hours**: sand from the hourglass drifts onto the floor and bursts (traps).
* **The Dreamer**: as his second phase begins he lies down again. Asleep, his hide is open, he
  heals slowly, and his nightmares fight; struck hard enough (a share of his health, or a
  finisher) he wakes with a roar and fights. Sloth is the lesson: a hero who stands back and
  waits lets him mend.
* **Hourglass Slam**: he lifts the hourglass and brings it down (a quake).
* **Clock Hands**: a long sweeping blow all the way round him (a wide cone) from far.
* **His hide, Torpor**: as Mammon's, but open while he sleeps and as he wakes.
* **The Nightmares** (the add): dream-wisps, hexers that keep their distance.
* **His general, the Lamplighter**: slain, no nightmares; decree done (the hourglass wound),
  Belphegor wakes rested: he does not sleep at the start (no free blow), and his nightmares come
  at once.

## 5. Beelzebub, Gluttony: the Banquet of Flies

Baal-Zebub, "lord of the flies"; in Collin de Plancy's *Dictionnaire infernal* a great fly.
Binsfeld's prince of gluttony. The realm before him is the Rotting Feast: *"Finish your plate,
and you may leave."*

**The court.** A banquet hall gone to rot: a great table in a U round the floor, platters of food
grey with mould, cauldrons over cold fires, a throne of piled plates at the head. Flies drift in
the air as a living dark.

**Him.** A bloated lord with a fly's head: great red compound eyes, a proboscis, translucent
veined wings, a crown, a stained robe over a swollen belly, a meat cleaver.

| Phase | Title | Moves | What else |
|---|---|---|---|
| 1 | THE LORD OF FLIES | Swarm, Cleaver, Gorge | |
| 2 | THE SWARM | Scatter, Swarm, Cleaver, Gorge | two Fly Swarms |
| 3 | THE GLUTTON | Belly Roll, Bile Spew, Belly Slam, Gorge | swollen; quicker |

* **Cleaver**: a chop (a cone).
* **Swarm**: a fan of slow flies (a volley of five, slower bolts than coins).
* **Gorge** (Gluttony's own): when hurt he goes to one of the court's four platters and eats: a
  long move (startup 50 or more) that heals him a share of his health. Struck while he eats, he
  chokes (staggered, hide open) and the platter is spoiled. Each platter feeds him once. He also
  eats a green cross lying on the floor if the hero leaves it: the hero takes the breathers first.
* **Scatter**: he bursts into flies (no blow on him, none of his) and gathers again beside the
  hero, landing with a belly slam.
* **Belly Roll**: he rolls in a straight line (like Amon's charge, without the daze).
* **Bile Spew**: marks of bile that burst and leave a puddle that hurts to stand in for a few
  seconds (it never slows).
* **Belly Slam**: a quake.
* **His hide, Bloat**: as Mammon's.
* **The Fly Swarms** (the add): skirmishers, quick and light.
* **His general, the Steward of the Last Course**: slain, no swarms; decree done (the feast laid),
  six platters instead of four, and the swarms come at once.

## 6. Asmodeus, Lust: the Rose Bower

Asmodeus (Ashmedai): in the Book of Tobit he loved Sarah and killed her seven bridegrooms on their
wedding nights, until the archangel Raphael taught Tobias to burn a fish's heart and liver on the
embers of incense; the smoke drove the demon away to Egypt, where Raphael bound him. In the
Goetia a king with three heads (a bull's, a man's, a ram's), a serpent's tail and goose's feet,
riding a dragon, a lance and banner in his hand. Binsfeld's prince of lust. The realm before him
is the Garden of Thorns: *"Turn around."*

**The court.** A walled garden of crimson roses and black thorn hedges, a marble fountain in the
middle, a bridal arch to the north, and four bronze censers on plinths round the floor, cold.

**Him.** Tall and elegant in rose-red silks, three heads (the man's in the middle, handsome and
smiling; a bull's and a ram's either side), a serpent's tail, a lance with a pennon.

| Phase | Title | Moves | What else |
|---|---|---|---|
| 1 | THE SUITOR | Lance Thrust, Blown Kiss, Thorn Line | |
| 2 | THE THREE HEADS | Bull's Charge, Ram's Butt, Blown Kiss, Lance Thrust | two elf-maids step from the hedges |
| 3 | THE BRIDEGROOM OF ASHES | Thorn Ring, Lance Thrust, Blown Kiss, Bull's Charge | quicker |

* **Lance Thrust**: a narrow, long thrust (a cone of 2.2 tiles, ±20°).
* **Blown Kiss**: a slow pink heart (a single bolt). If it strikes him the hero is **smitten**:
  he cannot guard or parry for 90 frames (he can still move, leap and swing). It can be leapt, or
  struck out of the air by his blade; it is never a hold and never turns his controls.
* **Thorn Line**: thorns burst from the ground in a line toward the hero (traps in a row).
* **Bull's Charge**: a straight charge (as Amon's, without the daze).
* **Ram's Butt**: a short blow that pushes the hero back (`Rider::Shove`).
* **Thorn Ring**: thorns burst in a ring round him (traps).
* **The censers** (Tobit): a blow on a cold censer lights it, and it smokes for ten seconds. While
  any censer smokes, Asmodeus' hide is gone (Raphael's smoke), and he will not stand within three
  tiles of a smoking censer. Each censer lights twice in a fight. The hero's tool against him.
* **His hide, Allure**: as Mammon's, except while a censer smokes.
* **The elf-maids** (the add: the realm's Ellepige, hollow behind).
* **His general, the Bridesmaid**: slain, no elf-maids; decree done (the vows spoken), the
  elf-maids come at once and the censers light only once each.

---

## Build notes

Each builder writes here, under their prince, what was built, what changed from the design and
why, and the court's results (the bots' table, as COMBAT.md §13.2).

### Lucifer

Built 2026-10-04. **The court** (`logic/court/pride.rs`, art `render/view/court_pride.rs`): a
round hall 29 tiles across (33-tile grid, floor within √204 of the middle), white marble veined
with gold and polished, a morning star of gold inlaid at its heart in rings of gold and lapis, a
lapis way from the gate to the steps. Six tall gilt mirrors stand on the floor's edge, three
either side (`MIRRORS`); in the north the steps (`STEPS`, rock) rise to the empty throne, whose
back is the seventh mirror, before a high apse with the star window; five shafts of light fall
from it to the floor (`SHAFTS`). Nothing stands on the floor: his lance crosses it from wall to
wall. Gate in the south wall, the hero four tiles inside it, Lucifer at the foot of his steps.

**His fight** (`logic/combat/prince/pride.rs`, `game/court/pride.rs`), the Prince's numbers as
Mammon's (900 / 150 / 40, arm +0.9; the Fallen ×1.35 pace and ×0.6 rest):

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Radiant Cut | 32 | 8 | 42 | 24 | cone 1.3 (+0.9), ±100° |
| Lance of Dawn | 52 | 10 | 40 | 26 | beam 11.5 × 1.0 along his aim (locked at f22), from 2.5-9 tiles, cooldown 210 |
| Mirror Step | 36 | 1 | 72 | — | spell: out of the mirror nearest the hero (marked to f30), from 4.5 tiles, cooldown 480 |
| Starfall | 40 | 1 | 36 | 16 | five stars falling round the hero (one where he stands), 0.85 wide, bursting 17 + 44 f after |
| Pride's Fall | 56 | 6 | 54 | 30 | a mark 1.4 wide following the hero to f36; he is aloft (untouchable) from f12 and lands on it; from 2-8 tiles |

Phases: THE MORNING STAR (Lance of Dawn, Radiant Cut), THE MIRROR (Mirror Step, Lance, Cut; two
Reflections step out of the two mirrors that stand about four tiles from the hero, once), THE FALLEN
(Starfall, Pride's Fall, Cut; his white wings give way to the burnt six, smoking). **Vainglory**
(`Duelist::taken_from`): his hide turns 70% outside his recoveries, except a blow from more than
100° round from where he faces, which lands whole. **The Reflections**: his body as in a mirror
(left-handed, silver, a silver halo), Brutes that cut with his Radiant Cut at half his damage
(and his court's weight), one hit point: one blow shatters them; they shatter as he falls. The
Chamberlain slain: none; his decree done: they stand at Lucifer's side as the fight begins (and
not again at the Mirror). The bot (`Bot::lucifer_dodges`) steps across his lance's line once it
has seen it, as it walks out of a mark; nothing else of its play changed.

What changed from the design, and why:

| Change | Why |
|---|---|
| The stars fall round the hero, not round Lucifer, five of them | as Mammon's rain: it is a blow from afar that the Fallen needs; five leave a gap to walk out through between the ring's stars |
| No lingering starfire | a lingering trap's stings land outside the threat manager's plan, and two blows inside 18 frames came of it (overlaps) |
| The Reflections are two thirds his size, and cast a shadow like his | an add is no taller than 2.2 tiles (the shared sculpt test), and every rig's shadow is drawn alike; the real one is told by his gold halo, his size and his blue tabard |
| The seventh mirror is the throne's back | six round the wall and the throne-mirror of his Chamberlain's decree; the mirror step may come out at the throne's foot too |
| Pride's Fall is a mark (no guard), with the floor shaking as he lands, not a quake | a quake is the shaking round a cone where he stands; his fall comes down where the hero was |

Results (`--court pride SKILL TIER SEED`, seeds 1-3, Ragnar's kit at the tier; time and taken are
the last life's):

| Skill | Tier | Win | Time (s) | Deaths | Taken |
|---|---|---|---|---|---|
| novice | 1 | 0/3 | lost every life in 46.4 | 4 | 384 |
| novice | 2 | 0/3 | lost every life in 49.9 | 4 | 487 |
| adept | 1 | 0/3 | timed out (240), at the Mirror | 0-1 | 162 |
| adept | 2 | 0/3 | timed out (240), at the Mirror | 0-1 | 119 |
| expert | 1 | 3/3 | 105.9 (98.0-112.1) | 0 | 33 |
| expert | 2 | 3/3 | 95.0 (88.6-102.6) | 0 | 7 |

No two blows inside 18 frames in any of them. Pinned by
`the_expert_beats_lucifer_in_his_court_and_a_masher_does_not` (game/court/pride.rs).

### Leviathan

Built 2026-10-04. **The Drowned Bell** (`logic/court/envy.rs`): a round hall 27 tiles across
(radius 13.9 about (16, 16)), the gate in the south wall at (16, 30), the hero at (16, 26) in the
water, Leviathan at (16, 7) before the drowned altar (14-18, 3-4). The ring of shallow water runs
from 9.2 to 12.3 tiles out (looks only); six broken columns stand in it, and six springs between
them are where the serpentlings rise. The bell is a 3 x 3 block of rock in the middle (rim 1.45),
open floor all round it for him to coil and the hero to walk.

| Phase | Title | Moves (preferred first) | What else |
|---|---|---|---|
| 1 | THE CROOKED SERPENT | Brine Spit, Coil Lash, Dive | |
| 2 | THE EVIL EYE | Covet, Brine Spit, Coil Lash | two serpentlings rise from the springs nearest the hero |
| 3 | THE BELL | Bell Toll, Burning Coil, Coil Lash | he swims to the bell and keeps within 3.3 of it (7 t/s), pace x1.35, rest x0.6; his scales burnt on the bell's side |

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Coil Lash | 34 | 8 | 44 | 24 | cone 1.4 (+0.9), +-85 deg |
| Brine Spit | 34 | 1 | 34 | 12 x3 | volley of three, +-0.3 rad, 2.6-8 tiles, cooldown 180 |
| Dive | 60 | 6 | 56 | 26 | mark 1.1 following him for 44 f (Burrow's rules: out of reach under the water) |
| Covet | 44 | - | - | - | becomes one of the four below on its first frame, cooldown 420 |
| Coveted Fire (Michael) | 44 | 1 | 36 | 11 x4 | volley of four, +-0.36 rad |
| Coveted Thunder (Gabriel) | 44 | 6 | 40 | 18 | mark 1.0 (28 f tracking) and two more 1.8 behind him either side |
| Coveted Beam (Raphael) | 44 | 8 | 40 | 20 | beam 9 x 1.0, locked at frame 12 (block only) |
| Coveted Wave (Uriel) | 44 | 6 | 46 | 22 | quake 2.2 (+0.9) at 80% |
| Bell Toll | 72 | 6 | 46 | 28 | mark 3.6 on the bell's middle (no tracking), cooldown 360 |
| Burning Coil | 46 | 6 | 40 | 20 | six fires (0.85) in a ring 2.8 round the bell and one where he stood, cooldown 330 |

HP x1.15 of the Prince's 900 (`Breed::of`), Scales a hide as Mammon's gold.

**How it is built.** `logic/combat/prince/envy.rs` holds his tables, pace and the pure rules
(`covet`, `burnt_side`, `fire_spots`, `thunder_marks`, `coiled`); `game/court/envy.rs` his doings
(the angel watched, the covet turned, the toll's mark pinned to the bell, the fires and the extra
thunder marks struck on the first active frame through `general_hazard_blow`, the swim and the
tether, the serpentlings); `game/court/mod.rs` gained one field, `Court::envy` (what the court
keeps: the angel last called and his move as last seen).

**What changed from the design, and why.**

* *Covet* is one MoveId in the table that becomes `CovetFire`, `CovetThunder`, `CovetBeam` or
  `CovetWave` on its first frame (a table is static and has four cooldowns). All four start up as
  the covet does, so only the fire's landing moves (its bolts fly): it is planned again with the
  threat manager, and if it would land too near another strike he copies Raphael's beam instead.
  The covet is kept off (its cooldown held) while the hero is out of the coveted move's range.
  The angel last called is seen from the hero's blessing charges and his patron's wait (no field
  added to `Game`); none called, he copies Raphael. The banner reads "HE COVETS YOUR <ANGEL>" with
  what to do.
* *Gabriel's thunder*, three marks round the hero: the first is a `Shape::Mark` on him, the other
  two stand behind him from Leviathan, so the way out is toward Leviathan or straight back.
* *Bell Toll* is a `Shape::Mark` whose mark is set on the bell's middle, radius 3.6, so the pale
  ring inlaid in the floor shows where it reaches. A ring that wide must give a fifth to spare
  from its middle (the frames test), hence a 72-frame startup: a long rear-back, punished from the
  bell's side.
* *Burning Coil* is a spell with damage: its fires are reckoned from where the hero stood as it
  began (`Lore::mark`), so the view draws them without new shared state, and they are not
  `Trap`s (which `emit_traps` draws as Mammon's gold coins).
* *His hide in the third phase*: `Lore::bell` (new, shared) holds the hot bell; `taken_from` lets
  the whole blow through from within 100 degrees of the way from him to the bell. To keep him
  coiled round it he swims back whenever he is more than 3.3 from its middle (the Bell's moves
  all work from there).
* *Dive* reuses the burrow's movement and untouchability (`Duelist::think`/`untouchable` now take
  `MoveId::Dive` with `Burrow`), drawn as water rings instead of heaving earth.

**Shared files touched.** `logic/combat/frames.rs` (his MoveIds, `FOE`, `data()`; the shared-shapes
test now lets the coveted beam use `Shape::Beam`), `logic/combat/duelist.rs` (`BoltLook` spit for
him, `Lore::bell`, `taken_from`, `Dive` with `Burrow`), `logic/combat/archetype.rs` (his row: HP
x1.15), `game/court/mod.rs` (`Court::envy`), `game/bot.rs` (`envy_dodges`, `envy_stand`, two hooks
in `act`), `render/view/animation.rs`, `render/view/duel.rs` (tells, crown, poses, the dive unseen),
`audio/voices.rs` (his and the serpentlings' tells, deaths and swings), `logic/guide.rs` (his page),
`game/tour.rs` (his card's line).

**The bot.** Not the novice: it steps off the coveted beam's locked line, off the fires and out of
the thunder's three marks by the clearest way, and in the third phase goes round him to the
bell's side before it strikes.

**Results** (`--court envy SKILL TIER SEED`, three lives, 240 s limit; expert and adept with
Ragnar's kit at the tier):

| Skill | Tier | Seeds won | Time (s) | Taken | Deaths |
|---|---|---|---|---|---|
| expert | 1 | 3/3 | 113-120 | 0 | 0 |
| expert | 2 | 3/3 | 101-103 | 0-17 | 0 |
| adept | 1 | 0/3 | timed out in phase 2 | 58-163 | 0-1 |
| adept | 2 | 0/3 | timed out in phase 2 | 79-172 | 0-1 |
| novice | 1 | 0/3 | game over in 48 | 384 | all |
| novice | 2 | 0/3 | game over in 51 | 487 | all |

Pinned by `the_expert_beats_leviathan_in_his_court_and_a_masher_does_not`. Before the bot learnt
his beam the expert lost two lives a fight to it; at HP x1.0 tier 2 took 86-92 s.

### Amon

Built 2026-10-04. The court is `logic/court/wrath.rs` (the Anvil of Wrath: a round basalt hall 27
tiles across inside a 33 grid, the gate south at (16, 30), the hero at (16, 26), Amon before his
anvil at (16, 8); the dais (12-20, 3-5) with the anvil on it as three solid tiles (15-17, 5), the
throne in the north wall behind; four broken pillars two tiles by two at (9, 11), (22, 11),
(9, 20), (22, 20), each 6-8 tiles from the middle with open floor all round it; six perches by
the banners where the ravens come down). His fight is `logic/combat/prince/wrath.rs` (tables,
pace, rage) and `game/court/wrath.rs` (the daze, the twin charge, the ravens); his art
`render/foes_court/wrath.rs` (Amon 100 x 124 x 80 at 0.025 in seven parts with a tail; a raven
80 x 64 x 54 at 0.025, hovering in its grid) and `render/view/court_wrath.rs`.

| Move | Startup | Active | Recovery | Damage | Shape | Cooldown |
|---|---|---|---|---|---|---|
| Fiery Charge | 40 (tracks 26) | 40 | 44 | 26 | charge 7 tiles at 11 t/s, struck within 1.15 of his middle | 300 |
| Axe Cleave | 38 | 6 | 46 | 30 | cone 1.7 (+0.9), ±22° | 150 |
| Flame Breath | 34 | 14 | 40 | 22 | cone 1.3 (+0.9 = 2.2), ±35° | 0 |
| Rift | 38 | 1 | 40 | 20 | a row of six lava seams, 0.65 wide, 1.25 apart, fuse 50, lingering 150 | 300 |
| Eruption | 44 | 6 | 50 | 30 | cone 1.2 (+0.9); quake 2.6 (+0.9) at 60% | 240 |

Phases: the Wolf (charge, cleave, breath), the Raven (rift, charge, breath; three ravens come
down), the Fury (charge twice running, eruption, breath; pace x1.25, rest x0.65). His moves carry
armour 70 (`frames::AMON_ARMOR`): with no hide the finisher would otherwise stop every one.

What changed from the design, and why:

* **The charge from any distance** (range 0-7). The players and the bots fight him at arm's
  length, where a charge kept for 2.4 tiles and more never came (two in a fight). Now when it is
  rested he lowers his head wherever he stands; the line of his run shows on the floor and stops
  following 14 frames before he sets off (not "as his startup ends": a lock on the last frame
  could not be side-stepped), so it is parried, blocked, leapt or stepped out of, and he runs on.
* **The daze** is 150 frames staggered (the whole blow and a half lands), and it spends his rage.
  The brawl's `Cue::Crashed` only shows dust; `wrath_tick` turns a crashed charge into the daze.
* **Rage**: a step for every blow that lands (`brawl::duel_hit`), up to five; each step +6% to his
  pace and -8% to his rest (`prince::wrath::enraged`, read by `Duelist::stats`); his startups never
  shorten. Only stone (or a fall) empties it. His health is 1.3 times the prince's.
* **Twice running**: a Fury charge that ends on open floor has its cooldown cleared and his rest
  cut to 8 frames (`Lore::twin`), so the second still asks the threat manager for its landing.
* **The rift's lingering lava** stings every 30 frames for a quarter of its damage; those stings no
  longer count as landings (`brawl::update_traps`): they are the floor he chose to stand on, as
  Surtr's fire is, and the threat manager cannot plan them.
* **The old man's counsel** on Amon is no longer "his hide turns your blade": a new answer,
  `BaitIntoStone`, says his charge runs straight and stone dazes him.

The bots: the expert (only in this court, `Bot::amon_court`) steps aside once the charge's aim
has locked with it in the path (leaping if too late to walk), and the adept and the expert walk
out of the lingering lava. Results (`--court wrath SKILL TIER SEED`, Ragnar's kit at the tier):

| Skill | Tier | Won | Time (s) | Deaths | Taken | His charges / dazed |
|---|---|---|---|---|---|---|
| expert | 1 | 3/3 | 123.7, 129.1, 127.0 | 0 | 0-0 | 18 / 2-10 |
| expert | 2 | 3/3 | 99.0, 100.5, 101.0 | 0 | 0-12 | 13-15 / 1-5 |
| adept | 1 | 0/3 | timed out (240) twice, lost once | 2-4 | 315-355 | 29-31 / 7-11 |
| adept | 2 | 0/3 | timed out (240) twice, lost once | 2-4 | 302-376 | 26-35 / 5-7 |
| novice | 1 | 0/3 | lost all lives in 48 s, in his first phase | 4 | 372 | 9 / 0 |
| novice | 2 | 0/3 | lost all lives in 59 s, in his first phase | 4 | 427 | 8 / 0 |

Pinned by `the_expert_beats_amon_in_his_court_and_a_masher_does_not` (tiers 1 and 2, no
overlaps, at most one fall; the masher loses).

### Belphegor

Built 2026-10-04. The court is `logic/court/sloth.rs` (the Hall of Stopped Clocks, 33 × 33, its
corners cut: 27 across), his fight `logic/combat/prince/sloth.rs` and `game/court/sloth.rs`, his
sculpt and his nightmares' `render/foes_court/sloth.rs`, the hall's art `render/view/court_sloth.rs`.

**The court.** Worn grey boards under fog (mist sheets and the haze, thickest at the walls), a great
faded rug in the middle; grey-blue hangings over a wainscot, small clocks hung on the walls (cuckoo
clocks among them), candles in iron sconces (some guttered), tall moonlit windows in the north
wall; four timber posts under the beams, cobwebs on beams, posts, clocks, chair and bed. Pieces of
the floor (rock): his great wing chair against the north wall (3 × 4), the giant hourglass before
it (off to his right, at (20, 10), so it does not stand between the gate and him), twelve tall
longcase clocks against the walls, the four posts, a canopied bed in the north-east. The drift of
sand in the south-west is walked over. While he lives every clock is stopped at its own hour and
the hourglass's sand hangs in mid-fall; when he falls the pendulums swing, the hands go round and
the sand runs ("THE CLOCKS START AGAIN").

**His fight.**

| Phase | Title | Moves | What else |
|---|---|---|---|
| 0 | ASLEEP | (none) | slumped in his chair; hide open; the first blow lands whole and wakes him (a phase change: roar, banner, cross) |
| 1 (> 2/3) | THE SLEEPER | Yawn, Swat, Sand of Hours | |
| 2 | THE DREAMER | (asleep) then Yawn, Swat, Sand of Hours | lies down where he stands; two nightmares, once |
| 3 (< 1/3) | THE WAKING | Hourglass Slam, Clock Hands, Yawn, Swat | wakes for good; slowest on his feet |

| Move | Startup | Active | Recovery | Damage | Shape | Rider / cooldown |
|---|---|---|---|---|---|---|
| Swat | 38 | 8 | 46 | 28 | cone 1.5 (+0.9), ±100° | |
| Yawn | 36 | 12 | 40 | 8 | cone 2.4 (+0.9), ±32° | shove 2.8 (half blocked, none parried); 240 |
| Sand of Hours | 36 | 1 | 40 | 16 | six heaps round him (one on him), 0.8 wide, bursting 16 + 50 f after | 270 |
| Hourglass Slam | 46 | 6 | 54 | 34 | cone 1.4 (+0.9); quake 2.6 (+0.9) at 50% | 300 |
| Clock Hands | 44 | 10 | 50 | 26 | cone 3.0 (+0.9), ±150° | 150 |

Pace (walk, rest): ×0.85, ×1.1 in his first phases; ×0.75, ×0.9 standing (slower than Mammon in
every phase: his reach makes up for it).

* **His sleep** is `prince::sloth::Slumber` on his `Lore` (Awake, Drowsy, Nodding, Asleep, Waking),
  stepped once a combat frame by `sloth_tick`. While he nods off, sleeps or wakes he is a puppet of
  his own torpor (no token, no walking, no move) and `Duelist::exposed` is true, so Torpor (still a
  hide, `is_hide`) lets the whole blow through, and the bar says HIS HIDE IS OPEN.
* **The Dreamer.** At two thirds he finishes his move, then lies down where he stands (40 f). Asleep
  he mends 0.8% of his health a second (a floater says so), never past the two-thirds line. Blows
  taking 6% of his health in one sleep, or any blow while the hero's finisher (L3) is out, wake him
  (54 f, open, "STRUCK AWAKE!"); 540 frames later, if still in his second phase, he nods off again.
* **The nightmares** are hexers now (they keep their distance and hex; health ×1.4 of the
  Hexer's), raised either side of him (`nightmare_spots`), weighted as the court's tier. They fall
  when he does.
* **His general.** Slain: no nightmares (he still dreams). Decree done (the hourglass wound): he
  wakes rested, no free blow (phase 1 from the first), and his nightmares come at once.

**What changed from the design, and why.** Phase 2's table is the Sleeper's moves once he is struck
awake (the design names no moves for it), and he nods off again after nine seconds awake, so the
phase stays about his sleep rather than one free window. The hourglass stands before the chair but
off his line to the gate (it hid him from the hero walking in). His waking from his chair is a phase
change (0 to 1), so it brings a banner and a breather like every other.

**Shared files touched.** `logic/combat/frames.rs` (Swat's real data; Yawn, SandOfHours,
HourglassSlam, ClockHands in Belphegor's blocks; the coin test counts at least two trap moves),
`logic/combat/duelist.rs` (`Lore.slumber`; `exposed` true while he sleeps or wakes),
`logic/combat/archetype.rs` (the nightmare a Hexer; Torpor's doc), `audio/voices.rs` (his own tell,
swing, cry and fall, and the nightmares'; his and theirs out of the kin arms), `render/view/duel.rs`
(his pose hook after `overshoot_duel`; Sloth's traps drawn as sand; three tell colours),
`render/view/animation.rs` (his shuffle, the nightmares' hover and sway), `game/bot.rs` (a sleeping
Belphegor is the most open target, and it leans into him to face him as it swings:
`Bot::belphegor_asleep`), `logic/guide.rs` (his page).

**Results** (`--court sloth SKILL TIER SEED`, three seeds):

| Skill | Tier | Win | Time (s) | Deaths | Taken |
|---|---|---|---|---|---|
| novice | 1 | 0/3 | lost in 62.8 (phase 1) | 4 | 363 |
| novice | 2 | 0/3 | lost in 70.7 (phase 1) | 4 | 440 |
| adept | 1 | 0/3 | timed out (240), phase 2 | 0 | 0-7 |
| adept | 2 | 0/3 | timed out (240), phase 2 | 0 | 0-24 |
| expert | 1 | 3/3 | 96.6, 102.6, 118.5 | 0 | 0-42 |
| expert | 2 | 3/3 | 88.2, 95.0, 106.4 | 0 | 0 |

The expert parries everything and strikes him whenever he sleeps; the adept blocks and strikes
little, and in his dream he mends faster than it hurts him: Sloth's lesson. Pinned by
`the_expert_beats_belphegor_in_his_court_and_a_masher_does_not` (game/court/sloth.rs).

### Beelzebub

Built 2026-10-04 in `logic/court/gluttony.rs`, `logic/combat/prince/gluttony.rs`,
`game/court/gluttony.rs`, `render/foes_court/gluttony.rs` and `render/view/court_gluttony.rs`.

**The court, the Banquet of Flies.** A long hall 27 tiles across (33 x 33 grid), floored in greasy
flags with a moth-eaten red runner up its middle. The great table runs round the floor in a U open
to the south: its arms down the west and east (x 6 and 26, y 8-23), its head across the north
(y 8) broken in the middle, seven tiles wide, where his way to the floor runs. Narrow lanes run
behind the arms (the hero can go round the table); four cauldrons over cold fires stand in the
lanes and by the gate; his throne of piled plates stands on a low dais behind the head of the
table, two green banners with a fly on them either side. Four platters stand on the arms (six
with the feast laid: two more on the head). The gate is in the south wall, the hero just inside
it, Beelzebub before his throne. Pinned by
`the_banquet_is_a_long_hall_entered_from_the_south_with_a_table_in_a_u_round_its_floor`.

**His fight** (HP x1.15 of the Prince's 900, else the Prince's numbers; the Glutton walks x1.3
and rests x0.65):

| Phase | Title | Moves, in the order he prefers them | What else |
|---|---|---|---|
| 1 (100-66%) | THE LORD OF FLIES | Swarm, Cleaver, Gorge | |
| 2 (66-33%) | THE SWARM | Scatter, Swarm, Cleaver, Gorge | two Fly Swarms rise from the food beside the platters nearest the hero (once) |
| 3 (< 33%) | THE GLUTTON | Belly Roll, Bile Spew, Belly Slam, Gorge | quicker |

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Cleaver | 34 | 6 | 44 | 26 | cone 1.4 (+0.9), +-55 deg |
| Swarm | 36 | 1 | 36 | 9 x5 | volley of five, +-0.5 rad, the flies at 62% of a bolt's pace (2.6-8 tiles) |
| Gorge | 60 | 1 | 44 | heals 9% (a cross 5%) | a spell; the court sends him to his food |
| Scatter | 64 | 6 | 48 | 28 | a mark of 1.3 following the hero for 40 f; he is flies (out of reach) from f12 to 8 f before it lands |
| Belly Roll | 40 | 46 | 46 | 30 | a charge of 7 tiles at 9.5 t/s (rock stops him; no daze) |
| Bile Spew | 38 | 1 | 38 | 16 | four traps (one under the hero) 0.85 wide, fuse 50, lingering 180 f |
| Belly Slam | 42 | 6 | 50 | 30 | cone 1.2 (+0.9), quake 2.4 (+0.9) at 45% |

* **Gorge, his meal** (`game::court::gluttony`). Under 85% of his health, between moves and 9 s
  after his last meal, he walks (a puppet of the court, his token given back) to the nearest
  green cross that has lain 2.5 s on the floor, else the nearest laden platter, and eats: a 60 f
  spell whose first active frame heals him (never past his phase's threshold). A blow while he
  eats chokes him: staggered for 100 f (his bloat open) and the platter spoiled. Each platter
  feeds him once; a cross he eats is gone. He gives up a walk of more than 10 s or food taken from
  under him. A fall lays the table again.
* **His flies** leave his hand at a bolt's pace and are re-planned at once by the threat manager
  for their own (62%); if no frame is clear they keep a bolt's pace and its plan.
* **His scatter** is the court's too: the mark follows the hero (no faster than `MARK_FOLLOW`),
  he goes up in flies (smoke, untouchable, drawn as a cloud) drifting to it, and gathers on it a
  moment before his slam. Nothing he does slows or holds the hero.
* **The decree** (the Steward's feast laid): six platters, and the swarms at once; **the Steward
  slain**: no swarms.

**What changed from the design, and why.** Gorge sits in every table (for its place among his
moves), but no range chooses it: the court sends him to his food. The heal stops at his phase's
threshold, so a meal never turns his phase back. The head of the table is broken seven tiles wide
(not five) so his bulk never wedges on its corners. His health is 1.15 times the Prince's: at 1.0
the expert's tier-2 kit felled him in under 90 s. The Fly Swarm is a skirmisher a quarter quicker
than the archetype ("quick and light"). His bile is drawn as bile (the view picks it for his
court), not Mammon's coins.

**The bots.** The adept and the expert take a green cross off the floor whenever nothing they
have seen lands within half a second (`Bot::banquet`), and the expert strikes him while he eats
(`Bot::feasting`); neither changes their play in any other fight.

**Results** (`--court gluttony SKILL TIER SEED`, Ragnar's kit at the tier; 240 s limit):

| Skill | Tier | Seed | Result | Time (s) | Deaths | Taken | Meals eaten / choked |
|---|---|---|---|---|---|---|---|
| expert | 1 | 1 | slain | 113.5 | 0 | 33 | 1 / 1 |
| expert | 1 | 2 | slain | 117.2 | 0 | 10 | 1 / 1 |
| expert | 1 | 3 | slain | 118.5 | 0 | 10 | 1 / 1 |
| expert | 2 | 1 | slain | 100.6 | 0 | 11 | 1 / 1 |
| expert | 2 | 2 | slain | 90.4 | 0 | 0 | 1 / 1 |
| expert | 2 | 3 | slain | 93.8 | 0 | 0 | 1 / 1 |
| adept | 1 | 1 | timed out in phase 2 | 240 | 0 | 182 | 0 / 4 |
| adept | 2 | 1 | timed out in phase 2 | 240 | 0 | 69 | 0 / 4 |
| novice | 1 | 1 | lost (game over) | 44.6 | 4 | 354 | |
| novice | 2 | 1 | lost (game over) | 49.9 | 4 | 460 | |

Pinned by `the_expert_beats_beelzebub_in_his_court_and_a_masher_does_not` (tiers 1 and 2: slain
through all three phases, lives to spare, no overlaps; the masher loses every life).

### Asmodeus

Built 2026-10-04. **The Rose Bower** (`logic::court::lust`): a walled garden 27 tiles across
(33 × 33 grid, corners cut by rose beds), its walls rose hedges; the fountain's basin covers the 13
tiles about the middle (between the gate and the prince, cover against the kiss and the charge);
four censers at (9, 11), (23, 11), (9, 21), (23, 21), each open all round and at least six tiles
from the next; two hedge runs out from the east and west walls (x 5 and 27, y 13 to 19), the
elf-maids step out beside them; rose beds at the four corners; the bridal arch's two posts at
(14, 4) and (18, 4) (the way under it open) and the bed behind it (15 to 17, 3). Gate (16, 30), hero
(16, 26), Asmodeus (16, 8). Art in `render::view::court_lust` (hedges of roses, rose-marble paths
crossing a lawn of petals and ringing the fountain, its golden rose and arching water, bronze
censers on marble plinths, lanterns, the arch draped in veils, the canopied bed and its candles,
moonlight); his sculpt in `render::foes_court::lust` (rose silk, three heads, goose's feet, a
serpent's tail that is a rig part of its own, the lance and its pennon); his voice and his sounds
per move (the man's purr and laugh, the kiss, the bull's bellow, the ram's bleat, thorns).

| Move | Startup | Active | Recovery | Damage | Shape | Cooldown |
|---|---|---|---|---|---|---|
| Lance Thrust | 34 | 6 | 40 | 22 | cone 2.2 (+0.9), ±20° | |
| Blown Kiss | 36 | 1 | 36 | 8 | one bolt (a heart), from 3.2-8 tiles | 330 |
| Thorn Line | 36 | 1 | 38 | 18 | five thorns in a row along his aim, 0.7 wide, 1.3 apart, fuse 40, from 3-7 tiles | 300 |
| Bull's Charge | 40 | 46 | 46 | 26 | charge 7 tiles at 9 t/s, from 3.2-7 tiles, guard blocks or parries | 300 |
| Ram's Butt | 32 | 6 | 38 | 16 | cone 1.0 (+0.9), ±50°, shoves 2.5 tiles | 180 |
| Thorn Ring | 40 | 1 | 40 | 20 | nine thorns in a ring 2.15 tiles round him, 0.8 wide, fuse 46, within 3.4 | 300 |

Phases: the Suitor (kiss, thorn line, lance), the Three Heads (charge, kiss, butt, lance; two
elf-maids from the hedges, once), the Bridegroom of Ashes (thorn ring, charge, kiss, lance; ×1.35
walk, ×0.6 rest). His health, poise and pace are the Prince's own (900 at tier 1). The banners say
what to do ("LIGHT A CENSER, STRIKE THE MAIDS' BACKS"; "STEP OUT, LIGHT A CENSER, THEN STRIKE"), and
the first moment of the fight puts "STRIKE A CENSER" over the nearest one.

* **The censers** (`game::court::lust`): a blow of his swing on a cold censer lights it for 600
  frames (one censer for one swing); each lights twice (once if the Bridesmaid's vows were spoken),
  then is burnt out. While any smokes his Allure is gone (his breed's power is set to none, and
  back as the last goes cold) and he backs out of three tiles of every smoking one at 3.2 t/s (not
  in the middle of a move). The censers' state is the court's (`Court::bower`, private to the
  court); the scene learns of the smoke from an ember the fight leaves in each smoking bowl (a
  `Hazard` of no width on the censer's own rock tile: no one can stand in it).
* **The kiss**: Asmodeus's bolt (`BoltLook::Kiss`, a beating pink heart), paced as every bolt to
  land on the frame the threat manager planned. It is met by the court the frame before it would
  touch him: no shield stops it and no parry turns it (as `general_hazard_blow`); in a leap it
  passes under him; his blade in its active frames cuts it out of the air. If it strikes him he is
  smitten for 90 frames: his guard will not rise (his guard button is ignored), and he walks, leaps
  and swings as ever.
* **The thorn ring**: the game throws its traps about the hero as for any `Shape::Traps`; the court
  lays them round him instead (the threat manager's landing unchanged).

**Changes from the design.** The kiss goes through the shield (the design says it is leapt or
struck from the air; were it blockable the guard would be the whole answer and "smitten" would
never come). The elf-maids serve him lighter than in their garden (0.8 of their health, 0.6 of
their blows): no blow tells on their fronts, and two of them hunting with the prince were the
bots' deaths. The kiss and the thorn line begin no nearer than 3.2 and 3 tiles, so between them
and his lance's reach he closes in to thrust rather than standing off throwing (the expert's time
went to nothing there). The Bull's Charge does not daze him, as designed.

**The bot** (`bot::lust_court`, the adept's and the expert's, only in this court): it leaps a kiss
on its line in its last 10 frames (out of the line), leaps what it would have guarded while
smitten, guards the bull's charge as the runner comes (the expert into its parry window; nothing
else in the bot knows a charge), leaves the elf-maids alone, and when no censer smokes, nothing it
has seen comes within 45 frames and he is not mid-move, walks to the nearest censer that will
still light and strikes it.

His court (three lives, `--court lust SKILL TIER SEED`):

| Skill | Tier | Seed 1 | Seed 2 | Seed 3 |
|---|---|---|---|---|
| expert | 1 | won 134.8 s, no fall, took 6 | won 120.2 s, no fall, took 39 | won 122.4 s, no fall, took 39 |
| expert | 2 | won 96.0 s, no fall, took 45 | won 109.8 s, no fall, took 63 | won 89.5 s, no fall, took 96 |
| adept | 1 | timed out (240 s) in phase 2, 1 fall | timed out in phase 2, 1 fall | timed out in phase 2, 1 fall |
| adept | 2 | timed out in phase 2, 2 falls | timed out in phase 3, 1 fall | timed out in phase 2, 2 falls |
| novice | 1 | lost all three lives in phase 1, 48.7 s | the same | the same |
| novice | 2 | lost all three lives in phase 1, 56.4 s | the same | the same |

Pinned by `the_expert_beats_asmodeus_in_his_court_and_a_masher_does_not` (game::court::lust).
