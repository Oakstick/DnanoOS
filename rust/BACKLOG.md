# Backlog: from 70 to 80+

From the editor's review of 1.0.22 (2026-10-09). The estimated Metacritic score was 70; the target is 78–82. Every item below is
to be fixed. IDs are stable: refer to them in briefs, commits and QA.md. Status: **todo**, **doing**,
**done** (with the release), **blocked** (waiting on a decision), **to decide** (a proposal: the user picks whether and when).

Each item lists:
- **Done when:** the acceptance criteria;
- **Size:** S = under a day, M = 1–3 days, L = about a week;
- **Est. gain:** the estimated Metacritic gain, where it applies.

The waves follow the review's tiers and can partly run in parallel. The constraints are file overlap and at most about 3 agents at once (memory).

## Decisions needed from the user

| # | Question | Blocks |
|---|---|---|
| D1 | The kidnapped friend's name. **Decided: Toby (Tobias)**, after the Book of Tobit (Raphael guards young Tobias; Asmodeus is defeated). | B08 |
| D2 | The old man's name. **Decided: Hızır (Khidr)**, the ageless guide who comes to travellers in need when called. Quitmer stays the dog's name. | B09 |
| D3 | Voice. **Decided: better synth, no recorded voices.** Warmer, more expressive stylized speech. | B10 |
| D4 | "House of the Devil". **Decided: rename to "The House of Seven Seals".** | B23 |
| D5 | Lucifer's last phase "The Fallen": keep or rename? | B23 |
| D6 | Music per place: do you want to compose or supply themes (Krampus, each prince), or assign from the 12 tracks we have? | B14 |
| D7 | Localisation: which languages first (proposed: Turkish, Japanese, German)? | B24 |
| D8 | Web build: make it run again (WebGPU only), or drop it? | B27 |
| D9 | The boys in play. **Decided: all three:** the silhouette in the glow, a line per realm, a scene after each prince. | B07 |
| D10 | Rune carvings: stay with the blade/shield they were cut into (today), or carry over to a newly bought tier? | B31 |
| D11 | With no deals (B40): do the princes' other cheats stay (the Whisper, Loaded Dice, the Turned Sign: lies with a tell, not deals), and the end-of-realm lies ("take your throne")? | B40 |
| D12 | Generals and keys (B41). **Decided: 3 generals per realm, each carrying a key; the 3 keys open the prince's court.** Still open: do the realms' climaxes (the Peerless, Kiyohime's bell, Hel's bed, the pylorus...) stay as the road to a general, or go? | B41 |

## Wave A: the first hour (the review's Tier 1, about +4 to +5)

| ID | Item | Done when | Size | Status |
|---|---|---|---|---|
| B01 | **The Garden fights by frame data.** Stage-1 foes as gentle duelists (slow tells, low damage, few at once); the keys and the north gate unchanged. | The Garden uses `Field::Vault`-style brawl rules. The novice bot clears the Garden, the expert's time is within ±20%, and the old contact rules are no longer used on the road. | L | todo |
| B02 | **The Cyclops by frame data.** The meadow's boss as a frame-data boss: tells, punish windows, phases. | The Cyclops has an FSM, tells and a readable punish window; the novice bot beats it on 3 lives in most seeds. | M | todo |
| B03 | **The side dungeons by frame data.** The Cisterns, the Bone Vaults and their Cyclopes (off the road, LEVELS map). | No old contact rules left anywhere; the code path removed or test-only. | M | todo |
| B04 | **The old man teaches.** A short, skippable duel in the Garden: guard, parry, dodge, and the tells (glow, flash, red glint) on a sparring foe; the Sparring Ring reachable from the hub or the wares. | A first-time player is shown each of guard, parry, dodge and the three tells once, with prompts in the right glyphs. The Garden tour's control card names the shield. | M | todo |
| B05 | **Jump and dodge are one verb.** Space always leaps; in fights the leap carries the i-frames. Fixes "jumping in the Vaults didn't work". | In the Vaults and every brawl, Space leaps from any state except hitstun, a hold and a guard break. A test covers each state. | S | todo |
| B06 | **Stale player-facing text.** README controls table, bombs (disabled), "only Ragnar", steam/README, PRINCES.md "Satan is the last boss", WARBAND §14 "a tier harder". | No stale claims found by a sweep; the controls table matches `input/bindings.rs`. | S | todo |

## Wave B: carry the heart through play (Tier 2, about +3)

