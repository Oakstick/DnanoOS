"""Imports a rig the game exported and checks it: every bone there with its parent, every voxel mesh
bound wholly to its own bone, the meshes coloured. Then renders it twice (its colours, and each
bone's voxels in the bone's own colour) and can save the .blend to animate in.

    AdventuresOfTheAncientBlades --export-rig ragnar ragnar_rig.glb
    blender -b -P tools/blender/import_rig.py -- ragnar_rig.glb OUT_DIR [ragnar.blend]

Writes OUT_DIR/rig_colours.png and OUT_DIR/rig_bones.png; exits 1 if a check fails.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import bpy  # noqa: E402
import vh_blender as vh  # noqa: E402


def check(arm):
    errors = []
    names = [b.name for b in arm.data.bones]
    # (A humanoid, hero or foe, against the heroes' skeleton; another skeleton is checked by the game's
    # own export test, here only for its binding.)
    for b in (vh.BONES if "thigh.L" in names else []):
        if b not in names:
            errors.append(f"no bone {b}")
            continue
        parent = arm.data.bones[b].parent
        want = vh.PARENTS.get(b)
        if (parent.name if parent else None) != want:
            errors.append(f"{b} hangs from {parent.name if parent else None}, not {want}")
    voxels = 0
    for o in bpy.data.objects:
        if o.type != "MESH" or not o.name.endswith("_voxels"):
            continue
        bone = o.name[: -len("_voxels")]
        if not any(m.type == "ARMATURE" and m.object == arm for m in o.modifiers):
            errors.append(f"{o.name} is not bound to the armature")
        group = o.vertex_groups.get(bone)
        if group is None:
            errors.append(f"{o.name} has no group for {bone}")
            continue
        for v in o.data.vertices:
            ws = [(o.vertex_groups[g.group].name, g.weight) for g in v.groups if g.weight > 0.0]
            if ws != [(bone, 1.0)] and not (len(ws) == 1 and ws[0][0] == bone and abs(ws[0][1] - 1.0) < 1e-6):
                errors.append(f"{o.name}: a vertex weighted {ws}")
                break
        if not o.data.color_attributes:
            errors.append(f"{o.name} has no colours")
        voxels += len(o.data.vertices) // 4
    return errors, voxels


def main():
    a = vh.args()
    if len(a) < 2:
        sys.exit(__doc__)
    path, out = a[0], a[1]
    os.makedirs(out, exist_ok=True)
    scene = vh.fresh_scene()
    arm = vh.import_rig(path)
    errors, faces = check(arm)
    print(f"import_rig: {len(arm.data.bones)} bones, {faces} faces on {sum(1 for o in bpy.data.objects if o.name.endswith('_voxels'))} meshes")
    for e in errors:
        print("import_rig: FAIL", e)
    vh.setup_render(scene)
    # (Framed on the rig's own height: a raven is a hand high, the rasetsu twice a man.)
    top = max((o.matrix_world @ v.co).z for o in bpy.data.objects if o.name.endswith("_voxels") for v in o.data.vertices)
    at, dist = (0.0, top * 0.5, 0.0), max(1.2, top * 2.3)
    vh.camera(scene, at=at, dist=dist, yaw=25.0)
    vh.render(scene, os.path.join(out, "rig_colours.png"))
    colors = vh.bone_colors(vh.bones_of(arm))
    for o in bpy.data.objects:
        if o.name.endswith("_voxels"):
            o.color = colors.get(o.name[: -len("_voxels")], (1, 1, 1, 1))
    vh.setup_render(scene, color="OBJECT")
    vh.render(scene, os.path.join(out, "rig_bones.png"))
    vh.camera(scene, at=at, dist=dist, yaw=180.0 + 25.0)
    vh.render(scene, os.path.join(out, "rig_bones_back.png"))
    if len(a) > 2:
        bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(a[2]))
    print("import_rig:", "FAILED" if errors else "ok")
    sys.exit(1 if errors else 0)


main()
