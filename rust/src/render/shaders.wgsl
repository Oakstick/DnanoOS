// Voxel scene shading, matched to the Three.js build: Lambert diffuse under a
// hemisphere, ambient, one moonlight and a pool of point lights with Three's
// distance falloff; flat normals from screen derivatives (Three's flatShading);
// ACES filmic tone mapping; sRGB encode; then linear fog mixed in sRGB.

const PI: f32 = 3.141592653589793;
const MAX_LIGHTS: u32 = 32u;

struct Light {
    pos: vec4<f32>,   // xyz position, w cutoff distance
    color: vec4<f32>, // linear color times intensity
};

struct Globals {
    view_proj: mat4x4<f32>,
    light_vp: mat4x4<f32>, // world to the moon's (or sun's) shadow map
    cam_pos: vec4<f32>,
    cam_fwd: vec4<f32>,    // xyz camera forward; w shadow map texel size in uv
    fog: vec4<f32>,        // rgb fog color (sRGB), w near
    params: vec4<f32>,     // x fog far, y exposure, z nugget pulse, w light count
    hemi_sky: vec4<f32>,
    hemi_ground: vec4<f32>,
    ambient: vec4<f32>,
    moon_dir: vec4<f32>,
    moon_color: vec4<f32>,
    sky_fwd: vec4<f32>,    // xyz forward view ray, w night sky amount
    sky_right: vec4<f32>,  // xyz to the right screen edge, w time
    sky_up: vec4<f32>,     // xyz to the top screen edge
    sky_moon: vec4<f32>,   // xyz toward the moon
    bounds: vec4<f32>,     // the level's footprint: min x, min z, max x, max z
    lights: array<Light, 32>,
};

@group(0) @binding(0) var<uniform> g: Globals;
@group(0) @binding(1) var shadow_map: texture_depth_2d;
@group(0) @binding(2) var shadow_cmp: sampler_comparison;

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
    @builtin(position) clip: vec4<f32>,
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

// Depth only, from the light: the shadow map.
@vertex
fn vs_shadow(v: VIn) -> @builtin(position) vec4<f32> {
    let m = mat4x4<f32>(v.m0, v.m1, v.m2, v.m3);
    return g.light_vp * (m * vec4<f32>(v.pos, 1.0));
}

// How much of the moonlight (or sunlight) reaches `world`: a 4x4 filtered
// lookup for soft shadow edges.
fn sun_visibility(world: vec3<f32>, n: vec3<f32>) -> f32 {
    let texel = g.cam_fwd.w;
    let lp = g.light_vp * vec4<f32>(world + n * 0.025, 1.0);
    let c = lp.xyz / lp.w;
    let uv = vec2<f32>(c.x * 0.5 + 0.5, 0.5 - c.y * 0.5);
    if (any(uv < vec2<f32>(0.0)) || any(uv > vec2<f32>(1.0)) || c.z > 1.0) {
        return 1.0;
    }
    var lit = 0.0;
    for (var y = 0; y < 4; y++) {
        for (var x = 0; x < 4; x++) {
            let o = (vec2<f32>(f32(x), f32(y)) - 1.5) * texel * 1.35;
            lit += textureSampleCompareLevel(shadow_map, shadow_cmp, uv + o, c.z - 0.0008);
        }
    }
    return lit / 16.0;
}

// ------------------------------------------------------------ voxel materials
//
// Character voxels carry a material (shade mode 6 and up). Each face of each
// voxel gets procedural detail in its own little square, so it sticks to the
// model as it moves: woven threads, fur strands, hammered dents, wood grain,
// and a bevel that darkens every voxel's edges. Detail fades out when a voxel
// covers only a few pixels, so it never shimmers.

struct Surface {
    albedo: vec3<f32>,
    n: vec3<f32>,
    spec: f32,     // specular strength
    shine: f32,    // specular exponent
    metal: f32,    // 1 tints highlights with the albedo
    wrap: f32,     // light wrapping past the terminator (soft materials)
    sheen: f32,    // rim glow at grazing angles (fur, cloth)
};

