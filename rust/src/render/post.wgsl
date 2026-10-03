// Post-processing, after the scene pass has written its color and a buffer of
// world normals and view depth (the "gbuf"):
//
//   ssao       contact shading: how much nearby geometry hides each pixel's sky
//   blur       a depth-aware blur of that, at half resolution
//   bloom_*    glow: bright parts, shrunk down a chain of textures and grown back
//   composite  color x contact shading, background blur, glow, grading, vignette
//
// The scene color arrives as light (linear HDR, times the exposure): glow is taken from it,
// and the composite tone maps and encodes it, once.

struct Post {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,   // w: 1 for an orthographic camera
    cam_fwd: vec4<f32>,   // w: depth of the in-focus plane
    cam_r: vec4<f32>,     // right, scaled to the screen edge; w: blur starts this far behind focus
    cam_u: vec4<f32>,     // up, scaled to the screen edge; w: blur is full this far behind focus
    params: vec4<f32>,    // contact radius, contact strength, glow strength, blur max px at 1080p (0 = off)
    params2: vec4<f32>,   // glow threshold, vignette, time, grading
    moon: vec4<f32>,      // the moon's place on screen (uv), how strongly it shines through
    haze: vec4<f32>,      // ground haze (RP19): density at the floor, floor height, falloff, the fog's own scattering
    vol: vec4<f32>,       // lamps that light the air: how many (the first in priority order), exposure, strength
};

// The scene's lights (RP17's list, in priority order: the hero's lantern first, then nearest the eye).
struct Light {
    pos: vec4<f32>,   // xyz position, w cutoff distance
    color: vec4<f32>,
};
@group(0) @binding(7) var<storage, read> lights: array<Light>;

@group(0) @binding(0) var<uniform> p: Post;
@group(0) @binding(1) var t0: texture_2d<f32>;
@group(0) @binding(2) var t1: texture_2d<f32>;
@group(0) @binding(3) var t2: texture_2d<f32>;
@group(0) @binding(4) var t3: texture_2d<f32>;
@group(0) @binding(5) var samp: sampler;
// The composite's fifth texture: the scene's indirect light, which contact shading darkens (RP15).
@group(0) @binding(6) var t4: texture_2d<f32>;

const SKY: f32 = 5000.0;

struct POut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_post(@builtin(vertex_index) i: u32) -> POut {
    let q = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: POut;
    o.clip = vec4<f32>(q * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(q.x, 1.0 - q.y);
    return o;
}

fn world_at(uv: vec2<f32>, d: f32) -> vec3<f32> {
    let ndc = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0);
    let side = ndc.x * p.cam_r.xyz + ndc.y * p.cam_u.xyz;
    if (p.cam_pos.w > 0.5) {
        return p.cam_pos.xyz + side + p.cam_fwd.xyz * d;
    }
    return p.cam_pos.xyz + (p.cam_fwd.xyz + side) * d;
}

// ------------------------------------------------------------ contact shading

