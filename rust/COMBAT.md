# Combat Systems Specification

The combat loop, defined in frames, formulas and state machines. Code is the
source of truth: every number below lives in `src/logic/combat/` and is pinned by
tests (`cargo test combat`). Change the table, then change this document.

Phase 1 runs in **the Sparring Ring** in the gym (west of the arrival platform).
Everywhere else the game still plays its original contact rules, until the lab
proves the loop (see the roadmap at the end).

---

## 0. Time base

| Quantity | Value |
|---|---|
| Simulation step | 120 Hz (`app::STEP`) |
| Combat frame | **60 Hz, 1 f = 16.67 ms** (`combat::FPS`) |
| Clock | `FrameClock` turns sim steps into whole frames (2 steps = 1 f) |
| Hitstop | global freeze, frames / 60 s (`Game::hitstop`) |

All frame data is in 60 Hz frames. A move is **Startup → Active → Recovery**:
it cannot hurt during startup, it hurts on active frames, and in recovery it is
committed and open to a punish.

Design limits the table must satisfy (each is a test in `combat::frames`):

* **Reaction floor**: every foe attack's startup ≥ 18 f (300 ms). Human visual
  reaction is ~250 ms. Below that, an attack can't be read; it can only be memorized.
* **Punishable**: every foe attack's recovery ≥ the hero's L1 startup + active (10 f).
  Every whiffed attack can be punished.
* **Parry by reaction**: every foe melee startup ≥ reaction (15 f) + guard raise (2 f) = 17 f.
  If the hero presses guard 15 f after the tell starts, the hit lands inside the parry window.
  The Bite (18 f) sits one frame above that floor: it's the hardest read in the game.

---

## 1. Frame data

### 1.1 Hero: the Training Sword (one chain, three hits)

This is a fresh Ragnar: his first blade and shield, no path. Every other kit is this table with
different numbers (§8).

| Move | Startup | Active | Recovery | Total | Dmg | Poise | Hitstun | Reach | Arc (±) | Cancel (rec. f) | Chain | Hitstop |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| L1 | 6 (100 ms) | 4 | 14 | 24 (400 ms) | 10 | 20 | 14 | 1.5 | 60° | 4 | → L2 | 3 |
| L2 | 5 (83 ms) | 4 | 16 | 25 (417 ms) | 11 | 20 | 14 | 1.5 | 60° | 4 | → L3 | 3 |
| L3 | 9 (150 ms) | 5 | 22 | 36 (600 ms) | 16 | 45 | 24 | 1.7 | 75° | 10 | none | 6 |

* **Chain timing** (attack pressed early, buffered): L1 begins f0, L2 begins f14,
  L3 begins f27. Hits land on f6, f19, f36. Full string 63 f (1.05 s), 37 base damage
  → **35.2 DPS** baseline, 85 poise damage.
* **Chain grace**: 12 f after a move ends, an attack continues the string; later, it restarts at L1.
* **Stamina**: 16 per swing (100 max, regen 30/s after 0.45 s rest), so one full chain costs 48.
  Two chains back to back nearly empty the bar. Stamina is what limits a stunlock.

### 1.2 Hero: defence

| Action | Frames | Rule |
|---|---|---|
| Guard (LB / Shift) | raise 2 f | Blocks hits from the front 180°. Walk speed × 0.55. Can't attack while raised. |
| **Parry** | guard f2-f9 (8 f, 133 ms) | A hit landing in the window: no damage, no stamina. Attacker → *Deflected* **60 f** (longer than any recovery, so a parry always opens more than a dodge) and loses 50% of max poise. A hex bolt is **reflected** (×1.5 damage to foes). |
| Block | after f9 | Stamina − **0.7** × damage (the training buckler's 30% stability). Hero → *Blockstun* 6 + dmg/3 f (he walks on at his guard's pace). |
| Guard break | block with stamina < cost | 50% damage through, hero → *GuardBroken* 48 f, stamina → 0. He walks on at his whole pace. |
| Evade (Space / A) | 25 f airborne + 6 f landing | The leap. **Airborne = i-frames** (f0-f24), then 6 f landing recovery. Cancels any startup frame, or recovery from its cancel frame. |
| Hit taken | hitstun 10 + dmg/2 f | Then **30 f mercy** i-frames, so two attacks can't stack into one combo. The formula of §2 applies to him too: a foe that catches his recovery deals ×1.25, a broken guard ×1.5. |
| No hit-lock | any reel | Hitstun, blockstun and a broken guard keep him from swinging and guarding for their frames, and nothing else: he walks at his whole pace through them (`Fighter::stride`). Only his own swing plants him. No foe slows him but the bog slime's trail. |
| Soft lock | at Began and the first active frame | The swing turns onto the nearest foe within its reach + 1 tile and 46° of his facing, and he turns with it. Four keys and a stick are coarse. |

### 1.3 Foes

| Move | Owner | Startup | Active | Recovery | Total | Dmg | Reach | Arc (±) | Track | Armor |
|---|---|---|---|---|---|---|---|---|---|---|
| Slam | Brute | 42 (700 ms) | 6 | 48 (800 ms) | 96 | 30 | 1.8 | 35° | 30 f | 35 |
| Sweep | Brute | 30 (500 ms) | 8 | 36 | 74 | 20 | 1.6 | 110° | 16 f | 35 |
| Bite | Skirmisher | 18 (300 ms) | 6 | 24 | 48 | 20 | 1.1 (from ≤ 1.3) | 70° | 12 f | 0 |
| Hex Bolt | Hexer | 36 (600 ms) | 1 | 30 | 67 | 15 | bolt 7 t/s (time-locked), 90 f | — | 30 f | 0 |

* **Track**: for the first *track* startup frames the attacker turns toward the hero.
  After that its aim is **committed**: the last 12 f of a Slam and the last 6 f of a
  Bite are locked, so a late sidestep beats them.
* **Nothing travels.** No foe's move carries it across the floor, and nothing of it stretches
  out to him and snaps back. A body foe bites from beside him (`frames::BITE_RANGE`, 1.3 tiles
  centre to centre) and stays where it bit; only a distance foe strikes from afar, with a bolt.
* **Telegraph**: during startup the hit zone is drawn on the floor and fills as the
  startup runs. In the last 8 f it flashes (commit). In recovery the foe dims: punish now.

---

## 2. Formulas

```
power      P = (1 + 0.15·(tier − 1)) · (0.4 + 0.6·edge)          blade tier, sharpness 0..1
state      S = 1.5  if target Staggered or Deflected            (vulnerable)
               1.25 if target in Recovery                        (whiff punish)
               1.0  otherwise
proration  C(n) = max(0.6, 1 − 0.08·(n − 1))                     n = hit index in the combo
damage     D = round(base · P · S · C(n))
poise dmg  Π = base_poise · P

flinch     if Π ≥ armor(target)          armor = move.armor in its startup/active,
                                          0 in recovery or reeling, else archetype armor
hitstun    H(n) = max(4, round(H₀ · 0.8^(n − 1)))                frames; H₀ from the move
break      poise −= Π; if poise ≤ 0 and resolve = 0 → Staggered(stagger frames)
           at the end of the stagger: poise = max, resolve = R frames (cannot break again)
regen      60 f after the last hit, poise += max / 90 per frame  (empty → full in 1.5 s)
combo      n resets once the target has spent 30 f out of hitstun and stagger
```

Why these shapes:

