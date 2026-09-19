# Void Harvest

A voxel dungeon crawler through a moonlit forest labyrinth with verticality.
Popolon walks wide forest roads, three characters across, walled by undergrowth
and trees, to gather twenty-five gold nuggets while a skeleton, two bats, and a
slime hunt him. The forest rises in levels: ramps join some floors, ledges must
be jumped, trenches across the road must be cleared, and bats fly over all of it. Power crystals stun every foe for a few seconds; cut
them down while they glow for chain gold. Viewed from a third-person camera
that rides behind Popolon and turns with him (C switches to a top-down
follow view). The dungeon is 53 by 45 tiles, composed from four mirrored quadrants.

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
| `npm test` | Run unit tests once |
| `npm run test:watch` | Tests in watch mode |

## Controls

| Input | Action |
| --- | --- |
| W / up (hold) | Walk forward in the direction Popolon faces; release to rest on the next tile |
| A / D, left / right | Turn left or right (taken at the next junction, or at once when stopped) |
| S / down | Reverse |
| Touch drag | Same as the keys, relative to facing |
| C | Switch between the third-person camera and the top-down follow view (top-down uses grid-absolute keys) |
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
ledge: stand on the lower side and jump. Fourteen trenches, one tile deep and
spanning the full road width, are cut across straight roads; walkers cannot
cross them, jumpers and flyers can.

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
#  rock          .  gold nugget     o  power crystal
   open floor    P  hero spawn     G  hollow exit
N  hollow floor   -  tunnel (wraps horizontally)
W  weapon shrine I  item shrine
r  life potion   b  shield potion    v  invisibility potion
```

The tests check that every nugget and crystal is reachable from the spawn.

## Art pipeline

`tools/gen-image.mjs` generates images with Gemini through the REST API. It
reads `GEMINI_API_KEY` from the environment or `~/.gemini/.env` and never
prints it. Image models require a billing-enabled Google project.

```bash
node tools/gen-image.mjs --out art/wall-albedo.png --aspect 1:1 "seamless tileable dungeon stone brick wall, voxel game texture"
```

## Next steps

- Richer voxel models (imported from MagicaVoxel or Blender)
- Enemy hit points and attack telegraphs for the skeleton and slime
- Health pickups and treasure chests as bonus items
- Hit sparks and blast debris are the only effects; a fuller particle pass would help the win and death moments
- Rendering: static geometry is instanced per 32-tile chunk so the frustum culls what is behind the camera; tree density and chunk size at the top of `Renderer3D.ts` are the main performance knobs
- More dungeon layouts, gamepad support
