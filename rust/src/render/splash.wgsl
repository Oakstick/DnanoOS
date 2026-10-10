// The engine's loading screen (splash.rs): the controller legend (the text layer) over a dark
// gradient (or backdrop art, dimmed), a slow push in, the logo breathing light with a sheen sweeping over
// it, the progress bar, a spinner on the step under way, and the text layer over all.
// Colours are display values: the target is not sRGB and the art is not converted.

struct U {
    // Seconds in, progress (0-1), the window's aspect, the art's aspect.
    a: vec4<f32>,
    // The window's size in pixels, whether there is art, and the pixel scale.
    res: vec4<f32>,
    // Where the logo sits in the art (uv): its centre and half size.
    logo: vec4<f32>,
    // The bar in pixels: its two ends, its height, its half thickness.
    bar: vec4<f32>,
    // The spinner in pixels: its centre, its radius, and whether it shows.
    spin: vec4<f32>,
    bg: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var art: texture_2d<f32>;
@group(0) @binding(2) var samp: sampler;
@group(0) @binding(3) var layer: texture_2d<f32>;

struct VOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VOut {
    let p = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    var o: VOut;
    o.pos = vec4<f32>(p * 2.0 - 1.0, 0.0, 1.0);
    o.uv = vec2<f32>(p.x, 1.0 - p.y);
    return o;
}

// Distance to a capsule from a to b (horizontal) of radius r.
fn capsule(p: vec2<f32>, x0: f32, x1: f32, y: f32, r: f32) -> f32 {
    let q = vec2<f32>(p.x - clamp(p.x, x0, max(x1, x0)), p.y - y);
    return length(q) - r;
}

fn backdrop(uv: vec2<f32>, time: f32) -> vec3<f32> {
    if (u.res.z < 0.5) {
        // No art: a cold dark, lighter round where the logo would be, and a faint horizon glow.
        let d = length((uv - vec2<f32>(0.5, 0.42)) * vec2<f32>(u.a.z, 1.0));
        var c = mix(vec3<f32>(0.05, 0.075, 0.1), u.bg.rgb, smoothstep(0.0, 0.9, d));
        c += vec3<f32>(0.1, 0.25, 0.45) * exp(-pow((uv.y - 0.66) / 0.05, 2.0)) * 0.12;
        return c;
    }
    let logo = u.logo.xy;
    // Cover the window, cropping the art's long side about its middle.
    var iuv = uv;
    if (u.a.z > u.a.w) {
        iuv.y = 0.5 + (uv.y - 0.5) * (u.a.w / u.a.z);
    } else {
        iuv.x = 0.5 + (uv.x - 0.5) * (u.a.z / u.a.w);
    }
    // A slow push in toward the logo.
    let zoom = 1.0 + 0.03 * smoothstep(0.0, 14.0, time);
    iuv = logo + (iuv - logo) / zoom;
    var col = textureSample(art, samp, iuv).rgb;
    // The logo: its bright blue glass, found by colour inside its box.
    let rel = (iuv - logo) / u.logo.zw;
    let in_box = (1.0 - smoothstep(0.85, 1.0, abs(rel.x))) * (1.0 - smoothstep(0.85, 1.0, abs(rel.y)));
    let glass = smoothstep(0.04, 0.22, col.b - col.r) * smoothstep(0.3, 0.75, col.b) * in_box;
    // It breathes light into the air round it, and a sheen sweeps across it every few seconds.
    let d = length((iuv - logo) * vec2<f32>(u.a.w, 1.0));
    let breath = 0.5 + 0.5 * sin(time * 1.9);
    col += vec3<f32>(0.18, 0.5, 1.0) * exp(-d * d * 30.0) * (0.04 + 0.06 * breath);
    let sweep = fract(time / 3.4) * 2.6 - 0.8;
    let along = rel.x * 0.75 + rel.y * 0.45;
    col += vec3<f32>(0.65, 0.88, 1.0) * exp(-pow((along - sweep) / 0.12, 2.0)) * glass * 0.7;
    col += vec3<f32>(0.1, 0.3, 0.6) * glass * breath * 0.1;
    return col;
}

@fragment
fn fs(in: VOut) -> @location(0) vec4<f32> {
    let time = u.a.x;
    let progress = u.a.y;
    let s = u.res.w;
    let p = in.pos.xy;
    var col = backdrop(in.uv, time);
    // Art behind the controller legend is dimmed well under it.
    col = mix(col, col * 0.3, u.res.z);

    // The bar: a hairline track, the filled part a blue gradient with a glow and a bright head,
    // and a glint running along what is filled.
    let x0 = u.bar.x;
    let x1 = u.bar.y;
    let y = u.bar.z;
    let r = u.bar.w;
    let xf = mix(x0, x1, progress);
    let track = capsule(p, x0, x1, y, r + 1.0 * s);
    col = mix(col, vec3<f32>(0.45, 0.52, 0.6), (1.0 - smoothstep(0.0, 1.0, abs(track) - 0.35 * s)) * 0.35);
    col = mix(col, u.bg.rgb * 0.6, (1.0 - smoothstep(-0.5, 0.5, track)) * 0.5);
    if (progress > 0.001) {
        let fill = capsule(p, x0, xf, y, r);
        let t = clamp((p.x - x0) / max(x1 - x0, 1.0), 0.0, 1.0);
        let blue = mix(vec3<f32>(0.08, 0.42, 0.85), vec3<f32>(0.42, 0.82, 1.0), t);
        let glint = exp(-pow((p.x - x0 - fract(time * 0.45) * (xf - x0)) / (40.0 * s), 2.0)) * 0.5;
        col = mix(col, blue + glint, 1.0 - smoothstep(-0.5, 0.5, fill));
        col += vec3<f32>(0.2, 0.55, 1.0) * exp(-max(fill, 0.0) * max(fill, 0.0) / (36.0 * s * s)) * 0.25;
        let hd = length((p - vec2<f32>(xf, y)) / vec2<f32>(14.0 * s, 5.0 * s));
        col += vec3<f32>(0.55, 0.85, 1.0) * exp(-hd * hd) * 0.9;
    }

    // The spinner on the step under way: a bright arc chasing round its ring.
    if (u.spin.w > 0.5) {
        let q = p - u.spin.xy;
        let ring = abs(length(q) - u.spin.z) - 1.2 * s;
        let ang = atan2(q.y, q.x);
        let head = fract((ang / 6.2831853) - time * 0.9);
        let arc = smoothstep(0.55, 1.0, head);
        col = mix(col, vec3<f32>(0.45, 0.85, 1.0), (1.0 - smoothstep(-0.5, 0.5, ring)) * arc);
        col += vec3<f32>(0.2, 0.55, 1.0) * exp(-pow(max(ring, 0.0) / (3.0 * s), 2.0)) * arc * 0.35;
    }

    // Text and icons (premultiplied).
    let l = textureSample(layer, samp, in.uv);
    col = col * (1.0 - l.a) + l.rgb;

    // A soft vignette, and the fade in from the window's first colour.
    let v = length(in.uv - 0.5) * 1.25;
    col *= 1.0 - 0.3 * v * v;
    let fade = smoothstep(0.0, 0.7, time);
    return vec4<f32>(mix(u.bg.rgb, col, fade), 1.0);
}