fn hash12(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(12.9898, 78.233))) * 43758.5453);
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

fn voxel_surface(kind: i32, local: vec3<f32>, detail: f32, bevel_detail: f32, base: vec3<f32>, n: vec3<f32>, seed: f32) -> Surface {
    var m: Surface;
    m.albedo = base;
    m.n = n;
    m.spec = 0.0;
    m.shine = 16.0;
    m.metal = 0.0;
    m.wrap = 0.0;
    m.sheen = 0.0;
    let uv = voxel_uv(local);
    let e = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    m.albedo *= mix(1.0, mix(0.9, 1.0, smoothstep(0.0, 0.11, e)), bevel_detail);
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
            m.spec = 1.0;
            m.shine = 44.0;
            m.metal = 0.85;
        }
        case 2: {
            // Fur and hair: strands of varied shade, soft light, a sheen at the rim.
            let col = floor(uv.x * 5.0);
            let s = hash12(vec2<f32>(col, seed * 31.0));
            let strand = 0.7 + 0.55 * s * (0.65 + 0.35 * sin(uv.y * 8.0 + s * 6.0));
            m.albedo *= mix(1.0, strand, detail);
            m.wrap = 0.35;
            m.sheen = 0.3;
            m.spec = 0.08;
            m.shine = 10.0;
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
        }
        case 4: {
            // Leather: a soft mottle and a dull shine.
            let g = hash12(floor(uv * 5.0) + seed * 13.0);
            m.albedo *= mix(1.0, 0.86 + 0.24 * g, detail);
            m.spec = 0.25;
            m.shine = 18.0;
        }
        case 5: {
            // Skin: light wraps around, a faint oily shine.
            m.wrap = 0.45;
            m.spec = 0.1;
            m.shine = 14.0;
        }
        case 6: {
            // Ivory horn: growth rings and a polished gloss.
            m.albedo *= mix(1.0, 0.93 + 0.08 * sin(uv.y * 19.0 + seed * 5.0), detail);
            m.spec = 0.45;
            m.shine = 36.0;
        }
        case 7: {
            // Wood: long grain.
            let grain = abs(sin(uv.x * 13.0 + sin(uv.y * 4.0 + seed * 9.0) * 1.8));
            m.albedo *= mix(1.0, 0.8 + 0.26 * grain, detail);
            m.spec = 0.08;
        }
        case 8: {
            // Gold and bronze: bright, warm, polished.
            m.spec = 1.3;
            m.shine = 60.0;
            m.metal = 1.0;
            m.albedo *= 0.85;
        }
        default: {}
    }
    return m;
}

// The scene pass writes color and, for post-processing, the world normal and view depth.
struct FOut {
    @location(0) color: vec4<f32>,
    @location(1) gbuf: vec4<f32>,
};

fn rrt_odt_fit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
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
    return clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
}

