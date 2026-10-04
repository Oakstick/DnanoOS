# The Warband: enemy AI, rivals, generals, the two voices, the workshop and game feel

An implementation plan, written to be handed back and built phase by phase (§16). It builds on
what already exists: the frame data, the fighter state machine and the threat manager
(`COMBAT.md`), the realms and their `Dealer` (`ROADMAP.md`, `REALMS.md`), the four trees per
hero (`logic/progression.rs`) and the old man's wares (`game/shop.rs`). Nothing here replaces
the combat core. Everything here sits **above** it, as a layer that decides *who* fights,
*where*, *when* and *why*, and **around** it, as the feel, the sound and the voices.

Status: approved 2026-10-04 ("I like the plan... Lets go"); building. §17's proposals are taken as
the defaults until the user says otherwise. W1 (feel), W2 (voices) and the workshop's set (W8 art)
run in parallel worktrees while W0 is built on `rust-port`.

---

## 0. The goal: player flow

The design goal is flow. Each part of the plan has to serve one of these five, and §15 turns them
into numbers the harness measures.

| Pillar | What it means here | What serves it |
|---|---|---|
| **Readable challenges** | Every threat announces itself before it can hurt, to the eye *and* the ear. The player loses to what he misread, never to what he could not see. | Telegraphs (exist), tells by sound (§5), order cards and horns (§9), the prince's lies always carry a tell (§10) |
| **Responsive combat** | The hero obeys at once; the feedback lands on the same frame as the blow. | Hit pause, trauma, overshoot, layered sound (§4); no hit-lock (exists, kept) |
| **Meaningful decisions** | Choices with a visible price: fight the general now or later, break the horn or chase the herald, trust the voice or not, which elixir to bring for which rival. | Generals' instruments (§9), the two voices (§10), the workshop's counsel and trials (§12) |
| **Opportunities for mastery** | The enemy learns the player's habits, so habits must grow into skills. Every enemy pattern has a counter that can be learned. | Rivals' lessons (§8), pack plays with answers (§7), hero mastery ranks (§11) |
| **Pressure and recovery** | A fight breathes: build, peak, release, rest. Rest is where the player shops, listens and decides. | The Drum (§6), pack morale and the rout (§7) |

The player fantasy in one sentence: *I am a soul the guilty old gatekeeper armed and sent down,
and hell remembers me: its captains learn my name, its generals change their plans for me, and
its princes try to trick me. The old man never lies to me.*

---

## 1. What exists, and the rules this plan keeps

### 1.1 The baseline

| System | Where | Kept as |
|---|---|---|
| Frame data, `Fighter` FSM, input buffer | `logic/combat/{frames,fsm}.rs` | the unchanged core: no layer here edits a move's frames |
| Threat manager: tokens, cadence, commit gate, sectors | `logic/combat/threat.rs` | **the only authority on who strikes when**. Roles and plays only bias its requests and slots. |
| Archetypes and breeds (Brute / Skirmisher / Hexer / Prince) | `logic/combat/archetype.rs`, `prince.rs` | a role (§7) is a second axis on top of the archetype |
| Duelists, dens (sleep, wake, leash, return) | `logic/combat/duelist.rs`, `game/brawl.rs` (`update_dens`) | dens become packs with a leader and morale |
| `Dealer` deals foes by progress | `logic/realms/mod.rs` | it still picks the bodies; the pack composer (§7.4) gives them roles |
| Hitstop (global freeze), trauma shake, particles, debris, floaters, flashes | `game/flow.rs:177`, `render/view/effects.rs`, `game/feedback.rs` | extended in §4 |
| Procedural synth, ~60 `Sfx`, pitch/gain variance, `tell_sfx` | `audio/mod.rs`, `audio/synth.rs`, `game/brawl.rs:80` | extended in §5 |
| The old man's wares: 4 trees + loadout, the gateway crypt set | `game/shop.rs`, `render/hud/shop.rs`, `render/view/story.rs:2054` | rebuilt as the workshop (§12) |
| The princes' lies: Mammon's toll and one line per realm | `game/vault.rs`, `game/realms/*` | grown into the prince's cheats (§10.3) |
| The bot, the harnesses, the captures, the Map | `game/bot.rs`, `--crawl`, `--bout`, `capture.rs`, `game/map.rs` | every new system gets its harness, capture and Map stop |

### 1.2 The invariants (each is, or becomes, a test)

| # | Rule | Source |
|---|---|---|
| I1 | Every foe attack: startup ≥ 18 f, recovery ≥ 10 f, melee startup ≥ 17 f. | COMBAT.md §0 |
| I2 | No two strikes land within 18 f of each other (the commit gate). Every new hurt (hazards, reinforcements, rivals, generals) goes through the threat manager. | COMBAT.md §4.1 |
| I3 | No enemy slows, holds or drags the hero, except the bog slime's trail, the realms' own ground and Mammon's grasp. No general's hazard or rival's lesson may break this. | COMBAT.md §11.4, §12 |
| I4 | Body foes bite from beside him; only distance foes strike from afar, with bolts. No rubber band. | COMBAT.md §12 |
| I5 | The simulation is deterministic per seed. Presentation (juice, sound variance, barks) draws from **its own rng** and never touches the simulation's. | new: §3 |
| I6 | A layer may change *choices* (which move, which slot, when to ask), never *frame data*. The one exception is the time scale (§4.2), which scales everything evenly, so every frame relation holds. | new |
| I7 | Pressure never arrives unannounced: every reinforcement, order and hazard has a tell of ≥ 60 f (1 s) by eye and ear. | new: §6, §9 |
| I8 | The old man never lies; the prince's lies always carry a tell and a counter, and never take what the hero owns for good. | new: §10 |

---

## 2. Architecture: four layers of command

```
 ┌───────────────────────────────────────────────────────────────────────────┐
 │ THE DRUM (macro, seconds)        logic/war/drum.rs                        │
 │   reads the fight's intensity, schedules build / peak / release / rest     │
 │   bounds: threat capacity, cadence, wake range, when orders may fire      │
 ├───────────────────────────────────────────────────────────────────────────┤
 │ THE GENERAL (strategic, the realm)   logic/war/general.rs                  │
 │   issues orders (watch, muster, kindle, decree) through instruments       │
 │   the player can break; only when the Drum allows                          │
 ├───────────────────────────────────────────────────────────────────────────┤
 │ THE PACK (tactical, the room)        logic/war/pack.rs                    │
 │   roles, a leader, morale, plays (pincer, screen, rotate, crossfire...)    │
 │   output: slot preferences and token-request biases, nothing else         │
 ├───────────────────────────────────────────────────────────────────────────┤
 │ THE DUELIST + THREAT MANAGER (micro, frames)  logic/combat/*  (exists)    │
 │   frame data, FSM, commit gate: the last word on every strike              │
 └───────────────────────────────────────────────────────────────────────────┘
          ▲ reads                                   │ emits
          │                                         ▼
 THE LEDGER (rivals, across the run)        THE WAR LOG (events, §3)
 logic/war/ledger.rs                          → juice, sound, counsel, ledger,
                                                telemetry, the HUD
```

Each layer only talks *down* through a narrow interface, and every outcome goes *out* through one
event stream. That keeps the layers testable alone, as the threat manager is today.

### 2.1 New modules

