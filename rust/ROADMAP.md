# The Portals of Hell: roadmap

Enemy AI, rivals, generals, the old man's counsel, the princes' cheats, the workshop and the feel: `WARBAND.md` (built 2026-10-04). Combat: `COMBAT.md` has its own roadmap. Phases 1 to 4 are built: the Sparring Ring (frame data, the fighter state machine, the threat manager, three archetypes), the bot balance harness, the hero's kit as numbers, and the roll-out to the Vaults of Mammon, whose packs now fight by frame data. Each new realm builds its foes as breeds of the archetypes and its fight on the brawl (`game::brawl`).

Status: phases 0, 1 and 2 are done (the engine groundwork, the hub with a polish round on its art, and the whole loop with stand-ins), and phase 3's realm, the Vaults of Mammon, is built (`logic/vault.rs`, `game/vault.rs`, `render/view/vault.rs`: a Diablo-style dungeon of rooms and corridors in gold and marble, packs that idle until he comes near, gold everywhere, and Mammon's toll at the far portal); the other six realms (phases 4 to 9) are built too, each to its design in `REALMS.md` (`logic/realms`, `game/realms`, `render/view/realms`: the Hall of Mirrors, the Serpent Grotto, the Burning Field, the Grey Mire, the Rotting Feast and the Garden of Thorns, each with its spatial lie, its trap and its prince's lie at the end). Greed's prince is built too: Mammon fights in his own court, **the Counting House** (`logic/court.rs`, `game/court.rs`, `render/view/court_greed.rs`: an octagonal hall of gold-veined dark marble, black-and-gold pillars, heaps of coins, counting tables, his throne under giant brass scales), by frame data as the fourth archetype, the Prince (`logic/combat/prince.rs`, COMBAT.md §13): the Banker's sceptre and coins that lie on the floor and burst, the Collector's two gold golems and a grasp that drains silver until he is struck, the Hoard's rain of coins and slam. He drops the Key of Greed, and the way on is as before. The other six princes fight as themselves too, each in his own court on the same archetype (`PRINCES.md`, built 2026-10-04): Lucifer on the Throne of the Morning Star (his beam, his mirror step and Reflections, the Fallen's stars; struck from behind he bleeds whole), Leviathan at the Drowned Bell (he dives, covets the archangel you last called, and coils the burning bell), Amon at the Anvil of Wrath (no hide, a rage that quickens him, charges that daze him on stone, lava rifts), Belphegor in the Hall of Stopped Clocks (asleep until struck, asleep again and mending while his nightmares fight), Beelzebub at the Banquet of Flies (he eats from his platters to heal unless struck mid-meal, scatters into flies) and Asmodeus in the Rose Bower (a blown kiss that takes the guard away, thorns, and Raphael's censers that strip his hide). Each drops his sin's key; the bots' expert beats each in 88-135 s and a masher none. Next: the House of the Devil (phase 10). Decisions 1 to 3 were settled on the proposals: a realm and its prince behind each portal, the hub straight after the Garden's Cyclops (the Cisterns and the Bone Vaults are off the road), and free order with difficulty following the keys held. Changes from the plan: the estate's single gate stays as it was, with the hub's portals kept in their own list beside it; the bridges are three tiles wide rather than two, so they sit centred on the platforms; and the stand-in realms are fresh estate seeds rather than the retired layouts. The names, colours and bosses below are proposals; the open decisions are listed at the end. Sizes: M is roughly one working session, L several.

## The idea

The Cyclops of the Garden of Krampus drops the red diamond key, and its gate opens the way down to **the Portals of Hell**, a hub over a sea of lava. Eight stone platforms float round a volcano where the House of the Devil stands. The hero comes in from outside onto the entrance platform. Each of the other seven platforms has a portal gate to the realm of one of the seven deadly sins: its name is carved over the door, and the door itself shows the sin. Wooden bridges join every platform to its two neighbours, in a ring.

Each realm ends with a boss, the prince of that sin, who drops the sin's key. With all seven keys the seal breaks: causeways rise out of the lava to the volcano, and the last fight waits in the House of the Devil.

With the hub and the House that makes nine new levels, and with Krampus and Satan nine new bosses: the "9 more levels and 9 more bosses" planned earlier.

## The layout

A 3×3 grid of platforms with the volcano in the middle cell. Every ring bridge then runs straight north–south or east–west, so the hub works with grid movement as well as free movement.

```
  NW  Wrath  ════  N  Sloth  ════  NE  Greed
      ║                              ║
  W   Envy        [ volcano ]        E   Gluttony
      ║         House of the Devil   ║
  SW  Pride  ════  S  Entrance ════  SE  Lust
                      ║
                 from outside
```

- **Size:** 65×65 tiles. Platforms are 13×13, with 7 tiles of lava between them and 6 round the edge.
  - The hero's jump reaches 2 tiles, so only the bridges cross.
  - Lava all round the edge also keeps the grid's wrap-around from ever showing.
- **Bridges:** 2 tiles wide, made of bridge tiles as in the garden, so slimes will not cross them.
- **Lava:** Pit ground (impassable on foot, flyable), drawn far below the platforms so they seem to float.
- **Entrance:** the south platform, reached from outside along a rocky ledge from the south edge. On it stands the Seal of the Seven, an altar with seven key sockets.
- **Centre:** cut off until the seal breaks. Then four causeways rise from the N, E, S and W platforms; the corner platforms reach them round the ring.

## The seven portals

Clockwise from the entrance, in the order of the terraces of Dante's Purgatory.

| Platform | Sin | Colour | On the door | Prince (boss) | His realm |
|---|---|---|---|---|---|
| SW | Pride | violet | a peacock fanning its tail under a crown | Lucifer | a palace of marble, gold and mirrors |
| W | Envy | green | a great slit-pupilled eye in a coiled serpent | Leviathan | a sunken grotto of green water and serpent bones |
| NW | Wrath | red | a snarling horned face in flames over crossed axes | Amon | a burning battlefield of broken weapons and lava rivers |
| N | Sloth | slate blue | a snail on a stopped hourglass, hung with cobwebs | Belphegor | a grey swamp under fog, where time runs slow |
| NE | Greed | gold | gold spilling from a chest into a grasping hand | Mammon | vaults heaped with coins, chests and scales |
| E | Gluttony | orange | a fanged maw swallowing a feast | Beelzebub | a rotting banquet hall of cauldrons and flies |
| SE | Lust | rose | two serpents twined into a heart among thorned roses | Asmodeus | a crimson garden of roses and thorns |

Wrath's prince is Amon rather than Satan, because Satan is the last boss.

## Phases

### Phase 0: groundwork in the engine (M) — done

- **Sins:** `logic/sins.rs` defines `Sin`, with its name, colour, emblem, prince and realm.
- **Where the adventure stands:** a `Place` (Garden, Meadow, Hub, Realm(sin), SinArena(sin), DevilHouse) sits beside the difficulty tier (today's `dungeon` number).
  - Save format v2 adds the place and the sins cleared. Saves in the old v1 format still load.
- **Themes:** levels name their theme, and the view builds by theme instead of the `meadow: bool` it takes now. The themes are Garden, Dungeon, Meadow and Hell, with one per realm added later.
- **Several gates per level:** `portals: Vec<(P, Portal)>` replaces the single `gate`.
- **Debug map:** it lists the hub, each realm, each prince and the House.

Done when every current level plays exactly as it does now and all tests pass.

### Phase 1: the hub (L) — done

1. **Layout:** `portals_of_hell()` builds the grid. Tests check:
   - there are eight platforms;
   - the ring is joined only by bridges;
   - every portal is reachable;
   - the centre is out of reach.
2. **Art,** in `render/view/hell.rs`:
   - **Lava sea:** it flows and glows under a darker crust, with bubbles, and lava falls pour off the platform edges.
   - **Platforms:** cracked flagstones with glowing veins and jagged rock hanging beneath. On top: bones and skulls, broken columns, ruined walls, braziers and chains.
   - **Wooden bridges:** planks, some charred or missing, with rope rails on posts and a slight sag.
   - **Volcano:** lava rivers down its flanks, a glowing crater, and a column of smoke and ash.
   - **House of the Devil:** a black keep with horned spires and a door glowing red, behind a wall of fire.
   - **Sky:** blood red to violet, with ash falling, embers rising and demons wheeling far off. The hub gets its own ambience and music.
3. **Seven portal gates:**
   - Each gate has a stone arch and double doors carrying the sin in relief.
   - The sin's name is carved into the lintel, and braziers burn in the sin's colour.
   - The name floats over the gate as the hero comes near.
   - The doors open as he comes up, showing a swirling portal in that colour. Stepping in plays the portal sound and a flash, then loads the realm.
   - Once the realm is cleared, the doors stand shut with the key set in the lock, and the flames burn calm.
4. **Arriving:** the hero walks in from outside and the level name comes up: "The Portals of Hell". The first time, the tour flies round the seven doors with one card for each.
5. **The seal:** with seven keys, the ground shakes and the camera sweeps round. The causeways rise, the wall of fire dies, and the House opens.

Done when the hub is playable, reads like the concept art and holds 60 fps, with captures of every door.

### Phase 2: the whole loop, with stand-ins (M) — done

- **After the Garden:** Cyclops I's gate leads to the old man, then to the hub.
- **One portal's loop:**
  1. The portal leads to a realm: for now an estate layout, seeded and tinted per sin and carrying the sin's name.
  2. Eight golden keys open the realm's gate.
  3. Behind it is the prince's arena, for now a recoloured Cyclops carrying the prince's name.
  4. He drops the sin's key, which opens the arena gate.
  5. The old man comes next, then the hero is back in the hub before that portal, now sealed.
- **Order:** realms can be taken in any order. Each is as hard as the number of sins already cleared.
- **Saves:** Save and Continue work at every step.

Done when the game plays from the Garden to the door of the House.

### Phases 3 to 9: the seven realms (L each)

One realm per phase, each finished before the next. Each gets:
- its own ground, props, sky and music;
- one or two new foes;
- its prince, with his own way of fighting;
- its key.

Proposed order: Greed first, because gold and treasure are easy to read and Mammon's fight is simple to explain. The others follow in any order.

| Prince | How he fights (proposal) |
|---|---|
| Lucifer (Pride) | splits into mirror images, and only the real one bleeds; beams of light |
| Leviathan (Envy) | a serpent circling through water; copies the archangel power you last used |
| Amon (Wrath) | fiery charges; cracks the floor open into lava; grows faster the more he is hurt |
| Belphegor (Sloth) | sleeps until struck; pools of slowness; yawns that push you back |
| Mammon (Greed) | coins that are traps; steals your silver unless you strike him; gold golems |
| Beelzebub (Gluttony) | fly swarms; eats crosses and bottles to heal; belly slam |
| Asmodeus (Lust) | charm (your controls flip for a moment); thorned vines; illusions |

### Phase 10: the House of the Devil (L)

1. Krampus waits at the door: the one who took your friend.
2. Satan fights in the heart of the volcano, in several phases.
3. Your friend is set free.
4. An ending cutscene (Scene II), then the credits.

## Every phase

- Tests for the new rules, with the whole suite passing.
- Captures of everything new, checked before the phase counts as done.
- A frame-time check against the previous build. The hub's budget is about 300k static voxels.
- The README updated, and a commit for each step.

## Decisions (proposals first)

1. **Behind a portal:** a realm and its prince / only the prince's fight / only a realm with keys.
2. **Where the hub sits:** straight after the Garden's Cyclops, retiring the Cisterns and the Bone Vaults / after those two levels. If they retire, their layouts can stand in for realms in Phase 2.
3. **Order:** free, with difficulty following the keys you hold / fixed, with harder portals marked on their doors.
4. **The hub:** calm, with no enemies / demons patrol it, more with every key.
5. **The end:** Krampus, then Satan / Satan alone.
6. **Details:** the sin order, colours and princes as in the table above.

Phases 0 and 1 do not depend on these. Phase 2 needs decisions 1–3, and the realms need 4–6.
