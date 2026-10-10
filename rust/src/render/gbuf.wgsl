// The normal-and-depth buffer's resolve (RP15). The scene draws it with 4x MSAA; averaging the
// samples, as a hardware resolve does, would give an edge pixel a normal and a depth halfway
// between the thing in front and the thing behind, a surface that is not there, and contact
// shading and the background blur would read it so. Instead each pixel keeps its nearest sample:
// the thing in front, whole.

@group(0) @binding(0) var gbuf_ms: texture_multisampled_2d<f32>;

@vertex
fn vs_resolve(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
    let q = vec2<f32>(f32((i << 1u) & 2u), f32(i & 2u));
    return vec4<f32>(q * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_resolve(@builtin(position) at: vec4<f32>) -> @location(0) vec4<f32> {
    let px = vec2<i32>(at.xy);
    var best = textureLoad(gbuf_ms, px, 0);
    for (var s = 1; s < 4; s++) {
        let g = textureLoad(gbuf_ms, px, s);
        if (g.w < best.w) {
            best = g;
        }
    }
    return best;
}
