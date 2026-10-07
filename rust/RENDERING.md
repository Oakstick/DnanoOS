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
- **RP08, performance captures** (`src/perf.rs`, `--perf [all|names] [--perf-mode throughput|gameplay]
  [--frames N] [--size WxH] [--fx-off …] [--out dir]`). There are fifteen scenes: close-up, garden,
  corridor, vista, 30 foes, the warband (the vaults with the expert bot fighting), souls, fast rotation,
  camera switching, destruction, siege, portal arrival, vault, the Pit's seventh level with the
  expert bot, and its sixth run on into the seventh (the descent). Each has a fixed seed and script, played by the same driver as `--capture`
  (`src/driver.rs`: the script, the bot's pad every step), a stated simulated duration the frame count
  comes from, and 120 warm-up frames with the simulation held. **Throughput** mode (the default) takes
  one simulation step per drawn frame, so the scene keeps moving however fast it is drawn; **gameplay**
  mode measures each scene at 60 FPS (two steps per frame) and 30 FPS (four), as `app.rs`'s
  accumulator runs them, reporting update and render CPU per drawn frame. At most two frames are in
  flight. The report gives the median, 95th and 99th percentile and worst of wall, CPU (update and
  render apart) and GPU time, plus the first-frame cost. Every scene states what must have happened
  (a fight's swings, damage, foes engaged, blows met and packs slain; the portal's four changes of
  level), counted by the driver; a scene that falls short is marked FAILED and the run exits with 3.
  It writes a Markdown summary (machine, driver, backend, build `git hash[-modified]`, settings) and a
  CSV of every frame per scene to `perf/`. Until 2026-10-05 the harness never let the setup's bot act
  (the warband was measured with an idle hero) and its 600 frames ended before the portal's last
  return.
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
  frame. It is drawn again only when the light, its box or the static scenery changes, and for a
  change of the scenery only over it (no stalls mid-fight, below).
- **Depth prepass** for opaque geometry: the expensive shading then runs once per pixel. Fades dither
  the same way in both passes; water, clouds and glows are left out of the prepass.
- **RP10 and RP11, resident rigs** (`voxels::RigMesh`). Each rig is meshed once, per part, keeping only
  the voxel faces nothing in the same part covers; faces at joints stay, each a 32-byte record the
  vertex shader reads. Each frame uploads only a
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
  one pose drives them all. A foe's levels are made as its level loads, not mid-fight (below). The
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

### RP15: contact shading on indirect light, and an honest normal buffer

- **AO darkens only indirect light.** The scene pass writes a third target holding the indirect part
  of each opaque pixel: the ambient sky and ground light, environment reflections and the soft
  materials' sky rim, after fog. The composite takes away only what contact shading occludes
  (`colour − indirect × (1 − AO)`). The sun or moon, the lamps, emission, and see-through layers (mist,
  water) keep all their light, so AO no longer dims a lamp-lit corner or a glowing ember.
- **The normal/depth buffer keeps real surfaces.** It used to be resolved by averaging its four samples,
  giving edge pixels a normal and depth halfway between foreground and background. Now it is stored
  multisampled, and a resolve pass (`gbuf.wgsl`) keeps each pixel's nearest sample, the surface in
  front, whole. Contact shading and the background blur read only real surfaces.
- **Look:** very close to RP14. Sunlit and lamplit edges in creases keep their light (between the hero's
  arm and body, at the feet of walls); unlit creases are shaded as before.
- **Cost:** within the noise (resolve 0.05–0.24 ms). The extra multisampled target is paid back by no
  longer hardware-resolving the normal buffer.
- **Still open (TA):** dark colours crushed by the tone curve's toe (the Look Dev ramp) is a tone-curve
  and exposure decision, not AO.

### RP17: clustered forward lighting

- **Clusters** (`render/clusters.rs`): the view is cut into 16×9 screen tiles × 24 depth slices
  (logarithmic from the near plane to 80 m), 3,456 clusters in all. Each frame the CPU bins every light
  into the clusters its sphere of reach touches. It first takes the tiles and slices the sphere's
  bounds cover (through the real view projection, so the map's orthographic camera works too), then
  keeps a cluster only if the sphere really meets its box.
- **Shading:** each pixel loops over its own cluster's lights only, at most 32 (`cluster_at` in
  `shaders.wgsl`, used by surfaces, mist, water and jets). Lights live in a storage buffer of up to
  1,024, so the old global limit of 32 is gone.
- **Predictable limits:** lights are ranked the hero's lantern first, then nearest the eye. A cluster
  that overflows keeps its most important lights and drops the same ones each frame.
- **Diagnostics:** F3 and the `--perf` CSV report lights, pairs kept, the fullest cluster and pairs over
  the cap; `--light-heat` tints every pixel by its cluster's light count.
- **Measured** (30-foe battle, interleaved against the global list):
  - All 40 lights now shade, where the old list dropped 8. GPU time is about the same (21.5 vs 21.0 ms
    and 21.9 vs 24.5 ms at low clocks); Garden, Vaults and Lust are within noise.
  - 13.8k light-in-cluster pairs, the fullest cluster 25, none over the cap.
  - The CPU binning costs 0.1–0.25 ms a frame.
- **Shadow-casting local lights:** there are none yet (only the sun or moon casts shadows). When there
  are, they get their own budget, separate from these unshadowed lights.

### RP18: the level's light, from load-time probes

- **Probes** (`render/probes.rs`): when a level is first drawn, its static cubes are laid into a grid
  of cells a quarter unit across, each knowing its colour and glow. The cells that see the sun or moon
  are found by marching toward it. A probe at every tile's centre, on six heights, then casts eight
  cosine-spread rays along each of the six axes, out to 8 units. Each side records the light coming
  back off what its rays hit (colour × ambient light, plus sun or moon where that spot sees it, plus
  glow) and the share of rays that escape to the sky.
- **Shading:** inside the level, a surface's ambient light becomes ambient + sky × seen share + light
  from the surroundings, each from the probes blended toward the normal. Its environment reflection
  becomes sky cube × seen share + the room's own light along the reflection. That replaces RP14's
  stand-in for metals. Story sets, the title and Look Dev lie outside any level and keep the old path.
- **Walls:** rays stop at solid cells, so light does not pass through walls; a probe buried in a wall
  takes its open neighbours' values. Samples are taken 0.3 units off the surface, so floors do not see
  themselves.
- **Broken walls:** the region round each hidden or added cube is laid again, its sun worked out again,
  and the probes within ray reach are traced again (a test checks this matches a full rebuild). It costs
  8 ms per break in the Garden; the destruction scene's worst frame is 13.9 ms.
- **Local reflections, the compromise:** reflections come from the probes' six directions (a very
  blurred local view), not from rendered local cubemaps. That suits the worn metals, and the sharp
  sky stays in the environment cube. Rendered room cubemaps would need the scene drawn again into
  each one; that stays a later option if polished metal ever needs it.
- **Build time:** 12 ms (meadow) to 125 ms (Vaults, 138×102 tiles), on a level's first frame. **GPU
  cost:** within the noise.
