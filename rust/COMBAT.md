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
| Block | after f9 | Stamina − **0.7** × damage (the training buckler's 30% stability). Hero → *Blockstun* 6 + dmg/3 f. |
| Guard break | block with stamina < cost | 50% damage through, hero → *GuardBroken* 48 f, stamina → 0. |
| Evade (Space / A) | 25 f airborne + 6 f landing | The leap. **Airborne = i-frames** (f0-f24), then 6 f landing recovery. Cancels any startup frame, or recovery from its cancel frame. |
| Hit taken | hitstun 10 + dmg/2 f | Then **30 f mercy** i-frames, so two attacks can't stack into one combo. The formula of §2 applies to him too: a foe that catches his recovery deals ×1.25, a broken guard ×1.5. |
| Soft lock | at Began and the first active frame | The swing turns onto the nearest foe within its reach + 1 tile and 46° of his facing, and he turns with it. Four keys and a stick are coarse. |

### 1.3 Foes

| Move | Owner | Startup | Active | Recovery | Total | Dmg | Reach | Arc (±) | Track | Travel | Armor |
|---|---|---|---|---|---|---|---|---|---|---|---|
| Slam | Brute | 42 (700 ms) | 6 | 48 (800 ms) | 96 | 30 | 1.8 | 35° | 30 f | 0 | 35 |
| Sweep | Brute | 30 (500 ms) | 8 | 36 | 74 | 20 | 1.6 | 110° | 16 f | 0 | 35 |
| Lunge | Skirmisher | 18 (300 ms) | 8 | 22 | 48 | 18 | 0.9 | 70° | 12 f | 2.6 tiles | 0 |
| Hex Bolt | Hexer | 36 (600 ms) | 1 | 30 | 67 | 15 | bolt 7 t/s (time-locked), 90 f | — | 30 f | 0 | 0 |

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
| Brute | 200 | 100 | Slam recovery 48 f fits L1+L2+L3 if the punish starts by f12; a parried slam opens 60 f at ×1.5. 85 poise per chain, so the 2nd punish breaks it; 90 f stagger at ×1.5 fits one full chain (≈ 52). | 4 punishes, **10-12 s** (measured: 11.6 s) |
| Skirmisher | 80 | 25 | Any hit flinches it; L2 breaks it (40 ≥ 25); L3 lands ×1.5. It sidesteps the first swing it sees. | 2 chains, **3-4 s** once caught |
| Hexer | 55 | 20 | One chain + L1, or a parried bolt (22) and a chain. The cost is reaching her. | **2-3 s** in reach |

Foe pressure (if every attack landed): Brute 30 per ~140 f ≈ **13 DPS**, Skirmisher 18 per ~136 f
≈ **8 DPS**, Hexer 15 per ~127 f ≈ **7 DPS**. With 3 threat points, the worst case is
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
For a blow that frame is now + startup. For a lunge it adds the active frames it travels before
it is near enough. For a hex bolt it adds the flight (`duelist::lands_after`). Otherwise the foe
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
* **Plans keep slack.** A moving hero shifts landings by a few frames: a lunge closes sooner on a
  hero walking into it. Plans 24 f apart land 18 f apart.
* **Bolts are time-locked.** A bolt flies straight but paces itself (0.5× to 1.6× its speed) to
  touch the hero on its planned frame. A hero walking into it no longer brings it in early. A
  sidestep still dodges it.

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
| HP / Poise / Armor | 200 / 100 / 35 | 80 / 25 / 0 | 55 / 20 / 0 |
| Stagger / Resolve | 90 f / 120 f | 50 f / 60 f | 60 f / 60 f |
| Walk / Strafe (t/s) | 1.8 / 1.2 | 4.6 / 2.8 | 2.2 / 2.0 (retreat 3.2) |
| Turn rate | 150°/s | 540°/s | 360°/s |
| Moves | Slam (front), Sweep (when flanked within 1.3) | Lunge from 1.6-3.2 tiles, then 30 f backstep; **sidesteps** a swing begun within 2.4 tiles (12 f hop at 8 t/s, then 120 f before the next) | Hex Bolt from ≤ 7.5 tiles |
| Answer | evade the slam, punish the 48 f recovery, break it with L3 | don't open with a swing: parry the lunge (deflect 60 f) or punish its 22 f recovery | dash in between bolts; parry reflects |
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

* Losing restores the hero and restarts the bout. Winning moves on to the next.
* **F5**: frame readout (hero state and frame, buffer, threat points, foe states, bout stats).
* **F6**: skip to the next bout.
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
| 4 Roll-out | the realms' foes become archetypes (Helmeted → Brute-lite, Large Bat → Skirmisher-flier), the Vaults' packs run through the threat manager, contact damage retired outside the Garden's chase levels | the Vaults play as a Diablo-like brawl with readable cadence |
| 5 Bosses | Mammon from the same FSM: phase-specific move tables, armor windows, token cost 3 | the boss is a fourth archetype, not a special case |

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
