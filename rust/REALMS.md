# The six realms: level concepts

These are concepts for the six realms still to build, phases 4 to 9 of `ROADMAP.md`. Greed's realm, the Vaults of Mammon, is already built. Each realm is designed around one spatial technique that lies to the player, and each closes on its prince's lie, as Greed's closes on Mammon's toll. All numbers are starting values for tuning. The natives come from the 33-foe bestiary (COMBAT.md §10–11) and are tagged by stage. (Superseded 2026-10-03: the realms are built, and their packs now hold every foe met so far plus the stage the realm brings in, by how many realms have been taken; see `logic::realms::Dealer`. The native lists below are the original themed picks.) Status: proposals (2026-10-03), not yet confirmed.

| Sin | Realm · Prince | Technique | The prince's lie | When it springs |
|---|---|---|---|---|
| Pride | The Hall of Mirrors · Lucifer | Asymmetric Portal & Warp Seams | *"Take your throne."* | Your reflection steps out with your own kit, swollen by your pride |
| Envy | The Serpent Grotto · Leviathan | Dynamic Inter-Surface Parallax | *"All you want lies below."* | The temple bell drops over you, and the serpent coils it red-hot |
| Wrath | The Burning Field · Amon | Dynamic Inter-Surface Parallax | *"Sound the horn and the battle ends."* | The field breaks into drifting slabs, and every corpse rises |
| Sloth | The Grey Mire · Belphegor | Focal Landmark Anchors | *"Lie down. The door opens when you wake."* | The hourglass turns, and sleep resets the realm a century older |
| Gluttony | The Rotting Feast · Beelzebub | Architectural Transition Seams | *"Finish your plate, and you may leave."* | The stomach convulses, and only a lean hero fits the way out |
| Lust | The Garden of Thorns · Asmodeus | Focal Landmark Anchors | *"Turn around."* | The Bride walks up behind you; one look makes you her eighth groom |

---

## 1 · PRIDE: The Hall of Mirrors
*Lucifer · violet*

**Biome & Architecture**
- **The Spire of the Morning Star:** seven octagonal galleries stacked round a hollow light-well, climbing toward a white sun.
- **Materials:**
  - black-violet obsidian floors polished to mirrors, so the room hangs upside down beneath you;
  - white marble, gold leaf, peacock-eye stained glass, chandeliers of mirror shards;
  - statues of *you* in triumph lining every gallery, grander on each floor.
- **Light:** a single source, the Morning Star at the top of the well.
  - Mirrors relay it as hard white-violet beams, and the beams burn.
  - Shadows are razor sharp. The only soft glow falls on your statues.
- **Geometry:** each gallery is a ring about 24 tiles across round the open well; falling in drops you a gallery. There are no stairs: galleries connect only through full-height mirror-doors, three per gallery.

**Folklore Integration**
- **Lucifer, "son of the morning"** (Isaiah 14): *"I will ascend… I will exalt my throne"*, and then *"brought down… to the sides of the pit."* The level is the climb and the fall.
- **The devil's mirror** (Andersen, *The Snow Queen*): it was built to shrink the good and swell the bad, carried up to mock heaven, and shattered.
  - The Hall is made of its shards.
  - A beam hit lodges a **Splinter**: for 6 s, true and vain mirrors swap faces.
- **Tengu:** monks too proud for paradise are reborn as tengu, and *tengu ni naru* ("to become a tengu") still means to grow conceited. They hold the galleries and press harder the more Glory you carry.
- **Cin:** the Anatolian warning against mirrors after dark. A cin that vanishes steps into one mirror-door and out of another.
- **Natives:** tengu (stage 3), cin (5), frost giant (7: a heart frozen by a shard of the mirror).
- **New foe:** the Peerless.

**The Spatial Illusion: Asymmetric Portal & Warp Seams**
- **Seamless warps:** mirror-doors move you with no fade. The floor pattern and the light continue unbroken, so the jump never registers.
- **Vain mirrors flatter you.** Your reflection is crowned and unwounded, its health bar full, your statue gilded behind it.
  - They seem to lead *up* into grander galleries.
  - In fact they loop you one or two galleries *down*, redressed in brighter gold and louder applause.
