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

(to come)

### Amon

(to come)

### Belphegor

(to come)

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

(to come)
