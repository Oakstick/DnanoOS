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

### U0: the foundations (2026-10-06)

Built on `ui-u0`: the type, the overlay's textured quads, the portraits, the input as data, the
prompt glyphs, rumble, and mute off M. The input half was built alongside on `ui-u0-input` and
merged. The simulation is untouched: `--court`, `--pit 2 8 1`, the sixteen ring bouts
(`--bout 1..16 2 1 1`) and `--stage-three 1` print the same numbers as before U0.

**The type** (`render/text.rs`, `render/overlay.rs`). Alegreya Sans Medium (the body) and Cinzel
at its bold weight (the display), both SIL OFL 1.1, in `assets/fonts/` with their licences and
embedded with `include_bytes!`. At start-up every glyph of printable ASCII, Latin-1, Latin
Extended-A and the marks the game writes (dashes, quotes, arrows, the heart, drawn since neither
face has one) is rasterised at 96 px to the em, its exact signed distance found in and out
(Felzenszwalb and Huttenlocher), halved to 48 px to the em and shelf-packed into one 8-bit
texture, 1024 wide and 2048 tall (2 MB). What one face lacks, the other lends. The kerning of every ASCII
pair comes from the faces' GPOS `kern` lookups (ab_glyph reads only the old `kern` table, which
neither face has; `ttf-parser`, already in the tree under ab_glyph, is now a direct dependency).
The overlay's shader reads the field four times a pixel on a rotated grid, so small text keeps its
shape (mip levels made small text blobby: dropped).

```rust
let s = Style::display(t * 1.3, rgba(GOLD, 1.0)).centered().shadow(2.0, 3.0);  // or ::body
o.text_styled("THE LEDGER", x, y, &s) -> f32;            // y is the capitals' middle, as before
o.measure_styled(text, &s) -> f32;                       // real advances, kerning, tracking
o.pens(text, &s) -> (Vec<f32>, f32);                      // each letter's pen, for per-letter effects
o.wrap_lines(text, max_w, &s) -> Vec<String>; o.wrap_styled(text, x, y, max_w, line_h, &s) -> f32;
Style { family, size, color, align, weight, outline: Option<(px, color)>, shadow: Option<Shadow>,
        tracking, tabular }   // .bold() .weight(w) .outline(w, c) .shadow(off, blur) .glow(r, c)
                              // .tracking(em) .tabular() .centered() .right() .color(c) .size(s)
```