* **Hitstun decay** (0.8ⁿ) is what stops an infinite. Against an unarmoured foe,
  L1 → L2 is a true combo (L2 lands f19, L1's 14 f hitstun ends f20). By the second chain
  the n=4 hit gives only 7 f of hitstun, so the foe acts again between swings.
* **Resolve** stops a stagger loop: a broken foe can't be broken again for R frames.
* **Proration** keeps long strings from beating the poise game: damage per hit falls to 60%.
* **Armor** gives the Brute its identity: light hits (20 poise) never interrupt it. Only L3 (45)
  or a poise break does.

### 2.1 Worked balance (P = 1)

| Archetype | HP | Poise | What kills it | Expected TTK |
|---|---|---|---|---|
| Brute | 200 | 100 | Slam recovery 48 f fits L1+L2+L3 if the punish starts by f12; a parried slam opens 60 f at ×1.5. 85 poise per chain, so the 2nd punish breaks it; 90 f stagger at ×1.5 fits one full chain (≈ 52). | 4 punishes, **10-12 s** (measured: 11.6 s) |
| Skirmisher | 80 | 25 | Any hit flinches it; L2 breaks it (40 ≥ 25); L3 lands ×1.5. | 2 chains, **3-4 s** once caught |
| Hexer | 55 | 20 | One chain + L1, or a parried bolt (22) and a chain. The cost is reaching her. | **2-3 s** in reach |

Foe pressure (if every attack landed): Brute 30 per ~140 f ≈ **13 DPS**, Skirmisher 20 per ~136 f
≈ **9 DPS**, Hexer 15 per ~127 f ≈ **7 DPS**. With 3 threat points, the worst case is
Brute + one other ≈ **21 DPS** against 100 HP: about 5 s of zero-skill play.

---

## 3. The fighter state machine

One struct serves the hero and every foe: `combat::fsm::Fighter`.

```rust
pub enum State {
    Neutral,                                        // free: walk, wait, act
    Attack { mv: MoveId, frame: u16 },              // phase = mv.data().phase(frame)
    Guard { frame: u16 },                           // parry while PARRY_FROM <= frame <= PARRY_TO
    Evade { frame: u16 },                           // the leap + landing
    Reel { kind: Reel, left: u16, total: u16 },     // Hitstun | Blockstun | Staggered | Deflected | GuardBroken
    Dead,
}
```

| From | Input / event | To | Condition |
|---|---|---|---|
| Neutral | Attack (buffered) | Attack{L1 or chain, 0} | stamina ≥ cost; chain if within 12 f grace |
| Neutral | Guard held | Guard{0} | |
| Neutral | Evade (buffered) | Evade{0} | the leap took off |
| Attack: Startup | Evade | Evade{0} | **startup cancel** (feint out) |
| Attack: Active | anything | — | **no cancels**: commitment |
| Attack: Recovery ≥ cancel | Attack | Attack{chain, 0} | chain exists |
| Attack: Recovery ≥ cancel | Evade / Guard | Evade{0} / Guard{0} | |
| Attack: frame = total | — | Neutral | starts the 12 f chain grace |
| Guard | release | Neutral | |
| Evade: frame = 31 | — | Neutral | an attack buffered in landing fires at f31 |
| any but Dead | hit, Π ≥ armor | Reel{Hitstun, H(n)} | interrupts the move; the token is revoked |
| any but Dead | poise ≤ 0, resolve 0 | Reel{Staggered} | |
| Attack (foe) | parried | Reel{Deflected, 40} | |
| Reel | left = 0 | Neutral | Staggered refills poise and starts resolve |
| any | hp ≤ 0 | Dead | |

**Input buffer**: one slot, the latest press, held **8 f** (133 ms). Each frame the state
tries to consume it, so a press made early in L1 comes out as L2 on L2's first legal
frame, and a press during hitstun comes out the frame the hitstun ends.

---

## 4. Spatial and temporal architecture: the threat manager

`combat::threat::ThreatManager` decides **who may attack and when**. Foes without a token
hold position. That gives the fight a readable cadence instead of a pile-on.

### 4.1 Tokens

| Parameter | Value |
|---|---|
| Capacity | **3 threat points** |
| Cost | Brute 2, Skirmisher 1, Hexer 1 |
| Cadence | ≥ **30 f** between grants |
| Reaction window | the projected **active frames of any two attacks ≥ 18 f apart** |
| Timeout | a token not spent within `max(150, eta + 60)` f returns |
| Cooldown | after its attack, or when its token is revoked, a foe waits 45-90 f before asking again |

Grant score for each waiting foe:

```
score = 1.0 · clamp(1 − d/10, 0, 1)        closeness
      + 0.8 · (1 + aim·dir)/2               in front of the hero (readable)
      + 0.5 · min(wait/240, 1)              patience (flanks happen, eventually)
```

The **commit gate** backs the projection up. A foe holding a token may only begin its move if
the frame it **lands** on is ≥ 24 f (18 f + 6 f of slack) from every other committed landing.
For a blow that frame is now + startup. For a bolt it adds the flight (`duelist::lands_after`). Otherwise the foe
holds for a few frames. *No two foe attacks land within 300 ms of each other*, and that is a
test (`the_gauntlet_never_lands_two_strikes_inside_the_reaction_window`, a minute of five foes
against a hero who stands still), and the bots measure it with the hero moving (§7).

Lessons from the lab, now in the rules:

* **Space by landing, not by release.** The first build spaced a hex by its cast. A bolt then
  arrived 10 f before a Brute's slam, and the hero's mercy frames swallowed the slam.
* **A token comes back, but its strike keeps its slot.** When a Hexer's recovery ends with its
  bolt still in the air, its threat point frees up while its landing frame keeps the others clear
  (`ThreatManager::release`, a *spent* hold).
* **The leash scales with the walk.** A flat 150 f timeout took the token back from a Brute
  still walking in. A token now lasts `max(150, eta + 60)` frames.
* **Telemetry counts real landings.** A landing is the frame a strike reaches the hero, however
  he meets it (hit, block, parry, evade). *Overlaps* counts landings inside the 18 f window.
* **Plans keep slack.** A moving hero shifts landings by a few frames: he steps into a bite's
  reach a frame or two into its active frames. Plans 24 f apart land 18 f apart.
* **Bolts are time-locked.** A bolt flies straight but paces itself (0.5× to 1.6× its speed) to
  touch the hero on its planned frame. A hero walking into it no longer brings it in early. A
  sidestep still dodges it, and a bolt that has missed its moment by more than the slack burns
  out, so it cannot come late into another strike's slot.

### 4.2 Positional play: rings and sectors

Around the hero:

| Ring | Radius | Who |
|---|---|---|
| Engage | the move's range | the token holder only |
| Hold | Brute 3.6, Skirmisher 3.2 | melee foes without a token, strafing (the biters wait nearer: they must walk in to bite) |
| Ranged | 6.0 | Hexers; they back off if the hero comes within 3.0 |

The circle is cut into **8 sectors of 45°**. Each waiting foe claims one free sector on its
ring, nearest its current bearing. Sectors in the hero's back arc (±45° behind him) cost an
extra 3.2 rad, more than any turn, so waiting foes walk round to his sides and front, where he
can see them; his back fills only when the five sectors before it are taken. Foes keep
0.9 tiles apart (separation) and never stand inside the hero.

### 4.3 The three archetypes: one question each

| | **Brute** (Blemmyes) | **Skirmisher** (Loup-garou) | **Hexer** (Witch) |
|---|---|---|---|
| Question it asks | *Can you wait?* | *Can you read the timing?* | *Can you close the distance?* |
| HP / Poise / Armor | 200 / 100 / 35 | 80 / 25 / 0 | 55 / 20 / 0 |
| Stagger / Resolve | 90 f / 120 f | 50 f / 60 f | 60 f / 60 f |
| Walk / Strafe (t/s) | 1.8 / 1.2 | 4.6 / 2.8 | 2.2 / 2.0 (retreat 3.2) |
| Turn rate | 150°/s | 540°/s | 360°/s |
| Moves | Slam (front), Sweep (when flanked within 1.3) | Bite from beside him (≤ 1.3 tiles); it walks up, bites, and stays where it bit: no lunge, no spring back, no hop | Hex Bolt from ≤ 7.5 tiles |
| Answer | evade the slam, punish the 48 f recovery, break it with L3 | parry the bite (deflect 60 f), step out of its reach, or punish its 24 f recovery | dash in between bolts; parry reflects |
| Rest after a move | 70 f | 60 f | 75 f |
| Radius | 0.5 | 0.38 | 0.35 |

What was **subtracted** in the ring: contact damage, the old toughness counts, witches'
minions, the loup-garou's fury, bombs, archangels, shrine weapons and blade gifts. Each of those
goes back only if it earns a place on top of the core loop.

---

## 5. The lab: the Sparring Ring

West of the gym's arrival platform: a 17×17 platform. Step in and a bout begins.

| Bout | Foes | What it tests |
|---|---|---|
| 1 The Brute | Brute | waiting, punish windows, poise break |
| 2 The Pack | Skirmisher ×2 | parry timing, token cadence |
| 3 The Hexer's Guard | Hexer + Skirmisher | zoning while pressured |
| 4 The Trio | one of each | the whole triangle |
| 5 The Gauntlet | Brute, Skirmisher ×2, Hexer ×2 | the threat manager under load |
| 6 Steel | tengu + rasetsu | melee: two blades, one token between them |
| 7 Fang and Fire | dire wolf + cobra + jorogumo | jaws beside him while fire comes from afar |
| 8 The Night Parade | the third stage's five | every way of attacking at once |
| 9 The Iron Club | oni + kappa | an unstoppable club, a shove from beside him |
| 10 The Fox's Wedding | the fourth stage's four | a long neck that bites, foxfire and illusions |
| 11 The Call in the Cold | karakoncolos + tülükuş | doomed by a roar and a cry; talons from out of reach |
| 12 Zemheri Nights | the fifth stage's five | a hag's weight, root darts, fire out of nowhere, a vanishing, all at once |
| 13 The One Eye | Tepegöz | a body no blade cuts: strike the eye when it comes within reach |
| 14 The Old Tales | the sixth stage's four | a grab, seven heads and a plea, lightning from a ghost |
| 15 Fire and Ice | frost giant + Surtr | frostbite through the shield, fire on the floor |
| 16 Ragnarök | the seventh stage's four | the end of the world: everything above, harder |
| 17 The Prince of Greed | Mammon | a boss on the same rules: phases, a hide, coins on the floor, a grasp (§13) |

Bouts 1-5 are the lab: the archetypes in their own bodies. Bouts 6-8 bring the third stage's
foes, each a breed of an archetype (§10); bouts 9-16 the later stages', with their legends (§11),
each measured with the kit of its stage (tiers 3, 5, 7 and 9).

* Losing restores the hero and restarts the bout. Winning moves on to the next.
* **F8**: frame readout (hero state and frame, buffer, threat points, each foe's way of attacking and its state, bout stats).
* **F6**: skip to the next bout; **F7**: to the first bout of the next stage's foes.
* When a bout ends, a **results card** shows its time against the target, the health lost, the
  damage dealt, how the strikes were met (parried, blocked, evaded) and the mistakes (caught
  mid-swing, guards broken). A line goes into `combat_log.csv` in the save folder.
* Captures: `ring` (into the ring), `bout:N`, `readout`, `bot:K` (a bot plays: 0 novice, 1 adept, 2 expert).

---

## 6. Roadmap

| Phase | Scope | Exit criterion |
|---|---|---|
| **0 Spec** | this document | numbers pinned by tests ✔ |
| **1 The Ring** | `logic::combat` (frames, FSM, buffer, formulas, threat manager, archetypes), the gym's Sparring Ring, telegraphs, HP/poise bars, frame readout, five bouts | builds, tests green, five bouts playable ✔ |
| **2 Tune** | the balance harness (§7): three bot skills play every bout; telemetry on screen when a bout ends and in `combat_log.csv`; tables tuned in two rounds | expert TTK inside every target ✔, zero overlaps for a player who stands his ground ✔ (≈ 0.4 a minute for one who moves), the skill gradient of §7 ✔. Your own parry rate is in the log. |
| **3 Hero kit** | `combat::kit`: blade tiers as move-table scalars (power, poise, tempo, reach, arc, finisher), shields as guard stats (parry window, block cost, riposte, deflect, reflect), the path audited against the FSM (§8) | every tier is the same three verbs with different numbers ✔; each hero's tier 10 wins ≥ 2× faster than his tier 1 ✔; at every tier the heroes kill within 1.3× of each other on the Brute and the Trio ✔ |
| **4 Roll-out** | every foe kind a breed of an archetype; the brawl (`game::brawl`) runs the Vaults of Mammon: packs of duelists asleep in their dens, one threat manager, real death, shield and blade wear, contact damage retired there (§9) | the expert clears the vaults with every kit, the adept from tier 4; zero overlaps in every crawl ✔ |
| **Stage 3** | five new bodies, each a breed of an archetype; every stage's cast attacks all three ways (melee, body, distance); a third platform in the gym and three bouts in the ring (§10) | the expert wins them with the training sword, the adept with tier-4 gear, a masher with neither ✔ |
| **Stages 4-7** | seventeen bodies from folklore, each a breed of an archetype with a move, a rider and a power of its own; new shapes (lane, marks, volley, quake), the hero's ails, holds, summons; four more platforms and eight bouts (§11) | at its stage's tier the expert wins every bout inside its target and a masher none; three tiers up (or the last) a careful player wins them all; zero overlaps ✔ |
| **5 Bosses** | Mammon from the same FSM: phase-specific move tables, armor windows, token cost 3; his court, the Counting House; a bout for him in the ring (§13) | the boss is a fourth archetype, not a special case ✔; the expert beats him with the kit a hero has on reaching him (tiers 1-2) and a masher does not ✔; zero overlaps ✔ |

---

## 7. Phase 2: the balance harness and the tuning log

A human playtest gives one data point an hour. The harness gives forty-five in seven seconds.
`game::bot` is a scripted player that drives the real controller path (a pad's stick and
buttons into `Input`). It sees only what a player sees: a foe's tell from the frame its
reaction delay has passed, bolts in flight, its own stamina.

| Skill | Model |
|---|---|
| Novice | walks at the nearest foe and mashes attack; never guards |
| Adept | holds the guard when it sees a strike coming (reaction 18 ± 4 f); attacks what is in reach, keeping a block's worth of stamina |
| Expert | raises the guard to put each strike in the parry window (reaction 12 ± 2 f, timing ± 2 f); strikes only into openings, and never into a swing it could not guard out of before the next strike it has seen |

Run it: `AdventuresOfTheAncientBlades.exe --balance N` (N seeds per bout and skill).
`the_bouts_meet_their_balance_targets` pins the expert's times, the novice's losses and the
adept's win over the Brute.

### Targets

* **Expert** wins every bout, inside the bout's target time (§2.1, ± 20%).
* **Adept** beats the Brute and wins the early bouts at least half the time. The Gauntlet
  may beat it.
* **Novice** loses to the Brute (*can you wait?*), the Pack (*can you read the timing?*) and
  everything after. The Hexer's Guard may fall to it.
* **Overlaps**: none for a player who stands his ground, and rare for one who moves.

### Round 0 (as built in phase 1), 3 seeds

| Bout | Novice | Adept | Expert TTK (target) |
|---|---|---|---|
| Brute | 0% | 0% (5.5 guard breaks a bout) | 14.0 s (8-14) |
| Pack | 100% | 100% | 6.7 s (5-10) |
| Hexer's Guard | 100% | 100% | 4.7 s (5-10) |
| Trio | 0% | 0%, 3 overlaps | 17.6 s (13-24) |
| Gauntlet | 0% | 0%, 4 overlaps | 22.4 s (20-32) |

### What changed, and why

| Change | Why |
|---|---|
| Overlaps counted where strikes reach the hero | a lunge lands a few frames into its active window, and a bolt when it arrives |
| Plans keep 6 f of slack (24 f apart) | a moving hero shifts landings by a few frames |
| Bolts are time-locked | a hero walking into a bolt brought it in 12 f early |
| A spent token keeps its landing slot | a Hexer's recovery ended with its bolt still flying |
| Block costs 0.7 × damage, not 1.0 | the adept's guard broke on every slam (30 stamina) |
| Deflect 40 → 60 f | a parried slam opened less than a dodged one (48 f recovery) |
| Skirmisher: lunge 15 → 18, HP 70 → 80, the sidestep | a masher beat the Pack every time; now its first swing whiffs and the lunge lands in its recovery |
| Brute: HP 240 → 200, rest 90 → 70 f | the expert took 14 s, the top of its target |
| Hex bolt 12 → 15; reflect ×2 → ×1.5; Hexer HP 45 → 55 | two reflected bolts killed the Hexer, so the expert's Hexer's Guard ran 3.3 s |

### Round 2 (now), 6 seeds

| Bout | Novice | Adept | Expert TTK (target) | Expert health lost |
|---|---|---|---|---|
| Brute | 0% | 100%, 29 s | **11.6 s** (8-14) | 0 |
| Pack | 0% | 50% | **6.2 s** (5-10) | 6 |
| Hexer's Guard | 100% | 100% | **6.2 s** (5-10) | 0 |
| Trio | 0% | 83% | **16.6 s** (13-24) | 15 |
| Gauntlet | 0% | 0% | **25.9 s** (20-32), 100% wins | 38 |

Overlaps: 0 for the expert in every bout; 3 for the adept across all its Trio and Gauntlet runs
(about 0.4 a minute of fighting).

What the numbers say for phase 3: the adept wins by turtling (43 blocks in a Trio, 77 s),
because blocking is safe and every swing is a risk. When blade tiers arrive, blade **tempo**
should shorten that risk, so a better blade rewards attacking rather than just making numbers
bigger.

---

## 8. Phase 3: the hero's kit as numbers

Every blade, shield and path level is the same three verbs (strike, guard, evade) with different
numbers. `combat::kit::Kit::of(hero, blade, edge, shield, path)` builds the kit when a bout
begins. The hero's `Fighter` then runs on the kit's table (`Kit::moves`), its parry window, its
evade landing and its broken guard, and the ring reads the rest from it. A fresh Ragnar's kit is
the training kit of §1, exactly (`a_fresh_ragnar_fights_the_training_table`), so phase 2's
balance stands.

### 8.1 Blade → strike

```
weight  = sqrt(min(cooldown, 0.6) / 0.42)                 a heavier blade: slower, and it breaks more
tempo   = clamp(weight · (1 − 0.02·(tier − 1)) · sqrt(hero tempo) · sqrt(0.85 if path ≥ 1)
                · (1.08 for the janissary), 0.7, 1.25)
power   = (1 + 0.15·(tier − 1)) · (0.4 + 0.6·edge) · might^0.25 · (1.1 if its gift is cut)
poise   = min(1.5, (1 + 0.04·(tier − 1)) · weight · (1.15 if it stuns or flashes))
reach   = clamp(swing reach − 1.2, 0, 0.9)                thrown weapons and bows: +0.3
arc     = clamp(atan2(half width, reach) − atan2(0.7, 1.2), −15°, +29°)
frames  = startup, recovery and cancel × tempo  (startup ≥ 3, recovery ≥ 8; active unchanged)
stamina = 16 × tempo per swing                            a quick blade buys more blows per breath
```

| Blade feature | In the ring |
|---|---|
| Swings behind (a whirl) or flashes | the finisher sweeps all round (360°) |
| Thrown, or a bow | the finisher flies down his aim (its speed, its range up to 9 tiles) |
| A wave | the finisher reaches 1 tile further |
| A flash | the finisher lands ×1.5 |
| Pierces armour | +15 poise toward armour checks (not to poise damage): light blows flinch the Brute |
| Critical | +0.2 on the multiplier of an opening |
| Drinks blood | 2 health per blow that lands |
| Stuns, flashes | poise ×1.15 |
| Trick, bleed, burn, venom, lightning, stealth, fire trail | **cut** (they act outside the state machine); the blade bites ×1.1 instead |

### 8.2 Shield → guard

```
parry window = 8 + (tier − 1)/3 frames               8 at tier 1, 11 at tier 10
block cost   = 0.7 − 0.4 · (block share − 0.4)/0.45   stamina per point: 0.70 down to 0.30
```

| Shield gift | In the ring |
|---|---|
| Bash (tier 4) | a parry takes 10 more poise (riposte) |
| Missile-proof (5) | a bolt caught on the guard costs nothing |
| Reflect (6) | a parried bolt does ×2 to the foes (×1.5 without) |
| Stagger (7) | a parried foe reels 80 f (60 without) |
| Unbreakable (8) | a broken guard lets 25% through and reels 24 f (50%, 48 f without) |
| Thorns (9) | a parry deals the attacker 10 |
| Aegis (10) | a parry gives back 5 health |
| No shield | a bare-handed guard: 5 f window, a block costs the whole blow |

Durability is not worn down in the ring: the lab never breaks real gear. Wear comes in phase 4.

### 8.3 The path audit

| Level | Skill | Verdict | In the ring |
|---|---|---|---|
| 1 | Rage | keep | tempo × √0.85 (Ragnar takes ×1.1) |
| 2 | Guard | keep | +50 health with a shield |
| 3 | Seer's Sight (Ragnar, Mehmed) | adapt | +0.1 on openings (it was crits on the boss) |
| 3 | Wind Step (Miyamoto) | adapt | the evade lands in 2 f |
| 4 | Song | keep | stamina comes back ×1.6 (unchanged) |
| 5 | Bifrost Step | adapt | the evade lands in 0 f (the blink itself is cut: one evade verb) |
| 6 | Fang (Ragnar) / Flank | adapt | ×1.5 from behind (Ragnar) / ×1.25 from the flank or behind |
| 7 | Forge-Heart (Ragnar) | keep | free swings when his breath runs out (unchanged) |
| 7 | Defiance (others) | n/a | nothing in the ring hinders him |
| 8 | Terrify (Ragnar) | adapt | a kill takes half the poise of the foes round it (it was panic) |
| 8 | Last Stand (others) | adapt | second wind: once a bout, a fatal blow leaves 30% |
| 9 | Valkyrie Call (Ragnar) | adapt | second wind: 50% |
| 9 | Spirits (others) | **cut** | they strike outside the state machine |
| 10 | World Tree | adapt | +25% on the second wind |

A blow from anything outside the frame data does nothing to a duelist (`Game::stray_blow`).

### 8.4 Results: every hero at tiers 1, 4, 7 and 10 (6 seeds, `--kits 6`)

```
hero      tier  tempo power reach parry cost | expert: brute   trio  gauntlet      | adept: brute (blocks)  trio (win, blocks)
Mehmed       1   0.90  1.06   0.0    8f 0.70 |          10.4   13.9   25.3 (100%) |         24.2 ( 9.0)    83%  40.3
Mehmed       4   0.98  1.53   0.4    9f 0.56 |           5.4    7.3   12.7 (100%) |         10.7 ( 4.0)   100%  13.5
Mehmed       7   1.00  2.21   0.7   10f 0.43 |           3.9    5.9    8.9 (100%) |          6.9 ( 2.7)   100%   8.5
Mehmed      10   0.98  2.48   0.3   11f 0.30 |           2.6    4.3    9.2 (100%) |          4.8 ( 2.0)   100%   3.5
Ragnar       1   0.92  1.00   0.0    8f 0.70 |          11.4   16.6   25.6 ( 83%) |         28.2 (10.0)   100%  47.3
Ragnar       4   0.93  1.45   0.3    9f 0.56 |           5.9    7.7   12.0 (100%) |         10.6 ( 4.0)   100%  13.7
Ragnar       7   0.84  1.90   0.5   10f 0.43 |           3.8    6.6    7.8 (100%) |          5.6 ( 0.7)   100%   2.7
Ragnar      10   0.82  2.58   0.3   11f 0.30 |           2.4    4.7    7.5 (100%) |          3.5 ( 1.0)   100%   3.0
Miyamoto     1   0.74  1.00   0.0    8f 0.70 |           8.9   14.1   22.6 ( 83%) |         24.2 ( 9.0)    67%  34.5
Miyamoto     4   0.83  1.59   0.3    9f 0.56 |           5.4    6.7   10.3 (100%) |          9.9 ( 4.0)   100%   9.3
Miyamoto     7   0.75  1.90   0.5   10f 0.43 |           4.5    6.3    8.1 (100%) |          4.8 ( 0.0)   100%   2.8
Miyamoto    10   0.81  2.35   0.3   11f 0.30 |           2.4    4.4    6.4 (100%) |          3.4 ( 1.0)   100%   4.0
```

* **Growth**: each hero's expert kills the Brute 3.7-4.8× faster at tier 10 than at tier 1, and
  the Trio 3.2-3.5× faster. Every step of the sweep is faster than the one before it on the Brute
  and the Trio.
* **Parity**: at every tier the three heroes are within 1.3× on the Brute and the Trio (worst:
  1.28× at tier 1). The five-foe Gauntlet spreads to 1.44× at tier 10.
* **Attacking pays**: the adept's blocks per Brute fall from 9-10 at tier 1 to 1-2 at tier 10.
  The turtling of §7 was a tier-1 problem.

### 8.5 What changed, and why

| Change | Why |
|---|---|
| Poise: +4% a tier, ×1.15 for stunning blades, capped at 1.5 (was √power × weight, uncapped) | tier 4 was a cliff (11.6 → 4.2 s): poise breaks compound, and a light blow beat the Brute's armour at tier 10 |
| Riposte 25 → 10, crit 0.3 → 0.2, Seer 0.15 → 0.1 | the shield's and the path's bonuses multiplied with the blade's |
| Might counts at a quarter; janissary tempo ×1.08 | Mehmed killed 35% faster than Ragnar at tier 1, on top of his extra health |
| A cut gift folds into ×1.1 power | blades whose gift the ring cuts fell behind (Miyamoto's tier 4 was 1.5× slower than Mehmed's) |
| A swing's stamina scales with tempo | the expert waits for openings, so a quick blade helped it little: stamina ran out first |
| Cooldown counts up to 0.6 s | the sun-blades' long wait paid for a flash the ring cuts; Miyamoto's tier 10 was slower than his tier 7 |

What the lab taught:

* For a player who waits for openings, **power is worth more than tempo**, and the Brute's
  kill time has a floor set by its own attack cycle.
* **A shield is defence until tier 7**: on its own it does not shorten a fight (Mehmed's tier-4
  shield: 11.6 s on the Brute with it or without it), until the longer deflect and the doubled reflect turn guarding into offence.

### 8.6 Every blade and shield as the ring reads it (`--kit-table`; path 0, keen)

```
hero      tier blade                     tempo power poise reach  arc   finisher        traits
Mehmed       1 Bronze Yatagan             0.97  1.06  0.90  +0.0   +0deg 1.7 t           
Mehmed       2 Iron Kilic                 1.15  1.22  1.13  +0.3  +15deg 2.0 t           
Mehmed       3 Steel Pata Guard           1.13  1.37  1.18  +0.2   -7deg 2.9 t           
Mehmed       4 Dervish Crescent Blade     1.06  1.53  1.17  +0.4  +15deg whirl           
Mehmed       5 Koroglu's Steel Flintlock  1.19  1.69  1.39  +0.3   +0deg thrown          
Mehmed       6 Janissary Mace of Conquest  1.06  1.85  1.31  +0.3   +3deg 2.0 t           pierce
Mehmed       7 Zulfikar-Infused Greatsword  1.09  2.21  1.42  +0.7  +10deg 2.4 t           
Mehmed       8 Simurg Feather Bow         1.11  2.38  1.50  +0.3   +0deg thrown          
Mehmed       9 Masterwork Djezzar Saber   0.97  2.33  1.50  +0.6   +3deg 2.3 t           crit +0.20
Mehmed      10 Kaf Dagi Sun-Blade         1.06  2.48  1.50  +0.3   +8deg whirl           finisher x1.5
Ragnar       1 Bronze Hrotti              1.00  1.00  1.00  +0.0   +0deg 1.7 t           
Ragnar       2 Iron Skofnung              1.01  1.15  1.08  +0.3   +0deg thrown          
Ragnar       3 Steel Angurvadel           1.05  1.30  1.18  +0.2   -1deg 2.9 t           
Ragnar       4 Tyr's Rune-Etched Axe      1.00  1.45  1.38  +0.3   +2deg 2.0 t           
Ragnar       5 Dwarven Masterwork Dual Axes  0.96  1.60  1.21  +0.4  +15deg whirl           
Ragnar       6 Laevateinn Wand-Sword      0.93  1.93  1.24  +0.4   +2deg 2.1 t           
Ragnar       7 Mistilteinn Magic Blade    0.91  1.90  1.28  +0.5   +3deg 2.2 t           pierce
Ragnar       8 Gram, Sigurd's Bane        0.94  2.25  1.40  +0.6   +6deg 2.3 t           
Ragnar       9 Gungnir, Odin's Spear      0.96  2.20  1.50  +0.3   +0deg thrown          crit +0.20
Ragnar      10 Mjolnir, Thor's Hammer     0.89  2.58  1.48  +0.3   +0deg thrown          
Miyamoto     1 Katana of the Iron Gate    0.80  1.00  0.90  +0.0   +0deg 1.7 t           
Miyamoto     2 Oni-Slayer Nodachi         0.96  1.15  1.13  +0.3  +15deg 2.0 t           
Miyamoto     3 Tengu Wind-Blade           0.98  1.30  1.24  +0.1  +12deg whirl           
Miyamoto     4 Kusanagi-no-Tsurugi Fragment  0.90  1.59  1.20  +0.3   +3deg 2.0 t           
Miyamoto     5 Dual Ninjato of the Shadow  0.86  1.76  1.21  +0.4  +15deg whirl           
Miyamoto     6 Yari of the Shifting Serpent  0.88  1.93  1.31  +0.9  -15deg 2.6 t           
Miyamoto     7 Muramasa Cursed Blade      0.81  1.90  1.28  +0.5   +3deg 2.2 t           pierce, blood 2
Miyamoto     8 Yumi of the Heavenly Bow   0.92  2.05  1.50  +0.3   +0deg thrown          
Miyamoto     9 Masamune Masterpiece Katana  0.78  2.20  1.37  +0.6   +3deg 2.3 t           pierce, crit +0.20
Miyamoto    10 Amaterasu's Sun-Forged Blade  0.88  2.35  1.50  +0.3   +8deg whirl           finisher x1.5

tier shield                    parry cost  riposte deflect reflect bolt-proof break
   1 Linden Board Shield          8f 0.70       0     60f    1.5x no         50%/48f
   2 Iron-Boss Round Shield       8f 0.63       0     60f    1.5x no         50%/48f
   3 Rawhide Warboard             8f 0.59       0     60f    1.5x no         50%/48f
   4 Jarl's Painted Shield        9f 0.56      10     60f    1.5x no         50%/48f
   5 Oak of the Shield-Wall       9f 0.52      10     60f    1.5x yes        50%/48f
   6 Ullr's Ship                  9f 0.48      10     60f    2.0x yes        50%/48f
   7 Hildr's Warboard            10f 0.43      10     80f    2.0x yes        50%/48f
   8 Brokkr's Bulwark            10f 0.39      10     80f    2.0x yes        25%/24f
   9 Hel's Bone Shield           10f 0.34      10     80f    2.0x yes        25%/24f
  10 Svalinn                     11f 0.30      10     80f    2.0x yes        25%/24f
```

Tools: `--kits N` (the sweep), `--kit-parts T` (each tree on its own at tier T: what each is
worth), `--kit-table` (the table above). `every_heros_kit_grows_and_the_heroes_stay_level` pins
growth and parity at tiers 1 and 10.

---

## 9. Phase 4: the roll-out to the Vaults of Mammon

The frame-data rules leave the lab. `game::brawl` holds what the ring and the vaults share:
the hero's fighter on his kit, the threat manager, the bolts, the telemetry. The `Field` says
where it is fought:

| | Ring (the lab) | Vault (the game) |
|---|---|---|
| Foes | a bout's archetypes | every pack, asleep in its den until woken |
| Ground | a square (`Arena`) | rooms and corridors (`GridTerrain`): foes slide along walls and find their way round them by the hero's flow field (one BFS from his tile, redrawn when he steps onto a new one), or a search for anything farther |
| Threat | 3 points, a token per 30 f | **2 points, a token per 60 f**: a pack is a fight, not a pile-on |
| His health | a lost bout, never death | real: lives, the dying, the packs back asleep at their posts and whole |
| His gear | never worn | his shield wears as it blocks and breaks for good; his blade dulls as it strikes |
| Blows from outside the frame data | nothing (the lab is pure) | a plain blow through the fighter (a bomb, an archangel's fire: 20 damage, 30 poise) |
| A kill | gone | gone, with its coins, rune chance, streak and points, and 20% a health potion |

The kit is refreshed every frame, so a dulled blade, a broken shield or a potion counts at once
(strength ×1.25 power; haste ×0.75 tempo; Ragnarok ×2 power and ×0.67 tempo).

### 9.1 Ten bodies, three archetypes

Every kind fights as one of the three, its numbers scaled (`archetype::Breed`):

| Kind | Archetype | Health | Damage | Poise | Pace | Cost |
|---|---|---|---|---|---|---|
| Blemmyes | Brute | 1.0 | 1.0 | 1.0 | 1.0 | 2 |
| Helmeted skeleton | Brute | 0.6 | 0.7 | 0.6 | 1.15 | 1 |
| Draugr | Brute | 0.45 | 0.6 | 0.5 | 1.1 | 1 |
| Loup-garou | Skirmisher | 1.0 | 1.0 | 1.0 | 1.0 | 1 |
| Large bat | Skirmisher | 0.8 | 0.85 | 0.8 | 1.1 | 1 |
| Night raven | Skirmisher | 0.6 | 0.7 | 0.6 | 1.0 | 1 |
| Raven | Skirmisher | 0.45 | 0.6 | 0.5 | 1.05 | 1 |
| Witch's minion | Skirmisher | 0.35 | 0.55 | 0.4 | 1.0 | 1 |
| Witch | Hexer | 1.0 | 1.0 | 1.0 | 1.0 | 1 |
| Bog slime | Hexer (green spit) | 0.8 | 0.8 | 0.8 | 0.7 | 1 |

Armor scales with poise, so the helmeted skeleton (21) still shrugs off the training sword's
light blows and the draugr (17.5) does not. The vaults weigh their foes as the first realm:
health ×0.85 and damage ×0.8. Later realms raise these as the keys held mount up.

### 9.2 What the old rules did, and what they do now

| Old rule | Under the frame data |
|---|---|
| Contact damage | gone: a duelist hurts only on its active frames |
| Toughness (one blow fells) | health and poise |
| Witches conjure minions | subtracted |
| The loup-garou's fury, the draugr's reassembling | subtracted: what falls in the vaults stays down |
| Scare, confuse, frenzy, Michael's turning | they pass a duelist's mind by; Michael's fire *awes* it (all its poise: it staggers), a stunning blow or a shield's clash takes 25 poise |
| Bifrost Step's blink | cut (one evade verb); path 5 lands his leap ready |
| Shrine weapons from vault chests | still fire as they always have; their shots are stray blows |

### 9.3 The crawl (`--crawl 3`: Ragnar's kit at each tier, from the antechamber to the last pack)

```
skill   tier  cleared  kills/foes  deaths  minutes  combat  peak  taken  parry block caught  overlaps/min
adept      1      0%     34/61       3.0     11.5    10.4     6    531     53   318     12          0.00
expert     1    100%     61/61       0.0     10.0     8.4     6    172    200     0      7          0.00
adept      4    100%     61/61       0.7      9.1     7.4     6    549     34   157     20          0.00
expert     4    100%     61/61       0.0      5.5     3.9     6    166    105     0      5          0.00
adept      7    100%     61/61       0.0      5.5     3.6     6    290     23    29     12          0.00
expert     7    100%     61/61       0.0      4.2     2.2     6     98     44     0      3          0.00
```

* **Every kit clears the vaults**, except the adept at tier 1, which needs better gear first
  (34 of 61, then its lives run out). The expert clears even at tier 1 without dying.
* **Readable cadence**: no two strikes land within 300 ms of each other in any crawl, and at
  most 6 foes hunt him at once.
* **A Diablo dungeon's length**: 4 to 10 minutes from the antechamber to the last pack.

### 9.4 What changed, and why

| Change | Why |
|---|---|
| Packs: chambers 2-4 (were 3-6), the hall 6 (8), the exit 5 (6), at most 70% second stage (85%) | sized for the old rules, where any blow felled a foe; with health and poise, 93 foes and 8-10 hunting him at once outlasted every adept |
| Wake range 8 → 6 tiles | the packs next door woke with the one he walked into |
| The vaults' threat: 2 points, 60 f | the adept met a strike every 1.7 s and ran out of health in under 4 minutes, however well it blocked |
| Health potions from 20% of kills; the bots drink below 40% and pick up what lies near | sustain, as Diablo's health globes give it |
| The realm's weight: health ×0.85, damage ×0.8 | Greed is the first realm |
| Every harness run starts from a fresh save | runs shared the save folder: one crawl's broken shield left the next "fresh" hero bare-handed (a 5-frame parry), and the ring's balance seemed to shift |

Since the bots drink potions now, the adept's ring numbers of §7 rise a little (the Pack and
the Trio: 100%). The expert's are unchanged to the decimal.

Tools: `--crawl N`. `the_vaults_are_a_brawl_a_skilled_player_can_clear_with_a_readable_cadence`
pins the expert's tier-4 clear and the adept's tier-7 clear, with no overlaps.

---

## 10. The third stage: five new bodies, three ways of attack

The brief for every stage: some foes attack with their body, some in melee, some from a
distance. Those are the three archetypes' ways of reaching the hero (`archetype::Attack`):

| Way | Archetype | Its move | The question |
|---|---|---|---|
| Melee | Brute | a weapon swung at arm's length: the slam, the sweep | can you wait? |
| Body | Skirmisher | teeth, claws or weight, from beside him: the bite | can you read the timing? |
| Distance | Hexer | a bolt from afar, each kind's its own | can you close the distance? |

Every stage's cast has all three (`every_stages_cast_attacks_all_three_ways`):

| Stage | Melee | Body | Distance |
|---|---|---|---|
| 1 The Garden | draugr | raven, night raven | bog slime |
| 2 The realms | helmeted skeleton, blemmyes | loup-garou, large bat, witch's minion | witch |
| 3 | tengu, rasetsu | dire wolf, cobra | jorogumo |
| 4 The yōkai | oni | kappa, rokurokubi | kitsune |
| 5 Anatolian nights | karakoncolos | tülükuş, albastı | cin, arçura |
| 6 The legends | Tepegöz, gashadokuro | yedi başlı ejder | goryō |
| 7 Ragnarök | frost giant, Surtr | Níðhöggr | Hel |

(The rasetsu, a man-eater of the Buddhist hells, was the stage-3 "oni" until the fourth stage
brought the oni of the bestiary's own cards.)

### 10.1 The third stage's breeds

Stronger than the archetypes' own bodies (the second stage's), yet inside the archetypes' design
limits: every brute still flinches from the training sword's finisher (its armor under the
finisher's 45 poise), and every skirmisher is still slower than the hero and broken inside one
string.

| Kind | Archetype (way) | Health | Damage | Poise | Pace | Cost | As it fights |
|---|---|---|---|---|---|---|---|
| Tengu | Brute (melee) | 1.0 (200) | 1.15 | 0.9 | 1.5 | 2 | a swordsman: closes in half again as fast as a brute; slam 35, sweep 23 |
| Rasetsu | Brute (melee) | 1.5 (300) | 1.2 | 1.25 | 1.0 | 2 | the heavy: the longest fight of the stage, armor 44; slam 36 |
| Dire wolf | Skirmisher (body) | 1.5 (120) | 1.2 | 1.3 | 1.1 | 1 | the bite, harder and longer-lived than the loup-garou's; 24 |
| Cobra | Skirmisher (body) | 1.25 (100) | 1.3 | 1.0 | 0.8 | 1 | slow on the ground, the hardest bite of the stage: 26 |
| Jorogumo | Hexer (distance) | 1.6 (88) | 1.2 | 1.5 | 1.0 | 1 | balls of fire (orange, her own tell colour); backs away; 18 |

No new moves: systematic reduction holds at fifteen bodies. Each has its own tell to the ear
(the tengu's steel, the cobra's hiss, the dire wolf's growl, the jorogumo's shriek) and its own
pose on the archetype's frames (the cobra rears back on its coil, then strikes down off it).

### 10.2 In the gym

* A third platform east of the second holds the five at their posts, as the first two hold theirs.
* The Sparring Ring has three bouts of them after the lab's five (§5): Steel, Fang and Fire,
  and the Night Parade. Bouts now name their foes' bodies; the lab's are the archetypes' own
  bodies (pinned), so bouts 1-5 play as before, to the decimal.

### 10.3 Results (`--stage-three 6`: Ragnar's kit at tiers 1, 4 and 7)

```
bout                 tier  skill   win   ttk(s)  target    taken  hurt  parry block overlaps
STEEL                   1  novice     0%     0.0      -        124   3.0    0.0   0.0    0
STEEL                   1  adept    100%    93.9      -         33   1.7    1.5  36.7    0
STEEL                   1  expert   100%    25.4    20-32        0   0.0   17.2   0.0    0
STEEL                   4  novice     0%     0.0      -        159   4.0    0.0   0.0    0
STEEL                   4  adept    100%    33.5      -          0   0.0    0.3  14.7    0
STEEL                   4  expert   100%    12.3    20-32        0   0.0    7.0   0.0    0
STEEL                   7  novice   100%     6.4      -          0   0.0    0.0   0.0    0
STEEL                   7  adept    100%    16.0      -          0   0.0    0.3   6.7    0
STEEL                   7  expert   100%     6.1    20-32        0   0.0    1.0   0.0    0
FANG AND FIRE           1  novice     0%     0.0      -        117   5.0    0.0   0.0    0
FANG AND FIRE           1  adept     17%    24.8      -        124   5.2    4.7   4.8    0
FANG AND FIRE           1  expert   100%    12.9    10-16        0   0.0   11.5   0.0    0
FANG AND FIRE           4  novice     0%     0.0      -        155   6.0    0.0   0.0    0
FANG AND FIRE           4  adept    100%     9.2      -         62   2.5    3.3   1.3    0
FANG AND FIRE           4  expert   100%     7.1    10-16       32   1.3    3.7   0.0    0
FANG AND FIRE           7  novice   100%     6.3      -         93   4.0    0.0   0.0    0
FANG AND FIRE           7  adept    100%     5.6      -         33   1.3    1.7   0.8    0
FANG AND FIRE           7  expert   100%     3.7    10-16        0   0.0    2.0   0.0    0
THE NIGHT PARADE        1  novice     0%     0.0      -        110   4.0    0.0   0.0    0
THE NIGHT PARADE        1  adept      0%     0.0      -        148   6.5    5.8   7.5    0
THE NIGHT PARADE        1  expert   100%    35.9    29-45       38   1.7   41.0   0.0    0
THE NIGHT PARADE        4  novice     0%     0.0      -        166   6.0    0.0   0.0    0
THE NIGHT PARADE        4  adept    100%    63.8      -         76   3.0    8.7  40.0    0
THE NIGHT PARADE        4  expert   100%    18.9    29-45        5   0.2   18.3   0.0    0
THE NIGHT PARADE        7  novice     0%     0.0      -        151   6.0    0.0   0.0    0
THE NIGHT PARADE        7  adept    100%    37.5      -         41   1.7    3.5  23.7    0
THE NIGHT PARADE        7  expert   100%    11.3    29-45       95   3.0    5.5   0.2    0
```

* **Skill or gear**: with the training sword only the expert wins the third stage's bouts (the
  adept wins Steel only by turtling, 37 blocks in 94 s, and Fang and Fire one time in six); a
  masher wins none. With a tier-4 kit the adept wins all three; at tier 7 even the masher beats
  Steel and Fang and Fire, but not the Night Parade.
* **Readable cadence**: zero overlaps everywhere; the threat manager paces fifteen bodies as it
  paced three.
* The tier-7 expert wins the Night Parade fastest (11 s) but trades more blows doing it (95
  health lost, against 5 at tier 4).

### 10.4 What changed, and why

| Change | Why |
|---|---|
| The stage's damage trimmed: rasetsu 1.3 → 1.2, dire wolf 1.25 → 1.2, cobra 1.4 → 1.3, jorogumo 1.3 → 1.2 | the expert first won the Night Parade with 1 health left (99 lost); now 38 |

Tools: `--stage-three N`. `the_bouts_meet_their_balance_targets` pins the expert's times in
every bout, the third stage's included; `the_third_stage_wants_skill_or_better_gear` pins the
masher's losses with the training sword and the adept's wins at tier 4.

---

## 11. Stages 4 to 7: seventeen bodies and their legends

The brief: seventeen creatures from Japanese, Turkish and Turkic, and Norse folklore, sorted into
four stages, their powers and attacks drawn from their legends. Every body is still a breed of an
archetype, and so attacks one of the three ways (§10). Each also brings its legend in three
parts, systematic as the archetypes are:

* **A move of its own**: a variation on its archetype's verb. It is a bite that shoves, a bolt out
  of nowhere, or a slam that shakes the ground. It is frame data like any other move, held to the
  same design limits.
* **A rider**: what its blows carry besides their damage. No rider slows him, holds him, roots him
  or drags him: a legend hurts, dooms or shoves, and nothing slows his walk, not even a landed
  blow's hitstun (`ails` keeps no pace of its own; `Fighter::stride`).
* **A power**: one rule from its legend on top of the frame data.

### 11.1 The stages

| Stage | Theme | Melee | Body | Distance | Bouts (their kit's tier) |
|---|---|---|---|---|---|
| 4 The yōkai | Japan's folk demons | oni | kappa, rokurokubi | kitsune | The Iron Club, The Fox's Wedding (3) |
| 5 Anatolian nights | Turkish and Turkic spirits of the dark | karakoncolos | tülükuş, albastı | cin, arçura | The Call in the Cold, Zemheri Nights (5) |
| 6 The legends | two giants, a dragon and a ghost | Tepegöz, gashadokuro | yedi başlı ejder | goryō | The One Eye, The Old Tales (7) |
| 7 Ragnarök | the Norse end of the world | frost giant, Surtr | Níðhöggr | Hel | Fire and Ice, Ragnarök (9) |

Each stage's foes are tougher and hit harder than the stage before's, on average
(`every_stage_is_stronger_than_the_last_and_every_breed_inside_its_archetypes_limits`). The
giants' arms reach as far again as their girth outgrows a man's (`Breed::reach`). They are
sculpted at a coarser voxel, 0.02 to 0.025, and stand two and a half to three tiles tall.

### 11.2 The legends, one rule each

| Body | The legend | In the ring |
|---|---|---|
| Oni | *oni ni kanabō*: an iron club in an oni's hand makes the strong unstoppable | **Kanabō**: a slam no blow interrupts (armor `UNSTOPPABLE`; only the parry stops it), and the ground shakes all round it: 45% of the blow out to 2.5 tiles |
| Kappa | the dish of water on its head is its strength; bow to it and it bows back, and spills it | **Sumo Shove**: a palm thrust from beside him that shoves him 2.2 tiles back (1.1 through a block). Parried, it bows and its dish spills: for 6 s it takes ×1.75 damage at 60% of its pace |
| Rokurokubi | by night her neck grows long | **Neck Bite**: beside him, her neck bows forward a little way and her head bites; it never leaves her neck, nor her neck its length |
| Kitsune | foxfire (*kitsune-bi*), and illusions | **Foxfire**: three bolts in a fan, one strike to the threat manager. Wounded below 70% and again below 35%, it conjures two illusions and swaps places with one of them. One blow pops an illusion, its foxfire is harmless, and it casts no light |
| Karakoncolos | on the coldest nights it calls people out by name in a loved one's voice | **The Call**: it roars his name; heard within its 7 tiles as the roar begins, it dooms him as the owl's cry does (his next wound ×1.3, for 4 s). He is not drawn in |
| Tülükuş | an owl's cry foretells a death; it flies without a sound | **Talons**: beside him it rises straight up on its wings, out of reach for frames 4-21 of its startup, and drops straight down talons first. Its cry as it takes wing dooms him: his next wound ×1.3 |
| Albastı | the night hag who sits on a sleeper's chest | **Smother**: she rears up beside him and falls on him with all her weight (24); no pin, no leap. She is a shadow with red eyes until she rears up |
| Cin | made of smokeless fire, seen only when it wishes; *cin çarpması*, struck by a cin | **Smokeless Fire**: a bolt out of nowhere, an ordinary hit (only the usual hitstun). Come within 2.6 tiles and it vanishes in smoke, to stand 6 tiles off (once every 5 s) |
| Arçura | grows as tall as a tree or as small as a blade of grass; guards the forest | **Root Dart**: a thorny seed flung at him like any bolt; roots burst up where it lands (18); he stays free. It grows to cast, and slips away half again as fast, shrunk to the grass |
| Tepegöz | his mother the peri's ring made his body proof against blade and arrow; Basat struck his one eye | **The Ring**: 15% of every blow, damage and poise alike, unless his eye is within reach: in his recovery, or reeling. Then the whole blow lands, and his eye lights up |
| Gashadokuro | the skeleton of the starved: a ringing in the ears, then its hands | **Grab**: two hands no shield holds (parry them, or leap); they crush him, the whole 30 at once, and let go. A bell is its tell |
| Yedi başlı ejder | the seven-headed dragon on the town's spring; felled, it begs *"Bir daha vur!"* | **Seven Heads**: four bites from beside him, its necks their own length (one strike; a parry stops them all). Felled, it begs for 150 frames: a second blow brings it back at a seventh of its health (a head grown again); restraint kills it ("my mother bore me but once") |
| Goryō | the wrath of a wronged noble: Michizane's struck the palace as lightning | **Thunderbolt**: a bolt of lightning flung at him, coming down as lightning where it lands. Nothing touches it but while it rages (in its moves) |
| Frost giant | the rime-giants of Jötunheim | **Rime**: its blows frostbite, even through the shield: 2 health every half second for 3 s (struck again, the 3 s start over). It never slows him. **Frost Slam**: both fists down beside him, and ice bursts from the floor as far as they reach |
| Surtr | the fire giant of Muspelheim, whose sword burns the world | **Fire**: his slam and sweep leave fire on the floor for 3 s: 5 health every 20 frames he stands in it |
| Níðhöggr | the dragon gnawing the roots of Yggdrasil from below | **Burrow**: into the floor, along under it (out of reach) and up beneath him on its mark. Its bites are venom: 3 health every half second for 2 s |
| Hel | Loki's daughter, half living and half corpse, queen of the dead | **Raise the Dead**: two draugr climb out of the floor about him; they fall when she does. Her bolts drain: what they deal, she heals |

### 11.3 Frame data of the legends' moves

| Move | Startup | Active | Recovery | Damage | Shape | Rider |
|---|---|---|---|---|---|---|
| Sumo Shove | 22 | 6 | 28 | 18 | cone 1.2, from ≤ 1.4 | shove 2.2 |
| Neck Bite | 26 | 6 | 30 | 20 | cone 1.2, from ≤ 1.3 | |
| Foxfire | 34 | 1 | 32 | 9 ×3 | volley of three, ±0.28 rad | |
| Kanabō | 46 | 6 | 52 | 32 | cone; quake 2.3 at 45% | (unstoppable) |
| Talons | 30 | 6 | 34 | 26 | cone 1.2, from ≤ 1.3 (aloft f4-21) | |
| Smother | 26 | 6 | 34 | 24 | cone 1.2, from ≤ 1.3 | |
| Smokeless Fire | 36 | 1 | 30 | 14 | bolt | |
| Root Dart | 38 | 1 | 30 | 18 | bolt (roots where it lands) | |
| The Call | 30 | 45 | 12 | | spell | (doom) |
| Grab | 40 | 8 | 44 | 30 | cone 2.0, unblockable | |
| Seven Heads | 30 | 24 | 34 | 11 ×4 | cone 1.4, from ≤ 1.4, four bites | |
| Thunderbolt | 40 | 1 | 32 | 22 | bolt (lightning where it lands) | |
| Frost Slam | 40 | 6 | 44 | 22 | cone 1.8, from ≤ 1.7, armor 35 | frost |
| Burrow | 60 | 6 | 40 | 26 | mark 1.0 | venom 120 |
| Raise the Dead | 50 | 1 | 40 | | spell | raise 2 |

Every move a foe may use is in `MoveId::FOE` and held to §0. Each is readable (18 frames or more),
punishable (recovery past L1's 10), and parried on reaction (17 or more). Two new limits:

* **What cannot be guarded gives time to step out of it.** A burrow's mark follows the hero at
  `MARK_FOLLOW` (2.6 tiles a second, half his pace) through its tracking frames, then holds. A
  hero who sets out on seeing it clears its radius and his own with a fifth to spare
  (`what_cannot_be_guarded_gives_time_to_step_out_of_it`).
* **A flurry's bites are spread over its active frames.**

A move's range and angle to begin it are now in its data (`range`, `off`, `line`), and so is its
rest between uses (`cooldown`). A foe takes the first move it knows whose conditions hold, so the
lab's choices are the same table as before.

### 11.4 What the guard can do

* **Guarding:** most blows are blocked or parried. A grab is parried only (`GuardRule::ParryOnly`).
  A burrow's mark on the floor is neither (`GuardRule::Bare`): he walks out of it, or leaps. The leap is airborne, and every blow passes under it.
* **Nothing holds him:** no legend pins, grabs, roots, freezes, stuns past the ordinary hitstun,
  or drags him. (The pin, the grab's hold, the roots, the freeze, the long stun and the call's
  pull were cut: the bites they dealt went into the blows.)
* **Counting strikes:** a volley lands as one strike, and so does a flurry; neither counts as an
  overlap with itself.
* **The ring's edge:** no shove carries him out of a bout (`carry_hero`).
* **His ails** (`combat::ails`) are each a clock in frames: frostbite, doom, venom, the fire
  underfoot, shoved. None of them touches his pace. They show about him (frost's motes, a dark
  sigil, venom's bubbles) and in a line on screen.

### 11.5 The breeds

Multiples of the archetype (Brute 200 health, Skirmisher 80, Hexer 55):

| Kind | Archetype | Health | Damage | Poise | Pace | Radius | Cost |
|---|---|---|---|---|---|---|---|
| Oni | Brute | 1.9 | 1.35 | 1.25 | 1.0 | 0.6 | 2 |
| Kappa | Skirmisher | 1.9 | 1.35 | 1.4 | 0.95 | 0.42 | 1 |
| Rokurokubi | Skirmisher | 1.7 | 1.4 | 1.2 | 0.7 | 0.4 | 1 |
| Kitsune | Hexer | 2.0 | 1.35 | 1.5 | 1.3 | 0.38 | 1 |
| Karakoncolos | Brute | 2.3 | 1.35 | 1.25 | 1.15 | 0.6 | 2 |
| Tülükuş | Skirmisher | 3.8 | 1.5 | 1.3 | 1.1 | 0.42 | 1 |
| Albastı | Skirmisher | 3.4 | 1.45 | 1.4 | 1.05 | 0.42 | 1 |
| Cin | Hexer | 2.6 | 1.5 | 1.6 | 1.0 | 0.38 | 1 |
| Arçura | Hexer | 3.8 | 1.45 | 1.8 | 0.95 | 0.45 | 1 |
| Tepegöz | Brute | 4.0 | 1.7 | 1.25 | 0.85 | 0.85 | 2 |
| Gashadokuro | Brute | 3.0 | 1.65 | 1.25 | 0.9 | 0.9 | 2 |
| Yedi başlı ejder | Skirmisher | 5.0 | 1.6 | 1.6 | 0.75 | 0.9 | 2 |
| Goryō | Hexer | 4.2 | 1.7 | 2.0 | 1.0 | 0.4 | 1 |
| Frost giant | Brute | 3.6 | 1.9 | 1.25 | 0.9 | 0.9 | 2 |
| Surtr | Brute | 4.0 | 2.0 | 1.25 | 0.85 | 0.95 | 2 |
| Níðhöggr | Skirmisher | 5.0 | 1.7 | 1.6 | 0.85 | 0.8 | 2 |
| Hel | Hexer | 4.0 | 1.9 | 2.0 | 1.0 | 0.42 | 1 |

Every brute still flinches from the finisher in neutral (armor under 45), and every skirmisher is
still slower than the hero.

### 11.6 Results (`--stage S 3`: Ragnar's kit at the stage's tier, two below and three above)

```
bout                 tier  skill   win   ttk(s)  target    taken  hurt  parry block overlaps
THE IRON CLUB           1  novice     0%     0.0      -        140   3.0    0.0   0.0    0
THE IRON CLUB           1  adept      0%     0.0      -        148   7.0    7.7  16.3    0
THE IRON CLUB           1  expert   100%    26.0    14-30        9   0.3   18.0   0.0    0
THE IRON CLUB           3  novice     0%     0.0      -        177   4.0    0.0   0.0    0
THE IRON CLUB           3  adept    100%    57.4      -         32   1.3    5.7  20.0    0
THE IRON CLUB           3  expert   100%    18.4    14-30        8   0.3   11.3   0.0    0
THE IRON CLUB           6  novice     0%     0.0      -        153   3.0    0.0   0.0    0
THE IRON CLUB           6  adept    100%    18.3      -          0   0.0    1.0   7.7    0
THE IRON CLUB           6  expert   100%     7.8    14-30       60   2.0    4.0   0.0    0
THE FOX'S WEDDING       1  novice     0%     0.0      -        109   4.0    0.0   0.0    0
THE FOX'S WEDDING       1  adept      0%     0.0      -        144   6.0    1.3   8.3    0
THE FOX'S WEDDING       1  expert   100%    49.6    25-50       78   3.3   41.3   0.0    0
THE FOX'S WEDDING       3  novice     0%     0.0      -        182   5.0    0.0   0.0    0
THE FOX'S WEDDING       3  adept      0%     0.0      -        195   8.7    5.3  25.0    0
THE FOX'S WEDDING       3  expert   100%    33.6    25-50       37   2.0   24.0   0.0    0
THE FOX'S WEDDING       6  novice     0%     0.0      -        156   5.0    0.0   0.0    0
THE FOX'S WEDDING       6  adept    100%    41.3      -         13   1.0    3.0  29.0    0
THE FOX'S WEDDING       6  expert   100%    16.0    25-50       30   2.3   11.7   0.0    0
THE CALL IN THE COLD    3  novice     0%     0.0      -        169   3.0    0.0   0.0    0
THE CALL IN THE COLD    3  adept    100%    63.7      -         33   0.7    1.0  16.7    0
THE CALL IN THE COLD    3  expert   100%    29.0    14-30       16   0.3    6.7   0.0    0
THE CALL IN THE COLD    5  novice     0%     0.0      -        189   3.0    0.0   0.0    0
THE CALL IN THE COLD    5  adept    100%    24.3      -         81   1.3    0.7   5.0    0
THE CALL IN THE COLD    5  expert   100%    20.8    14-30       20   0.3    4.3   0.0    0
THE CALL IN THE COLD    8  novice   100%     6.3      -        110   2.0    0.0   0.0    0
THE CALL IN THE COLD    8  adept    100%    18.1      -         13   0.3    0.7   2.3    0
THE CALL IN THE COLD    8  expert   100%     9.4    14-30        0   0.0    1.3   0.0    0
ZEMHERİ NIGHTS          3  novice     0%     0.0      -        166   5.0    0.0   0.0    0
ZEMHERİ NIGHTS          3  adept      0%     0.0      -        185   6.0    7.0  25.3    2
ZEMHERİ NIGHTS          3  expert     0%     0.0    30-60      199   5.3   14.0   0.0    0
ZEMHERİ NIGHTS          5  novice     0%     0.0      -        168   5.0    0.0   0.0    0
ZEMHERİ NIGHTS          5  adept    100%    54.4      -         38   1.3    4.0  17.3    0
ZEMHERİ NIGHTS          5  expert   100%    39.2    30-60       44   1.7   13.3   0.0    0
ZEMHERİ NIGHTS          8  novice     0%     0.0      -        152   5.0    0.0   0.0    0
ZEMHERİ NIGHTS          8  adept    100%    35.8      -         56   1.7    4.7  13.0    0
ZEMHERİ NIGHTS          8  expert   100%    26.2    30-60       54   2.0   11.3   0.0    0
THE ONE EYE             5  novice     0%     0.0      -        210   3.0    0.0   0.0    0
THE ONE EYE             5  adept      0%     0.0      -          9   0.3    4.0  41.3    0
THE ONE EYE             5  expert   100%    23.2    10-24        0   0.0   11.0   0.0    0
THE ONE EYE             7  novice     0%     0.0      -        210   3.0    0.0   0.0    0
THE ONE EYE             7  adept    100%    73.2      -          0   0.0    0.0  31.0    0
THE ONE EYE             7  expert   100%    14.3    10-24        0   0.0    5.0   0.0    0
THE ONE EYE            10  novice     0%     0.0      -        322   5.0    0.0   0.0    0
THE ONE EYE            10  adept    100%    44.3      -         19   0.3    0.0  20.0    0
THE ONE EYE            10  expert   100%    10.6    10-24        0   0.0    3.0   0.0    0
THE OLD TALES           5  novice     0%     0.0      -        189   4.0    0.0   0.0    0
THE OLD TALES           5  adept      0%     0.0      -         56   1.0    3.0  32.3    0
THE OLD TALES           5  expert   100%    57.2    30-60       68   1.7   18.7   0.0    0
THE OLD TALES           7  novice     0%     0.0      -        166   4.0    0.0   0.0    0
THE OLD TALES           7  adept     33%    95.8      -         48   1.3    6.7  29.3    1
THE OLD TALES           7  expert   100%    48.8    30-60       78   2.3   16.7   0.0    0
THE OLD TALES          10  novice     0%     0.0      -        330   7.0    0.0   0.0    0
THE OLD TALES          10  adept    100%    79.6      -         18   0.3    4.0  22.0    0
THE OLD TALES          10  expert    67%    39.0    30-60       63   3.3   15.0   0.0    0
FIRE AND ICE            7  novice     0%     0.0      -        204   4.0    0.0   0.0    0
FIRE AND ICE            7  adept     33%    60.4      -        176  22.7    1.0  16.0    0
FIRE AND ICE            7  expert   100%    30.8    20-40      110  11.7   10.3   0.0    0
FIRE AND ICE            9  novice     0%     0.0      -        240  12.0    0.0   0.0    0
FIRE AND ICE            9  adept    100%    52.6      -        137  18.3    0.3  16.0    0
FIRE AND ICE            9  expert   100%    32.1    20-40      153  13.7   12.3   0.0    0
FIRE AND ICE           10  novice     0%     0.0      -        300  18.0    0.0   0.0    0
FIRE AND ICE           10  adept    100%    39.6      -         92   9.7    1.7  10.0    0
FIRE AND ICE           10  expert   100%    29.0    20-40       90  11.7   10.3   0.0    0
RAGNARÖK                7  novice     0%     0.0      -        173   9.0    0.0   0.0    0
RAGNARÖK                7  adept      0%     0.0      -        196  22.3    3.7  21.7    0
RAGNARÖK                7  expert   100%    53.3    30-60      127  10.0   19.7   0.0    0
RAGNARÖK                9  novice     0%     0.0      -        285  18.0    0.0   0.0    0
RAGNARÖK                9  adept    100%    85.5      -        158  20.7    6.0  27.0    0
RAGNARÖK                9  expert   100%    50.9    30-60      187  14.0   20.7   0.0    0
RAGNARÖK               10  novice     0%     0.0      -        329  16.0    0.0   0.0    0
RAGNARÖK               10  adept    100%    72.2      -        211  25.7    5.7  27.0    0
RAGNARÖK               10  expert   100%    41.8    30-60      307  17.7   19.3   0.0    0
```

* **At its stage's tier**, the expert wins every bout inside its target and a masher none.
* **Three tiers up (or the last tier)**, a careful player wins them all.
* **Zero overlaps** at each bout's tier, for every skill.

These are pinned by `every_later_stage_wants_skill_and_its_kit` and
`the_bouts_meet_their_balance_targets` (the expert with each bout's tier). The legend tests in
`ring_tests` play each rule through the game. They cover the spilled dish and the shove through a
block, the tremor behind the oni, the pin struggled short, the plea (a second blow revives, restraint
kills), the ring and the eye, the stun, roots and freezing, fire and the dead falling with Hel, and
the illusions and the call. `duelist` tests the burrow's mark following and holding, the burrow
and the owl out of reach, every body foe biting from beside him and staying where it bit, the
cin's vanishing, and a giant's reach.

### 11.7 What changed, and why

| Change | Why |
|---|---|
| The expert bot counts how long a press really commits it (the rest of its swing to the cancel frame, and the next swing's) before pressing again | a slow folklore blow coming in mid-string (the neck lash, Surtr's slam) caught it in its own swing |
| The bot counts a lane's head down through the active frames | it read the rokurokubi's head as 7 frames away all the way in, and never raised its guard |
| The bot guards what lands first, then walks out of a mark; when both come at once, it leaps | walking out of the owl's mark, it took the karakoncolos's slam |
| The bot's dodges and its walk away from the call keep to the ring | it walked off the ring, and the bout was called off |
| The owl's doom ×1.5 → ×1.3; the karakoncolos's damage 1.5 → 1.35 | doomed, one slam took three quarters of a tier-8 kit's health |
| Tepegöz's health 2.6 → 4.0; his bout's target 10-24 s | the expert killed him in 9 s at tier 7: no duel at all |
| Níðhöggr's damage 1.9 → 1.7; venom 4 → 3; Surtr's fire 6 → 5 | with the last tier, a careful player lost Ragnarök one time in three |

The expert's new care moved the lab's times a little (Steel 25.4 → 26.6 s, Fang and Fire 12.9 →
11.5 s, the Night Parade 35.9 → 36.0 s), all still inside their targets.

**Tools:**
* `--stage S N` plays a stage's bouts with every skill at three kit tiers.
* `--bout K SKILL TIER SEED` plays one bout. With `VH_TRACE=1` set, it prints every blow and wound on the hero as it comes: who struck, with what, how it was met, and his state.

---

## 12. Close bites, bolts from afar, and no hit-lock

Playtesting found the Skirmisher's lunge a rubber band: a lane on the floor, a dart down it at
the hero, and a spring back to where it stood. A guard broken by it held him in place. The rules
now:

* **No hit-lock.** No foe's blow slows him or holds him. Struck, blocking, or with his guard
  broken, he walks at his whole pace; the reel only keeps him from swinging or guarding for its
  frames (`Fighter::stride`; `no_foes_blow_slows_him_or_roots_him_struck_blocking_or_with_his_guard_broken`).
  The damage, the flash, the floater and the hitstop stay. What still slows him: the bog slime's
  trail, the realms' own ground (mud, thorns, Bloat, a purge's stun), and his own raised shield.
* **Body foes bite up close.** Every Skirmisher breed bites from beside him (`BITE_RANGE`) and
  stays where it bit: no lane, no travel, no spring back, no sidestep hop. The folklore moves
  follow: the kappa's shove, the rokurokubi's neck bite (her neck keeps its length), the owl's
  talons (straight up, straight down), the albastı's weight, the ejder's heads. Níðhöggr's
  burrow, which goes under the floor and comes up beneath him, stays.
* **Only distance foes strike from afar, and only with bolts.** The arçura's roots and the
  goryō's lightning became bolts (the roots and the lightning now burst where the bolt lands).
  The frost giant's wave became a close slam. Every caster's bolt has its own look (`BoltLook`):
  the slime's dripping green glob, the witch's violet star in a ring, the jorogumo's ball of fire,
  the kitsune's foxfire wisps on long tails, the cin's tall pale flame with no trail, the arçura's
  thorny dart shedding leaves, the goryō's jagged white-blue lightning, Hel's dark drain circled
  by cold motes; his own thrown blade spins gold
  (`every_distance_foes_bolt_has_its_own_look`,
  `a_body_foe_only_bites_from_beside_him_and_only_a_distance_foe_strikes_from_afar`).

| Change | Why |
|---|---|
| Lunge (18/8/22, 18, 2.6 travel) → Bite (18/6/24, 20, reach 1.1 from ≤ 1.3); the backstep and the sidestep are gone | the rubber band; the bite is easier to step away from, so it bites a little harder |
| Skirmisher hold ring 4.2 → 3.2 | a biter must walk all the way in; from the old ring it held its token so long that the brutes waited, and the Night Parade ran to 46 s |
| Tülükuş health 2.2 → 3.8, Albastı 2.4 → 3.4, Arçura 2.8 → 3.8, Ejder 4.0 → 5.0, Goryō 3.0 → 4.2 | their blows are parried now, and a parry opens them: the Call in the Cold fell to 12.8 s, the Old Tales to 27 s |
| The bot reckons a bolt 2 frames earlier than its flight says | its bolt parries came on guard frames 1-4 against a blow's 3-7; with more bolts about it lost the Old Tales |
| A bolt that misses its moment by more than the slack burns out | a root dart dodged and then walked into came 23 f late, into the cin's slot: an overlap |

Measured with the expert at each bout's tier (3 seeds, `--bout`): the Brute 11.6 s, the Pack 8.0,
the Hexer's Guard 5.0, the Trio 15.9, the Gauntlet 23.3, Steel 26.6, Fang and Fire 14.2, the
Night Parade 40.0, the Iron Club 17.3, the Fox's Wedding 30.8, the Call in the Cold 15.3, Zemheri
Nights 34.6, the One Eye 14.3, the Old Tales 30.9, Fire and Ice 29.5, Ragnarök 37.9: all inside
their targets, no overlaps. Open: with the sidestep gone a masher beats the Pack (two loup-garous)
again; no test pins it.

---

## 13. Phase 5: the princes, a fourth archetype

The boss is not a special case. A sin's prince is a duelist like every foe: the same `Fighter`,
frames, guard, parry, mercy and threat rules, his own `Archetype::Prince` with its numbers, and a
`Breed` that carries the phase of his fight (`combat::prince`). Three things make him a boss:

* **Phases by health.** Above two thirds he fights with one table of moves, down to a third with a
  second, below it with a third (`prince::table`). A move's cooldown is its own; the tables change
  as a blow takes him past a threshold (`Cue::Phase`), and the game shows it: a banner, his roar,
  a green cross for a breather.
* **Armour windows.** His hide turns 70% of a blow, damage and poise alike, outside his recoveries
  (`Power::GoldHide`, as Tepegöz's ring: `HIDE_SHARE` 0.3). In a recovery, reeling, or while his
  fist holds the hero, the whole blow lands, with the ×1.25 of a punish. No blow
  interrupts him through his hide, not even the finisher (armour 40); a parry does (deflected
  60 f, half his poise).
* **A token of three.** He fills his court's pool by himself (capacity 3, a grant every 45 f):
  while he strikes, what he raises waits.

### 13.1 Mammon, the prince of Greed

| | Prince (Mammon) |
|---|---|
| HP / Poise / Armor | 900 / 150 / 40 (hide: 30% of a blow outside his recoveries) |
| Stagger / Resolve | 90 f / 300 f |
| Walk / Strafe (t/s) | 2.0 / 1.3 (the Hoard: ×1.35) |
| Rest after a move | 50 f (the Hoard: 30 f) |
| Radius / arm | 0.95 / +0.9 tiles of reach |
| Cost | 3 |

| Phase (health) | Moves, in the order he prefers them | What else |
|---|---|---|
| 1 The Banker (100-66%) | Coin Toss (from 2.4-7 tiles), Sceptre Sweep | |
| 2 The Collector (66-33%) | Grasp, Coin Toss, Sceptre Sweep | two gold golems climb out of his heaps (once) |
| 3 The Hoard (< 33%) | Coin Rain, Hoard Slam, Coin Volley (3-8 tiles), Sceptre Sweep | quicker on his feet, quicker to strike again |

| Move | Startup | Active | Recovery | Damage | Shape | Rider |
|---|---|---|---|---|---|---|
| Sceptre Sweep | 32 | 8 | 42 | 24 | cone 1.3 (+0.9), ±105° | |
| Coin Toss | 30 | 1 | 36 | 18 | five coins on the floor round him (one where he stands), 0.8 wide, bursting 16 + 48 f after the throw | |
| Grasp | 36 | 8 | 46 | 10 | cone 1.4 (+0.9), ±45°, parry only | the hold |
| Coin Volley | 34 | 1 | 32 | 11 ×5 | volley of five, ±0.42 rad | |
| Coin Rain | 40 | 1 | 34 | 16 | seven coins falling round him (two rings), 0.85 wide, bursting 16 + 42 f after | |
| Hoard Slam | 42 | 6 | 50 | 32 | cone 1.3 (+0.9); quake 2.4 (+0.9) at 45% | |

Every move holds to §0: readable (30 f or more), punishable, parried on reaction where it can be
parried. The coins are `Shape::Traps`: what bursts from the floor is neither blocked nor parried
(`GuardRule::Bare`); he steps off them, or leaps. A throw's coins burst together and land as one
strike, on the frame the threat manager planned (`lands_after`: startup, flight, fuse), and the
fuse leaves time to walk off them with a fifth to spare
(`a_coin_on_the_floor_bursts_late_enough_to_step_off_it`).

**The grasp** is the one thing in the game that holds the hero, and it holds only his feet. A
grasp that lands open (not parried, not leapt) closes his fist on him: Mammon keeps still in the
grasp's recovery, his hide open; the hero cannot walk or leap, but he can swing and guard. Every
30 frames the fist takes 10 of his silver and gives Mammon 1.2% of his health back. A blow on
Mammon breaks it (and lands whole, as a punish); after 300 frames he lets go of himself. Nothing
else may strike while he holds (the threat manager is kept quiet). This bends §11.4's rule that
nothing holds him, deliberately: the legend is "steals your silver unless you strike him", and a
hero who strikes at once is held for a third of a second.

### 13.2 Results

The bout (the ring, `--bout 17 SKILL TIER SEED`; the expert's kit at the bout's tier, 2):

| Skill | Tier | Win | Time (s) | Taken |
|---|---|---|---|---|
| novice | 1 | 0/3 | lost in 8.3 | 118 |
| novice | 2 | 0/3 | lost in 10.8 | 162 |
| adept | 1 | 0/3 | timed out (120) | 82 |
| adept | 2 | 0/3 | timed out (120) | 52 |
| expert | 1 | 3/3 | 84.9 | 18 |
| expert | 2 | 3/3 | 69.0 (target 45-90) | 0 |

His court (the Counting House, three lives, `--court SKILL TIER SEED`): the expert brings him down
through all three phases at tiers 1 and 2 without falling (100-133 s with the walking, the golems
and the coins); a masher loses all its lives in his first phase (about 46 s). The adept blocks
everything and parries little, and his hide turns what it strikes outside his recoveries: it does
not finish him in two minutes. Pinned by `the_bouts_meet_their_balance_targets` (the expert, no
overlaps; a masher cannot win) and `the_expert_beats_mammon_in_his_court_and_a_masher_does_not`.

| Change | Why |
|---|---|
| The court's pool 4 → 3 points | the golems' slams came while Mammon's own were planned; with his token filling the pool, they strike between his |
| The bots play the court from the top view | the stick is the camera's way round in the chase view: a bot pushing south walked north into the wall |
