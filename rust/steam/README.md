# Shipping Adventures of the Ancient Blades on Steam

What is already done in the code, and what only you can do on the Steamworks side.

## Ready in the build

- **Standalone executable.** `Adventures of the Ancient Blades.exe` has everything baked in: the voxel art and the sound effects are generated in code, and the music (twelve OGG tracks in `assets/music`), the two fonts (Cinzel and Alegreya Sans, `assets/fonts`) and the animation clips (`assets/anim`) are embedded at build time. The C runtime is linked statically (`.cargo/config.toml`), so players need no Visual C++ redistributable. No console window in release builds.
- **Direct3D 12 first** on Windows, with Vulkan as the fallback (set `WGPU_BACKEND=vulkan` to force it). That keeps the Steam overlay and Steam's recording happy.
- **Borderless fullscreen by default** in release builds, toggled with F11 or Alt+Enter and remembered. Pass `--windowed` or `--fullscreen` to override it (for example in Steam launch options).
- **Controllers** through gilrs (XInput, DualShock/DualSense, Steam Deck), with the defaults of `src/input/bindings.rs` (UI.md §3; every play control can be rebound in Settings): left stick runs, right stick aims the camera (R3 recentres), A jumps (in a fight, the evade), X attacks, LB raises the shield, B opens the old man's guide (there are no bombs), Y calls the old man, RB drinks the chosen potion and the D-pad's left and right choose it, LT the Bifrost Step, RT the archangel's power, D-pad up or View the map, D-pad down the camera, L3 the minimap's zoom, Menu pauses. The old man's wares and every menu are fully navigable with the stick or D-pad and A. Keep Steam Input enabled and pick "Gamepad" as the default template.
- **Pause on focus loss**, so opening the Steam overlay pauses the game.
- **Save data** lives in `%APPDATA%\VoidHarvest\settings.txt` (the settings and bindings, the progress, the bestiary and the saved adventure; the copy before the last write is `settings.txt.bak`). A panic writes `crash.log` there too.
- **Achievements** (build with `-Steam`, `src/steam.rs`): only three so far, with API names older than today's game: `ACH_FIRST_NUGGET` (the first key taken), `ACH_HARVEST` (the first time through a dungeon's gate) and `ACH_CYCLOPS` (the first boss slain). Today's set is BACKLOG.md B28. The testing keys (F1 and the rest) work only in developer builds, so they cannot unlock anything in a distributable. Create the achievements with exactly these API names in the partner site before testing.
- **Icon** embedded in the exe, generated from Ragnar's voxel front view (`AdventuresOfTheAncientBlades --write-icon assets/icon.ico` rebuilds it).

## Before you can upload

1. **Original cast.** The hero is now Ragnar, a viking of our own, and the Knightmare-styled Popolon is gone, which removes the Konami-specific character the first port flagged (still worth a quick trademark search on the new title before the store page goes up). The shipped executable is `Adventures of the Ancient Blades.exe` (Cargo builds it as `AdventuresOfTheAncientBlades.exe`, since Cargo names cannot hold spaces, and `build-windows.ps1` renames it).
2. Join Steamworks (Steam Direct fee, tax and bank forms), create the app, and note the **App ID** and the **Windows depot ID**.
3. Replace `480` / `481` in `app_build.vdf`, `depot_build_windows.vdf` and `steam_appid.txt`. Keep `steam_appid.txt` out of the depot; it is only for running the exe outside Steam while testing.
4. Set the launch option: executable `Adventures of the Ancient Blades.exe`, OS Windows.
5. Steam Cloud (optional): Auto-Cloud with root `WinAppDataRoaming`, subdirectory `VoidHarvest`, pattern `settings.txt`.
6. Third-party licenses: the Rust crates are MIT/Apache-2.0, font8x8 is public domain, the fonts are SIL OFL 1.1 (`assets/fonts/OFL-*.txt`, to ship beside the game), and the music in `assets/music` needs its licences noted too. Ship a notice file; `cargo install cargo-about` then `cargo about generate` produces the crates' part.

## Building and uploading

```
powershell -File build-windows.ps1 -Steam
steamcmd +login <builder-account> +run_app_build C:\path\to\void-harvest\rust\steam\app_build.vdf +quit
```

`build-windows.ps1` fills `dist/windows/` with `Adventures of the Ancient Blades.exe` and `steam_api64.dll`; `app_build.vdf` uploads that folder as the Windows depot. Use a dedicated builder account with Steam Guard.

## Store page and review

- Capsules: header 920x430, small 462x174, main 1232x706, vertical 748x896, library capsule 600x900, library hero 3840x1240, logo (transparent PNG). At least five 1920x1080 screenshots; `AdventuresOfTheAncientBlades --capture shot.png --size 1920x1080 --time 8 --script "..."` renders them headless (see `src/capture.rs`).
- A "Coming Soon" page has to be live for at least two weeks before release, and both the store page and the build go through Valve's review.
- Steam Deck: the UI scales with the shorter window side, so 1280x800 stays legible. Run the Deck compatibility checklist once the app exists.
