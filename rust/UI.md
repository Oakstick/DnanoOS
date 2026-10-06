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

(to come)
