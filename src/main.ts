import { GameLoop } from './engine/GameLoop';
import { Game } from './game/Game';

const canvas = document.getElementById('game') as HTMLCanvasElement | null;
const canvas3d = document.getElementById('game3d') as HTMLCanvasElement | null;
if (!canvas || !canvas3d) throw new Error('Missing #game or #game3d canvas');

const game = new Game(canvas, canvas3d);
const loop = new GameLoop(
  (dt) => game.update(dt),
  () => game.render(),
  120,
);
loop.start();

// Handy for poking at state from the browser console.
declare global {
  interface Window {
    voidHarvest?: Game;
  }
}
window.voidHarvest = game;
