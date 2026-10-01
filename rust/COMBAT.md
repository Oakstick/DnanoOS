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
  The Lunge (18 f) sits one frame above that floor: it's the hardest read in the game.

---

## 1. Frame data

### 1.1 Hero: the Training Sword (one chain, three hits)

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
| **Parry** | guard f2-f9 (8 f, 133 ms) | A hit landing in the window: no damage, no stamina. Attacker → *Deflected* 40 f and loses 50% of max poise. A hex bolt is **reflected** (×2 damage to foes). |
| Block | after f9 | Stamina − damage. Hero → *Blockstun* 6 + dmg/3 f. |
| Guard break | block with stamina < cost | 50% damage through, hero → *GuardBroken* 48 f, stamina → 0. |
| Evade (Space / A) | 25 f airborne + 6 f landing | The leap. **Airborne = i-frames** (f0-f24), then 6 f landing recovery. Cancels any startup frame, or recovery from its cancel frame. |
| Hit taken | hitstun 10 + dmg/2 f | Then **30 f mercy** i-frames, so two attacks can't stack into one combo. The formula of §2 applies to him too: a foe that catches his recovery deals ×1.25, a broken guard ×1.5. |
| Soft lock | at Began and the first active frame | The swing turns onto the nearest foe within its reach + 1 tile and 46° of his facing, and he turns with it. Four keys and a stick are coarse. |

### 1.3 Foes

| Move | Owner | Startup | Active | Recovery | Total | Dmg | Reach | Arc (±) | Track | Travel | Armor |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Slam | Brute | 42 (700 ms) | 6 | 48 (800 ms) | 96 | 30 | 1.8 | 35° | 30 f | 0 | 35 |
| Sweep | Brute | 30 (500 ms) | 8 | 36 | 74 | 20 | 1.6 | 110° | 16 f | 0 | 35 |
| Lunge | Skirmisher | 18 (300 ms) | 8 | 22 | 48 | 15 | 0.9 | 70° | 12 f | 2.6 tiles | 0 |
| Hex Bolt | Hexer | 36 (600 ms) | 1 | 30 | 67 | 12 | bolt 7 t/s, 90 f | — | 30 f | 0 | 0 |

* **Track**: for the first *track* startup frames the attacker turns toward the hero.
  After that its aim is **committed**: the last 12 f of a Slam and the last 6 f of a
  Lunge are locked, so a late sidestep beats them.
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
| Brute | 240 | 100 | Slam recovery 48 f fits L1+L2+L3 if the punish starts by f12. 85 poise per chain, so the 2nd punish breaks it; 90 f stagger at ×1.5 fits one full chain (≈ 52). | 4-5 punishes, **10-12 s** |
| Skirmisher | 70 | 25 | Any hit flinches it; L2 breaks it (40 ≥ 25); L3 lands ×1.5. | 2 chains, **3-4 s** once caught |
| Hexer | 45 | 20 | One chain + L1. The cost is reaching her. | **2 s** in reach |

Foe pressure (if every attack landed): Brute 30 per 96 f = **18.8 DPS**, Skirmisher 15 per ~136 f
≈ **6.6 DPS**, Hexer 12 per ~127 f ≈ **5.7 DPS**. With 3 threat points, the worst case is
Brute + one other ≈ **25 DPS** against 100 HP: about 4 s of zero-skill play.

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
the frame it **lands** on is ≥ 18 f from every other committed landing. For a blow that frame is
now + startup. For a hex bolt it is now + startup + flight (`duelist::lands_after`). Otherwise
it holds for a few frames. *No two foe attacks land within 300 ms of each other*, and that is a
test (`the_gauntlet_never_lands_two_strikes_inside_the_reaction_window`, a minute of five foes
against a hero who stands still).

Lessons from the lab, now in the rules:

* **Space by landing, not by release.** The first build spaced a hex by its cast. A bolt then
  arrived 10 f before a Brute's slam, and the hero's mercy frames swallowed the slam.
* **A token comes back, but its strike keeps its slot.** When a Hexer's recovery ends with its
  bolt still in the air, its threat point frees up while its landing frame keeps the others clear
  (`ThreatManager::release`, a *spent* hold).
* **The leash scales with the walk.** A flat 150 f timeout took the token back from a Brute
  still walking in. A token now lasts `max(150, eta + 60)` frames.
* **Telemetry counts real landings.** A blow lands on its first active frame, a bolt when it
  touches the hero. *Overlaps* counts landings inside the window, with one frame of slack for the
  hero's drift while a bolt flies.

### 4.2 Positional play: rings and sectors

Around the hero:

| Ring | Radius | Who |
|---|---|---|
| Engage | the move's range | the token holder only |
| Hold | Brute 3.6, Skirmisher 4.2 | melee foes without a token, strafing |
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
| HP / Poise / Armor | 240 / 100 / 35 | 70 / 25 / 0 | 45 / 20 / 0 |
| Stagger / Resolve | 90 f / 120 f | 50 f / 60 f | 60 f / 60 f |
| Walk / Strafe (t/s) | 1.8 / 1.2 | 4.6 / 2.8 | 2.2 / 2.0 (retreat 3.2) |
| Turn rate | 150°/s | 540°/s | 360°/s |
| Moves | Slam (front), Sweep (when flanked within 1.3) | Lunge from 1.6-3.2 tiles, then 30 f backstep | Hex Bolt from ≤ 7.5 tiles |
| Answer | evade the slam, punish the 48 f recovery, break it with L3 | parry the lunge (deflect 40 f) or punish its 22 f recovery | dash in between bolts; parry reflects |
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

* Losing restores the hero and restarts the bout. Winning moves on to the next.
* **F5**: frame readout (hero state and frame, buffer, threat points, foe states, bout stats).
* **F6**: skip to the next bout.
* Captures: `ring` (into the ring), `bout:N`, `readout`.

---

## 6. Roadmap

| Phase | Scope | Exit criterion |
|---|---|---|
| **0 Spec** | this document | numbers pinned by tests ✔ |
| **1 The Ring** | `logic::combat` (frames, FSM, buffer, formulas, threat manager, archetypes), the gym's Sparring Ring, telegraphs, HP/poise bars, frame readout, five bouts | builds, tests green, five bouts playable ✔ |
| 2 Tune | playtest the bouts; per-bout telemetry (TTK, damage taken, parries, blocks, evades, overlaps) on screen when a bout ends; change only the tables | TTK within ±20% of §2.1; zero overlaps; parry rate climbs with practice |
| 3 Hero kit | map blade tiers onto the move table (reach, arc, tempo scalars), shields onto guard stats (parry window, block cost), audit path skills against the FSM (cut what bypasses it) | every tier is the same three verbs with different numbers |
| 4 Roll-out | the realms' foes become archetypes (Helmeted → Brute-lite, Large Bat → Skirmisher-flier), the Vaults' packs run through the threat manager, contact damage retired outside the Garden's chase levels | the Vaults play as a Diablo-like brawl with readable cadence |
| 5 Bosses | Mammon from the same FSM: phase-specific move tables, armor windows, token cost 3 | the boss is a fourth archetype, not a special case |