- **True mirrors** show you as you are: your cracks, your real health, your empty belt slots. Only true mirrors climb.
- **Asymmetry:** no door works both ways. The back face of every door is another vain mirror, so retreating loops you too.
- **The count lies:** looking up the light-well shows how many galleries remain.
  - Each vain loop adds one: the spire grows as you're flattered.
  - The Morning Star blooms larger, as if you were closing in.
- **Tell:** true mirrors carry hairline cracks, and their health readout drops with yours. The more you're hurt, the easier the truth is to read.

**The Journey & Challenge**
- **The Trap: Glory**
  - **Loop:** clear a gallery, read its three mirrors, step through one, repeat. A vain pick costs a loop: one or two galleries replayed and repopulated.
  - **Lever:** clearing a gallery without being hit adds a Glory stack: +8% damage, a brighter aura, and a new statue of you rises. A hit or a potion humbles you, and all stacks are lost.
  - **Manipulation:** Glory is light, and it blinds you.
    - At 3 stacks or more, the beams track you and tengu earn extra attack tokens.
    - At 5, the vain mirrors fake the cracks. At full health the true mirror's readout matches theirs, so a flawless hero can't see the truth.
    - Your statues stand in front of the true mirrors. Smashing your own likeness (3 blows) clears the view and costs a stack.
- **The Climax: The Peerless**
  - **The lie:** the top gallery is a mirror floor 20 tiles across, round a dais and one vast mirror. The Morning Star speaks: *"You stand above them all. Take your throne."*
  - **It springs:** step onto the dais, and your reflection rises out of the floor: **the Peerless**.
    - It carries your blade, your shield, your tier and a copy of your belt.
    - It gains 10% damage and health for each Glory stack you brought. The pride you hoarded is its strength.
  - **Its rule:** it never acts on its own. It repeats every move you make **24 frames (0.4 s) later**, mirrored across the throne.
  - **Punishment:** each hit it lands drops the floor's outer ring into the well. With three rings gone, you fall to gallery 1 and climb again.
  - **Survive it:**
    - **Bait the echo.** Throw a single L1 and raise your guard as it ends. The copy lands inside your parry window, which Deflects it for 60 f, so punish with a full chain.
    - **Strike only while it reels.** A Deflected echo drops the moves it missed, so your punish isn't copied back. A chain thrown at a standing echo is copied, straight into your own recovery.
    - **Never leap toward it.**
    - **Break the symmetry.** Potions and blessings aren't copied, because grace can't be mirrored. Drink, and it stands posing, open, for 40 f.
    - **Shrink it first.** Humble yourself before the dais (take a hit, or drink) to face a smaller echo.
  - **The way out is down:** the Peerless shatters, the floor gives way, and you fall the full length of the spire. Lucifer's court was at its foot all along. *How art thou fallen.*

---

## 2 · ENVY: The Serpent Grotto
*Leviathan · green*

**Biome & Architecture**
- **A flooded sea-cave cathedral:** six caverns ring the Bell Cavern, where a drowned temple's bronze bell hangs over a black pool.
- **Two surfaces:**
  - **The Waterline,** where you start: wading shallows, bridges made from the ribs of dead sea-serpents, floating bone rafts.
  - **The Drowned Floor:** a sunken twin of the grotto, two storeys down under clear green water. It is always visible, and you can enter it.
- **Materials:** wet black basalt, bleached serpent bone, verdigris bronze, nacre, jade-glass water, glowing green algae.
- **Light:** it comes from below, and the lit place is always the other one.
  - The Drowned Floor glows a warm gold-green, with lanterns and treasure.
  - The Waterline is cold and dim, and caustics crawl the walls.
- **Geometry:** each layer is whole only where the other is broken. A missing rib-bridge above means an intact path below, and the reverse. The real route zigzags between them.

**Folklore Integration**
- **Leviathan**, *"that crooked serpent… the dragon that is in the sea"* (Isaiah 27), is Binsfeld's prince of envy. His coils thread both layers: a moving hump above, a moving wall below.
- **Nazar, the evil eye:** in Anatolia envy strikes through the gaze, and the blue *nazar boncuğu* takes the blow and cracks.
  - A great slit-pupilled **Eye**, the one on the door, opens in the water and sweeps the caverns.
  - Nazar beads ward it.
