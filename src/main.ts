import { GameLoop } from './engine/GameLoop';
import { Game } from './game/Game';

const canvas = document.getElementById('game') as HTMLCanvasElement | null;
if (!canvas) throw new Error('Missing #game canvas');

const game = new Game(canvas);
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
