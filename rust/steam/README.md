# Shipping The Great Adventures of Sebastian on Steam

What is already done in the code, and what only you can do on the Steamworks side.

## Ready in the build

- **Standalone executable.** `The Great Adventures of Sebastian.exe` has everything baked in: voxel art, font, music and sound are all generated in code. The C runtime is linked statically (`.cargo/config.toml`), so players need no Visual C++ redistributable. No console window in release builds.
- **Direct3D 12 first** on Windows, with Vulkan as the fallback (set `WGPU_BACKEND=vulkan` to force it). That keeps the Steam overlay and Steam's recording happy.
- **Borderless fullscreen by default** in release builds, toggled with F11 or Alt+Enter and remembered. Pass `--windowed` or `--fullscreen` to override it (for example in Steam launch options).
- **Controllers** through gilrs (XInput, DualShock/DualSense, Steam Deck): left stick runs, right stick aims the camera (R3 recenters), A jump, X or RT attack, B or LB bomb, Y camera, Start begins and pauses. Keep Steam Input enabled and pick "Gamepad" as the default template.
- **Pause on focus loss**, so opening the Steam overlay pauses the game.
- **Save data** lives in `%APPDATA%\VoidHarvest\settings.txt` (high score, mute, fullscreen). A panic writes `crash.log` there too.
- **Achievements** (build with `-Steam`): `ACH_FIRST_NUGGET` (first nugget), `ACH_HARVEST` (all 25 nuggets) and `ACH_CYCLOPS` (the first boss slain). The F1 boss cheat is meant for testing; take it out (the `Cheat` key in `src/input.rs`) or accept that it can unlock `ACH_CYCLOPS` before you ship. Create them with exactly these API names in the partner site before testing.
- **Icon** embedded in the exe, generated from Sebastian's voxel front view (`TheGreatAdventuresOfSebastian --write-icon assets/icon.ico` rebuilds it).

## Before you can upload

1. **Original cast.** The hero is now Sebastian, a viking of our own, and the Knightmare-styled Popolon is gone, which removes the Konami-specific character the first port flagged (still worth a quick trademark search on the new title before the store page goes up). The shipped executable is `The Great Adventures of Sebastian.exe` (Cargo builds it as `TheGreatAdventuresOfSebastian.exe`, since Cargo names cannot hold spaces, and `build-windows.ps1` renames it).
2. Join Steamworks (Steam Direct fee, tax and bank forms), create the app, and note the **App ID** and the **Windows depot ID**.
3. Replace `480` / `481` in `app_build.vdf`, `depot_build_windows.vdf` and `steam_appid.txt`. Keep `steam_appid.txt` out of the depot; it is only for running the exe outside Steam while testing.
4. Set the launch option: executable `The Great Adventures of Sebastian.exe`, OS Windows.
5. Steam Cloud (optional): Auto-Cloud with root `WinAppDataRoaming`, subdirectory `VoidHarvest`, pattern `settings.txt`.
6. Third-party licenses: the Rust crates are MIT/Apache-2.0, font8x8 is public domain. Ship a notice file; `cargo install cargo-about` then `cargo about generate` produces one.

## Building and uploading

```
powershell -File build-windows.ps1 -Steam
steamcmd +login <builder-account> +run_app_build C:\path\to\void-harvest\rust\steam\app_build.vdf +quit
```

`build-windows.ps1` fills `dist/windows/` with `The Great Adventures of Sebastian.exe` and `steam_api64.dll`; `app_build.vdf` uploads that folder as the Windows depot. Use a dedicated builder account with Steam Guard.

## Store page and review

- Capsules: header 920x430, small 462x174, main 1232x706, vertical 748x896, library capsule 600x900, library hero 3840x1240, logo (transparent PNG). At least five 1920x1080 screenshots; `TheGreatAdventuresOfSebastian --capture shot.png --size 1920x1080 --time 8 --script "..."` renders them headless (see `src/capture.rs`).
- A "Coming Soon" page has to be live for at least two weeks before release, and both the store page and the build go through Valve's review.
- Steam Deck: the UI scales with the shorter window side, so 1280x800 stays legible. Run the Deck compatibility checklist once the app exists.
