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

(to come)

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

(to come)

### Belphegor

(to come)

### Beelzebub

(to come)

### Asmodeus

(to come)
