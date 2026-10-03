# Technical art and rendering: status

This tracks the *Technical Art and Rendering Plan* (version 1, 3 October 2026; native Windows and Steam,
GTX 1080 at 30 FPS on a reduced preset, RTX 3060 at 60 FPS, native 1920×1080, no upscaling). Task
IDs (RP for the rendering programmer, TA for the technical artist) are the plan's own.

**Measured on:** the development laptop only (RTX 5070 Ti Laptop GPU, Ryzen AI 9 HX 370, Windows 11,
DX12). That is neither target GPU, and its clocks shift between runs, so only interleaved before/after
comparisons on this machine are meaningful. **Nothing here has been measured on a GTX 1080 or an RTX
3060 yet**; the plan's acceptance gates are still open.

## Done

### Stage A: baseline (RP07, RP08)

- **RP07, instrumentation** (`render/profile.rs`). GPU timestamps per pass (shadow cache, near and far
  cascade, main, AO, bloom, shafts, composite, HUD), read back up to three frames late so nothing waits.
  Per frame it counts draws, instances, triangles, bytes uploaded, static chunks drawn, shadow casters,
  rigs, and lights requested and dropped. CPU time is split into simulation, emission, HUD, upload and
  recording. F3 shows it all above the frame rate, and `--bench` and `--perf` print it.
- **Effect switches** (`--fx-off`): `shadows, lights, grass, materials, fog, ao, bloom, dof, grade, prepass,
  mist, backface`. Each one captures the same scene without one effect, as the plan's per-effect
  comparisons need.
- **RP08, performance captures** (`src/perf.rs`, `--perf [all|names] [--frames N] [--size WxH]
  [--fx-off …] [--out dir]`). There are eleven scenes: close-up, garden, corridor, vista, 30 foes,
  souls, fast rotation, camera switching, destruction, portal arrival, vault. Each has a fixed seed and
  script and gets a second of untimed warm-up. Frames are paced as in the game (one simulation step per
  frame, at most two frames in flight). The report gives the median, 95th and 99th percentile and worst
  of wall, CPU and GPU time, plus the first-frame cost. It writes a Markdown summary (machine, driver,
  backend, build `git hash[-modified]`, settings) and a CSV of every frame per scene to `perf/`.
