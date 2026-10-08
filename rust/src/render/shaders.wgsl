// Voxel scene shading, matched to the Three.js build: Lambert diffuse under a
// hemisphere, ambient, one moonlight and a pool of point lights with Three's
// distance falloff; flat normals from screen derivatives (Three's flatShading);
// ACES filmic tone mapping; sRGB encode; then linear fog mixed in sRGB.

const PI: f32 = 3.141592653589793;

struct Light {
    pos: vec4<f32>,   // xyz position, w cutoff distance
    color: vec4<f32>, // linear color times intensity
};

struct Globals {
    view_proj: mat4x4<f32>,
    light_vp: mat4x4<f32>, // world to the moon's (or sun's) shadow map: the whole level
    light_vp_near: mat4x4<f32>, // ...and the fine cascade ahead of Ragnar
    cam_pos: vec4<f32>,    // xyz the eye; w 1 for a perspective camera, 0 for the map's orthographic one
    cam_fwd: vec4<f32>,    // xyz camera forward; w shadow map texel size in uv
    fog: vec4<f32>,        // rgb fog color (sRGB), w near
    params: vec4<f32>,     // x fog far, y exposure, z nugget pulse, w light count
    hemi_sky: vec4<f32>,
    hemi_ground: vec4<f32>, // w, and ambient.w and moon_dir.w: the fog as linear light, for HDR
    ambient: vec4<f32>,
    moon_dir: vec4<f32>,
    moon_color: vec4<f32>,
    sky_fwd: vec4<f32>,    // xyz forward view ray, w night sky amount
    sky_right: vec4<f32>,  // xyz to the right screen edge, w time
    sky_up: vec4<f32>,     // xyz to the top screen edge
    sky_moon: vec4<f32>,   // xyz toward the moon
    bounds: vec4<f32>,     // the level's footprint: min x, min z, max x, max z
    shadow: vec4<f32>,     // x fine cascade half width, y coarse half width (world), z 1 when the fine one is drawn, w effects switched off (FX_ bits)
    cluster: vec4<f32>,    // the clusters' screen (pixels w, h) and depth range (near, far): see `cluster_at`
    haze: vec4<f32>,       // ground haze (RP19): density at the floor, the floor's height, falloff
    probe_min: vec4<f32>,  // the probe volume's corner (xyz) and 1 when there is one (RP18)
    probe_size: vec4<f32>, // its extent
    shadow_q: vec4<f32>,   // the preset's shadows (RP16): x blocker search taps, y filter taps
};

// The level's irradiance probes (RP18, `render::probes`): per side (+x, -x, +y, -y, +z, -z), the
// light coming back off what that side sees (rgb, linear radiance) and the share of sky (a).
@group(0) @binding(10) var probe_px: texture_3d<f32>;
@group(0) @binding(11) var probe_nx: texture_3d<f32>;
@group(0) @binding(12) var probe_py: texture_3d<f32>;
@group(0) @binding(13) var probe_ny: texture_3d<f32>;
@group(0) @binding(14) var probe_pz: texture_3d<f32>;
@group(0) @binding(15) var probe_nz: texture_3d<f32>;

struct Probe {
    px: vec4<f32>,
    nx: vec4<f32>,
    py: vec4<f32>,
    ny: vec4<f32>,
    pz: vec4<f32>,
    nz: vec4<f32>,
    inside: bool,
};

// The probes at `world`, nudged off the surface along `n` (so a floor does not see itself).
fn probe_at(world: vec3<f32>, n: vec3<f32>) -> Probe {
    var p: Probe;
    let uvw = (world + n * 0.3 - g.probe_min.xyz) / g.probe_size.xyz;
    p.inside = g.probe_min.w > 0.5 && all(uvw.xz >= vec2<f32>(0.0)) && all(uvw.xz <= vec2<f32>(1.0));
    if (!p.inside) {
        return p;
    }
    p.px = textureSampleLevel(probe_px, env_samp, uvw, 0.0);
    p.nx = textureSampleLevel(probe_nx, env_samp, uvw, 0.0);
    p.py = textureSampleLevel(probe_py, env_samp, uvw, 0.0);
    p.ny = textureSampleLevel(probe_ny, env_samp, uvw, 0.0);
    p.pz = textureSampleLevel(probe_pz, env_samp, uvw, 0.0);
    p.nz = textureSampleLevel(probe_nz, env_samp, uvw, 0.0);
    return p;
}

// What a probe sees toward `d`: its sides blended by how much `d` faces each (an ambient cube).
fn probe_toward(p: Probe, d: vec3<f32>) -> vec4<f32> {
    let w = d * d;
    return select(p.nx, p.px, d.x > 0.0) * w.x + select(p.ny, p.py, d.y > 0.0) * w.y + select(p.nz, p.pz, d.z > 0.0) * w.z;
}

@group(0) @binding(0) var<uniform> g: Globals;

// Effects switched off for comparison captures (`render::profile::fx`).
const FX_SHADOWS: u32 = 1u;
const FX_LIGHTS: u32 = 2u;
const FX_GRASS: u32 = 4u;
const FX_MATERIALS: u32 = 8u;
const FX_FOG: u32 = 16u;
const FX_BACKFACE: u32 = 2048u;
const FX_HAZE: u32 = 8192u;
// Not an effect switched off but a view: how many lights each pixel's cluster holds (RP17).
const FX_LIGHT_HEAT: u32 = 4096u;

fn fx_off(bit: u32) -> bool {
    return (u32(g.shadow.w) & bit) != 0u;
}
// One map per cascade, each its own size (RP16): the fine one round Ragnar, the coarse one over the level.
@group(0) @binding(1) var shadow_near: texture_depth_2d;
@group(0) @binding(16) var shadow_far: texture_depth_2d;
@group(0) @binding(2) var shadow_cmp: sampler_comparison;
// The shadow pass draws one cascade at a time with its own matrix.
@group(0) @binding(3) var<uniform> cascade_vp: mat4x4<f32>;
// The scene's surroundings as light, prefiltered by roughness down its mips (RP14), and the
// split-sum lookup of how much of them a surface reflects (`render::brdf`).
@group(0) @binding(4) var env_map: texture_cube<f32>;
@group(0) @binding(5) var env_samp: sampler;
@group(0) @binding(6) var brdf_lut: texture_2d<f32>;
// Clustered lights (RP17, `render::clusters`): every light, each cluster's run in `light_index`
// (start shifted 8, count in the low 8 bits), and the runs.
@group(0) @binding(7) var<storage, read> lights: array<Light>;
@group(0) @binding(8) var<storage, read> cluster_cells: array<u32>;
@group(0) @binding(9) var<storage, read> light_index: array<u32>;
const TILES_X: u32 = 16u;
const TILES_Y: u32 = 9u;
const SLICES: u32 = 24u;

struct Cluster {
    start: u32,
    count: u32,
};

// The lights that reach the pixel at `frag` (pixels) showing `world`.
fn cluster_at(frag: vec2<f32>, world: vec3<f32>) -> Cluster {
    let tx = min(u32(max(frag.x, 0.0) / g.cluster.x * f32(TILES_X)), TILES_X - 1u);
    let ty = min(u32(max(frag.y, 0.0) / g.cluster.y * f32(TILES_Y)), TILES_Y - 1u);
    let d = dot(world - g.cam_pos.xyz, g.cam_fwd.xyz);
    let k = log(max(d, g.cluster.z) / g.cluster.z) * f32(SLICES) / log(g.cluster.w / g.cluster.z);
    let s = min(u32(max(floor(k), 0.0)), SLICES - 1u);
    let cell = cluster_cells[(s * TILES_Y + ty) * TILES_X + tx];
    return Cluster(cell >> 8u, cell & 255u);
}
// The environment's coarsest mip (roughness 1).
const ENV_MIPS: f32 = 5.0;

struct VIn {
    @location(0) pos: vec3<f32>,
    @location(1) m0: vec4<f32>,
    @location(2) m1: vec4<f32>,
    @location(3) m2: vec4<f32>,
    @location(4) m3: vec4<f32>,
    @location(5) color: vec4<f32>,
    @location(6) emissive: vec4<f32>,
};

struct VOut {
    // Invariant, so the depth prepass and the shading pass land on the very same depths.
    @builtin(position) @invariant clip: vec4<f32>,
    @location(0) world: vec3<f32>,
    @location(1) color: vec4<f32>,
    @location(2) emissive: vec4<f32>,
    @location(3) depth: f32,
    @location(4) local: vec3<f32>, // position inside the unit cube, for per-voxel surface detail
};

@vertex
fn vs_scene(v: VIn) -> VOut {
    let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    let w = m * vec4<f32>(v.pos, 1.0);
    var out: VOut;
    out.clip = g.view_proj * w;
    out.world = w.xyz;
    out.color = v.color;
    out.emissive = v.emissive;
    out.depth = out.clip.w;
    out.local = v.pos;
    return out;
}

// Depth only, from the light: one cascade of the shadow map.
@vertex
fn vs_shadow(v: VIn) -> @builtin(position) vec4<f32> {
    let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return cascade_vp * (m * vec4<f32>(v.pos, 1.0));
}

// ------------------------------------------------------------ resident rigs
//
// Characters stay on the GPU as meshes of their open voxel faces (`voxels::RigMesh`), one quad
// per face, six vertices whose corners the shader makes (as for static faces) from the face it
// reads (`rig_faces`, bound per mesh). Each frame brings only a record
// per rig drawn and its parts' matrices; the vertex picks its part's matrix and colours itself
// as the record says (`frame::rig_look`).

struct RigDraw {
    tint: vec4<f32>,     // rgb tint, a opacity
    emissive: vec4<f32>, // rgb emitted, a how far the blade's metal has rusted
    part_base: u32,
    look: u32,
    rust_mask: u32,
    size: f32,
};

@group(1) @binding(0) var<storage, read> rig_draws: array<RigDraw>;
@group(1) @binding(1) var<storage, read> rig_parts: array<mat4x4<f32>>;

const LOOK_GREY: u32 = 1u;
const LOOK_FADED: u32 = 2u;
const LOOK_GHOST: u32 = 4u;
const LOOK_TINT_ALL: u32 = 8u;
const LOOK_RIM: u32 = 16u;
const LOOK_INNARDS: u32 = 32u;
const LOOK_GOO: u32 = 64u;
// Jelly (`rig_look::GOO`, the bog slime): its material's shade mode, the mode it is drawn in
// (`jelly`), and its heart, in voxels over its root's joint (`foes::SLIME_HEART`): its skin's
// soft normal leans away from there.
const GOO_SHADE: f32 = 14.0;
const JELLY_MODE: f32 = 23.0;
const GOO_HEART: f32 = 11.0;
// A rim-lit tell (`rig_look::RIM`): the share of its light over the body, and on an edge-on face.
const RIM_BODY: f32 = 0.1;
const RIM_EDGE: f32 = 1.5;

// One voxel face of a rig (`voxels::RigFace`).
struct RigFace {
    pos: vec3<f32>,
    part_face: u32,   // the part in the low byte, the face (0 to 5) in the next
    color: vec4<f32>,
};

@group(2) @binding(0) var<storage, read> rig_faces: array<RigFace>;

// The corner each of a quad's six vertices takes, as its two triangles walk them.
const QUAD_CORNERS = array<u32, 6>(0u, 1u, 2u, 0u, 2u, 3u);

struct RIn {
    @location(0) draw: u32,
    @builtin(vertex_index) vertex: u32,
};

