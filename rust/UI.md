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