| Module | Holds |
|---|---|
| `logic/war/mod.rs` | `WarEvent`, the shared types, `PresRng` (presentation rng) |
| `logic/war/drum.rs` | intensity, phases, the bounds it hands down |
| `logic/war/pack.rs` | `Pack`, `Role`, `Play`, morale, elite affixes, the composer |
| `logic/war/ledger.rs` | `Rival`, traits, memory, lessons, names, promotion |
| `logic/war/names.rs` | name and epithet tables per culture |
| `logic/war/general.rs` | `General`, `Order`, `Instrument`, the seven generals' tables |
| `logic/war/counsel.rs` | the old man's rules and lines |
| `logic/war/deceit.rs` | the prince's cheats: tell, counter, reward |
| `logic/war/mastery.rs` | the heroes' mastery ranks and deeds |
| `logic/juice.rs` | hitstop table, dilation, trauma sources (numbers only) |
| `game/war.rs` | the runtime glue: feeds the brawl, reads `WarEvent`s, owns the Drum/packs/general per level |
| `game/juice.rs` | applies `WarEvent`s to the time scale, camera, debris, sound |
| `game/workshop.rs` | the workshop's stations (wraps today's `ShopState`) |
| `audio/voices.rs` | per-breed voice sets, recipes, the layered hit builder |
| `audio/mix.rs` | priorities, voice limits, dedupe, ducking, a limiter, pan/attenuation |
| `render/view/workshop.rs` | the workshop set, three dressings |
| `render/view/spring.rs` | springs for overshoot |
| `render/hud/war.rs` | the general's banner, order cards, rival name cards, the voices' lines |
| `render/hud/ledger.rs` | the Ledger of Grudges board (workshop and pause) |

---

## 3. Phase W0's foundation: the war log

Today every branch of `brawl.rs` calls `audio.play`, `view.shake`, `floater` and `hitstop`
directly, and whiffs emit nothing. Every system in this plan needs to *know* what happened, so the
first job is one event stream.

```rust
pub enum WarEvent {
    Tell     { foe: u32, kind: EnemyKind, mv: MoveId, at: V },          // a move began (its startup)
    Commit   { foe: u32, mv: MoveId },                                  // tracking over, aim locked
    Landed   { foe: u32, mv: MoveId, how: Defense, dmg: f64, dir: V },  // reached the hero
    Whiffed  { foe: u32, mv: MoveId },                                  // active frames over, nothing met
    HeroHit  { foe: u32, effect: Effect, dmg: f64, finisher: bool, dir: V, matter: Matter },
    Broken   { foe: u32 },                                              // poise break
    Slain    { foe: u32, kind: EnemyKind, last_of_pack: bool, rival: Option<RivalId>, dir: V },
    Drank    { potion: PotionKind },
    Rally    { pack: PackId }, Rout { pack: PackId },
    Order    { general: Sin, order: OrderKind, stage: OrderStage },     // announced / live / broken
    Instrument { id: u16, broken: bool },
    RivalSeen { rival: RivalId }, Promoted { rival: RivalId, why: Deed },
    Whisper  { cheat: CheatKind }, SeenThrough { cheat: CheatKind },
    DrumPhase { phase: DrumPhase },
}
```

* `Brawl.log: Vec<WarEvent>` is filled during `brawl_frame` and drained once per game frame by
  `game::war` and `game::juice`. The direct calls move into the consumers, one branch at a time,
  with captures before and after (the feel must not change in W0).
* `PresRng`: a second rng for presentation only (I5). The harness runs with presentation off and
  must print the same numbers before and after W0, to the decimal.