fn to_srgb(c: vec3<f32>) -> vec3<f32> {
    let hi = pow(c, vec3<f32>(0.41666)) * 1.055 - vec3<f32>(0.055);
    let lo = c * 12.92;
    return select(hi, lo, c <= vec3<f32>(0.0031308));
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

fn cloud_sea(in: VOut) -> FOut {
    let seed = in.emissive.x;
    let layer_y = in.emissive.y;
    let time = g.sky_right.w;
    let xz = in.world.xz;
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
    let rgb = to_srgb(aces_filmic(col, g.params.y));
    let fog_f = smoothstep(g.fog.w, g.params.x, in.depth);
    var out: FOut;
    out.color = vec4<f32>(mix(rgb, g.fog.rgb, fog_f * 0.6), cover * in.color.a);
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
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
    // Screen-door fade: a 4x4 ordered dither keeps fading trees in the opaque pass.
    if (mode > 2.5 && mode < 3.5 && in.color.a < bayer4(in.clip.xy)) {
        discard;
    }
    var albedo = in.color.rgb;
    if (mode > 3.5 && mode < 4.5 && n.y > 0.9) {
        // Grass: blade tips catch a warmer, yellower light than the shaded roots.
        let gs = grass_shade(in.world, in.depth);
        albedo = albedo * gs.x;
        albedo = mix(albedo, albedo * vec3<f32>(1.12, 1.1, 0.8), gs.z * 0.5);
        albedo = mix(albedo, albedo * vec3<f32>(0.85, 1.05, 0.9), (gs.y - 0.5) * 0.6 * gs.z);
    }
    var surf: Surface;
    surf.albedo = albedo;
    surf.n = n;
    surf.spec = 0.0;
    surf.shine = 16.0;
    surf.metal = 0.0;
    surf.wrap = 0.0;
    surf.sheen = 0.0;
    if (mode > 5.5) {
        let px = max(local_w.x, max(local_w.y, local_w.z));
        let detail = 1.0 - smoothstep(0.1, 0.3, px);
        let bevel = 1.0 - smoothstep(0.2, 0.5, px);
        let seed = fract(dot(in.color.rgb, vec3<f32>(17.13, 31.71, 11.37)));
        surf = voxel_surface(i32(round(mode - 5.0)), in.local, detail, bevel, albedo, n, seed);
    }
    let sn = surf.n;
    let view_dir = normalize(g.cam_pos.xyz - in.world);
    let spec_tint = mix(vec3<f32>(1.0), surf.albedo * 1.6, surf.metal);
    let spec_norm = (surf.shine + 8.0) / (8.0 * PI);
    var rgb = surf.albedo;
    if (mode < 0.5 || mode > 1.5) {
        var irr = g.ambient.rgb;
        irr += mix(g.hemi_ground.rgb, g.hemi_sky.rgb, 0.5 * sn.y + 0.5);
        var spec = vec3<f32>(0.0);
        let wrap = surf.wrap;
        let ndl_raw = dot(sn, g.moon_dir.xyz);
        let ndl = max((ndl_raw + wrap) / (1.0 + wrap), 0.0);
        if (ndl > 0.0) {
            let vis = sun_visibility(in.world, n);
            irr += ndl * g.moon_color.rgb * vis;
            if (surf.spec > 0.0 && ndl_raw > 0.0) {
                let h = normalize(g.moon_dir.xyz + view_dir);
                spec += pow(max(dot(sn, h), 0.0), surf.shine) * ndl_raw * g.moon_color.rgb * vis;
            }
        }
        let count = min(u32(g.params.w), MAX_LIGHTS);
        for (var i = 0u; i < count; i++) {
            let l = g.lights[i];
            let to_light = l.pos.xyz - in.world;
            let d = length(to_light);
            let ld = to_light / max(d, 1e-4);
            let nl_raw = dot(sn, ld);
            let nl = max((nl_raw + wrap) / (1.0 + wrap), 0.0);
            var att = 1.0 / max(d * d, 0.01);
            if (l.pos.w > 0.0) {
                let r = d / l.pos.w;
                att *= pow2(clamp(1.0 - r * r * r * r, 0.0, 1.0));
            }
            irr += nl * l.color.rgb * att;
            if (surf.spec > 0.0 && nl_raw > 0.0) {
                let h = normalize(ld + view_dir);
                spec += pow(max(dot(sn, h), 0.0), surf.shine) * nl_raw * l.color.rgb * att;
            }
        }
        var emissive = in.emissive.rgb;
        if (mode > 1.5 && mode < 2.5) {
            emissive *= g.params.z;
        }
        rgb = surf.albedo * irr / PI + spec * spec_tint * surf.spec * spec_norm + emissive;
        // Metals mirror the sky above and the ground below; fur and cloth catch a rim of sky.
        let fres = pow(1.0 - max(dot(sn, view_dir), 0.0), 3.0);
        let r = reflect(-view_dir, sn);
        let env = mix(g.hemi_ground.rgb * 0.6, g.hemi_sky.rgb * 1.3, smoothstep(-0.35, 0.35, r.y));
        rgb += env * spec_tint * surf.metal * (0.3 + 0.5 * fres);
        rgb += g.hemi_sky.rgb * surf.albedo * surf.sheen * fres;
    }
    rgb = to_srgb(aces_filmic(rgb, g.params.y));
    let fog_f = smoothstep(g.fog.w, g.params.x, in.depth);
    rgb = mix(rgb, g.fog.rgb, fog_f);
    var out: FOut;
    out.color = vec4<f32>(rgb, in.color.a);
    out.gbuf = vec4<f32>(n, dot(in.world - g.cam_pos.xyz, g.cam_fwd.xyz));
    return out;
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
    let up = ray.y;

    let zenith = vec3<f32>(0.004, 0.006, 0.014);
    var col = mix(g.fog.rgb * 0.6, zenith, smoothstep(-0.2, 0.18, up));

    // A faint milky band across the sky.
    let band_n = normalize(vec3<f32>(0.35, 0.3, 0.88));
    let band = exp(-pow(dot(ray, band_n) / 0.2, 2.0));
    let cloud = vnoise2(vec2<f32>(atan2(ray.z, ray.x) * 6.0, ray.y * 9.0)) * vnoise2(vec2<f32>(atan2(ray.z, ray.x) * 17.0, ray.y * 23.0));
    let above = smoothstep(-0.02, 0.2, up);
    col += vec3<f32>(0.05, 0.055, 0.075) * band * (0.4 + 1.2 * cloud) * above;

    var stars = star_layer(ray, 90.0, 0.07, px, time);
    stars += star_layer(ray, 170.0, 0.05 + 0.08 * band, px, time) * 0.8;
    stars += star_layer(ray, 320.0, 0.03 + 0.1 * band, px, time) * 0.6;
    col += stars * above;

    // The full moon.
    let md = normalize(g.sky_moon.xyz);
    let mr = normalize(cross(md, vec3<f32>(0.0, 1.0, 0.0)));
    let mu = cross(mr, md);
    let radius = 0.042;
    let ang = acos(clamp(dot(ray, md), -1.0, 1.0));
    let halo = exp(-max(ang - radius, 0.0) * 22.0) * 0.12 + exp(-max(ang - radius, 0.0) * 5.0) * 0.04;
    col += vec3<f32>(0.55, 0.6, 0.75) * halo;
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
    // Dither so the dark halo and horizon glow do not band in 8 bits.
    col += (hash31(vec3<f32>(in.clip.xy, fract(time) * 7.0)) - 0.5) / 255.0;
    var out: FOut;
    out.color = vec4<f32>(col, 1.0);
    out.gbuf = vec4<f32>(0.0, 1.0, 0.0, 10000.0);
    return out;
}

// ------------------------------------------------------------------ overlay

struct Screen {
    size: vec4<f32>,
};

@group(0) @binding(0) var<uniform> screen: Screen;
@group(0) @binding(1) var atlas: texture_2d<f32>;
@group(0) @binding(2) var atlas_sampler: sampler;

struct OIn {
    @location(0) pos: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct OOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_overlay(v: OIn) -> OOut {
    var out: OOut;
    out.clip = vec4<f32>(v.pos.x / screen.size.x * 2.0 - 1.0, 1.0 - v.pos.y / screen.size.y * 2.0, 0.0, 1.0);
    out.uv = v.uv;
    out.color = v.color;
    return out;
}

@fragment
fn fs_overlay(in: OOut) -> @location(0) vec4<f32> {
    let a = textureSample(atlas, atlas_sampler, in.uv).r;
    return vec4<f32>(in.color.rgb, in.color.a * a);
}