@vertex
fn vs_rig(v: RIn) -> VOut {
    let d = rig_draws[v.draw];
    let f = rig_faces[v.vertex / 6u];
    let part = f.part_face & 0xffu;
    let face = (f.part_face >> 8u) & 7u;
    let local = FACE_CORNERS[face * 4u + QUAD_CORNERS[v.vertex % 6u]];
    let pm = rig_parts[d.part_base + part];
    var w = pm * vec4<f32>(f.pos + local * d.size, 1.0);
    var c = f.color.rgb;
    var shade = f.color.a;
    // A slime is drawn twice: what it holds solid, its jelly see-through (each draw drops the other's faces).
    let goo = abs(shade - GOO_SHADE) < 0.5;
    let jelly = (d.look & LOOK_GOO) != 0u;
    if (((d.look & LOOK_INNARDS) != 0u && goo) || (jelly && !goo)) {
        var gone: VOut;
        gone.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
        return gone;
    }
    var soft = vec3<f32>(0.0, 1.0, 0.0);
    var inner = false;
    if (jelly) {
        // Its skin bent soft (leaning away from its heart; its inside looks in), and rippling
        // (`rust` is how hard: a hop, a blow).
        let heart = (rig_parts[d.part_base] * vec4<f32>(0.0, GOO_HEART * d.size, 0.0, 1.0)).xyz;
        let face_n = normalize((pm * vec4<f32>(FACE_NORMALS[face], 0.0)).xyz);
        var out_n = normalize(w.xyz - heart + vec3<f32>(0.0, 1e-4, 0.0));
        let time = g.sky_right.w;
        let wave = sin(dot(w.xyz, vec3<f32>(9.0, 13.0, 7.0)) + time * 4.2) * 0.6 + sin(dot(w.xyz, vec3<f32>(-6.0, 8.0, 11.0)) * 1.3 - time * 6.1) * 0.4;
        w = vec4<f32>(w.xyz + out_n * wave * (0.3 + 1.6 * d.emissive.a) * d.size, 1.0);
        if (dot(out_n, face_n) < 0.0) {
            out_n = -out_n;
            inner = true;
        }
        soft = normalize(mix(face_n, out_n, 0.8));
    }
    // A dull blade: its metal (material 1, mode 6) turns to rust on the parts that rust.
    if (!jelly && d.emissive.a > 0.0 && abs(shade - 6.0) < 0.5 && ((d.rust_mask >> part) & 1u) != 0u) {
        c = mix(c, vec3<f32>(0.32, 0.12, 0.05), d.emissive.a);
    }
    var tint = d.tint.rgb;
    // A voxel lit from within keeps its own light, whatever tints the rest.
    if (abs(shade - 16.0) < 0.5 && (d.look & LOOK_TINT_ALL) == 0u) {
        tint = vec3<f32>(1.0);
    }
    var color = c * tint;
    var emissive = d.emissive.rgb;
    let lightness = sqrt(dot(c, vec3<f32>(0.2126, 0.7152, 0.0722)));
    if ((d.look & LOOK_GREY) != 0u) {
        color = vec3<f32>(0.1 + 0.3 * lightness) * d.tint.rgb;
        emissive = vec3<f32>(0.0);
    }
    if ((d.look & LOOK_FADED) != 0u) {
        color = c;
        shade = 3.0;
        emissive = vec3<f32>(0.0);
    }
    if ((d.look & LOOK_GHOST) != 0u) {
        color = d.tint.rgb * (0.2 + 0.55 * lightness);
        shade = 1.0;
        emissive = vec3<f32>(0.0);
    }
    let outward = (pm * vec4<f32>(FACE_NORMALS[face], 0.0)).xyz;
    let center = (pm * vec4<f32>(f.pos + FACE_NORMALS[face] * 0.5 * d.size, 1.0)).xyz;
    let to_eye = select(-g.cam_fwd.xyz, g.cam_pos.xyz - center, g.cam_pos.w > 0.5);
    if ((d.look & LOOK_RIM) != 0u) {
        // The tell's light on the silhouette: faces seen edge-on take it all, those facing the eye
        // a breath of it, so the body and its move show through.
        // (Jelly by its soft normal: its voxels' ledges are not its silhouette.)
        let facing = clamp(dot(normalize(select(outward, soft, jelly)), normalize(to_eye)), 0.0, 1.0);
        let edge = (1.0 - facing) * (1.0 - facing);
        emissive *= RIM_BODY + RIM_EDGE * edge;
    }
    var out: VOut;
    out.clip = g.view_proj * w;
    out.world = w.xyz;
    out.color = vec4<f32>(color, d.tint.a);
    out.emissive = vec4<f32>(emissive, shade);
    out.depth = out.clip.w;
    out.local = local;
    if (jelly) {
        // (Its soft normal rides where the voxel's place inside its cube would.)
        out.emissive.w = JELLY_MODE;
        out.local = soft;
        // (Its far wall, seen through it from inside, thinner: what it holds shows between.)
        out.color.a *= select(1.0, 0.55, inner);
    }
    // As for static faces: a solid rig's faces turned from the eye never show (nor jelly's: its
    // outside, and its inside's far wall, show). (The face's centre comes from the same numbers
    // at all four corners, so they all decide alike.)
    if (dot(outward, to_eye) <= 0.0 && (d.tint.a > 0.999 || jelly) && (d.look & LOOK_FADED) == 0u && !fx_off(FX_BACKFACE)) {
        out.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    }
    return out;
}

// A portrait (UI.md §6, `Gpu::render_portraits`): the rig lit for the HUD on a clear ground, a
// warm key from above the camera's left, a cool fill from its right, a rim behind; tone mapped and
// written as display colour, solid (the clear ground stays transparent).
@fragment
fn fs_portrait(in: VOut) -> @location(0) vec4<f32> {
    var n = normalize(cross(dpdx(in.world), dpdy(in.world)));
    let to_eye = normalize(g.cam_pos.xyz - in.world);
    if (dot(n, to_eye) < 0.0) {
        n = -n;
    }
    let fwd = normalize(g.cam_fwd.xyz);
    let right = normalize(cross(fwd, vec3<f32>(0.0, 1.0, 0.0)));
    let up = cross(right, fwd);
    let key = normalize(-right * 0.55 + up * 0.8 - fwd * 0.5);
    let fill = normalize(right * 0.85 + up * 0.15 - fwd * 0.5);
    let rim = normalize(right * 0.35 + up * 0.55 + fwd * 0.9);
    let albedo = in.color.rgb;
    // (A warm key like a lamp's, a cool soft fill, a warm rim that lifts the head off the ground.)
    var c = albedo * (vec3<f32>(0.19, 0.17, 0.18)
        + max(dot(n, key), 0.0) * vec3<f32>(1.38, 1.16, 0.88)
        + max(dot(n, fill), 0.0) * vec3<f32>(0.3, 0.36, 0.5)
        + max(n.y, 0.0) * vec3<f32>(0.1, 0.09, 0.08));
    let edge = pow(1.0 - max(dot(n, to_eye), 0.0), 2.0);
    c += vec3<f32>(1.0, 0.78, 0.5) * edge * max(dot(n, rim) + 0.3, 0.0) * 0.7;
    c += in.emissive.rgb;
    if (abs(in.emissive.w - 16.0) < 0.5) {
        // A voxel lit from within keeps its own light.
        c = max(c, albedo * 1.4);
    }
    let shown = aces_filmic(c, 1.0);
    return vec4<f32>(pow(shown, vec3<f32>(1.0 / 2.2)), 1.0);
}

@vertex
fn vs_rig_shadow(v: RIn) -> @builtin(position) vec4<f32> {
    // As for static faces: a face turned from the light is skipped (RP16).
    let d = rig_draws[v.draw];
    let f = rig_faces[v.vertex / 6u];
    let pm = rig_parts[d.part_base + (f.part_face & 0xffu)];
    let face = (f.part_face >> 8u) & 7u;
    let away = vec3<f32>(cascade_vp[0].z, cascade_vp[1].z, cascade_vp[2].z);
    // (A slime's jelly lets the light through: only what it holds casts a shadow.)
    let clear = (d.look & LOOK_INNARDS) != 0u && abs(f.color.a - GOO_SHADE) < 0.5;
    if (clear || dot((pm * vec4<f32>(FACE_NORMALS[face], 0.0)).xyz, away) > 0.0) {
        return vec4<f32>(2.0, 2.0, 2.0, 1.0);
    }
    return cascade_vp * (pm * vec4<f32>(f.pos + FACE_CORNERS[face * 4u + QUAD_CORNERS[v.vertex % 6u]] * d.size, 1.0));
}

// ------------------------------------------------------------ static faces
//
// Static scenery is drawn face by face: each open face of a static cube (`gpu::open_faces`)
// is a quad that reads its cube from the instance store, so a cube rewritten in place (a tree
// fading) changes every face of it at once.

struct Inst {
    m: mat4x4<f32>,
    color: vec4<f32>,
    emissive: vec4<f32>,
};

@group(1) @binding(0) var<storage, read> statics: array<Inst>;

// The corners of each face of the unit cube, in the order the quad's two triangles walk them:
// +x, -x, +y, -y, +z, -z.
const FACE_CORNERS = array<vec3<f32>, 24>(
    vec3<f32>(0.5, -0.5, -0.5), vec3<f32>(0.5, 0.5, -0.5), vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(0.5, -0.5, 0.5),
    vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(-0.5, 0.5, 0.5), vec3<f32>(-0.5, 0.5, -0.5),
    vec3<f32>(-0.5, 0.5, -0.5), vec3<f32>(-0.5, 0.5, 0.5), vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(0.5, 0.5, -0.5),
    vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(0.5, -0.5, -0.5), vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(-0.5, -0.5, 0.5),
    vec3<f32>(-0.5, -0.5, 0.5), vec3<f32>(0.5, -0.5, 0.5), vec3<f32>(0.5, 0.5, 0.5), vec3<f32>(-0.5, 0.5, 0.5),
    vec3<f32>(-0.5, -0.5, -0.5), vec3<f32>(-0.5, 0.5, -0.5), vec3<f32>(0.5, 0.5, -0.5), vec3<f32>(0.5, -0.5, -0.5),
);

const FACE_NORMALS = array<vec3<f32>, 6>(
    vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, -1.0),
);

struct FIn {
    @builtin(vertex_index) corner: u32,
    // The cube's slot in the store times 8, plus which face.
    @location(0) face: u32,
};

@vertex
fn vs_face(v: FIn) -> VOut {
    let inst = statics[v.face >> 3u];
    let face = v.face & 7u;
    let local = FACE_CORNERS[face * 4u + v.corner];
    let w = inst.m * vec4<f32>(local, 1.0);
    var out: VOut;
    out.clip = g.view_proj * w;
    // A face turned from the eye is behind its own cube: an opaque cube's back faces never show.
    // (A cube fading away keeps them, for they show through its dither.) The cube's matrix is a
    // turn times a scale, so it carries the face's outward axis to the outward normal's direction.
    let outward = (inst.m * vec4<f32>(FACE_NORMALS[face], 0.0)).xyz;
    let center = (inst.m * vec4<f32>(FACE_NORMALS[face] * 0.5, 1.0)).xyz;
    let to_eye = select(-g.cam_fwd.xyz, g.cam_pos.xyz - center, g.cam_pos.w > 0.5);
    let fading = abs(inst.emissive.w - 3.0) < 0.5 && inst.color.a < 0.999;
    if (dot(outward, to_eye) <= 0.0 && !fading && !fx_off(FX_BACKFACE)) {
        out.clip = vec4<f32>(2.0, 2.0, 2.0, 1.0);
    }
    out.world = w.xyz;
    out.color = inst.color;
    out.emissive = inst.emissive;
    out.depth = out.clip.w;
    out.local = local;
    return out;
}

// The whole target at the far plane: clears the depth of the part a scissor leaves (the part of
// the cached cascade drawn again).
@vertex
fn vs_depth_clear(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
    return vec4<f32>(p, 1.0, 1.0);
}

@vertex
fn vs_face_shadow(v: FIn) -> @builtin(position) vec4<f32> {
    let inst = statics[v.face >> 3u];
    let face = v.face & 7u;
    // A face turned from the light is never nearest to it (the cube's lit faces cover the same
    // ground in front of it), so it is skipped (RP16): the map comes out the same. The cascade's
    // depth row says which way is away from the light.
    let away = vec3<f32>(cascade_vp[0].z, cascade_vp[1].z, cascade_vp[2].z);
    if (dot((inst.m * vec4<f32>(FACE_NORMALS[face], 0.0)).xyz, away) > 0.0) {
        return vec4<f32>(2.0, 2.0, 2.0, 1.0);
    }
    return cascade_vp * (inst.m * vec4<f32>(FACE_CORNERS[face * 4u + v.corner], 1.0));
}

// How wide the moon (or sun) looks: sets how fast a shadow softens with distance.
const LIGHT_SIZE: f32 = 0.035;
// The light's depth range, near to far, in world units.
const LIGHT_DEPTH: f32 = 199.0;