- **Look Dev room** (TA01, `render/view/lookdev.rs`, `--lookdev neutral|dungeon|realm:N`). Every voxel
  material stands on a stepped block, once at neutral grey and once in its usual colour. Beside them are
  the shared palette, the seven proposed realm accents (each capped with its realm's identity colour), an
  eleven-step value ramp, and Ragnar and a draugr for scale. It can be lit three ways: a neutral chart
  light, the dungeons' moonlit night with two torches, or a realm's tint.

### Stage C: geometry (RP09 to RP11, part of RP16)

- **RP09, static chunks.** Static scenery is sorted into 8×8 m chunks with bounds and drawn front to back
  after a CPU frustum test. Each shadow cascade culls its own casters, so off-screen objects still cast.
  Tree fades still patch in place.
- **RP16 (part), cached static shadow.** The coarse cascade's static part is drawn once and copied in each
  frame. It is drawn again only when the light, its box or the static scenery changes.
- **Depth prepass** for opaque geometry: the expensive shading then runs once per pixel. Fades dither
  the same way in both passes; water, clouds and glows are left out of the prepass.
- **RP10 and RP11, resident rigs** (`voxels::RigMesh`). Each rig is meshed once, per part, keeping only
  the voxel faces nothing in the same part covers; faces at joints stay. Each frame uploads only a
  48-byte record and the part matrices per rig, instead of 96 bytes per voxel. Ghosts and fades use a
  mesh with every face, since their look depends on the inner layers.
- **RP11, static surfaces** (`render/faces.rs`). A static cube's face is dropped when an identical cube
  (same turn and size) sits right against it and can hide it. The rest are drawn as single quads that
  read their cube from a storage buffer, so fades and hides still patch the cube in place. The index is
  kept after load: a wall blown up, a coin taken or a hedge burnt reopens only the neighbouring faces,
  and new cubes go into spare slots. Nothing makes the whole level re-upload any more.
- **Back faces skipped** in the vertex shader for static faces and solid rigs. All four corners of a face
  decide together; fading cubes keep their back faces.
- **Prebuilt sets kept on the GPU** (`view::InstanceSet`): ground mist, Scene I's sets and the
  character-screen cavern were re-sent every frame and are now sent once.
- **Start-up**: the pipelines that compile the scene shader are built in parallel (the DX12 shader
  compiler, FXC, is slow). The HUD's first draw built its sprites in quadratic time; that is now linear.

### RP12: levels of detail for rigs

- **Levels** (`Rig::lod`): every rig has three, itself plus copies at half and a quarter of the
  resolution, made on first use and cached on the rig. Parts and joints are the same at every level, so
  one pose drives them all. A foe's levels are made when it joins the view, not mid-fight. The
  hand-kept `foes_far` copies and the fixed 6-unit switch are gone.
- **Choosing a level** (`frame::LodPolicy`): by projected size, i.e. pixels per voxel under the actual
  projection (perspective or the orthographic map) and screen height. A rig drops to half below 2 pixels
  per voxel and to a quarter below 1, so each level is used while its own voxels stay under about 4
  pixels. That matches the old rule at 1080p, and now holds at any resolution, for any rig and camera.
- **Hysteresis:** coming back up needs 20% more than going down. The memory is kept per rig and per
  turn in the frame's draw order.
- **Shadows:** the near cascade casts each rig at its on-screen level, so self-shadowing stays aligned.
  The coarse far cascade casts one level coarser.
- **Knobs:** `--lod-bias B` (above 1 drops detail sooner, as a low preset would; 0 never drops) and
  `--lod-debug` (half-resolution rigs tinted green, quarter red). F3 shows the count at each level.
- **Measured:** rig voxels drawn fell from 25.7k to 15.4k in the Garden and from 17.4k to 1.3k on the
  map view; frame time was about the same (rigs are now a small share). The stage-7 giants cost about
  0.7 ms more: their voxels are bigger, so by projected size they stay whole past the old fixed 6 units.
- **TA review:** at half resolution head, helmet, beard, weapon and shield all read. At a quarter the
  eyes average away, but that level is only used when the whole figure is about 15 pixels wide.
- **Static chunks get no LOD yet, by measurement:** dropping every chunk beyond 10 m entirely did not
  lower GPU time in the Garden (5.2–6.2 ms either way). After the prepass and back-face skipping, far
  scenery is cheap. Revisit on the vista and top-down cameras and on the GTX 1080.

### RP13: linear HDR (the default since approval)

Approved on 3 October 2026 from the side-by-side captures, then made the only path: the old
per-surface tone mapping into an 8-bit target is gone, and so is the `--hdr` switch.

**How it works:**
- **Target:** half floats holding light times the exposure (`shaders.wgsl`, output section: `finish`,
  `fogged`, `from_display`).
- **Fog and blending in light:** fog, alpha blending and additive effects all mix in linear light. The
  fog colour (and the clear behind everything) becomes the light that tone maps to it (`untonemap`).
- **Sky:** the sky, drawn as display colour, is turned into light the same way, so it looks as before.
- **Bloom:** taken from the tone-mapped light, so its energy matches the old look; a lit marble floor
  would otherwise drown the hero. It joins the light before the final tone map.
- **Encoding:** the composite tone maps (the same ACES fit) and encodes to sRGB exactly once.
- **Glow orbs:** they now add light rather than display colour, which made their faint rims visible;
  they get a steeper falloff so halos keep their drawn size.

**Compared side by side** (Garden, hub, meadow, Vaults, Burning Field, Scene I): close to the original.
Bright sources (torches, lamps, fire, lava edges) are punchier and keep their order of brightness. The
souls are a little paler, and the Vaults' floor a little brighter. Approving or tuning this is TA02's
job.

**Cost at full GPU clocks:** about 0–8% (Garden +0%, 30 foes +6%, Scene I +8%). At this laptop's low
memory clock it was 13–22%: the float target doubles the 4× MSAA colour traffic. Check it on the GTX 1080.

**Not in this step:** AO still multiplies all light (RP15 moves it to indirect light only); HDR monitor
output is a separate feature.

### RP14: physically based materials

- **Material model** (`shaders.wgsl`, `Surface`): base colour, roughness, metalness and a specular
  level (0.5 means 4% reflectance), with emission as before. The voxel materials map onto it:
  | Material | Roughness | Metalness | Specular | Notes |
  |---|---|---|---|---|
  | stone | 0.9 | 0 | 0.5 | grass 0.8 |
  | iron | 0.45 | 0.7 | — | worn |
  | gold | 0.38 | 0.7 | — | |
  | fur | 0.75 | 0 | 0.25 | keeps wrap and sheen |
  | cloth | 0.9 | 0 | 0.3 | keeps wrap and sheen |
  | leather | 0.55 | 0 | 0.45 | |
  | skin | 0.5 | 0 | 0.5 | wraps light |
  | ivory | 0.35 | 0 | 0.6 | |
  | wood | 0.75 | 0 | 0.4 | |
  | slime | 0.12 | 0 | 1.0 | |
  | marble | 0.15 | 0 | 1.5 | about 12%, deliberately glossier than real stone so the Vaults shine |
- **Lighting:** GGX specular (Smith-Schlick visibility, Schlick Fresnel) for the sun or moon and every
  point light. Diffuse is Lambert weighted by (1 − metalness), with the soft materials' light wrap kept.
- **Environment:** each light change redraws a 128-pixel cube of the surroundings as light: the
  hemisphere's sky and ground (irradiance over π) plus the sky itself (moon, stars, dusk sun). Its five
  further mips are prefiltered by GGX importance sampling, one per roughness step (`fs_env`, `env.wgsl`).
  Surfaces reflect it through a split-sum BRDF lookup computed at start-up (`render/brdf.rs`, 64×64).
  Metals also reflect a share of the direct light, standing in for the lit walls and ground until
  rooms get their own probes (RP18).
- **Metal colours:** the game's metals were painted as they look worn and lit (dark iron at 2%), so a
  metal's reflectance is its hue raised to at least 35%. **For TA04:** author metals by reflectance from
  now on.
- **Pixel footprint:** a voxel only a few pixels across gets rougher (up to 0.6), so small highlights do
  not glitter.
- **Look against HDR alone** (sheet in the session):
  - Dielectrics gain a soft grazing sheen.
  - Lit floors show the lights' reflections (Scene I's flagstones).
  - Metals read as worn metal: the helmet now carries a little of the dusk sky.
  - Marble's light pools are tighter.
  - The neutral chart is slightly lighter on flat surfaces (physically correct for rough stone).
