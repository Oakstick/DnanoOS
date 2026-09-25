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
    cam_pos: vec4<f32>,
    fog: vec4<f32>,        // rgb fog color (sRGB), w near
    params: vec4<f32>,     // x fog far, y exposure, z nugget pulse, w light count
    hemi_sky: vec4<f32>,
    hemi_ground: vec4<f32>,
    ambient: vec4<f32>,
    moon_dir: vec4<f32>,
    moon_color: vec4<f32>,
    lights: array<Light, 32>,
};

@group(0) @binding(0) var<uniform> g: Globals;

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
    return out;
}

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

@fragment
fn fs_scene(in: VOut) -> @location(0) vec4<f32> {
    // Derivatives first, while control flow is still uniform.
    var n = normalize(cross(dpdx(in.world), dpdy(in.world)));
    if (dot(n, g.cam_pos.xyz - in.world) < 0.0) {
        n = -n;
    }

    let mode = in.emissive.w;
    // Screen-door fade: a 4x4 ordered dither keeps fading trees in the opaque pass.
    if (mode > 2.5 && mode < 3.5 && in.color.a < bayer4(in.clip.xy)) {
        discard;
    }
    var albedo = in.color.rgb;
    if (mode > 3.5 && n.y > 0.9) {
        // Grass: blade tips catch a warmer, yellower light than the shaded roots.
        let gs = grass_shade(in.world, in.depth);
        albedo = albedo * gs.x;
        albedo = mix(albedo, albedo * vec3<f32>(1.12, 1.1, 0.8), gs.z * 0.5);
        albedo = mix(albedo, albedo * vec3<f32>(0.85, 1.05, 0.9), (gs.y - 0.5) * 0.6 * gs.z);
    }
    var rgb = albedo;
    if (mode < 0.5 || mode > 1.5) {
        var irr = g.ambient.rgb;
        irr += mix(g.hemi_ground.rgb, g.hemi_sky.rgb, 0.5 * n.y + 0.5);
        irr += max(dot(n, g.moon_dir.xyz), 0.0) * g.moon_color.rgb;
        let count = min(u32(g.params.w), MAX_LIGHTS);
        for (var i = 0u; i < count; i++) {
            let l = g.lights[i];
            let to_light = l.pos.xyz - in.world;
            let d = length(to_light);
            let nl = max(dot(n, to_light / max(d, 1e-4)), 0.0);
            var att = 1.0 / max(d * d, 0.01);
            if (l.pos.w > 0.0) {
                let r = d / l.pos.w;
                att *= pow2(clamp(1.0 - r * r * r * r, 0.0, 1.0));
            }
            irr += nl * l.color.rgb * att;
        }
        var emissive = in.emissive.rgb;
        if (mode > 1.5 && mode < 2.5) {
            emissive *= g.params.z;
        }
        rgb = albedo * irr / PI + emissive;
    }
    rgb = to_srgb(aces_filmic(rgb, g.params.y));
    let fog_f = smoothstep(g.fog.w, g.params.x, in.depth);
    rgb = mix(rgb, g.fog.rgb, fog_f);
    return vec4<f32>(rgb, in.color.a);
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