- **Kiyohime of Dōjō-ji:** jealousy turned her into a serpent, which coiled round the temple bell and burned the man hiding inside. That bell is the climax.
- **The dragon on the spring** (yedi başlı ejder) sits on the town's water and won't share it.
- **Natives:**
  - cobra (stage 3);
  - kappa (4): it drags swimmers under, so it can pull you down a layer;
  - yedi başlı ejder (6);
  - Níðhöggr (7), "the malice-striker", which burrows *between* the layers.

**The Spatial Illusion: Dynamic Inter-Surface Parallax**
- **Depth by motion:** as the camera tracks you, the Drowned Floor slides more slowly than the Waterline. Stand still and they collapse into one picture; move and they separate.
- **Refraction lie:** as in real water, everything below looks shallower and shifted toward the camera, by about a quarter of its depth. A dive lands about 1.5 tiles beyond where the target appears, so players learn to aim past it.
- **The greener side:** each layer draws the other with phantom treasure.
  - From above, the Drowned Floor shows chests, open paths and the exit's glow.
  - From below, the rafts you left look heaped with gold.
- **One-way seams:** deep pools only go down, and geyser vents only go up (a 4 s ride, exposed). Every crossing is a commitment.
- **Tell:** a phantom moves at the wrong depth. It slides at the surface's rate instead of the floor's, and it casts no caustic shadow. Move first, then believe.

**The Journey & Challenge**
- **The Trap: Covet**
  - **Loop:** cross a cavern on one layer, see its better twin on the other, then choose whether to dive.
    - The true route needs about four crossings.
    - Each phantom you chase costs a dive, a geyser ride back and 10–15 s.
  - **The Eye:** every 20 s or so it opens and sweeps a green cone for 6 s.
    - If it sees you, you are **Doomed**: your next wound deals ×1.3.
    - Use a blessing, a potion or a rune while it watches, and the grotto takes it. The nearest native keeps it until you kill it: a cobra throws your Raphael beam, a kappa drinks your potion.
  - **The ward:** about one nazar bead per cavern, on either layer. A bead takes one gaze whole, then cracks audibly on your belt. Phantom beads glitter on the other layer to bait dives.
  - **Manipulation:** the other side always looks richer, and you bleed time and beads chasing the second picture.
- **The Climax: Kiyohime's Bell**
  - **The lie:** in the Bell Cavern, the court portal glows on the Drowned Floor directly beneath the bell. The water whispers: *"All you want lies below."*
  - **It springs:** dive for it and the bell drops, its rim sealing at the waterline with you inside a 7-tile disc of shallows under a bronze dome. The serpent coils round it, and the bronze begins to heat.
  - **Punishment:**
    - The outer tile scorches (5 health every 20 f), and the safe circle shrinks by a ring every 6 s.
    - Steam vents burst in rings.
    - The Eye watches through the bell's sound-hole, so any blessing you use inside is stolen.
    - When the disc is gone, the bell crushes you, and you surface at the mouth of the cavern.
  - **Survive it:**
    - **Find the head.** Where the serpent's head presses, the bronze glows white. Strike that patch with your L3; after three recoils it uncoils, and the bell lifts for **1 s**.
    - **Dive in that second.** A seam opens under the bell, straight down to the Drowned Floor, where the portal is real.
    - **Clear the steam.** Leap the steam rings: your airborne frames clear the bursts. Fight with the blade only.
    - **If you miss the window,** the bell drops again on a smaller disc.

---

## 3 · WRATH: The Burning Field
*Amon · red*

**Biome & Architecture**
- **An endless battlefield** broken into slabs of scorched earth 8–20 tiles across, adrift on a lava sea. Amon's banner-hill rises at the far end.
- **Shifting platforms:** slabs drift, grind and tilt in three height bands: the high ridge, the field and the low shelf. Each band drifts at its own speed, so gaps open and close like ice floes.
- **Materials:**
  - charred palisades, siege towers and war wagons;
  - thickets of broken spears and swords planted in the earth, impassable "fields of blades";
  - torn red banners, and obsidian glass where the ground melted;
  - bones and armour in blood-mud.