* Test: `the_war_log_reports_every_strike_once` (each landing, whiff and parry appears once; the
  log's landings equal `BoutStats`).

---

## 4. Game feel: the juice

Six tools, each with a table of numbers in `logic/juice.rs`, each switched by `--fx-off`
(as RP07 does) so the perf and balance harnesses can run without them.

### 4.1 Hit pause (hitstop)

It exists as a global freeze. The plan makes it a **table**, with a **budget**, and fixes one drift.

| Event | Freeze (f, 60 Hz) | Note |
|---|---|---|
| Hero's light blow lands | move's own (3) | unchanged |
| Hero's finisher lands | move's own (6) | unchanged |
| Poise break | 8 | unchanged (`HITSTOP_BREAK`) |
| Parry | 8, plus 4 on a prince's move | the parry is the game's best moment; it should feel like it |
| Kill | 6 | unchanged |
| Last foe of a pack | 6 + dilation (§4.2) | the release beat |
| Rival or general slain | 12 + dilation | the trophy beat |
| **Hero is struck** | 3 | short: being hit must never feel sticky |
| Blocked | 2 | |
| Guard broken | 5 | |

* **Budget**: freezes add up to at most 14 f in any 60 f window; past it, each new freeze is
  halved. A sweep that meets four foes must not stall the game for half a second.
* **The input survives it**: the 8 f buffer counts only simulated frames, so a press made during a
  freeze comes out after it (verify in `fsm.rs`; test
  `a_press_during_hitstop_comes_out_when_it_ends`).
* **Fix**: the hero's slash pose is timed in seconds (`slash_t += dt`, `render/view.rs:748`) and
  runs on while the fighter is frozen. Drive it from the fighter's frame, as the foes' poses are.

### 4.2 Time dilation

New: `Game.time_scale`, a small stack of `Dilation { scale, hold, ease }`, applied where hitstop is
applied (`game/flow.rs:178`, scaling the `dt` handed to `tick`). `FrameClock` keeps the fraction,
so the brawl runs slower evenly and every frame relation holds (I6). The view eases on real time.

| Trigger | Scale | Hold | Ease back |
|---|---|---|---|
| Last foe of a pack slain | 0.35 | 0.20 s | 0.25 s |
| Rival or general slain | 0.2 | 0.45 s | 0.35 s |
| Prince's phase falls | 0.3 | 0.35 s | 0.3 s |
| Parry of a prince's move | 0.5 | 0.12 s | 0.15 s |
| A blow passes under his leap at < 15% health | 0.5 | 0.15 s | 0.2 s |

* At most one dilation every 3 s; none while a story or tour camera runs.
* During dilation the SFX bus drops in pitch by `scale^0.25` and the music is low-passed, so the
  ear feels it too.
* Settings: "Slow motion on kills" on by default.

### 4.3 Camera trauma

Exists as translation from trauma² (`effects.rs:132`, decay 1.9). The plan adds:

* **Rotation**: roll ±2.5° × trauma².
* **Directional kick**: an impulse along the blow's direction (`WarEvent.dir`), a spring back with
  overshoot (§4.5), separate from the noise. The player feels *where* the hit came from.
* **FOV punch**: −3° for 80 ms on a finisher or a break, +2° on a quake.
* **Sources and caps** (trauma added, capped per source):

| Source | Trauma | Cap |
|---|---|---|
| Hero struck | 0.22 + dmg/200 | 0.5 |
| Hero's finisher lands / break | 0.12 / 0.2 | 0.35 |
| Quake (kanabō, frost slam, hoard slam) | 0.45 × falloff over 6 tiles | 0.6 |
| Giant's footfall within 4 tiles | 0.05 | 0.1 |
| Muster horn | 0.15 | 0.15 |
| Rival/general slain | 0.4 | 0.4 |

* Settings: "Screen shake 0-100%" (accessibility), applied after the sum.
* Captures seed the noise, so shots stay comparable.

### 4.4 Kinetic debris

Debris today comes only from walls and bombs (`MAX_DEBRIS` 160). The plan adds matter to every
blow, and voxel shatter to every death: this is a voxel game, and breaking a foe into its voxels is
the most satisfying thing it can show.

* `Matter` per breed: Flesh (ichor drops), Bone (chips), Iron (sparks), Stone, Feather (slow fall,
  high drag), Smoke (rising, no bounce), Ember (glowing, lights the floor for 0.5 s), Ice (shards),
  Gold (coins that ring when they land), Slime (globs that stick and fade).
* **On a blow**: 4-10 chips from the struck side, thrown along the blade's arc plus the blow's
  direction, more on a finisher, a fan of sparks on a block (from the shield's rim).
* **On a death**: shatter the foe's coarse rig (`Rig::lod` quarter copy) into 24-60 cubes, part by
  part, each with the blow's direction × 3 t/s + radial 1.5 + up 2.5, gravity, restitution 0.3,
  friction, rest, then sink and fade over 2.5 s. Giants: 90 cubes and a dust ring.
* **Budget**: `MAX_DEBRIS` 160 → 480 as one instanced set (as the mist uses `InstanceSet`),
  ≤ 0.3 ms CPU at 480 in `--perf battle30`. Over budget, the oldest go first, then counts scale down.
* Variance: counts ±25% and spin from `PresRng`.

### 4.5 Animation overshoot

Poses ease with smoothstep and ease-out cubic today; nothing overshoots. `render/view/spring.rs`
adds a damped spring (`Spring { x, v, k, zeta }`, stepped on view time) and these uses:

| Use | Spring | What it shows |
|---|---|---|
| Foe flinch | kicked back along the blow, ζ 0.35, settles in 150 ms | weight |
| Hero's blade follow-through | past the end pose by 12%, back | the swing has mass |
| Foe anticipation | pulls back 8% in the first third of startup, held until commit | the tell reads in silhouette |
| Brute's slam impact | squash 0.85 on the first active frame, overshoot to 1.06 | the floor took it |
| Evade landing | squash 0.9, overshoot | the leap ends |
| Shield on block | knocked 10° back, ζ 0.4 | the block cost something |
| Death topple | the body tips along the blow before it shatters (6 f) | read who killed it and how |

Rule: the anticipation must still read as the tell. The pose is fully wound by 60% of startup, and
the commit flash (the last 8 f) is not hidden by any overshoot. Driven by the fighter's frame, not
seconds, so hitstop and dilation freeze it too.

### 4.6 Audio layering, tempo and variance

**Layering.** Every hit is built from up to five layers by `audio/voices.rs` (§5 gives the voices):

| Outcome | Transient | Body | Tail | Sweetener | Voice |
|---|---|---|---|---|---|
| Blow on flesh | click | thud by the foe's matter | short room | crit: high ring | foe's hurt cry |
| Blow on armor (no flinch) | tick | dull iron | — | — | — |
| Finisher | click ×2 | heavy thud | long room | low boom | foe's hurt cry |
| Poise break | crack | thud | — | glass ring | foe's stagger groan |
| Parry | bright ting | low thump | reverse whoosh | bell | hero's grunt |
| Block | clang by shield tier (wood, iron, steel, gilded) | — | — | — | — |
| Guard broken | crack | splinter | — | — | hero's gasp |
| Kill | crunch | matter's shatter | room | streak semitone (exists) | death cry |
| Hero struck | slap | thud | — | — | hero's hurt (one per hero) |

**Mixing** (`audio/mix.rs`, new): priorities (the token holder's tell first, then the hero's own
blows, then the rest), at most 3 of the same sound inside 40 ms (merged, louder), a 96-voice
budget for SFX, a ducking of everything else by 4 dB for 120 ms under a parry or a struck hero, and
a soft limiter on the master in place of the hard clamp. **Stereo pan and distance**: `play_at(sfx,
pos)` pans by the camera-relative bearing and falls off over 18 tiles; a tell from his back arc gets
+3 dB, because the eye can't help there.

**Tempo.** The recorded songs can't change tempo. Each track gets its BPM in `Soundtrack`, and a
synth **war-drum layer** plays over it, locked to that BPM, its density following the Drum (§6):
silent at rest, a heartbeat in build, full drums at peak, a single low hit on release. Muster horns
and general orders land on a bar line.

**Variance.** Pitch ±6% and gain (exist), plus 3 round-robin variants per voice, never the same
twice in a row, idle timings and breath per foe offset by `PresRng`, barks drawn without repeat.
None of it may touch the simulation (I5).

---

## 5. The voices of the bestiary: attack sounds by character

Today one `tell_sfx` maps 33 bodies onto about 15 shared sounds, and every impact is generic. The
plan gives every breed a **voice set** and keeps one rule above it: **the archetype is a family,
the breed is a voice.** The ear first learns the three ways of attack, then the faces.

| Family | Signature on every tell | Why |
|---|---|---|
| Melee (Brute) | low brass swell + a metal scrape as the weapon rises | slow, heavy: "wait" |
| Body (Skirmisher) | a sharp intake of breath, rising snarl | close and quick: "time it" |
| Distance (Hexer) | a rising shimmer that peaks on release | "close in now" |

```rust
pub struct VoiceSet {
    pub tell: Recipe,      // the startup cry, family signature + the breed's timbre
    pub swing: Recipe,     // weapon whoosh: blade, club, claw, jaws, wings, spell
    pub matter: Matter,    // its body's impact layer (§4.4)
    pub hurt: Recipe, pub stagger: Recipe, pub death: Recipe,
    pub alert: Recipe,     // waking in its den
    pub bark: BarkSet,     // rivals and generals only (§8, §9)
    pub size: f32,         // pitch scales down with size: giants an octave below
}
```

Recipes are short descriptions for the procedural synth (`Voice`: wave, envelope, filter, detune,
vibrato), so a new voice costs data, not samples.

| Stage | Breed | Tell (on top of the family) | Swing / release | Matter | Death |
|---|---|---|---|---|---|
| 1 | Draugr | rattling exhale, grave-cold | rusty blade whoosh | Bone | collapsing bones |
| 1 | Raven / night raven | harsh caw (night: lower, doubled) | wingbeat burst | Feather | caw cut short |
| 1 | Bog slime | bubbling gurgle | wet spit | Slime | splat and hiss |
| 2 | Witch | cackle rising into the shimmer | violet star chime | Flesh | shriek fading to wind |
| 2 | Witch's minion | squeak | small claw | Flesh | pop |
| 2 | Helmeted skeleton | helmet rattle, sword drawn | steel whoosh | Bone/Iron | armour clatter |
| 2 | Loup-garou | growl into howl-breath | claw rake | Flesh | whimper |
| 2 | Blemmyes | chest-mouth bellow | club whoosh, heavy | Flesh | belly groan |
| 2 | Large bat | ultrasonic chirp sweep | leathery wing | Flesh | squeal |
| 3 | Tengu | wind gust, "kaa" | twin steel | Feather | gust dying |
| 3 | Rasetsu | demonic double laugh | two blades, crossed ring | Flesh | roar choked |
| 3 | Dire wolf | deep growl | jaws snap | Flesh | yelp |
| 3 | Jorogumo | woman's hum into chitter | fireball whoomp | Flesh/Chitin | hiss |
| 3 | Cobra | rattle-less hiss, hood flare | strike snap | Flesh | hiss trails off |
| 4 | Oni | drum-chant grunt | kanabō: iron club whoosh, studs ringing | Flesh | earthshaking fall |
| 4 | Kappa | water slosh + "hah" | palm slap | Slime | dish spill |
| 4 | Rokurokubi | giggle, neck creak | neck whip | Flesh | sigh |
| 4 | Kitsune | fox bark into bell | foxfire whisper ×3 | Smoke | yip and bells |
| 5 | Karakoncolos | calls the hero's own name (per hero: Ragnar/Mehmed/Miyamoto) in a cold voice | fur-heavy swing | Flesh | howling wind |
| 5 | Tülükuş | owl's cry (dooming) | silence then talon rip | Feather | soft thud |
| 5 | Albastı | wheezing breath, close | weight-drop thump | Flesh | rattle |
| 5 | Cin | crackling flame, no source | flame snap | Smoke/Ember | flame gutters |
| 5 | Arçura | creaking wood, leaves | thorn dart whistle | Wood | timber crack |
| 6 | Tepegöz | single-eye roar, stone grind | boulder-arm sweep | Stone | eye bursts, rumble |
| 6 | Gashadokuro | ringing in the ears (rising tone) | bone hands, clatter | Bone | huge bone collapse |
| 6 | Yedi başlı ejder | seven hisses in a chord | heads snap in sequence | Flesh | "Bir daha vur!" plea |
| 6 | Goryō | thunder murmur, court flute | lightning crack | Smoke | thunder rolls away |
| 7 | Frost giant | glacier groan | ice-crack slam | Ice | ice shatter |
| 7 | Surtr | forge roar | burning sword roar | Ember | fire collapses |
| 7 | Níðhöggr | root-gnawing, earth groan | burrow rumble under | Flesh/Stone | roots snapping |
| 7 | Hel | two voices at once, alive and dead | drain whisper | Smoke | a long exhale |
| realms | Peerless, Ganglot, Gaki, Ellepige | from their realm's sound palette | — | — | — |
| prince | Mammon | coins poured, toll bell (exists), banker's laugh | sceptre ring | Gold | an avalanche of coins |

* The karakoncolos speaking the hero's name is a tell that is also a story beat: three recipes,
  one per hero, from the `Speech` synth (`Old`/`Beast` voices).
* Test: `every_breed_has_its_own_voice_and_its_familys_signature` (no two breeds share a tell
  recipe; each tell starts with its family's layer).

---

## 6. The Drum: pressure and recovery

The threat manager spaces strikes **inside** a fight (frames). Nothing spaces fights **across** a
level (seconds). The Drum does. It is not a difficulty cheat: it never changes a number of the
frame data, never spawns without a tell, and never refuses a fight the player picks.

### 6.1 Intensity

Every 30 f:

```
I = clamp( 0.35 · taken_5s / (0.25 · max_hp)       recent damage
         + 0.25 · engaged / 6                       foes hunting him
         + 0.20 · (1 − hp / max_hp)                  how low he is
         + 0.20 · landings_5s / 5 , 0, 1 )          strikes reaching him
```

### 6.2 Phases

```
   REST ──(player enters a den / a timer)──▶ BUILD ──(I ≥ 0.6)──▶ PEAK
     ▲                                                              │
     └──────(6-10 s)─── RELEASE ◀──(pack routed, cleared, or peak > 35 s)┘
```

| Phase | What the AI layers may do | Bounds handed down |
|---|---|---|
| **Rest** | nothing new on their own. The old man may speak; the prince may whisper. | threat capacity as the field's; no orders; dens wake only by the hero |
| **Build** | generals issue orders; patrols walk; dens' wake range normal | normal |
| **Peak** | no new orders; announced reinforcements still arrive | capacity as the field's, peak length 25-40 s |
| **Release** | the pack's morale drops; a routed pack flees | capacity −1, cadence +30 f, for 6 s; potion drop chance ×1.5 |

* **Player-driven escalation is always allowed.** If he wakes three dens, three dens fight. The
  Drum only holds back what the *AI* would start.
* Difficulty presets map to the Drum's bounds, openly: **Steady** (longer rests, shorter peaks)
  and **Relentless** (the reverse). No hidden rubber band on skill.
* F9: the Drum's readout (I, phase, time in phase, what it blocks). `--drum-trace` writes the
  intensity curve to CSV for the crawl.
* The war-drum music layer (§4.6) follows the phase.

---

## 7. Packs: roles, leaders, plays, morale (the Diablo IV lesson)

Diablo IV's packs work because their members do different jobs that fit together. Here each foe
keeps its archetype (how it attacks) and gains a **role** (what job it does in the pack).

### 7.1 Roles

| Role | Who | Job | How (only slots and request biases) | Counter |
|---|---|---|---|---|
| **Vanguard** | Brute breeds | hold the front, screen the callers | claims the front sectors; when he comes within 3 tiles of a caller, takes the sector between them, always leaving a 1.2-tile gap (no trapping) | break its poise, or go round: it turns at 150°/s |
| **Harrier** | Skirmisher breeds | press his sides | claims side sectors in pairs (never the back arc first, as now) | parry one, step away from the other |
| **Caller** | Hexer breeds | punish standing still | ranged ring; asks for a token more eagerly when he is blocking | close in between bolts |
| **Leader** | one per pack (an elite of any archetype) | command the plays, hold the morale | crowned (`crown` exists), banner aura, one or two affixes | kill it first: the pack routs or breaks |

### 7.2 Plays

The leader picks one play every 30 f from those whose conditions hold, by score. A play only
changes **slot preferences** and **request scores**; the threat manager still decides who strikes,
so I2 holds by construction.

| Play | When | What it does | The answer the player learns |
|---|---|---|---|
| **Pincer** | 2 harriers awake | they take opposite sides | turn to one; the other must wait for its token |
| **Screen** | a caller and a vanguard | vanguard interposes | flank the vanguard, or reflect the bolts |
| **Rotate** | a foe below 30% health | it falls back to the hold ring; a fresh one takes the token | chase the wounded (a choice: risk vs. a kill) |
| **Crossfire** (tier ≥ 4) | 2 callers | their bolts are time-locked to land on the same frame from two sides, as **one strike** (as a volley counts) | leap: one parry can't meet both |
| **Hold the door** | a den with one mouth | the vanguard stands in it, gap kept | bomb or archangel, or bait it out |
| **Rally** | leader hurt, 2+ allies | the leader's own move: 40 f startup, a horn, allies refill 30% poise | punish the rally: it is a 40 f window |

### 7.3 Morale and the rout

Each pack holds morale 100: −35 when its leader dies, −20 per ally slain, −10 per poise break it
sees, +10 per rally. At 0 the pack **routs**, by its breeds' temperament:

| Temperament | Breeds (examples) | At 0 morale |
|---|---|---|
| Cowardly | ravens, minions, bats, cin, kitsune | flee to the den for 180 f, no token requests: a release beat |
| Stalwart | brutes, giants, skeletons | hold: they fight on (no change) |
| Frenzied | loup-garou, rasetsu, dire wolf, oni | +15% pace, rest after a move −20%, for 6 s (frame data untouched) |

The rout feeds the Drum's release. Killing the leader first is the pack's lesson.

### 7.4 Elites and the composer

`pack::compose(dealt_foes, tier, seed)` turns the `Dealer`'s bodies into a pack: roles by
archetype, a leader (the toughest, or a rival from the Ledger, §8), and affixes on the leader from
tier 3 (one) and tier 6 (two). Every affix is checked against I1-I4 by test
(`no_affix_slows_holds_or_quickens_a_move_past_the_floor`).

| Affix | Effect |
|---|---|
| Ironhide | +15 armor (the finisher still flinches it below 45) |
| Bannered | allies within 4 tiles regain poise 50% faster |
| Vengeful | when an ally dies, it gains 20 poise and rest −20% for 4 s |
| Echoing | its tell is followed by a harmless ghost tell; the floor fill shows only the real one |
| Warding | callers within 3 tiles take bolts reflected at them at 50% |
| Gilded | drops twice the silver, flees at 30% |

---

## 8. The Ledger of Grudges: rivals (the Shadow of Mordor lesson)

Foes that hurt the hero, or get away from him, become **somebody**: named, remembered, ranked,
and they remember him.

### 8.1 Promotion

A foe is promoted to a rival when it:

| Deed | Epithet it earns (examples) |
|---|---|
| deals the killing blow | "Ragnar's Bane", "Who Ate the Janissary", "Bane of the Two Swords" |
| breaks his guard 3 times in one fight | "the Shield-Breaker" |
| survives a fight below 25% health (routed, or he walked off) | "the Twice-Wounded", "Who Fled the Blade" |
| is a leader he left behind (the leash ran out with it alive) | "the Unfinished" |
| takes his silver at a death (§17, decision 2) | "the Silver-Drinker" |

Names come from the breed's folklore (`names.rs`): Norse for draugr and the giants (Hrafn, Skuld,
Grimkel...), Japanese for the yōkai (Kurokane, Akaishi...), Turkish and Turkic for the Anatolian
nights (Kara Yusuf, Ayaz, Tepeli...), old European for the witches and the loup-garou, hellish
titles for the realm natives. Name + epithet + breed: *Kara Ayaz the Shield-Breaker, karakoncolos*.

At most **9 live rivals**; a tenth replaces the lowest rank. A rival rises in rank (1-5) each time
he wins again: +8% health per rank, an affix at ranks 2 and 4, a lesson from rank 3.

### 8.2 Traits

Each rival rolls one **strength**, one **weakness** and one **personality**. Weaknesses are what the
old man learns and sells (§10.1), and they tie to the hero's weapons, elixirs and mastery.

| Strengths | Weaknesses | Personalities |
|---|---|---|
| Unflinching: +15 armor | Proud: after he lands a blow he taunts, open for 60 f | **Braggart**: taunts by name, by memory |
| Patient: tracks 4 f longer (still commits for the last 12) | Scarred: blows from the side of his old wound ×1.3 | **Coward**: routs at 40% health and runs to rank up |
| Elixir-proof: one elixir's effect on him halved | Fears steel / fire / the thrown blade: that family staggers him at once | **Duelist**: his pack stands back; the threat pool is his alone (a 1-on-1 the player can accept) |
| Pack-lord: leads a pack of 5 | Greedy: stops to pick up dropped silver (a 50 f opening) | **Schemer**: when hurt, blows a horn that wakes the next den (60 f tell; interrupt it) |
| Vengeful: enraged when his pack dies | Old wound: poise −30% | **Avenger**: hunts the hero if he slew a rival of the same breed (his "brother") |

### 8.3 Memory and lessons

Each rival keeps up to 8 encounters: where, how it ended, which hero, which blade family and tier,
which elixirs were drunk, and counts of the hero's verbs (parries, blocks, evades, finishers).
From rank 3 he carries **one lesson**, drawn from the hero's most used answer to him, and the name
card says so. One lesson at a time, always announced: the player's mastery is to change his answer.

| The hero mostly... | The lesson | What it does (choices only, I6) | The new answer |
|---|---|---|---|
| parries | "He remembers your parry." | varies his startup between two lengths of the same move (e.g. 26 and 34 f, both ≥ the floors); the floor's fill shows the real landing | read the fill, not the rhythm; or evade |
| evades | "He remembers your leap." | prefers his wide arc or his floor move, and waits out the leap's 6 f landing | block, or parry |
| blocks | "He remembers your shield." | prefers the move with the most poise; his blows wear the shield ×1.3 | parry or step out |
| drinks one elixir | "He brought the antidote." | that elixir's effect on him is halved | another elixir, or none |
| finishes with L3 | "He knows your finisher." | evades on L3's startup when he can (his own leap) | end chains on L2, punish the leap |

### 8.4 Meeting a rival

* The Ledger offers one rival per realm visit (two from tier 5), placed as a pack's leader in a
  den chosen by the realm's seed.
* Within 10 tiles: his **name card** (name, epithet, rank, the lesson if any, the known traits;
  unknown ones as "?"), his bark (by memory: "The janissary's shield again? It broke last time."),
  and a 0.5× dilation for 0.25 s. No pause.
* Killing him: 12 f freeze + dilation, his **trophy** (a piece of him: a helm, a horn, a fang)
  goes to the workshop's wall (§12), plus runes by rank, and the silver he took, ×1.5.
* A rival who wins: ranks up and walks off (a rout to his den) with a bark.
* The Ledger is part of the **run**: Start wipes it, Continue keeps it.

### 8.5 The ledger board

`render/hud/ledger.rs`: portraits drawn from each rival's rig (`sprite_from`, as the HUD portrait
is made), in a pinned-note style, with name, rank, the traits as found, the lesson, and his last
encounter. Seen in the workshop (§12) and from the pause menu.

---

## 9. The Prince's Hands: the generals (the dungeon-general lesson)

Each realm gets a **general**: a named commander serving its prince. He does not wait in a room
for the hero: he **runs the realm against him**, by orders the player can see and counter.

### 9.1 Orders and instruments

A general has three orders, from four kinds. Each order works through an **instrument**, a thing in
the realm the player can break, snuff, bar or kill. Every order is announced (I7): a horn or bell,
a herald's bark, an order card on screen (`render/hud/war.rs`), and a ping on the compass.

| Kind | What the general does | The instrument | Counters (and the reward for each) |
|---|---|---|---|
| **Watch** (patrols) | 1-2 patrol packs walk between rooms; a patrol that sees him blows a whistle (60 f) that wakes the dens within 10 tiles | the watch-fire | snuff it (3 blows): patrols stop. Ambush a patrol before it whistles: its leader drops runes. |
| **Muster** (reinforcements) | a war horn sounds (4 s), a marked gate glows, a pack marches in. Only in Build. | the war horn on its tower, or the herald who carries the order (a fleeing courier) | break the horn: no more musters. Bar the gate (interact, 1.5 s): that gate is shut. Catch the herald: the order fails, and he carries a map of the general's post. |
| **Kindle** (hazards) | wakes the realm's hazard in a zone (coins, lava vents, acid, censers' smoke), marked 3 s ahead. Hazards are `Shape::Traps`, `GuardRule::Bare`, planned by the threat manager (I2), and never slow (I3). | the bellows, valve or lever | break it: the zone goes quiet. **Turn it**: it now burns the foes; lure them in. |
| **Decree** (objectives) | a timed scheme that changes the prince's fight: treasurers empty the chests into the court, sappers burn the bridges, cooks lay the prince's feast | carriers walking a route, or a chain to cut | kill the carriers or cut the chain before the timer. Whatever the decree finished, the prince fights with. |

