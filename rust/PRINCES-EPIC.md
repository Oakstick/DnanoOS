# The princes made epic: a design for every court and for Krampus (B43)

BACKLOG.md B43: "more epic boss battles for every prince". Today a court is a round arena, three
phases and one to two minutes for the expert (WARBAND.md W10/W11); New Game+ makes it longer
(WARBAND.md, New Game+). This is the design for the user to approve before anything is built.
**No game code changes with it.** Status: proposal, 2026-10-10 (branch `princes`, from `rust-port`
at 55788d3).

Read with: PRINCES.md (the courts and the frame data as built), COMBAT.md §0 and §13 (the floors),
REALMS.md (the realms' lies), STORY.md (Toby, Hızır, the boys), FINALE.md (Krampus), BACKLOG.md B11,
B12, B14, B15, B33, B41, B44, B56.

---

## 0. What "epic" means here, and the rules it keeps

**What every court gets** (the shared toolkit, §9):

* an **entrance**: 5-15 s, a composed shot, a line in his own voice, his name card;
* an **arena that changes** at each phase, tied to his sin;
* **scale**: he grows, towers or shows a giant form, in at least his last phase;
* **one or two set pieces** built on his realm's lie and his generals' instruments (B41);
* a **finale**: a short fourth phase carved from the bottom of his bar, the spectacle of the fight;
* a **death worth the wait** and **the seal-stone** (B44);
* his **theme** (B14), built up phase by phase.

**The floors stay** (COMBAT.md §0, PRINCES.md §0). Every new move below keeps them:

* startup 30 f or more for a prince (18 f for an add);
* recovery 30 f or more, his hide open;
* no two landings within 18 f (300 ms): the threat manager plans every one, set pieces too;
* what lands from afar shows where it lands, with a fifth of the time to spare;
* nothing slows the hero, holds him or turns his controls. Mammon's grasp stays the one hold. A
  push (`Rider::Shove`) is allowed; so is a telegraphed, leapable loss of guard (Asmodeus's kiss);
* a set piece never takes the camera while anything is planned to land (§9.4, the hush).

**Epic, not longer.** The adept already takes 5.8-7.7 minutes at the late courts (W10's table), and
the try ends at 8 (`COURT_LIMIT`). So:

* the finale is **carved from the existing bar** (a fourth table below 12-15%), not added health;
* set pieces and transitions add **at most 45 s** a court;
* where the adept is over 6 minutes at his own kit today (Mammon met seventh, Beelzebub sixth,
  Asmodeus seventh, Leviathan fifth), that prince's health is trimmed 10-15% (`court::temper`), and
  measured;
* the entrance is not counted: it plays before the fight's clock starts.

Targets for every prince below: **the expert 2.5-3.5 minutes** in his court (today 1.1-1.9 in the
run, 90-150 s in `--court` at tiers 1-2), **the adept 4-7 minutes** at his kit where the run meets
him (today 2.4-7.7), the masher still unable to fell him.

**The family-adventure tone.** Menacing, never gory. No blood, no dismemberment, no Devil or
Satan anywhere. A prince's death is a fading, a sinking, a turning to ash, petals, sand or
stars. A little humour where the sin allows it (Mammon's grabbing, Belphegor's snore, Beelzebub's
appetite). Hızır always leads to good; no cheat takes his form.

**B41 changes the generals' part, and this design leans on it.** With three generals' keys
needed to open every court, **all three generals are always slain before the prince**. Today's
`PrinceTerms` ("his general slain: no adds") would then remove every prince's adds in every run.
Proposed instead (a decision for the user, §11):

* the adds always come in the second phase (they are part of the spectacle);
* **each general's instrument returns in the court** as a set piece: the thing the hero learnt to
  break in the realm (the horn, the bell, the lamp), now in the prince's hand, and breakable again
  in three blows, as in the realm;
* a general's **decree**, if it finished before he fell, still adds one twist to the court, as
  today.

The generals' names and instruments are B41's (the realms job, in parallel). Where this design
names an instrument, it uses today's general (WARBAND.md §9.3); the builder aligns the names with
B41 when both land.

**Reference captures** of today's courts, one wide frame each (release build of 1.0.24, drawn at
3 s so the probes are in), are in `C:/Users/ulask/Games/vh-princes-cap/` (not committed):
`court-pride.png`, `court-envy.png`, `court-wrath.png`, `court-sloth.png`, `court-greed.png`,
`court-gluttony.png`, `court-lust.png`, `court-krampus.png`. What they show: a 3 m prince on a
27-29 tile floor is a small figure at the far end from above (about 3% of the frame's height);
Envy, Sloth, Gluttony, Wrath and the lair read very dark from above (B11/B12 still to come). Both
argue for scale and for light that changes with the phase.

---

## 1. Lucifer, Pride: the Throne of the Morning Star

1. **The fantasy.** Duel the most beautiful angel there ever was in a hall of mirrors that breaks
   round him, until the Morning Star falls burning out of his own sky.

2. **The entrance** (10 s).
   * The hero steps in. The hall is empty, the throne is empty. The camera sits low on the
     mirror-polished floor, the hero's reflection upside down beneath him.
   * The star window brightens to white. A figure comes down the shaft of light from it, wings
     folded, and lands on the steps without a sound.
   * His six wings open a pair at a time: three beats, three high chimes, the shafts of light
     swinging with them. The camera cranes up from behind the hero to his face, then cuts to his
     reflection in the floor.
   * **"Kneel, little one. Everyone kneels to me in the end."**
   * Name card: **LUCIFER · THE MORNING STAR · PRINCE OF PRIDE**.

3. **Scale.**
   * Phases 1 and 2 as today (3.1 m).
   * Phase 3, THE SHATTERED STAR (D5): he rises to **1.4×** (4.3 m), floating a hand above the
     floor, his burnt wings spread across eight tiles (looks only; his sim radius 0.95 → 1.2, his
     arm +0.9 → +1.1).
   * The finale: **his reflection, a giant of white light, fills the star window** in the apse
     (a background figure about 12 m tall, not a body: it never collides and is never struck). It
     is the source of the finale's beams, so the tells are drawn on the floor where they land.
   * Camera: the tall framing of §9.3 in both views. In the chase view the hero fights facing
     north, toward the window, so the giant is in frame; in the top view the apse is at the top of
     the frame.

4. **The arena by phase.**
   * **1 The Morning Star**: as built, white and gold, the six mirrors whole.
   * **2 The Mirror**: the Reflections step out of the mirrors and leave cracks behind; each Mirror
     Step cracks one more. The light cools toward blue.
   * **3 The Shattered Star**: in the phase change he cries out and **all six mirrors shatter
     outward** (glass debris, a two-second hush in slow motion). The star window cracks. Radial
     cracks run out from the star inlay across the floor, and **four great shards of mirror stand
     up out of the floor** (2 × 1 rock each, marked two seconds ahead): new cover against the
     lance. The light turns from white-gold to ember and violet; the ceiling opens on a night sky
     where stars fall.
   * **Finale**: the outer ring of the floor (more than 11 tiles from the middle) falls away into
     the starry dark. The hall closes from 29 tiles across to 22: "how art thou fallen".

5. **Set pieces.**
   * **The Chamberlain's mirror-lines** (his Kindle instrument, from the Hall of Mirrors). In
     phase 2 Lucifer touches a mirror and light runs between pairs of mirrors, straight lines
     across the hall, glowing 60 f before they fire. The hero steps across them. **A mirror struck
     three times cracks and dies**: its line is gone, and Lucifer can no longer Mirror Step out of
     it. The realm's lesson (break the instrument) pays off in the court.
   * **"Take your throne."** (The realm's lie, kept by D11.) In phase 3 the empty throne glows
     violet; the lie appears in violet in the prince's voice, without Quitmer's bark: "TAKE YOUR
     THRONE." A lane of light runs from the throne down the steps to the middle. The hero who
     stands in it a second "is crowned": the throne-mirror fires a Lance of Dawn down the lane
     (the usual tell, the usual floors). Hızır's amber counsel tells the truth: "That chair has
     never held anyone. Stay out of its light."

6. **The finale: HOW ART THOU FALLEN** (below 15%).
   * A hush: he flies straight up through the open ceiling (untouchable for two seconds), and the
     giant reflection blazes in the window.
   * **Dawnbreaker**: the giant's gaze sweeps the floor as five beams, fired one after another
     round a fan (each line glows 50 f, they fire 24 f apart). The gaps between them are safe, and
     the order is always clockwise.
   * **Pride's Fall, three times**: he dives on a mark that follows the hero, three falls 60 f
     apart, each with its 30 f open recovery; the last one cracks away the floor's outer ring and
     leaves him on his knees for **120 f**: the punish the whole fight builds to.
   * Then Starfall and the Radiant Cut, and the cycle again.
   * **His death.** He stands and sees himself in a shard: small and grey. His wings fall apart
     into ash-feathers that drift up; his crown of light goes out. He kneels and comes apart into
     falling stars that streak upward. The star window goes dark, then fills with a soft dawn.
   * **The seal-stone**: a violet star-cut amethyst, which falls where his crown went out and lands
     ringing among the shards, seven reflections of it round the hero.

7. **His theme.** Celestia Ardent 2 (B14). Entrance: the track's opening under the low-pass, opening
   up as the name card lands. Phase 2: the war drum's heartbeat. Phase 3: full drum, +3% varispeed,
   a glass-shatter stinger. Finale: seek to the track's climax bar.

8. **References.**
   * Elden Ring, Rennala: the phase change that turns a library into a starry void. Borrow the
     transformation into the night sky.
   * Zelda, Phantom Ganon (Ocarina): he rides out of paintings. Borrow the read of the Mirror Step
     (the mirror that lights is where he comes out).
   * Dark Souls III, Pontiff Sulyvahn: the phantom copy. Borrow how the copy differs at a glance
     (the Reflections: silver, left-handed, no gold halo).
   * Bloodborne, Micolash: the mirror chase. Borrow the hall's mirrors as his doors.

9. **Fairness and numbers.**
   * Expert 2.5-3 min (95-106 s today), adept 4-6 (176 s met second, 349 s met sixth).
   * Phases: 3 and the finale (15%). Thresholds: 66% / 33% / 15%.
   * Stays: every move, Vainglory (a blow from behind lands whole), the Reflections.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Mirror-Lines (phase 2) | 60 | 8 | 40 | 20 | two fixed beams between mirror pairs, 0.9 wide, glowing through the startup; cooldown 360 |
| Wing Gale (phase 3) | 38 | 10 | 44 | 6 | cone 3.0, ±40°, shove 2.5 tiles (leapable, blockable) |
| Dawnbreaker (finale) | 50 | 8 ×5 | 60 | 22 | five fixed beams round a fan, fired 24 f apart, each glowing 50 f |
| Pride's Fall ×3 (finale) | 56 | 6 | 30, 30, 120 | 30 | the built mark, three falls 60 f apart |

10. **Cost: M.** The shatter (phase-variant dressing and debris), the floor ring falling away (tile
    edits), scale 1.4, the window giant (one emissive background sculpt, simple: light, not
    detail), the mirror-lines and the throne lane (beams from the court). Pairs with B11's Pride
    pass (next in B11's order): build both in one job.

11. **The tone.** A beautiful, cold, proud angel; his fall is sad, not cruel. No blood, no Devil:
    he is Pride's prince and nothing more.

---

## 2. Leviathan, Envy: the Drowned Bell

1. **The fantasy.** Fight a sea serpent so big you only ever see a piece of him: the temple floods,
   the bell burns, and the whole drowned hall is his coils.

2. **The entrance** (12 s).
   * The hero steps into the drowned temple. The water ring is still, the bell silent.
   * The camera skims low over the water. A ripple; then a long hump of scales slides through the
     water **all the way round the ring**, and the camera turns to follow it, a full circle in
     four seconds. His length, implied.
   * The bell swings by itself and tolls once. The nazar beads tremble on their cords.
   * His head bursts out of the water before the altar in a fountain of spray, the crest raised,
     the slit eyes narrowing.
   * **"You came all this way for a friend? I never had one. I'll have yours."**
   * Name card: **LEVIATHAN · THE CROOKED SERPENT · PRINCE OF ENVY**.

3. **Scale.** The duelist stays his head and front third (3.1 m), as built. The giant is **his
   body**: a background figure of 20-30 segments that moves along paths round the hall, never
   collides and is never struck.
   * Phase 1: a hump or two in the water ring.
   * Phase 2: coils rise out of the water and arch round the walls, five metres high, threading
     behind the columns.
   * Phase 3: coiled round the bell's posts, as he coils round the bell.
   * Finale: the head grows **1.3×** and the body arches **over the floor** (an arch about eight
     metres high).
   * Camera: the body fades where it comes between the camera and the hero, as trees do; the arch
     is left out of the chase camera's collision spans. `--camera-check` with the body in every
     phase's pose.

4. **The arena by phase.**
   * **1 The Crooked Serpent**: as built.
   * **2 The Evil Eye**: **the temple floods**. The water rises from the ring over the whole floor,
     ankle-deep. It is looks only, as it is today: it never slows. The inlays and every telegraph
     are drawn through it. Each covet cracks a nazar bead (a counter of his envy).
   * **3 The Bell**: the bell glows red-hot; steam rises from the water round it; the burnt side's
     hide rule (as built) is lit red on the floor.
   * **Finale**: the water drains in a slow whirlpool round the bell (looks only), uncovering the
     cracked temple floor, and the body arches overhead.

5. **Set pieces.**
   * **The Bell-Warden's toll** (his Muster instrument, Kiyohime's bell). At the start of phase 2
     Leviathan strikes the bell with his tail; the drowned rise from the springs (the serpentlings,
     as built). The bell's rope hangs to the floor: **three blows on the bell's clapper rope
     silence it**, and it raises no more. If the Bell-Warden's decree finished (the bell lowered),
     the flood comes in phase 1.
   * **He covets everything.** B58 gives every shrine all four archangels, so the hero has likely
     used all four by the finale. At the finale he covets them all at once: "HE COVETS EVERYTHING
     YOU HAVE". He throws the four coveted moves in order (fire, thunder, beam, wave), at least
     40 f apart, each with the tell the player learnt in phase 2, then sags for **90 f**, scales
     dull: he cannot hold what he took.

6. **The finale: THE DRAGON THAT IS IN THE SEA** (below 15%, Isaiah 27:1).
   * **Covet All** (above).
   * **Coil Crush**: a coil of the background body slams across the floor from wall to wall: a beam
     12 × 1.4 glowing 60 f, then the coil **lies there 90 f as rock**, splitting the floor, and
     slides away.
   * Bell Toll, Coil Lash.
   * **His death.** He rears up as the bell cools. He sees his own reflection in the still water,
     alone. He sinks beneath it without a splash; the coils slide off the walls and away into the
     dark; the water stills to a mirror; the cracked nazar beads glow whole and blue again. The
     cooled bell tolls once, softly.
   * **The seal-stone**: an emerald like a serpent's eye, which surfaces in rings of ripples and
     floats to the hero.

7. **His theme.** Celestia Ardent 4 (the darkest, the widest swells). Entrance on a quiet passage;
   phase 2 on a swell, with the drum's heartbeat; phase 3 the full drum; the finale on the biggest
   swell; the death on the bell alone.

8. **References.**
   * Elden Ring, Rykard: a serpent bigger than his arena. Borrow the body as scenery and the head
     as the target.
   * God of War (2018), Jörmungandr: scale shown by a fragment. Borrow the hump circling the ring.
   * Zelda, Morpheel (Twilight Princess): a water arena with the giant beneath. Borrow the flood.
   * Shadow of the Colossus, Dirge: the trail through the surface before the strike. Borrow it for
     the Dive's ripples.

9. **Fairness and numbers.**
   * Expert 2.5-3 min (101-120 s today), adept 4-6.5 (245 s met third, 335 s met fifth: **health
     trimmed 10%**, as the longest of breed).
   * Phases: 3 and the finale (15%).
   * Stays: every move, Covet, Scales and the burnt side, the serpentlings.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Covet All (finale) | 44 each | as each | 40 between, 90 last | as each | the four coveted moves in order, ≥ 40 f apart; once per 20 s |
| Coil Crush (finale) | 60 | 8 | 46 | 26 | beam 12 × 1.4 wall to wall, then rock for 90 f; cooldown 360 |

10. **Cost: L.** The background body (a segmented giant rig on splines, fading for the camera) is
    the one big new piece; the flood plane is shared with Mammon's coin flood (§9.2); Coil Crush
    needs the transformation framework's temporary rock.

11. **The tone.** Lonely more than cruel. "I never had one" is the heart of him; his death is a
    quiet sinking.

---

## 3. Amon, Wrath: the Anvil of Wrath

1. **The fantasy.** A wolf-and-raven warlord whose rage breaks the ground under you: bait the beast
   into stone until his battlefield is ruins and he is the last thing standing.

2. **The entrance** (8 s).
   * Drums. Amon kneels at his anvil, hammering a half-forged blade with the back of his axe; each
     blow throws sparks and lands **on the beat** of his track (the beat clock knows it).
   * He stops. His raven head lifts and he roars, fire in his throat lighting the hall red.
   * He drives the blade into the anvil and takes his axe.
   * **"Hit me. Go on. Make me angry."**
   * Name card: **AMON · THE WOLF · PRINCE OF WRATH**.

3. **Scale.** **His rage is his size**: each rage step +4% (up to +20% at five), and a crash into
   stone shrinks him back as it empties his rage. Phase 3, THE FURY, starts at **1.25×** (3.9 m)
   with a mane of fire; the finale at **1.4×**. His sim radius grows with it (0.95 → 1.3); his
   cones are measured from his edge, so their telegraphs grow with him. Camera: the tall framing.

4. **The arena by phase.**
   * **1 The Wolf**: as built, four pillars. **The pillars break**: two crashes crack a pillar, a
     third brings it down to a low mound of rubble (still solid, no longer tall). The same tech as
     the Cyclops' cover (B57).
   * **2 The Raven**: his rifts leave scars that glow for the rest of the fight; the burning
     banners drop embers; the ravens come.
   * **3 The Fury**: the Eruption at the phase change **splits the floor**: four lava channels open
     along the pre-drawn seams from the anvil to the wall (marked 120 f ahead, then lingering lava
     one tile wide, stinging and never slowing), and the floor slabs between them **tilt up as new
     stone** (2 × 1 rock): fresh walls to bait him into as the pillars fall.
   * **Finale**: the war banners burn through and fall; the anvil cracks; the hall is ruins.

5. **Set pieces.**
   * **"Sound the horn and the battle ends."** (The realm's lie, kept.) Marshal Gorebrand's war
     horn (his Muster instrument) hangs on a post by the anvil. In phase 2 the lie comes in red, in
     the prince's voice: "SOUND THE HORN AND IT ENDS." The hero who strikes it sounds it, and it
     brings three more ravens and fills Amon's rage to five. Hızır, in amber: "Leave the horn. It
     only ever called more."
   * **The bridges burn.** If Gorebrand's decree finished (the sappers burnt the bridges), the lava
     channels open in phase 2 instead of phase 3.

6. **The finale: ASH AND IRON** (below 15%).
   * His rage fills and locks: nothing empties it now.
   * **Wildfire Charge**: three charges running, each aimed and locked 14 f before he sets off, a
     30 f open stop between them; the third always runs at the cracked anvil's dais and he crashes
     into it, **dazed 180 f**: the finale's punish.
   * **Slab Toss**: he tears up a slab and hurls it: a mark 1.4 wide that lands 50 f later.
   * Flame Breath, Eruption.
   * **His death.** The axe drops from his hands. The fire in his throat gutters out. He looks at
     his hands, the rage gone, and sits down heavily against the cracked anvil: **"...so tired of
     being angry."** The raven head bows, and he turns to embers that rise like a flock of small
     ravens and fly off into the dark. The lava cools to black glass; the banners' fires die.
   * **The seal-stone**: a ruby that glows like a coal and never burns, lying on the anvil where the
     half-forged blade was.

7. **His theme.** Scavenger Guild (150 bpm, B14). The entrance's hammer on its beats; phase 2 the
   drum's heartbeat; phase 3 the full drum; the finale +4% varispeed; the death: silence, then
   the embers' crackle.

8. **References.**
   * Sekiro, the Demon of Hatred: a rage boss in an arena that burns. Borrow the burning field's
     escalation.
   * Dark Souls III, the Abyss Watchers: the ring of fire and the music swelling into phase 2.
     Borrow the phase change.
   * Hades, Asterius: the charging bull. Borrow the readable charge lines and the open stop.
   * Monster Hunter's wall slams: bait into stone. Borrow the reward (a long daze) and that the
     walls are worn out by it.

9. **Fairness and numbers.**
   * Expert 2.5-3 min (99-129 s today), adept 5-6 (264 s met fourth).
   * Phases: 3 and the finale (15%).
   * Stays: every move, the daze, rage (now also his size), his armour 70, the ravens.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Wildfire Charge (finale) | 40 (tracks 26, locked 14 f before) | 40 ×3 | 30, 30, then crash 180 | 26 | three charges of 7 tiles at 11 t/s; the third at the dais |
| Slab Toss (phase 3 and finale) | 46 | 6 | 44 | 24 | a mark 1.4 wide, landing 50 f after the throw, from 3-8 tiles |
| Ground Split (phase change) | in the hush | — | — | stings only | four lava channels, marked 120 f, then lingering lava |

10. **Cost: M-L.** Breakable pillars (shared with B57), lava channels and tilted slabs (tile edits:
    hazard and rock), rage as size (cheap), the horn (a court instrument).

11. **The tone.** Wrath, felt and then let go. His last line is the lesson the boys spoke after him
    (STORY.md: "you can be angry and still be kind").

---

## 4. Belphegor, Sloth: the Hall of Stopped Clocks

1. **The fantasy.** A giant who would rather sleep than fight you, in a bedroom where time has
   stopped: wake him, and the whole stopped century comes crashing down at once.

2. **The entrance** (10 s).
   * The fog parts. Snoring. Every clock shows a different hour, and none moves.
   * The camera drifts (a slow, soft-focus push) through hanging cobwebs to the great chair:
     Belphegor asleep, a stopped pocket watch on his belly rising and falling with his breath.
   * One grain falls in the giant hourglass: a single tick.
   * In his sleep: **"Five more minutes... five more centuries..."**
   * Name card: **BELPHEGOR · THE SLEEPER · PRINCE OF SLOTH**.
   * The fight begins as built: asleep until struck, the first blow whole.

3. **Scale.** In phase 3, THE WAKING, he stands up **and keeps on standing**: he unfolds to
   **1.5×** (4.65 m), his head among the beams (which fade for the camera). "Slow, vast reach"
   made literal: sim radius 1.3, arm +1.2. Camera: the tall framing; the top view climbs 1.5.

4. **The arena by phase.**
   * **0-1 Asleep, the Sleeper**: as built.
   * **2 The Dreamer**: when he lies down, **the dream leaks**. The walls drift back into the fog
     (the hall seems bigger), the furniture bobs a little in the air, the floor fog glows soft
     lavender, the clocks' hands turn backward. All looks only.
   * **3 The Waking**: **every clock starts at once** with a deafening chime. Then the **longcase
     clocks topple**, one every four seconds or so: a clock's pendulum glows, it tilts for 60 f
     and falls along its line (a landing planned by the threat manager), and lies there as rock: a
     new wall. At most four fall.
   * **Finale**: **the hourglass turns** (the realm's climax: "sleep resets the realm a century
     older"). In seconds the room ages a hundred years: cobwebs thicken, the hangings rot to grey
     rags, the boards sag, dust everywhere (an atmosphere blend and a dressing swap).

5. **Set pieces.**
   * **The Lamplighter's lamps** (his Kindle instrument, the torpor lamps). In phase 2 the four
     timber posts carry his lamps, burning lavender. **While a lamp burns, Belphegor mends twice
     as fast in his sleep.** Three blows snuff a lamp, as in the Mire. If the Lamplighter's decree
     finished (the hourglass wound), he wakes rested, as built.
   * **The hourglass turns** (the finale, below).

6. **The finale: A CENTURY OLDER** (below 15%).
   * He turns the hourglass over: **30 seconds of sand**, shown in the glass and on his bar.
   * While it runs he is at his quickest: Clock Hands, Hourglass Slam, the toppling clocks, and
     the **Long Yawn** (a great shove, no damage, leapable).
   * When the sand runs out **he falls asleep on his feet: 6 seconds open, his hide gone**, the
     finale's punish. If he still stands after it, he wakes and turns the glass again.
   * **His death.** He yawns hugely, sits back into his chair (which crumbles to dust under him),
     and falls asleep for good: he fades into the fog like a dream on waking, a contented snore
     fading with him. All the clocks strike midnight, then tick on as clocks should ("THE CLOCKS
     START AGAIN", as built); morning light comes in at the windows.
   * **The seal-stone**: a slate-blue moonstone, which falls with the hourglass's last grain.

7. **His theme.** Level One Boss (83 bpm, the drone; B14). The dream: low-passed and slowed a
   little. The waking: the clocks' chime as the stinger, then the full track. The finale: the
   ticking locked to the beat under the drum.

8. **References.**
   * Zelda, Majora's Mask: the clocks and the falling moon. Borrow time made visible as the threat
     (the hourglass on his bar).
   * Bloodborne, Rom the Vacuous Spider: a passive giant whose minions fight while you strike.
     Borrow the dreamer's phase.
   * Hollow Knight, the dream bosses: the world turned dreamlike. Borrow the leaking dream.
   * Rip Van Winkle: a sleep of a century. Borrow the room aging in seconds.

9. **Fairness and numbers.**
   * Expert 2.5-3 min (88-118 s today), adept 4-6 (223 s met third, 314 s met fifth).
   * Phases: asleep, 3, and the finale (15%).
   * Stays: every move, his sleep and mending (the lamps double it), the nightmares, Torpor.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Toppling Clock (phase 3, finale) | 60 (the tilt) | 6 | — | 24 | a court hazard: beam 3.2 × 1.2 from the clock's foot, then rock |
| Long Yawn (finale) | 40 | 14 | 50 | 0 | cone 3.4, ±40°, shove 3.0 (leapable, half blocked) |
| Asleep on his feet (finale) | — | — | 360 | — | open, hide gone, when the 30 s of sand run out |

10. **Cost: S-M.** The clocks' start already exists; toppling clocks (dressing to rock), the dream
    and the century (atmosphere and dressing blends), scale 1.5, the lamps (the realm's instrument
    rules). The cheapest big gain.

11. **The tone.** The gentlest prince: funny, drowsy, tempting. His death is falling asleep.

---

## 5. Mammon, Greed: the Counting House

1. **The fantasy.** Fight a fat, smiling king who owns every coin in the room, until the hoard
   itself rises to bury you.

2. **The entrance** (9 s).
   * The gate locks behind the hero. Darkness.
   * One coin rolls out of the dark across the polished floor to the hero's boots (the camera low,
     following it), spins, and falls flat: Mammon's face.
   * The braziers on their chains flare one by one round the octagon, each with a counting tick:
     one, two... eight.
   * The camera cranes up over the giant scales: Mammon on his throne, counting coins from one pan
     into the other. He stops. The pans swing level.
   * **"Everything has a price, little warrior. Let's find out yours."**
   * Name card: **MAMMON · THE BANKER · PRINCE OF GREED**.

3. **Scale.**
   * Phases 1 and 2 as today.
   * At phase 3, THE HOARD, he calls the gold: streams of coins fly from every heap into him (a
     two-second hush), and he swells to **1.35×** (4.2 m), gold plates over his robe, his crown a
     tier higher. Sim radius 0.95 → 1.2, arm +0.9 → +1.1.
   * Finale: **1.5×**. Every stagger knocks coins out of him and shrinks him a little (−0.05, never
     below 1.2): the hero sees his blows tell.
   * Camera: the tall framing in both views.

4. **The arena by phase.**
   * **1 The Banker**: as built (already at the Cisterns' bar, B11).
   * **2 The Collector**: the strongroom doors either side of the gate grind open and spill rivers
     of coin across the floor (looks only); the counting tables tip; the golems climb out of the
     heaps (as built). **The giant scales start to tilt with his health**: a bar in the world,
     the pan on his side sinking as he loses.
   * **3 The Hoard**: **the gold rises**. A sheet of coins floods the floor ankle-deep (looks only,
     never slows: the flood plane shared with Leviathan's water), and the four corner heaps surge
     into dunes that set hard as gold (2 × 2 rock each, a gold glow ring 120 f ahead, bodies
     pushed off): new cover against his volley, new stone between him and his coins. The pillars
     gild over.
   * **Finale**: coins rain from the dark all the time (looks only); the braziers swing.

5. **Set pieces.**
   * **The Count** (the Assessor's counting-bell, his Muster instrument from the Vaults). At 50%
     Mammon rings the dead Assessor's bell, hung from the scales' foot: three tolls, and on each a
     ring of coins bursts round him, at 2.5, 5 and 7.5 tiles, 40 f apart. Every ring's band glows
     from the first toll; the gaps between them are safe. **Three blows on the bell silence it**:
     the count stops and the noise staggers him (an open 90 f), once a fight.
   * **Weighed and found wanting.** At the finale's threshold the scales slam down on the hero's
     side: the pan on his side crashes onto the north end (a quake under its shadow), pouring out
     coin. The finale begins.
   * **The treasurers' decree** (if it finished): the golems come at once (as built) and the dunes
     rise in phase 2.

6. **The finale: ALL THAT GLITTERS** (below 15%).
   * **Avalanche**: he rears back and throws both arms down; coins roll out from him as three
     rings bursting 24 f apart, with **one wedge of 60° left open** along a lane of bare marble.
     Read the wedge and walk into it. Recovery 60: the big punish.
   * Hoard Slam, Coin Rain, and the Grasp (still the one hold, its rules unchanged).
   * **His death.** He clutches at the coins as they turn to dust in his fingers: **"No, no, no,
     they're mine..."** He sinks to his knees into his hoard, and the hoard drains away through the
     floor's seams like sand in an hourglass, taking him with it. His crown rolls across the floor
     to the hero's boots and falls flat, as the coin did at the start. The scales swing level.
     Every coin left in the hall turns to dull grey lead: it was never worth anything.
   * **The seal-stone**: a golden citrine cut like a coin, which rises out of the last heap.

7. **His theme.** Bastion Lux, "Enter" (B14). The entrance on the track's opening; phase 2 the
   drum's heartbeat; phase 3 the full drum and +3% varispeed; the finale on its brightest bar; the
   death: a coin's ring, then silence.

8. **References.**
   * Spirited Away, No-Face: he swells as he devours, and shrinks as it comes out of him. Borrow
     the swell and the shrink as the read of his health.
   * Cuphead, King Dice: a money boss as a show. Borrow the Count's rings as a pattern to learn.
   * Shadow of the Colossus: no bar on the screen, the beast itself tells you. Borrow the scales as
     a bar in the world, so the eye stays on the arena.

9. **Fairness and numbers.**
   * Expert 2.5-3 min (100-133 s today), adept 4-7 (203 s met first, **464 s met seventh: health
     trimmed 15%**, measured at the seventh court).
   * Phases: 3 and the finale (15%).
   * Stays: every move, the grasp, the gold hide, the golems.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| The Count (at 50%) | 40 | 1 ×3 | 44 | 16 | three rings of coins at 2.5 / 5 / 7.5, 0.9 wide, 40 f apart, all glowing from the first toll |
| Avalanche (finale) | 52 | 1 ×3 | 60 | 18 | three rings 24 f apart, one 60° wedge open; cooldown 300 |

10. **Cost: M.** The coin flood (the shared flood plane), the dunes (tile edits), the swell and
    shrink (scale), the scales tilting (a rigged prop driven by his health), the bell. His court
    is already at the art bar: no art rework.

11. **The tone.** Greedy and a little ridiculous; his end is losing his gold, not his life in
    pain.

---

## 6. Beelzebub, Gluttony: the Banquet of Flies

1. **The fantasy.** A bloated lord of flies at a feast that never ends: stop him eating, and watch
   the banquet collapse into his gut.

2. **The entrance** (9 s).
   * A fly lands on the hero's shoulder (close up), and buzzes off.
   * The camera follows it up the long table, past the mouldy platters, to the throne of plates:
     Beelzebub, his back turned, eating noisily.
   * He turns: the proboscis, the great red eyes. A second-long shot **through his compound
     eyes**: the hero seen in a hundred little hexagons.
   * **"Ah. Dessert."**
   * Name card: **BEELZEBUB · THE LORD OF FLIES · PRINCE OF GLUTTONY**.

3. **Scale.** **He grows with every meal**: each Gorge +10% (up to 1.3× from meals), and a choke
   shrinks him back a step. Phase 3, THE GLUTTON, starts at **1.3×**, the finale at **1.5×**
   (his belly a dome). Belly Roll's radius grows with him. Camera: the tall framing.

4. **The arena by phase.**
   * **1 The Lord of Flies**: as built.
   * **2 The Swarm**: the flies thicken into a dark ceiling (the haze and the swarm vortices up);
     the linen tears; the platters he eats leave holes in the table.
   * **3 The Glutton**: **the banquet collapses**. The table cracks in the middle of each arm
     (marked two seconds ahead), and its pieces tip into heaps of rubble (rock). Then the hall
     turns into the gut, the Rotting Feast's lie ("rooms of one palace" that are "stages of a
     single gut"): the walls pulse slowly, the flags shine with bile. Looks only.
   * **Finale**: the **silver dome** of the Last Course comes down in the middle of the hall.

5. **Set pieces.**
   * **The Last Course** (the realm's climax). At 15% the Steward's silver dome drops into the
     middle of the hall (a quake under its shadow, marked 60 f) and Beelzebub lumbers to it to eat
     the last course: a Gorge of **120 f startup** that would heal him 15%. **Struck while he eats,
     he chokes on it**: a 200 f stagger, his bloat open, and he never eats again. If he eats it,
     he heals and swells, once. The Gorge rules as built, made into a finale.
   * **The dinner gong** (the Steward's Muster instrument). At phase 2 the gong by the throne
     sounds and the fly swarms come (as built). **Three blows on the gong** and no swarm comes
     back; left whole, two more swarms come in phase 3.
   * **The feast laid** (the Steward's decree, if it finished): six platters (as built).

6. **The finale: THE LAST COURSE** (below 15%).
   * The Last Course (above), then Belly Roll twice running (rock stops him), Bile Spew, Belly Slam.
   * **His death.** He chokes, staggers, and deflates, wheezing, smaller and smaller, until he is
     **a single fat fly**, which bumbles off into the dark. The flies stream out through the high
     windows after him. The mould fades from the food: the feast is plain and fresh again, bread
     and apples, as food should be.
   * **The seal-stone**: an orange amber with a tiny fly in it, under the silver dome when it lifts.

7. **His theme.** Celestia Ardent 3 (80 bpm, the heaviest bass; B14). Phase 2 the drum's
   heartbeat; phase 3 full; at the choke **the music stops dead** for a beat (comic timing), then
   comes back on its climax bar.

8. **References.**
   * Spirited Away, No-Face at the bathhouse feast: he devours, swells, spews and shrinks. Borrow
     the arc of the whole fight.
   * Dark Souls, the Gaping Dragon: the maw and the belly. Borrow the gluttony's menace (without the
     gore).
   * Cuphead, Baroness Von Bon Bon: a food boss with playful menace. Borrow the tone.

9. **Fairness and numbers.**
   * Expert 2.5-3 min (90-118 s today), adept 4-6.5 (220 s met second, **380 s met sixth: health
     trimmed 10%**).
   * Phases: 3 and the finale (15%).
   * Stays: every move, Gorge and the choke, Scatter, the swarms.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| The Last Course (at 15%) | 120 | 1 | 44 | heals 15% | a spell at the dome; struck in it, a choke of 200 f |
| The dome's fall | 60 (its shadow) | 6 | — | 20 | a court hazard: quake 2.0 round the dome's spot |

10. **Cost: M.** The table's collapse (dressing to rubble rock), growth by meals (scale), the dome
    (a Gorge variant), the gut's pulse and the swarm ceiling (atmosphere), the compound-eye shot
    (a post effect, optional).

11. **The tone.** Disgusting in a way children laugh at, never in a way that sickens. He ends as a
    fly.

---

## 7. Asmodeus, Lust: the Rose Bower

1. **The fantasy.** A smiling, three-headed bridegroom in a moonlit rose garden: light Raphael's
   smoke to strip his charm, as the thorns close in and the garden burns to ash.

2. **The entrance** (11 s).
   * The garden at night; a music box plays; petals fall. At the bridal arch, nobody.
   * The hero walks in. In rose letters: **"TURN AROUND."**
   * The camera swings behind the hero: Asmodeus is standing right there, bowing low, smiling (the
     man's head). Then the bull's head turns to look. Then the ram's.
   * **"Seven came before you. None of them stayed for the dance."**
   * Name card: **ASMODEUS · THE SUITOR · PRINCE OF LUST**. The censers glint ("STRIKE A CENSER",
     as built).

3. **Scale.**
   * Phase 2, THE THREE HEADS: the bull's and ram's heads swell (1.15× overall).
   * Phase 3, THE BRIDEGROOM OF ASHES: **1.4×**, his silks burning to embers, a crown of black
     thorns.
   * The giant: **the Black Thorn** (the realm's landmark) grows over the garden's north wall as a
     vast dead tree of thorns, its branches reaching over the hedges as they close in (background,
     never struck).
   * Camera: the tall framing. (A dragon mount, from the Goetia, is left for later: L on its own.)

4. **The arena by phase.**
   * **1 The Suitor**: the bower in bloom, as built.
   * **2 The Three Heads**: thorns sprout through the hedges.
   * **3 The Bridegroom of Ashes**: **the thorns close in**. The hedge walls advance in two steps,
     the open garden shrinking from 27 tiles across to 22, then 18 (marked two seconds ahead, the
     hedge foot glowing; bodies pushed). The four censers stay inside it. The roses wither and
     burn to ash; the fountain dries.
   * **Finale**: the bridal arch burns; ash falls like snow.

5. **Set pieces.**
   * **Raphael's smoke** (Tobit). When **all four censers have been lit at least once**, their smoke
     gathers above the garden into a shining shape of wings (Raphael, the archangel who drove him
     off) and drives Asmodeus to his knees: **a 6 s open stagger**, once a fight. The tool the
     court gives the hero, given its climax. Toby's name is Tobias's (STORY.md): Hızır's line after
     this fight already tells it.
   * **The wedding procession** (the Bridesmaid's Watch). At phase 2 the elf-maids come in a
     procession through the bridal arch, not out of the hedges: a staged entrance for the adds.
     If the Bridesmaid's decree finished (the vows spoken), the censers light once each (as built).
   * **"Turn around."** (The realm's lie, kept.) In phase 3, now and then, a rose-coloured whisper
     and a footstep behind the hero: "TURN AROUND." It is a lie with a tell (rose, not amber, no
     bark); the real blow is the one telegraphed in front. Hızır: "Eyes on him." It never turns
     the camera or the controls.

6. **The finale: BOUND** (below 15%; in Tobit Raphael binds him).
   * **Thorn Garden**: thorns burst over the shrunken garden in two alternating sets of tiles like a
     chequerboard, 30 f apart. Stand on the other colour.
   * **Three Kisses**: three slow hearts in a fan (one volley, one landing); each smites as the
     kiss does. Leap them or cut them from the air.
   * Thorn Ring, Bull's Charge twice, Lance Thrust.
   * **His death.** Raphael's smoke rises from all four censers at once. His smile fades; the three
     heads speak together: **"...alone again."** He crumbles into rose petals and ash, blown away on
     a wind toward the south (Tobit: he fled into Egypt). The thorns wither; the hedges bloom with
     white roses.
   * **The seal-stone**: a rose-pink stone, which lies in the dry fountain's bowl as it fills with
     clear water again.

7. **His theme.** Boss (shared with the Cyclops until B56 finds his own). The entrance on a synth
   music box playing the track's motif; phase 2 the drum's heartbeat; phase 3 full; the finale on
   its climax bar.

8. **References.**
   * Cuphead, Cagney Carnation: a flower boss whose vines take the arena in the last phase. Borrow
     the closing garden.
   * Dark Souls III, Yhorm and the Storm Ruler: the arena gives you the tool to beat him. Borrow
     the censers' payoff.
   * Elden Ring, Malenia: the bloom and the finale with one signature move to learn. Borrow the
     transformation into ash and the Thorn Garden as his signature.
   * Zelda, the Gleeok (Tears of the Kingdom): three heads, each with its own attack. Borrow a
     look per head (the bull charges, the ram butts, the man kisses).

9. **Fairness and numbers.**
   * Expert 2.5-3 min (90-135 s today), adept 4-7 (144 s met first, 353 s met seventh: **health
     trimmed 15%**, the closing garden makes him harder; B33 notes him twice as long as the others
     at high tiers).
   * Phases: 3 and the finale (15%). The garden never smaller than 18 tiles across.
   * Stays: every move, the censers, Allure, smitten, the elf-maids.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Thorn Garden (finale) | 44 | 1 ×2 | 46 | 18 | traps on alternating tiles, two sets 30 f apart, fuse 50; cooldown 330 |
| Three Kisses (finale) | 40 | 1 | 40 | 8 ×3 | a volley of three hearts, ±0.35 rad, each smites 90 f; leapable, cut from the air |

10. **Cost: M-L.** The closing hedges (tile edits in two steps, phase-variant hedge dressing),
    the Black Thorn (one background sculpt), Raphael's wings of smoke (an emissive particle shape),
    the ash palette (an atmosphere blend).

11. **The tone.** Charming and wrong; the suitor who will not take no. Nothing romantic is shown;
    his menace is his smile.

---

## 8. Krampus: the bottom of the world

1. **The fantasy.** The monster from the very first scene, at the bottom of the world: bells in the
   dark, a friend in a cage, and a fight that ends in white light.

2. **The entrance** (15 s the first time, 3 s after).
   * The platform settles at the bottom. Silence. Snow falls in the shaft's light. The cage hangs
     in the north, Toby in it, lit.
   * Bells in the dark, far off, coming closer, **circling the hero** (sound placed round him).
     The chains sway.
   * Red eyes open in the dark above the north wall. Krampus drops out of the dark onto the snow
     before the cage: snow bursts, the camera shakes, every chain rings.
   * He unfolds from his hunch to full height, tongue lolling, and drags his birch through the snow.
   * Toby: **"I knew you'd come. I knew it!"** (as built). The hero's boy answers (as built).
   * Krampus, with his slow laugh: **"Three little warriors. My basket has room for four."**
   * Name card: **KRAMPUS · THE LAST FIGHT**.

3. **Scale.**
   * Already 3.25 tiles.
   * Phase 2: **his shadow**, thrown by the braziers, stands huge on the hollow's walls (a
     flattened copy of his rig drawn on the wall: cheap, and it moves with his tells).
   * Phase 3, KRAMPUSNACHT: he rises out of his hunch to **1.3×** (4.2 m); his horns scrape the
     icicles.
   * The finale: no more growth. **The light grows instead.**
   * Camera: the tall framing.

4. **The arena by phase.**
   * **1 The Bells**: as built.
   * **2 The Chains**: great chains crash down out of the dark and anchor in the floor (four iron
     anchors, rock: new stone). **The ice columns crack** when he crashes into them, and the
     third crash breaks one into rubble (the same tech as Amon's pillars).
   * **3 Krampusnacht**: a blizzard whirls (atmosphere only: every telegraph is drawn bright,
     above the haze); the braziers tip and spill their coals (lingering patches, never slowing);
     icicles fall (traps with shadows, fuse 50).
   * **Finale, THE LAST BELL**: the dark above cracks, and **a thin white light falls from far
     above** onto the cage: the first touch of Scene II's light. It widens as his health falls,
     until the whole hollow is lit and the snow turns to slow, glittering motes.

5. **Set pieces.**
   * **Toby unties the birch** (the friend helps; Toby is never in danger). On Krampus's second cage
     taunt, while he grips the bars with his birch hand, Toby pulls the red ribbon off the birch
     bundle (it is bound in red ribbon, FINALE.md). The switches fall apart. Krampus staggers in
     surprise (an open 120 f) and **loses the Birch Lash** for the rest of the fight (a slower Claw
     Swipe takes its place). Banner: "TOBY UNTIES THE BIRCH!"
   * **The seven seals** (B44). At the finale's threshold the seven seal-stones the hero carries
     rise round him in a ring, each in its prince's colour, and shine. Their light breaks the dark
     above (the falling light of the finale) and **burns away Krampus's pelt**: in the finale his
     hide is gone, and every blow lands whole. The seven fights of the run pay off in the last one.
   * **The Perchten** come out of the clefts, as built.

6. **The finale: THE LAST BELL** (below 12%).
   * His hide gone, he fights desperately: Hoof Charge twice running, Sack Swing, Chain Lash.
   * **Bell Storm**: he whirls his great chain and its bell over his head (its shadow circling the
     floor), then brings it down: a ring that shakes from 2.6 to 5.0 tiles, and a second from 5.0
     to 7.5 24 f later. **The safe place is close to him**, the opposite of every quake before.
   * **The bells count down**: the seven bells on his chain fall silent one by one as his health
     runs from 12% to nothing (his built death, spread across the finale).
   * **The last blow**: in slow motion, the other two warriors' souls stand with the hero and strike
     with him, in their soul colours (the ghost rigs of Scene I, looks only), and a white flash.
   * **His death** (as built): he sinks to his knees in the snow, the last bell falls silent, the
     cage's lock breaks, the boy runs to the hero. The light from above widens into Scene II's.

7. **His theme.** The Last One (140 bpm; shared with the Pit, B56 wants his own). The entrance: no
   music, only bells. The name card lands the track. Phase 2 the drum's heartbeat; phase 3 full,
   +3% varispeed; the finale: the track fades under **the first chords of the hymn** (`audio/hymn.rs`),
   the light's music overtaking his; the death: silence and single bells, then Scene II's hymn.

8. **References.**
   * Shadow of the Colossus: the slow, sad death and the light. Borrow the light that takes over.
   * Zelda (Ocarina, Twilight Princess): Zelda holds Ganon or fires the light arrows in the last
     fight. Borrow the friend's help (Toby's ribbon).
   * Dark Souls III, the Soul of Cinder: the last boss carries the whole journey. Borrow the seven
     seal-stones' light.
   * Hades, the last fight with family at stake: a readable duel with the heart in it. Borrow the
     stakes in the staging.
   * Hollow Knight, the Radiance: the light from above. Borrow its arrival.

9. **Fairness and numbers.**
   * Expert 3.5-4 min (2.5-2.9 in the run today), adept 7-9 (7-8 today; the lair's try is 10).
   * Phases: 3 and the finale. Thresholds: 70% / 40% / 12%. The finale's hide is gone, which pays
     for its spectacle.
   * Stays: every move, the cage taunt, the daze on stone, the Perchten, his built death.
   * New:

| Move | Startup | Active | Recovery | Damage | Shape |
|---|---|---|---|---|---|
| Bell Storm (finale) | 50 | 6 ×2 | 56 | 28 | a ring shaking from 2.6 to 5.0 tiles, then from 5.0 to 7.5 24 f later; safe within 2.6 |
| Claw Swipe (after the ribbon) | 36 | 8 | 46 | 24 | cone 1.3 (+1.0), ±70° (the Birch Lash's place, slower) |
| Falling icicles (phase 3) | 50 (shadow) | 1 | — | 16 | a court hazard: three traps 0.8 wide round the hero, fuse 50 |

10. **Cost: L** (with B12's art pass of the Pit and the lair). The entrance with sound placed round
    the hero, Toby's ribbon (a Toby animation and the birch's dressing), the seal-stones' ring
    (B44's models), the shadow on the walls, icicles, breakable ice (shared with Amon), the
    three-souls blow (the story's ghost rigs exist: `emit_rig_ghost`).

11. **The tone.** The scariest fight, and the warmest ending. Krampus menaces, laughs, rattles the
    cage; he never hurts the child, and nothing in the fight can (as built).

---

## 9. The shared toolkit

Built once, in the template (§10), then used by every court. Each piece names where it plugs in.

### 9.1 The entrance (a boss-entrance cinematic system)

* **Shots** as keyframed camera tracks relative to the court's anchors (the gate, the throne, the
  prince), on the story camera that exists (`story_cam`: focus, lens roll, the cinema depth of
  field; `WorldView::cinematic`).
* **Beats** for the prince: poses and clips at times, a path to walk, a scale.
* **The name card**: a HUD piece, NAME · TITLE · PRINCE OF SIN, in the sin's colour (Cinzel), 2.5 s,
  in the manner of Hades' and Zelda's boss cards.
* **The first line** through the speech panel in his own voice (B10's synth).
* **Skipping**: the first time it plays through; after that any key skips it, and a save flag per
  court (`entrances_seen`) remembers. New Game+ plays a 3 s short form (the
  card and the line).
* **The sim waits**: the fight's clock and the threat manager start when it ends. Bots and the
  harness skip it (`--court` starts at the fight; no number moves).
* After a fall the prince is back before his throne as built (HIS WOUNDS REMAIN), with no entrance.
* Captures `entrance:S:T`; a LEVELS map stop per court is not needed (the courts are on it).

### 9.2 The court's transformation (the stage framework)

* **Stages**: a court lists a stage per phase and one for the finale. A stage is:
  * **tile edits**: floor → rock (a shard, a dune, a hedge, a fallen clock), floor → lingering
    hazard (lava, coals), floor → gone (Lucifer's outer ring: a wall), and back (Coil Crush's
    temporary rock);
  * **dressing groups** built at load as ranges of static cubes and toggled with a transition
    (shatter, fall, rise, sink, burn, wither), the debris from W4's kinetic debris (WARBAND.md
    §4.4);
  * **the light**: an atmosphere and haze preset blended in over 1-2 s (the court's
    `court_atmosphere` per stage), stage lights through the clustered lights;
  * **a surface plane**: one shared flood plane at a height with a material (water, coins, fog,
    ash), rising or draining.
* **Fairness rules** (tests):
  * every tile edit is marked 120 f ahead, glowing;
  * anything on a closing tile is pushed to the nearest open tile (`Rider::Shove`'s rules), never
    trapped;
  * an open, connected floor of at least 18 tiles across always remains, and every open tile is
    reachable (`pathing`);
  * hazards never slow; their stings are the floor, not landings (as Amon's lava).
* **The hush**: the transition plays in the phase change's breather (the roar, the banner, the
  green cross, as built), with the threat manager granting nothing for its length (2-3 s).
* The flow field rebuilt after each edit. The probes are left as they were at load (most changes
  are dressing); a court whose light changes a lot re-probes in the hush if cheap (RP18).

### 9.3 Scale and the camera

* **His look per phase**: `scale`, `radius`, `arm` in his phase table, eased over a second. The rig's
  root matrix carries the scale (as the boys' ghosts are drawn larger than life).
* **Sculpts scale to 1.5× at most** (voxels of 3.75 cm read well at fight distance; B35 warns of
  blocky giants scaled further). A larger form is a **background figure**: its own sculpt, built
  when its court loads (not at startup), never colliding, fading for the camera like trees
  (Lucifer's reflection, Leviathan's body, the Black Thorn, Krampus's shadow).
* **His sim radius and arm grow with him**, so his shapes and their telegraphs grow too; the frame
  data does not change.
* **The tall framing**: within 5 tiles of a prince taller than 3.2 m the chase camera's distance
  grows (3.4 → 4.6 at most), its pivot rises (1.7 → 2.2) and its look lifts toward his chest; the
  field of view stays (the tells keep their size). The top view climbs by 1.2 × the height he adds.
* `--camera-check court<N>` with each phase's scale and each stage's dressing (B25's harness).

### 9.4 Set pieces (triggers and beats)

* `SetPiece { when, beats }`. **When**: a phase begins, his health below a share, seconds in a
  phase, the Nth cast of a move, an instrument broken, a decree done (B41).
* **Beats**: hush the threat manager for N frames; a banner or a line (the lie in the prince's
  colour, Hızır in amber); a court hazard (landings planned through `general_hazard_blow`); a stage
  edit; slow motion (the juice's dilation); a camera nudge, only inside a hush.
* **Instruments in the court**: an object struck three times to break it, by the realms'
  instrument rules (`logic/war/general.rs`), with its own reward.

### 9.5 Phase music

* A cue sheet per court on the B14 track: the entrance (low-passed, or silent and diegetic), phase
  1 (full), phase 2 (+ the war drum's heartbeat, then its peak), phase 3 (+ the full drum, +3%
  varispeed), the finale (seek to the track's climax bar), the death (cut to silence in 0.3 s, a
  stinger, the seal-stone's chime).
* New in the mixer: a one-pole low-pass and a gain envelope on the music bus, a seek to a bar (the
  beat clock knows each track's tempo and downbeat), varispeed (a resample rate within ±5%), synth
  stingers per sin (an interval and bells).

### 9.6 The fall and the seal-stone

* The last blow: the trophy freeze and slow motion (as built), an orbit shot (2-6 s, skippable),
  his death beat (dressing and his rig), **the court healed** (the lie undone: the dawn window,
  the water still, the embers cold, the clocks running, the gold turned to lead, the feast fresh,
  the white roses), then **B44's seal-stone** rises, hovers in his colour, and the pickup's
  close-up (1.5 s). The gate opens.
* The seal-stones (B44 builds the drop, the hub and the save; this adds the moment):

| Prince | Seal-stone | Colour (`Sin::color`) | Where it appears |
|---|---|---|---|
| Lucifer | a star-cut amethyst | violet 9a5aff | where his crown of light went out |
| Leviathan | an emerald like a serpent's eye | green 3ad06a | surfaces from the still water |
| Amon | a ruby that glows like a cold coal | red ff3a2a | on the anvil, in the blade's place |
| Belphegor | a moonstone | slate blue 7a9ac8 | falls with the hourglass's last grain |
| Mammon | a citrine cut like a coin | gold f2c53d | rises from the last heap |
| Beelzebub | an amber with a tiny fly in it | orange ff8a2a | under the lifted silver dome |
| Asmodeus | a rose-cut pink stone | rose ff4a9a | in the fountain's bowl as it refills |

### 9.7 The finale phase

* `FINALE_AT` per prince (0.12-0.15), a fourth table of moves (`prince::table`), its banner without
  a green cross, a 1.5 s hush and its set piece. The bar's notch at it.

### 9.8 The bots

* Each court's new reads go into the bots as its built answers did (`Bot::lucifer_dodges`,
  `amon_court`, `lust_court`...): step into the gap or the wedge, break the instrument when nothing
  lands within 45 f, keep off a closing tile. The masher learns nothing new.

---

## 10. The build: ranking and order

### 10.1 Epic gain per cost

| Rank | Who | Gain (1-5) | Cost | Why |
|---|---|---|---|---|
| 1 | **Lucifer** | 5 | M | The shatter and the window giant are the most striking change for the work: dressing, debris and one emissive figure. |
| 2 | **Belphegor** | 4 | S-M | His sleep, his clocks and his chair are built; the standing giant, the toppling clocks and the century are blends and edits. |
| 3 | **Mammon** | 4 | M | Court already at the art bar; the flood plane, the dunes and the swell; the scales as a bar in the world. |
| 4 | **Amon** | 5 | M-L | The breaking floor and falling pillars are the most "physical" spectacle; shares breakable cover with B57. |
| 5 | **Krampus** | 5 | L | The last fight: the most gain, at the most cost; best built with B12. |
| 6 | **Beelzebub** | 4 | M | The collapse and the Last Course reuse his Gorge; the fly ending is a gift. |
| 7 | **Asmodeus** | 4 | M-L | The closing garden and Raphael's smoke; the arena shrink needs care with the censers. |
| 8 | **Leviathan** | 5 | L | The body round the walls is the biggest new piece of tech. |

### 10.2 Build order

1. **Lucifer, the template** (toolkit §9 built inside his court). He uses every piece (entrance,
   stages, scale, a background giant, set pieces, the finale, the music cue sheet, the fall), ranks
   first, and his court is next in B11's art order: **build B43 Lucifer and B11 Pride in one job**,
   so the hall is lit and dressed once for its stages.
2. **Mammon**: art already done; the flood plane for Leviathan later; met first in the vaults-first
   order.
3. **Amon**: with B11 Wrath (next in B11's order) and after B57 (breakable cover).
4. **Belphegor**: with B11 Sloth.
5. **Beelzebub**: with B11 Gluttony.
6. **Asmodeus**: with B11 Lust.
7. **Leviathan**: with B11 Envy; last of the princes for his background body.
8. **Krampus**: with B12 (the Pit and the lair to the bar).

After the template, two builders can work at once (one court each), within the machine's limits.

### 10.3 The estimate

| Piece | Days (agent) |
|---|---|
| The toolkit (§9.1-9.8), inside the template | 8-10 |
| Lucifer (on top of the toolkit; B11 Pride's art separate) | 3 |
| Mammon | 2-3 |
| Amon | 3-4 |
| Belphegor | 2 |
| Beelzebub | 3 |
| Asmodeus | 3-4 |
| Leviathan | 5 |
| Krampus | 5 |
| Balance per court (the harness, `--court-at` at the adept's kit, the bots) | 0.5-1 each, 4-8 |
| **Total** | **about 38-47 agent-days** |

The template takes about two weeks; with two builders after it, the other seven take about three
more: **about five weeks in all**, art passes (B11, B12) not included.

---

## 11. Decisions for the user

1. **The generals' part in the courts** (with B41, all three are always slain): the adds always come;
   each general's instrument returns in the court as a breakable set piece; a finished decree adds
   one twist. Or keep today's terms (no adds at all once B41 lands).
2. **The finale** carved from the bottom of the bar (12-15%), with health trims where the adept is
   near the eight-minute try. Or add length instead.
3. **The entrances**: played through the first time, skippable at once after (the brief's rule). Or
   skippable after 3 s even the first time.
4. **The template**: Lucifer with B11 Pride in one job. Or Mammon first (no art rework, less
   spectacle).
5. **Asmodeus's dragon** (the Goetia's mount) left out for now (L on its own).
6. **Krampus's finale**: Toby helps (the ribbon) and the seven seal-stones burn his pelt away.