- **Light:**
  - Lava glows from below and throws shadows upward; ash fog erases distance.
  - Lightning strikes every 8 s or so. It is a white flash with hard shadows, the only moment the field reads true.
  - A ghost army wheels in the storm.

**Folklore Integration**
- **Amon** (*Ars Goetia*): a wolf with a serpent's tail vomiting flame, or a man with a raven's head, who "procures feuds".
  - His wolves and ravens hold the field.
  - **Michael's** fire, which turns foes on each other, is this realm's best blessing: let them feud.
- **The Einherjar of Valhalla** fight all day, rise at evening and fight again. Here, everything slain rises at the next lightning flash.
- **Berserkergang,** the wolf-coat frenzy of the *úlfheðnar*, becomes Fury.
- **Goryō:** the rage of the wronged dead, falling as lightning. They ride the storm.
- **Natives:**
  - raven (stage 1);
  - draugr and helmeted skeletons (1–2): the Einherjar;
  - loup-garou and dire wolf (2–3);
  - rasetsu (3), oni (4), goryō (6), Surtr (7).

**The Spatial Illusion: Dynamic Inter-Surface Parallax**
- **The isometric lie:** in the fixed 3/4 view, a slab one band lower and one tile nearer is drawn exactly where a level slab would be. Height and distance collapse onto the same pixels, and the ash fog strips the haze that would tell them apart.
- **Only motion tells:** as the camera follows you, each band slides across the screen at its own rate. Stand still and the field is a flat painting; move and the layers peel apart.
- **Drift:** a gap that reads as 1 tile can be 3 by the time you land, past your 2-tile leap.
- **Bait:** foes taunt from slabs that look adjacent but sit a band down. Charging them means leaping into the gap.
- **Tell:** lightning. For 0.3 s every slab edge is outlined in white and casts a true shadow. The patient cross on the flash.

**The Journey & Challenge**
- **The Trap: Fury**
  - **Loop:** cross slab to slab toward the hill while packs charge from all sides.
  - **Lever:** every hit you land adds a Fury stack (+6% damage and +4% speed, up to 10). Fury decays only after 2 s without attacking.
  - **Berserkergang:** at 10 stacks, for 6 s, you hard-lock onto the nearest foe and charge it, and your guard is gone. On this field the nearest foe is usually a band below. Wrath takes the controls.
  - **The ground keeps score:** every blow, yours or theirs, cracks the slab it lands on. At the third crack the slab splits, and the smaller piece sinks over 4 s.
  - **The dead return:** anything you kill rises at the next flash, unless its body goes into the lava. A foe whose poise breaks within 1 tile of an edge topples in for good.
  - **Manipulation:** the field invites violence and punishes it.
    - The invitation: taunts and rising buffs.
    - The punishment: cracks, the berserk lock, and an army that never ends.
    - The fastest way through is to stop fighting: parry, topple, move.
- **The Climax: The Horn of the Field**
  - **The lie:** at the foot of the hill a war horn hangs on a burnt gallows-tree. A raven-headed herald says: *"Sound the horn and the battle ends."*
  - **It springs:** the horn calls the whole field.
    - Every band lurches, and the lava rises and drowns the low shelf.
    - Flashes come every 4 s, and the dead rise on each.
    - The causeway-slab to the hill begins to sink: you have **60 s**.
  - **Punishment:** blows crack slabs twice as fast, so a slab you fight on sinks beneath you. Falling in the lava costs a third of your health and drops you back on the herald's slab.
  - **Survive it:**
    - **Cross by reading.** Cross five drifting slabs, reading them by motion and jumping on the flash.
    - **Guard instead of swinging.** Parry to Deflect, and finish attackers at slab edges with an L3, so they topple in instead of rising.
    - **Reach the Truce Stone.** It stands on the hill and is the real gate. It opens after **5 s in which you attack nothing** within 4 tiles. A white ring counts down, and any swing resets it.
    - **Hold the ring.** Against the charging dead, use guard, parry and leap alone. To win, you have to stop.

---

## 4 · SLOTH: The Grey Mire
*Belphegor · slate blue*

**Biome & Architecture**
- **A fog-drowned peat bog** where you can see about 8 tiles.
  - Its hills are a sleeping giant under peat and moss: a shoulder, a hip, a curled hand.
  - His breath is the tide. Every 12 s the low ground floods a tile deep, then drains.
