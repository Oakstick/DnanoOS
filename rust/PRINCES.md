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

(to come)

### Asmodeus

(to come)