// Point `i` of `n` on a disc, spread evenly (a Vogel spiral), turned by `spin`.
fn vogel(i: i32, n: i32, spin: f32) -> vec2<f32> {
    let r = sqrt((f32(i) + 0.5) / f32(n));
    let a = f32(i) * 2.39996 + spin;
    return vec2<f32>(cos(a), sin(a)) * r;
}

// One cascade's verdict for `world`: (how lit, how far inside the cascade).
// Soft shadows that harden where an object meets what it shades: a search
// for the things between here and the light says how far above they are,
// and the filter spreads with that distance.
fn cascade_light(map: texture_depth_2d, vp: mat4x4<f32>, world: vec3<f32>, half_width: f32, bias: f32, spin: f32) -> vec2<f32> {
    let lp = vp * vec4<f32>(world, 1.0);
    let c = lp.xyz / lp.w;
    let uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
    let edge = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    if (edge <= 0.0 || c.z > 1.0) {
        return vec2<f32>(1.0, 0.0);
    }
    let inside = smoothstep(0.0, 0.06, edge);
    let size = f32(textureDimensions(map).x);
    let texel = 1.0 / size;
    let z = c.z - bias;
    var blockers = 0.0;
    var found = 0.0;
    let search = i32(g.shadow_q.x);
    let taps = i32(g.shadow_q.y);
    // The search reaches as far in the world whatever the map's size.
    let reach = texel * 7.0 * size / 4096.0;
    for (var i = 0; i < search; i++) {
        let q = clamp((uv + vogel(i, search, spin) * reach) * size, vec2<f32>(0.0), vec2<f32>(size - 1.0));
        let d = textureLoad(map, vec2<i32>(q), 0);
        if (d < z) {
            blockers += d;
            found += 1.0;
        }
    }
    if (found < 0.5) {
        return vec2<f32>(1.0, inside);
    }
    let gap = (z - blockers / found) * LIGHT_DEPTH;
    let spread = clamp(gap * LIGHT_SIZE / (2.0 * half_width), texel * 1.2, texel * 9.0);
    var lit = 0.0;
    for (var i = 0; i < taps; i++) {
        lit += textureSampleCompareLevel(map, shadow_cmp, uv + vogel(i, taps, spin) * spread, z);
    }
    return vec2<f32>(lit / f32(taps), inside);
}

// How much of the moonlight (or sunlight) reaches `world`: the fine cascade
// where it covers, fading into the coarse one toward its edge.
fn sun_visibility(world: vec3<f32>, n: vec3<f32>, frag: vec2<f32>) -> f32 {
    if (fx_off(FX_SHADOWS)) {
        return 1.0;
    }
    let spin = fract(52.9829189 * fract(dot(frag, vec2<f32>(0.06711056, 0.00583715)))) * 6.2831853;
    // Surfaces are pushed off along their normal by three texels of the cascade they read.
    let near_size = f32(textureDimensions(shadow_near).x);
    let far_size = f32(textureDimensions(shadow_far).x);
    let far_w = g.shadow.y;
    if (g.shadow.z > 0.5) {
        let near_w = g.shadow.x;
        let near = cascade_light(shadow_near, g.light_vp_near, world + n * (3.0 * near_w / near_size), near_w, 0.02 / LIGHT_DEPTH, spin);
        if (near.y > 0.999) {
            return near.x;
        }
        let far = cascade_light(shadow_far, g.light_vp, world + n * (3.0 * far_w / far_size), far_w, 0.05 / LIGHT_DEPTH, spin);
        return mix(far.x, near.x, near.y);
    }
    return cascade_light(shadow_far, g.light_vp, world + n * (3.0 * far_w / far_size), far_w, 0.05 / LIGHT_DEPTH, spin).x;
}

// ------------------------------------------------------------ voxel materials
//
// Character voxels carry a material (shade mode 6 and up). Each face of each
// voxel gets procedural detail in its own little square, so it sticks to the
// model as it moves: woven threads, fur strands, hammered dents, wood grain,
// and a bevel that darkens every voxel's edges. Detail fades out when a voxel
// covers only a few pixels, so it never shimmers.

// A surface as the lighting sees it (RP14): base colour, roughness, metalness and a specular
// level (0.5 is the usual 4% reflectance of a dielectric), plus two soft-material terms.
struct Surface {
    albedo: vec3<f32>,
    n: vec3<f32>,
    rough: f32,
    metal: f32,
    specular: f32,
    wrap: f32,     // light wrapping past the terminator (soft materials)
    sheen: f32,    // rim glow at grazing angles (fur, cloth)
};

fn hash12(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
}

// Smooth value noise over the plane (for stone mottle and puddles).
fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn voxel_uv(local: vec3<f32>) -> vec2<f32> {
    let a = abs(local);
    if (a.x >= a.y && a.x >= a.z) {
        return local.zy + 0.5;
    }
    if (a.y >= a.z) {
        return local.xz + 0.5;
    }
    return local.xy + 0.5;
}

// Wet gravel (`voxel_surface` kind 11, the garden's walks), worked out in world space so the
// walk reads as one bed across its tiles: small chips of stone, each its own shade with a dark
// gap round it, and rain standing in the hollows, dark and glossy, holding the lamps.
fn gravel(m_in: Surface, world: vec3<f32>, n: vec3<f32>, detail: f32) -> Surface {
    var m = m_in;
    m.rough = 0.82;
    m.specular = 0.45;
    if (n.y < 0.5) {
        return m;
    }
    // Chips: a fine speckle, each a stone of its own shade, over a broader mottle of the
    // stones' sizes and colours (no grid: the walk is raked, not laid).
    let p = world.xz;
    let speck = hash12(floor(p * 38.0 + vec2<f32>(floor(p.y * 38.0) * 0.5, 0.0)));
    let grain = vnoise(p * 17.0) * 0.6 + vnoise(p * 6.3) * 0.4;
    let chip = (0.62 + 0.42 * speck) * (0.8 + 0.4 * grain);
    m.albedo *= mix(1.0, chip, detail);
    let tilt = vec2<f32>(vnoise(p * 23.0 + vec2<f32>(7.0, 1.0)), vnoise(p * 23.0 + vec2<f32>(2.0, 9.0))) - vec2<f32>(0.5);
    m.n = normalize(n + vec3<f32>(tilt.x, 0.0, tilt.y) * 0.5 * detail);
    let wet = smoothstep(0.5, 0.62, vnoise(p * 0.8 + vec2<f32>(3.1, 1.7)) * 0.75 + vnoise(p * 2.3) * 0.25);
    m.albedo *= mix(1.0, 0.55, wet);
    m.rough = mix(m.rough, 0.07, wet);
    m.specular = mix(m.specular, 0.9, wet);
    m.n = normalize(mix(m.n, n, wet));
    return m;
}

// How far `p` lies from the nearest border between the cells of a jittered grid (0 on a border):
// the cracks between plates of cooling crust.
fn cell_border(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    var d1 = 8.0;
    var d2 = 8.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let o = vec2<f32>(f32(x), f32(y));
            let r = o + vec2<f32>(0.15) + hash22(i + o) * 0.7 - f;
            let d = dot(r, r);
            if (d < d1) {
                d2 = d1;
                d1 = d;
            } else if (d < d2) {
                d2 = d;
            }
        }
    }
    return sqrt(d2) - sqrt(d1);
}

// Lava (`voxel_surface` kind 12, the Portals of Hell), worked out in world space so a sea or a
// river reads as one flow across its blocks: plates of dark crust drifting slowly, cracked, the
// heat glowing in their cracks; where it runs hot the plates melt away into open flow, ember
// going to amber. Upright faces run downhill (the volcano's rivers, the falls off the platforms).
// Returns the crust's shade, the glow (about 1 on the average) and how hot it runs (0 to 1).
fn lava_at(world: vec3<f32>, n: vec3<f32>, time: f32) -> vec3<f32> {
    var p = world.xz;
    var flow = vec2<f32>(0.05, 0.03) * time;
    if (abs(n.y) < 0.5) {
        p = vec2<f32>((world.x + world.z) * 1.3, world.y * 1.6);
        flow = vec2<f32>(0.0, 0.55) * time;
    }
    let heat = vnoise(p * 0.22 + flow * 0.25) * 0.5 + vnoise(p * 0.6 + vec2<f32>(7.0, 3.0) - flow * 0.5) * 0.32
        + vnoise(p * 1.7 + flow) * 0.18;
    let hot = clamp((heat - 0.46) / 0.3, 0.0, 1.0);
    // Plates of uneven size (the grid warped), their cracks healed over here and there.
    let warp = vec2<f32>(vnoise(p * 0.7 + vec2<f32>(3.0, 1.0)), vnoise(p * 0.7 + vec2<f32>(8.0, 5.0))) - vec2<f32>(0.5);
    let c = p * 1.15 + warp * 1.4 + flow * 1.4;
    let e = cell_border(c);
    let healed = smoothstep(0.35, 0.7, vnoise(c * 0.9 + vec2<f32>(11.0, 4.0)));
    let crack = (1.0 - smoothstep(0.01, 0.035 + 0.16 * hot, e)) * (1.0 - healed * (1.0 - hot));
    let melt = smoothstep(0.42, 0.8, hot + 0.25 * (vnoise(c * 1.9 + vec2<f32>(time * 0.15, 0.0)) - 0.5));
    let open = max(crack * (0.12 + 0.88 * hot), melt);
    // The crust: darker plates, paler and greyer where it has cooled longest.
    let shade = (0.65 + 0.6 * vnoise(c * 3.3)) * (1.0 - 0.6 * open);
    // A slow pulse where the flow is open.
    let pulse = 0.85 + 0.15 * sin(time * 1.3 + heat * 9.0);
    return vec3<f32>(shade, open * (0.6 + 0.9 * hot * hot) * pulse + 0.03, hot);
}