- **The landmark:** **the Stopped Hourglass**, a tower of slate and smoked glass whose sand is frozen in mid-fall. It is the only thing that pierces the fog, and the court door is at its foot.
- **Materials:**
  - black peat water, grey reeds, causeways of rotten planks;
  - half-sunk cottages and a drowned chapel, clockwork gears in the mud;
  - cobwebs over everything, and moss beds as soft as pillows.
- **Light:** flat grey with no shadows, so there's no sense of direction. The only point lights are blue marsh-lights and the hourglass's faint sand-gold.
- **Geometry:** there are no straight lines.
  - Causeways loop between moss islands.
  - Deep mud halves your pace.
  - Duckweed hides bog holes.

**Folklore Integration**
- **Belphegor:** the demon who tempts with clever inventions and riches without labour. The mire is full of his shortcuts.
- **Skrýmir** (*Gylfaginning*): Thor's party slept in a "hall" that was really a giant's glove, and the night's earthquakes were his snoring. Here the land is the giant: his snores shake it, and waking him moves the map.
- **Hel's Niflheim:** the mist-world of those who died in bed.
  - Her servants **Ganglati** and **Ganglöt** (both names mean "slow-walker") are the new foe: shrouded, snail-slow, unstoppable.
  - Her bed **Kör** ("sick-bed") is the final lure.
- **The night hag and the owl:** albastı sits on a sleeper's chest, and tülükuş's silent cry foretells a death. Sleep invites both.
- **Urashima Tarō:** three days in the Dragon Palace were three hundred years at home. Rest costs time.
- **Lygtemænd,** the Danish lantern-men: marsh-lights that walk travellers into the bog.
- **Natives:**
  - bog slime (stage 1);
  - large bat (2): it hangs asleep in the willows until noise wakes it;
  - tülükuş and albastı (5);
  - Hel (7).
- **New foe:** Ganglati and Ganglöt.

**The Spatial Illusion: Focal Landmark Anchors**
- **One anchor, two lies:** the hourglass is all you can steer by.
  - **Distance:** haste drives it away, and stillness draws it in.
    - The tower stands on the giant's palm.
    - After 3 s of moving, the hand withdraws (1 tile/s). While you stand still, it curls toward you (1.5 tiles/s).
    - The goal comes to those who wait: Sloth's own argument.
  - **Bearing:** a loud fight near the giant's head (three or more foes engaged, or any blessing) makes him turn in his sleep.
    - Over 3 s the land rotates 90° round the hourglass.
    - The tower stays put. The chapel, the dead oak and every causeway you mapped swing to new bearings.
- **False anchors:** the marsh-lights look like lanterns along a path, but they always hover over bog holes.
- **Tell:** the hourglass is the pivot, so anything mapped *relative to it* stays true. The snores always come from the head; listen, and you know which way the land will turn.

**The Journey & Challenge**
- **The Trap: Torpor**
  - **Lever:** the Torpor meter.
    - It fills while you stand still (+10/s), wade through mud (+5/s) or lie in moss (+25/s).
    - It drains while you move on firm ground (−8/s), and by 20 with each kill.
    - At 100 you lie down and sleep.
  - **Moss beds** heal 5 health a second, but every second rested is a minute of mire-time. For each 10 s of rest:
    - the tide rises a notch;
    - a causeway plank rots away;
    - another servant climbs out of the bog.
  - **Belphegor's shortcuts:** a rail cart, a rope ferry and a clockwork lift.
    - Each promises to carry you to the hourglass, and works beautifully.
    - Each stops halfway on a lovely moss island, out of wind. Winding it takes 20 s of standing still.
  - **Manipulation:** the hourglass rewards idling, and idling fills Torpor and ages the world. The winning rhythm is the snail's: stand still for a beat (2 s at most, while the tower comes closer), then move and fight to shake the Torpor off.
