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
    pause, title) or on quit (`Score::save_high`).
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
- **Camera check:** both pass their walks with no clips or hidden poses. The chase survey hides the
  hero in 33 of 16,248 poses in the Cisterns and 24 of 15,720 in the Vaults.

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
  - Rare foe pathfinding peaks, up to 5 ms, in the Garden maze.
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
