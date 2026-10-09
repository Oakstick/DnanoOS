# QA pass (2026-10-08/09, from 1.0.18; rust-port's 1.0.19 merged in)

A full playthrough QA pass to protect the review score: crashes, softlocks, stuck foes, broken saves
and camera traps anywhere a reviewer could go. Branch `qa` (worktree `vh-qa`), from `rust-port` at
4153341, with `rust-port` at 16e5715 (1.0.19, the bestiary) merged in before the last checks.
Nothing merged into `rust-port` or released.

## Summary

| Severity | Found | Fixed | Open |
|---|---|---|---|
| crash | 1 | 1 | 0 |
| softlock | 0 | 0 | 0 |
| broken | 7 | 7 | 0 |
| ugly | 5 | 5 | 0 |
| minor | 5 | 5 | 0 |
| design (for the user) | 5 | 3 decided and built | 2 (the adept's wall, runes: the balance agent's) |

The open issues were fixed on branch `fixes` (2026-10-09, worktree `vh-fixes`, from `rust-port` at
6b45a7b): see "The fixes pass" at the end.

No panic, stall, softlock or save bug was found in about 100 hours of simulated play (below). The
fixes keep every harness number identical except one whole run (Q3, explained there).

## Issues

| Id | Severity | Where | How to reproduce | What happens | Cause | State |
|---|---|---|---|---|---|---|
| Q1 | broken | everywhere play begins by itself | Alt-tab during a countdown (READY), the loading card, the return from Dying, or the old man's door | The countdown ends and the fight starts with nobody at the keys; foes hit him while the window is behind another | `focus_lost` paused only from `Playing` | **fixed** 4711d4a: play that would begin by itself while away waits on the pause menu; any press (a controller's reaches the game unfocused) counts as being back. Test `play_never_begins_while_he_is_away` |
| Q2 | crash | anywhere, rare | A GPU device loss (driver reset/TDR, sleep and resume, a GPU switch). Found in the player's own `%APPDATA%\VoidHarvest\crash.log` (2026-10-03): `Error in Surface::present: Validation Error ... Parent device is lost` | The game dies (release builds abort on panic); progress since the last checkpoint and anything not yet flushed is lost | No device-lost handling: wgpu panics in `present` once the device is lost | **fixed** 5bfdd0d, e333e28: the device's lost callback raises a flag the app reads before presenting; lost mid-frame, the frame's texture is let go unpresented (a present or a discard on a lost device ends the process in wgpu); the game writes what is marked and pauses, the renderer is dropped and started again as at launch, the level goes up whole, a line says so. Tests `a_lost_device_raises_the_flag_before_any_present`, `a_lost_gpu_saves_first_and_waits_on_the_pause_menu`; `VH_LOSE_DEVICE=SECS` loses the device on purpose (run windowed here: lost twice in 45 s, back each time) |
| Q3 | broken | the Rotting Feast (Gluttony), the Steward's post | `--monkey 11 300 --stop 34` (and seeds 12-18) | Foes stand inside rock: the Steward's watch keeps walking its round through the closed gatehouse of a seam, a muster marches in from a gate in the digested zone through the closed teeth, guards are placed into rock; after a fall, the garrison of a closed gatehouse is put back inside the rock | The general's puppets follow ways laid on the realm's ground as first laid out; the Feast's seam closing never told him. `reset_round` sends foes home even when home has become rock | **fixed** 26d172a: `general_ground_shut` (his map learns the rock, ways through it are given up, rounds through it not walked again, cut-off gates barred for good); a guard never holds a tile turned to rock; a foe whose den is rock makes its den where it stood. Tests `the_teeth_shut_the_generals_ways_too`, `a_foe_whose_den_turned_to_rock_goes_home_where_it_stood`. Harness: identical except run-e-3-1 (expert seed 3 order 1): a gaki that used to be sealed into the rock on the expert's fall stays in the stomach's fight; the expert fails the Last Course once more and the run still reaches the ending (109 min against 102) |
| Q4 | minor | every court (and the Cyclops' meadow) | F2 (the gate cheat) in a court | He is set two tiles south of the court's south-wall gate, inside the rock; in the meadow, inside the low wall behind the gate | It always placed him at `gate.y + 2` | **fixed** 5e3467a: the nearest floor on the gate's inner side. Test `the_gate_cheat_sets_him_on_floor_before_every_gate` |
| Q5 | broken | everywhere | Unplug the controller (or its battery dies) mid-fight | The fight goes on without him | No handling of a controller going away | **fixed** fa68503: the game pauses (as for focus loss) when the controller in use goes; nothing happens if he plays on the keyboard. Test `a_controller_lost_mid_fight_pauses_the_game` |
| Q6 | broken | Belphegor's court (the Hall of Stopped Clocks) | `--monkey 41 180 --stop 10`: hands off by the north wall, Belphegor yawns | The yawn carries him through the wall's face into the rock, and further yawns deeper; he stands inside the wall, the camera with him | The yawn shoves 0.36 tile a frame, more than his 0.3 reach of a wall; the walls push out only of the tiles about him, not the one he stands in | **fixed** 76a30dc: a shove that would take him onto a tile he could not step to is taken again in short steps. Harness numbers unchanged. Test `a_hard_shove_never_carries_him_into_a_wall` |
| Q7 | broken | the HUD's location panel (bottom right) | Any court with a long name, e.g. `--capture x.png --script "0.1:court:0" --time 6`: `vh-qa-cap/q-location-label-under-glyph-court0.png` | THE THRONE OF THE MORNING STAR runs under the B glyph | The name shrank only to 70% and the panel was capped at its slot | **fixed** 63b05e5: shrinks to half, then widens the panel left. Test `every_place_name_clears_the_guides_glyph` (every place, five screen shapes) |
| Q15 | broken | the pause menu over the old man's line | `--capture x.png --script "0.1:realm:4,3:ledger,3.5:pause" --time 5`: `vh-qa-cap/q-pause-over-counsel-panel.png` | Paused while he speaks (every realm's welcome, a counsel), his panel stays under the menu: THE ROAD DOWN and the score are set over his words, and its "Enter skip" reads as the menu's | `dialogue::draw` ran on every play screen, the pause too | **fixed** ea82798. Test `the_pause_hides_the_line_being_spoken` |
| Q16 | broken | grid movement | Free movement switched off mid-stride (F4, shown on the controls page as "Free or grid movement", or the setting) next to a wall: `--monkey 72 600 cheats --stop 1` | On the grid he walks on into the wall (found inside the garden's fountain); 86 of 672 starts round the fountain did | The free mover leaves its heading and pace on the grid mover, which only checks a way when it reaches a centre | **fixed** 35fef65: on a centre with a way he may not take, he rests. Grid movement only (the harnesses play free; numbers unchanged). Test `switching_to_the_grid_mid_stride_never_walks_into_a_wall` |
| Q17 | ugly | Asmodeus's court (the Rose Bower) | `--monkey 81 180` (stop 16, at 142 s) | Asmodeus stands inside the rock of the bower's middle (his Thorn Line under way) for over three seconds | A body's fit tested only the four corners of its square; for a body wider than a tile (his 0.95) two corners straddled the fountain's single-tile tip and let his middle onto it (traced: `GridTerrain::slide` from (15.65, 13.47) to (15.65, 13.53)) | **fixed** 09034ef: points between the corners no more than a tile apart. Test `a_big_body_never_slides_onto_a_single_tile_of_rock`. Harness: below |
| Q8 | ugly | all body text (dialogue, HUD lines) | Scene I, beat 2: `vh-qa-cap/q-word-spaces-scene1-scream.png` | "NO! He took him! LET HIM GO!" reads "NO! Hetook him! LETHIMGO!" | The body face's space is 0.16 em (4 px at 24 px); `Overlay::layout` adds word spacing only to all-capital text | **fixed** 028be22: both faces set a space of at least 0.24 em (`text::WORD_SPACE`; Cinzel's own 0.25 unchanged; neither face kerns against the space). Captures before/after: `vh-fixes-cap/q8-*-before-after.png`. Test `a_word_space_sets_words_apart_in_both_faces_at_every_size` |
| Q9 | ugly | camera check (`--camera-check all`) | see the table under "Cameras" | The Hall of Mirrors shows the void through the floor (1039 hole poses in the chase survey, 64-98 in the top camera) and the chase camera clips there (72 poses); the Garden of Thorns clips in the walk with both cameras (56, 24); Cisterns and Bone Vaults hide him in 24-33 chase poses; every court's top camera loses him at the south gate (16,28) | Traced pose by pose (`VH_CAMERA_DBG` names the cube in front of each mark): the Hall's "holes" were its throne floor, drawn each frame near him and missing from the check's one frame; its hides and clips all at the mirror-doors' recesses, which send him through the glass at once (never framed in play); the Garden's walk clips at the court's rose arch, its hides under the pavilion, the Black Thorn and a bower; the vaults' hides behind arch piers (under the crown's fade) and the masks, banners and brackets of fading walls; the courts' at the tall portal of the south wall | **fixed** 5d13be2: those fade as trees do (the courts' portals, the Banquet's candle wheels, the Garden's court arch, pavilion, bowers and Black Thorn, the vaults' arch piers and a fading wall's ornaments); the check counts what a pose draws each frame before calling a ray a hole, and passes over tiles that send him on at once. Numbers in "Cameras" below. Tests `the_court_portal_fades_out_of_the_top_cameras_way`, `a_mirror_doors_recess_is_passed_through_at_once` |
| Q10 | ugly | the hub's top camera | `--camera-check hub` | Hides him at four platform corners (6,30 58,30 6,50 58,50), still so after the hub's art pass (1.0.21) | Each is the edge tile beside a side portal's north tower: the top camera, looking north over the portal, met the pier's courses; only what stands above the imposts faded | **fixed** d0de9ed: each pier below the imposts fades on its own as trees do, only while it stands in the way. Top survey 4 -> 0 hides of 405; the chase camera and both walks 0 as before. Test `a_portals_pier_fades_out_of_the_top_cameras_way`. Capture `vh-fixes-cap/q10-hub-before-after.png` |
| Q11 | ugly | the Burning Field | `--monkey 42 180` (stop 7) | A drifting wagon or tower on a slab overruns a standing foe, which stands inside it for seconds | The grid follows the slabs a whole tile at a time (rounded), so a block's rock steps onto the tiles beside it as its slab drifts; a foe there (carried, or at its post) was held fast in rock until the slab drifted on (traced: a werewolf at its post beside slab 17's block) | **fixed** 5ba209f: after every drift (and the horn's lurch) a foe in rock is shoved to the nearest spot with ground under it (never the lava). Test `a_drifting_block_never_holds_a_foe_in_its_rock`. Harness: below |
| Q12 | minor | the Pit, while the platform goes down | B or Q during the descent | The guide does not open and nothing is drunk for those five seconds | `tick` returns early while the platform moves | **fixed** 4e09132: the guide and the belt are read during the descent (the descent waits while the guide is up). Test `the_guide_and_the_belt_work_while_the_platform_goes_down`. The Pit harness (7 runs) unchanged |
| Q13 | minor | the camera choice before play | Continue, then Esc on "VIEW: TOP-DOWN / FOLLOW" | Esc does nothing; only a choice goes on | `update_view_choice` reads no Back | **fixed** db48bf1: Back closes the choice and keeps the camera in use when it opened. Test `back_closes_the_camera_choice_and_keeps_the_camera` |
| Q14 | minor | the test suite | full `cargo test` under load | `l3_steps_the_minimaps_zoom_and_its_turning_is_a_setting` failed once: it saved and read a setting in the save folder the other tests share | shared folder | **fixed** 1f7b84c (a folder of its own). Also seen twice: `clippy-driver` crashing with 0xc0000409 under heavy load (passes on retry; the toolchain, not the code) |
| Q18 | minor | the test suite | full `cargo test` while other work loads every core | `the_minimap_is_quick_and_sends_up_only_what_changed` failed once on its 5 ms time limit (passes alone) | a wall-clock assertion | **fixed** d077f45: the fastest of 20 batches of 10 draws against the same 5 ms (a batch the scheduler took the core from does not count) |

## Design calls for the user (measured, not decided)

1. **Decided and built (6ba86d2): the testing keys work in developer builds only.** **The testing cheats are live in the distributables.** F1 (the Cyclops), F2 (every key, at the
   gate; in a court it opens the gate past a living prince), 0 (the old man with 1000 silver), F5 god
   mode, F6 +1000 coins, F7/F9 the ring's bouts, F10 a rival, F12 the LEVELS map, F4 grid movement.
   F1 is the help key on many keyboards: a reviewer pressing it is thrown out of his level into the
   Cyclops' meadow. Proposal: keep them behind `dev` (as LEVELS is) in `build-windows.ps1` builds.
2. **Decided and built (c9d4fe7): after the end the run is closed, Start begins anew; New Game+
   beside it (72ff15c).** **After the ending, Continue goes back to Krampus.** The last checkpoint is `Stage::Lair`, kept
   after Scene II and the credits, so the title's Continue reads "Ragnar, Krampus's lair" and puts him
   before Krampus again (the finished mark stays). Many games do this; the alternative is to clear the
   run at `begin_ending` so Continue opens the heroes with the purse and unlocks kept.
3. **The adept's wall at the princes** (W9), measured on 1.0.18 (`--runs 1 4 0`, `--runs 1 4 1`,
   `--court-at`):
   - Order 0 (the vaults first): the adept clears the vaults and fells Mammon in all four runs, then
     ends at Lucifer (seeds 1, 3, 4: 53-58 min, 6-15 lives lost) or Leviathan (seed 2: 83 min, 20).
   - Order 1 (the Garden of Thorns first): all four runs end at Asmodeus, the first prince, at 21
     min with 18 lives lost; the Garden of Thorns was lost three times first (the elf-maids) and the
     court met with a 2/2/1 kit.
   - What it takes (`--court-at`, 240 s): Lucifer at tier 2 is not felled with kit 2, 3 or 4 (57-90%
     of him left; with kit 4 he is whole again after each of three falls: 81-84% left); Asmodeus at
     tier 1 falls to kit 4 (92 s) but not 2 or 3; Leviathan at tier 3 falls to kit 5 (220 s), not 3
     or 4. The prince is whole again after each of the hero's falls, so the adept's lives do not add
     up against him.
   - The expert reaches the ending in all 8 runs (87-112 min, 2-12 lives lost, 0-3 tries, all on the
     Burning Field) and fells every prince at tier 4 and 7 in 33-69 s without a death; the novice
     loses every life in the vaults (2 of 2) and to every prince at tier 1 (7 of 7).
   - **Softened in W10** (WARBAND.md: the princes keep their wounds across the hero's falls, and are
     tuned to the adept's kit): the adept now fells every prince it reaches; its walls are the
     Rotting Feast and the Pit.
4. **Runes are unused in a whole run.** The expert ends with 63-80 runes unspent in all 8 runs (purse
   228-1442 silver); the adept with 3-21. The relics unseal tiers before runes are wanted (the bots'
   rule never needs them).
5. **Word spacing** (Q8) and the **camera regressions** (Q9): fixed on `fixes`.

## What was checked, and passed

**The whole road, by bots** (CPU, quick profile, 1.0.18 and again on this branch):
`--run` for the expert, adept (seeds 1-4, both orders) and novice (seeds 1-2); `--court` for all seven
princes at expert tier 4 and 7, adept tier 4, novice tier 1; `--general` for all seven; `--pit` (expert,
adept, novice at tier 8; expert tier 7 seed 2); `--lair` (all three skills); `--balance 2`, `--crawl 3`,
`--rivals 6 1 1 1`, `--mastery 2`, `--kits 2`, `--stage 4..7 2`, `--stage-three 2`. No panic, no stall:
every realm ends (cleared or at its time), every court ends (prince slain or the tries spent), the
Pit reaches the bottom or the lives run out, Krampus falls to the expert in 165 s and the ending
begins. Princes can be beaten (expert 14/14) and can win (novice 7/7). The one run whose numbers
changed is Q3's.

**The monkey** (`--monkey SEED SECS [cheats]`, new in `game/qa.rs`): every stop on the LEVELS map (43)
entered as a player enters it, then played by the expert bot, a monkey pressing keys, clicks and
controller buttons (pause, every page, the old man, potions, the camera, the guide, menus picked at
random) and hands off, in turns; it reports panics, a screen held past two minutes, the hero or a foe
in rock, NaN positions and the lists that grow. Run: 6 seeds x 180 s, 6 x 180 s again after the fixes,
4 x 600 s, half with the testing keys pressed too, and single stops for 300-3600 s: about 65 hours of
play (and some 30 more in the whole runs and courts). Findings: Q3, Q6, Q11 only. No panic anywhere, no screen ever held.

**Every stop on the LEVELS map**: entered from the title's LEVELS (the monkey) and captured with
`VH_GPU_STRICT=1` at 1920x1080 (title, Scene I's two sets, the garden and its pause, the Cyclops, the
hub, the seven realms and courts, the Pit at levels 1, 4, 7 with its bell and descent, the lair, the
taunt, Krampus's fall, the gym, Cisterns, Bone Vaults, the old man's wares and workshop, the Ledger,
the full map, settings, controls, LEVELS, the loading card, a general's post, the ending's words and
credits): no GPU validation error, no black or broken frame, no missing model. Q7 and Q8 came from
these. (The aspect-ratio captures are below.)

**The bestiary (1.0.19, merged in)**: opened from the title, the pause menu and the old man's guide
(Back goes back to his words), every group and page turned, its saved book (`vh.bestiary`) cut short,
its numbers swapped and its bytes flipped (`the_bestiary_opens_everywhere_and_a_spoiled_book_never_stops_it`),
and among the pages opened at every special moment. A save without the key loads with an empty book.
The monkey on the merged build (2 seeds x 180 s, 1 with the testing keys): no panic; Q11 and Q17 only.

**Every screen shape**: the garden, its pause, a realm with the old man speaking, two courts, the Pit's
bell, Krampus at the cage, the wares and the workshop, the Ledger, LEVELS, settings, controls, the
full map, the loading card and Scene I captured again at 1920x1200 (16:10), 2560x1080 (21:9) and
1600x1200 (4:3): nothing off screen or overlapping beyond Q7 and Q15 (now fixed).

**Performance** (`--perf all`, VH_GPU_STRICT, the merged build, RTX 5070 Ti laptop, 1080p): no scene
FAILED (every fight within its stall budget, every load counted). Over a 60 FPS frame at p99: the Pit
(17.4 ms), Scene I's souls (18.2), destruction (18.2), the descent (13.6, fine), all GPU-bound (GPU p95
15-17 ms); the portal and start scenes' worst frames (97-99 ms) are their level loads behind the
loading card. Not optimised here (the hub agent owns the art).

