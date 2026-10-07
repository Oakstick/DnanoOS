"""Thins the keys of the clips the game embeds: each track of a GLB keeps only the keys a straight line
between their neighbours does not already give, within a tolerance (Ramer-Douglas-Peucker on the
whole vector). Blender samples every bone on every frame; the game resamples a clip at 60 Hz as it
reads it (`anim::clip`), so a clip keeps its shape in about half the bytes, which the web build,
embedding them all, is glad of. `export_clips.py` thins what it writes; this does it to clips
already written, without Blender:

    python tools/blender/thin_clips.py assets/anim/*/*.glb
"""
import sys

# How far a track may stray from the clip as baked when its keys are thinned (`thin_glb`): a
# quaternion's component (0.003 is about a third of a degree), a metre, a scale.
TOLERANCE = {"rotation": 0.003, "translation": 0.0005, "scale": 0.002}


def thin_glb(path):
    """Rewrites the GLB at `path` with each track's keys thinned: those a straight line between their
    neighbours already gives (within `TOLERANCE`, Ramer-Douglas-Peucker on the whole vector) are
    dropped. Blender samples every bone on every frame; the game resamples a clip at 60 Hz as it reads
    it, so a clip keeps its shape in a fraction of the bytes (the web build embeds them all). Returns
    the bytes before and after."""
    import json
    import struct

    data = open(path, "rb").read()
    jlen = struct.unpack_from("<I", data, 12)[0]
    doc = json.loads(data[20:20 + jlen])
    bin_at = 20 + jlen + 8
    blob = data[bin_at:]
    width = {"SCALAR": 1, "VEC3": 3, "VEC4": 4}

    def read(acc):
        a = doc["accessors"][acc]
        v = doc["bufferViews"][a["bufferView"]]
        n = a["count"] * width[a["type"]]
        off = v.get("byteOffset", 0) + a.get("byteOffset", 0)
        return list(struct.unpack_from(f"<{n}f", blob, off))

    tracks = []
    for anim in doc.get("animations", []):
        for ch in anim["channels"]:
            smp = anim["samplers"][ch["sampler"]]
            times = read(smp["input"])
            k = width[doc["accessors"][smp["output"]]["type"]]
            vals = read(smp["output"])
            pts = [vals[i * k:(i + 1) * k] for i in range(len(times))]
            eps = TOLERANCE.get(ch["target"]["path"], 0.001)
            keep = [False] * len(times)
            keep[0] = keep[-1] = True
            stack = [(0, len(times) - 1)] if len(times) > 2 else []
            while stack:
                a, b = stack.pop()
                worst, at = 0.0, -1
                for i in range(a + 1, b):
                    u = (times[i] - times[a]) / max(1e-9, times[b] - times[a])
                    err = max(abs(pts[i][c] - (pts[a][c] + (pts[b][c] - pts[a][c]) * u)) for c in range(k))
                    if err > worst:
                        worst, at = err, i
                if worst > eps:
                    keep[at] = True
                    stack += [(a, at), (at, b)]
            idx = [i for i in range(len(times)) if keep[i]]
            # (A track that never leaves the bone's rest is dropped: a bone without one holds its rest.)
            node = doc["nodes"][ch["target"]["node"]]
            rest = node.get(ch["target"]["path"], {"translation": [0, 0, 0], "rotation": [0, 0, 0, 1], "scale": [1, 1, 1]}[ch["target"]["path"]])
            if len(idx) == 2 and all(abs(pts[i][c] - rest[c]) <= eps for i in idx for c in range(k)):
                continue
            tracks.append((ch, smp, [times[i] for i in idx], [x for i in idx for x in pts[i]], doc["accessors"][smp["output"]]["type"]))
    # A fresh buffer: each track's times and values in views of their own.
    out = bytearray()
    views, accs = [], []

    def add(floats, kind, minmax=False):
        while len(out) % 4:
            out.append(0)
        views.append({"buffer": 0, "byteOffset": len(out), "byteLength": 4 * len(floats)})
        out.extend(struct.pack(f"<{len(floats)}f", *floats))
        acc = {"bufferView": len(views) - 1, "componentType": 5126, "count": len(floats) // width[kind], "type": kind}
        if minmax:
            acc["min"], acc["max"] = [min(floats)], [max(floats)]
        accs.append(acc)
        return len(accs) - 1

    for anim in doc.get("animations", []):
        anim["channels"], anim["samplers"] = [], []
    anim = doc["animations"][0] if doc.get("animations") else None
    for ch, smp, times, vals, kind in tracks:
        smp = {"input": add(times, "SCALAR", True), "output": add(vals, kind), "interpolation": "LINEAR"}
        anim["samplers"].append(smp)
        anim["channels"].append({"sampler": len(anim["samplers"]) - 1, "target": ch["target"]})
    doc["accessors"], doc["bufferViews"] = accs, views
    doc["buffers"] = [{"byteLength": len(out)}]
    js = json.dumps(doc, separators=(",", ":")).encode()
    js += b" " * (-len(js) % 4)
    out += bytes(-len(out) % 4)
    total = 12 + 8 + len(js) + 8 + len(out)
    glb = struct.pack("<4sII", b"glTF", 2, total) + struct.pack("<I4s", len(js), b"JSON") + js + struct.pack("<I4s", len(out), b"BIN" + bytes(1)) + bytes(out)
    open(path, "wb").write(glb)
    return len(data), len(glb)


if __name__ == "__main__":
    total = [0, 0]
    for f in sys.argv[1:]:
        a, b = thin_glb(f)
        total[0] += a
        total[1] += b
    print(f"thin_clips: {len(sys.argv) - 1} clips, {total[0] // 1024} KB to {total[1] // 1024} KB")