@fragment
fn fs_ssao(in: POut) -> @location(0) vec4<f32> {
    let full = vec2<f32>(textureDimensions(t0));
    let gb = textureLoad(t0, vec2<i32>(in.uv * full), 0);
    let d = gb.w;
    if (d > SKY) {
        return vec4<f32>(1.0);
    }
    let n = normalize(gb.xyz + vec3<f32>(0.0, 1e-4, 0.0));
    let pos = world_at(in.uv, d) + n * 0.01;
    let radius = p.params.x;
    let noise = fract(52.9829189 * fract(dot(in.clip.xy, vec2<f32>(0.06711056, 0.00583715))));
    let spin = noise * 6.2831853;
    let helper = select(vec3<f32>(0.0, 1.0, 0.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.y) > 0.9);
    let t = normalize(cross(helper, n));
    let b = cross(n, t);
    var occ = 0.0;
    let count = 12;
    for (var i = 0; i < count; i++) {
        let fi = f32(i);
        let a = spin + fi * 2.39996;
        let rr = sqrt((fi + 0.5) / f32(count));
        let dir = t * (cos(a) * rr) + b * (sin(a) * rr) + n * sqrt(max(1.0 - rr * rr, 0.0));
        let k = (fi + 1.0) / f32(count);
        let s = pos + dir * radius * mix(0.12, 1.0, k * k);
        let clip = p.view_proj * vec4<f32>(s, 1.0);
        let ndc = clip.xy / clip.w;
        let suv = vec2<f32>(ndc.x * 0.5 + 0.5, 0.5 - ndc.y * 0.5);
        if (any(suv < vec2<f32>(0.0)) || any(suv > vec2<f32>(1.0))) {
            continue;
        }
        let scene_d = textureLoad(t0, vec2<i32>(suv * full), 0).w;
        let sample_d = dot(s - p.cam_pos.xyz, p.cam_fwd.xyz);
        if (scene_d < sample_d - 0.015) {
            occ += smoothstep(0.0, 1.0, radius / max(abs(d - scene_d), 1e-3));
        }
    }
    var ao = 1.0 - occ / f32(count);
    ao = mix(1.0, ao, 1.0 - smoothstep(30.0, 48.0, d));
    return vec4<f32>(ao, ao, ao, 1.0);
}

@fragment
fn fs_blur(in: POut) -> @location(0) vec4<f32> {
    let dim = vec2<i32>(textureDimensions(t0));
    let full = vec2<f32>(textureDimensions(t1));
    let c = vec2<i32>(in.uv * vec2<f32>(dim));
    let d0 = textureLoad(t1, vec2<i32>(in.uv * full), 0).w;
    var sum = 0.0;
    var wsum = 0.0;
    for (var y = -2; y <= 1; y++) {
        for (var x = -2; x <= 1; x++) {
            let q = clamp(c + vec2<i32>(x, y), vec2<i32>(0), dim - 1);
            let a = textureLoad(t0, q, 0).r;
            let quv = (vec2<f32>(q) + 0.5) / vec2<f32>(dim);
            let dq = textureLoad(t1, vec2<i32>(quv * full), 0).w;
            let w = exp(-abs(dq - d0) / (0.03 * d0 + 0.05));
            sum += a * w;
            wsum += w;
        }
    }
    let ao = sum / max(wsum, 1e-4);
    return vec4<f32>(ao, ao, ao, 1.0);
}

// ------------------------------------------------------------------ glow

// What glows (the bloom's and the shafts' input): the light tone mapped, so a brightly lit
// floor, holding many times the threshold's light, glows as much as it looks bright and does
// not swallow what stands on it. The glow still joins the light before the final tone map.
fn glow_light(c: vec3<f32>) -> vec3<f32> {
    return aces_filmic(max(c, vec3<f32>(0.0)));
}

// The scene's colour as light to blur and composite.
fn scene_light(c: vec3<f32>) -> vec3<f32> {
    return max(c, vec3<f32>(0.0));
}

// The scene shader's tone curve (ACES filmic as Three.js fits it) and sRGB encoding, for HDR.
fn rrt_odt_fit(v: vec3<f32>) -> vec3<f32> {
    let a = v * (v + 0.0245786) - 0.000090537;
    let b = v * (0.983729 * v + 0.4329510) + 0.238081;
    return a / b;
}

fn aces_filmic(c_in: vec3<f32>) -> vec3<f32> {
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
    var c = c_in / 0.6;
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

@fragment
fn fs_bloom_pre(in: POut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(t0));
    var c = vec3<f32>(0.0);
    c += glow_light(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(-1.0, -1.0), 0.0).rgb);
    c += glow_light(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(1.0, -1.0), 0.0).rgb);
    c += glow_light(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(-1.0, 1.0), 0.0).rgb);
    c += glow_light(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(1.0, 1.0), 0.0).rgb);
    c *= 0.25;
    let th = p.params2.x;
    // Soft-knee threshold on the brightest channel.
    let l = max(c.r, max(c.g, c.b));
    let knee = th * 0.5;
    var soft = clamp(l - th + knee, 0.0, 2.0 * knee);
    soft = soft * soft / (4.0 * knee + 1e-4);
    let w = max(soft, l - th) / max(l, 1e-4);
    return vec4<f32>(c * w, 1.0);
}

