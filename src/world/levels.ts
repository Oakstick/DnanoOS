import { LevelData } from './Level';

/** 27 x 23 trench. Center box (rows 8-14) is the predator nest region. */
const TRENCH_ROWS: string[] = [
  '###########################',
  '#............#............#',
  '#o###.#####..#..#####.###o#',
  '#.###.#####..#..#####.###.#',
  '#.........................#',
  '#.###.#.###########.#.###.#',
  '#.....#......#......#.....#',
  '#####.#####..#..#####.#####',
  '#####.#             #.#####',
  '#####.#  ####G####  #.#####',
  '-.....   #NNNNNNN#   .....-',
  '#####.#  #NNNNNNN#  #.#####',
  '#####.#  #########  #.#####',
  '#####.#             #.#####',
  '#####.# ########### #.#####',
  '#............#............#',
  '#.###.#####..#..#####.###.#',
  '#o..#........P........#..o#',
  '###.#.#.###########.#.#.###',
  '#.....#......#......#.....#',
  '#.#########..#..#########.#',
  '#.........................#',
  '###########################',
];

export const LEVELS: LevelData[] = [
  {
    name: 'Trench I: Coral Artery',
    rows: TRENCH_ROWS,
    droneSpeed: 6.5,
    predatorSpeed: 5.4,
    frenzySeconds: 7,
  },
  {
    name: 'Trench II: Pressure Vents',
    rows: TRENCH_ROWS,
    droneSpeed: 7,
    predatorSpeed: 6.3,
    frenzySeconds: 5.5,
  },
  {
    name: 'Trench III: The Blind Deep',
    rows: TRENCH_ROWS,
    droneSpeed: 7.5,
    predatorSpeed: 7.3,
    frenzySeconds: 4,
  },
];

/** Beyond the last authored level, speeds keep ramping. */
export function levelFor(index: number): LevelData {
  if (index < LEVELS.length) return LEVELS[index];
  const base = LEVELS[LEVELS.length - 1];
  const extra = index - LEVELS.length + 1;
  return {
    ...base,
    name: `Trench ${index + 1}: Abyssal Loop`,
    predatorSpeed: Math.min(base.predatorSpeed + extra * 0.3, 9),
    droneSpeed: Math.min(base.droneSpeed + extra * 0.2, 9),
    frenzySeconds: Math.max(base.frenzySeconds - extra * 0.5, 2),
  };
}