| ID | Item | Done when | Size | Status |
|---|---|---|---|---|
| B07 | **The boys present in play** (D9: all three). The boy's silhouette inside the warrior's soul-glow (select screen, shrines, when he falls); a line from the boy entering each realm; a short scene at Hızır's after each prince with the three boys and the warriors. | Each realm and court has at least one moment that names or shows the boy. The story tests pass; the simulation is unchanged. | L | todo |
| B08 | **Toby's trail** (D1). His things left in every realm (scarf, ball, chalk mark, a toy), picked up and kept; they pay off in the lair and the ending. | 7 tokens, one per realm, shown in the pause menu and the bestiary; the reunion references them. | M | todo |
| B09 | **The old man is Hızır** (D2) in Scene I, the wares, the guide and the credits. | His name in Scene I and the credits; the guide's title uses it. | S | todo |
| B10 | **Warmer synth speech** (D3: no recorded voices): expressive formant speech with emotion per line (fear, resolve, warmth, menace), distinct voices for the three boys, Toby, Hızır, each prince and Krampus; extra care at the key beats ("He took him!", Hızır's promise, the reunion). | Each speaker distinct and the key beats emotionally readable; the voices slider and subtitles work. | M | todo |

## Wave C: the climax looks and sounds best (Tier 3, about +2 to +3)

| ID | Item | Done when | Size | Status |
|---|---|---|---|---|
| B11 | **Every realm and court to the Cisterns' bar**, in the order players meet them; character kept (see RENDERING.md: the Garden's green, the hub's fire, the Vaults' gold). Order: Greed (doing), Pride, Wrath, Envy, Sloth, Gluttony, Lust. | Each has a RENDERING.md section; camera checks clean; GPU within ~+50%; the user approves each. | L ×7 | doing (Greed, 2nd pass) |
| B12 | **The Pit and Krampus's lair to the bar.** The finale must be the best-looking place in the game. | As B11. | L | todo |
| B13 | **The Sparring Ring** (gym) to the bar, if B04 sends players there. | As B11. | M | todo |
| B14 | **A theme per place and prince** (D6): fixed, not random; a Krampus theme; the realm themes reflect each realm's character. | `track.rs` maps every stage to a fixed track; no random choice on the road. | S–M | blocked (D6) |
| B15 | **Animation gaps critics see:** deaths (foes, heroes, princes); the blade visible through all its active frames; the evade not a flip; the parry keyed; Miyamoto's grip; Mehmed's börk; foot slide on fast foes; Krampus's hoof. | Each item captured before/after; ANIMATION.md open list emptied. | L | todo |
| B16 | **Tells polish:** the glint's sound, a white edge on bolts, the rim glow not washing foes out up close. | Listed in WARBAND/ANIMATION as built. | S | todo |

## Wave D: fairness and clarity (Tier 4, about +2)

| ID | Item | Done when | Size | Status |
|---|---|---|---|---|
| B17 | **The five hard realms readable** (Mirrors, Grotto, Field, Mire, Thorns): one landmark or route cue each where people get lost, the lie intact. Proxy: the bots navigate them without the harness's carry. | `--run` with no carry in all 7 realms; a human playtest note per realm in QA.md. | L | todo |
| B18 | **No mirrored controls.** Replace Thorns' Enthralled mirror with a pull toward the Bride, slower turning or blurred sight. | Nothing in the game turns the controls (PRINCES.md §0 holds everywhere); test. | S | todo |
| B19 | **Difficulty: Story / Normal / Hard.** Built from the Drum's pacings (Steady, Relentless), the prince damage and the lives; chosen at Start, changeable in Settings. | Three modes. The adept finishes Normal in ≥3/4 runs per order, the novice finishes Story, the expert finds Hard hard. Shown in the HUD/pause. | M | todo |
| B20 | **The adept's walls:** the Rotting Feast and the Pit. | The adept reaches the ending in ≥3/4 runs per order. | M | done (merged 9873fe2, W11) |
| B21 | **Runes have a use:** rune-carving at the anvil (Option A). | Built, measured: the expert spends ≥40 runes a run, the adept ≥20. | M | done (merged 9873fe2, W11) |
| B22 | **New Game+ bites.** | The expert's NG+1 is longer than its first run, with the courts real fights. | M | done (1.0.23) |
| B23 | **One cosmology** (D4, D5): one line in Scene I, the guide and the bestiary on why every culture's fears share Krampus's underworld, paid off by the poem; the hub's house renamed "The House of Seven Seals" everywhere (Scene I, the hub, the banner, the guide, docs). | The line appears in three places; the no-Devil rule is consistent with every name. | S | todo |
| B24 | **Surface the warband.** Rivals, generals and the princes' cheats are seen in a normal run, not just possible: rival intro cards, a rival per realm guaranteed, a nemesis returning at least twice a run. | Lessons learned in most expert runs (W9 measured 3 of 15); every run meets a named rival twice. | M | todo |
| B25 | **The remaining camera spots:** the Burning Field (6 clips, 11 hides), the Rotting Feast (3/1), court 5 (3), the Bone Vaults (2), the Hall of Mirrors' chase view. | `--camera-check all` at 0 clips and 0 hides. | M | todo |

