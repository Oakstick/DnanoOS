# The UI and input, brought up to date

The user's brief (2026-10-06): "You are an input and UI designer. Reconfigure the mapping for controller
and keyboard. Change the background of the loading screen to a controller legend. Go through the
entire UI system and bring it up to today's standards, and add a mini map. The archangels are not
that visible and the old man is very small in the corner; the quality has to be better."

What exists is in the survey of 2026-10-06 (summarised in §1). The plan is in §2–§9, the build in
§10. Rules that hold throughout: the old man always leads to good; no Devil anywhere; nothing in
the UI may change the simulation (the bots' harness numbers stay identical); the web build keeps
working (no threads, no file system); every new screen goes on the debug Map.

---

## 1. Where it stands

- **Text** is font8x8 at 4× in a 512² atlas: blocky capitals, rough lowercase, monospace widths.
- **Input** is two hard-coded tables (`input.rs::key_to_action`, `app.rs::poll_pad`): no rebinding,
  no saving of bindings, no rumble, triggers read as digital, prompts written into strings ("B THE
  OLD MAN'S WORD", "R / LT", "Q {potion}") that never follow the device being used. The D-pad
  duplicates the stick; L3 (a click of the stick) does the Bifrost Step; RT and X both attack.
- **The HUD** is charcoal/copper panels in the four corners, one `t` unit (20 px at 720p, 28 px at
  1080p); no safe-area margin, no UI scale.
- **The old man** speaks from a 3t (84 px) square at the bottom left: a 2D flattening of his voxel
  model taking every second voxel, drawn rect by rect.
- **The archangels**: a 9.4t card at the bottom left only while a blessing is held; their shrines
  in the world are 0.9 tiles tall with a soft glow; their names float as 0.52t labels.
- **Settings** sit inside the pause menu (view, slow motion, shake); there is no settings screen,
  no volume, sensitivity, invert, text size, colour-blind or flashing options.
- **The map** is a debug list of level stops; there is no minimap and no explored-tile tracking.
- **Loading**: the engine's splash (a gradient and "Distinguisher Nano", its art file missing) and
  the in-game loading card (black, the level's name after 0.6 s).

## 2. Principles (today's standards)

1. **Readable first**: vector type at every size, minimum 18 px body text at 1080p, contrast at
   least 4.5:1, a UI scale setting (80–150%), safe-area margins (5% of each edge).
2. **Device-aware prompts**: every prompt shows the glyph of the device used last (Xbox, PlayStation,
   keyboard and mouse), and changes the moment the player switches.
3. **Everything rebindable**, saved, with sensible defaults that match the genre's conventions.
4. **One look**: dark glass panels with a thin copper rule, gold for what matters, one type family,
   icons before words, animated in and out (no pop).
5. **Diegetic where it can be**: the archangels and the old man are characters, not labels; they
   get presence in the world and on screen.
6. **Accessible**: colour-blind palettes for tells and markers, subtitles size and background,
   reduce flashing and shake, hold-or-toggle for the shield, rumble off/low/high.

## 3. The mapping (defaults; everything rebindable)

Controller (Xbox names; PlayStation in brackets):

| Button | In play | In menus |
|---|---|---|
| Left stick | Run | Move the highlight |
| Right stick | Camera | Scroll |
| R3 [R3] | Recentre the camera | – |
| L3 [L3] | Minimap: zoom in/out | – |
| A [Cross] | Jump / evade | Select |
| B [Circle] | The old man's word (his guide) | Back |
| X [Square] | Attack | – |
| Y [Triangle] | Call the old man (his wares) | Secondary (tab) |
| LB [L1] | Shield (hold) | Previous tab |
| RB [R1] | Drink the chosen potion | Next tab |
| LT [L2] | Bifrost Step | – |
| RT [R2] | Archangel's power | – |
| D-pad ← → | Choose a potion | Move |
| D-pad ↑ | Full map | Move |
| D-pad ↓ | Switch camera (follow/top) | Move |
| View [Share] | Full map | – |
| Menu [Options] | Pause | Back |

Keyboard and mouse:

| Key | In play |
|---|---|
| W A S D (arrows) | Run |
| Mouse | Camera (while captured) |
| Left click | Attack |
| Right click (hold) | Shield |
| Space | Jump / evade |
| Left Shift | Bifrost Step |
| R | Archangel's power |
| Q | Drink the chosen potion |
| Mouse wheel / 1–5 | Choose a potion / drink slot N |
| E | Call the old man (his wares) |
| B | The old man's word (his guide) |
| M or Tab | Full map |
| C | Switch camera |
| Esc | Pause / back |
| F11, Alt+Enter | Fullscreen |

Debug keys (F1–F10, 0) stay as they are, listed only on a developer page. Mute moves into Audio.

Rumble: a light pulse on a hit landed, a stronger one on a hit taken, a long low one on a guard
break, a short one on a parry; scaled by the setting (off, low, high).

## 4. The look

- **Type**: two families, both OFL: a display serif for titles and names (Cinzel), a clean
  humanist sans for everything else (the Ubuntu shipped with the splash, or Alegreya Sans), drawn
  from a signed-distance-field atlas made at start-up (crisp at any size, outlines and glows for
  free). The pixel font stays only for the logo's look where it is part of the art.
- **Panels**: dark glass (ink 85%, a soft inner gradient), 1 px copper rule, corner notches;
  9-slice, so any size; slide and fade in over 0.15 s.
- **Icons**: the 16 px sprites redrawn at 64 px (vector-like, from shapes), button glyphs drawn
  as shapes (coloured face buttons, bumpers, triggers, keycaps).
- **Colour**: the existing palette (gold, sand, ink, copper), plus one alert red and the angels'
  four colours; a colour-blind variant swaps reds and greens for orange and blue in tells and
  markers.

## 5. The HUD

```
┌────────────────────────────────────────────────────────────────────────────────┐
│ [portrait] RAGNAR ♥♥♥            BOSS / PRINCE / PIT BAR              ┌──────┐ │
│ ███████████ 105/105                                                   │ MINI │ │
│ ▬▬▬▬ stamina                                                          │ MAP  │ │
│                                                                       └──────┘ │
│                                                              OBJECTIVE          │
│                                                              keys · silver      │
│                                                                                │
│                                                                                │
│   ┌───────────────────────────────┐                                            │
│   │ [OLD MAN, 3D, 7t]  THE OLD MAN│  (dialogue: bottom centre, above loadout)  │
│   │  text in vector type ...      │                                            │
│   └───────────────────────────────┘                                            │
│ [ANGEL]                [weapon] [shield] [belt 1-5]                      area  │
└────────────────────────────────────────────────────────────────────────────────┘
```

- **Top left**: the hero's live 3D portrait, name, lives, health with its trail, stamina.
- **Top right**: the **minimap** (§7); under it the objective line and the purse (silver, runes,
  keys); the score moves into the pause screen's summary (it is a remnant of the arcade game).
- **Top centre**: boss, prince, Pit clock and general banners, as now, restyled.
- **Bottom centre**: the loadout and belt with device glyphs on each slot.
- **Bottom left**: the **archangel** (§6), always shown once one is met.
- **Dialogue** (§6): bottom centre, above the loadout, 7t portrait.
- **Floaters, banners, name cards**: the same, in the new type.

## 6. The archangels and the old man

- **The old man on screen**: a live 3D portrait, rendered by the game's own renderer into a texture
  (his cast model, lit, framed head and shoulders, blinking, his mouth moving while text types),
  7t square (196 px at 1080p), in a dialogue panel at the bottom centre with his name plate, the
  text in vector type at 0.75t, and a prompt glyph to skip or to open his guide. The guide page uses
  the same portrait at 9t. The prince's whisper keeps its own frame (smoke, the sin's colour), so the
  two can never be confused (the old man is amber and gold; a whisper never is).
- **The archangels on screen**: a 4t emblem at the bottom left in the angel's colour (wings around
  a halo, drawn as shapes), the charges as pips round it, a radial fill while recharging, a pulse
  and the glyph of its button when ready, the angel's name in display type. When a blessing is
  given, a full-width banner with the angel's name and what the power does (2.5 s).
- **The archangels in the world**: each shrine gets a pillar of light visible from across the level
  (tall, faint, the angel's colour, gone when spent), a brighter halo, and an off-screen arrow at
  the screen's edge toward an unspent shrine within 25 tiles; their icons on the minimap.

## 7. The minimap and the full map

- **Minimap**: top right, round, 7t across (196 px at 1080p), north-up or turning with the camera
  (setting), showing the explored part of the level: floor, walls and water in the level's
  palette; the hero as an arrow; foes within range as red dots (gold for elites and rivals, a
  crown for a general, a skull for a boss); shrines in their angel's colour; keys, chests, gates,
  portals, the Pit's doors, the exit; the old man's lamp where the compass points. L3 zooms.
- **Explored tiles**: a fog of war per level, revealed in a radius round the hero (presentation
  only: never read by the game), remembered for the level while it lives.
- **Full map** (View / M / Tab): the whole explored level large, with a legend, the objective, and
  the realm's name; zoom and pan with the sticks or the mouse.
- **Built from** the level grid at load (a texture made once, its fog updated as tiles are
  revealed), icons drawn live on top.
- **The debug stop list** moves to a developer page (from the pause menu while debugging, and F12).

## 8. Screens

- **Settings** (from the title and the pause menu), tabs: Gameplay (camera, free movement, slow
  motion, shake, minimap turning, hold or toggle shield), Controls (rebind keyboard and controller,
  sensitivity, invert Y, deadzones, rumble), Audio (master, music, effects, voices; mute), Display
  (fullscreen, quality preset, UI scale, text size), Accessibility (colour-blind tells, subtitle
  size and backing, reduce flashing).
- **Pause**: Resume, Settings, Controls, Map, The Ledger, Call the Old Man, Quit to Title, Quit Game;
  beside it the hero's summary (realm, keys, silver, runes, score).
- **Controls**: the legend (§9) with every binding, live from the bindings.
- **Title, select, shop, guide, ledger**: restyled in the new type and panels; layouts kept.

## 9. Loading screens are the controller legend

- **The engine's start screen** (`render/splash.rs`): its backdrop becomes the controller legend:
  the gamepad drawn large in the middle, every button called out to what it does (from the saved
  bindings), the keyboard and mouse summary down the side, the game's title above, and the
  progress bar and its steps kept below. The splash's own thread draws it (no GPU wait).
- **The in-game loading card**: the same legend (small), with the level's name and a tip from the
  old man; it shows the controller if one was used last, else the keyboard.

## 10. The build

- **U0, foundations (one builder, first)**: the SDF type atlas and the new text API (sizes, weights,
  outline, measure by real advances); textured quads in the overlay (atlas regions, rotation, a
  circular mask); offscreen portrait renders into atlas regions; the action map with bindings as
  data, persisted (`vh.bindings`), rebind API, device-aware glyphs (drawn as shapes), the new
  defaults (§3), rumble through gilrs force feedback; every hard-coded prompt replaced by a glyph.
- **U1, in parallel after U0**: (a) the minimap, the fog and the full map; (b) the settings screen,
  rebinding UI and the two loading legends; (c) the HUD restyle, the old man's dialogue and portrait,
  the archangels on screen and in the world.
- **U2**: a pass over every screen in captures at 720p, 1080p and 4K; the harness numbers checked
  unchanged; release.

## Build log

### U1a, the map's data (2026-10-06)

The part of U1a that needs no new drawing, built beside U0: the fog, the map's image and its
markers, as pure code in two new files, with a debug picture to look at before the overlay can
draw it. Nothing of the overlay, the HUD, input or the renderer was touched.

- **The fog** (`logic/explore.rs`, `Explore`, held as `Game::explore`): a radius of 9 tiles round
  the hero, by line of sight (Bresenham from his tile; rock blocks it, and so do two rocks meeting
  at a corner), the walls about every floor tile in sight revealed with it. It looks again only
  when he comes onto another tile (and every 20 steps while he stands, for a door opening before
  him). Kept while the level lives and through a fall; on a load it is kept if the level is the one
  he was in (its name and its rock layout hashed: a restart, Continue into the same stage) and
  begun again otherwise. **Revealed whole from the start**: the hub, the gym, the Pit, the lair, and
  every boss arena and court (`revealed_whole`): small, walled by lava or stone, and nothing in them
  is a secret to find. The estates and the realms (the vaults included) are explored. It keeps a log
  of the tiles revealed, in order, for the image to patch. Wiring: `enter` as a level loads
  (`load_inner`), `observe` at the end of `reset_round` and in `tick` after the hero moves. Nothing
  reads it but the map; `Explore::off` switches it off for the test that proves so.
- **The image** (`render/minimap.rs`, `MapImage`): RGBA, the grid's north up, at
  `px_per_tile(w, h)` = as many pixels a tile as fit 1024 (8 at most, 2 at least): the garden 8, the
  hub 8, the vaults 7, the Serpent Grotto 4. Each tile is a class (`class::VOID, WALL, FLOOR, RAMP,
  BRIDGE, WATER, HOLE, SPECIAL`) in a `Style` of the level (`Style::of(theme, boss, dungeon)`: the
  garden, the Cisterns, the crypts, the meadow, the hub, the gym, the vaults, each realm and court by
  its sin, the Pit, the lair) and its `Palette`, muted toward grey so the markers read: floor
  lighter by 8% a level it stands up, a dark lip where it drops, a grain; walls lighter than the
  floor with a dark outline where they meet it; rock that touches no floor left clear, so the level
  reads as its own shape; the garden's and the arena's watercourses, the realms' pools and bogs as
  water; lava darkened but hot (the hub, the gym, Wrath's court, Gluttony's crust); chasms near
  black; bridges as planks across the way over; ramps as steps; the hollow and the Pit's platform
  as their own floor. The fog: unseen tiles clear, each pixel as seen as the four tile middles
  about it (smoothed over 0.3..1), so the frontier fades inside the seen side; clear rock next to a
  seen tile counts as seen, so walls never fade on their outside. Built once a level; `sync(game)`
  rebuilds on a new level (the fog's epoch), patches the tiles the fog's log has revealed since (and
  two round them), and looks over 4 rows of the grid a call for ground that changed (a gate opened,
  a causeway risen, a realm's rules), repainting that tile and its neighbours. `dirty` (a
  `PixelRect`) is the union of what changed since `take_dirty()`.
- **The markers** (`markers(&Game) -> Vec<Marker>`, places first, foes, the hero last): `Marker {
  kind, at: V (tiles), known }`, `Marker::color()`. Kinds: `Hero { heading }`, `Foe(FoeRank::
  Plain | Elite | Rival | General | Boss)` (a pack's leader is elite; the prince, Krampus and the
  Cyclops are bosses), `Shrine { angel, spent }`, `Key`, `RedKey`, `Remedy`, `Potion`,
  `ItemShrine`, `Loot`, `Chest`, `Gate { open }`, `Entry`, `Portal { sin, conquered }`,
  `HouseDoor { open }`, `PitDoor { open }`, `Platform { half }`, `Lamp` (where the old man's lamp
  points: the realm's far portal, always true and always known), `Instrument { order, broken }`,
  `Post`, `Cage { open }`. Known: the hero, the lamp and the boss always; foes while in his sight
  (within 18 tiles where the level is revealed whole); the post once seen or marked by a caught
  herald; everything else once its tile is seen. The prince's turned needle is not a marker: the
  map never shows a lie.
- **The debug picture**: `--capture out.png ... --minimap` writes `out-minimap.png`
  (`debug_picture`: the image over the ink panel with a faint grid every 8 tiles, enlarged up to
  4x, every known marker drawn on the CPU: the hero an arrow, foes red dots, gold for leaders and
  rivals with a red ring for the rival, a crown for a general, a skull for a boss, shrines as
  diamonds in their angel's colour with a halo, keys gold diamonds, chests, gates, portal rings in
  their sin's colour, the lamp an amber diamond). Looked at for the garden after a walk, a crypt,
  the vaults and two realms crawled by the bot, the hub, the Pit, a court, the lair and the
  Cyclops' meadow; the lava and the floors were darkened and the frontier's fade moved into the
  seen side after the first look.

Numbers (quick profile, this laptop): the largest map, the Vaults of Mammon (138 x 102 tiles,
966 x 714 at 7 px), built in 9-11 ms; a step of the fog (a full look from a new tile) 0.006 ms;
patching 400 looks' worth of fog 0.9 ms; an idle `sync` (nothing new, 4 rows looked over) 0.006 ms;
`markers` for the vaults (1390, their gold) 0.08 ms. The harnesses print the same numbers before
and after: `--crawl 1` (six crawls), `--general 1 2 4 1` and `--run 2 1` (the whole run to the
ending, 87 min of play) are identical line for line.

Tests: `explore` (it reveals round him and not through walls, and sees into a room only through
its door; kept through a reload and forgotten for another level, the whole-revealed kinds; its log
holds each tile once; off reveals nothing), `minimap` (the image's size and colours for the garden,
the Cisterns, a crypt, the meadow, the vaults, two realms, the hub, two courts, the gym, the Pit
and the lair; a patch covers exactly the new tiles and two round them and matches a fresh build,
changed ground is found and patched round its tile, a new level rebuilds; the markers in the
garden (shrines, the gate, the keys, the foes known only in sight), the vaults' lamp and chests,
the hub's seven portals, the Pit's doors and platform, a court's prince; the fog never changes the
game (the bot through the vaults and a walk in the garden, with the fog and without: the same
deeds, hero, foes and score); the timings above, the fog's step held under 0.1 ms).

Left for the on-screen builder (U1a proper, after U0): own a `MapImage` beside the overlay, call
`sync(&game)` each frame and upload `take_dirty()`'s rows into a texture (1024² at most); draw the
round minimap (7t, north-up or turning) from it with the markers as U0's shapes and glyphs, the
full map (zoom and pan, legend, objective, the realm's name), L3 for zoom; the colour-blind palette
for the foes' dots; the Map stop.
