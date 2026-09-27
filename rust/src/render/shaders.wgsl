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
    sky_fwd: vec4<f32>,    // xyz forward view ray, w night sky amount
    sky_right: vec4<f32>,  // xyz to the right screen edge, w time
    sky_up: vec4<f32>,     // xyz to the top screen edge
    sky_moon: vec4<f32>,   // xyz toward the moon
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
fn fs_sky(in: SkyOut) -> @location(0) vec4<f32> {
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
    return vec4<f32>(col, 1.0);
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