@fragment
fn fs_bloom_down(in: POut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(t0));
    var c = textureSampleLevel(t0, samp, in.uv, 0.0).rgb * 4.0;
    c += textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(-1.0, -1.0), 0.0).rgb;
    c += textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(1.0, -1.0), 0.0).rgb;
    c += textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(-1.0, 1.0), 0.0).rgb;
    c += textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(1.0, 1.0), 0.0).rgb;
    return vec4<f32>(c / 8.0, 1.0);
}

@fragment
fn fs_bloom_up(in: POut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(t0));
    var c = vec3<f32>(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let w = (2.0 - abs(f32(x))) * (2.0 - abs(f32(y)));
            c += textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(f32(x), f32(y)), 0.0).rgb * w;
        }
    }
    return vec4<f32>(c / 16.0, 1.0);
}

// ------------------------------------------------------------ light shafts
//
// Moonlight streaming past whatever stands against the sky: from each pixel
// walk toward the moon's place on screen gathering the sky seen on the way
// (brightest near the moon, dimmed where smoke or cloud hangs in front of
// it), fading with every step. Added into the glow at half resolution.

@fragment
fn fs_rays(in: POut) -> @location(0) vec4<f32> {
    let moon = p.moon.xy;
    let full = vec2<f32>(textureDimensions(t0));
    let aspect = vec2<f32>(full.x / full.y, 1.0);
    let steps = 28;
    let delta = (moon - in.uv) / f32(steps) * 0.85;
    let jitter = fract(52.9829189 * fract(dot(in.clip.xy, vec2<f32>(0.06711056, 0.00583715))));
    var uv = in.uv + delta * jitter;
    var decay = 1.0;
    var sum = vec3<f32>(0.0);
    for (var i = 0; i < steps; i++) {
        uv += delta;
        if (all(uv > vec2<f32>(0.0)) && all(uv < vec2<f32>(1.0))) {
            let d = textureSampleLevel(t1, samp, uv, 0.0).w;
            if (d > SKY) {
                let c = glow_light(textureSampleLevel(t0, samp, uv, 0.0).rgb);
                let near_moon = exp(-length((uv - moon) * aspect) * 4.0);
                sum += c * decay * near_moon;
            }
        }
        decay *= 0.94;
    }
    let reach = exp(-length((in.uv - moon) * aspect) * 1.6);
    return vec4<f32>(sum / f32(steps) * p.moon.z * reach * 4.0, 1.0);
}

// ------------------------------------------------------------------ composite

fn contact(uv: vec2<f32>) -> f32 {
    return pow(textureSampleLevel(t2, samp, uv, 0.0).r, p.params.y);
}

// The lamps' light scattered toward the eye by the air in front of the pixel (RP19): for each of
// the first few lamps, the haze's density round it times the light along the view ray, whose
// inverse-square falloff integrates in closed form (an arctangent), cut to the lamp's reach.
// No history: every frame stands alone, so a cut, a portal or a falling wall leaves no trail.
fn lamp_glow(uv: vec2<f32>, d: f32) -> vec3<f32> {
    let count = u32(p.vol.x);
    if (count == 0u || p.cam_pos.w > 0.5) {
        return vec3<f32>(0.0);
    }
    let eye = p.cam_pos.xyz;
    let far_point = world_at(uv, min(d, 60.0));
    let seg = far_point - eye;
    let len = length(seg);
    let dir = seg / max(len, 1e-4);
    let fall = max(p.haze.z, 0.05);
    var sum = vec3<f32>(0.0);
    for (var i = 0u; i < count; i++) {
        let l = lights[i];
        let to = l.pos.xyz - eye;
        let t0 = dot(to, dir);
        // (No nearer than the lamp's own size, so a ray grazing it does not blaze.)
        let h2 = max(dot(to, to) - t0 * t0, 0.0225);
        let h = sqrt(h2);
        // Only where the ray passes within the lamp's reach.
        var a = 0.0;
        var b = len;
        if (l.pos.w > 0.0) {
            if (h >= l.pos.w) {
                continue;
            }
            let half_chord = sqrt(l.pos.w * l.pos.w - h2);
            a = max(a, t0 - half_chord);
            b = min(b, t0 + half_chord);
        }
        if (b <= a) {
            continue;
        }
        let along = (atan((b - t0) / h) - atan((a - t0) / h)) / h;
        let density = p.haze.x * exp(-max(l.pos.y - p.haze.y, 0.0) / fall) + p.haze.w;
        sum += l.color.rgb * density * along;
    }
    // Scattered evenly every way (1 / 4 pi), as the scene holds light (times the exposure).
    return sum * p.vol.y * p.vol.z / (4.0 * 3.14159265);
}