### 9.2 The general himself

He waits at his **command post**, a fortified room of the realm, with a bodyguard pack playing
Screen. He fights as a lesser prince: two phases by health, `Archetype::Prince` machinery with
smaller numbers (health ≈ 300 × the realm weight, hide only in phase 2, token cost 2). Slain:

* every order still standing ends at once, and his banner hangs in the workshop;
* the prince's fight is weakened by one step (proposal: the prince loses his phase-two adds, or
  his hide falls from 70% to 50%; §17, decision 5);
* skipping him is a choice: the hero saves a hard fight now, but the general's orders go on for the
  rest of the realm and the prince keeps his full strength.

**The general adapts once per loss.** When an instrument breaks he answers with one counter-order
(a guard pack moves to the next instrument, or the remaining muster gate is doubled), announced as
any order is. This is the "they change their plans for me" beat.

A rival of rank ≥ 4 can **usurp** a general's place in a realm not yet taken: the general becomes
the rival, with his memory and lesson, and the orders keep their instruments. The Mordor tie.

### 9.3 The seven generals (proposals)

Bodies are existing breeds dressed with a banner, a cloak and a sigil, so no new sculpt is
required (a new sculpt per general can come later).

| Realm | General | Body | Orders |
|---|---|---|---|
| Greed: the Vaults of Mammon | **The Assessor**, who counts what the hero carries | blemmyes, gilded | Muster (the counting-bell), Kindle (coin traps), Decree (treasurers empty the chests into the court: Mammon's golems come sooner) |
| Pride: the Hall of Mirrors | **The Chamberlain of Glass** | tengu, white and gold | Watch (patrols that walk out of mirrors), Kindle (light beams along mirror lines), Decree (polish the throne-mirror: the Peerless gains his crown) |
| Envy: the Serpent Grotto | **The Bell-Warden** | kappa, verdigris | Kindle (geysers), Muster (the drowned rise at the bell's toll), Decree (lower the bell: the court's water rises) |
| Wrath: the Burning Field | **Marshal Gorebrand** | oni, scarred | Muster (war horns), Kindle (lava sluices), Decree (sappers burn the bridges) |
| Sloth: the Grey Mire | **The Lamplighter** | karakoncolos, hooded | Watch (lantern patrols), Kindle (torpor lamps), Decree (wind the hourglass: Belphegor wakes rested) |
| Gluttony: the Rotting Feast | **The Steward of the Last Course** | gaki, aproned | Muster (the dinner gong), Kindle (acid tide valves), Decree (cooks lay the feast Beelzebub eats to heal) |
| Lust: the Garden of Thorns | **The Bridesmaid** | albastı, veiled | Watch (the wedding procession), Kindle (censers), Decree (the vows: the Black Thorn's gate seals for a time) |

Realm generators expose candidate places for the post and the instruments (`Base.posts`), picked
by seed, each reachable (a test as the hub's portals have).

---

## 10. The two voices: the old man guides, the prince cheats

The game has two voices that speak to the hero. Learning to tell them apart is a skill.

### 10.1 The old man's counsel (always true)

`logic/war/counsel.rs`: rules of the form *trigger → line set*, each with a priority, a cooldown
and a once-flag. Lines are **generated from the game's own data** (a rival's real weakness, a
general's real instrument), so they are true by construction, and a test pins it
(`the_old_man_never_lies`: every counsel line about a rival or general names a trait it really has).

| Trigger | What he says (examples; each hero hears his own register) |
|---|---|
| First time a breed kills him | its counter, in one line: "The blemmyes swings slow. Wait for it, then strike." |
| Three deaths in the same room | a specific tip, and an offer: "Come back to me. I'll mend the shield, and we'll talk." |
| A rival is met | one true trait, if the player has paid him for it (or found it) |
| Entering a realm | the general's name, and where one instrument is |
| His blade dulls, his shield cracks | the existing warnings, in his voice instead of a floater |
| A prince's cheat was seen through | praise, and Quitmer barks |
| Rest phase, nothing else to say | silence (he only speaks when it matters) |

* **Delivery**: a portrait in a bottom corner, one or two lines typed out, Quitmer's bark as his
  signature sound, amber text. Only in Rest or Release (the Drum), or on the death screen; never in
  Peak. At most one line every 45 s.
* **Calling him** (H / Back, exists) opens the workshop's War Table first when he has news (§12).
* **Intel costs.** True weaknesses of a rival cost silver at the War Table (cheap: 40-120), or are
  learnt by exploiting them once in a fight.

### 10.2 What makes the old man recognisable

His lines are amber and come with Quitmer's bark. His shop has Quitmer in it. His lantern burns
warm white. These three are the tells the prince fakes, and gets wrong.

### 10.3 The prince's cheats (always lies, always with a tell)

`logic/war/deceit.rs`. Every cheat is a struct of **the lie, its tell, its counter, the reward for
seeing through it, and what it costs if believed**, and a test checks each part exists
(`every_cheat_has_a_tell_a_counter_and_a_bounded_cost`). Cheats come only in Rest and Build, one at
a time, at most two per realm. They never take an owned tier, a relic or a life; what they take is
time, health, silver carried, or a better route.

| Cheat | The lie | Its tell | Counter → reward | Cost if believed |
|---|---|---|---|---|
| **The Whisper** | false counsel: a wrong weakness, a wrong instrument, a wrong way | the sin's colour instead of amber, a reversed echo, no bark | ask the old man (H): "I said no such thing." → **Seen-through** mark (+runes) | a wasted elixir, a walk into a den |
| **The False Gatekeeper** | a side room where "the old man" sells at half price | no Quitmer (or a shadow dog that doesn't wag), a lantern in the sin's colour, too-good prices | strike him: he is a demon (an elite of the realm's stage); slay him → the price of his wares in silver | **tainted** wares: an elixir that turns (strength becomes weakness for 10 s), a blade that shatters after 30 blows; all tainted wares vanish when the realm is left |
| **The Crooked Bargain** | a deal mid-realm, like Mammon's toll: "Leave the horn whole and I'll open the court now" | his offers always cost more than they say (a learnable rule; the old man says so early) | refuse → nothing; accept → the gain is real, and so is the hidden price (the general's decree completes) | a harder prince for a shorter road: sometimes worth taking, which is the decision |
| **Loaded Dice** | a mimic chest (Greed, Gluttony) | it breathes; its coins don't glint; the hinge creaks | strike it first → double loot | it bites (a body strike, planned by the threat manager) |
| **The Turned Sign** | the realm's compass points toward a trap for 20 s | the arrow in the sin's colour, and it flickers | follow the old man's amber marker, which is always true | a detour |
| **The Prince's lie** (exists) | the end-of-realm lie (the toll, "take your throne", ...) | kept as built | kept as built | kept as built |

Each prince flavours his cheats with his sin: Mammon's are all prices, Lucifer's flatter, Leviathan
offers what another hero had, Amon goads, Belphegor offers rest, Beelzebub offers food, Asmodeus
offers what the hero wants most. A "lies seen through" count is kept per run, and the old man
remarks on it at the workshop.

---

## 11. The heroes: mastery, weapons and elixirs

Each hero has three things to grow: **weapons** (the blade and shield trees, exist), **elixirs**
(the elixir tree and the belt, exist) and **mastery**. Mastery today is the path tree, bought with
silver. The plan keeps the path tree, and adds a **mastery of technique** that is *earned*, by
deeds in fights and by the old man's trials, because mastery that is bought is not mastery.

### 11.1 The three masteries (5 ranks each)

| Hero | Mastery | Deeds that feed it | Ranks (proposal; each audited by `kit.rs`'s path audit) |
|---|---|---|---|
| Ragnar | **The Berserker's Way** | punishes, poise breaks, finishers | 1: a gold glint when a foe enters recovery (teaches the punish) · 2: the finisher's poise +5 · 3: a kill during dilation refunds 16 stamina · 4: **War Cry**, a move (frame data, 30 f startup) that staggers body foes about him · 5: once per pack, a guard break leaves him guarded |
| Mehmed | **The Discipline of the Ocak** | blocks, halberd-tip hits (the last 0.3 tiles of reach) | 1: block cost −10% · 2: tip hits +25% poise · 3: **Hold the Line**: a block shoves a body foe 0.5 tiles back · 4: blocks in a row fill a "discipline" meter; full, his next chain costs no stamina · 5: once per pack, a guard break leaves him guarded |
| Miyamoto | **The Way of Two Heavens** | parries, evades with a blow passing under | 1: the parry window shows a white edge on the telegraph · 2: **Riposte**: a strike in the 20 f after a parry lands ×1.25 more · 3: **Mushin**: three parries without a wound, and the next strike ×1.5 · 4: an evade through a blow refunds 10 stamina · 5: the parry window +1 f (8 → 9; still within the bout's targets) |

* Deeds are counted from `WarEvent`s; each rank needs deeds **and** a trial (§12) to confirm it.
* `--kits N` and `the_bouts_meet_their_balance_targets` must stay green with every rank: mastery
  rewards skill, so its effects grow the expert's margin far more than the masher's.

### 11.2 How the enemy meets them

* **Rivals read them** (§8.3): mastery becomes the habit a rival learns, and the next lesson.
* **Weapons**: weaknesses name weapon families (steel, fire, the thrown blade); the old man's
  counsel says which blade to bring; trophies of rivals slain unlock a cosmetic **inscription**
  on that blade ("Bane of Kara Ayaz") that the next rival of that breed remarks on.
* **Elixirs**: drinking one is a `WarEvent`. Cowardly packs lose 15 morale when he drinks
  strength or Ragnarök; a rival who saw an elixir comes with the antidote; the Steward of the Last
  Course's Decree is to steal one bottle from the belt each time his cooks reach the court (a
  stated cost, countered by killing the cooks); and the prince's tainted elixirs (§10.3) are
  tellable by a wrong colour in the bottle.

---

## 12. The old man's workshop, rebuilt

Today the wares are a right-hand panel over the Scene I crypt set with a static line per tab. The
rebuild turns it into a **place**: the old gatekeeper's hidden workshop, where the hero is armed,
counselled and tested, and where the war against him hangs on the walls.

### 12.1 The set

Its own stage (as the title and the cutscenes have theirs), drawn by `render/view/workshop.rs`, in
the board's palette (charcoal, umber, copper, amber, mist teal, pale stone). The camera moves
between **stations**; the old man walks to each and works there; Quitmer follows and reacts.

| Station | Tree / job | What happens there |
|---|---|---|
| **The Anvil** | weapons | blades on racks, owned ones lit, sealed tiers bound in chains with rune seals. Buying: the old man hammers it out (three strikes, sparks, `Anvil` with a 6 f freeze and a little trauma on the last), the hero takes it up. |
| **The Shield Wall** | shields, repair | shields on the wall by tier; reforging is done in front of you |
| **The Alchemy Bench** | elixirs, the belt | bottles on shelves, the belt laid out on the bench; brewing shows the colour that the prince's fakes get wrong |
| **The Training Yard** | path, mastery, trials | a sparring post and a ring of chalk. Preview: the hero swings the chosen blade at a dummy, and a card compares it with what he holds (damage, tempo, reach, parry window, from `Kit`). **Trials** are short bouts in the Sparring Ring's rules: one per mastery rank, against archetypes or the ghosts of slain rivals. |
| **The War Table** | counsel, intel | a map of the hub and the realms: each general's sigil, his orders, the instruments found; the Ledger of Grudges pinned beside it (§8.5); true weaknesses for sale |
| **The Trophy Wall** | — | what the hero took: rivals' trophies, generals' banners, the princes' keys. Every trophy has its line from the old man. |

### 12.2 Three dressings, one layout

Same stations in the same places (so the menus and the code stay one), dressed for the hero being
played, because the fantasy is his:

* **Ragnar**: a Norse smithy, a turf roof, a rune-carved anvil, axes on the wall, a hearth.
* **Mehmed**: an Ottoman armoury tent, carpets, brass lamps, halberds racked, a kazan on the fire.
* **Miyamoto**: a Japanese forge, tatami, a whetstone, katana on a stand, paper screens.

### 12.3 Controls and flow

* LT/RT (and Tab) step between stations as they page the tabs today; up/down in a station's
  rows as today (`ShopState` is kept: each station wraps its existing tab; Counsel, Trials and
  Trophies are new). Esc goes to the door.
* Calling him (H / Back) mid-realm opens at the **War Table** if he has news, otherwise where you
  left.
* The old man's static line per tab becomes counsel (§10.1): what he says depends on what just
  happened down there.
* `emit_wares` retires; `wares_camera` becomes a camera per station.

---

## 13. The HUD and the screens

| Element | Where | Shows |
|---|---|---|
| General's banner | top centre, small | his sigil, his three orders (active / counting down / broken) |
| Order card | slides in under it, 3 s | "THE ASSESSOR MUSTERS · 4" with the instrument's ping |
| Rival name card | beside the rival, 2.5 s | name, epithet, rank, lesson, traits known |
| The voices | bottom left | the old man (amber, portrait, Quitmer) or the whisper (sin colour, no portrait) |
| Pack leader | over his head | the crown (exists) and his morale as a thin ring |
| Drum readout (F9) | debug | intensity, phase, what is held back |
| Ledger board | pause menu + workshop | §8.5 |

All in the charcoal-and-copper panels of the art-direction pass.

---

## 14. Saving

| Key | Scope | Holds |
|---|---|---|
| `vh.warband` (new) | the run (Start wipes, Continue keeps) | the Ledger (rivals, memory, trophies), each realm's general state (instruments broken, decrees done, slain), lies seen through, counsel once-flags |
| `vh.progress` (exists) | across runs | mastery ranks and trial results join `HeroProgress` (format v3, v2 still loads) |

`Run` checkpoints write `vh.warband` alongside `vh.run`. Captures never write it (as now).

---

## 15. Tools: harness, telemetry, tests, captures, the Map

### 15.1 Flow targets (measured by the crawl)

| Measure | Target |
|---|---|
| Overlaps (two strikes inside 18 f) | 0, as now |
| Share of combat time in Peak | 35-55% |
| Rest between peaks | ≥ 8 s on average (Steady ≥ 12, Relentless ≥ 5) |
| Pressure without a ≥ 60 f tell | 0 |
| Rivals met per realm | 1-2 |
| Instruments broken by the expert bot / the adept | ≥ 2 of 3 / ≥ 1 of 3 |
| Expert's time through a realm with its general alive vs slain | the difference ≤ 25% (the choice is real, not forced) |
| AI cost with 30 foes | ≤ 0.5 ms per frame (`--perf battle30`) |
| Debris at 480 | ≤ 0.3 ms |

### 15.2 Harnesses

* `--crawl N` gains the Drum, packs and generals; prints peak share, rest length, routs, orders
  issued and countered.
* `--drum-trace` writes the intensity per second.
* `--rivals RUNS SEED`: plays RUNS crawls in a row with one Ledger and prints promotions, ranks,
  lessons and how the bots fared against each lesson.
* `--general SIN SKILL TIER SEED`: one realm with its general, instrument by instrument.
* The bot learns: break instruments it passes, bar gates, refuse whispers (asks the old man), strike
  the false gatekeeper, and switch answers when a lesson is announced (the expert does; the adept
  doesn't, which is the gap mastery should open).

### 15.3 Tests (names as the suite writes them)

* `the_war_log_reports_every_strike_once`
* `presentation_never_touches_the_simulation` (same harness numbers with fx on and off)
* `a_press_during_hitstop_comes_out_when_it_ends`, `hitstop_stays_inside_its_budget`
* `time_dilation_keeps_every_frame_relation`
* `no_play_lets_two_strikes_land_inside_the_reaction_window`
* `no_affix_slows_holds_or_quickens_a_move_past_the_floor`
* `a_rout_releases_the_pressure` (threat requests stop for its frames)
* `every_rivals_lesson_changes_choices_not_frame_data`
* `a_rival_remembers_and_says_so` (lesson announced on the card)
* `every_order_is_announced_a_second_before_it_bites`
* `every_instrument_can_be_reached_and_broken`
* `the_old_man_never_lies`
* `every_cheat_has_a_tell_a_counter_and_a_bounded_cost`
* `the_prince_never_takes_what_the_hero_owns`
* `every_breed_has_its_own_voice_and_its_familys_signature`
* `mastery_grows_the_experts_margin_more_than_the_mashers`
* plus the existing balance tests, kept green.

### 15.4 Captures, Map stops and cheats

* Captures: `rival:N` (a rival of rank N leads the nearest pack), `ledger`, `general:S`,
  `order:K`, `whisper`, `gatekeeper`, `workshop:STATION`, `trial:N`, `drum:PHASE`, `juice` (a scene
  that fires every juice event in turn for side-by-side shots).
* Map stops (per the standing rule): the workshop (each station), each general's command post,
  and a "rival yard" (a test room where a chosen rival waits). Bump the count in
  `the_map_jumps_to_any_dungeon_or_cyclops`.
* Cheats: F9 the Drum readout; F10 promote the nearest foe to a rival; F11 break the nearest
  instrument. F5 (GOD MODE) and F8 stay as they are.

---

## 16. Roadmap

Sizes as in ROADMAP.md: M is about one working session, L several. Each phase ends with tests green,
captures checked, a frame-time check, the docs updated, and a commit.

| Phase | Scope | Exit criterion |
|---|---|---|
| **W0 The war log** (M) | `WarEvent`, `Brawl.log`, `PresRng`; move the direct audio/shake/floater/hitstop calls into consumers; the hero's slash pose on frames | captures look and sound the same; the harness prints identical numbers |
| **W1 Feel** (L) | hitstop table and budget; time dilation; trauma rotation, kick, FOV punch, sources, the shake slider; matter, chips and voxel shatter, 480 debris instanced; springs and overshoot | the `juice` capture shows each; `--perf` within budget; balance tests unchanged with `--fx-off` |
| **W2 Voices** (M) | `audio/voices.rs` with every breed's set; layered hits; `audio/mix.rs` (priority, dedupe, ducking, limiter, pan and distance); BPM per track and the war-drum layer | the voice test green; a listening pass by you on stage 1-3 bouts |
| **W3 The Drum** (M) | intensity, phases, bounds, the presets, F9, `--drum-trace` | the crawl meets §15.1's peak and rest targets, overlaps 0 |
| **W4 Packs** (L) | roles, leaders, plays, morale and the rout, affixes, the composer; the vaults and the six realms use it | the crawl still cleared by the expert at tier 1 and the adept from tier 4; routs visible; zero overlaps |
| **W5 The Ledger** (L) | promotion, names, traits, memory, lessons, name cards, barks, trophies, the board, `vh.warband`, `--rivals` | over 10 crawls the expert meets 1-2 rivals a realm and beats each lesson; the adept loses more to lessons than to plain foes |
| **W6 Generals** (L, Greed first, then one M per realm) | `general.rs`, orders and instruments, the herald, the post fight, the counter-order, the usurper, the banner HUD, `--general`; then the other six | Greed: §15.1's general targets; then each realm in turn |
| **W7 The two voices** (M) | counsel rules and lines per hero; the cheats and their tells; the seen-through count | the two truth tests green; every cheat captured with its tell |
| **W8 Mastery and the workshop** (L) | `mastery.rs`, deeds and trials; the workshop set and three dressings; stations over `ShopState`; the War Table, the Trophy Wall | the kit tests green at every rank; captures of every station for each hero |
| **W9 Tune** (M) | a full run from the Garden to the House door with the bots and by hand; tune the Drum, morale and lesson rates; write the tuning log in this file | §15.1 targets met; your playthrough notes closed |

Order: the feel and the log first, because every later system speaks through them and they make
the game better at once. The workshop shell (the set and dressings, W8's art) has no dependency and
can run in parallel from W1 as a background agent in a worktree, as the realms were built.

---

## 17. Decisions (proposals first)

1. **Mastery**: an earned technique track beside the path tree (proposal) / the path tree itself
   renamed Mastery, nothing new.
2. **A rival who kills the hero takes 15% of the silver he carries**, given back ×1.5 when slain
   (proposal) / takes nothing.
3. **The Ledger lasts for the run** and Start wipes it (proposal) / it lasts across runs.
4. **The Drum paces only** (proposal) / it also adapts to skill.
5. **Slaying a general**: the prince loses his phase-two adds (proposal) / his hide falls from 70% to 50%.
6. **The False Gatekeeper and tainted wares** are in (proposal) / the prince's cheats stay
   words only (whispers and bargains).
7. **Slow motion on kills** on by default (proposal) / off by default.
8. **Generals' bodies**: dressed existing breeds now, own sculpts later (proposal) / sculpt seven now.
9. **The workshop**: one layout in three dressings (proposal) / three different workshops.

## 18. Risks

| Risk | Guard |
|---|---|
| The AI layers create overlaps or pile-ons | they only bias the threat manager, which keeps the commit gate; the overlap tests run on every crawl |
| Rival lessons feel unfair | one at a time, announced, choices only; the fill shows the real landing |
| The prince's cheats annoy | two per realm, never in Peak, always a tell, never a permanent loss; decision 6 can pare them back |
| Juice hurts responsiveness | the hitstop budget, a short freeze when he is struck, the input buffer frozen with the game |
| Juice hurts frame time | `--fx-off`, the debris budget, instancing, `--perf` per phase |
| Mastery breaks the balance | the kit audit and the balance tests at every rank |
| Scope | each phase stands alone and ships; the generals go one realm at a time |

---

## Build log

### W0 The war log (4f5c65e)

`logic::war` (`WarEvent`, `PresRng`), `game::war` (`War`, `war_note`, `update_war`, `WarTally`). The
brawl notes tells, landings (with how he met them), whiffs, his hits, kills (with `last_of_pack`)
and drinks. `BoutStats.landings`. The direct audio and shake calls stay where they are until the
feel (W1) and the voices (W2) are merged: their consumers replace them then.
Test: `the_war_log_reports_every_strike_once`.

### W3 The Drum

`logic::war::drum` (`Drum`, `DrumPhase`, `Pacing`, `Bounds`, `intensity`), heard and ticked in
`update_war`; its bounds go to the threat manager of the vaults and the realms (`pace_the_brawl`;
the ring stays the lab, a prince's court keeps its own pace) and to the health-potion chance.
The readout is on F8 with the ring's (not F9). `VH_NO_DRUM=1` switches it off for A/B harness runs.
`--drum-trace` writes `perf/drum_trace.csv`; `--crawl` prints the share at peak, the peaks and the
mean rest.

Intensity, as built: 0.30 × foes hunting him (full at 4) + 0.30 × strikes reaching him in 5 s (full
at 4, however he met them) + 0.25 × damage taken in 5 s (full at a fifth of his health) + 0.15 × how
low he is. Peak from 0.5 (Steady 0.6, Relentless 0.45).

| Change | Why |
|---|---|
| Damage and low health weigh less, foes and strikes more | the expert, barely hit, never reached a peak (p90 0.27) |
| A release never takes the pool below 2 points | at 1 point a brute (cost 2) could not strike: lone brutes stood idle and two adept crawls ran out of time |

The crawl (`--crawl 6`, Drum on / off):

| | adept t1 | expert t1 | adept t4 | expert t4 | adept t7 | expert t7 |
|---|---|---|---|---|---|---|
| cleared | 0% / 0% | 100% / 100% | 100% / 100% | 100% / 100% | 100% / 100% | 100% / 100% |
| kills of 61 | 27 / 32 | 61 / 61 | 61 / 61 | 61 / 61 | 61 / 61 | 61 / 61 |
| at peak | 52% | 21% | 50% | 14% | 25% | 3% |
| overlaps/min | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |

The adept, the player the pacing is for, spends about half its fighting at peak, inside §15.1's
35-55%; the expert feels less pressure, as it should. Rests are short (about 6 s) because the bot
walks straight on to the next den: player-driven, which the Drum allows.

### W4 Packs

`logic::war::pack` (`Role`, `Temperament`, `Affix`, `Play`, `Pack`, `compose`, `Directive`) and
`game::packs` (composing the dens into packs, hearing the war log, `packs_frame` from the brawl's
foes' frame after the dens). The threat manager's `Request` gained `bias` and its `Waiter` a
`want` (a bearing): the only ways a play reaches the fight. `MoveId::Rally` (40/1/36, a spell: no
blow) is the leader's move. The HUD writes LEADER and its affixes over a leader's bars, with its
pack's heart as a thin blue bar. `VH_NO_PACKS=1` switches the packs off for A/B runs;
`--crawl-one SKILL TIER SEED` plays one crawl and lists what is left standing.

Built: Hold, Pincer, Screen, Rotate (a foe under 30% falls back while another takes its place),
Rally (600 f between rallies); morale and the break by temperament (cowardly flee for 180 f and
give their tokens back at once, frenzied +15% pace and quicker rests for 360 f, stalwart fight on);
all five affixes (Ironhide, Bannered, Vengeful, Warding, Gilded). Not built: Crossfire and Hold the
door (the riskiest for the reaction window and the den mouths; left for W9), Echoing (dropped: a
ghost tell with no blow fights the rule that every tell means a strike).

| Change | Why |
|---|---|
| Rallies rest 600 f per pack | without it a hurt leader rallied every 1.3 s (35 rallies in one adept crawl) |
| A foe slain in the vaults gives its token back as it falls | the slain are swept from the field before their next frame; two committed tokens never came back and closed the pool for good (an expert crawl stood still for 13 minutes). Not in the ring: its slain stay a frame, and the lab's numbers hold to the decimal |
| A cowardly pack gives its tokens back the moment it breaks | it held them for a frame while turning to flee |

The crawl (`--crawl 6`, Drum and packs on): every clear as before (the adept from tier 4, the
expert at every tier), 5-11 packs broken and 1-8 rallies a crawl, the adept 45-54% of its fighting
at peak, overlaps 0 but for the adept at tier 1 (0.02 a minute: one in six crawls, a hero on the
move). Test: `the_packs_play_together_inside_the_reaction_window_and_a_rout_releases_the_pressure`.