- **Cost:** about 2–5% GPU in the cleaner round (Garden 5.58 → 5.70 ms, 30 foes 8.24 → 8.45 ms). The
  environment passes run only when the light changes.

### Results (interleaved, same machine and clock state, 1920×1080, `--bench`)

| Scene | Before | After |
|---|---|---|
| Garden of Krampus | 31.2 ms | 18.0 ms |
| 30 foes, stage 2 | 38.6 ms | 23.2 ms |
| Scene I, the souls | 14.6 ms | 10.7 ms |
| Portals of Hell (vista) | 11.8 ms | 9.5 ms |
| Vaults of Mammon | 12.2 ms | 8.6 ms |
| Garden of Thorns | 13.3 ms | 9.7 ms |

The bench waits for every frame, so absolute times are inflated; the ratios are what count. Other effects:

- **Per-frame upload:** the Garden fell from 5.8 MB to 2.0 MB, the 30-foe scene from 5.5 to about 2 MB,
  and Scene I from 8.2 MB to 0.1 MB.
- **Start-up:** DX12 GPU setup went from about 5 s (baseline) to 3.2 s.
- **First gameplay frame:** 1.8 s down to 0.3 s.
- **Breaking a wall:** a full static re-upload (worst frame 99 ms with face extraction) became an
  incremental patch (worst frame 30 ms, the same as ordinary frames).