- **The Climax: The Long Sleep**
  - **The lie:** at the hourglass's foot, on the threshold-stone over a bog hole, stands Hel's bed Kör, hung with gleaming curtains. *"Lie down. The door opens when you wake."*
  - **It springs:** whether you lie down or step past, the hourglass turns over.
    - The door opens only when the sand runs out, in **60 s**.
    - The giant half-wakes: the snoring stops, and the island tilts.
    - Every 5 s he yawns. Each yawn is a wave that pushes you 3 tiles toward the hole and adds 15 Torpor.
  - **Punishment:** lie in the bed, or let Torpor fill, and you sleep.
    - You wake at the realm's entrance three hundred years later.
    - The door *is* open, but the mire is older: half the causeways gone, the servants doubled, the tide a tile higher.
    - It is a loop that worsens each time: Mammon's restart, with interest.
  - **Survive it:** hold the 7-tile threshold island for 60 s.
    - **Beat the yawns.** Leap through each yawn's crest (your airborne frames pass the push), or brace on guard and be pushed 1 tile instead of 3.
    - **Keep moving.** Never stand still for more than 2 s.
    - **Stay noisy.** Kill the servants wading in from every side, since the noise keeps you awake (−20 Torpor each). They're slow, but they never stop.
    - **Go at once.** When the sand runs out, the giant's hand opens the door; go through before the next yawn.

---

## 5 · GLUTTONY: The Rotting Feast
*Beelzebub · orange*

**Biome & Architecture**
- **A banquet palace that is a gullet.** It opens as a candlelit feast and ends as a stomach, in four zones joined by archways that close behind you:
  1. **The Hall of Courses:** marble, white linen, silver domes, long tables you can run along, and roasts too glossy to be real, all in warm amber candlelight.
  2. **The Kitchens:** copper cauldrons over open fires, butcher's hooks, chopping blocks, greasy tile, and ovens shaped like mouths, in orange firelight.
  3. **The Larder:** salt-crusted walls turn ribbed and wet, the hanging meat becomes the ceiling, and the floor is slick enough to slide on.
  4. **The Stomach:** a vast cavity with acid pools glowing a sick orange-green, half-digested feasts and armour, and air thick with flies.
- **Materials shift at every seam:** marble to tile to flesh, silver to copper to tooth, candles to fires to glowing glands. Each archway is a ring of carved columns that are really teeth.
- **Light:** appetising at the start, sickly and wet at the end. Beelzebub's swarm drifts through as a living darkness.

**Folklore Integration**
- **Beelzebub,** Baal-Zebub, "lord of the flies", is Binsfeld's prince of gluttony: the swarm.
- **Gaki:** the hungry ghosts, with swollen bellies and needle throats, whose food turns to fire in their mouths. They are the new foe, and they mob anyone who smells of food.
- **Hansel and Gretel:** a house of bread and cake built as bait. The witch fattens her guest, checks his finger, and heats the oven. The stage-2 witch and her minions run the Kitchens.
- **Tepegöz** (*Dede Korkut*): the one-eyed giant who was fed a daily tribute of men and sheep. He is the Stomach's tenant at high tiers.
- **Gashadokuro:** the giant skeleton of the starved, haunting the feast. A ringing in the ears comes before its hands.
- **Natives:**
  - the witch and her minions, and blemmyes (stage 2), whose mouths are in their chests;
  - gashadokuro and Tepegöz (6).
- **New foe:** gaki.

**The Spatial Illusion: Architectural Transition Seams**
- **The floor plan is a lie:** the zones read as rooms of one palace, side by side. In fact they are stages of a single gut, and they run one way only.
  - Every archway is a valve dressed as a doorway, and its teeth close after you.
  - The zone behind you is digested: go back and you find wet flesh where the hall stood.
- **Hidden hand-off:** the first tiles past each seam keep the old zone's dressing, so the dining room seems to continue past the arch. Columns block the view at the exact line, and you notice only when the floor starts to move.
- **The space contracts:**
  - Floors tilt 1°, then 3°, then 6°, so you drift forward faster in each zone (0.2, then 0.5, then 1.0 tiles/s).
  - Each zone is 20% narrower and lower than the last, closing in on a hero who keeps getting wider.
- **Tell:** the next zone shows through the current one.
  - There are red veins in the marble, copper tiles that sweat, and floor tiles that pulse.
  - Crumbs roll toward the stomach, and candle flames lean into the draught of the throat.