fn voxel_surface(kind: i32, local: vec3<f32>, detail: f32, bevel_detail: f32, base: vec3<f32>, n: vec3<f32>, seed: f32) -> Surface {
    var m: Surface;
    m.albedo = base;
    m.n = n;
    m.rough = 0.9;
    m.metal = 0.0;
    m.specular = 0.5;
    m.wrap = 0.0;
    m.sheen = 0.0;
    let uv = voxel_uv(local);
    let e = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    // (Slime has no edges; nor has wet gravel, laid as one bed across the walks.)
    if (kind != 9 && kind != 11) {
        m.albedo *= mix(1.0, mix(0.9, 1.0, smoothstep(0.0, 0.11, e)), bevel_detail);
    }
    let helper = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.9);
    let t = normalize(cross(helper, n));
    let b = cross(n, t);
    switch kind {
        case 1: {
            // Hammered iron: shallow dents that bend the highlight.
            let c = uv * 2.0 + vec2<f32>(seed, seed * 1.7);
            let cell = fract(c) - 0.5;
            let dent = hash12(floor(c));
            m.n = normalize(n + (t * cell.x + b * cell.y) * 0.55 * detail * (0.4 + dent));
            m.albedo *= mix(1.0, 0.86 + 0.22 * dent, detail) * 0.55;
            // Worn, not polished: broad highlights that catch every light, and a little diffuse left.
            m.rough = 0.45;
            m.metal = 0.7;
        }
        case 2: {
            // Fur and hair: strands of varied shade, soft light, a sheen at the rim.
            let col = floor(uv.x * 5.0);
            let s = hash12(vec2<f32>(col, seed * 31.0));
            let strand = 0.7 + 0.55 * s * (0.65 + 0.35 * sin(uv.y * 8.0 + s * 6.0));
            m.albedo *= mix(1.0, strand, detail);
            m.wrap = 0.35;
            m.sheen = 0.3;
            m.rough = 0.75;
            m.specular = 0.25;
        }
        case 3: {
            // Woven cloth: over-under threads, four to a voxel.
            let tt = uv * 4.0;
            let f = fract(tt);
            let over = (i32(floor(tt.x)) + i32(floor(tt.y))) % 2 == 0;
            let thread = select(sin(f.y * PI), sin(f.x * PI), over);
            m.albedo *= mix(1.0, 0.86 + 0.18 * thread, detail);
            m.wrap = 0.15;
            m.sheen = 0.12;
            m.rough = 0.9;
            m.specular = 0.3;
        }
        case 4: {
            // Leather: a soft mottle and a dull shine.
            let g = hash12(floor(uv * 5.0) + seed * 13.0);
            m.albedo *= mix(1.0, 0.86 + 0.24 * g, detail);
            m.rough = 0.55;
            m.specular = 0.45;
        }
        case 5: {
            // Skin: light wraps around, a faint oily shine.
            m.wrap = 0.45;
            m.rough = 0.5;
        }
        case 6: {
            // Ivory horn: growth rings and a polished gloss.
            m.albedo *= mix(1.0, 0.93 + 0.08 * sin(uv.y * 19.0 + seed * 5.0), detail);
            m.rough = 0.35;
            m.specular = 0.6;
        }
        case 7: {
            // Wood: long grain.
            let grain = abs(sin(uv.x * 13.0 + sin(uv.y * 4.0 + seed * 9.0) * 1.8));
            m.albedo *= mix(1.0, 0.8 + 0.26 * grain, detail);
            m.rough = 0.75;
            m.specular = 0.4;
        }
        case 9: {
            // Slime: wet and glossy, a sharp highlight and light soaking in.
            m.rough = 0.12;
            m.specular = 1.0;
            m.wrap = 0.4;
            m.sheen = 0.2;
        }
        case 8: {
            // Gold and bronze: bright, warm, polished.
            m.rough = 0.38;
            m.metal = 0.7;
            m.albedo *= 0.85;
        }
        case 10: {
            // Polished marble: a high gloss, faint veins wandering through it.
            let c = uv * 3.0 + vec2<f32>(seed * 7.0, seed * 3.0);
            let vein = abs(sin(c.x * 2.1 + sin(c.y * 3.7 + seed * 9.0) * 1.6 + sin(c.y * 1.3) * 0.8));
            let v = smoothstep(0.93, 1.0, vein);
            m.albedo *= mix(1.0, 0.86, v * detail);
            // (Glossier than real stone: the vaults' marble is meant to shine.)
            m.rough = 0.15;
            m.specular = 1.5;
        }
        case 13: {
            // Dressed stone (the Bone Vaults): chisel-pocked, a mottle of tool marks and grime,
            // darker in its pits; broad and rough.
            let q = uv * 3.0 + vec2<f32>(seed * 11.0, seed * 5.0);
            let mottle = vnoise(q) * 0.6 + vnoise(q * 2.7) * 0.4;
            let pits = smoothstep(0.62, 0.8, vnoise(q * 5.3));
            m.albedo *= mix(1.0, (0.82 + 0.3 * mottle) * (1.0 - 0.25 * pits), detail);
            m.rough = 0.85;
            m.specular = 0.45;
        }
        case 14: {
            // Wet flagstone: the same stone, with standing water in its hollows: darker there and
            // glossy, so the braziers' light lies in it.
            let q = uv * 2.2 + vec2<f32>(seed * 7.0, seed * 13.0);
            let mottle = vnoise(q * 1.6) * 0.6 + vnoise(q * 4.1) * 0.4;
            let puddle = smoothstep(0.5, 0.68, vnoise(q * 0.9 + vec2<f32>(3.1, 1.7)));
            m.albedo *= mix(1.0, 0.85 + 0.25 * mottle, detail) * mix(1.0, 0.62, puddle);
            m.rough = mix(0.6, 0.06, puddle);
            m.specular = mix(0.5, 0.9, puddle);
        }
        default: {}
    }
    return m;
}

// The scene pass writes color and, for post-processing, the world normal and view depth.
struct FOut {
    @location(0) color: vec4<f32>,
    @location(1) gbuf: vec4<f32>,
    // The part of `color` that is indirect light (RP15): what contact shading may take away.
    @location(2) indirect: vec4<f32>,
};

fn rrt_odt_fit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}


// The tone curve's toe, lifted: ACES's fit sends everything under about 0.3% of white to black
// and crushes the next few percent, so dark palette colours (charcoal, umber) read as black. Below
// about 3% the curve now runs straight at TOE_SLOPE of the light, joined to ACES by a smooth
// maximum; above that ACES is untouched. (`post.wgsl` and `gpu::aces_filmic` have the same.)
const TOE_SLOPE: f32 = 0.55;
const TOE_KNEE: f32 = 0.004;

fn toe(aces: vec3<f32>, light: vec3<f32>) -> vec3<f32> {
    let lin = max(light, vec3<f32>(0.0)) * TOE_SLOPE;
    let h = max(vec3<f32>(TOE_KNEE) - abs(aces - lin), vec3<f32>(0.0)) / TOE_KNEE;
    return max(aces, lin) + h * h * TOE_KNEE * 0.25;
}

fn aces_filmic(c_in: vec3<f32>, exposure: f32) -> vec3<f32> {
    let input_m = mat3x3<f32>(
        vec3<f32>(0.59719, 0.07600, 0.02840),
        vec3<f32>(0.35458, 0.90834, 0.13383),
        vec3<f32>(0.04823, 0.01566, 0.83777),
    );
    let output_m = mat3x3<f32>(
        vec3<f32>(1.60475, -0.10208, -0.00327),
        vec3<f32>(-0.53108, 1.10813, -0.07276),
        vec3<f32>(-0.07367, -0.00605, 1.07602),
    );
    var c = c_in * (exposure / 0.6);
    c = input_m * c;
    c = rrt_odt_fit(c);
    c = output_m * c;
    return clamp(toe(c, c_in * exposure), vec3<f32>(0.0), vec3<f32>(1.0));
}

// ------------------------------------------------------------ output (RP13)
//
// The scene target holds light itself (linear, times the exposure, in half floats): fog, alpha
// blending and additive effects mix as light, and the composite tone maps and encodes once.
// Colours made to look a certain way on screen (the sky, the fog's colour) become the light
// that tone maps to them, so they look as they were drawn.

// Worked-out light (linear radiance) as the target holds it.
fn finish(col: vec3<f32>) -> vec3<f32> {
    return max(col, vec3<f32>(0.0)) * g.params.y;
}

// How much of the light from `world` the air between it and the eye takes away: the fog by
// distance, and the ground haze (RP19), thickest at its floor, worked out exactly along the ray
// (its density falls off exponentially with height, so the ray's mean density has a closed form).
fn fog_at(world: vec3<f32>, depth: f32) -> f32 {
    let distance_fog = smoothstep(g.fog.w, g.params.x, depth);
    let d0 = g.haze.x;
    if (d0 <= 0.0 || fx_off(FX_HAZE)) {
        return distance_fog;
    }
    let eye = g.cam_pos.xyz;
    let fall = max(g.haze.z, 0.05);
    let ye = max(eye.y - g.haze.y, 0.0);
    let yp = max(world.y - g.haze.y, 0.0);
    let fe = exp(-ye / fall);
    let fp = exp(-yp / fall);
    let dy = yp - ye;
    let mean = select(fe, fall * (fe - fp) / dy, abs(dy) > 1e-3);
    let haze = 1.0 - exp(-d0 * length(world - eye) * mean);
    return 1.0 - (1.0 - distance_fog) * (1.0 - haze);
}

// `c` from `finish`, a fraction `f` of the way into the fog (as light: see `gpu::untonemap`).
fn fogged(c: vec3<f32>, f: f32) -> vec3<f32> {
    return mix(c, vec3<f32>(g.hemi_ground.w, g.ambient.w, g.moon_dir.w), f);
}

