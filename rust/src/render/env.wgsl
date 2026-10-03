// The environment's prefilter pass (RP14), compiled after `shaders.wgsl` (whose `vs_env`,
// `cube_dir` and globals it uses) in a module of its own: its group 1 holds a sampler, which the
// scene's pipelines must not see (their group 1 is the rigs' or the static faces' storage).

@group(1) @binding(0) var env_src: texture_cube<f32>;
@group(1) @binding(1) var env_src_samp: sampler;

fn hammersley(i: u32, n: u32) -> vec2<f32> {
    var b = (i << 16u) | (i >> 16u);
    b = ((b & 0x55555555u) << 1u) | ((b & 0xAAAAAAAAu) >> 1u);
    b = ((b & 0x33333333u) << 2u) | ((b & 0xCCCCCCCCu) >> 2u);
    b = ((b & 0x0F0F0F0Fu) << 4u) | ((b & 0xF0F0F0F0u) >> 4u);
    b = ((b & 0x00FF00FFu) << 8u) | ((b & 0xFF00FF00u) >> 8u);
    return vec2<f32>(f32(i) / f32(n), f32(b) * 2.3283064365386963e-10);
}

// A half vector about `n`, drawn as often as GGX with roughness `rough` would reflect along it.
fn sample_ggx(xi: vec2<f32>, n: vec3<f32>, rough: f32) -> vec3<f32> {
    let a = rough * rough;
    let phi = 2.0 * PI * xi.x;
    let cos_t = sqrt((1.0 - xi.y) / (1.0 + (a * a - 1.0) * xi.y));
    let sin_t = sqrt(1.0 - cos_t * cos_t);
    let up = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), abs(n.z) < 0.999);
    let tx = normalize(cross(up, n));
    let ty = cross(n, tx);
    return normalize(tx * cos(phi) * sin_t + ty * sin(phi) * sin_t + n * cos_t);
}

// One mip of the prefiltered environment: the first mip seen through this mip's roughness.
@fragment
fn fs_prefilter(in: EnvOut) -> @location(0) vec4<f32> {
    let n = cube_dir(in.face, in.uv);
    let count = 96u;
    var sum = vec3<f32>(0.0);
    var weight = 0.0;
    for (var i = 0u; i < count; i++) {
        let h = sample_ggx(hammersley(i, count), n, max(in.rough, 0.02));
        let l = 2.0 * dot(n, h) * h - n;
        let nl = dot(n, l);
        if (nl > 0.0) {
            sum += textureSampleLevel(env_src, env_src_samp, l, 0.0).rgb * nl;
            weight += nl;
        }
    }
    return vec4<f32>(sum / max(weight, 1e-4), 1.0);
}
