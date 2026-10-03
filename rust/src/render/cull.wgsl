// GPU visibility for the static scenery (RP20 in the Technical Art and Rendering Plan): each
// chunk's bounds are tested against each view's frustum on the GPU, and the chunks seen are
// written out as indirect draws (one quad, `count` faces from `faces` on), so a pass draws all
// its scenery in one call and the CPU neither tests the chunks nor records a draw for each.
//
// Views: 0 the camera, 1 the fine shadow cascade, 2 the coarse one's static cache. With
// `views.n.z` set, the draws seen are packed to the front of each view's run and counted (for a
// counted multi-draw); without it each chunk keeps its slot, drawn with no faces when unseen.

struct Chunk {
    lo: vec4<f32>,
    hi: vec4<f32>,
    faces: u32,
    count: u32,
    dir: u32,   // the way all its faces look (0 to 5), or 6 for any
    pad: u32,
};

const AXES = array<vec3<f32>, 6>(
    vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-1.0, 0.0, 0.0), vec3<f32>(0.0, 1.0, 0.0),
    vec3<f32>(0.0, -1.0, 0.0), vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(0.0, 0.0, -1.0),
);

struct Views {
    planes: array<vec4<f32>, 18>,
    // Per view, where it looks from: xyz an eye (w 1), or the way toward the viewer (w 0), or
    // nothing (w 2, every face kept).
    eyes: array<vec4<f32>, 3>,
    // x chunks, y views, z 1 to pack and count
    n: vec4<u32>,
};

struct Args {
    index_count: u32,
    instance_count: u32,
    first_index: u32,
    base_vertex: i32,
    first_instance: u32,
};

@group(0) @binding(0) var<storage, read> chunks: array<Chunk>;
@group(0) @binding(1) var<uniform> views: Views;
@group(0) @binding(2) var<storage, read_write> args: array<Args>;
@group(0) @binding(3) var<storage, read_write> counts: array<atomic<u32>, 4>;

@compute @workgroup_size(64)
fn cs_cull(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    let v = id.y;
    let n = views.n.x;
    if (i >= n || v >= views.n.y) {
        return;
    }
    let c = chunks[i];
    // Unseen only when the box lies wholly outside one plane (as `Frustum::sees`).
    var seen = c.count > 0u;
    for (var k = 0u; k < 6u; k++) {
        let p = views.planes[v * 6u + k];
        let corner = select(c.lo.xyz, c.hi.xyz, p.xyz >= vec3<f32>(0.0));
        if (dot(p.xyz, corner) + p.w < 0.0) {
            seen = false;
        }
    }
    // A group facing one way is dropped whole when every face in it is turned from the view (RP21):
    // from an eye, when the eye lies behind the nearest face plane of the group; from afar,
    // when the way toward the viewer is behind the faces. (The vertex shader skips such faces
    // one by one; this spares their vertices.)
    let e = views.eyes[v];
    if (seen && c.dir < 6u && e.w < 1.5) {
        let n = AXES[c.dir];
        if (e.w > 0.5) {
            // The group's most favourable face plane: its lowest along n.
            let lowest = select(dot(c.hi.xyz, n), dot(c.lo.xyz, n), dot(n, vec3<f32>(1.0)) > 0.0);
            seen = dot(e.xyz, n) > lowest;
        } else {
            seen = dot(e.xyz, n) > 0.0;
        }
    }
    if (views.n.z == 1u) {
        if (seen) {
            let slot = atomicAdd(&counts[v], 1u);
            args[v * n + slot] = Args(6u, c.count, 0u, 0, c.faces);
        }
    } else {
        args[v * n + i] = Args(6u, select(0u, c.count, seen), 0u, 0, c.faces);
    }
}