fn srgb_to_lin(c: vec3<f32>) -> vec3<f32> {
    let hi = pow((max(c, vec3<f32>(0.0)) + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(hi, c / 12.92, c <= vec3<f32>(0.04045));
}

// The light that tone maps to linear display colour `t` (the curve turned round, a few steps).
fn untonemap(t: vec3<f32>) -> vec3<f32> {
    let goal = clamp(t, vec3<f32>(0.0), vec3<f32>(0.97));
    var x = goal;
    for (var i = 0; i < 8; i++) {
        let y = aces_filmic(x, 1.0);
        x = x * clamp(goal / max(y, vec3<f32>(1e-5)), vec3<f32>(0.25), vec3<f32>(4.0));
    }
    return x;
}

// A colour made to look a certain way on screen (the sky, sRGB) as the target holds it.
fn from_display(srgb: vec3<f32>) -> vec3<f32> {
    return untonemap(srgb_to_lin(srgb));
}

fn bayer4(p: vec2<f32>) -> f32 {
    var m = array<f32, 16>(0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0);
    let i = (u32(p.y) % 4u) * 4u + (u32(p.x) % 4u);
    return (m[i] + 0.5) / 16.0;
}

fn pow2(x: f32) -> f32 {
    return x * x;
}

// ------------------------------------------------------------ grass (POM)
//
// Parallax occlusion mapping on the top face of a grass tile: the face is the
// tips of a field of blades, and the view ray is marched down into a
// procedural height field of tapered blades until it hits one. What it hits
// sets the color: dark down at the roots, bright at the tips, each blade its
// own shade. No textures; the field is a hashed cell grid in world xz.

const GRASS_FREQ: f32 = 26.0;   // blade cells per world unit
const GRASS_DEPTH: f32 = 0.07;  // how far below the surface the roots sit, world units

fn hash22(p: vec2<f32>) -> vec2<f32> {
    var q = vec2<f32>(dot(p, vec2<f32>(127.1, 311.7)), dot(p, vec2<f32>(269.5, 183.3)));
    return fract(sin(q) * 43758.5453);
}

// Height (0 roots .. 1 tips) of the field at `uv`, and the id of the blade there.
fn blade_field(uv: vec2<f32>) -> vec2<f32> {
    let cell = floor(uv);
    let f = uv - cell;
    var best = 0.0;
    var id = 0.0;
    for (var j = -1; j <= 1; j++) {
        for (var i = -1; i <= 1; i++) {
            let o = vec2<f32>(f32(i), f32(j));
            let r = hash22(cell + o);
            let center = o + vec2<f32>(0.15) + 0.7 * r;
            let tall = 0.5 + 0.5 * fract(r.x * 7.13 + r.y * 3.71);
            // A blade: a thin ridge at a random angle, pointed at one end.
            let a = fract(r.y * 9.31 + r.x * 1.7) * 6.2832;
            let axis = vec2<f32>(cos(a), sin(a));
            let p = f - center;
            let along = dot(p, axis);
            let across = abs(dot(p, vec2<f32>(-axis.y, axis.x)));
            let len = 0.62;
            let taper = clamp(0.5 - along / (2.0 * len), 0.0, 1.0);
            let width = 0.32 * taper + 0.03;
            let ridge = clamp(1.0 - across / width, 0.0, 1.0);
            let h = tall * ridge * sqrt(taper) * step(abs(along), len);
            if (h > best) {
                best = h;
                id = fract(r.x * 13.7 + r.y * 5.3);
            }
        }
    }
    return vec2<f32>(best, id);
}

// Returns (brightness multiplier, blade id, fade) for a grass top face at `world`.
fn grass_shade(world: vec3<f32>, view_depth: f32) -> vec3<f32> {
    let fade = 1.0 - smoothstep(9.0, 22.0, view_depth);
    if (fade <= 0.0) {
        return vec3<f32>(0.9, 0.5, 0.0);
    }
    let v = normalize(g.cam_pos.xyz - world);
    let steps = select(10, 18, view_depth < 8.0);
    let uv0 = world.xz * GRASS_FREQ;
    // How far the ray slides across the field while it sinks the full depth.
    let slide = -v.xz / max(v.y, 0.2) * (GRASS_DEPTH * GRASS_FREQ);
    let dstep = 1.0 / f32(steps);
    var depth = 0.0;
    var hb = blade_field(uv0);
    var prev_depth = 0.0;
    var prev_gap = 1.0 - hb.x;
    for (var s = 0; s < steps; s++) {
        let gap = (1.0 - hb.x) - depth;
        if (gap <= 0.0) {
            // Refine between the last two samples.
            let t = prev_gap / max(prev_gap - gap, 1e-4);
            depth = mix(prev_depth, depth, clamp(t, 0.0, 1.0));
            hb = blade_field(uv0 + slide * depth);
            break;
        }
        prev_depth = depth;
        prev_gap = gap;
        depth += dstep;
        hb = blade_field(uv0 + slide * depth);
    }
    let h = 1.0 - depth;
    let shade = mix(0.34, 1.3, h * h * (3.0 - 2.0 * h)) * (0.85 + 0.3 * hb.y);
    return vec3<f32>(mix(0.9, shade, fade), hb.y, fade * h);
}

// ------------------------------------------------------------ cloud sea
//
// Each layer is a flat sheet of drifting fbm mist. Layers above the floor
// only appear outside the level's footprint, so they wrap its sides; every
// layer thins out far from the level, so the night sky shows past the edge.

fn cnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = fract(sin(dot(i, vec2<f32>(127.1, 311.7))) * 43758.5453);
    let b = fract(sin(dot(i + vec2<f32>(1.0, 0.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
    let c = fract(sin(dot(i + vec2<f32>(0.0, 1.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
    let d = fract(sin(dot(i + vec2<f32>(1.0, 1.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn cfbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 5; i++) {
        v += a * cnoise(p);
        p = mat2x2<f32>(1.6, 1.2, -1.2, 1.6) * p + vec2<f32>(3.1, 1.7);
        a *= 0.5;
    }
    return v;
}

// Value noise with quintic easing: smooth enough in its slope that shading
// by that slope shows no creases along the lattice.
fn qnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = fract(sin(dot(i, vec2<f32>(127.1, 311.7))) * 43758.5453);
    let b = fract(sin(dot(i + vec2<f32>(1.0, 0.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
    let c = fract(sin(dot(i + vec2<f32>(0.0, 1.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
    let d = fract(sin(dot(i + vec2<f32>(1.0, 1.0), vec2<f32>(127.1, 311.7))) * 43758.5453);
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Three octaves of it, each turned against the last, the first turned off the axes too.
fn cfbm3(p_in: vec2<f32>) -> f32 {
    var p = mat2x2<f32>(0.8, 0.6, -0.6, 0.8) * p_in;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 3; i++) {
        v += a * qnoise(p);
        p = mat2x2<f32>(1.6, 1.2, -1.2, 1.6) * p + vec2<f32>(3.1, 1.7);
        a *= 0.5;
    }
    return v / 0.875;
}

// The smoke wall round the island: tall curtains of billowing smoke, rank
// behind rank, rising slowly and heaving at the crest, about five times
// Ragnar's height. The moon catches the tops; the depths stay dark.
fn smoke_wall(in: VOut) -> FOut {
    let time = g.sky_right.w;
    let rank = in.emissive.x - 30.0;
    let out_d = in.emissive.y;
    let side = in.emissive.z;
    let along_x = side < 1.5;
    let along = select(in.world.z, in.world.x, along_x);
    let y = in.world.y;
    let u = along * 0.21 + rank * 7.31;
    let crest = 5.8 + (qnoise(vec2<f32>(u * 0.55 + time * 0.035, rank * 3.0)) - 0.5) * 1.9 - out_d * 0.06;
    if (y > crest + 1.0) {
        discard;
    }
    let rise = time * (0.1 + 0.02 * rank);
    let p = vec2<f32>(u, y * 0.26 - rise);
    let pw = mat2x2<f32>(0.6, -0.8, 0.8, 0.6) * p * 1.3;
    let warp = vec2<f32>(qnoise(pw + vec2<f32>(3.1, rise * 0.3)), qnoise(pw + vec2<f32>(7.7, 1.9)));
    let q = p + warp * 1.25;
    let n = cfbm3(q);
    // Each billow's slope in the curtain's plane: the sides that bulge toward the moon catch its light.
    let e = 0.3;
    let grad = vec2<f32>(cfbm3(q + vec2<f32>(e, 0.0)) - n, cfbm3(q + vec2<f32>(0.0, e)) - n) / e;
    let md = g.moon_dir.xyz;
    let moon2 = vec2<f32>(select(md.z, md.x, along_x), md.y);
    let shade = clamp(0.5 - dot(grad, moon2) * 0.9, 0.1, 1.5);
    // Solid low down, breaking into billows toward the crest.
    let body = smoothstep(crest + 0.4, crest - 2.4, y + (n - 0.5) * 1.8);
    var cover = body * smoothstep(0.18, 0.55, n + body * 0.3);
    // Never a flat wall right in front of the camera.
    cover *= smoothstep(0.7, 3.5, length(g.cam_pos.xyz - in.world));
    // Thin out toward the curtain's ends, where it overlaps the next side's.
    let b = g.bounds;
    let half_len = select(b.w - b.y, b.z - b.x, along_x) * 0.5 + out_d + 3.0;
    let mid = select((b.y + b.w) * 0.5, (b.x + b.z) * 0.5, along_x);
    cover *= smoothstep(half_len, half_len - 2.5, abs(along - mid));
    // Moonlit billows over dark depths, brightest toward the crest; the outer ranks sink into the night.
    let lit = smoothstep(crest - 3.4, crest - 0.3, y);
    let tint = in.color.rgb * 2.4;
    let col = tint * (0.3 + 0.7 * n) * (0.45 + 0.75 * shade) + g.moon_color.rgb * 0.045 * shade * n * (0.35 + 0.65 * lit);
    let rgb = finish(col);
    let fog_f = smoothstep(g.fog.w, g.params.x, in.depth);
    var out: FOut;
    out.color = vec4<f32>(fogged(rgb, fog_f * 0.45), cover * in.color.a * (1.0 - 0.08 * rank));
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

fn cloud_sea(in: VOut) -> FOut {
    let seed = in.emissive.x;
    let layer_y = in.emissive.y;
    let time = g.sky_right.w;
    let xz = in.world.xz;
    if (seed > 29.5) {
        return smoke_wall(in);
    }
    if (seed > 19.5) {
        return ground_mist(in, seed - 20.0);
    }
    let drift = vec2<f32>(0.11, 0.05) * time * (0.6 + 0.15 * seed);
    let p = xz * 0.16 + vec2<f32>(seed * 17.3, seed * 9.1);
    // Domain warp for billowing shapes.
    let warp = vec2<f32>(cfbm(p * 0.7 + drift * 0.5), cfbm(p * 0.7 - drift.yx * 0.5 + 5.2));
    let n = cfbm(p + drift + warp * 1.6);

    // Signed distance outside the level's footprint (negative inside).
    let b = g.bounds;
    let q = max(vec2<f32>(b.x, b.y) - xz, xz - vec2<f32>(b.z, b.w));
    let outside = length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0);

    var cover = smoothstep(0.28, 0.56, n);
    if (seed < 0.5) {
        cover = 0.75 + 0.25 * cover; // the floor of the sea: nearly solid
    }
    // Above the floor, hug the outside of the level and pile up against it.
    if (layer_y > -0.2) {
        // A dense bank right against the walls, breaking into wisps further out.
        let reach = 2.0 + 4.0 * n - (layer_y + 0.1) * 1.6;
        let bank = smoothstep(reach, reach * 0.25, outside) * (0.6 + 0.4 * smoothstep(0.3, 0.6, n));
        cover = max(cover * (1.0 - smoothstep(3.0, 7.0, outside)), bank);
        cover *= smoothstep(-0.2, 0.25, outside);
    }
    // Thin out away from the level, so the starry sky shows beyond.
    cover *= 1.0 - smoothstep(3.0 + (layer_y + 1.6) * 1.0, 9.5 + (1.6 - abs(layer_y)) * 2.0, outside + (n - 0.5) * 6.0);

    // Moonlit billow tops, darker troughs, a cool rim of light at the level's edge.
    let tint = in.color.rgb;
    var col = mix(tint * 0.35, tint * 2.1 + vec3<f32>(0.015, 0.018, 0.03), smoothstep(0.4, 0.85, n));
    col += vec3<f32>(0.02, 0.03, 0.03) * smoothstep(2.0, 0.0, abs(outside)) * smoothstep(0.4, 0.8, n);
    let rgb = finish(col);
    let fog_f = smoothstep(g.fog.w, g.params.x, in.depth);
    var out: FOut;
    out.color = vec4<f32>(fogged(rgb, fog_f * 0.6), cover * in.color.a);
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// Low night mist in slow drifting banks, thinning right in front of the camera
// so it never fogs the view of Ragnar. Moonlit, and warmed by lights nearby.
fn ground_mist(in: VOut, layer: f32) -> FOut {
    let time = g.sky_right.w;
    let drift = vec2<f32>(0.16, 0.06) * time * (1.0 + 0.3 * layer);
    let p = in.world.xz * 0.42 + vec2<f32>(layer * 7.1, layer * 3.3);
    let warp = vec2<f32>(cfbm(p * 0.5 + drift * 0.4), cfbm(p * 0.5 - drift.yx * 0.4 + 3.7));
    let n = cfbm(p + drift + warp * 1.3);
    var cover = smoothstep(0.34, 0.74, n) * in.color.a;
    cover *= smoothstep(1.2, 4.5, length(g.cam_pos.xyz - in.world));
    // Thin away toward the island's edge, so the sheets never end in a hard line against the sky.
    let b = g.bounds;
    let inside = min(min(in.world.x - b.x, b.z - in.world.x), min(in.world.z - b.y, b.w - in.world.z));
    cover *= smoothstep(0.0, 0.9, inside);
    // Moonlight on the mist, and a warm glow where lanterns and candles are near.
    var light = g.moon_color.rgb * 0.22 + g.hemi_sky.rgb * 0.25 + g.ambient.rgb;
    let cl = cluster_at(in.clip.xy, in.world);
    for (var i = 0u; i < cl.count; i++) {
        let l = lights[light_index[cl.start + i]];
        let d = length(l.pos.xyz - in.world);
        // Out of its reach, a light adds nothing: skip the rest.
        if (d >= l.pos.w && l.pos.w > 0.0) {
            continue;
        }
        var att = 1.0 / max(d * d, 0.25);
        if (l.pos.w > 0.0) {
            let r = d / l.pos.w;
            att *= pow2(clamp(1.0 - r * r * r * r, 0.0, 1.0));
        }
        light += l.color.rgb * min(att, 1.5) * 0.06;
    }
    let col = in.color.rgb * light * (0.8 + 0.4 * n);
    let rgb = finish(col);
    let fog_f = smoothstep(g.fog.w, g.params.x, in.depth);
    var out: FOut;
    out.color = vec4<f32>(fogged(rgb, fog_f), cover * (1.0 - fog_f * 0.5));
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// ------------------------------------------------------------------ water
//
// A river at night: a dark body under a rippled surface that mirrors the
// starry sky and the moon (so the moon lays a broken path on the water),
// catches glints from lanterns, and grows more mirror-like at grazing
// angles. Upright sheets are cascades: white streaks falling over dark water.

// The surface's height at `q`: ripples carried along by the current (`flow`,
// world units a second), and, where water falls in, a fine fast chop.
fn water_height(q_in: vec2<f32>, time: f32, flow: vec2<f32>, chop: f32) -> f32 {
    let q = q_in - flow * time;
    let t = time * (1.0 + chop * 1.5);
    var h = cnoise(q * 2.3) * 0.55 + cnoise(q * 5.1 + vec2<f32>(t * 1.1, t * 0.4)) * 0.3 + cnoise(q * 11.0 - vec2<f32>(t * 0.3, t * 1.3)) * 0.15;
    if (chop > 0.0) {
        h += (cnoise(q_in * 23.0 + vec2<f32>(t * 2.1, -t * 1.7)) - 0.5) * 0.5 * chop;
    }
    return h;
}

fn water(in: VOut, n_geom: vec3<f32>) -> FOut {
    let time = g.sky_right.w;
    let view_dir = normalize(g.cam_pos.xyz - in.world);
    var out: FOut;
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    let fog_f = fog_at(in.world, in.depth);
    if (abs(n_geom.y) < 0.5) {
        // Falling water: streaks rushing down, whiter where it breaks.
        let across = dot(in.world.xz, vec2<f32>(0.7071, 0.7071)) * 14.0;
        let fall = in.world.y * 5.0 + time * 6.0;
        let streak = cnoise(vec2<f32>(across, fall)) * 0.7 + cnoise(vec2<f32>(across * 2.3, fall * 2.0 + 5.0)) * 0.3;
        let white = smoothstep(0.35, 0.8, streak);
        var col = mix(vec3<f32>(0.02, 0.05, 0.06), in.color.rgb * 0.5, white);
        col *= g.moon_color.rgb * 0.5 + g.hemi_sky.rgb * 0.6 + g.ambient.rgb * 2.0;
        let rgb = finish(col * 2.2);
        // Falls off the island's edge thin away into the clouds below it.
        let sink = smoothstep(-3.2, -0.6, in.world.y);
        out.color = vec4<f32>(fogged(rgb, fog_f), in.color.a * (0.55 + 0.45 * white) * sink);
        return out;
    }
    // Ripples: the normal from the slope of a moving height field.
    let q = in.world.xz;
    let e = 0.04;
    let flow = in.emissive.xy;
    let chop = in.emissive.z;
    let h0 = water_height(q, time, flow, chop);
    let hx = water_height(q + vec2<f32>(e, 0.0), time, flow, chop);
    let hz = water_height(q + vec2<f32>(0.0, e), time, flow, chop);
    let bump = 0.06 + 0.07 * chop;
    let n = normalize(vec3<f32>(-(hx - h0) / e * bump, 1.0, -(hz - h0) / e * bump));
    let r = reflect(-view_dir, n);
    var sky = mix(g.fog.rgb, g.fog.rgb * 1.4 + vec3<f32>(0.05), clamp(r.y, 0.0, 1.0));
    if (g.sky_fwd.w > 0.0) {
        sky = sky_color(normalize(vec3<f32>(r.x, abs(r.y), r.z)), 0.004, time);
        // At dusk (the Cyclops' meadow) the stream lies low between its banks, ringed by the
        // treeline: low down it mirrors the dark woods, not the bright west, and higher up the sky
        // at half strength, so it reads dark and holds the lamps.
        if (g.sky_fwd.w > 1.5) {
            sky *= mix(0.12, 0.5, smoothstep(0.08, 0.5, r.y));
        }
    }
    let fres = 0.03 + 0.97 * pow(1.0 - max(dot(n, view_dir), 0.0), 5.0);
    // The body of the water: dark, lit faintly from above and by lights nearby.
    var body = in.color.rgb * (g.ambient.rgb * 2.0 + g.hemi_sky.rgb * 0.35 + g.moon_color.rgb * 0.15);
    var spec = vec3<f32>(0.0);
    let cl = cluster_at(in.clip.xy, in.world);
    for (var i = 0u; i < cl.count; i++) {
        let l = lights[light_index[cl.start + i]];
        let to_light = l.pos.xyz - in.world;
        let d = length(to_light);
        // Out of its reach, a light adds nothing: skip the rest.
        if (d >= l.pos.w && l.pos.w > 0.0) {
            continue;
        }
        let ld = to_light / max(d, 1e-4);
        var att = 1.0 / max(d * d, 0.05);
        if (l.pos.w > 0.0) {
            let rr = d / l.pos.w;
            att *= pow2(clamp(1.0 - rr * rr * rr * rr, 0.0, 1.0));
        }
        body += in.color.rgb * l.color.rgb * att * 0.25;
        let hv = normalize(ld + view_dir);
        spec += pow(max(dot(n, hv), 0.0), 220.0) * l.color.rgb * att * 3.0;
    }
    // A broad moonlit sheen around the moon's reflection, broken up by the ripples (at dusk the
    // low sun's, kept to a glitter so the stream stays dark).
    let md = normalize(g.sky_moon.xyz);
    let sheen = pow(max(dot(r, md), 0.0), 40.0) * 0.5 + pow(max(dot(r, md), 0.0), 6.0) * 0.08;
    let ripple_light = g.hemi_sky.rgb * (0.05 + 0.25 * clamp(h0 - 0.35, 0.0, 1.0)) + g.moon_color.rgb * sheen * select(g.sky_fwd.w, 0.35, g.sky_fwd.w > 1.5);
    let body_out = finish(body + spec + ripple_light);
    let mirror = clamp(fres * 1.5 + 0.3, 0.0, 1.0);
    let sky_out = from_display(sky);
    var col = mix(body_out, max(sky_out, body_out), mirror) + finish(spec) * 0.5;
    out.color = vec4<f32>(fogged(col, fog_f), mix(in.color.a, 1.0, fres));
    return out;
}

// A jet of water: a glassy rope, clear through the middle and bright at its
// edges, bubbles rushing along it, glinting in the lamps and the moon.
fn water_jet(in: VOut, n: vec3<f32>) -> FOut {
    let time = g.sky_right.w;
    let view_dir = normalize(g.cam_pos.xyz - in.world);
    let edge = pow(1.0 - abs(dot(n, view_dir)), 1.5);
    let rush = cnoise(vec2<f32>(in.local.y * 3.0 - time * 14.0 + in.emissive.x * 7.0, atan2(in.local.z, in.local.x) * 2.0 + in.emissive.x));
    var light = g.ambient.rgb * 2.0 + g.hemi_sky.rgb * 0.6 + g.moon_color.rgb * 0.35;
    var spec = vec3<f32>(0.0);
    let cl = cluster_at(in.clip.xy, in.world);
    for (var i = 0u; i < cl.count; i++) {
        let l = lights[light_index[cl.start + i]];
        let to_light = l.pos.xyz - in.world;
        let d = length(to_light);
        // Out of its reach, a light adds nothing: skip the rest.
        if (d >= l.pos.w && l.pos.w > 0.0) {
            continue;
        }
        var att = 1.0 / max(d * d, 0.05);
        if (l.pos.w > 0.0) {
            let rr = d / l.pos.w;
            att *= pow2(clamp(1.0 - rr * rr * rr * rr, 0.0, 1.0));
        }
        light += l.color.rgb * att * 0.5;
        let hv = normalize(to_light / max(d, 1e-4) + view_dir);
        // (A lamp right beside a jet would blaze it white: its highlight is held to a lamp's at
        // about two thirds of a unit.)
        spec += pow(max(dot(n, hv), 0.0), 60.0) * l.color.rgb * min(att, 2.0);
    }
    spec += pow(max(dot(n, normalize(g.moon_dir.xyz + view_dir)), 0.0), 60.0) * g.moon_color.rgb;
    let col = in.color.rgb * light * (0.55 + 0.5 * rush + 0.6 * edge) + spec * 0.8;
    let rgb = finish(col);
    let fog_f = fog_at(in.world, in.depth);
    var out: FOut;
    out.color = vec4<f32>(fogged(rgb, fog_f), in.color.a * clamp(0.35 + 0.65 * edge + 0.25 * rush, 0.0, 1.0));
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// Jelly (`rig_look::GOO`, the bog slime's skin): see-through, glossy and wet. Its soft normal
// comes in `local`. The light falls on it wrapped and shines through it from behind; the moon,
// the lamps and the sky glint off it sharply; seen edge-on it thickens (less of what is behind
// shows through), and its highlights are not thinned by its clearness.
fn jelly(in: VOut) -> FOut {
    let n = normalize(in.local);
    let view_dir = normalize(g.cam_pos.xyz - in.world);
    let nv = max(dot(n, view_dir), 0.0);
    let edge = pow(1.0 - nv, 3.0);
    let hemi = mix(g.hemi_ground.rgb, g.hemi_sky.rgb, 0.5 * n.y + 0.5);
    var diffuse = g.ambient.rgb + hemi;
    var through = vec3<f32>(0.0);
    var spec = vec3<f32>(0.0);
    let f0 = vec3<f32>(0.04);
    let ml = dot(n, g.moon_dir.xyz);
    diffuse += max((ml + 0.5) / 1.5, 0.0) * g.moon_color.rgb;
    through += max(-ml, 0.0) * g.moon_color.rgb;
    if (ml > 0.0) {
        spec += ggx(n, view_dir, g.moon_dir.xyz, 0.16, f0) * ml * g.moon_color.rgb;
    }
    let cl = cluster_at(in.clip.xy, in.world);
    let count = select(cl.count, 0u, fx_off(FX_LIGHTS));
    for (var i = 0u; i < count; i++) {
        let l = lights[light_index[cl.start + i]];
        let to_light = l.pos.xyz - in.world;
        let d = length(to_light);
        if (d >= l.pos.w && l.pos.w > 0.0) {
            continue;
        }
        let ld = to_light / max(d, 1e-4);
        var att = 1.0 / max(d * d, 0.01);
        if (l.pos.w > 0.0) {
            let r = d / l.pos.w;
            att *= pow2(clamp(1.0 - r * r * r * r, 0.0, 1.0));
        }
        let nl = dot(n, ld);
        diffuse += max((nl + 0.5) / 1.5, 0.0) * l.color.rgb * att;
        through += max(-nl, 0.0) * l.color.rgb * att;
        if (nl > 0.0) {
            spec += ggx(n, view_dir, ld, 0.16, f0) * nl * l.color.rgb * att;
        }
    }
    let refl = reflect(-view_dir, n);
    let env = textureSampleLevel(env_map, env_samp, refl, 0.16 * ENV_MIPS).rgb;
    let shine = spec + env * (0.04 + 0.96 * pow(1.0 - nv, 5.0));
    let albedo = in.color.rgb;
    var rgb = albedo * (diffuse / PI + through * 0.3) + in.emissive.rgb;
    let fog_f = fog_at(in.world, in.depth);
    // (Its shine is added over what shows through, not thinned with it: its share of the cover
    // is raised to carry it.)
    let a = clamp(in.color.a + (1.0 - in.color.a) * edge * 0.55, 0.0, 1.0);
    let glint = min(dot(shine, vec3<f32>(0.2126, 0.7152, 0.0722)), 1.0);
    let cover = max(a, glint);
    rgb = (rgb * a + shine) / max(cover, 1e-3);
    var out: FOut;
    out.color = vec4<f32>(fogged(finish(rgb), fog_f), cover);
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// A soft glowing orb: a disc turned to face the camera, drawn additively,
// brightest in the middle and fading to nothing at its rim (as a lit sphere
// of haze would look).
fn glow_orb(in: VOut) -> FOut {
    let r2 = min(dot(in.local.xy, in.local.xy), 1.0);
    // Added as light, a halo's faint rim shows far more than added as display colour did; a
    // steep falloff keeps it the size it was drawn.
    let k = pow(1.0 - r2, 4.0);
    let fog_f = fog_at(in.world, in.depth);
    var out: FOut;
    out.color = vec4<f32>(finish(in.color.rgb), in.color.a * k * (1.0 - fog_f * 0.8));
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// ------------------------------------------------------------ the BRDF (RP14)

// GGX specular for a light from `l`: the distribution, Smith visibility (Schlick's fit, as UE4
// uses for analytic lights) and Schlick's Fresnel. Times the light's colour and n.l, it is the
// reflected light, in the same units as the diffuse `albedo / PI * light * n.l`.
fn ggx(n: vec3<f32>, v: vec3<f32>, l: vec3<f32>, rough: f32, f0: vec3<f32>) -> vec3<f32> {
    let h = normalize(v + l);
    let nl = max(dot(n, l), 1e-4);
    let nv = max(dot(n, v), 1e-4);
    let nh = max(dot(n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a = max(rough * rough, 0.002);
    let a2 = a * a;
    let d = a2 / (PI * pow2(nh * nh * (a2 - 1.0) + 1.0));
    let k = pow2(rough + 1.0) / 8.0;
    let vis = 0.25 / ((nl * (1.0 - k) + k) * (nv * (1.0 - k) + k));
    let f = f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - vh, 5.0);
    return d * vis * f;
}

@fragment
fn fs_scene(in: VOut) -> FOut {
    // Derivatives first, while control flow is still uniform.
    var n = normalize(cross(dpdx(in.world), dpdy(in.world)));
    if (dot(n, g.cam_pos.xyz - in.world) < 0.0) {
        n = -n;
    }

    let mode = in.emissive.w;
    let local_w = fwidth(in.local);
    if (mode > 4.5 && mode < 5.5) {
        return cloud_sea(in);
    }
    if (mode > 21.5 && mode < 22.5) {
        return glow_orb(in);
    }
    if (mode > 22.5 && mode < 23.5) {
        return jelly(in);
    }
    if (mode > 20.5 && mode < 21.5) {
        return water_jet(in, n);
    }
    if (mode > 19.5) {
        return water(in, n);
    }
    // Screen-door fade: a 4x4 ordered dither keeps fading trees in the opaque pass.
    if (mode > 2.5 && mode < 3.5 && in.color.a < bayer4(in.clip.xy)) {
        discard;
    }
    var albedo = in.color.rgb;
    if (mode > 3.5 && mode < 4.5 && n.y > 0.9 && !fx_off(FX_GRASS)) {
        // Grass: blade tips catch a warmer, yellower light than the shaded roots.
        let gs = grass_shade(in.world, in.depth);
        albedo = albedo * gs.x;
        albedo = mix(albedo, albedo * vec3<f32>(1.12, 1.1, 0.8), gs.z * 0.5);
        albedo = mix(albedo, albedo * vec3<f32>(0.85, 1.05, 0.9), (gs.y - 0.5) * 0.6 * gs.z);
    }
    var surf: Surface;
    surf.albedo = albedo;
    surf.n = n;
    // Plain surfaces (stone, earth, painted props): broad and rough, a dielectric. Grass a little less so.
    surf.rough = select(0.9, 0.8, mode > 3.5 && mode < 4.5);
    surf.metal = 0.0;
    surf.specular = 0.5;
    surf.wrap = 0.0;
    surf.sheen = 0.0;
    let px = max(local_w.x, max(local_w.y, local_w.z));
    if (mode > 5.5) {
        let plain = select(1.0, 0.0, fx_off(FX_MATERIALS));
        let detail = (1.0 - smoothstep(0.1, 0.3, px)) * plain;
        let bevel = (1.0 - smoothstep(0.2, 0.5, px)) * plain;
        let seed = fract(dot(in.color.rgb, vec3<f32>(17.13, 31.71, 11.37)));
        surf = voxel_surface(i32(round(mode - 5.0)), in.local, detail, bevel, albedo, n, seed);
        if (mode > 15.5 && mode < 16.5) {
            surf = gravel(surf, in.world, n, detail);
        }
    }
    // Lava (kind 12): its crust and its glow from the flow's pattern; the instance's own glow is
    // the flow's mean (so the probes see the light it gives).
    var lava = vec3<f32>(1.0, 1.0, 0.0);
    let is_lava = mode > 16.5 && mode < 17.5;
    if (is_lava) {
        lava = lava_at(in.world, n, g.sky_right.w);
        surf.albedo = albedo * lava.x;
        surf.rough = mix(0.7, 0.35, lava.z);
        surf.specular = 0.4;
    }
    // A voxel only a few pixels across would glitter with a sharp highlight, so the smaller it
    // is on screen the rougher it is drawn (filtering the shading by the pixel's footprint).
    surf.rough = max(surf.rough, 0.6 * smoothstep(0.2, 0.6, px));
    let sn = surf.n;
    let view_dir = normalize(g.cam_pos.xyz - in.world);
    let nv = max(dot(sn, view_dir), 1e-3);
    // Reflectance head-on: 4% for a dielectric at the usual specular level; for a metal its colour,
    // raised to at least a worn metal's brightness (bare iron reflects over half the light; tarnished
    // and dulled, about a third). The game's metals were painted as they look worn and lit (dark
    // iron), so their hue is kept and their level set here.
    let metal_f0 = surf.albedo * clamp(0.35 / max(max(surf.albedo.r, surf.albedo.g), max(surf.albedo.b, 1e-3)), 1.0, 30.0);
    let f0 = mix(vec3<f32>(0.08 * surf.specular), metal_f0, surf.metal);
    var rgb = surf.albedo;
    var indirect = vec3<f32>(0.0);
    if (mode < 0.5 || mode > 1.5) {
        // The light from all around (ambient, sky and ground) is indirect; the sun or moon and the
        // lamps are direct. Inside the level it comes from the probes: the sky only as far as
        // this spot sees it, and the light coming back off its surroundings (RP18).
        let probe = probe_at(in.world, sn);
        let hemi = mix(g.hemi_ground.rgb, g.hemi_sky.rgb, 0.5 * sn.y + 0.5);
        var ambient_irr = g.ambient.rgb + hemi;
        if (probe.inside) {
            let q = probe_toward(probe, sn);
            ambient_irr = g.ambient.rgb + hemi * q.a + q.rgb * PI;
        }
        var irr = ambient_irr;
        var spec = vec3<f32>(0.0);
        let wrap = surf.wrap;
        let ndl_raw = dot(sn, g.moon_dir.xyz);
        let ndl = max((ndl_raw + wrap) / (1.0 + wrap), 0.0);
        if (ndl > 0.0) {
            let vis = sun_visibility(in.world, n, in.clip.xy);
            irr += ndl * g.moon_color.rgb * vis;
            if (ndl_raw > 0.0) {
                spec += ggx(sn, view_dir, g.moon_dir.xyz, surf.rough, f0) * ndl_raw * g.moon_color.rgb * vis;
            }
        }
        let cl = cluster_at(in.clip.xy, in.world);
        let count = select(cl.count, 0u, fx_off(FX_LIGHTS));
        for (var i = 0u; i < count; i++) {
            let l = lights[light_index[cl.start + i]];
            let to_light = l.pos.xyz - in.world;
            let d = length(to_light);
            // Out of its reach, a light adds nothing: skip the rest.
            if (d >= l.pos.w && l.pos.w > 0.0) {
                continue;
            }
            let ld = to_light / max(d, 1e-4);
            let nl_raw = dot(sn, ld);
            let nl = max((nl_raw + wrap) / (1.0 + wrap), 0.0);
            var att = 1.0 / max(d * d, 0.01);
            if (l.pos.w > 0.0) {
                let r = d / l.pos.w;
                att *= pow2(clamp(1.0 - r * r * r * r, 0.0, 1.0));
            }
            irr += nl * l.color.rgb * att;
            if (nl_raw > 0.0) {
                spec += ggx(sn, view_dir, ld, surf.rough, f0) * nl_raw * l.color.rgb * att;
            }
        }
        var emissive = in.emissive.rgb;
        if (is_lava) {
            // Ember in the cracks, going to amber and a pale gold where it runs hot.
            let k = max(max(emissive.r, emissive.g), emissive.b);
            let hot_col = vec3<f32>(1.0, 0.42, 0.09) * k;
            emissive = mix(emissive, hot_col, lava.z * lava.z) * lava.y;
        }
        if (mode > 1.5 && mode < 2.5) {
            emissive *= g.params.z;
        }
        // The surroundings reflected: prefiltered by roughness, scaled by the split-sum lookup.
        let lut = textureSampleLevel(brdf_lut, env_samp, vec2<f32>(nv, surf.rough), 0.0).rg;
        let refl = reflect(-view_dir, sn);
        var env = textureSampleLevel(env_map, env_samp, refl, surf.rough * ENV_MIPS).rgb;
        if (probe.inside) {
            // Inside the level the sky shows only where the probes see it, and the room itself
            // (lit walls, ground, lava) fills the rest (RP18).
            let q = probe_toward(probe, refl);
            env = env * q.a + q.rgb;
        } else {
            // Away from any level (the story's sets, the title): a share of the direct light
            // stands in for the lit surroundings the sky cube does not hold.
            env += g.moon_color.rgb * mix(0.3, 0.18, smoothstep(-0.3, 0.5, refl.y)) * surf.metal;
        }
        rgb = surf.albedo * (1.0 - surf.metal) * irr / PI + spec + env * (f0 * lut.x + lut.y) + emissive;
        // Fur and cloth catch a rim of sky.
        let fres = pow(1.0 - nv, 3.0);
        rgb += g.hemi_sky.rgb * surf.albedo * surf.sheen * fres;
        indirect = surf.albedo * (1.0 - surf.metal) * ambient_irr / PI + env * (f0 * lut.x + lut.y)
            + g.hemi_sky.rgb * surf.albedo * surf.sheen * fres;
        if (mode > 5.5) {
            // Characters and statues: a cool rim of moonlight on the edges turned toward it, so they stand out of the dark.
            let rim = pow(1.0 - nv, 4.0);
            rgb += g.moon_color.rgb * rim * (0.08 + 0.22 * max(dot(sn, g.moon_dir.xyz), 0.0)) * (0.5 + 0.5 * surf.albedo);
        }
        if (mode > 15.5 && mode < 16.5) {
            // Lit from within (`mat::GLOW`): its own colour, bright, whatever falls on it.
            rgb = surf.albedo * 2.4 + in.emissive.rgb;
            indirect = vec3<f32>(0.0);
        }
    }
    rgb = finish(rgb);
    let fog_f = select(fog_at(in.world, in.depth), 0.0, fx_off(FX_FOG));
    rgb = fogged(rgb, fog_f);
    if (fx_off(FX_LIGHT_HEAT)) {
        // Blue none, through green, to red at 16 or more lights in the cluster.
        let k = f32(cluster_at(in.clip.xy, in.world).count) / 16.0;
        rgb = mix(rgb * 0.25, vec3<f32>(clamp(k * 2.0 - 1.0, 0.0, 1.0), 1.0 - abs(k * 2.0 - 1.0), clamp(1.0 - k * 2.0, 0.0, 1.0)), 0.6) * g.params.y;
    }
    var out: FOut;
    out.color = vec4<f32>(rgb, in.color.a);
    out.gbuf = vec4<f32>(n, dot(in.world - g.cam_pos.xyz, g.cam_fwd.xyz));
    // (As much of it as the fog leaves.)
    out.indirect = vec4<f32>(finish(indirect) * (1.0 - fog_f), 1.0);
    return out;
}

// Depth only, before the shading: the opaque surfaces settle the depth buffer first, so the
// costly shading below runs once a pixel instead of once for every face drawn over it. Fades
// dither away exactly as they do when shaded; water, clouds and glows wait for the shading pass.
@fragment
fn fs_prepass(in: VOut) -> FOut {
    let mode = in.emissive.w;
    if ((mode > 4.5 && mode < 5.5) || mode > 19.5) {
        discard;
    }
    if (mode > 2.5 && mode < 3.5 && in.color.a < bayer4(in.clip.xy)) {
        discard;
    }
    var out: FOut;
    out.color = vec4<f32>(0.0);
    out.gbuf = vec4<f32>(0.0);
    return out;
}

// ------------------------------------------------------------ environment (RP14)
//
// Each frame (when the light changes) the surroundings are drawn into a small cubemap, face by
// face, then each mip is filtered from the first for a rougher surface (GGX, importance
// sampled), so `fs_scene` reflects them as sharply as its roughness allows.

struct EnvOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    // The cube face, and the roughness this mip is for.
    @location(1) @interpolate(flat) face: u32,
    @location(2) @interpolate(flat) rough: f32,
};

// A full-screen triangle on one face of one mip: the instance is the face plus six times the mip.
@vertex
fn vs_env(@builtin(vertex_index) i: u32, @builtin(instance_index) inst: u32) -> EnvOut {
    let q = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: EnvOut;
    o.clip = vec4<f32>(q * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(q.x * 2.0 - 1.0, (1.0 - q.y) * 2.0 - 1.0);
    o.face = inst % 6u;
    o.rough = f32(inst / 6u) / ENV_MIPS;
    return o;
}

// The direction through `uv` (-1..1, v down) on cube face `face` (+x, -x, +y, -y, +z, -z).
fn cube_dir(face: u32, uv: vec2<f32>) -> vec3<f32> {
    let u = uv.x;
    let v = uv.y;
    switch face {
        case 0u: { return normalize(vec3<f32>(1.0, -v, -u)); }
        case 1u: { return normalize(vec3<f32>(-1.0, -v, u)); }
        case 2u: { return normalize(vec3<f32>(u, 1.0, v)); }
        case 3u: { return normalize(vec3<f32>(u, -1.0, -v)); }
        case 4u: { return normalize(vec3<f32>(u, -v, 1.0)); }
        default: { return normalize(vec3<f32>(-u, -v, -1.0)); }
    }
}

// What surrounds the scene, as light: the ambient light as it falls from each side (sky above,
// ground below), and where there is a sky, the sky itself (the moon, the stars, the dusk sun).
@fragment
fn fs_env(in: EnvOut) -> @location(0) vec4<f32> {
    let d = cube_dir(in.face, in.uv);
    // (The hemisphere's values are the light falling from each side; seen along one direction, a
    // uniform sky that sends that much has a radiance of that over pi.)
    var c = mix(g.hemi_ground.rgb, g.hemi_sky.rgb, smoothstep(-0.35, 0.35, d.y)) / PI;
    if (g.sky_fwd.w > 0.0 && d.y > -0.05) {
        // The sky is drawn as display colour; as light, before the exposure the scene applies.
        c = max(c, from_display(sky_color(d, 0.02, g.sky_right.w)) / max(g.params.y, 1e-3));
    }
    return vec4<f32>(c, 1.0);
}

// ------------------------------------------------------------------ night sky
//
// A black sky over a faint glow at the horizon (the fog color, so distant
// trees melt into it), three layers of twinkling stars, a thin milky band and
// a full moon with its maria and a soft halo. All procedural, per pixel.

struct SkyOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_sky(@builtin(vertex_index) i: u32) -> SkyOut {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u)) * 2.0 - 1.0;
    var out: SkyOut;
    out.clip = vec4<f32>(p, 1.0, 1.0);
    out.ndc = p;
    return out;
}

fn hash31(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(127.1, 311.7, 74.7))) * 43758.5453);
}

fn vnoise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = p - i;
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash31(vec3<f32>(i, 1.0));
    let b = hash31(vec3<f32>(i + vec2<f32>(1.0, 0.0), 1.0));
    let c = hash31(vec3<f32>(i + vec2<f32>(0.0, 1.0), 1.0));
    let d = hash31(vec3<f32>(i + vec2<f32>(1.0, 1.0), 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn star_layer(ray: vec3<f32>, scale: f32, density: f32, px: f32, time: f32) -> vec3<f32> {
    let p = ray * scale;
    let cell = floor(p);
    let h = hash31(cell);
    if (h < 1.0 - density) {
        return vec3<f32>(0.0);
    }
    let h2 = hash31(cell + 17.3);
    let h3 = hash31(cell + 41.9);
    let center = cell + 0.5 + (vec3<f32>(h2, h3, fract(h2 * 9.1 + h3)) - 0.5) * 0.45;
    let d = length(p - center) / (px * scale);
    let size = 0.9 + 1.4 * pow(h2, 3.0);
    let core = exp(-d * d / (size * size));
    let twinkle = 0.65 + 0.35 * sin(time * (1.5 + 3.0 * h3) + h2 * 40.0);
    let tint = mix(vec3<f32>(0.75, 0.85, 1.0), vec3<f32>(1.0, 0.9, 0.75), h3);
    return tint * core * twinkle * (0.35 + 0.65 * pow(h2, 2.0));
}

@fragment
fn fs_sky(in: SkyOut) -> FOut {
    let ray = normalize(g.sky_fwd.xyz + in.ndc.x * g.sky_right.xyz + in.ndc.y * g.sky_up.xyz);
    let time = g.sky_right.w;
    // Radians per pixel, to keep stars and the moon's rim crisp at any resolution.
    let px = length(fwidth(ray));
    var col = sky_color(ray, px, time);
    // Dither so the dark halo and horizon glow do not band in 8 bits.
    col += (hash31(vec3<f32>(in.clip.xy, fract(time) * 7.0)) - 0.5) / 255.0;
    var out: FOut;
    out.color = vec4<f32>(from_display(col), 1.0);
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// Dusk (sRGB): flame at the horizon through rose and violet to deep blue, a low
// sun with a wide glow, streaks of cloud lit gold on the sun's side, first stars.
fn dusk_sky(ray: vec3<f32>, px: f32, time: f32) -> vec3<f32> {
    let up = ray.y;
    let u = max(up, 0.0);
    let sun = normalize(g.sky_moon.xyz);
    // The art direction's dusk: amber at the horizon, into copper, mist-teal grey, a charcoal
    // zenith (it was pink, mauve and navy).
    var col = mix(vec3<f32>(1.0, 0.62, 0.32), vec3<f32>(0.74, 0.42, 0.28), smoothstep(0.0, 0.08, u));
    col = mix(col, vec3<f32>(0.3, 0.35, 0.36), smoothstep(0.08, 0.32, u));
    col = mix(col, vec3<f32>(0.05, 0.08, 0.09), smoothstep(0.32, 0.85, u));
    let flat_ray = normalize(vec3<f32>(ray.x, 0.0, ray.z) + vec3<f32>(1e-4, 0.0, 0.0));
    let toward = max(dot(flat_ray, normalize(vec3<f32>(sun.x, 0.0, sun.z))), 0.0);
    col = mix(col, col * vec3<f32>(1.15, 0.92, 0.75) + vec3<f32>(0.12, 0.05, 0.0), toward * toward * (1.0 - smoothstep(0.0, 0.45, u)));
    if (up < 0.0) {
        col = mix(g.fog.rgb, col, smoothstep(-0.25, 0.0, up));
    }
    // The sun: a disc and its glow.
    let ang = acos(clamp(dot(ray, sun), -1.0, 1.0));
    col += vec3<f32>(1.0, 0.55, 0.25) * (exp(-ang * 5.0) * 0.3 + exp(-ang * 28.0) * 0.5);
    col = mix(col, vec3<f32>(1.0, 0.93, 0.72), 1.0 - smoothstep(0.034 - px, 0.034 + px, ang));
    // Streaky clouds on a high sheet, gold toward the sun, mauve away from it.
    if (up > 0.005) {
        let q = ray.xz / (up + 0.06) * 0.55;
        let n = cfbm(vec2<f32>(q.x * 0.45 + time * 0.012, q.y * 1.7 + time * 0.004));
        let cover = smoothstep(0.46, 0.76, n) * smoothstep(0.005, 0.1, up) * (1.0 - smoothstep(0.45, 0.85, up));
        let sunward = pow(max(dot(normalize(ray), sun), 0.0), 3.0);
        let lit = mix(vec3<f32>(0.34, 0.3, 0.29), vec3<f32>(1.0, 0.64, 0.4), clamp(sunward * 1.5 + toward * 0.35, 0.0, 1.0));
        col = mix(col, lit * (0.75 + 0.45 * n), cover * 0.85);
    }
    col += star_layer(ray, 90.0, 0.05, px, time) * smoothstep(0.35, 0.8, up) * 0.8;
    return col;
}

// The night sky along `ray` (sRGB): horizon glow, milky band, stars, the moon. `px` is radians per pixel.
fn sky_color(ray: vec3<f32>, px: f32, time: f32) -> vec3<f32> {
    if (g.sky_fwd.w > 1.5) {
        return dusk_sky(ray, px, time);
    }
    let up = ray.y;

    // A teal-charcoal night (the art direction's), the mist's teal glowing low along the horizon.
    let zenith = vec3<f32>(0.005, 0.011, 0.013);
    var col = mix(g.fog.rgb * 0.6, zenith, smoothstep(-0.2, 0.18, up));
    col += vec3<f32>(0.045, 0.08, 0.075) * exp(-max(up, 0.0) / 0.09) * smoothstep(-0.06, 0.01, up);

    // A faint milky band across the sky.
    let band_n = normalize(vec3<f32>(0.35, 0.3, 0.88));
    let band = exp(-pow(dot(ray, band_n) / 0.2, 2.0));
    let cloud = vnoise2(vec2<f32>(atan2(ray.z, ray.x) * 6.0, ray.y * 9.0)) * vnoise2(vec2<f32>(atan2(ray.z, ray.x) * 17.0, ray.y * 23.0));
    let above = smoothstep(-0.02, 0.2, up);
    col += vec3<f32>(0.042, 0.058, 0.058) * band * (0.4 + 1.2 * cloud) * above;
    col += vec3<f32>(0.03, 0.02, 0.008) * band * band * cloud * above;

    // Fewer stars than there were (they read as snow), thickest in the band.
    var stars = star_layer(ray, 90.0, 0.05, px, time);
    stars += star_layer(ray, 170.0, 0.025 + 0.07 * band, px, time) * 0.75;
    stars += star_layer(ray, 320.0, 0.012 + 0.08 * band, px, time) * 0.55;
    col += stars * above;

    // The full moon.
    let md = normalize(g.sky_moon.xyz);
    let mr = normalize(cross(md, vec3<f32>(0.0, 1.0, 0.0)));
    let mu = cross(mr, md);
    let radius = 0.042;
    let ang = acos(clamp(dot(ray, md), -1.0, 1.0));
    let halo = exp(-max(ang - radius, 0.0) * 22.0) * 0.12 + exp(-max(ang - radius, 0.0) * 4.0) * 0.06;
    col += vec3<f32>(0.52, 0.68, 0.66) * halo;
    if (ang < radius + px * 2.0) {
        let uv = vec2<f32>(dot(ray, mr), dot(ray, mu)) / radius;
        let r2 = dot(uv, uv);
        let z = sqrt(max(1.0 - r2, 0.0));
        var surf = 0.55 * vnoise2(uv * 2.2 + 3.0) + 0.3 * vnoise2(uv * 5.0 + 11.0) + 0.15 * vnoise2(uv * 11.0);
        let maria = smoothstep(0.48, 0.7, surf);
        var moon = mix(vec3<f32>(0.97, 0.96, 0.91), vec3<f32>(0.7, 0.72, 0.74), maria);
        // A few craters with bright rims.
        for (var k = 0; k < 7; k++) {
            let fk = f32(k);
            let c = vec2<f32>(hash31(vec3<f32>(fk, 3.0, 1.0)), hash31(vec3<f32>(fk, 7.0, 2.0))) * 1.4 - 0.7;
            let cr = 0.08 + 0.12 * hash31(vec3<f32>(fk, 5.0, 9.0));
            let dd = length(uv - c) / cr;
            moon *= 1.0 - 0.07 * smoothstep(1.0, 0.5, dd) + 0.04 * smoothstep(0.75, 1.0, dd) * smoothstep(1.2, 1.0, dd);
        }
        moon *= 0.72 + 0.28 * pow(z, 0.5);
        let edge = 1.0 - smoothstep(radius - px * 1.5, radius + px * 0.5, ang);
        col = mix(col, moon, edge);
    }
    return col;
}

// ------------------------------------------------------------------ overlay

struct Screen {
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;
// The type's signed distance field (`render::text`): 0.5 on the outline, more inside.
@group(0) @binding(17) var sdf_atlas: texture_2d<f32>;

struct OIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) extra: vec4<f32>,
};

struct OOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) extra: vec4<f32>,
};

@vertex
fn vs_overlay(v: OIn) -> OOut {
    var out: OOut;
    out.clip = vec4<f32>(v.pos.x / screen.size.x * 2.0 - 1.0, 1.0 - v.pos.y / screen.size.y * 2.0, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    out.extra = v.extra;
    return out;
}

// How a vertex's texel is read (`overlay::FLAG_*`).
const O_SDF: u32 = 1u;
const O_CIRCLE: u32 = 2u;
const O_PREMULTIPLIED: u32 = 4u;
// A pixel's four samples for text, in pixels from its centre.
const O_GRID = array<vec2<f32>, 4>(vec2<f32>(-0.125, -0.375), vec2<f32>(0.375, -0.125), vec2<f32>(0.125, 0.375), vec2<f32>(-0.375, 0.125));

@fragment
fn fs_overlay(in: OOut) -> @location(0) vec4<f32> {
    // Both textures read, and their derivatives taken, while control flow is still uniform.
    let t = textureSample(atlas, atlas_sampler, in.uv);
    let d = textureSample(sdf_atlas, atlas_sampler, in.uv).r;
    let fw = max(fwidth(d), 1e-4);
    let du = dpdx(in.uv);
    let dv = dpdy(in.uv);
    let r = length(in.extra.yz);
    let fr = max(fwidth(r), 1e-4);
    let flags = u32(in.extra.x + 0.5);
    if ((flags & O_SDF) != 0u) {
        // The outline's edge moved out by `extra.y` (a heavier weight, an outline, a glow), softened
        // over `extra.z` more. Four samples a pixel on a rotated grid, each anti-aliased over its
        // quarter, so small text keeps its shape where one sample would skip texels.
        let edge = 0.5 - in.extra.y;
        let w = fw * 0.5;
        var a = 0.0;
        for (var k = 0; k < 4; k++) {
            let o = O_GRID[k];
            let s = textureSampleLevel(sdf_atlas, atlas_sampler, in.uv + du * o.x + dv * o.y, 0.0).r;
            a += smoothstep(edge - in.extra.z - w, edge + w, s);
        }
        return vec4<f32>(in.color.rgb, in.color.a * a * 0.25);
    }
    // Glyphs of the pixel font are white with their coverage as alpha; the pictures carry their own colour.
    var rgb = t.rgb;
    var a = t.a;
    if ((flags & O_PREMULTIPLIED) != 0u) {
        rgb = t.rgb / max(t.a, 1e-3);
    }
    if ((flags & O_CIRCLE) != 0u) {
        a = a * (1.0 - smoothstep(1.0 - fr, 1.0, r));
    }
    return vec4<f32>(in.color.rgb * rgb, in.color.a * a);
}