`size` keeps the old font's meaning (capitals 0.7 of it tall), so every old layout sits where it
did. The old `o.text(..., bold)` now draws the body sans, or for bold lines of title size (0.85 of
the HUD unit and up) the display serif; small bold lines are the sans made heavier. Capitals are
spaced a little and small text opened up as it shrinks (optical spacing, in `measure` too). The old
drop shadow is kept short and crisp on small text. The title word is Cinzel with an outline, a
bronze bevel and the travelling glint (laid out by the face's advances); the pixel font survives
as `o.pixel_text` (nothing uses it now). `--capture out.png --specimen` draws a type specimen (both
faces at the HUD's sizes, plain, bold, shadowed, outlined, glowing, the accented names, a wrapped
paragraph, every glyph on every device, and the quads).

**Textured quads** (`render/overlay.rs`). The UI atlas is 2048 square (RGBA): the old atlas in its
corner (`LEGACY`), `ICON_REGION` (512 square, for the HUD builder's redrawn icons),
`MAP_REGION` (1024 square, for the minimap's level and fog) and 32 portrait slots of 256. Every
vertex carries how its texel is read: a field (text), straight, premultiplied (portraits), or
masked to a circle.

```rust
o.region(r: AtlasRegion, x, y, w, h, tint);
o.region_rotated(r, cx, cy, w, h, angle, tint);
o.region_circle(uv_centre, uv_half, angle, cx, cy, radius, tint);   // the minimap's round window
o.nine_slice(r, border_texels, x, y, w, h, edge_px, tint);
o.textured_quad(corners, uvs, tint, FLAG_*);                       // the low-level one
o.triangle(a, b, c, color);  glyphs::rounded_rect(o, x, y, w, h, r, color);
gpu.write_atlas(r: AtlasRegion, rgba: &[u8]);                       // fill a region (sRGB RGBA rows)
```

**Portraits** (`render/portrait.rs`, `render/view/portrait.rs`, `Gpu::render_portraits`).
`PortraitId::{Hero(kind), OldMan, Foe(kind), Prince(sin)}`; `Pose::{still(), live(time, every),
talking(time, since)}`. The HUD asks as it draws: `o.portrait(id, pose, x, y, w, h, round, tint)`.
The book (`overlay.portraits`) keeps which slot holds which portrait at which size and lists the
ones due (new, live and due, or talking: every frame); the app hands that list to
`gpu.render_portraits(&view, &overlay.portraits.due)` after the HUD is laid out and before the frame
is drawn, so a portrait shows the frame it is asked for. The view poses the rig at the origin
(breathing; the old man's head and hand move while he talks) and frames head and shoulders from the
crown of its head part (wide enough for horns); the GPU draws it with the rigs' own vertex shader
and a portrait's light (`fs_portrait`: warm key, cool fill, rim, the game's ACES curve), 4x
multisampled on a clear ground, resolved and copied into the slot. The hero's live portrait is in
the health panel (made once a second), the old man's in his panel (every frame while his words type
out, once a second after) and on his guide page; both are made behind the loading screen, where
their first making (the meshes going up, the driver readying the pipeline) costs most. The HUD
builder restyles round them.

**Input as data** (`input/bindings.rs`, `input/mod.rs`, `input/rumble.rs`; built on
`ui-u0-input`). The action map: play's rebindable layer with §3's defaults on both devices, and the
menus' fixed layer (D-pad, arrows and WASD move, A / Enter / Space select, B / Esc / Menu back,
LB / RB and Q / E or Tab change tabs, Y and R the secondary). The D-pad in play turns the belt (left
and right), opens the map (up) and switches the camera (down); in menus it navigates. Triggers are
read as buttons with hysteresis (pressed at 0.35, let go at 0.25). Saved as `vh.bindings`
(`v1;Action=Control,Control;...`, forgiving of anything it does not know). `Bindings::{bind,
unbind, conflicts, reset, reset_action, shown, controls, actions}`; `Input::{last_device,
set_last_device, set_context, prompt, rumble, set_rumble}`. The device used last switches on a key,
a mouse button or the wheel, real mouse motion, or a pad button or stick past its deadzone (by
gilrs's vendor id and name: Sony is PlayStation, Microsoft is Xbox, else generic). The mouse wheel
chooses the potion; right click (held) is the shield. Debug keys are as they were. M and Tab (and
View, D-pad up) are the map: for now the debug stop list. Mute is Ctrl+M until the settings page
has Audio. Rumble (gilrs force feedback, a no-op on the web and on pads without it) on a hit
landed, a hit taken, a guard broken, a parry and a quake, scaled off / low / high (`vh.rumble`),
fed by a presentation-only queue (`Game::rumbles`) the app drains.

**Prompts** (`render/hud/glyphs.rs`). The bound control on the device used last, drawn as shapes:
Xbox's coloured letters, PlayStation's symbols, bumpers, triggers, sticks, the D-pad with its arm
lit, View and Menu, keycaps with the key's name, the mouse with its button lit, the wheel.

```rust
glyphs::draw_prompt(o, &g.input, Action::Angel, x, y, size) -> f32;       // its width
glyphs::draw_prompt_right(o, &g.input, action, right_x, y, size) -> f32;
glyphs::draw_control(o, control, device, x, y, size) -> f32;
glyphs::prompt_text(o, &g.input, "{Confirm} take   {Back} leave", x, y, &style) -> f32;
glyphs::prompt_line(o, &g.input, text, x, y, size, color, align) -> f32;  // the old text's look
glyphs::measure_prompt_text(o, &g.input, text, &style) -> f32;
glyphs::words(&g.input, "Guard with {Block}") -> String;   // names, for wrapped prose
// tokens: any action's (`input::token`), and {Move} {Look} {Nav} {NavLR}
```

Every hard-coded prompt in the HUD and its screens is now a glyph: the title's legend, the pause
line, the tour, the view choice, the character screen, the guide, the Ledger, the map, the shop
(and its words on the patron and the shield), the story, the belt's drink key, the archangel's
card, the won and fallen screens. The controls page is made from the bindings (keyboard rows and
the pad's callouts, the pad named as the one in use names it, PlayStation's symbols on its face)
and opens on the device used last. The old man's guide and the path's words name the controls by
token, so they follow the bindings and the device.

**Changed from the plan.** The SDF has no mip levels (they filled small letters' counters); small
text is supersampled in the shader instead. Glyphs are drawn as shapes in the overlay, not as
atlas images (crisp at any size, no atlas room). Portraits draw with the rigs' vertex shader and
a light of their own rather than through the deferred pipeline, whose shadows, probes and caches
are the level's (drawing a portrait through it would have thrown them away every time); slots are
256 square, enough for the 9t guide portrait up to 1440p (4K wants 512: two rows of slots, later).
The input layer resolves pad buttons in `Input::poll` (so a button held across a switch between
play and a menu presses nothing new), and `PadState` (the bots') kept its shape.

**Numbers** (this machine, the `quick` profile, Direct3D 12):
- The type's atlas: about 95 ms to make at start-up (on the renderer's own thread natively,
  alongside the shaders' compiling), 101 ms with every UI texture up on the GPU (the field is
  1024 x 2048 bytes; the RGBA atlas 16 MB of GPU memory, written only where used).
- The HUD's CPU time per frame: 0.05-0.06 ms (garden, the old man speaking; 14,142 vertices),
  0.2 ms the first frame (the type made ready).
- A portrait: about 0.25 ms each, the GPU waited for (two in 0.53 ms, median of 30); the first
  making of a level's portraits 13 ms, behind the loading screen.
- Size (`quick` profile): the Windows exe 38.76 MB to 39.42 MB (+665 KB), the web build's wasm
  35.07 MB to 35.62 MB (+554 KB); the two fonts are 390 KB of it.

**Captures.** The title, the garden (keyboard, Xbox, PlayStation), the vaults, pause, the guide,
the shop and the alchemy bench, the Ledger, the map, the controls pages (both devices), the old man
speaking, an archangel's card, the ring, at 1280x720 and 1920x1080: the text is crisp and nothing
overflows.

**Left for U1.** The HUD restyle around the new type and portraits (sizes still the old ones:
0.36-0.5 of the unit is small at 720p), the 9-slice panel art in `ICON_REGION`, the minimap in
`MAP_REGION`, the settings and rebinding pages (on the rebind API; `Bindings::save` is called by
nothing yet), the legends on the loading screens, 4K portrait slots.

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

### U1c, play's HUD, the old man's dialogue and the archangels (2026-10-07)

Built on `ui-u1c`: the look (§4), play's HUD laid out anew (§5), the old man's dialogue and the
archangels on screen and in the world (§6). The simulation is untouched: `--court`, `--pit 2 8 1`
and the sixteen ring bouts (`--bout 1..16 2 1 1`) and `--stage-three 1` print the same numbers as
U0's build, line for line.

**The look** (`render/hud/style.rs`). Dark glass panels as 9-slice art made from shapes at start-up
into `ICON_REGION` (`style::art::bake`, written by `Gpu::new`): the glass (ink at 86-92% with a teal
cast, lit a little at its top, a faint sheen inside its top edge, the corners cut), its rule apart
(white, so it is tinted: copper for the HUD, the old man's amber, a sin's colour), and a soft
shadow under every panel so it holds over the vaults' gold and the ending's white. Beside them a
soft glow, a crisp disc (large circles no longer show their sides), a halo, the archangels' wings
(feathers laid over feathers, each darker at its own edge), the whisper's smoke and a fade.

```rust
let t = style::unit(w, h);            // = hud::tile_for: 20 px at 720p, 28 at 1080p, 56 at 2160p, times the UI scale
style::set_ui_scale(1.25);             // 0.8..1.5, for the settings page (U1b); every size follows
style::safe(w, h);                     // 5% of the shorter edge
style::panel_glass(o, [x, y, w, h], t, alpha);   // shadow, glass, copper rule; panel_ruled(.., rule)
style::panel_header(o, rect, "TITLE", t, color, alpha) -> y;   style::divider(o, x, y, w, t, color);
style::bar(o, [x, y, w, h], frac, trail, color, alpha);        // track, trail, lit fill, glow edge
style::notches(..); style::pill(o, text, x, y, &style, back, edge) -> w; style::badge(..);
style::glow / disc / ring / pie / cooldown / diamond / fade_quad / vignette / band(..);
let (alpha, slide) = style::appear(age, left);  // in and out over 0.15 s, eased
style::under(o, |o| content -> bottom, |o, bottom| panel);  // a panel as tall as what is in it
style::body(t, units, c) / label(..) / name(..)  // the type's three voices, shadowed or outlined
```

The unit no longer stops at 28 px past 1080p: it grows in proportion (1440p 37, 4K 56), so a 4K
screen at 100% shows the HUD as 1080p does, not at half its size (at 1080p and below nothing
changes). `hud::panel()` (every older screen's panel: the general's banner, the rival's card, the
mastery toast, the Ledger, the shop's) now draws the glass.

**Play's HUD** (`render/hud/layout.rs`, `render/hud/play.rs`). `HudLayout::of(w, h)` gives every
corner's rectangle inside the safe area: health top left (the live portrait at 3.2 units, the name
in the display serif, lives, health with its trail, flash and the glow at its edge, stamina), the
effects under it (each an icon in a little glass, a draining bar and its seconds, then SLOWED and
GOD MODE as pills), **the minimap's square top right at `[w - safe - 7t, safe, 7t, 7t]`
(`layout::minimap_rect`: nothing is drawn there; the map builder draws in it)**, under it the
objective (the guide's first "what you must do", made short: `play::short`) and the purse (keys,
sin keys or what the fight wants, silver and runes, the realm's meters, double points), the top
centre for the boss's, a prince's, the Pit's and a general's banners (restyled: the same data, the
glass, `style::bar`), the loadout at the bottom centre (the weapon with its cooldown sweeping over
its icon, the shield, the belt with the chosen potion lit and raised; each with its control's glyph:
the attack's and the shield's on their slots' corners, the keyboard's 1-5 on the belt's slots, the
D-pad's arms at the belt's ends on a pad, the drink's before the potion's name), the archangel
bottom left and the location bottom right (the level's name with the guide's glyph). The layout's
test keeps every part inside the safe area and apart at 16:9, 16:10, 21:9 and 4:3, from 720p to
2160p, at UI scales 0.8 to 1.5 (a scale too large for the screen is held to what fits). The score
left play's HUD: it is in the pause menu's summary and on the fallen and won screens. Banners, the
blessing's and the milestones', are a band of glass across the screen; floaters, shrine names, GO!
and the hurt and BRUTAL vignettes are in the new type and soft edges.

**The old man's dialogue** (`render/hud/dialogue.rs`). Bottom centre above the loadout: a glass
panel ruled in his amber, his live portrait at 7 units (196 px at 1080p) on a warm ground in a gold
frame, his name plate in the display serif (with "& QUITMER, HIS DOG" when the line speaks of the
dog), his words in the body sans at 0.72 units (20 px at 1080p), wrapped whole at a measure of
about sixty letters (so lines never jump as they type), typed as before with a caret, the panel as
wide as the words; at its foot `{Confirm} skip` and `{Guide} his guide`. Enter (play's Confirm) now
types the line out at once, and a second press puts it away (`Game::skip_line`, presentation only:
the next line keeps its gap). The prince's whisper keeps its own frame: no face (smoke drifting in
the sin's colour where one would be), its rule, name and words in `whisper_color`, never amber or
gold. The guide page (`hud/guide.rs`) is the same glass with his portrait at 9 units, the parts in
a column beside it at 0.66 units (shrinking only if they would overflow). The portrait's framing is
closer for the old man (his face and beard fill it) and the portrait's light is warmer (a lamp's key,
a cooler fill, a stronger warm rim, a little light from above).

**The archangels** (`render/hud/angel.rs`, `render/view/angels.rs`). Bottom left, once he has met
one (a shrine's blessing or his patron; `Juice::angel_met`, presentation only): a 4-unit emblem in
the angel's colour (the wings round a light, the halo over them, on a dark disc), the charges as
diamonds round its foot, the ring filling while the patron's power comes back, a pulse and the
`{Angel}` glyph when ready, the name in Cinzel on a glass plate with "2 OF 3", READY or the seconds;
dim with NONE HELD when the blessing is spent; Uriel's rest over it. A blessing given raises a
banner across the screen for 2.5 s (`Juice::blessed`): the emblem, "GABRIEL'S BLESSING", what it
does and its control. In the world each unspent shrine stands under a pillar of its light, about
16 units tall, seen from across the level: glows turned to the camera about the upright (so it is
a column from any side), a wide soft one and a bright core, fading as it rises, breathing; gone
when spent. Its halo burns brighter. Off screen within 25 tiles, an arrow at the screen's edge (kept
clear of the corners) in the angel's colour with a small emblem and the distance, its direction
taken from the camera's own axes (sound for a shrine behind the camera).

**Portrait slots for 4K.** The portrait rows of the atlas are now 16 slots of 256 and 4 of 512
(`overlay::BIG_PORTRAIT`): a portrait larger than 256 px (the dialogue's at 1440p and up, the
guide's) takes a big slot; the portraits' targets are 512 square. The atlas keeps its 2048 square
(WebGL2's floor).

**Changed from the plan.** The old man's rig has no eyelids or jaw (his head, staff arm and free
arm are the joints), so he does not blink or move his mouth: his head nods and his hand moves while
he talks, as U0 made it; a jaw would need his model re-rigged. The skip needs a control: in play it
is Enter (Confirm); a controller has none free in play, so on a pad only the guide's glyph shows.
The icons are still the 16 px sprites (the redrawn 64 px icons of §4 are left). The effects moved
from over the belt to under the health panel; the potion's name and its drink control sit over the
belt. The minimap's square is left empty (no placeholder: the map builder's merge fills it).

**Numbers** (this machine, `quick`, Direct3D 12):
- The HUD's CPU time a frame: 0.15-0.21 ms in the garden with the old man speaking and an angel
  held (19,638 vertices; U0's HUD 0.05-0.06 ms and 14,142), 0.5 ms the first frame; Mammon's court
  0.04-0.13 ms; the guide page 0.6-0.8 ms (33,000 vertices, its words wrapped each frame).
- The panels' art made from shapes at start-up, on the renderer's thread: about 20 ms (the UI's
  textures up 118-127 ms after the type's atlas at 98-102, against U0's 6 ms between them).
- The portraits: the hero's and the old man's (talking, every frame) 0.6 ms together waiting for
  the GPU at 720p and 1080p, 0.7 ms at 4K with his at 400 px in a big slot.
- A shrine's pillar: 27 glows (108 for the garden's four), drawn with the other glows.
- The atlas is still 2048 square: no memory added.

**Captures** (720p, 1080p, and 2160p for some): the garden with the old man speaking and Gabriel
held, a blessing's banner, the whisper, the guide, the vaults mid-fight, Mammon's court with the
prince's bar, the Pit with its clock, shrines from afar with their pillars and an arrow to one out
of sight, the patron ready, pause, the ring, the shop and the Ledger in the new glass.

Tests: the layout (inside the safe area and apart at four shapes of screen and four scales; the
minimap's promised square; the unit at 720p, 1080p, 1440p and 4K and growing without a jump), the
art (its regions apart and inside `ICON_REGION`; the glass solid with its corners cut, the rule on
the edge, the wings on both sides), panels coming in and going in 0.15 s, the dialogue never looking
like a whisper or a whisper like him (amber and a face against the sin's colour and none, for every
sin; the dog only when named), the words at 18 px or more and the face at 196 px at 1080p, the
emblem's charges and the patron's power coming back and dim when none is held, a blessing given
raising its banner and the emblem staying, the shrine's pillar only while it has a blessing to give
(and gone for the angel who has given, and for all when all have), the objective made short for
every place, portraits larger than a small slot made in a big one.

Left: the 64 px icons (§4); a jaw and eyelids for the old man's rig; the colour-blind variants of
the angels' and the HUD's reds and greens (U1b's settings); the map builder draws the minimap in
`layout::minimap_rect` and the settings page calls `style::set_ui_scale`; U1a's and U1b's panels
switch to `style::panel_glass` at the merge (their `panel()` calls already draw it).

### U1a, the minimap and the full map on screen (2026-10-07)

Built on `ui-u1a`, on U0 and the map's data above. Nothing of the game reads any of it: the
harnesses print what they printed before (`--court`, `--pit 2 8 1`, `--bout 1..16 2 1 1`).

**The image on the GPU** (`render/minimap.rs`, `MapLayer`). The `MapImage` is held in the overlay
(`Overlay::map`: the HUD has the overlay mutably and the game only to read), brought up to date as
the HUD draws (`sync`, every frame the minimap or the full map shows, and behind the loading
screen, so a new level's image, 9-19 ms in the vaults, is built under the black). After the HUD
the app sends what changed (`overlay.map.upload(|r, px| gpu.write_atlas(r, px))`, in `App::render`,
the capture and the perf harness): a new level's image whole, with the rest of `MAP_REGION` cleared
(1024 x 1024 x 4 = 4 MB, once a level, so nothing of the last level shows past the new one's edge);
after that only the rectangle the fog or the ground changed, as packed rows of a part of the region
(U0's `write_atlas` takes any sub-rectangle: no new writer was needed).

**The minimap** (`render/hud/minimap.rs`). In the square play's HUD keeps for it, top right inside
the safe area (`layout::minimap_rect`: `[w - safe - 7t, safe, 7t, 7t]`), round. In U1c's look
(`style`): a soft glow of shadow, a ring of the dark glass lit a little from above with its copper
rule outside and a fainter one in,
the level through a round window centred on the hero, a soft shadow inside the rim; the camera's
view as a faint wedge from him; what the map knows as small shapes in their colours
(`draw_marker`, shared with the full map: foes red dots, a pack's leader a red dot in a gold ring,
a rival a gold diamond, a general a crown, a boss a skull; shrines diamonds in their angel's colour
with a breathing halo, grey when spent; golden and red keys, remedies, potions, gold, chests, the
gate shut or open, the way in, the hub's portals in their sin's colour (ringed once conquered), the
House's door, the Pit's doors and its platform drawn to scale and cut to the circle, the old man's
lamp in amber, the general's instruments and post, the cage); the hero's arrow on top. The north
mark (`style::badge`, a gold N) sits on the rim where north is. Out of range, the objectives (the
exit, the lamp, unspent shrines, the boss; the gate left to the lamp where both stand on the far
portal) become arrows on the rim pointing their way, the lamp's breathing (`plan`, `objective`);
nothing else does. Three zooms, 9, 15 and 26 tiles to the rim, stepped by L3 (`MinimapZoom`) and
shown as three pips on the rim (L3's glyph beside them on a controller); saved as
`vh.minimap_zoom`. North up, or turning with the camera (`Game::map_ui.turn`, saved as
`vh.minimap_turn` by `Game::set_minimap_turn`, which U1b's Settings toggle calls): the camera's
heading along the ground is up (for a camera looking straight down, its screen's up). Shown in
play, the countdown, the fall, the fanfare and the pause (under its dim); hidden on the title, in
the story, the tour, the wares, the character screen, the won, fallen and toll screens, under the
full map and behind the loading screen (`shown`).

The round window is not U0's `region_circle`: past `MAP_REGION`'s edge that reads the icons'
region or a portrait, and the window passes the edge whenever the hero is near the level's. It is
`image_polygon`: the circle's square clipped (in screen space, by the four planes of the image's
texel centres, the screen-to-image map being affine) to the image, drawn as a fan of vertices that
carry the circle mask's coordinates, through a new `Overlay::raw_triangle` (vertices as they are;
also the gradients of the rim and the shadows). The full map draws through it too, clipped to its
panel.

**The full map** (`render/hud/fullmap.rs`, `game/fullmap.rs`). The map action (View, the D-pad's
up, M or Tab) from play pauses under it and opens it; from the pause menu MAP or the map action
opens it over the menu. Over a dim: the place's name in the display type, what he must do there
(the old man's guide's first word on it, `guide_now().task`, wrapped to two lines), the explored
level large in a panel with a faint grid every eight tiles, the markers with the hub's portals
named, the north mark, off-panel objectives as arrows at its edge; a legend of what is marked here
(only the kinds known on this level, each drawn as on the map); the controls below in the glyphs of
the device used last. It opens framed on what he has explored, as close as that allows (up to
three units a tile). The left stick or the run keys pan it, a drag of the mouse moves it with the
pointer; the wheel (about the pointer), the right stick, the bumpers or Q and E zoom (1: the whole
level fits, to three units a tile); Y or R brings it back to him. Whatever is bound to the map
action puts it away (so Tab, M, View, the D-pad's up, or a rebinding; not the press that opened
it), and so do B, Esc and Menu: back to play if it came from play, else to the pause menu with MAP
highlighted. Its middle stays on the level. Input for it, added to `Input` and read as it is:
`went_down(control)` (the controls that went down this update, so the map action closes it however
it is bound), `wheel_turned()` and `sticks()`.

**The developer's map** (`game/map.rs`, `render/hud/map.rs`, unchanged in look). The pause menu's
MAP is the player's map now. The list of stops is the developer's map: **F12 opens it in every
build** (from the title, play, the pause menu, or over the full map; a fixed key like the other
testing keys, taken out of the keys the player may bind), and **DEV MAP** stands on the title and
in the pause menu in developer builds, with `--dev` on the command line, or for the rest of the
session once F12 has opened it (`Game::dev`). Decided so because the user tests with the release
build: one key from anywhere, and its buttons back after the first press. It gained two stops, the
full map of the garden and of the vaults (every new screen on the Map). On the web, F12 reaches the
page (the browser's tools want it too: a debug build or the buttons after one press).

**The HUD** (`render/hud.rs`, kept to calls into the new files): the minimap drawn after the
corners, the full map over the pause (over its summary), the image prepared under the loading
screen. Merged with U1c: the minimap in `layout::minimap_rect` (the objective and purse under it are
U1c's), the full map's panels `style::panel_glass` with `panel_header` for the legend, its type
`style::name`, `body` and `label`, the north marks `style::badge`; nothing of it near the HUD's
corner panels (its labels are the hub's portals, inside its own panel).

**Captures**: `fullmap` opens the full map, `mapzoom:Z` and `mappan:X:Y` set it; `minizoom:N` and
`miniturn` set the minimap (none saved). The capture prints the bytes sent to the atlas.

**Changed from the plan.** The round window clipped on the CPU in place of `region_circle` (above).
The image lives in the overlay rather than beside it. The keyboard has no minimap zoom by default
(L3 only, as U0's defaults have it; the action is rebindable). The minimap shows during the fall and
the fanfare too (they are play). The colour-blind palette for the foes' dots waits for U1b's
setting. The unit is U1c's (`style::unit`, with the UI scale): the minimap is 140 px at 720p, 196
at 1080p, 392 at 2160p.

**Numbers** (this laptop, the `quick` profile; tests `the_minimap_is_quick_and_sends_up_only_what_changed`,
`the_page_fits_the_screen_and_draws_quickly`):
- The minimap, every frame, in the vaults (their 1390 marks the most of any level; the image's
  sync, the markers listed and drawn, the frame): 0.021 ms of CPU, 1,194 vertices.
- The full map over the vaults at 1080p: 0.064 ms, 4,575 vertices.
- Sent to the GPU: a new level's image once, 4,194,304 bytes (the whole region, cleared round it);
  then a step that reveals ground sends its patch, 106 KB on average and 118 KB at most on a walk in
  the garden (the new tiles' rectangle and two round it; 3 of 12 steps revealed any); nothing while
  nothing changes.
- The image's build for a new level (behind the loading screen): 9-19 ms for the vaults.
- The harnesses (`--court`, `--pit 2 8 1`, `--bout 1..16 2 1 1`) print the same before and after,
  but for their own wall-clock timings.

Tests: the minimap keeps him in its middle (north up: north above him, east to his right) and turns
with the camera when set (the way the camera looks is up, east and south tried; screen and tile
agree both ways); only objectives out of range go to the rim as arrows (the lamp, the boss, an
unspent shrine, the gate; not a foe, a key, a chest or a spent shrine; never what is unknown; the
gate left to the lamp on the far portal); it shows in play, the countdown and the pause and hides
on the title, in the story, the wares, the character screen, the won, fallen and toll screens,
under the full map and behind the loading card; the map action opens and closes the full map from
play and from the pause (and Esc, Back, the pause menu's MAP, a rebinding); it pans, zooms and comes
back to him, and keeps to the level; L3 steps the minimap's zoom and its turning is saved; F12 opens
the developer's map from the title, play, the pause and over the full map, its DEV MAP buttons
appear after, and its full-map stop opens the full map; the expert bot through the vaults with the
minimap drawn, sent up, zoomed and turned plays the same game as without; the page fits 720p,
1080p and 2160p. The menus' and the D-pad's and the keyboard's tests updated for the player's map.

Left: the Settings toggle for turning (U1b, on `set_minimap_turn`); the colour-blind palette; a key for the
minimap's zoom if wanted.