// The scene's light at `uv`, contact shading taking away from its indirect part only: the sun,
// the moon, the lamps and what glows keep all theirs (RP15).
fn shaded(uv: vec2<f32>) -> vec3<f32> {
    let lit = scene_light(textureSampleLevel(t0, samp, uv, 0.0).rgb);
    let indirect = max(textureSampleLevel(t4, samp, uv, 0.0).rgb, vec3<f32>(0.0));
    return max(lit - indirect * (1.0 - contact(uv)), vec3<f32>(0.0));
}

fn blur_amount(d: f32, max_px: f32) -> f32 {
    return smoothstep(p.cam_r.w, p.cam_u.w, d - p.cam_fwd.w) * max_px;
}

@fragment
fn fs_composite(in: POut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(t0));
    let px = vec2<i32>(in.uv * dim);
    let d = textureLoad(t1, px, 0).w;
    let centre = (vec2<f32>(px) + 0.5) / dim;
    var lin = shaded(centre);

    // Background blur: things well behind the in-focus plane soften.
    let max_px = p.params.w * dim.y / 1080.0;
    var coc = 0.0;
    if (max_px > 0.0 && d < SKY) {
        coc = blur_amount(d, max_px);
    }
    if (coc > 0.6) {
        var acc = lin;
        var wsum = 1.0;
        for (var i = 0; i < 16; i++) {
            let fi = f32(i);
            let r = sqrt((fi + 0.5) / 16.0) * coc;
            let a = fi * 2.39996;
            let suv = in.uv + vec2<f32>(cos(a), sin(a)) * r / dim;
            let sd = textureSampleLevel(t1, samp, suv, 0.0).w;
            // Sharp things in front must not smear over the blurred background.
            let scoc = select(blur_amount(sd, max_px), max_px, sd > SKY);
            let w = clamp(scoc - r + 1.0, 0.0, 1.0);
            acc += shaded(suv) * w;
            wsum += w;
        }
        lin = acc / wsum;
    }

    lin += textureSampleLevel(t3, samp, in.uv, 0.0).rgb * p.params.z;
    lin += lamp_glow(in.uv, d);
    var col = to_srgb(aces_filmic(lin));

    // Grading: a touch more saturation and contrast, cool shadows, warm highlights.
    let grade = p.params2.w;
    let l = dot(col, vec3<f32>(0.2126, 0.7152, 0.0722));
    col = mix(vec3<f32>(l), col, 1.0 + 0.08 * grade);
    col = mix(col, col * col * (3.0 - 2.0 * col), 0.18 * grade);
    col += vec3<f32>(-0.006, 0.0, 0.014) * (1.0 - smoothstep(0.0, 0.4, l)) * grade;
    col += vec3<f32>(0.012, 0.006, -0.01) * smoothstep(0.5, 1.0, l) * grade;

    // Vignette.
    let v = length((in.uv - 0.5) * vec2<f32>(dim.x / dim.y, 1.0));
    col *= 1.0 - p.params2.y * smoothstep(0.45, 1.15, v);

    let noise = fract(52.9829189 * fract(dot(in.clip.xy, vec2<f32>(0.06711056, 0.00583715))));
    col += (noise - 0.5) / 255.0;
    return vec4<f32>(clamp(col, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