**The Journey & Challenge**
- **The Trap: Bloat**
  - **Hunger:** here you starve, losing 1 point of maximum health every 4 s, down to 40%. Food is everywhere: each bite restores 4 maximum health, heals 10, and adds 1 **Bloat**.
  - **Bloat (0–10):** each stack costs 5% walk speed, grows your hitbox by 8%, and adds 1 frame to your leap's landing.
    - At 6, you no longer fit each seam's narrow arch (short and unguarded) and must take the wide one (long and garrisoned).
    - At 8, the Kitchens' thin crust floors break under you and drop you straight into the Stomach's acid.
  - **The smell:** from 3 Bloat, the gaki come for you.
  - **Burning it off:** every L3 that lands burns 1 Bloat. A purge font takes 5 Bloat and 20 health, and stuns you for 3 s.
  - **Manipulation:** starvation pushes you to eat, and every bite narrows your routes, slows you and thins the floor. The level rewards eating just enough.
- **The Climax: The Last Course**
  - **The lie:** in the heart of the Stomach, a single place is set under a silver dome, with the court door beyond it. *"Finish your plate, and you may leave."*
  - **It springs:** lift the dome and the plate is empty: the meal is you. The Stomach convulses:
    - acid floods in from the rim;
    - the walls squeeze the floor smaller over 45 s;
    - the swarm hunts you as a dark cloud;
    - the gut hurls food at you, and each piece that hits adds 1 Bloat.
  - **The exit:** the pylorus, a ring of teeth that opens for **2 s in every 10** and admits only a hero at **3 Bloat or less**.
  - **Punishment:** acid burns 3 health every half second. If the closing walls catch you, you're digested and wake in the Hall of Courses at the top of the gullet, hungry.
  - **Survive it:**
    - **Get lean.** Drop to 3 Bloat or less before the teeth open: land L3s on the gaki, use the one purge font, and eat nothing.
    - **Refuse the food.** Parry the thrown food, and it deflects into the gaki instead of into you.
    - **Time the teeth.** Leap the pylorus during its 2-second opening; your airborne frames carry you past the teeth.

---

## 6 · LUST: The Garden of Thorns
*Asmodeus · rose*

**Biome & Architecture**
- **A moonlit crimson pleasure garden:**
  - rose hedges as tall as walls, marble fountains, silk pavilions;
  - peach and pomegranate trees, and perfume mist pooled in the hollows;
  - behind every face of it, black bramble and the bones of grooms.
- **The landmark:** **the Bride**, veiled in white on a pale pavilion, lit by a moonbeam and singing. Her song is the realm's music, and she rises above every hedge.
- **The counter-landmark:** **the Black Thorn**, a dead tree of black thorns standing over the bramble. It never moves, and the court gate is at its foot.
- **Materials:** rose petals, pink marble, gilded trellis, white silk, pearl, black bramble, and silvered fountains that reflect everything but her.
- **Light:** silver moonlight and rose lanterns. The Bride is the brightest thing in the realm; the Thorn is a silhouette.
- **Geometry:** the rose paths form an inward spiral toward seven bridal bowers, while the thorn paths cut straight through the bramble.

**Folklore Integration**
- **Asmodeus** (Book of Tobit) killed seven bridegrooms on their wedding nights.
  - Tobias drove him off with the smoke of a fish's heart and liver, and the archangel Raphael bound him.
  - The fish's gall cured blindness.
  - So the garden has seven bowers, each with a dead groom and his last gear, plus the koi ponds, censers and gall described below.
  - **Raphael's** beams burn a permanent straight path through any hedge they cross.
- ***Elveskud,*** a Danish ballad: riding to his wedding, Sir Oluf refused the elf-king's daughter's dance and died of her blow. Rings of dancing elf-maids in the clearings drag passers-by in.
- **The Danish *ellepige*:** lovely from the front, hollow as a kneading trough behind. Every beautiful thing here is a facade, the Bride included.
- **The fox's wedding** (*kitsune no yomeiri*): a foxfire procession crosses the garden by night.
- **Karakoncolos** calls you by name in a loved one's voice.
- **Natives:**
  - jorōgumo (stage 3);
  - kitsune and rokurokubi (4);
  - karakoncolos (5): its Call pulls you toward the Bride;
  - arçura (5): the garden's guardian, whose roots are the thorns.
