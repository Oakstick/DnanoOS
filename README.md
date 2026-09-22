# Void Harvest

A voxel dungeon crawler through a moonlit forest labyrinth with verticality.
Popolon walks single-file forest paths, one tile wide, walled by undergrowth
and trees, to gather twenty-five gold nuggets while a skeleton, two bats, and a
slime hunt him. The forest rises in levels: ramps join some floors, ledges must
be jumped, trenches across the road must be cleared, and bats fly over all of it. Green crosses left along the paths
restore fifty health. Viewed over Popolon's shoulder from a camera the mouse
aims: click the view to capture the pointer, and he runs wherever it looks
(C switches to a top-down follow view). The dungeon is 27 by 23 tiles, composed from four mirrored quadrants.

Core loop: grid navigation and resource collection under asynchronous pursuit,
punctuated by a temporary state-inversion (stun) that flips hunter and hunted.

## Run

```bash
npm install
npm run dev
```

Open the printed URL (default http://localhost:5173).

| Script | Purpose |
| --- | --- |
| `npm run dev` | Dev server with hot reload |
| `npm run build` | Type-check and bundle to `dist/` |
| `npm run preview` | Serve the production build |
| `npm run package` | Build and zip `dist/` into `release/void-harvest-<version>.zip` for sharing |
| `npm run vox:import -- <file.vox>` | Convert a MagicaVoxel file into a `VoxelModel` literal |
| `npm run vox:export -- <MODEL>` | Write a model from `voxels.ts` out to `art/<model>.vox` |
| `npm test` | Run unit tests once |
| `npm run test:watch` | Tests in watch mode |

## Share it

The game is a static site: one HTML file and one JavaScript bundle, about
160 KB zipped, no server code. `npm run package` builds it and writes
`release/void-harvest-<version>.zip`. Friends need a link, not the zip:
opening `index.html` straight from disk does not work, because browsers
refuse to load module scripts over `file://`. Any static host works:

- **itch.io**: create a project, kind "HTML", upload the zip, tick "This
  file will be played in the browser", set the viewport to fullscreen or at
  least 1280 x 720. Free, and made for sharing browser games.
- **Netlify Drop** (https://app.netlify.com/drop): drag the `dist/` folder
  onto the page and you get a public URL in seconds.
- **GitHub Pages**: push the repo, then publish `dist/` with
  `npx gh-pages -d dist` or a Pages workflow.
- **On your own network**: `npm run preview -- --host` serves the build on
  your LAN address for people in the same room.

The build uses relative asset paths (`base: './'`), so it runs from any
folder or subpath. Best score is kept in the browser's localStorage.

## Controls

| Input | Action |
| --- | --- |
| W / up (hold) | Run the way the camera is looking; release to rest on the next tile |
| A / D, left / right | Step to either side of the way the camera looks |
| S / down | Reverse |
| Touch drag | Same as the keys, relative to facing |
| Mouse | Aims the camera once you click the view to capture the pointer; Popolon takes the corridor it points down |
| C | Switch between the chase camera and the top-down follow view (top-down uses grid-absolute keys) |
| Space | Jump two tiles forward: over trenches and enemies, up one level; one tile or a hop in place when two is blocked |
| E / tap | Attack: swing the sword, or fire the shrine weapon that has replaced it; facing a shrine, cycle it instead |
| B / L / Shift | Drop a bomb on the current tile (stock of 3) |
| Enter | Start, continue |
| Esc | Pause |
| M | Mute |
| F1 | Frame counter in the HUD |

## Verticality

Every tile has a floor level, 0 to 2. Quadrants rise clockwise from the
hollow: top-left at ground, top-right and bottom-left one level up, bottom-right
two. Walking down a drop is always allowed; walking up needs a ramp, which
joins two levels at a few gates and at the tunnel mouths. Every other gate is a
ledge: stand on the lower side and jump. Six pits, one tile deep, are cut
across straight corridors; walkers cannot cross them, jumpers and flyers can.
A pit is only kept where it leaves the whole dungeon connected, so no cut can
seal off a wing or trap the enemies in their hollow.

A jump takes 0.42 s and lands two tiles ahead if that floor is at most one level
up and the tile between is not a wall, otherwise one tile ahead, otherwise it is
a hop in place. While airborne Popolon cannot be touched, picks nothing up, and
ignores slime. Landing makes a footstep the skeleton can hear.

Ground enemies (skeleton, slime) obey the same rules and reach the upper levels
through the ramps. Bats fly and ignore pits and ledges. A bombed wall becomes
floor at the bomb's level.

## Combat

Popolon has 100 health. Touching a hunting enemy costs 25 and grants one
second of immunity with a blink. At zero health he falls and a life is lost.
Health refills on respawn. The sword reaches about
one tile ahead and half a tile to either side, with a short cooldown. Slaying
an enemy sends it back to the hollow to reform, except the skeleton, which
collapses in place and reassembles three seconds later. A normal kill pays
100; kills during a stun pay 200, 400, 800, 1600.

## Shrines

Two shrines stand in each dungeon, marked `W` and `I` in the level. A shrine
shows one gift at a time and cycles to the next every 1.6 seconds; a sword
strike cycles it at once so you can pick. Step onto the shrine to take what it
shows. A taken shrine goes dark and returns 25 seconds later.

**Weapon shrine** (six weapons). Popolon's default weapon is the sword. Taking
a shrine weapon replaces the sword until its ammo is spent, then the sword
comes back. The single attack button fires whatever is held. Dying loses the
weapon along with every boon:

| Weapon | Ammo | Behavior |
| --- | --- | --- |
| Arrow | 7 | Fast single shot, kills the first enemy it touches |
| Double Arrow | 7 | Two parallel arrows |
| Tri-Shot | 6 | One arrow forward and one to each side |
| Boomerang | 5 | Flies 5 tiles, hits everything both ways, comes back; catching it refunds the shot |
| Fireball | 5 | Slow, glowing, passes through every enemy in its path |
| Crescent | 6 | Short-range wide blade that pierces |

Shots pass through the wrap tunnels and over the hollow floor and break on
walls. Kills score like the sword, including the stun chain.

**Item shrine** (three boons):

| Boon | Effect |
| --- | --- |
| Shield | Absorbs the next 3 hits |
| Invincibility | 8 seconds of no damage, Popolon glows gold |
| Speed | 15 seconds at 1.4x speed |

Boons are lost with a life.

## Lives, nuggets, and potions

Popolon has three lives, shown as hearts in the HUD, and can never hold more
than three. Each dungeon hides exactly twenty-five gold nuggets worth 50 gold
apiece; every one plays a reward chime that climbs as you collect, the HUD
counts them, and taking the last one wins the game with a victory screen.
The longer you take, the more enemies crawl out of the hollow: the first
reinforcement after 20 seconds, then one every 25 seconds, up to twelve foes.

The HUD carries his portrait (the front view of the voxel model),
his health bar, the weapon he holds with its ammo, and any active potion
effects with their timers. The portrait goes ghostly while invisible, flashes
gold while invincible, and gains a blue frame while shielded.

Five potion bottles lie in each dungeon, placed away from nuggets and one
another. Walk over one to drink it. They do not return within a dungeon.

| Bottle | Effect |
| --- | --- |
| Red | Extra life, or 500 gold if you already have three |
| Blue | Shield: absorbs the next 3 hits |
| Purple | Invisibility for 10 seconds: enemies lose you, cannot hurt you, and the skeleton hears no footsteps |

## Bombs

Popolon carries three bombs, refilled on each new life and each new dungeon.
A dropped bomb sits on its tile and flashes faster for three seconds, then
detonates like TNT: it blows out unprotected forest within 3.2 tiles, enough to
punch through a three-tile wall (the outer border and the hollow walls survive),
slays every enemy within 3 tiles, deals 50 damage to Popolon within 1.5 tiles
and 25 within 3, chain-ignites any other bomb in range with a short random delay, and
throws leaves and clods of earth. Blasting walls permanently reshapes the dungeon for the
rest of the level, which opens shortcuts and escape routes.

## Enemies

Each enemy has a different sense in chase mode.

| Name | Kind | Behavior |
| --- | --- | --- |
| Skeleton | skeleton | Eyeless but sharp-eared: homes on the last footsteps it heard. Stand still 1.5 s and it loses you. Slaying it only scatters its bones: it lies harmless for 3 s, then reassembles on the spot. |
| Bat | bat | Fast. Swoops at the tile four ahead of Popolon. |
| Vampire Bat | vampire | Flanks by mirroring the skeleton around a point ahead of Popolon. |
| Slime | slime | Slow. Oozes toward Popolon when far, slinks to its corner when close. Leaves a slime trail for 8 s; standing in it halves your speed. |

Enemies alternate between scatter and chase on a global schedule and reverse
direction when the schedule flips. While stunned they wander at half speed.

## Project layout

```
src/
  main.ts              Entry: wires Game to GameLoop
  engine/
    GameLoop.ts        Fixed-step update, variable render, tab-visibility pause
    Input.ts           Keyboard hold-to-move and touch drag
    Audio.ts           Procedural Web Audio (sfx, ambient threat pulse, stun loop)
    Rng.ts             Seedable PRNG for reproducible enemy behavior
  world/
    types.ts           Tile enum, directions, vectors
    Grid.ts            Tile grid with elevation, ramps, pits, jump landings, wrap tunnels
    Level.ts           ASCII level parser
    levels.ts          Maze composed from a mirrored quadrant with heights, pits, nuggets, potions, then upscaled x3
  entities/
    Mover.ts           Grid-locked movement shared by hero and enemies
    Hero.ts            Popolon: health, shield, invincibility, speed, weapon and ammo
    Enemy.ts           AI modes: nest, leaving, scatter, chase, stunned, slain, scattered
    Weapons.ts         The six weapons and three boons
    Projectile.ts      Shots: straight flight, piercing, boomerang return
    Diamond.ts         Cycling shrine
    Bomb.ts            TNT-style bomb
  systems/
    Combat.ts          Sword and projectile hit detection
    Pathing.ts         Reachability with jumps, used by tests
    Puddles.ts         Slime trail marks that slow the hero
    Frenzy.ts          Stun timer and kill-chain scoring
    Score.ts           Score, lives, extra lives, persisted best
  render/
    Renderer.ts        Canvas 2D overlay: HUD portrait panel, labels, start and win screens
  render3d/
    voxels.ts          Voxel model format, instanced cube builder, the model set
    Renderer3D.ts      Three.js: forest maze with elevation, trees, lanterns, fireflies, third-person camera
  game/
    Game.ts            Screen state machine and the per-tick simulation
tests/                 Vitest unit tests for pure logic
tools/
  gen-image.mjs        Gemini (Nano Banana) image generation for concept art and textures
art/                   Generated 2D art
```

## Voxel models

Models are tiny text grids in `src/render3d/voxels.ts`: a list of layers from
the bottom up, each layer a list of rows from back to front, each row a string
of palette keys. They are baked into one instanced cube mesh per model. Models
face +z (grid down) and are rotated to the movement direction. To add or
replace a model, edit the grid or export voxels from MagicaVoxel or Blender
into the same format.

Characters are rigged by cutting the model into named parts with voxel boxes
and a pivot each (`POPOLON_RIG`, `SKELETON_RIG`, `BAT_RIG`). Each part becomes
a group hinged at its pivot, so the renderer animates by rotating groups:
Popolon and the skeleton swing legs and arms in a stride cycle driven by their
actual movement, the sword arm performs the slash, bats flap their wings and
dive forward while moving, and the slime squashes, stretches, and hops. Every
character eases into a new heading over a fraction of a second rather than
snapping, and the third-person camera swings round more slowly still. The
tests check that a rig claims every voxel and that no part is empty.

## Authoring a level

Levels are ASCII rows in `src/world/levels.ts`. Every row must be the same width.

```
#  rock          .  gold nugget     o  green cross (heals 50)
   open floor    P  hero spawn     G  hollow exit
N  hollow floor   -  tunnel (wraps horizontally)
W  weapon shrine I  item shrine
r  life potion   b  shield potion    v  invisibility potion
```

The tests check that every nugget and cross is reachable from the spawn.

## MagicaVoxel

Models can be authored in MagicaVoxel instead of by hand. `tools/vox.mjs`
converts in both directions, so a character can start in the editor or start
in code and move to the editor later.

```bash
npm run vox:import -- path/to/model.vox --name WIZARD   # prints a VoxelModel
npm run vox:export -- POPOLON                           # writes art/popolon.vox
node tools/vox.mjs info path/to/model.vox               # sizes and voxel counts
```

`import` prints a `VoxelModel` literal on stdout, ready to paste into
`src/render3d/voxels.ts`; size and colour counts go to stderr so you can
redirect the model on its own. It crops away empty space, orders the palette
by how often each colour is used, and assigns the keys. The text format holds
59 colours, so reduce the palette in MagicaVoxel first if a model exceeds it.

`export` writes a `.vox` you can open in the editor. The four characters are
checked in under `art/` and are regenerated by rerunning the command.

Axes differ between the two worlds: MagicaVoxel is z-up with y running away
from its default camera, this format is y-up with rows running back to front.
The converter remaps them so a model faces the camera in the editor and faces
+z in the game. Pass `--front ymax` if an imported model comes in backwards.
A rigged character also needs its `PartDef` boxes updated, because those are
voxel coordinates into the model grid and a new model will not match the old
one's limbs.

## Art pipeline

`tools/gen-image.mjs` generates images with Gemini through the REST API. It
reads `GEMINI_API_KEY` from the environment or `~/.gemini/.env` and never
prints it. Image models require a billing-enabled Google project.

```bash
node tools/gen-image.mjs --out art/wall-albedo.png --aspect 1:1 "seamless tileable dungeon stone brick wall, voxel game texture"
```

## Next steps

- Richer voxel models (the MagicaVoxel bridge is in; Blender is still manual)
- Enemy hit points and attack telegraphs for the skeleton and slime
- Health pickups and treasure chests as bonus items
- Hit sparks and blast debris are the only effects; a fuller particle pass would help the win and death moments
- Rendering: static geometry is instanced per 32-tile chunk so the frustum culls what is behind the camera; tree density and chunk size at the top of `Renderer3D.ts` are the main performance knobs
- More dungeon layouts, gamepad support