Pictures were checked against the untouched build on 36 scripted scenes. The differences are a few
hundred pixels per scene, where coplanar faces already z-fought, plus the blending order inside
see-through souls.

The latest full capture is in `perf/`; compare new captures against it only at equal settings.

## Findings

- **The GPU cost is geometry, not shading.** With every shading effect switched off, the main pass kept
  over half its cost. Instanced cubes at 4× MSAA dominate. This is why stage C came first.
- **The 30-foe battle has simulation spikes.** Every 1.2 s the update step takes 16 to 60 ms, growing
  over the run (foes being summoned or pathfinding, in game code). That is RP06 territory and outside
  the renderer.
- **The ground mist costs real GPU time.** The 13,500 transparent sheets in the Garden cost about 2 ms
  (see `--fx-off mist`), as the plan warns about large transparent layers.
- **Dark palette colours crush to black.** On the Look Dev value ramp under neutral light, sRGB 0, 25 and
  51 all land on black, and the palette's charcoal (#192321) is one of them. The plan warns about this
  for dark base colours; the tone curve and exposure need art review (RP13 and TA01).
- **Start-up on DX12 is still 3 s**, mostly compiling shaders with FXC. Moving to DXC would need
  `dxcompiler.dll` shipped, or wgpu's `static-dxc` feature (which downloads the compiler at build
  time). Vulkan starts in 1.9 s.
- The 8-bit scene target does not band visibly (the composite dithers). An earlier impression of
  banding came from scaled-down screenshots.

## Open, in the plan's order

- **B, art contract:**
  - TA02: one approved light per realm, captured with grading on and off (`--lookdev realm:N`,
    `--fx-off grade`).
  - TA03: cue colours.
  - TA04 and RP01: the source asset conventions and a MagicaVoxel/Blender round trip.
  - TA05: the benchmark asset set.
- **C, geometry:**
  - RP12 remainder: LOD for architectural chunks if the vista, top-down or GTX 1080 measurements call
    for it, and for voxel props (`Mesh`) if they become a measured cost. A feature-keeping coarsening
    (eyes, gems) could replace plain colour averaging if art review asks for it.
  - Merging coplanar faces of the same colour.
- **D, lighting:** RP13 and RP14 are done (RP14's look waits for approval).
  - Then RP15 (AO on indirect light only, an audit of the
    MSAA normal/depth resolve), RP17 (clustered lights), RP18 (probes) and RP19 (atmosphere).
- **E, world systems:**
  - RP02: camera validation.
  - RP03: area packaging and the portal loading screen. The portal scene's worst frame is 24 ms; cold
    and warm need their own figures.
  - RP04 and RP05: predefined broken pieces with collision and navigation updated together. Today only
    the bombable wall tiles break.
  - RP06: the 30-foe budget split (animation, AI, collision, rendering).
- **F, advanced:** RP20 (GPU visibility and Hi-Z), RP21 (meshlets) and RP22 (mesh shaders), each only
  after the indexed path is measured on both target GPUs.
- **G, shipping:** the GTX 1080 and RTX 3060 presets, and packaged validation.
- **Calibration** (the plan's own list):
  - Record both target machines.
  - Agree a numerical portal-loading target.
  - Ratify the percentile gates.
  - Approve the realm accents.
- The web build still compiles and lints, but the plan drops the browser target. Rigs and static faces
  now read storage buffers in the vertex shader, which WebGL2 cannot do.
