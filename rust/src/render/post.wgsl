// Post-processing, after the scene pass has written its color and a buffer of
// world normals and view depth (the "gbuf"):
//
//   ssao       contact shading: how much nearby geometry hides each pixel's sky
//   blur       a depth-aware blur of that, at half resolution
//   bloom_*    glow: bright parts, shrunk down a chain of textures and grown back
//   composite  color x contact shading, background blur, glow, grading, vignette
//
// The scene color arrives tone mapped and sRGB encoded; glow and blur mix in
// (approximately) linear light and encode again at the end.

struct Post {
    view_proj: mat4x4<f32>,
    cam_pos: vec4<f32>,   // w: 1 for an orthographic camera
    cam_fwd: vec4<f32>,   // w: depth of the in-focus plane
    cam_r: vec4<f32>,     // right, scaled to the screen edge; w: blur starts this far behind focus
    cam_u: vec4<f32>,     // up, scaled to the screen edge; w: blur is full this far behind focus
    params: vec4<f32>,    // contact radius, contact strength, glow strength, blur max px at 1080p (0 = off)
    params2: vec4<f32>,   // glow threshold, vignette, time, grading
};

@group(0) @binding(0) var<uniform> p: Post;
@group(0) @binding(1) var t0: texture_2d<f32>;
@group(0) @binding(2) var t1: texture_2d<f32>;
@group(0) @binding(3) var t2: texture_2d<f32>;
@group(0) @binding(4) var t3: texture_2d<f32>;
@group(0) @binding(5) var samp: sampler;

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

fn to_lin(c: vec3<f32>) -> vec3<f32> {
    return pow(max(c, vec3<f32>(0.0)), vec3<f32>(2.2));
}

@fragment
fn fs_bloom_pre(in: POut) -> @location(0) vec4<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(t0));
    var c = vec3<f32>(0.0);
    c += to_lin(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(-1.0, -1.0), 0.0).rgb);
    c += to_lin(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(1.0, -1.0), 0.0).rgb);
    c += to_lin(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(-1.0, 1.0), 0.0).rgb);
    c += to_lin(textureSampleLevel(t0, samp, in.uv + texel * vec2<f32>(1.0, 1.0), 0.0).rgb);
    c *= 0.25;
    // Soft-knee threshold on the brightest channel.
    let l = max(c.r, max(c.g, c.b));
    let th = p.params2.x;
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

// ------------------------------------------------------------------ composite

fn contact(uv: vec2<f32>) -> f32 {
    return pow(textureSampleLevel(t2, samp, uv, 0.0).r, p.params.y);
}

fn blur_amount(d: f32, max_px: f32) -> f32 {
    return smoothstep(p.cam_r.w, p.cam_u.w, d - p.cam_fwd.w) * max_px;
}

@fragment
fn fs_composite(in: POut) -> @location(0) vec4<f32> {
    let dim = vec2<f32>(textureDimensions(t0));
    let px = vec2<i32>(in.uv * dim);
    let d = textureLoad(t1, px, 0).w;
    var lin = to_lin(textureLoad(t0, px, 0).rgb) * contact(in.uv);

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
            acc += to_lin(textureSampleLevel(t0, samp, suv, 0.0).rgb) * contact(suv) * w;
            wsum += w;
        }
        lin = acc / wsum;
    }

    lin += textureSampleLevel(t3, samp, in.uv, 0.0).rgb * p.params.z;
    var col = pow(max(lin, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.2));

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