## Wave E: reach (Tier 5, +1; more for sales)

| ID | Item | Done when | Size | Status |
|---|---|---|---|---|
| B26 | **Localisation** (D7): string tables, fonts for each script (Japanese needs a CJK SDF font), then three languages. | A language setting; every player-facing string through the table; three languages complete. | L | blocked (D7) |
| B27 | **The web build** (D8): run or drop. | Runs in Chrome (WebGPU) as a demo, or the claim and code are removed. | M | blocked (D8) |
| B28 | **Steam:** achievements for today's game (each prince, NG+, no-hit court, every bestiary page, every rival), the real App ID, store text. | 20–30 achievements wired and tested in the harness. | M | todo |
| B29 | **Performance on modest hardware:** measure the Low/High presets on a mid GPU and a Steam Deck profile; tune the p99 spikes (the Pit 17.4 ms, the souls, destruction). | 60 FPS p99 at Low on the target; RENDERING.md "Open" updated. | M | todo |
| B30 | **A human playthrough** (the user): the W9 hand-playthrough checklist and QA.md's untested items (device loss in the wild, resize, controller hot-plug, audio, alt-tab). | The checklist ticked in QA.md; issues filed back here. | M | todo (user) |

## Wave F: from the work in progress (to decide)

Open items and proposals from the agents' reports. Every new idea lands here first; the user decides what to build.

