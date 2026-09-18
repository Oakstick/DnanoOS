# Void Harvest

Descend into ultra-deep-sea trench networks as a bio-scavenger drone. Harvest
mineral nodules while evading blind, sound-sensitive predators. When cornered,
vent a chem-charge: predators enter a bioluminescent frenzy and become edible
for a brief window.

Core loop: grid navigation and resource collection under asynchronous pursuit,
punctuated by a temporary state-inversion predator flip.

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
| WASD / arrow keys / swipe | Move (turns are buffered until the next junction) |
| Enter / Space / tap | Start, continue |
| Esc | Pause |
| M | Mute |
| F1 | Debug overlay: predator targets, AI modes, FPS |

## Predators

Each predator has a different sensory hook in chase mode.

| Name | Kind | Behavior |
| --- | --- | --- |
| Gulper | hunter | Homes on the last place the drone made noise. Stand still for 1.5 s and it loses you. |
| Lantern | ambusher | Targets four tiles ahead of the drone. |
| Siphon | flanker | Mirrors the Gulper around a point ahead of the drone. |
| Barbel | lurker | Chases when far, retreats to its corner when close. |

Predators alternate between scatter and chase on a global schedule and reverse
direction whenever the schedule flips. During a frenzy they move at half speed
and wander randomly. Eating them in one window pays 200, 400, 800, 1600.

## Project layout

```
src/
  main.ts              Entry: wires Game to GameLoop
  engine/
    GameLoop.ts        Fixed-step update, variable render, tab-visibility pause
    Input.ts           Keyboard + swipe with buffered direction
    Audio.ts           Procedural Web Audio (sfx, ambient threat pulse, frenzy loop)
    Rng.ts             Seedable PRNG for reproducible predator behavior
  world/
    types.ts           Tile enum, directions, vectors
    Grid.ts            Tile grid with wrap tunnels, walkability, consumption
    Level.ts           ASCII level parser
    levels.ts          Authored trench layouts and difficulty ramp
  entities/
    Mover.ts           Grid-locked movement shared by drone and predators
    Drone.ts           Player; tracks the "heard" position used by hunters
    Predator.ts        AI modes: nest, leaving, scatter, chase, frenzied, eaten
  systems/
    Frenzy.ts          Inversion timer and consume-chain scoring
    Score.ts           Score, lives, extra lives, persisted high score
    Particles.ts       Small particle pool
  render/
    Renderer.ts        Canvas 2D: tiles, entities, HUD, overlays, vignette
  game/
    Game.ts            Screen state machine and the per-tick simulation
  debug/
    DebugOverlay.ts    F1 overlay
tests/                 Vitest unit tests for pure logic
```

## Authoring a level

Levels are ASCII rows in `src/world/levels.ts`. Every row must be the same width.

```
#  rock          .  nodule        o  chem-charge
   open artery   P  drone spawn   G  predator nest exit
N  nest interior -  tunnel (wraps horizontally)
```

The tests check that every nodule and charge is reachable from the spawn.

## Next steps

- Sprite art and sampled audio to replace the procedural placeholders
- Cornering tolerance (pre-turn) for snappier drone control
- Bonus nodule spawner and additional trench layouts
- Gamepad support