- **Not included:** moving lights (the hero's lantern, spells) do not bounce. They light directly
  through the clusters (RP17).

### RP19: atmosphere within budget

- **Ground haze** (`fog_at` in `shaders.wgsl`, `frame::Haze`): fog thickest at a floor height that
  thins exponentially above it. Its amount along each view ray is exact: the mean density of an
  exponential layer along a straight ray has a closed form. It combines with the distance fog.
  Each kind of level sets its own (`WorldView::level_haze`):
  | Level | Density at the floor | Falloff |
  |---|---|---|
  | dungeons | 0.035 | 0.8 |
  | meadow | 0.018 | 1.5 |
  | hub, over the lava | 0.03 | 1.2 |
  | Vaults | 0.008 | 2.5 |
  | Grey Mire | 0.06 | 1.2 |
  | other realms | 0.025 | 1.0 |

  Story sets, the title and Look Dev have none. Surfaces, water, jets and glows take it; the cloud
  sea, smoke wall and ground mist keep distance fog only (they already are the atmosphere). The
  densities are kept low enough that a foe charging across a room stays plain; the 30-foe battle was
  checked.
- **Coarse volumetric light** (`lamp_glow` in `post.wgsl`): in the composite, the light of the first 8
  lamps in priority order (the hero's lantern first) scattered toward the eye by the haze and a little
  by the fog. It integrates in closed form along the view ray (an arctangent), cut to each lamp's reach,
  and the ray's distance to a lamp is floored at the lamp's size so a grazing ray does not blaze. It is
  isotropic and ignores shadowing. There is no history, so camera cuts, portal arrivals and falling
  walls leave no trails. It is off under the orthographic map camera.
- **Budget:**
  - The haze and the volumetric light each have an `--fx-off` switch (`haze`, `volumetric`), so a
    low preset can drop the volumetric light and keep the haze's depth cue.
  - The lamp count (`Preset::volumetric_lamps`) is the cost knob.
  - Measured in the 30-foe battle: the composite pass took 0.81 ms without and 0.82 ms with it.
- **The sun's light shafts** through the sky (`fs_rays`) are unchanged.

### RP16: shadow presets and per-cascade budgets

- **Quality presets** (`render/preset.rs`, `--preset high|low` for the game, `--capture` and `--perf`):

  | | high (RTX 3060, 60 FPS) | low (GTX 1080, 30 FPS) |
  |---|---|---|
  | Near cascade map | 4096 | 2048 (still a 1 cm texel round the hero) |
  | Far cascade map | 4096 | 2048 |
  | Shadow filter taps (blocker search + filter) | 8 + 16 | 4 + 8 |
  | Lamps lighting the haze | 8 | 4 |
  | Rig level-of-detail bias | 1.0 | 1.0 |
  | Budget, near / far cascade (p95 on its target GPU) | 1.0 / 1.0 ms | 2.0 / 1.5 ms |

  The ground haze is on in both, so the reduced preset keeps the fog's depth cue and loses only part
  of the lamps' glow in it. A level-of-detail bias of 1.25 was tried on low and rejected: it drops
  Ragnar to his half level at play distance, and the policy puts hero readability before
  distant detail. High renders exactly as before (36 scenes, largest difference 1 level in 255). Low
  differs only at shadow edges, with a little more grain in soft self-shadow close up.
- **Each cascade has its own map** (`shadow_near` and `shadow_far`, two textures in place of one
  array), so they can differ in size. The filter's tap counts come from the preset. The blocker search
  keeps the same reach in the world at any map size.
- **Per-cascade budgets:** `--perf` writes a *Shadow cascades* table:
  - Each cascade's p50 and p95, which include the far cascade's static cache when it redraws.
  - Their share of the preset's budget.
  - The static casters each one drew.
  - The CSV also has `shadow_near_ms` and `shadow_far_ms` per frame.
- **Faces turned from the light are skipped** in both shadow passes, for static faces and rigs:
  - For a closed voxel, the faces turned to the light cover the same ground and are always nearer to
    it, so the map comes out the same. The 36 scenes differ by a few hundred scattered pixels at
    voxel edges.
  - The undersides of every floor were a full plane drawn under each level.
  - Interleaved A/B in the same clock state: the near cascade went from 0.57–0.66 ms to
    0.32–0.39 ms (garden, 30-foe battle, close-up). Whole-frame GPU time was within noise on this
    GPU, which is not limited by the shadow pass. It should matter more on the GTX 1080, where
    geometry is the cost, but that is unmeasured.
- **Already in place from stage C and RP12:**
  - Caster culling per cascade.
  - The cached static far cascade, drawn again on a light, box or static change, which covers
    destruction and area changes.
  - The near cascade's texel snapping, which keeps projections stable.
  - The far cascade taking rigs a level coarser.
- **Measured** (perf/2026-10-03-195145 high, -195228 low; dev machine, same clock state):

  | Measure | high | low |
  |---|---|---|
  | Near cascade, p95 | 0.15–0.60 ms | 0.05–0.49 ms |
  | Far cascade, p95 (except Scene I's souls, which has no near cascade) | at most 0.05 ms | at most 0.04 ms |
  | Souls, far cascade, p95 | 0.70 ms | 0.22 ms |
  | GPU per frame, p50, all 11 scenes | — | 0.15 to 0.9 ms below high |
  | GPU per frame, p50, close-up | 6.49 ms | 6.20 ms |
  | GPU per frame, p50, battle | 10.54 ms | 9.85 ms |

## Stage E: world systems

### RP02: both gameplay cameras, validated

**`--camera-check [all|dungeon0,hub,...] [--stride 2] [--walk 20]`** (`src/camcheck.rs`) checks both
cameras in all 21 kinds of level (three dungeons, their arenas, the hub, seven realms, seven
courts, the gym). The report goes to `perf/camera-<time>.md`, listing poses to reproduce with
`--capture --script "at:x:z,aim:yaw:pitch"`.

- **Survey:** Ragnar is set down on every second tile he can stand on, with the world paused.
  - The chase camera is held at 8 headings × 3 pitches (rest, looking down hard, looking up).
  - The top camera is taken as it is.
- **Walk:** 20 s of wandering with the foes about. The mouse swings the view now and then, and the
  camera switches between the two views every 3 to 5 s.
- **What it checks**, against the cubes the level actually builds:

  | Check | What fails |
  |---|---|
  | Clipping | the near plane's corners inside a cube |
  | Hidden hero | all three marks on him (shin, waist, chest) behind solid cubes, after fades settle; in the walk, only longer than 0.3 s |
  | Off screen | his waist outside the middle 90% of the frame |
  | Holes | the void showing through ground he walks on |
  | Depth of field | more than 1 px of blur on a foe within 12 m of him |
  | Level of detail | his level under each camera at 1080p (reported) |

**Found and fixed:**
- **Tunnels:** the camera eased across the whole map when Ragnar wrapped through a tunnel. It now
  cuts whenever he moves more than 4 m in one frame (also teleports and portals).
- **Real scenery heights:** the chase camera assumed walls were at most 0.7 m tall and only checked
  its own spot, so it sat inside tall hedges, outer walls and gates. It now keeps clear of the
  scenery's real spans of height on each tile (`camera::Spans`), worked out again when a wall
  breaks:
  - It fits under a gallery or an arch.
  - Glass counts for its near plane, but he shows through it.
- **Sight line:** it also checks its line of sight to his waist and chest, in 10 cm steps with a
  4 cm margin.
- **Fallbacks, in order:**
  1. Back off only as far as there is room.
  2. Pressed close, tip up less (down to −1.0 rad).
  3. Where the heading leaves no room or no sight of him (a doorway, looking into the wall), swing
     to the nearest heading that works, up to a quarter turn.
  4. Duck under a low beam or rise over something low, and look at him.

  The player's own heading and pitch are kept for when there is room again.
- **Framing guard:** whatever the pose, and while easing, the chase camera turns its gaze just
  enough that he stays within 18° of the middle of the view.
- **Easing:** where easing would carry either camera's eye through scenery, it cuts to its clear
  goal. The top camera rises over towers in its way.
- **Switching views:** for 0.45 s after a switch, the gaze blends onto him. The top view's gaze also
  follows him faster than its eye, so neither direction of switch loses him off the frame.
- **Trees:** a tree fades when the camera is inside its crown as well as when it stands between
  camera and hero. Its fade test now uses its real crown, fitted to its cubes, not the builder's
  guess.
- **Depth of field:** blur used to start 5 m past the focus, so a foe charging from behind was
  blurred by up to 1.5 px. In play it now starts 12 m past and is full at 27 m
  (`gpu::DOF_CLEAR`, `DOF_FULL`). The story, title, character screen, forge and tour keep their
  composed 5 to 20 m. **Look change for approval:** distant scenery in play is a little sharper.
- **Level of detail:** under the top camera at 1080p, Ragnar covers under 2 px per voxel and would
  drop to half detail. He is now always drawn whole, because he is the one the player reads; foes
  keep the projected-size LOD.
- **Crash fix:** reinforcements in dungeon III, which is reachable from the map, unwrapped a foe
  stage-3 never defines, and the game crashed when they came. They now come from the stage's own
  foes. A regression test covers it.

**Results** (`perf/camera-*.md`, the last run of all 21 levels):
- **Clean:** the dungeons, arenas, courts and gym are clean or within 2 of about 17,000 poses.
  Envy's, Wrath's, Greed's, Gluttony's and Lust's realms are within 17 hidden or clipping poses of
  9,000 to 39,000.
- **Left over:**
  - **The Hall of Mirrors (Pride), chase camera only** (it starts in the top view):
    - 72 clipping and 133 hidden poses out of 17,088 in the survey, at its door frames and glass.
    - About 1,000 survey poses, and the walks' views round the throne, see down past its rings
      into the well. That looks to be the design (the rings fall into a bottomless well), but it
      is not confirmed.
  - **Top view:** a dozen tiles across the hub and the realms where an overhang hides him. There
    is no general overhead fade yet; trees fade, roofs and arches do not.
  - **Short clips:** up to 4 frames of clipping right after a switch in two realms.
  - **For TA06:** the backdrop past the level's edge (the cavern, the cloud sea, the lava) shows in
    many poses by design. It is counted, not failed, for the vista review.

### RP03: portals and the loading screen

- **Hold until ready:** a new level now waits behind a loading screen (`game::Loading`). The world
  stops, black, with the level's name and a pulse. It waits until the renderer has the level up
  (`Gpu::level_ready`: scenery uploaded, probes worked out), then fades in on a finished frame.
  - The camera is cut to the level's start during the hold, so the prepared frame is the one shown.
  - The clock restarts on reveal, so the fade-in does not jump the time the load took.
  - The name and the pulse come up only when the wait passes 0.6 s (2026-10-05). The Garden's cold
    start takes about 0.2 s (`--perf start`: 30-40 ms building it, one 140-165 ms frame sending
    its scenery up, then its probes), and a portal under 0.15 s; with the words at 0.12 s they
    flashed up half-seen and were gone. A quicker wait is a plain cut through black.
  - It gives up waiting after 10 s.
  - The game and `--perf` wait; tests and `--capture` (with no renderer between steps) do not.
  - `--capture --await-renderer` shows the screen.
- **Light probes cached and built in the background:**
  - Probes are kept for the last 8 levels, keyed by their scenery and light, so a return through a
    portal finds them ready.
  - A first visit builds them on a worker thread, so the loading screen keeps animating.
- **Timing:** `--perf` writes a *Changes of level* table, cold (the level's first load in this run
  of the game) and warm (a return), measured on the dev machine:

  | Transition | Load | To first frame |
  |---|---|---|
  | Hall of Mirrors, cold | 13 ms | 144 ms |
  | Hall of Mirrors, warm | 3 ms | 29 ms |
  | Portals of Hell, warm | 4 to 10 ms | 32 to 41 ms |

  Warm returns were 82 to 114 ms before the cache. The portal scene's worst frame fell from 156 ms
  to 57 ms, and is behind the loading screen.
- **Not built:**
  - Packaging by area: the assets are procedural and in the code, the character rigs stay resident
    once meshed, and there is nothing to stream.
  - Preparing the next area while he walks up to a portal: the cold cost left is about 140 ms
    behind the loading screen.
- **Decision for you:** the plan asks for a numerical target for "very short" once both machines
  are measured. The figures here are from the dev laptop only.

### RP04 and RP05: bounded destruction and a clean reset

- **What breaks:** bomb blasts break wall tiles: every wall within the blast that is not protected
  or a landmark.
  - **Same step:** the grid opens (collision, and the foes' way-finding, which reads the grid every
    step) in the same step the wall's cubes are hidden. A test checks it every step of a blast, and
    that a foe routes through the gap.
- **Rubble:** a blown wall leaves a low heap of its own stones on the floor, the same for the same
  tile every time. Nothing of it stands higher than a shin (tested), so it neither blocks the way
  nor hides a telegraph. It is a stand-in until the broken pieces are authored (TA07).
- **Debris budget:** at most 160 chunks fly at once; the oldest are retired first and the store is
  reserved up front. Sparks were already capped at 700.
- **Shadows:**
  - Any hidden or moved static cube now redraws the cached far shadow cascade. Before, only a cube
    that opened a neighbour's face did, so a blown wall's shadow could stay behind.
  - The probes refresh round the break (RP18).
- **Reset (RP05):** a level is rebuilt from its original layout on every entry, walls whole.
  - **Cleared with the visit:** the level's debris, sparks, blasts, the angels' bolts, beams and
    waves, falling arrows, flashes and afterimages. Before, the last seven of those could carry
    across a portal.
  - **Kept:** his progress (silver, runes, kit). Loot, quest, boss and foe respawn rules are
    unchanged.
  - A test blows a wall, leaves and comes back, and checks all of this.
- **During a fight:** a wall breaking in the middle of a 30-foe fight is the `siege` perf scene
  (RP06).
- **Decision for TA07:** the visual cue that says a wall can be broken. Today every interior wall
  can be, with no cue.

### RP06: the 30-foe budget

- **The simulation step is timed by section** (`profile::SimTimes`, filled by `Game::update`):

  | Section | What it covers |
  |---|---|
  | hero | input, movement, his blows |
  | shots | projectiles, angels, bombs |
  | boss | the boss |
  | foes | foe AI, movement and pathfinding |
  | fights | frame-data fights |
  | spawns | new foes summoned |
  | contact | collision between foes and the hero |
  | effects | the world's effects |
  | view | camera and animation state |

  With CPU emit (posing rigs and gathering the frame), record (culling and draw recording) and GPU
  time, this is the split the plan asks for: animation, visibility, collision, AI and rendering.
  `--perf` writes a *Simulation step* table (median, p95, worst) and per-frame `sim_*_ms` columns.
- **The battle's spikes were the high score being saved.**
  - Once the score passed the high score, every kill called `platform::save`, which reads the whole
    save file, rewrites it and renames it, on the game thread.
  - Each kill cost 2 to 13 ms, and a swing that took several foes cost up to 50 ms. Foe AI peaked at
    0.5 ms in the same fight.
  - The high score is now held as unsaved and written on the next screen change (death, level end,
    pause, title) or on quit, with the run's snapshot.
  - The rest of the run's save goes the same way (`game/persist.rs`): the checkpoint, the purse and
    the Ledger are one snapshot, marked when they change and written in one write at a calm moment
    (no freeze-frame, no foe mid-blow, a second at most), at once at checkpoints, screen changes and
    quit. A rival's fall used to write the file twice on the kill's frame; now once, later. The F3
    readout shows the last write's cost; a write measured 3 to 4 ms median here, up to ~20 ms.
  - This also happened in the shipped game whenever a player beat their high score.
  - battle30's worst frame went from 36.1 ms to 12.9 ms, and its worst hero step from 33 ms to
    0.1 ms.
- **A wall breaking during the fight** (the plan's test) is a new perf scene, `siege`: 30 stage-2 foes
  in the Garden, a bomb every second, the sword swinging.
  - Frame time: p99 11.9 ms, worst 12.6 ms.
  - Foe pathfinding: p95 0.7 ms, worst 5.3 ms (the maze is harder to path than the meadow).
  - Collision: worst 1.5 ms.
- **Measured** (perf/2026-10-03-201257, high preset, dev machine), the 30-foe battle:
  - Wall time: p50 10.3 ms, p99 12.4 ms.
  - CPU: p50 2.1 ms.
  - GPU: p50 10.1 ms.
  - The simulation is under 0.1 ms at p95.

  On this machine the GPU is the cost. RP06's gameplay rule holds: none of these settings touch the
  simulation, and the presets and level of detail are view-only.

### No stalls mid-fight (2026-10-05)

A fight must never stutter. The gameplay benchmark of 4 October showed fight frames of 2 to 4 ms
with spikes of 11 to 26 ms; the fight scenes found more once measured frame by frame (the CSV's
`rig_misses` and `saves` columns). What each one was, and what was done:

| Stall | Cost in its frame | Why | Now |
|---|---|---|---|
| A kind of foe's first appearance (a pack waking, a Pit door opening) | 5 to 26 ms | Its levels of detail meshed in the update that brought it (`add_enemy`) and its buffers made the first frame it was drawn, up to 35 MB at once (two giants' full and half-resolution meshes at 168 bytes a face); a kind unseen for 10 s let go and sent up again | Made ready with the level (below); kept while it lives |
| The hero raising his shield, blinking after a blow, his shield breaking | 6 to 14 ms | His shield-up, see-through and shieldless rigs meshed and sent up the first time | Made ready with the level; the shieldless one swapped in |
| A coin taken in the vaults (every 5 s or so as he walks) | 11 to 13 ms | Not the far cascade: the probes worked out again round the coin on the spot, laying every static cube of the level, and every side of every probe sent up again | Refreshed on a worker thread from the cubes near it, the light following a frame or two later; only the probes that changed go up (0.2 ms) |
| The far cascade's cache drawn again (the same coin, a wall) | 0.7 ms GPU, 1.1 M casters | The whole level's casters drawn again for a coin | Only its texels over what changed: cleared under a scissor, the casters of that column of the light drawn (60 k for a broken wall) |
| The chase camera turning | 1 to 13 ms | The environment map (36 passes) was keyed by the camera's heading too, though it is seen by world direction | Keyed by its light alone |
| The Pit's descent | 6 to 9 ms, several times | The light changes by steps on the way down, each one the environment drawn whole | Within a level, followed a mip a frame (6 passes); a new level's drawn whole at once |
| A wall broken | 7 + 3 to 4 ms | The chase camera's spans worked out for the whole level; the rubble written to the GPU cube by cube; the first added cube growing tables of a million entries | Its own tiles worked out again (tested equal to a whole rebuild); one write; room kept |
| The run's snapshot written at a calm moment | 3 to 10 ms | File work on the game thread | Written behind, in order, on a thread of its own (`platform::write_behind`; reads and quitting wait for it) |
| The old man's first word | 12 ms | His face's sprite made on first use | The HUD's first-use pictures made with the level |
| 30 foes hunting through the Garden's maze | up to 6 ms (30 FPS) | Their way-finding searched on a hash map | On a dense grid: the same ways (tested), foes p95 4.6 to 0.9 ms |

- **Prepared with the level** (`Game::level_kinds`, `WorldView::prepare_rigs`, `Gpu::sync_resident`):
  every kind the level can deal (its foes; a dungeon's reinforcements; a realm's or the vaults'
  roster, its general's, its rivals', its own kind; the Pit's waves on every level from this one
  down; and what each brings: illusions, the raised dead, minions, a prince's adds, Krampus's
  Perchten), the hero's rigs, the archangels' shrines and the Hall of Mirrors' statues, at every level
  of detail. Meshed on every core natively (in order on the web, which has no threads); their
  buffers made before the loading screen lifts (`level_ready` waits) and pinned until the next
  level, the last level's let go. The lazy path stays, and counts `rig_misses`. `VH_NO_PRELOAD`
  turns it off, for comparison; `VH_RIG_MISSES` names each miss.
- **Rig meshes a fifth the size.** A face is a 32-byte record the vertex shader reads from a storage
  buffer bound per mesh (six vertices a face, their corners made as for static faces), where four
  36-byte corners and six 4-byte indices took 168. Every foe at every level: 455 MB to 86 MB (tested
  under 100); Surtr's three levels 20 MB to 4. Building them is three times faster. Renders the same
  (the capture suite matches, under FXC and DXC). What could shrink further changes the look:
  rounded colours (the shader seeds each voxel's detail from its exact colour) or merged faces.
  See-through (whole) meshes are made only for the hero's rigs.
- **What it costs at load** (the *Frame budget* table, the scene's last level, cold, dev laptop):
  meshing 1 to 25 ms, buffers 5 to 12 ms, 18 to 42 MB kept; the Pit entered from its first level
  (35 kinds) about 50 ms with the rest of its load. The levels' load and first frame fell where measured (the Hall of Mirrors
  cold: load 34 to 16 ms, first frame 238 to 123 ms; warm: 58 to 34 ms).
- **The Pit's fight simulation** at 30 FPS: its worst frame 0.71 to 0.37 ms (p95 0.38 to 0.14),
  under the 0.5 ms budget, with the simulation unchanged (`--pit 2 8 1` gives the same 1007 s, 133
  kills, 232 taken; the warband and Pit scenes the same swings, hits and damage). Played alone the
  fights cost under 0.08 ms; their peaks in the harness were the new kinds meshed in the fight's own
  lap as they came out of the doors.
- **Asserted** (`--perf`): the fight scenes (warband, pit, battle30, siege and the new **descent**,
  the Pit's sixth level run on to the bell so the platform goes down and the seventh level's giants
  come out of the doors) fail if any measured frame takes over 8 ms of CPU (half a 60 FPS frame:
  three times their usual frame, the stalls above 9 to 26 ms) or draws a rig not made ready; the
  descent also if no kind of foe new to the fight came. Tests: each level's preparation covers what
  the bot meets in the vaults, a realm, four courts (wounded through their phases), the Pit's levels
  and the lair; a kept mesh is never let go; walking never moves the far cascade's box; a coin's or a
  wall's part of the cache is small and its view sees its casters.
- **Measured** (perf/2026-10-05-102926 before, -103108 after; release, gameplay mode, interleaved in
  one sitting, dev laptop):

  | Scene | Worst CPU frame, before | after | Frames over 8 ms, before | after | CPU p50, before | after |
  |---|---|---|---|---|---|---|
  | warband @60 / @30 | 13.8 / 16.1 | 4.6 / 6.2 | 4 / 4 | 0 / 0 | 1.6 / 1.9 | 1.1 / 1.7 |
  | pit @60 / @30 | 18.1 / 13.9 | 4.7 / 5.0 | 2 / 2 | 0 / 0 | 1.6 / 2.6 | 1.3 / 1.1 |
  | battle30 @60 / @30 | 25.7 / 9.6 | 5.0 / 4.2 | 2 / 2 | 0 / 0 | 2.3 / 2.5 | 2.2 / 2.3 |
  | siege @60 / @30 | 35.6 / 41.0 | 4.4 / 5.0 | 9 / 10 | 0 / 0 | 3.7 / 4.6 | 1.6 / 1.8 |
  | destruction @60 / @30 | 27.0 / 24.8 | 4.5 / 4.0 | 2 / 2 | 0 / 0 | 2.6 / 2.2 | 1.6 / 1.4 |
  | descent @60 / @30 | (new) | 4.1 / 4.5 | | 0 / 0 | | 1.1 / 1.3 |

  GPU time is within noise or lower (the environment no longer drawn again as the camera turns).
  This laptop shared its CPU with other work during some runs; the budget wants a quiet machine.
- **Left:** the Pit entered cold meshes its seven levels' kinds in the step that loads it (about
  50 ms before the loading screen shows; building them on a worker thread while it animates would
  hide it). Scene I's cast is not prepared (it is no fight). A shield broken mid-fight is covered;
  other changes of kit mid-level (none in play) would mesh as they came.

## Stage F: advanced rendering, evaluated

The plan gates stage F on measuring the indexed path on both target GPUs. That was waived by
decision on 2026-10-04: no hardware tests. Everything below is measured on the development laptop
only, by interleaved A/B in one sitting.

### RP20: GPU visibility

- **Built:** `GpuCull` in `gpu.rs`, with the shader in `cull.wgsl`.
  - The static scenery's cluster bounds are kept on the GPU.
  - Each frame a compute pass tests them against the camera and both shadow cascades.
  - It writes the visible ones as indirect draws: packed and counted where the adapter can (DX12,
    Vulkan 1.2), or one slot per cluster drawn empty where unseen.
  - Each pass then draws all its scenery with one multi-draw. There is no CPU readback.
  - Faces added since the level loaded (patches, rubble) are one more cluster, whose box is sent
    each frame.
  - It needs `MULTI_DRAW_INDIRECT` and `INDIRECT_FIRST_INSTANCE`, both requested where the adapter
    has them. Elsewhere (the web build), it falls back to the CPU path.
- **Result:**
  - Draw calls halved (Garden 105 to 51, vista 142 to 71).
  - The pictures match the CPU path.
  - **GPU and CPU time are no better:** within noise, and 0.1 to 0.3 ms slower on some runs.
    Culling 300 chunks on the CPU and recording their draws already cost almost nothing.
  - **So it is off by default** (`Preset::gpu_cull`). `--gpu-cull` turns it on in the game,
    `--capture` and `--perf`. It is worth trying on the GTX 1080, which is geometry-bound, where
    it might pay.
- **Hi-Z occlusion: measured, not built.**
  - `--perf --occlusion` counts, with a query per chunk in the depth prepass (nearest first), the
    static faces in view that lie in chunks wholly hidden behind nearer scenery:
    - Garden, corridor, destruction and siege: 7 to 8%.
    - Close-up, the 30-foe battle and switching cameras: 11 to 18%.
    - The hub vista and the portal scene: 31 to 42%, of only about 25k faces.
  - Those faces already fail the depth test early, so culling them saves only their vertex and
    depth work. RP21's measurement below shows that saving even 30 to 50% of the faces' vertex
    work changes nothing on this GPU.
  - To avoid popping, Hi-Z would need either splitting the multisampled main pass to build the
    pyramid mid-frame, or last frame's depth, which can hide scenery for a frame after a character
    walks away.
  - It does not meet the plan's bar ("when hidden geometry remains a measured cost").

### RP21: meshlet data

Meshlets are built from the extracted surfaces (the open faces of RP11), not from whole cubes.
Three grains were compared (`--perf` writes a *Culling granularity* table):

| Grain | Count in the dungeon | GPU memory | Faces the camera draws, against whole chunks |
|---|---|---|---|
| Whole 8 m chunks (today) | ~300 | — | 100% |
| Chunk × facing group | 1,536 | 74 kB | 71 to 95% |
| Clusters of at most 64 faces, with their facing | 18,215 | 0.9 MB | 45 to 71% |

- **Facing as an exact normal cone:** every face in a group or cluster looks the same way (one of
  six), so the cone test is exact.
  - From the camera, a group is dropped when the eye lies behind its nearest face plane.
  - From the light, it is dropped when the group faces away.
  - Faces of turned cubes, and of cubes that can fade (trees, the Thorns' hedges, which must keep
    their back faces), are never culled by facing.
- **The 64-face clusters cull the most** (29 to 55% of faces dropped before vertex work), and the
  GPU path uses them.
- **But on this GPU it measured no faster** than drawing whole chunks with per-face back-face
  skipping in the vertex shader. The cost here is pixels and 4× MSAA, not geometry. The data and
  path stay, for the GTX 1080.

### RP22: mesh shaders, evaluated and not adopted

- **Engine today (wgpu 25):** a mesh-shader feature flag only. There is no mesh pipeline or mesh
  draw in its API, and WGSL cannot express mesh shaders.
- **Later wgpu:** [wgpu 28](https://github.com/gfx-rs/wgpu/releases/tag/v28.0.0) added them:
  - in full on Vulkan;
  - on DX12 and Metal only through passthrough shaders, the unsafe API the plan says to review
    before adopting;
  - WGSL support through WESL or naga_oil.
- **Adopting them would mean** three major wgpu versions of migration (25 to 28 or later),
  passthrough shaders on our DX12 path or a move to Vulkan, and a second, mesh-shader path. The
  GTX 1080 has no mesh shaders, so the indexed path must stay first-class anyway.
- **Not adopted:** RP21 shows the geometry stage is not what costs on this GPU, so mesh shaders
  have no measured problem to solve. Recheck if the target GPUs show geometry cost.

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

## Stage B: art direction, first levels

Two dungeons follow the shared palette (charcoal, umber, copper, amber, mist teal, pale stone). The
builder is `view/crypt.rs`. Both are off the road on the map.

- **Dungeon III, the Bone Vaults:**
  - Walls are dressed-stone masonry in chunky blocks, each at its own depth, askew and chipped.
  - Fallen blocks lie broken at the walls' feet, and about one wall in six has lost half its top
    course.
  - Floors are wet flagstones.
  - Props: pillars, banners, copper braziers, arches with hanging lanterns, hooded statues on the
    walls, moss and bones.
  - The gatehouse is a crypt gate: towers with watchers, a round arch with a skull keystone, iron
    gates under a grille, and lock lamps in a fan over the arch.
- **Dungeon II, the Cisterns:** the same stone, flooded.
  - Shallow standing water lies over the low floors, in blocks of rooms.
  - Copper-lipped spouts pour thin falls, with splashes and ripples.
  - The walls' lowest course is dark and wet, with more verdigris.
  - Columns and arches are more frequent; banners and bones are rare.
  - The light is cooler, with more of it from above.
  - The layout and gameplay are unchanged.
- **Dungeon I, the Garden of Krampus** (`view/estate.rs`): the garden keeps its own scenery
  (hedges, woods, fountain, channels, churchyard, temples). A grade pulls everything built into
  the board:
  - greens go to moss and olive;
  - chalk paving comes down to pale stone, with the dressed-stone material;
  - loud blossoms are quieter.
  - Its lanterns and fireflies burn amber, and the night is mist teal under the moon.
- **The Krampus fountain plaza** (`view/fountain.rs`):
  - The basin is dressed stone (the grade gives it the material) under a copper coping with a
    dark copper lip, verdigris in its seams; the bowl's rim is copper too.
  - The water is the board's mist teal instead of cyan, and the lamp under it is a soft mist
    teal at about half its old strength.
  - An amber lantern on an iron post, with a copper cap, stands at each outer corner of the
    plaza. Each throws a small, soft pool of light, gentler than the garden's lanterns, so the
    plaza does not glare.
- **The title screen and menus:**
  - The stage stands in a ruined sanctum (`view/sanctum.rs`, built once):
    - a great round arch behind Ragnar, open on the moonlit mist;
    - crumbling walls in charcoal and umber courses;
    - pillars hung with copper and umber banners with amber sigils;
    - pale stone hooded knights on either side;
    - lanterns on brackets, flagstones with mossy seams, and dark pines beyond the walls.
  - The heroes stand on a dais of dressed stone with a copper band, between copper braziers on
    legs.
  - The trees take the garden's grade; the soul halos are gentler; the fireflies are amber and
    kept away from the camera.
  - The menu panels are charcoal with pale-stone edges and a copper rule; the gold is amber.
- **The HUD, the pause menu and the shop** (`hud.rs`, `hud/shop.rs`):
  - The in-play panels are charcoal (`INK`) with a copper edge and a brighter copper rule along
    the top, instead of a faint pale-stone edge.
  - The menu buttons (the pause menu, and the title's) sit on charcoal with copper edges; the
    highlighted one is washed copper under its amber outline.
  - The old gatekeeper's panel was a blue-grey frame on black: it is charcoal with a copper edge
    and a fine pale-stone line inside, its title amber. The open tab is a dark copper instead of
    blue, the next tier to buy is washed mist teal instead of blue, and the buttons and the card
    have copper edges.
  - Text keeps its colours (pale stone, amber, the warning red), so it reads as before.
- **The skies:**
  - The night is teal-charcoal with a low mist-teal glow at the horizon, a warmer milky band,
    fewer stars and a teal moon halo.
  - The dusk (the Cyclops' meadow) runs amber at the horizon, through copper and mist-teal grey,
    to a charcoal zenith, instead of pink, mauve and navy.
- **The Cyclops' meadow:** takes the Garden's grade and an amber dusk light instead of magenta
  twilight.
- **The character screen's cavern:** charcoal and umber stone, amber lava, crystals of mist teal.
- **The Portals of Hell:**
  - charcoal basalt, slabs of umber and pale dressed stone;
  - lava that burns amber and ember instead of blood red;
  - a charcoal smoke lit amber from below.
  - The sin portals keep their colours, and the gym keeps its look.
- **The princes' stand-in courts** (`court:0..6` but Greed's, which is Mammon's own): the Cyclops'
  meadow under the sin's sky is graded as the realms are (`grade_scenery` with the sin's hue
  kept), so its grass and hedges come to moss and olive, its walls and pillars to pale dressed
  stone, and only the sin's own colour stays loud. The sin's tint on the sky and the light is
  unchanged.
- **The seven realms and Mammon's vaults:** graded into the board as they are built
  (`grade_scenery`). Colours near the realm's sin hue keep their saturation as its accent:
  Pride's violet, Envy's green, Greed's gold, Lust's pink. Everything else (chalk floors, loud
  stone) comes into the palette.
- **The Hall of Mirrors (Pride):**
  - The obsidian floor comes down from loud violet to charcoal violet; its rims, plinths and the
    masonry caps follow, so the gold spokes and the mirror-doors carry the eye.
  - The light-well is keyed mist teal round an amber heart (shaft, rings and the light it throws),
    under a mist-teal sky light and a warm stone key instead of white-violet. Bloom is 1.0.
- **The Serpent Grotto (Envy):**
  - The algae fringe along the deep water and the waterline band are a dim verdigris (`FRINGE`)
    instead of the sin's full green, which outlined every pool in neon.
  - The jade water is a deeper mist teal; the Drowned Floor's gold and jade come toward umber and
    amber, and its lanterns burn amber instead of lime.
  - The Waterline's light is a cold mist teal from above with more ambient, so the basalt reads
    out of the black. Bloom is 0.95.
- **The Burning Field (Wrath):**
  - The lava sea was a blown yellow-red sheet. More of it is now crusted over in charcoal, and its
    rivers burn ember and amber at about a third of the glow (as in the Portals of Hell).
  - The obsidian loses its violet for charcoal; the light from below is ember, not scarlet, and
    the ash sky's light is umber grey. The lava's light through the gaps is softer. Bloom is 1.0.
  - Amon's red stays in the banners, the blood-mud and the wounds in the slabs.
- **The Grey Mire (Sloth):** still flat and shadowless, but its fog and light are a mist-teal grey
  instead of a cold blue-grey, with a little umber off the peat, and the moss pillows are deeper
  and nearer olive. Belphegor's slate blue stays in the marsh-lights and the portals.
- **The Rotting Feast (Gluttony):**
  - The Hall of Courses' chalk-white marble comes down to warm pale stone (its tables' marble and
    the Kitchens' white tile too), so the red runner, the food and the candles carry the hall.
  - The Stomach's acid and glands go from lime to a sick olive-amber, and its pools glow less.
  - The light is a little lower and the bloom 0.95 to 1.25 (was 1.15 to 1.45), so the candles
    no longer blow out in halos.
- **The Garden of Thorns (Lust):**
  - The moon is the board's: mist-teal light over a charcoal-rose dark, instead of blue-white.
  - The perfume mist was pooling as loud magenta sheets; it is a dusty rose, and thinner.
  - The pink marble and flagstones come down from near-white to rose-cast pale stone. Bloom 1.0.
  - Asmodeus's rose stays in the roses, the lanterns, the silk and the portals.
- **Props:** each realm was checked for bare stretches in both views. Their own dressing (wrecks
  and spear-fields, sunk cottages and gears, tables and ovens, bowers and bramble, ribs and
  rafts) already breaks every silhouette, so none was added; the passes are palette and light.
- **Camera check:** both pass their walks with no clips or hidden poses. The chase survey hides the
  hero in 33 of 16,248 poses in the Cisterns and 24 of 15,720 in the Vaults.

### Scene I's sets (2026-10-07)

Both sets were rebuilt to Dungeon II's quality (`view/story_sets.rs`). The cast's animation, the
boys' sculpts and the shots' framing and timing are unchanged.

- **What the story bypassed:** its sets were plain-lit cubes (`SHADE_LIT`, no `voxel_surface`
  material, RP14) and it drew no ground haze (RP19: `emit_story` returned before `level_haze`). The
  probes (RP18) were built only for the level's static cubes at the world's origin, so the sets
  took the shader's fallback light. AO on indirect (RP15), the clustered lights (RP17) and the
  presets (RP16) already applied, but with flat indirect light AO had little to work on.
- **Shared builders:** `crypt.rs`'s pieces are now free functions on a list of cubes. These are
  `masonry` (one `Wall` face of any length and height, in courses, with arched or windowed
  openings and fallen blocks), `arch_cubes` (round or flattened), `pillar_cubes`,
  `brazier_cubes`, `lantern_cubes`, `bracket_cubes`, `statue_cubes`, `banner_cubes`,
  `bone_cubes`, `moss_cap`, `moss_seam`, `flagstones`, `spout_cubes`, `emit_pool` and
  `emit_fall`. The vaults call the same functions.
  - The arch's wedge stones are now turned square to the curve. They were turned a quarter turn
    wrong part of the way round (`rotation_z(a - PI/2)` for a ring laid from -x). This changes
    the Vaults' and Cisterns' arches slightly; nothing else in the dungeons changed (536 and 168
    pixels differ in the reference views).
- **The gateway:**
  - Walls of chunky dressed stone (11 courses, 4.8 high), each block at its own depth, askew and
    chipped. The lowest course is dark and wet with verdigris, and fallen blocks lie at the feet.
  - Wet flagstones in grout, with moss seams and five shallow puddles under the drips.
  - Pillars carrying two flattened ribs, and blind arcades along the side walls with niches, a
    banner and a hooded knight.
  - The crypt gate: towers with a watcher on each, a round arch on pale piers, a skull keystone
    with ember eyes, a fixed grille, a cornice with five lock lamps that kindle red as the door
    opens, and an iron door of plates, bars and riveted straps.
  - Copper braziers (deep bowls, their flames drawn live) and lanterns hung from the ribs and the
    front pillars.
  - The old man's corner: a stool, a crate with a mug and bread, a straw pallet with a blanket,
    a pail, and his lantern. Quitmer's corner: straw, an iron bowl with a bone, and the chain's
    ring on a plate.
  - The way they came is a stone arch in the front wall, sealed deep, with the swirl fitted inside it.
- **The graveyard at dusk:**
  - The ground is uneven turf in half-unit cells, level where the cast moves, with a trodden
    patch where the friends play. A path of sunk stones and gravel leads to a lane, and coarser
    fields run out to the fog.
  - The churchyard wall is dressed stone on both faces, with a pale coping and moss. It carries
    rusted railings with spear heads and collars.
  - The gate's piers are dressed stone in courses, with lichen, a moulded cap and a cut stone
    ball. The iron arch is rusted.
  - Graves: rounded headstones with worn inscriptions, crosses (some ringed), obelisks, chest
    tombs and sunk broken slabs, with moss and lichen. There is a railed family plot.
  - A mausoleum with columns, an arched iron door and a verdigris roof.
  - The chapel has walls in courses, lit lancet windows, a slate roof and a bell tower with a
    spire.
  - The bare trees have umber bark, ridges and lichen, one level more of branches, and four more
    beyond the wall.
  - Outside the wall, which is beat 3's view: old graves at the yard's edge, a wayside cross, a
    dry-stone field wall, a bench, and the lantern on its post.
  - The dusk is the meadow's: amber, copper and mist teal instead of pink and violet. The mist
    is teal grey, and a low haze (`dusk_haze`) thickens with the pursuit's fog.
- **Light:**
  - Each set carries `SetProbes` (its cubes, extent and settled light) in `Frame::probes`, and
    the other set in `probes_warm`. `Gpu::sync_set_probes` works their probes out once, on a
    worker thread (`ProbeVolume::build_at`, a volume with an origin), and sends the shown set's
    up when the textures hold another's. The level's probes go back up whole when play resumes.
  - Working the probes out takes 3.6 ms for the gateway (11×6×14 probes) and 6.8 ms for the
    graveyard (32×6×30).
  - `begin_story` holds Scene I behind the loading screen until both sets' probes are worked out
    and both sets are on the GPU (`level_ready` via `WorldView::story_set_id`). Nothing is
    built or sent mid-scene. Captures call `Gpu::settle` so a shot never depends on the timing.
  - The gateway takes the vaults' light (mist-teal sky light from above, exposure 1.35) and
    their haze (0.05). Its practicals are 2 braziers, 4 lanterns, the old man's lantern, the lock
    lamps and three fills: 13 lights in all.
- **Numbers (1080p, RTX 5070 Ti Laptop):** interleaved A/B over two rounds, 300 frames each
  (`--bench`).

  | Beat | Before | After |
  |------|-------:|------:|
  | 1 (graveyard) | 6.1 ms | 7.2 ms |
  | 3 | 4.7–5.0 ms | 6.1 ms |
  | 5 (gateway) | 4.3 ms | 5.4 ms |
  | 18 | 5.6–5.7 ms | 6.9 ms |
  | 21 | 4.4–4.5 ms | 5.7 ms |

  - The GPU's share of beat 5 is 3.07 → 3.94 ms: the main pass +0.55 ms (materials, more
    lights), the shadow pass +0.15 ms, and the haze and lamp glow in the composite +0.17 ms.
  - The graveyard is 16,056 cubes and the gateway 1,761, built once at start-up. A test keeps
    them under 30,000 and 12,000.
  - Each set's probes go up once, in 0.2 ms.
- **Still open:**
  - The chapel and mausoleum read from far off only.
  - Seen from the crane, the turf under the mist is a flat grey-beige.
  - The gateway's walls are regular in plan (no ruined course, no deep recesses).
  - The probes are worked out under each set's settled light, so the graveyard's dimming when
    Krampus comes changes the direct light only.

### The Garden, at the Cisterns' bar (2026-10-07)

Dungeon I, the first level every player plays, was rebuilt to Dungeon II's quality
(`view/garden_beds.rs`, `view/garden_landmarks.rs`). The layout and gameplay are unchanged: the
same tiles are floor and wall, the maze, keys, nuggets, breakable walls, shrines, gate and foes'
paths are as they were.

- **What the Garden bypassed or under-used:**
  - **RP14 materials:** almost none. Every floor was one even lawn (`SHADE_GRASS`), every wall a
    plain-lit mound behind a dry-stone fence with voxel bushes and little voxel trees, and the
    grade only turned pale greys into dressed stone after the fact. The hedges, trees, paths,
    iron and copper had no material.
  - **RP17 lights:** the clustered lights could shade a thousand, but the garden lit only its
    three lanterns nearest the hero (`LIGHT_POOL_TORCHES`), so most lanterns in view were
    unlit glass.
  - **RP19 haze:** the generic dungeon haze; the ground mist was laid over every tile, beds
    included, veiling them grey.
  - **RP18 probes, RP15 AO and RP16 presets** already applied, but with flat ground and no
    occluding forms they had little to work on.
  - **Cost:** 295,000 static cubes (the Cisterns: 57,000), most of them voxel bushes and trees.
- **Built:**
  - **Beds of clipped yew** (`hedge`): every wall tile's sides on a walk carry yew at the old
    fence's height (0.58, so the cameras see past them as before): a dark core, the darkest band
    at the foot, the face in three tiers of clumps each standing out its own depth and turned a
    touch, an uneven clipped top with a rounded edge, sprigs the shears missed. Inside the yew,
    dark earth with cushions of box and laurel, moss and fallen leaves. All `SHADE_SOFT`.
  - **Trees** of trunks and tiered crowns, built of blocks (`garden_tree`): broadleaves on real
    trunks with boughs and three tiers of clumps lit on top, clipped yew columns, and pines of
    drooping boughs. Along the island's rim, taller, darker woods stand against the sky.
  - **Walks of wet gravel:** a new surface (`voxel_surface` kind 11, `gravel` in the shader,
    `SHADE_GRAVEL`), shaded in world space, so a walk reads as one raked bed across its tiles:
    chips each their own shade, rain standing in the hollows, dark and glossy, holding the
    lamps. Turf verges (with tufts) along the hedges' feet, edged by a line of setts; inner
    corners in turf; pebbles, fallen leaves in copper and umber, and puddles (`puddles`,
    drawn as the Cisterns' pools).
  - **Avenues of wet flagstones** (the crypt's `flagstones`, moss in the seams) beside the
    channels and round the fountain.
  - **Stone where there is stone, in the shared builders:** the terraces' retaining walls and
    the channels' banks are the crypt's `masonry` (wet and dark low down on the banks), steps
    are pale dressed stone with moss on their noses, pits have a pale kerb.
  - **The landmarks:**
    - The **temples**: stepped rings of chunky dressed stone, eight columns of the crypt's
      pillar drums, an entablature and cornice, a verdigris dome on copper ribs under a gilt
      finial, the hooded figure (`statue_cubes`) and a flame in a copper dish.
    - The **churchyard**: Scene I's graves (`story_sets::grave`) and dead tree at the garden's
      scale, facing the paths, on uneven dark turf behind low dressed-stone kerbs under a pale
      coping, with rusted railings and spear heads.
    - The **wayside shrines** stand in their beds (the yew opened for them): a short pillar of
      dressed stone with a candle in a niche under a gabled slate roof and an iron cross, or a
      stone urn. A broken wall takes its shrine with it.
    - The **gate**: towers of chunky courses on pale plinths, corbels and battlements under
      slate spires, a round arch of wedge stones on pale piers, an oak tympanum, the lock lamps
      in a fan over it, copper braziers before the towers.
    - The **fountain plaza**: pale stone and charcoal pavers in rings, wet; its four lanterns are
      the garden's iron posts, shorter.
- **Light:**
  - 56 lanterns on iron posts (75 before) (`lamp_post`: a stone foot, a collared post, an arm and its
    brace, the crypt's copper lantern), one at the outer corner of a bed where two walks cross,
    never within six tiles of another, each a pool of its own. The ten nearest the hero burn at
    once (`GARDEN_LANTERNS_LIT`), softer and shorter (reach 5) than the old three.
  - The fountain's water and jets are mist teal and its lamp under the water half as strong; a
    jet's highlight from a lamp beside it is held (`water_jet`: the attenuation in its specular
    capped at 2), so the jets no longer blaze white. The spouts under the bridges ring the water
    instead of throwing foam up through the decks.
  - The churchyard's wisps are pale mist teal, not neon green.
  - The ground mist lies on the walks, water and graves only, in a greyed mist teal; the garden's
    haze is a little thicker and lower (0.042, falloff 0.65), lying in the low walks under the
    terraces.
- **The grade** (`grade_scenery`) leaves alone the ranges built in the board's palette already
  (`WorldView::authored`); the remaining old pieces (bridges, reeds, lilies, flower beds, the
  fountain's sculpt) are graded as before.
- **Unchanged elsewhere:** the Cyclops' meadow (which shares the river, bridges and the old gate
  builder) and the Cisterns capture pixel-identical before and after.
- **Numbers (1080p, RTX 5070 Ti Laptop, interleaved A/B, two rounds):**

  | | Before | After |
  |---|---:|---:|
  | Static cubes | 295,430 | 137,288 |
  | `--bench` 300, the start | 11.5 / 11.1 ms (GPU 7.4 / 8.0) | 8.8 / 8.8 ms (GPU 5.1 / 4.9) |
  | `--bench` 300, 30 foes at the start | 13.7 / 14.5 ms (GPU 10.8 / 10.1) | 12.5 / 12.0 ms (GPU 9.1 / 8.2) |
  | `--perf garden`, wall p50 (GPU p50) | 7.7 / 8.8 ms (7.5 / 8.6) | 7.4 / 7.5 ms (7.2 / 7.3) |
  | `--perf siege` (30 foes, walls breaking), wall p50 | 10.4 / 10.2 ms | 9.3 / 9.3 ms |

  - Building it costs about what it did (`--perf start`: first frame 35 to 45 ms either way);
    the probes are baked behind the loading screen as before.
- **Checks:** `--camera-check dungeon0`: 1 hidden pose of 16,992 in the chase survey (2
  before), none in the walks or the top view. The harness is unchanged: `--run`, `--court`,
  `--pit 2 8 1` and all 16 bouts print the same before and after.
- **Captures:** `C:/Users/ulask/Games/vh-garden-cap/` (`side-*.png` before and after,
  `final-*.png` at 1080p, `garden-vs-cisterns.png`).
- **Still open:**
  - Close to the chase camera the tree crowns read as big cubes, lit lime by the hero's lantern.
  - The Garden is still brighter and cooler than the Cisterns; the moonlit gravel carries
    more of the frame than the Cisterns' dark flagstones.
  - The bridges, reeds, lily pads, flower beds and the fountain's own basin are the old pieces,
    graded.
  - A temple stands on four wall tiles; a bomb breaking one would leave it standing (as before).

## Findings

- **The GPU cost is geometry, not shading.** With every shading effect switched off, the main pass kept
  over half its cost. Instanced cubes at 4× MSAA dominate. This is why stage C came first.
- **Measuring on this laptop needs interleaving.** Its clocks halve and double between runs (the
  Garden's GPU p50 has read 6.7 ms and 16.5 ms on the same build). Only interleaved A/B runs in one
  sitting compare. Stage E's changes measured within noise of RP06's build that way (Garden,
  close-up, 30-foe battle, siege).
- **The 30-foe battle's simulation spikes are fixed (RP06).** They were not AI or pathfinding: past
  the high score, every kill rewrote the save file, which is read and written whole. See RP06 below.
- **The ground mist costs real GPU time.** The 13,500 transparent sheets in the Garden cost about 2 ms
  (see `--fx-off mist`), as the plan warns about large transparent layers.
- **Dark palette colours no longer crush to black (fixed 2026-10-04).**
  - **Cause:** ACES's fit sends light under about 0.3% of white to black and crushes the next few
    percent, and the grade's contrast curve and shadow offset darkened them further.
  - **The toe:** below about 3% the tone curve now runs straight at 0.55 of the light, joined to
    ACES by a smooth maximum (`toe` in both shaders, and the CPU's `aces_filmic`). Above that it is
    unchanged.
  - **The grade:** its contrast eases off in the darks, and its cool shadow cast is now a tint, not
    an offset.
  - **Result:** the Look Dev ramp's darkest steps went from 6, 11, 21 to 11, 15, 25 (0-255), with
    everything above unchanged. In the Bone Vaults, stone in shadow went from 0-12 to 10-19
    before the vaults' fill light was doubled, and now shows its blocks.
  - Other levels change only in their deepest shadows.
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
- **D, lighting:** RP13 to RP19 are done. The presets' numbers need the target GPUs to be set for
  real (stage G).
- **E, world systems:** RP02 to RP06 are done (above). Still open:
  - The Hall of Mirrors' chase camera (at its glass and door frames), and an overhead fade for
    roofs and arches in the top view.
  - A numerical portal-loading target, set on the target machines.
  - TA07's broken pieces and the cue that a wall can be broken; the rubble is a stand-in.
  - Foe pathfinding in the Garden maze: its peaks fell from up to 8 ms to 2 ms a frame (no stalls
    mid-fight); a 30-foe fight's frame is now dominated by the GPU.
- **F, advanced:** RP20 to RP22 evaluated (above). GPU visibility is built but off by default; Hi-Z
  and mesh shaders were not built, by measurement.
- **G, shipping:** the GTX 1080 and RTX 3060 presets, and packaged validation.
- **Calibration** (the plan's own list):
  - Record both target machines.
  - Agree a numerical portal-loading target.
  - Ratify the percentile gates.
  - Approve the realm accents.
- The web build still compiles and lints, but the plan drops the browser target. Rigs and static faces
  now read storage buffers in the vertex shader, which WebGL2 cannot do.