| ID | Item | Done when | Size | Status |
|---|---|---|---|---|
| B31 | **Carvings and new tiers** (D10): a newly bought blade or shield starts uncarved today; carry the carvings over, or refund part of the runes. | As decided; save and test. | S | to decide |
| B32 | **The blade's rune marks read faintly** next to the shield's ring (rune-carving captures). | Carved blades read at chase distance, from every hero's angle. | S | to decide |
| B33 | **New Game+ courts never cost the expert a life**: its deaths are all in the realms. Asmodeus is about twice as long as the other princes at high tiers (his base weight and temper), and NG+3 is near a wall in places (Mammon 8 min at tier 13). | The expert loses some lives in NG+1 courts; Asmodeus in line with the others; NG+3 hard but finishable. | M | to decide |
| B34 | **The novice's walls**: the tier-1 vaults (18–28 of 61 foes slain) and the first prince, which by design a masher must not fell. Covered for players by B19's Story mode? | The novice finishes on Story (B19), or a decision that the novice stays a measure only. | S | to decide |
| B35 | **The Vaults' leftovers** (first pass, before the second): the gilt pilasters read tan at chase distance; the treasury marble's veins look wavy from above; the giant Mammon statues are scaled-up niche statues, blocky close to; the paintings are old pixel canvases; the Assessor's counting-bell is plain-lit. Review again after the second pass. | Each fixed or accepted after the second pass's captures. | M | to decide |
| B36 | **Captures show the fallback light**: a `--capture` without `--draw-at` is drawn before the level's probes arrive, so captures can mislead reviews. Make captures wait for the probes by default. | Every capture is drawn with the probes in place; the capture docs updated. | S | to decide |
| B37 | **"Old saves" after the ending**: a save that finished before 1.0.22 still offers Continue to Krampus's lair (left alone so no real run is wiped). | Decide: leave, or convert such saves to the finished state with New Game+ offered. | S | to decide |
| B38 | **A death, a real device reset, sleep and resume**: Q2's GPU recovery is tested by a forced loss only; a real driver reset or sleep/resume is unchecked. | Checked by hand (part of B30) or a note that it holds. | S | to decide |
| B39 | **Bug (user, 2026-10-09): the jump on the pad's A plays the wrong animation.** Leads: in fights the leap is the evade and plays the `roll` clip ("Stand To Roll", a dive and tuck that reads as a flip, ANIMATION.md open list); outside fights `jump`. Check which clip plays from A vs Space in each place (the Garden, the Vaults, a court), the timing against the leap's frames, and whether B05 (Space always leaps in a brawl, branch `quick`) changes it. | The leap looks like a jump wherever it is pressed, pad or keyboard; the evade's i-frames unchanged; a capture sequence before/after. | S–M | todo (bug) |
| B40 | **The princes never make a deal** (user, 2026-10-09). Remove every offered deal: the Crooked Bargain (WARBAND §10.3, `logic/war/deceit.rs`), Mammon's toll ("KEEP THE GOLD", `game/vault.rs`) and his "a chance to keep the gold". The Vaults then end at their door like every realm (B41). Update the guide pages, the bestiary, the tips and the docs that mention the bargain or the toll. See D11 for the other cheats. | No prince offers the hero anything, anywhere; tests pin it; WARBAND/REALMS/PRINCES updated; the harness numbers that move reported. | M | to decide |
| B41 | **Every realm is fight, generals, keys, the prince's door** (user, 2026-10-09). Each level is about fighting through and beating its **three generals**; **each carries a key**; **with all three keys** the door to the prince's court opens. Today a realm has no keys (`keys_total` 0) and closes on its own trap or the prince's lie, with one general (the Watch/Muster/Kindle/Decree officer, WARBAND §9). Build: the generals as the realm's key-bearers (named, with a post, an entrance and a fight of their own), the keys on the HUD and the minimap, the court's door sealed until all are held. See D12. | In all 7 realms the court opens only with the three generals' keys (3 of 3 on the HUD); each general is a real fight with a name card; the bots find and beat them (`--run` with no carry); REALMS.md and WARBAND.md rewritten for it. | L | to decide |
| B42 | **More enemies in the levels** (user, 2026-10-09). The realms, the Vaults and the Garden feel thin: raise the density (more packs, larger packs, more patrols between dens, reinforcements), within the threat manager's fairness (no two blows within 300 ms; the attack tokens) and the frame budget. Then retune W10/W11 so the adept still finishes. | Foes per minute of play up by a target to agree (e.g. +50%) in every realm; the adept still ≥3/4 per order; p99 frame time held; a before/after table. | M–L | to decide |
| B43 | **More epic boss battles for every prince** (user, 2026-10-09). Today a court is a round arena, three phases, and 1–2 minutes for the expert (W10/W11). Explore for each of the seven princes (and Krampus):
- a **cinematic entrance** and a death worth the wait;
- **scale**: a prince that grows or towers in his last phase, or a giant form;
- an **arena that transforms** by phase (the floor breaks, the room floods, the mirrors shatter, the clocks stop, the banquet collapses);
- **set pieces** that use his realm's lie and his general's instrument;
- **his own theme** (B14);
- **a final phase that is a spectacle and still fair by frame data**.

Find references (Souls, God of War, Hades, Zelda) per prince and propose before building. Related: B12, B14, B15 (deaths), B33. | A short design per prince (approved first), then built prince by prince: each court has an entrance, a transformation and a finale, its tells readable, the adept still wins, the camera checks clean. | L ×8 | to decide |
| B44 | **Seven seal-stones open the final level** (user, 2026-10-09). Each prince drops a **seal-stone** when he falls; the hero collects it; with all **seven seal-stones** the final level opens. This matches the hub's seal, which breaks today with "all seven keys" (`game/hub.rs`, `sins_cleared`). Build:
- a seal-stone per prince, each with its own look and colour, tied to his sin;
- the drop and pickup in the court, a short moment when it is taken;
- the seal-stones shown in the hub (set into the House of Seven Seals, one per seal) and on the pause screen;
- with seven, the seals break and the way to the Pit opens.

**Name decided: "seal-stones"** (the user, 2026-10-09; "gems" stays Mehmed's currency word). | Every prince drops his seal-stone; the hub shows them set in the House; 7 seal-stones open the final level; the save keeps them; tests; the docs updated. | M | to decide |

## Suggested order

1. **Now:** B05 and B06 (small, and B05 is a live bug), with B20 and B21 (the balance job) and B11 Greed (the art job).
2. **Next:**
   - B01, B02 and B04 together (one combat job; they share files);
   - B18 and B19 (one job);
   - B11 Pride (an art job).
3. **After the decisions:** B07–B10, B14 and B23 (one story job, once D1–D6 and D9 are answered).
4. **Then:** B15 and B16 (animation), B17 and B24 (design), B11 and B12 (the remaining art, realm by realm), B25.
5. **Last:** B03, B13, B26–B29; B30 throughout.
