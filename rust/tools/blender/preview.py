"""Contact sheets of a .blend's clips, to look at them without the game: each action a row of frames
(its markers among them), rendered with Workbench in the voxels' colours.

    blender -b ragnar.blend -P tools/blender/preview.py -- OUT_DIR [action ...] [--yaw DEG] [--rise R] [--frames N]

Writes OUT_DIR/<action>.png (rows of five frames) for each action (all of them when none is named).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import numpy as np  # noqa: E402
import vh_blender as vh  # noqa: E402


def sheet(paths, out):
    imgs = []
    for p in paths:
        im = bpy.data.images.load(p)
        w, h = im.size
        px = np.empty(w * h * 4, dtype=np.float32)
        im.pixels.foreach_get(px)
        imgs.append(px.reshape(h, w, 4))
        bpy.data.images.remove(im)
    cols = 5
    while len(imgs) % cols:
        imgs.append(np.full_like(imgs[0], 0.0))
    rows = [np.concatenate(imgs[k:k + cols], axis=1) for k in range(0, len(imgs), cols)]
    # (Blender's pixels run bottom up: the first row goes on top.)
    row = np.concatenate(rows[::-1], axis=0)
    h, w = row.shape[:2]
    out_im = bpy.data.images.new("sheet", w, h, alpha=True)
    out_im.pixels.foreach_set(row.ravel())
    out_im.filepath_raw = out
    out_im.file_format = "PNG"
    out_im.save()
    bpy.data.images.remove(out_im)


def main():
    a = vh.args()
    out = a[0]
    os.makedirs(out, exist_ok=True)
    yaw, rise, count = 30.0, 0.2, 8
    names = []
    i = 1
    while i < len(a):
        if a[i] == "--yaw":
            yaw = float(a[i + 1])
            i += 2
        elif a[i] == "--rise":
            rise = float(a[i + 1])
            i += 2
        elif a[i] == "--frames":
            count = int(a[i + 1])
            i += 2
        else:
            names.append(a[i])
            i += 1
    scene = bpy.context.scene
    arm = vh.armature()
    vh.setup_render(scene, 300, 360)
    vh.camera(scene, at=(0.0, 0.56, 0.0), yaw=yaw, dist=2.1, rise=rise)
    for act in bpy.data.actions:
        if names and act.name not in names:
            continue
        arm.animation_data.action = act
        f0, f1 = int(act.frame_range[0]), int(act.frame_range[1])
        frames = sorted({round(f0 + (f1 - f0) * k / (count - 1)) for k in range(count)} | {int(m.frame) for m in act.pose_markers})
        paths = []
        for f in frames:
            scene.frame_set(f)
            p = os.path.join(out, f"_{act.name}_{f:03d}.png")
            vh.render(scene, p)
            paths.append(os.path.abspath(p))
        sheet(paths, os.path.abspath(os.path.join(out, f"{act.name}.png")))
        for p in paths:
            os.remove(p)
        print(f"preview: {act.name}: frames {frames}")


main()