- **New foe:** the *ellepiger*, elf-maid brides. They are untouchable from the front, so strike their hollow backs.

**The Spatial Illusion: Focal Landmark Anchors**
- **Composition as a leash:** every hedge gap, fountain jet, lantern row and rose arch points at the Bride. You steer by the brightest, loveliest thing on screen, as players always do.
- **The anchor moves:** she's never where you last saw her. Each time a hedge hides her, she reappears at the focus of the rose path you're on. She is always ahead and never reached, winding you into the bowers at the spiral's heart.
- **The anchor that doesn't:** the Black Thorn is the only fixed point, and it's ugly, so players ignore it. To leave the garden, walk toward what you don't want.
- **Facades:** everything lovely is built facing the camera. Its back is hollow bramble that the fixed view never shows, and the honest thorn paths run *behind* the facades, out of sight.
- **Tell:** the Bride casts no reflection in the fountains, and the Thorn never moves on the minimap.

**The Journey & Challenge**
- **The Trap: Charm**
  - **Lever:** the Charm meter.
    - It fills while you face the Bride (within 45°, +20/s) or stand in perfume mist (+10/s). Otherwise it drains (−15/s).
    - At 100 you are **Enthralled** for 3 s: your controls are mirrored, and you're pulled toward her at 3 tiles/s.
  - **The rose paths** are smooth, perfumed and free of enemies, and they heal 2 health a second. But each one points at her, so walking one *is* facing her: the geometry itself charms you.
  - **The thorn paths** run straight, cost 2 health a second, slow you by 20%, and are held by the brides. Walking them means turning your back on her.
  - **Tobit's cure:**
    - Catch a koi (stand in a pond for 2 s) and burn it at a censer. You get 10 s of smoke, 4 tiles wide, that blocks Charm, turns facades to glass and lights the honest paths.
    - A rare golden koi's gall grants 20 s of true sight: the Bride shows her hollow back.
  - **Manipulation:** comfort and healing lie toward desire; pain and clarity lie away from it. Every shot is framed to make you look.
- **The Climax: The Wedding Night**
  - **The lie:** you reach the Black Thorn, and its gate grinds open over 10 s. The song stops. The Bride is no longer on her pavilion: she's walking up behind you. *"Turn around."*
  - **It springs:** the garden collapses toward her.
    - Hedges uproot and crawl inward.
    - Brides and dance rings close in from the sides.
    - Facing her now fills Charm **three times as fast**.
  - **Punishment:** if she takes you, you become the eighth groom. You take a heavy blow and Doom (your next wound ×1.3), and you wake in the innermost bower with the whole spiral to walk again.
  - **Survive it:** either way works.
    - **Endure.** Keep your back to her for the gate's 10 s. Fight only what's in front of you, and burn your fish at the gate's censer for a smoke shield.
    - **Turn the trick on her.** Leap *through* her (your airborne frames pass her embrace) and land behind her, facing her hollow back, which doesn't charm. Three blows to it unmake her, and the gate opens at once.

---

## Build hooks

| Realm | Builds on what exists | Needs |
|---|---|---|
| Pride | `portals` as one-way mirror-doors; Pit tiles for the falling rings; a duelist on the hero's own kit (`brawl`) | an echo brain that replays the hero's inputs 24 f later, mirrored; mirrors that draw the hero's state (crown, health) |
| Envy | two regions of one map, with the dive and the geyser as warps; `Hazard` for the scorching rim; the kappa's pull | the lower region drawn under a water plane with parallax and refraction; the Eye's sweep and its power-steal |
| Wrath | `Rider::Raise` for the rising dead; Michael's turned foes; Pit tiles as lava; poise breaks | slabs as moving ground in height bands; crack counters per slab; the truce ring |
| Sloth | the albastı's pin, the tülükuş's doom, Hel's raise; the vault's restart for the Long Sleep | land that rotates round a pivot; the Torpor meter; a world-ageing step |
| Gluttony | gates that shut behind the hero; a venom-style tick for the acid | floor drift; Bloat as a modifier on speed, hitbox and landing; arches gated by width |
| Lust | the karakoncolos's Call (pull); Raphael's beam | a Charm meter driven by facing; mirrored input; facades whose backs the camera never shows; censers |