**The GPU validation error seen once** (UI.md, `create_texture` under heavy load): not reproduced. Six
heavy captures at once at 2560x1440 with VH_GPU_STRICT, twice (the vaults, Krampus's fall, the Pit with
the bot fighting, the garden with five rows of foes, the souls, Beelzebub's court with the bot): no
error. 45 single captures and the perf run: none. Likely cause, by elimination: the only
`create_texture` calls whose size varies are the screen-sized targets and the probes; an allocation
that fails under memory pressure is reported by wgpu as an error in `create_texture`, and this machine
runs out of commit memory with a few builds and harnesses beside the game (the harnesses here aborted
with "memory allocation failed" when a dozen ran at once). It is logged to `gpu-errors.log` if it comes
again.

**Save and Continue** (`continue_comes_back_to_every_kind_of_checkpoint`): the game closed and started
again at every kind of checkpoint (each dungeon and arena, the old man's after the Garden's Cyclops
and after a prince, the hub, every realm, every court, the gym, each of the Pit's seven levels, the
lair; the three heroes in turn, belts, lives and keys): the title offers Continue with its caption,
Continue restores the hero, keys, lives and belt and the place, and play goes on (the wares lead on to
the hub with the sin's key set, or the next dungeon). A v1 save (from before the Portals of Hell)
still loads. **Spoiled saves** (`a_spoiled_save_never_stops_the_game`): the run's checkpoint, the
purse and unlocks and the Warband cut short at every point, every number swapped for 0, 255, 65536,
past u32 or negative, bytes flipped: nothing panics, and whatever reads back is played (the wares on
every tab, the HUD at four screen shapes, Continue, the Ledger's board, a realm with its rivals). A
broken settings file falls back to its copy (existing test). Settings and bindings round-trip (existing
tests in `settings.rs` and `input/bindings.rs`).

**The story** (`the_story_skips_from_every_beat`, `the_story_plays_through_to_the_garden`): Scene I
skipped with Esc at every one of its beats goes to the heroes; Esc there goes back to the title, and
Start plays the story again from its first beat; played through with Enter it reaches the heroes,
the camera and the Garden. Scene II skipped at every beat goes to the title. The ending to the title
and Esc there are covered by the existing tests in `ending.rs`.

**Edge cases**: the pause, every page over it (settings, controls, full map, the Ledger, LEVELS) and
the old man called, during the Pit's descent, every prince's change of phase, Krampus at the cage and
his fall (`every_special_moment_survives_the_pause_and_the_old_man`): each moment goes on as it should
(the platform reaches the next level and saves there, each prince fights on and falls, the end
begins). Dying everywhere, on his feet and mid-leap (the garden, the meadow, the hub, every realm and
court, the Pit at 1 and 6, the lair): he rises on floor he can stand on, and the last life goes to GAME
OVER, then the title, where Continue comes back to the same place
(`dying_anywhere_brings_him_back_on_floor_and_the_last_life_to_the_title`). Focus loss (Q1) and
controller loss (Q5) are now tested; resizing and fullscreen cannot be driven headless.

**Memory over long sessions** (`memwatch.ps1` sampling the process): a whole expert run in one game
grows 175 to 490 MB as each new level's caches fill; `--reload-soak 12` (every place on the map, 12
times over, one game) sits flat at 481-486 MB from the first round to the last; single stops played
for an hour of game time (the vaults, the lair, the Pit, the hub, the garden with the testing keys
hopping everywhere) stay between 120 and 430 MB. No leak: the growth is the per-level caches, bounded.
(Running a dozen harnesses side by side on this machine runs out of commit memory and they abort with
"memory allocation failed": the machine, not the game.)

**Cameras** (`--camera-check all`, now also the Pit's seven levels and the lair): the Pit and the lair
pass in both cameras; the garden, the Cyclops' meadows, the gym, the Grotto (1 pose), the Mire, the
vaults pass or nearly. Failures (survey/walk, chase/top; clips, hidden poses, holes):

| Level | Clips | Hides him | Holes |
|---|---|---|---|
| dungeon1 (Cisterns) | 5 chase | 33 chase, 2 top | 0 |
| dungeon2 (Bone Vaults) | 3 chase | 24 chase | 0 |
| hub | 0 | 4 top | 0 |
| realm0 (Hall of Mirrors) | 72 chase | 133 chase | 1039 chase survey, 85 chase walk, 64 top survey, 98 top walk |
| realm2 (Burning Field) | 6 chase | 11 chase | 0 |
| realm5 (Rotting Feast) | 0 | 3 chase, 1 top | 0 |
| realm6 (Garden of Thorns) | 8 chase survey, 56 chase walk, 24 top walk | 7 chase, 3 top | 0 |
| courts 1-6 | 0 | 1-3 (top, at the south gate 16,28; chase walk 1) | 0 |

## What could not be tested headless (for the hand playthrough)

- Device loss (Q2): sleep and resume, a driver update, unplugging an external GPU's monitor.
- Window resize, fullscreen toggles (F11, Alt+Enter), minimising during a fight and during a load.
- A real controller unplugged and plugged back (Q5 is tested at the game's end; the app's polling is
  not), and a second controller.
- Audio: the voices, the hymn and the credits' song, mute, music ducking on pause.
- Alt-tab during the loading card and the countdown (Q1's fix), in the real window.
- The camera regressions (Q9) by eye: whether the Hall of Mirrors' holes are its glass floors (a lie
  by design) or the void.
- The W9 hand-playthrough checklist (WARBAND.md) still stands: the five realms the bots cannot read
  were played as stand-ins.

## Tools added

- `--monkey SEED SECS [cheats] [--stop N]` (`game/qa.rs`): the soak above; `VH_QA_TRACE=FROM:TO`
  traces his position, his fighter, the shove and the prince every frame in that stretch.
- `--reload-soak ROUNDS`: every place on the map in turn, for a memory watcher.
- `--camera-check` knows `pit1`..`pit7` and `lair`.
- Tests in `game/qa_tests.rs` (Continue everywhere, spoiled saves, the story's skips, special
  moments, dying anywhere, focus and controller loss, the gate cheat, the pause over a spoken line,
  the bestiary), and in `logic/hero.rs` (the hard shove, the switch to the grid), `game/general.rs`
  (the teeth shut the general's ways), `render/hud/play.rs` (every place name on its panel).

## The fixes pass (branch `fixes`, 2026-10-09)

Every open issue fixed (Q10, the hub's, once the hub's art was merged: d0de9ed), each in its own commit with a test where one can
catch it, and the user's two decisions built: the testing keys only in developer builds (6ba86d2:
`Game::testing_keys`, test `testing_keys_are_inert_outside_developer_builds`) and the run closed at
the end (c9d4fe7: no Continue, Start begins anew, the bestiary kept; test
`after_the_end_start_begins_anew_and_the_bestiary_stays`).

**The monkey after the fixes** (`--monkey 81 180`, `--monkey 42 180`, `--monkey 7 180 cheats`, before
the camera work, which changes no simulation): no findings (on the base build seed 81 found Q11 and
Q17).

**Harness numbers that moved** (quick profile, base 6b45a7b against the branch). Only Q17 and Q11
change the simulation:

- Q12 (the belt in the descent): the Pit, 7 runs (expert t8 s1, t7 s2; adept t8 s1-2, t4 s3; novice t8
  s1-2): identical. The lair, three skills: identical. (The ending's guard after Krampus too.)
- Q17 (the fit of bodies wider than a tile): the seven courts at expert t4/t7, adept t4, novice t1
  (seed 1): every outcome the same; Mammon expert t4 68.5 -> 65.7 s, expert t7 the same time, adept t4
  173.3 -> 175.0 s; Belphegor adept t4 166.2 -> 164.8 s (72 -> 24 taken); Asmodeus adept t4 91.7 s with
  no death -> 167.9 s with one (he no longer stands in his fountain's tip). The Pit: the expert's t8 s1
  kills 25 not 23 at level 7; the other six runs identical. The lair: identical.
- Whole runs (`--run`, expert and adept, seeds 1-2, both orders; base / Q17 alone / Q17 and Q11). A
  small change anywhere shifts a whole run (its foes, its purse, its tiers), so these vary within the
  spread the runs already had:
  - expert s1 o0: ending 87 min, 9 lives / 117 min, 10 / 90 min, 12
  - expert s1 o1: ending 96 min, 5 / 102 min, 7 / 110 min, 9
  - expert s2 o0: ending 90 min, 2 / 94 min, 3 / 105 min, 10
  - expert s2 o1: ending 112 min, 8 / 97 min, 3 / 113 min, 8
  - adept s1 o0: Lucifer 58 min / 57 / 57; adept s1 o1 and s2 o1: Asmodeus 21 min / 22 / 22
  - adept s2 o0: Leviathan at 83 min / Lucifer at 56 min (Q17) / Lucifer at 56 min

  The expert still reaches the ending in every run, the adept still ends at a prince. Q11's share is
  the Burning Field: foes that stood held in rock (harmless) fight now; the expert's Field part ran
  10-25 min before and 15-26 min after, as varied as ever.

**Cameras** (`--camera-check all`, base against the branch; the walks vary a little with the
simulation; on this branch before the camera work the Garden of Thorns' chase walk clipped 56 times,
as QA saw):

| Level | Camera | Pass | Clips | Hides him | Holes |
|---|---|---|---|---|---|
| dungeon1 (Cisterns) | Third | survey | 5 -> 0 | 33 -> 0 | 0 |
| dungeon1 | Top | survey | 0 | 2 -> 0 | 0 |
| dungeon2 (Bone Vaults) | Third | survey | 3 -> 0 | 24 -> 2 | 0 |
| realm0 (Hall of Mirrors) | Third | survey | 72 -> 0 | 133 -> 0 | 1039 -> 0 |
| realm0 | Third | walk | 0 | 0 | 85 -> 0 |
| realm0 | Top | survey / walk | 0 | 0 | 64 / 98 -> 0 |
| realm6 (Garden of Thorns) | Third | survey | 8 -> 0 | 7 -> 0 | 0 |
| realm6 | Top | survey | 0 | 3 -> 0 | 0 |
| realm6 | Top | walk | 24 -> 0 | 0 | 0 |
| courts 1-6 | Top | survey | 0 | 1 each (the south gate) -> 0; court5 3 -> 0 | 0 |
| court1, court4 | both | walk | 0 | 1 -> 0 | 0 |

Also the hub's top survey (Q10, against the merged hub art): 4 -> 0 hides. Unchanged: dungeon0 1 hide, realm1 1 and 1, realm2 6 clips and 11 hides (the
Burning Field, as at RP02's first pass), realm5 3 and 1, court2's walk 1, court5's chase survey 3 (by
the dais). Every level is at RP02's old numbers or better. Captures before|after:
`vh-fixes-cap/q9-court1-before-after.png` (the portal fades), `q9-cisterns-before-after.png` (the pier
fades), `q9-thorns-before-after.png`.

**Tools added**: `VH_LOSE_DEVICE=SECS` (the GPU device lost on purpose, Q2); `--camera-check` with
`VH_CAMERA_ALL` (every failing pose) and `VH_CAMERA_DBG` (the cube in front of each of his marks), and
it now counts what a pose draws each frame before calling a ray a hole, and passes over tiles that
send him on at once (a mirror-door's recess).

**Still open**: the design calls 3 and 4 (the balance agent's); the hand
playthrough items above.

**Then built** (the user's calls after the pass, same branch):

- **`--dev` only in developer builds** (e3acc49): the flag turns the developer's map and the testing
  keys on only where `DEV_BUILD` holds (debug builds, `tools/release.sh`'s `VH_DEV_MENU`); in
  `build-windows.ps1` and web builds it is ignored (`Game::ask_dev`). The harnesses that press the
  testing keys (`--capture` scripts, `--monkey ... cheats`) are unchanged. Test
  `dev_on_the_command_line_is_ignored_outside_developer_builds`.
- **New Game+** (72ff15c; the proposal above, approved): once the tale is told the title offers
  NEW GAME+ beside Start. It begins again from Scene I with the finished run's kit (every hero's
  tiers, mastery and loadout), its purse and its living rivals seeded back into the Ledger, every foe
  and prince a tier harder: an offset on top of the numbers (`Game::realm_tier` + the level, so the
  dealer's stage, the packs' weight, each prince's and Krampus's tier; the Garden a stage deeper),
  never a change to them. Finished again it offers NG+2, then NG+3 (`newgame::NG_MAX`). The level
  goes with the run's checkpoint (`Run::ng`, a "v3" line; a first run's line is "v2" as before) and is
  shown on the title (the button's caption, Continue's), over the HUD's place name and in the pause
  menu ("THE ROAD DOWN   NEW GAME+1"). Start beside it is a fresh adventure. A first run is
  bit-identical in every harness (`--run` expert and adept seed 1 both orders, the seven courts at
  expert t4 and adept t4, `--pit 2 8 1`, the 16 bouts: the same output as before, wall-clock lines
  aside; checked again after merging W10's princes, 6e64ac7). `--run SKILL SEED ORDER ng+N` plays New Game+ N times over (WARBAND.md §14). Test
  `after_the_end_new_game_plus_keeps_the_kit_a_tier_harder`. Captures `vh-fixes-cap/ng-title.png`,
  `ng-hud.png`, `ng-pause.png`.

