#!/usr/bin/env python3
"""MPFB2-powered Blender 5.3 hero-miniature generator for Alex Wen (Set 1).

Builds Alex Wen (1.78 m Chinese-Malay male, athletic ecto-endomorphic build) on
top of the MakeHuman / MPFB2 base mesh and skeleton rig (`bl_ext.blender_org.mpfb`),
posing him into a natural relaxed standing posture, conforming his Set 1 outfit
(dusty olive-grey crew-neck t-shirt, loose faded blue jeans, light-to-medium brown
trail shoes, bare wrists), exporting a `satya-web`-compatible `.glb`, and
optionally rendering 3-angle verification previews via `BLENDER_EEVEE`.

Usage (requires `--` before script args; run outside sandbox so Blender sees MPFB2):
    blender --background --python build_alex_wen_set01.py -- \
        --export-glb assets/models/characters/alex_wen/alex_wen_set01.glb \
        --render-dir path/to/renders
"""

from __future__ import annotations

import argparse
import importlib
import math
import os
import sys
from typing import Sequence

import bmesh
import bpy
from mathutils import Euler, Matrix, Quaternion, Vector

TARGET_HEIGHT_M = 1.78


def mpfb_import(module_suffix: str, symbol: str):
    """Resolve an MPFB2 class from Blender's extension module namespace."""
    for mod_name in list(sys.modules):
        if mod_name.endswith(module_suffix):
            mod = importlib.import_module(mod_name)
            if hasattr(mod, symbol):
                return getattr(mod, symbol)
    raise RuntimeError(
        f"MPFB2 module '{module_suffix}.{symbol}' not loaded. "
        "Ensure `bl_ext.blender_org.mpfb` is enabled and accessible."
    )


def srgb_to_linear(c: float) -> float:
    """Convert an sRGB channel [0, 1] into linear RGB for Principled BSDF."""
    if c <= 0.04045:
        return c / 12.92
    return ((c + 0.055) / 1.055) ** 2.4


def make_material(
    name: str,
    srgb: tuple[float, float, float],
    roughness: float = 0.75,
    metallic: float = 0.0,
) -> bpy.types.Material:
    """Create a Principled BSDF material with linear RGBA baseColorFactor."""
    mat = bpy.data.materials.new(name=name)
    mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF")
    if bsdf is None:
        bsdf = mat.node_tree.nodes.new(type="ShaderNodeBsdfPrincipled")
    bsdf.inputs["Base Color"].default_value = (
        srgb_to_linear(srgb[0]),
        srgb_to_linear(srgb[1]),
        srgb_to_linear(srgb[2]),
        1.0,
    )
    bsdf.inputs["Roughness"].default_value = roughness
    bsdf.inputs["Metallic"].default_value = metallic
    if "Specular IOR Level" in bsdf.inputs:
        bsdf.inputs["Specular IOR Level"].default_value = 0.25
    return mat


def create_palette() -> dict[str, bpy.types.Material]:
    """Build Set 1 canonical materials (avoiding 'tint'/'primary' in names)."""
    return {
        "skin": make_material("Alex_Skin_WarmOliveTan", (0.64, 0.45, 0.33), roughness=0.68),
        "lips": make_material("Alex_Lips_Muted", (0.57, 0.39, 0.30), roughness=0.62),
        "hair": make_material("Alex_Hair_JetBlack", (0.025, 0.027, 0.032), roughness=0.58),
        "sclera": make_material("Alex_Eye_Sclera", (0.91, 0.90, 0.88), roughness=0.22),
        "iris": make_material("Alex_Eye_IrisBrown", (0.16, 0.09, 0.055), roughness=0.18),
        "pupil": make_material("Alex_Eye_Pupil", (0.02, 0.02, 0.025), roughness=0.12),
        "catchlight": make_material("Alex_Eye_Catchlight", (0.98, 0.99, 1.0), roughness=0.05),
        "tee": make_material("Alex_Tee_OliveGrey", (0.35, 0.34, 0.27), roughness=0.88),
        "tee_trim": make_material("Alex_Tee_TrimDark", (0.28, 0.27, 0.21), roughness=0.90),
        "jeans": make_material("Alex_Jeans_FadedBlue", (0.29, 0.38, 0.48), roughness=0.86),
        "jeans_seam": make_material("Alex_Jeans_IndigoSeam", (0.24, 0.33, 0.43), roughness=0.88),
        "shoe_suede": make_material("Alex_Shoe_TanSuede", (0.48, 0.34, 0.22), roughness=0.84),
        "shoe_cap": make_material("Alex_Shoe_DarkTanCap", (0.39, 0.27, 0.17), roughness=0.84),
        "shoe_mesh": make_material("Alex_Shoe_BrownMesh", (0.30, 0.20, 0.13), roughness=0.80),
        "shoe_sole": make_material("Alex_Shoe_CharcoalSole", (0.095, 0.084, 0.076), roughness=0.74),
    }


def ensure_collection(name: str) -> bpy.types.Collection:
    """Get or create a named collection linked to the active scene."""
    col = bpy.data.collections.get(name)
    if col is None:
        col = bpy.data.collections.new(name)
        bpy.context.scene.collection.children.link(col)
    return col


def apply_all_modifiers(
    obj: bpy.types.Object,
    smooth: bool = True,
    recalc_normals: bool = False,
) -> bpy.types.Object:
    """Apply modifiers cleanly; preserve source winding unless `recalc_normals=True`."""
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    for mod in list(obj.modifiers):
        bpy.ops.object.modifier_apply(modifier=mod.name)

    if smooth:
        for poly in obj.data.polygons:
            poly.use_smooth = True

    if recalc_normals:
        bm = bmesh.new()
        bm.from_mesh(obj.data)
        bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
        bm.to_mesh(obj.data)
        bm.free()

    obj.data.update()
    return obj


def rotate_pose_bone_world(
    rig: bpy.types.Object,
    bone_name: str,
    world_axis: Vector,
    angle_deg: float,
) -> None:
    """Rotate a pose bone around a world-space axis cleanly regardless of bone roll."""
    pb = rig.pose.bones.get(bone_name)
    if pb is None:
        return
    pb.rotation_mode = "QUATERNION"
    bone_mat3 = (rig.matrix_world.to_3x3() @ pb.bone.matrix_local.to_3x3()).normalized()
    local_axis = bone_mat3.inverted() @ world_axis.normalized()
    pb.rotation_quaternion = Quaternion(local_axis, math.radians(angle_deg)) @ pb.rotation_quaternion


def aim_bone_toward_dir(
    rig: bpy.types.Object,
    bone_name: str,
    target_world_dir: Vector,
    blend: float = 1.0,
) -> None:
    """Rotate a rest-chain bone so its world axis aims toward `target_world_dir`."""
    pb = rig.pose.bones.get(bone_name)
    if pb is None:
        return
    pb.rotation_mode = "QUATERNION"
    cur_dir = (rig.matrix_world.to_3x3() @ (pb.bone.tail_local - pb.bone.head_local)).normalized()
    world_q = cur_dir.rotation_difference(target_world_dir.normalized())
    if blend < 1.0:
        world_q = Quaternion((1.0, 0.0, 0.0, 0.0)).slerp(world_q, blend)
    bone_mat3 = (rig.matrix_world.to_3x3() @ pb.bone.matrix_local.to_3x3()).normalized()
    local_m = bone_mat3.inverted() @ world_q.to_matrix() @ bone_mat3
    pb.rotation_quaternion = local_m.to_quaternion() @ pb.rotation_quaternion


def aim_posed_bone_toward_dir(
    rig: bpy.types.Object,
    bone_name: str,
    target_world_dir: Vector,
    blend: float = 1.0,
) -> None:
    """Rotate a child pose bone using its current posed transform (`bpy.context.view_layer.update()`)."""
    pb = rig.pose.bones.get(bone_name)
    if pb is None:
        return
    pb.rotation_mode = "QUATERNION"
    cur_dir = ((rig.matrix_world @ pb.tail) - (rig.matrix_world @ pb.head)).normalized()
    world_q = cur_dir.rotation_difference(target_world_dir.normalized())
    if blend < 1.0:
        world_q = Quaternion((1.0, 0.0, 0.0, 0.0)).slerp(world_q, blend)
    posed_mat3 = (rig.matrix_world @ pb.matrix).to_3x3().normalized()
    local_q = (posed_mat3.inverted() @ world_q.to_matrix() @ posed_mat3).to_quaternion()
    pb.rotation_quaternion = pb.rotation_quaternion @ local_q


def rotate_posed_bone_world(
    rig: bpy.types.Object,
    bone_name: str,
    world_axis: Vector,
    angle_deg: float,
) -> None:
    """Rotate a child pose bone around a world-space axis using its current posed transform."""
    pb = rig.pose.bones.get(bone_name)
    if pb is None:
        return
    pb.rotation_mode = "QUATERNION"
    world_q = Quaternion(world_axis.normalized(), math.radians(angle_deg))
    posed_mat3 = (rig.matrix_world @ pb.matrix).to_3x3().normalized()
    local_q = (posed_mat3.inverted() @ world_q.to_matrix() @ posed_mat3).to_quaternion()
    pb.rotation_quaternion = pb.rotation_quaternion @ local_q


def group_vertex_indices(obj: bpy.types.Object, group_names: Sequence[str]) -> set[int]:
    """Return vertex indices belonging to any of `group_names` with weight > 0.1."""
    g_ids = {
        obj.vertex_groups[g].index
        for g in group_names
        if g in obj.vertex_groups
    }
    if not g_ids:
        return set()
    return {
        v.index
        for v in obj.data.vertices
        if any(g.group in g_ids and g.weight > 0.1 for g in v.groups)
    }


def group_bounds(obj: bpy.types.Object, group_name: str) -> tuple[Vector, Vector, Vector]:
    """Return `(center, min_vec, max_vec)` of vertices in `group_name`."""
    verts = group_vertex_indices(obj, (group_name,))
    pts = [obj.data.vertices[i].co for i in verts]
    mn = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
    mx = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))
    return (mn + mx) * 0.5, mn, mx


def create_posed_mpfb_body() -> tuple[
    bpy.types.Object,
    bpy.types.Object,
    dict[str, tuple[Vector, Vector, Vector]],
    dict[str, tuple[Vector, Vector]],
]:
    """Spawn male Alex Wen in MPFB2, pose, and return `(body_obj, tights_obj, landmarks, bone_segs)`."""
    HumanService = mpfb_import("mpfb.services.humanservice", "HumanService")
    TargetService = mpfb_import("mpfb.services.targetservice", "TargetService")
    LocationService = mpfb_import("mpfb.services.locationservice", "LocationService")
    HumanObjectProperties = mpfb_import("mpfb.entities.objectproperties", "HumanObjectProperties")

    macro = TargetService.get_default_macro_info_dict()
    macro["gender"] = 1.0       # 1.0 = MALE in MPFB2
    macro["age"] = 0.48         # Mid-20s
    macro["muscle"] = 0.65      # Athletic martial artist / former Navy SARC corpsman
    macro["weight"] = 0.54      # Ecto-endomorphic solid build
    macro["proportions"] = 0.54
    macro["height"] = 0.64
    macro["cupsize"] = 0.0
    macro["firmness"] = 0.70
    macro["race"] = {"asian": 0.90, "caucasian": 0.05, "african": 0.05}

    human = HumanService.create_human(
        mask_helpers=False,
        detailed_helpers=True,
        extra_vertex_groups=True,
        feet_on_ground=True,
        scale=0.1,
        macro_detail_dict=macro,
    )
    HumanObjectProperties.set_value("gender", 1.0, entity_reference=human)
    HumanObjectProperties.set_value("cupsize", 0.0, entity_reference=human)
    HumanObjectProperties.set_value("muscle", 0.65, entity_reference=human)
    HumanObjectProperties.set_value("weight", 0.54, entity_reference=human)
    HumanObjectProperties.set_value("asian", 0.90, entity_reference=human)
    HumanObjectProperties.set_value("caucasian", 0.05, entity_reference=human)
    HumanObjectProperties.set_value("african", 0.05, entity_reference=human)
    TargetService.reapply_macro_details(human)

    # Apply balanced masculine Chinese-Malay facial sculpting targets (structured jaw, natural tapered chin)
    tdir = LocationService.get_mpfb_data("targets")
    facial_targets = (
        ("head/head-rectangular.target.gz", 0.35),
        ("head/head-square.target.gz", 0.25),
        ("head/head-fat-decr.target.gz", 0.55),
        ("chin/chin-bones-incr.target.gz", 0.45),
        ("chin/chin-width-incr.target.gz", 0.22),
        ("chin/chin-prominent-incr.target.gz", 0.30),
        ("cheek/l-cheek-bones-incr.target.gz", 0.45),
        ("cheek/r-cheek-bones-incr.target.gz", 0.45),
        ("cheek/l-cheek-volume-decr.target.gz", 0.45),
        ("cheek/r-cheek-volume-decr.target.gz", 0.45),
        ("eyebrows/eyebrows-trans-forward.target.gz", 0.15),
        ("eyebrows/eyebrows-trans-down.target.gz", 0.22),
        ("mouth/mouth-lowerlip-volume-decr.target.gz", 0.50),
        ("mouth/mouth-lowerlip-height-decr.target.gz", 0.40),
        ("nose/nose-nostrils-width-decr.target.gz", 0.30),
        ("nose/nose-scale-depth-incr.target.gz", 0.25),
        ("nose/nose-point-width-decr.target.gz", 0.20),
    )
    for rel_path, weight in facial_targets:
        tpath = os.path.join(tdir, rel_path)
        if os.path.exists(tpath):
            TargetService.load_target(human, tpath, weight=weight)
    bpy.context.view_layer.update()

    rig = HumanService.add_builtin_rig(human, "default")

    bpy.ops.object.select_all(action="DESELECT")
    rig.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.ops.object.mode_set(mode="POSE")

    # Relaxed shoulders + arms hanging naturally alongside outer thighs (clear of hips)
    rotate_pose_bone_world(rig, "clavicle.L", Vector((0.0, 1.0, 0.0)), 3.0)
    rotate_pose_bone_world(rig, "clavicle.R", Vector((0.0, -1.0, 0.0)), 3.0)
    rotate_pose_bone_world(rig, "shoulder01.L", Vector((0.0, 1.0, 0.0)), 4.0)
    rotate_pose_bone_world(rig, "shoulder01.R", Vector((0.0, -1.0, 0.0)), 4.0)

    aim_bone_toward_dir(rig, "upperarm01.L", Vector((0.23, -0.02, -1.0)), blend=0.90)
    aim_bone_toward_dir(rig, "upperarm01.R", Vector((-0.23, -0.02, -1.0)), blend=0.90)
    bpy.context.view_layer.update()

    aim_posed_bone_toward_dir(rig, "lowerarm01.L", Vector((0.08, -0.05, -1.0)), blend=0.95)
    aim_posed_bone_toward_dir(rig, "lowerarm01.R", Vector((-0.08, -0.05, -1.0)), blend=0.95)
    bpy.context.view_layer.update()

    # Relaxed gentle finger curl (inward toward thighs using current posed arm frames)
    for finger_idx in range(2, 6):
        for seg, ang in ((1, 22.0), (2, 26.0)):
            rotate_posed_bone_world(rig, f"finger{finger_idx}-{seg}.L", Vector((0.0, 1.0, 0.0)), ang)
            rotate_posed_bone_world(rig, f"finger{finger_idx}-{seg}.R", Vector((0.0, -1.0, 0.0)), ang)
            bpy.context.view_layer.update()

    # Natural balanced shoulder-width standing stance
    rotate_pose_bone_world(rig, "upperleg01.L", Vector((0.0, 1.0, 0.0)), 6.0)
    rotate_pose_bone_world(rig, "upperleg01.R", Vector((0.0, -1.0, 0.0)), 6.0)
    rotate_pose_bone_world(rig, "lowerleg01.L", Vector((0.0, -1.0, 0.0)), 4.5)
    rotate_pose_bone_world(rig, "lowerleg01.R", Vector((0.0, 1.0, 0.0)), 4.5)
    rotate_pose_bone_world(rig, "foot.L", Vector((0.0, -1.0, 0.0)), 1.5)
    rotate_pose_bone_world(rig, "foot.R", Vector((0.0, 1.0, 0.0)), 1.5)

    bpy.context.view_layer.update()

    # Record exact world-space posed bone head/tail coordinates for sleeves & landmarks
    bone_segs: dict[str, tuple[Vector, Vector]] = {}
    for bn in (
        "clavicle.L", "clavicle.R",
        "shoulder01.L", "shoulder01.R",
        "upperarm01.L", "upperarm01.R",
        "upperarm02.L", "upperarm02.R",
        "lowerarm01.L", "lowerarm01.R",
    ):
        pb = rig.pose.bones.get(bn)
        if pb is not None:
            bone_segs[bn] = (
                (rig.matrix_world @ pb.head).copy(),
                (rig.matrix_world @ pb.tail).copy(),
            )

    bpy.ops.object.mode_set(mode="OBJECT")

    depsgraph = bpy.context.evaluated_depsgraph_get()
    eval_obj = human.evaluated_get(depsgraph)
    baked_mesh = bpy.data.meshes.new("Alex_MPFB_Posed_Mesh")
    temp_mesh = eval_obj.to_mesh()
    baked_mesh.from_pydata(
        [v.co.copy() for v in temp_mesh.vertices],
        [],
        [list(p.vertices) for p in temp_mesh.polygons],
    )
    eval_obj.to_mesh_clear()

    baked_obj = bpy.data.objects.new("Alex_MPFB_Posed", baked_mesh)
    bpy.context.scene.collection.objects.link(baked_obj)

    for vg in human.vertex_groups:
        new_vg = baked_obj.vertex_groups.new(name=vg.name)
        for v in human.data.vertices:
            for g in v.groups:
                if g.group == vg.index and g.weight > 0.0:
                    new_vg.add([v.index], g.weight, "REPLACE")

    bpy.data.objects.remove(human, do_unlink=True)
    bpy.data.objects.remove(rig, do_unlink=True)

    landmarks: dict[str, tuple[Vector, Vector, Vector]] = {}
    for gname in (
        "helper-l-eye",
        "helper-r-eye",
        "helper-hair",
        "foot.L",
        "foot.R",
        "joint-l-ankle",
        "joint-r-ankle",
        "joint-l-elbow",
        "joint-r-elbow",
        "joint-l-knee",
        "joint-r-knee",
        "joint-pelvis",
        "joint-head",
        "joint-jaw",
        "neck01",
    ):
        if gname in baked_obj.vertex_groups:
            landmarks[gname] = group_bounds(baked_obj, gname)

    # Preserve posed `helper-tights` shell (nipple-free, clean crew-neck collar loop)
    tights_verts = group_vertex_indices(baked_obj, ("helper-tights",))
    tights_mesh = baked_obj.data.copy()
    tights_mesh.name = "Alex_MPFB_Tights_Mesh"
    tights_obj = bpy.data.objects.new("Alex_MPFB_Tights", tights_mesh)
    bpy.context.scene.collection.objects.link(tights_obj)
    for vg in baked_obj.vertex_groups:
        new_vg = tights_obj.vertex_groups.new(name=vg.name)
        for v in baked_obj.data.vertices:
            for g in v.groups:
                if g.group == vg.index and g.weight > 0.0:
                    new_vg.add([v.index], g.weight, "REPLACE")
    bm_t = bmesh.new()
    bm_t.from_mesh(tights_obj.data)
    bmesh.ops.delete(bm_t, geom=[v for v in bm_t.verts if v.index not in tights_verts], context="VERTS")
    bm_t.to_mesh(tights_obj.data)
    bm_t.free()
    tights_obj.data.update()

    body_verts = group_vertex_indices(baked_obj, ("body",))
    bm = bmesh.new()
    bm.from_mesh(baked_obj.data)
    del_verts = [v for v in bm.verts if v.index not in body_verts]
    bmesh.ops.delete(bm, geom=del_verts, context="VERTS")
    bm.to_mesh(baked_obj.data)
    bm.free()
    baked_obj.data.update()

    return baked_obj, tights_obj, landmarks, bone_segs


def duplicate_subset_mesh(
    src_obj: bpy.types.Object,
    name: str,
    keep_vert_pred,
    collection: bpy.types.Collection,
    mat: bpy.types.Material,
) -> bpy.types.Object:
    """Duplicate `src_obj` keeping faces whose vertices satisfy `keep_vert_pred(v_idx, co)`."""
    mesh_copy = src_obj.data.copy()
    mesh_copy.name = f"{name}_Mesh"
    obj = bpy.data.objects.new(name, mesh_copy)
    collection.objects.link(obj)
    obj.data.materials.clear()
    obj.data.materials.append(mat)

    keep_indices = {
        v.index
        for v in obj.data.vertices
        if keep_vert_pred(v.index, v.co)
    }

    bm = bmesh.new()
    bm.from_mesh(obj.data)
    del_faces = [f for f in bm.faces if not all(v.index in keep_indices for v in f.verts)]
    bmesh.ops.delete(bm, geom=del_faces, context="FACES")
    orphan_verts = [v for v in bm.verts if not v.link_faces]
    bmesh.ops.delete(bm, geom=orphan_verts, context="VERTS")
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()
    return obj


def bisect_mesh_horizontal(obj: bpy.types.Object, z_plane: float, keep_above: bool) -> None:
    """Slice `obj` cleanly along horizontal plane `Z = z_plane`."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    geom = list(bm.verts) + list(bm.edges) + list(bm.faces)
    bmesh.ops.bisect_plane(
        bm,
        geom=geom,
        plane_co=(0.0, 0.0, z_plane),
        plane_no=(0.0, 0.0, 1.0),
        clear_inner=keep_above,
        clear_outer=not keep_above,
    )
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def inflate_along_normals(obj: bpy.types.Object, offset_fn) -> None:
    """Offset mesh vertices outward along normals by `offset_fn(co)` (in meters)."""
    obj.data.update()
    norms = [v.normal.copy() for v in obj.data.vertices]
    for v, n in zip(obj.data.vertices, norms):
        v.co += n * offset_fn(v.co)
    obj.data.update()


def loft_profile_rings(
    name: str,
    rings: Sequence[tuple[float, float, float, float, float]],
    segments: int,
    collection: bpy.types.Collection,
    mat: bpy.types.Material,
    shape_fn=None,
    cap_bottom: bool = True,
    cap_top: bool = True,
    subdiv: int = 1,
) -> bpy.types.Object:
    """Loft an organic mesh through `(cx, cy, cz, rx, ry)` cross-section rings."""
    bm = bmesh.new()
    ring_verts: list[list[bmesh.types.BMVert]] = []
    for cx, cy, cz, rx, ry in rings:
        verts = []
        for i in range(segments):
            th = (2.0 * math.pi * i) / segments
            ux, uy = (math.cos(th), math.sin(th)) if shape_fn is None else shape_fn(th)
            verts.append(bm.verts.new((cx + rx * ux, cy + ry * uy, cz)))
        ring_verts.append(verts)

    for r_idx in range(len(ring_verts) - 1):
        r0, r1 = ring_verts[r_idx], ring_verts[r_idx + 1]
        for i in range(segments):
            j = (i + 1) % segments
            bm.faces.new((r0[i], r0[j], r1[j], r1[i]))

    if cap_bottom:
        c_bot = bm.verts.new((rings[0][0], rings[0][1], rings[0][2]))
        for i in range(segments):
            j = (i + 1) % segments
            bm.faces.new((ring_verts[0][j], ring_verts[0][i], c_bot))

    if cap_top:
        c_top = bm.verts.new((rings[-1][0], rings[-1][1], rings[-1][2]))
        for i in range(segments):
            j = (i + 1) % segments
            bm.faces.new((ring_verts[-1][i], ring_verts[-1][j], c_top))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    obj.data.materials.append(mat)
    if subdiv > 0:
        mod = obj.modifiers.new("Subsurf", "SUBSURF")
        mod.levels = subdiv
        mod.render_levels = subdiv
    return apply_all_modifiers(obj, smooth=True, recalc_normals=True)


def loft_tube_along_points(
    name: str,
    points: Sequence[tuple[Vector, float, float]],
    segments: int,
    collection: bpy.types.Collection,
    mat: bpy.types.Material,
    cap_start: bool = True,
    cap_end: bool = True,
    subdiv: int = 1,
) -> bpy.types.Object:
    """Loft an elliptical tube along a 3D spine `(center_vec, rx, ry)`."""
    bm = bmesh.new()
    ring_verts: list[list[bmesh.types.BMVert]] = []
    n = len(points)

    prev_ref = Vector((0.0, -1.0, 0.0))
    for idx, (pt, rx, ry) in enumerate(points):
        if idx == 0:
            tangent = (points[1][0] - pt).normalized()
        elif idx == n - 1:
            tangent = (pt - points[n - 2][0]).normalized()
        else:
            tangent = (points[idx + 1][0] - points[idx - 1][0]).normalized()

        if abs(tangent.dot(prev_ref)) > 0.92:
            prev_ref = Vector((1.0, 0.0, 0.0))
        side_x = tangent.cross(prev_ref).normalized()
        side_y = side_x.cross(tangent).normalized()
        prev_ref = side_y

        verts: list[bmesh.types.BMVert] = []
        for i in range(segments):
            theta = (2.0 * math.pi * i) / segments
            offset = side_x * (rx * math.cos(theta)) + side_y * (ry * math.sin(theta))
            verts.append(bm.verts.new(pt + offset))
        ring_verts.append(verts)

    for r_idx in range(n - 1):
        r0 = ring_verts[r_idx]
        r1 = ring_verts[r_idx + 1]
        for i in range(segments):
            j = (i + 1) % segments
            bm.faces.new((r0[i], r0[j], r1[j], r1[i]))

    if cap_start:
        c_start = bm.verts.new(points[0][0])
        for i in range(segments):
            j = (i + 1) % segments
            bm.faces.new((ring_verts[0][j], ring_verts[0][i], c_start))

    if cap_end:
        c_end = bm.verts.new(points[-1][0])
        for i in range(segments):
            j = (i + 1) % segments
            bm.faces.new((ring_verts[-1][i], ring_verts[-1][j], c_end))

    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    obj.data.materials.append(mat)
    if subdiv > 0:
        mod = obj.modifiers.new("Subsurf", "SUBSURF")
        mod.levels = subdiv
        mod.render_levels = subdiv
    return apply_all_modifiers(obj, smooth=True, recalc_normals=True)


def torso_tee_contour(theta: float, chest_bias: float = 0.0, drape_amp: float = 0.0) -> tuple[float, float]:
    """Athletic male t-shirt cross-section (Front is -Y, Back is +Y)."""
    ct, st = math.cos(theta), math.sin(theta)
    front_bulge = 1.0 + chest_bias * max(0.0, -st) ** 1.4
    back_groove = 1.0 - 0.05 * max(0.0, st) * math.exp(-5.0 * ct * ct)
    ripple = 1.0 + drape_amp * math.cos(6.0 * theta + 0.5)
    return ct * (1.0 + 0.04 * ct * ct) * ripple, st * front_bulge * back_groove * ripple


def build_tshirt_from_tights(
    tights_obj: bpy.types.Object,
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    bone_segs: dict[str, tuple[Vector, Vector]],
    collection: bpy.types.Collection,
    palette: dict[str, bpy.types.Material],
) -> tuple[list[bpy.types.Object], float, float]:
    """Extract ONE seamless crew-neck t-shirt from MPFB2's posed `helper-tights` shell.

    `helper-tights` is designed in MakeHuman specifically for close-fitting tops:
    it bridges the trapezius/clavicle/deltoid cleanly, has zero nipple bumps,
    and terminates at a smooth crew-neck loop around the base of the neck.

    Returns `(tee_objects, hem_z, sleeve_cut_z)`.
    """
    neck_bot_z = landmarks["neck01"][1].z
    pelvis_z = landmarks["joint-pelvis"][0].z
    hem_z = pelvis_z - 0.005

    # Compute left/right sleeve cuff planes from real posed upperarm bones
    ua1_l_h, _ = bone_segs["upperarm01.L"]
    ua2_l_h, ua2_l_t = bone_segs["upperarm02.L"]
    arm_dir_l = (ua2_l_t - ua1_l_h).normalized()
    p_cuff_l = ua2_l_h.lerp(ua2_l_t, 0.32)

    ua1_r_h, _ = bone_segs["upperarm01.R"]
    ua2_r_h, ua2_r_t = bone_segs["upperarm02.R"]
    arm_dir_r = (ua2_r_t - ua1_r_h).normalized()
    p_cuff_r = ua2_r_h.lerp(ua2_r_t, 0.32)
    sleeve_cut_z = max(p_cuff_l.z, p_cuff_r.z)

    exclude_vids = group_vertex_indices(
        tights_obj,
        (
            "lowerarm01.L", "lowerarm02.L", "wrist.L",
            "lowerarm01.R", "lowerarm02.R", "wrist.R",
            "metacarpal1.L", "metacarpal2.L", "metacarpal3.L", "metacarpal4.L",
            "metacarpal1.R", "metacarpal2.R", "metacarpal3.R", "metacarpal4.R",
            "finger1-1.L", "finger1-2.L", "finger1-3.L",
            "finger2-1.L", "finger2-2.L", "finger2-3.L",
            "finger3-1.L", "finger3-2.L", "finger3-3.L",
            "finger4-1.L", "finger4-2.L", "finger4-3.L",
            "finger5-1.L", "finger5-2.L", "finger5-3.L",
            "finger1-1.R", "finger1-2.R", "finger1-3.R",
            "finger2-1.R", "finger2-2.R", "finger2-3.R",
            "finger3-1.R", "finger3-2.R", "finger3-3.R",
            "finger4-1.R", "finger4-2.R", "finger4-3.R",
            "finger5-1.R", "finger5-2.R", "finger5-3.R",
        ),
    )

    # Keep `helper-tights` above hem_z - 0.05 (leaving its native crew-neck loop untouched!)
    tee = duplicate_subset_mesh(
        tights_obj,
        "Torso_TShirt",
        lambda vid, co: (vid not in exclude_vids) and (co.z >= hem_z - 0.05),
        collection,
        palette["tee"],
    )

    # 1. Clean horizontal bottom hem cut
    bisect_mesh_horizontal(tee, hem_z, keep_above=True)

    # 2. Cut perpendicular sleeves via bmesh (keeping `helper-tights`'s native neck loop 100% manifold!)
    bm = bmesh.new()
    bm.from_mesh(tee.data)
    for side_sign, p_cuff, arm_dir in ((1.0, p_cuff_l, arm_dir_l), (-1.0, p_cuff_r, arm_dir_r)):
        arm_faces = [f for f in bm.faces if all(v.co.x * side_sign > 0.175 for v in f.verts)]
        arm_verts = list({v for f in arm_faces for v in f.verts})
        arm_edges = list({e for f in arm_faces for e in f.edges})
        if arm_faces:
            bmesh.ops.bisect_plane(
                bm,
                geom=arm_verts + arm_edges + arm_faces,
                plane_co=p_cuff,
                plane_no=arm_dir,
                clear_outer=True,
            )
    bm.to_mesh(tee.data)
    bm.free()
    tee.data.update()

    # 2b. Smoothly lower the native front crew-neck loop into a U-curve & conform to tights_obj
    for v in tee.data.vertices:
        if v.co.z > neck_bot_z - 0.06 and abs(v.co.x) < 0.105:
            wx = max(0.0, 1.0 - (abs(v.co.x) / 0.105) ** 2)
            wy = max(0.0, min(1.0, (0.015 - v.co.y) / 0.095))
            wz = max(0.0, min(1.0, (v.co.z - (neck_bot_z - 0.06)) / 0.085))
            dz = 0.032 * wx * wy * wz
            if dz > 0.0:
                v.co.z -= dz
                v.co.y -= 0.35 * dz
    tee.data.update()

    sw = tee.modifiers.new("NeckConform", "SHRINKWRAP")
    sw.target = tights_obj
    sw.wrap_method = "NEAREST_SURFACEPOINT"
    sw.wrap_mode = "OUTSIDE_SURFACE"
    sw.offset = 0.001
    apply_all_modifiers(tee, smooth=False, recalc_normals=False)

    # Relax waist & lower torso outward so cotton tee hangs straight (no back-hem flare!)
    chest_z = bone_segs["shoulder01.L"][1].z - 0.12
    for v in tee.data.vertices:
        if v.co.z < chest_z and abs(v.co.x) < 0.21:
            t = max(0.0, min(1.0, (chest_z - v.co.z) / max(0.01, chest_z - hem_z)))
            cx, cy = 0.0, -0.020
            v.co.x = cx + (v.co.x - cx) * (1.0 + 0.12 * t)
            y_scale = 1.08 if v.co.y < -0.015 else 1.01
            v.co.y = cy + (v.co.y - cy) * (1.0 + (y_scale - 1.0) * t)
    tee.data.update()

    # Subtle outward cotton drape offset over `helper-tights`
    inflate_along_normals(
        tee,
        lambda co: 0.0055 + (0.0045 if abs(co.x) > 0.175 else 0.0),
    )

    sm = tee.modifiers.new("Smooth", "SMOOTH")
    sm.factor = 0.45
    sm.iterations = 4
    sub = tee.modifiers.new("Subsurf", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 1
    apply_all_modifiers(tee, smooth=True, recalc_normals=False)

    # Extract crew-neck collar ring directly from `tee`'s top neckline boundary faces (zero gap!)
    bm_col = bmesh.new()
    bm_col.from_mesh(tee.data)
    top_bound_verts = {
        v.index
        for v in bm_col.verts
        if v.is_boundary and v.co.z > neck_bot_z - 0.07 and abs(v.co.x) < 0.115
    }
    collar_face_verts = {
        v.index
        for f in bm_col.faces
        if any(v.index in top_bound_verts for v in f.verts)
        for v in f.verts
    }
    bm_col.free()

    collar = duplicate_subset_mesh(
        tee,
        "Torso_TShirt_Collar",
        lambda vid, _co: vid in collar_face_verts,
        collection,
        palette["tee_trim"],
    )
    inflate_along_normals(collar, lambda _co: 0.0012)
    sol_c = collar.modifiers.new("Solidify", "SOLIDIFY")
    sol_c.thickness = 0.0062
    sol_c.offset = 1.0
    sol_c.use_rim = True
    apply_all_modifiers(collar, smooth=True, recalc_normals=False)

    # Solidify main tee mesh after extracting collar
    sol = tee.modifiers.new("Solidify", "SOLIDIFY")
    sol.thickness = 0.0045
    sol.offset = 1.0
    sol.use_rim = True
    apply_all_modifiers(tee, smooth=True, recalc_normals=False)

    return [tee, collar], hem_z, sleeve_cut_z


def build_loose_jeans_from_body(
    body_obj: bpy.types.Object,
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    hem_z: float,
    collection: bpy.types.Collection,
    palette: dict[str, bpy.types.Material],
) -> list[bpy.types.Object]:
    """Build ONE closed-top pelvis-to-ankle loose straight-leg faded blue jeans mesh."""
    ankle_z = min(landmarks["joint-l-ankle"][0].z, landmarks["joint-r-ankle"][0].z)
    cuff_z = ankle_z + 0.012   # sits cleanly at shoe collar top so tongue & laces stay visible
    waist_z = hem_z + 0.115    # extends ~11.5 cm ABOVE shirt hem -> ZERO hip/waist void!
    crotch_z = hem_z - 0.055

    arm_hand_vids = group_vertex_indices(
        body_obj,
        (
            "upperarm01.L", "upperarm02.L", "lowerarm01.L", "lowerarm02.L", "wrist.L",
            "upperarm01.R", "upperarm02.R", "lowerarm01.R", "lowerarm02.R", "wrist.R",
            "metacarpal1.L", "metacarpal2.L", "metacarpal3.L", "metacarpal4.L",
            "metacarpal1.R", "metacarpal2.R", "metacarpal3.R", "metacarpal4.R",
            "finger1-1.L", "finger1-2.L", "finger1-3.L",
            "finger2-1.L", "finger2-2.L", "finger2-3.L",
            "finger3-1.L", "finger3-2.L", "finger3-3.L",
            "finger4-1.L", "finger4-2.L", "finger4-3.L",
            "finger5-1.L", "finger5-2.L", "finger5-3.L",
            "finger1-1.R", "finger1-2.R", "finger1-3.R",
            "finger2-1.R", "finger2-2.R", "finger2-3.R",
            "finger3-1.R", "finger3-2.R", "finger3-3.R",
            "finger4-1.R", "finger4-2.R", "finger4-3.R",
            "finger5-1.R", "finger5-2.R", "finger5-3.R",
        ),
    )

    jeans = duplicate_subset_mesh(
        body_obj,
        "Legs_Jeans",
        lambda vid, co: (vid not in arm_hand_vids) and (cuff_z - 0.05 <= co.z <= waist_z + 0.06),
        collection,
        palette["jeans"],
    )

    bisect_mesh_horizontal(jeans, waist_z, keep_above=False)
    bisect_mesh_horizontal(jeans, cuff_z, keep_above=True)

    # Cap top waistband loop cleanly so no hollow top is ever visible
    bm_cap = bmesh.new()
    bm_cap.from_mesh(jeans.data)
    top_verts = [v for v in bm_cap.verts if abs(v.co.z - waist_z) < 0.002]
    if len(top_verts) >= 3:
        center_top = bm_cap.verts.new(
            (
                sum(v.co.x for v in top_verts) / len(top_verts),
                sum(v.co.y for v in top_verts) / len(top_verts),
                waist_z,
            )
        )
        top_edges = [e for e in bm_cap.edges if e.is_boundary and all(v in top_verts for v in e.verts)]
        for e in top_edges:
            bm_cap.faces.new((e.verts[0], e.verts[1], center_top))
        bmesh.ops.recalc_face_normals(bm_cap, faces=bm_cap.faces)
    bm_cap.to_mesh(jeans.data)
    bm_cap.free()
    jeans.data.update()

    # Precompute true posed leg (cx(z), cy(z)) centroids per Z-slice below crotch_z
    n_bins = 16
    z_span = max(0.05, crotch_z - cuff_z)
    bin_sums = {side: [[0.0, 0.0, 0] for _ in range(n_bins)] for side in (1, -1)}
    for v in jeans.data.vertices:
        if v.co.z <= crotch_z:
            side = 1 if v.co.x >= 0.0 else -1
            b = max(0, min(n_bins - 1, int(((v.co.z - cuff_z) / z_span) * n_bins)))
            bin_sums[side][b][0] += v.co.x
            bin_sums[side][b][1] += v.co.y
            bin_sums[side][b][2] += 1

    bin_centers = {}
    for side in (1, -1):
        centers = []
        last = (side * 0.14, -0.02)
        for sx, sy, cnt in bin_sums[side]:
            if cnt > 0:
                last = (sx / cnt, sy / cnt)
            centers.append(last)
        bin_centers[side] = centers

    # Expand legs into relaxed straight-leg denim silhouette
    for v in jeans.data.vertices:
        z = v.co.z
        if z > crotch_z:
            # Keep waistband tucked cleanly inside Torso_TShirt
            t = max(0.0, min(1.0, (z - crotch_z) / max(0.01, waist_z - crotch_z)))
            expand = 0.008 * (1.0 - 0.85 * t)
            v.co += v.normal * expand
        else:
            side = 1 if v.co.x >= 0.0 else -1
            frac = max(0.0, min(1.0, (z - cuff_z) / z_span))
            b = max(0, min(n_bins - 1, int(frac * n_bins)))
            cx, cy = bin_centers[side][b]
            dx, dy = v.co.x - cx, v.co.y - cy
            r = math.hypot(dx, dy)
            target_r = 0.075 + 0.017 * frac
            if r > 1e-4:
                new_r = max(r + 0.008, target_r)
                blended_r = r * 0.25 + new_r * 0.75
                v.co.x = cx + (dx / r) * blended_r
                v.co.y = cy + (dy / r) * blended_r
            v.co.y += 0.002 * math.sin(z * 28.0)

    jeans.data.update()
    sm = jeans.modifiers.new("Smooth", "SMOOTH")
    sm.factor = 0.65
    sm.iterations = 10
    sol = jeans.modifiers.new("Solidify", "SOLIDIFY")
    sol.thickness = 0.0045
    sol.offset = 1.0
    sol.use_rim = True
    apply_all_modifiers(jeans, smooth=True, recalc_normals=False)

    # Subtle darker indigo denim cuffs + side outseam piping strips
    extras: list[bpy.types.Object] = []
    for side_tag, sx in (("L", 1.0), ("R", -1.0)):
        cuff = duplicate_subset_mesh(
            jeans,
            f"Legs_Jeans_Cuff_{side_tag}",
            lambda _vid, co, s=sx: (co.x * s > 0.02) and (cuff_z - 0.006 <= co.z <= cuff_z + 0.022),
            collection,
            palette["jeans_seam"],
        )
        inflate_along_normals(cuff, lambda _co: 0.0018)
        apply_all_modifiers(cuff, smooth=True, recalc_normals=False)
        extras.append(cuff)

        # Outer leg denim seam strip from hem_z down to cuff_z
        seam_pts = []
        for step in range(8):
            frac = step / 7.0
            sz = hem_z - 0.02 - frac * (hem_z - 0.03 - cuff_z)
            # Find outermost X at this Z slice on that leg
            slice_pts = [
                v.co for v in jeans.data.vertices
                if abs(v.co.z - sz) < 0.025 and v.co.x * sx > 0.05
            ]
            if slice_pts:
                outer_v = max(slice_pts, key=lambda p: p.x * sx)
                seam_pts.append((Vector((outer_v.x + sx * 0.001, outer_v.y, sz)), 0.0035, 0.0055))
        if len(seam_pts) >= 3:
            extras.append(
                loft_tube_along_points(
                    f"Legs_Jeans_Outseam_{side_tag}",
                    seam_pts,
                    segments=8,
                    collection=collection,
                    mat=palette["jeans_seam"],
                    subdiv=0,
                )
            )

    return [jeans, *extras]


def create_uv_sphere(
    name: str,
    center: tuple[float, float, float],
    radii: tuple[float, float, float],
    rot_euler: tuple[float, float, float],
    collection: bpy.types.Collection,
    mat: bpy.types.Material,
    u_seg: int = 16,
    v_seg: int = 12,
) -> bpy.types.Object:
    """Create a scaled/rotated UV sphere mesh."""
    bm = bmesh.new()
    bmesh.ops.create_uvsphere(bm, u_segments=u_seg, v_segments=v_seg, radius=1.0)
    scale_m = Matrix.Diagonal((radii[0], radii[1], radii[2], 1.0))
    rot_m = Euler(rot_euler, "XYZ").to_matrix().to_4x4()
    trans_m = Matrix.Translation(Vector(center))
    bmesh.ops.transform(bm, matrix=trans_m @ rot_m @ scale_m, verts=bm.verts)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    obj.data.materials.append(mat)
    for p in obj.data.polygons:
        p.use_smooth = True
    return obj


def create_beveled_box(
    name: str,
    center: tuple[float, float, float],
    size: tuple[float, float, float],
    rot_euler: tuple[float, float, float],
    bevel_width: float,
    collection: bpy.types.Collection,
    mat: bpy.types.Material,
    smooth: bool = True,
) -> bpy.types.Object:
    """Create a softly beveled cuboid mesh."""
    bm = bmesh.new()
    bmesh.ops.create_cube(bm, size=1.0)
    bmesh.ops.transform(bm, matrix=Matrix.Diagonal((size[0], size[1], size[2], 1.0)), verts=bm.verts)
    if bevel_width > 0.0:
        bmesh.ops.bevel(
            bm,
            geom=list(bm.edges),
            offset=bevel_width,
            segments=2,
            affect="EDGES",
            clamp_overlap=True,
        )
    rot_m = Euler(rot_euler, "XYZ").to_matrix().to_4x4()
    trans_m = Matrix.Translation(Vector(center))
    bmesh.ops.transform(bm, matrix=trans_m @ rot_m, verts=bm.verts)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    mesh = bpy.data.meshes.new(name)
    bm.to_mesh(mesh)
    bm.free()
    obj = bpy.data.objects.new(name, mesh)
    collection.objects.link(obj)
    obj.data.materials.append(mat)
    if smooth:
        for p in obj.data.polygons:
            p.use_smooth = True
    return obj


def foot_contour(theta: float) -> tuple[float, float]:
    """Asymmetric trail shoe footprint: broader rounded forefoot (-Y), tapered heel (+Y)."""
    ct, st = math.cos(theta), math.sin(theta)
    if st < 0.0:
        return ct * (1.03 - 0.12 * st * st), st * 1.04
    return ct * (0.83 - 0.06 * st), st * 0.96


def build_trail_shoes_at_feet(
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    collection: bpy.types.Collection,
    palette: dict[str, bpy.types.Material],
) -> list[bpy.types.Object]:
    """Build detailed trail shoes (`Legs_*`) with lugged sole, toe cap, padded collar, tongue & laces."""
    objs: list[bpy.types.Object] = []
    for side_tag, s in (("L", 1.0), ("R", -1.0)):
        _, f_min, f_max = landmarks[f"foot.{side_tag}"]
        ankle_center = landmarks[f"joint-{side_tag.lower()}-ankle"][0]
        foot_len = max(0.25, f_max.y - f_min.y + 0.025)
        foot_w = max(0.052, (f_max.x - f_min.x) * 0.56)
        base_z = f_min.z - 0.008
        pivot = Vector((ankle_center.x + s * 0.003, (f_min.y + f_max.y) * 0.5 - 0.018, base_z))
        yaw = s * math.radians(-6.5)

        def place(obj: bpy.types.Object) -> bpy.types.Object:
            rot = Euler((0.0, 0.0, yaw), "XYZ").to_matrix().to_4x4()
            obj.data.transform(Matrix.Translation(pivot) @ rot)
            obj.data.update()
            return obj

        ry = foot_len * 0.52
        # 1. Sculpted charcoal-brown lugged outsole (0.000..0.029 m)
        sole = loft_profile_rings(
            f"Legs_Shoe_Sole_{side_tag}",
            [
                (0.0, 0.000, 0.000, foot_w * 0.90, ry * 0.95),
                (0.0, -0.002, 0.008, foot_w * 1.03, ry * 1.02),
                (0.0, -0.002, 0.021, foot_w * 1.04, ry * 1.03),
                (0.0, 0.000, 0.029, foot_w * 0.98, ry * 0.97),
            ],
            segments=24,
            collection=collection,
            mat=palette["shoe_sole"],
            shape_fn=foot_contour,
            subdiv=1,
        )
        objs.append(place(sole))

        for idx, y_off in enumerate((-0.088, -0.044, 0.000, 0.048, 0.088)):
            lug = create_beveled_box(
                f"Legs_Shoe_Lug_{side_tag}_{idx}",
                (0.0, y_off, 0.005),
                (foot_w * 1.48, 0.019, 0.010),
                (0.0, 0.0, 0.0),
                0.002,
                collection,
                palette["shoe_sole"],
                smooth=False,
            )
            objs.append(place(lug))

        # 2. Tan-brown suede main upper
        upper = loft_profile_rings(
            f"Legs_Shoe_Upper_{side_tag}",
            [
                (0.0, 0.000, 0.024, foot_w * 0.96, ry * 0.97),
                (0.0, -0.004, 0.042, foot_w * 0.97, ry * 0.95),
                (0.0, 0.012, 0.060, foot_w * 0.88, ry * 0.78),
                (0.0, 0.036, 0.078, foot_w * 0.78, ry * 0.52),
                (0.0, 0.044, 0.090, foot_w * 0.74, ry * 0.42),
            ],
            segments=24,
            collection=collection,
            mat=palette["shoe_suede"],
            shape_fn=foot_contour,
            subdiv=1,
        )
        objs.append(place(upper))

        # Darker tan suede toe bumper / mudguard cap for clear layered trail-shoe look
        toecap = create_uv_sphere(
            f"Legs_Shoe_ToeCap_{side_tag}",
            (0.0, -ry * 0.72, 0.036),
            (foot_w * 0.78, ry * 0.34, 0.018),
            (math.radians(-8.0), 0.0, 0.0),
            collection,
            palette["shoe_cap"],
            u_seg=14,
            v_seg=10,
        )
        objs.append(place(toecap))

        # 3. Medium-brown padded collar & visible sloped tongue + 5 crossed lace bars
        collar = loft_profile_rings(
            f"Legs_Shoe_Collar_{side_tag}",
            [
                (0.0, 0.044, 0.072, foot_w * 0.80, ry * 0.44),
                (0.0, 0.046, 0.088, foot_w * 0.78, ry * 0.41),
                (0.0, 0.044, 0.098, foot_w * 0.68, ry * 0.35),
            ],
            segments=18,
            collection=collection,
            mat=palette["shoe_mesh"],
            subdiv=1,
        )
        objs.append(place(collar))

        tongue = create_beveled_box(
            f"Legs_Shoe_Tongue_{side_tag}",
            (0.0, -0.022, 0.078),
            (foot_w * 0.76, 0.064, 0.022),
            (math.radians(30.0), 0.0, 0.0),
            0.007,
            collection,
            palette["shoe_mesh"],
        )
        objs.append(place(tongue))

        for i, (ly, lz) in enumerate((
            (-0.060, 0.063),
            (-0.044, 0.072),
            (-0.028, 0.081),
            (-0.012, 0.090),
            (0.004, 0.098),
        )):
            lace = create_beveled_box(
                f"Legs_Shoe_Lace_{side_tag}_{i}",
                (0.0, ly, lz),
                (foot_w * 0.88, 0.007, 0.006),
                (math.radians(28.0), 0.0, 0.0),
                0.002,
                collection,
                palette["shoe_mesh"],
            )
            objs.append(place(lace))

    return objs


def build_eyes_brows_hair_and_skin(
    body_obj: bpy.types.Object,
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    sleeve_cut_z: float,
    collection: bpy.types.Collection,
    palette: dict[str, bpy.types.Material],
) -> list[bpy.types.Object]:
    """Build MPFB2-aligned eyes, proud eyebrows, textured jet-black hair, and full arms/head skin."""
    objs: list[bpy.types.Object] = []

    # 1. Visible Head & Neck Skin extending cleanly inside `helper-tights` neckline (zero collarbone slit or dropped nose quads)
    neck_z = landmarks["neck01"][1].z
    body_vids = group_vertex_indices(body_obj, ("body",))
    head_vids = group_vertex_indices(
        body_obj,
        ("head", "jaw", "neck01", "neck02", "neck03", "ears", "lips", "scalp"),
    )
    head_skin = duplicate_subset_mesh(
        body_obj,
        "Head_Skin",
        lambda vid, co: (vid in body_vids)
        and (
            (co.z > neck_z + 0.040)
            or ((vid in head_vids) and co.z >= neck_z - 0.015 and abs(co.x) < 0.075)
        ),
        collection,
        palette["skin"],
    )
    head_skin.data.materials.append(palette["lips"])
    lip_vids_orig = group_vertex_indices(body_obj, ("lips",))
    lip_pts = [body_obj.data.vertices[i].co.copy() for i in lip_vids_orig]
    if lip_pts:
        lip_vert_set = {
            v.index
            for v in head_skin.data.vertices
            if any((v.co - lp).length < 0.0015 for lp in lip_pts)
        }
        for poly in head_skin.data.polygons:
            if sum(1 for vi in poly.vertices if vi in lip_vert_set) >= 3:
                poly.material_index = 1

    apply_all_modifiers(head_skin, smooth=True, recalc_normals=False)
    objs.append(head_skin)

    # 2. Eyes seated in MPFB2 sockets + surface-conformed tapered eyebrows hugging the brow ridge
    for side_tag in ("L", "R"):
        eye_c, eye_mn, eye_mx = landmarks[f"helper-{side_tag.lower()}-eye"]
        eye_r = max(0.011, (eye_mx.x - eye_mn.x) * 0.46)
        eye_cy = eye_c.y + 0.0008
        objs.append(
            create_uv_sphere(
                f"Head_Eye_Sclera_{side_tag}",
                (eye_c.x, eye_cy, eye_c.z),
                (eye_r, eye_r, eye_r),
                (0.0, 0.0, 0.0),
                collection,
                palette["sclera"],
                u_seg=16,
                v_seg=12,
            )
        )
        objs.append(
            create_uv_sphere(
                f"Head_Eye_Iris_{side_tag}",
                (eye_c.x, eye_cy - eye_r * 0.76, eye_c.z),
                (eye_r * 0.54, eye_r * 0.28, eye_r * 0.54),
                (0.0, 0.0, 0.0),
                collection,
                palette["iris"],
                u_seg=14,
                v_seg=10,
            )
        )
        objs.append(
            create_uv_sphere(
                f"Head_Eye_Pupil_{side_tag}",
                (eye_c.x, eye_cy - eye_r * 0.90, eye_c.z),
                (eye_r * 0.25, eye_r * 0.14, eye_r * 0.25),
                (0.0, 0.0, 0.0),
                collection,
                palette["pupil"],
                u_seg=12,
                v_seg=8,
            )
        )
        objs.append(
            create_uv_sphere(
                f"Head_Eye_Catchlight_{side_tag}",
                (eye_c.x + eye_r * 0.20, eye_cy - eye_r * 0.96, eye_c.z + eye_r * 0.20),
                (eye_r * 0.06, eye_r * 0.04, eye_r * 0.06),
                (0.0, 0.0, 0.0),
                collection,
                palette["catchlight"],
                u_seg=10,
                v_seg=6,
            )
        )

        brow_z = eye_c.z + 0.0160
        brow_verts = [
            v.co
            for v in head_skin.data.vertices
            if abs(v.co.x - eye_c.x) < 0.018
            and abs(v.co.z - brow_z) < 0.007
            and v.co.y < eye_c.y
        ]
        brow_surface_y = min((p.y for p in brow_verts), default=(eye_c.y - 0.022))
        s = 1.0 if side_tag == "L" else -1.0
        brow_cx = eye_c.x + s * 0.001
        brow_w, brow_h = 0.030, 0.0026
        bm_brow = bmesh.new()
        cols = []
        for ix in range(7):
            u = ix / 6.0
            x = brow_cx + s * (u - 0.5) * brow_w
            h_scale = 0.75 + 0.25 * math.sin(u * math.pi) - 0.30 * (u ** 1.6)
            z_arch = 0.0012 * math.sin(u * math.pi)
            y = (brow_surface_y - 0.004) + 0.003 * (u ** 1.5)
            v_bot = bm_brow.verts.new((x, y, brow_z + z_arch - 0.5 * brow_h * h_scale))
            v_top = bm_brow.verts.new((x, y, brow_z + z_arch + 0.5 * brow_h * h_scale))
            cols.append((v_bot, v_top))
        for ix in range(6):
            v0, v3 = cols[ix]
            v1, v2 = cols[ix + 1]
            face = bm_brow.faces.new((v0, v1, v2, v3) if s > 0.0 else (v1, v0, v3, v2))
            face.normal_update()
            if face.normal.y > 0.0:
                face.normal_flip()
        brow_mesh = bpy.data.meshes.new(f"Head_Eyebrow_{side_tag}")
        bm_brow.to_mesh(brow_mesh)
        bm_brow.free()
        brow_obj = bpy.data.objects.new(f"Head_Eyebrow_{side_tag}", brow_mesh)
        collection.objects.link(brow_obj)
        brow_obj.data.materials.append(palette["hair"])

        sw_brow = brow_obj.modifiers.new("BrowConform", "SHRINKWRAP")
        sw_brow.target = head_skin
        sw_brow.wrap_method = "NEAREST_SURFACEPOINT"
        sw_brow.wrap_mode = "OUTSIDE_SURFACE"
        sw_brow.offset = 0.0005

        sol_brow = brow_obj.modifiers.new("BrowSolidify", "SOLIDIFY")
        sol_brow.thickness = 0.0010
        sol_brow.offset = 1.0
        sol_brow.use_rim = True
        apply_all_modifiers(brow_obj, smooth=True, recalc_normals=True)
        objs.append(brow_obj)

    # 3. Hair cap conformed to MPFB2 `scalp` + tapered natural male short-crop silhouette
    scalp_vids = group_vertex_indices(body_obj, ("scalp",))
    hairline_z = max(landmarks["helper-l-eye"][0].z, landmarks["helper-r-eye"][0].z) + 0.042
    hair_cap = duplicate_subset_mesh(
        body_obj,
        "Head_Hair_Cap",
        lambda vid, co: (vid in scalp_vids) or (vid in head_vids and co.z >= hairline_z and co.y > -0.148),
        collection,
        palette["hair"],
    )
    head_top_z = max(v.co.z for v in hair_cap.data.vertices)
    inflate_along_normals(
        hair_cap,
        lambda co: 0.0040 + 0.0095 * max(0.0, min(1.0, (co.z - (head_top_z - 0.09)) / 0.09)),
    )
    # Subtle structured width at temples only, tapering cleanly inward toward rounded crown
    for v in hair_cap.data.vertices:
        if (head_top_z - 0.055) <= v.co.z <= (head_top_z - 0.018) and abs(v.co.x) > 0.045:
            v.co.x += math.copysign(0.0035, v.co.x)
    hair_cap.data.update()

    sm_h = hair_cap.modifiers.new("Smooth", "SMOOTH")
    sm_h.factor = 0.45
    sm_h.iterations = 3
    sol_h = hair_cap.modifiers.new("Solidify", "SOLIDIFY")
    sol_h.thickness = 0.0065
    sol_h.offset = 1.0
    sol_h.use_rim = True
    apply_all_modifiers(hair_cap, smooth=True, recalc_normals=False)
    objs.append(hair_cap)

    # Tapered textured hair lock tufts across rounded crown & front hairline
    h_xs = [v.co.x for v in hair_cap.data.vertices]
    h_ys = [v.co.y for v in hair_cap.data.vertices]
    cx = (min(h_xs) + max(h_xs)) * 0.5
    cy = (min(h_ys) + max(h_ys)) * 0.5
    crown_z = head_top_z + 0.010
    tuft_specs = (
        ((-0.044, -0.062, head_top_z - 0.022), (-0.036, -0.064, crown_z - 0.006), (-0.024, -0.018, crown_z + 0.004), 0.018),
        ((-0.024, -0.070, head_top_z - 0.024), (-0.016, -0.074, crown_z - 0.002), (-0.008, -0.028, crown_z + 0.008), 0.020),
        ((0.000, -0.074, head_top_z - 0.026), (0.006, -0.076, crown_z - 0.001), (0.012, -0.028, crown_z + 0.009), 0.021),
        ((0.024, -0.068, head_top_z - 0.024), (0.030, -0.068, crown_z - 0.002), (0.030, -0.022, crown_z + 0.007), 0.020),
        ((0.044, -0.058, head_top_z - 0.022), (0.042, -0.050, crown_z - 0.006), (0.036, -0.010, crown_z + 0.002), 0.017),
        ((-0.036, -0.018, crown_z - 0.004), (-0.036, 0.018, crown_z + 0.002), (-0.028, 0.056, head_top_z - 0.006), 0.020),
        ((-0.014, -0.024, crown_z + 0.000), (-0.010, 0.012, crown_z + 0.008), (-0.006, 0.052, head_top_z + 0.000), 0.022),
        ((0.014, -0.024, crown_z + 0.000), (0.018, 0.012, crown_z + 0.008), (0.018, 0.052, head_top_z + 0.000), 0.022),
        ((0.036, -0.018, crown_z - 0.004), (0.036, 0.018, crown_z + 0.002), (0.028, 0.056, head_top_z - 0.006), 0.020),
    )
    for idx, (p0, p1, p2, r) in enumerate(tuft_specs):
        pts = [
            (Vector((cx + p0[0], cy + p0[1], p0[2])), r * 0.72, r * 0.40),
            (Vector((cx + p1[0], cy + p1[1], p1[2])), r, r * 0.55),
            (Vector((cx + p2[0], cy + p2[1], p2[2])), r * 0.35, r * 0.25),
        ]
        objs.append(
            loft_tube_along_points(
                f"Head_Hair_Tuft_{idx}",
                pts,
                segments=10,
                collection=collection,
                mat=palette["hair"],
                subdiv=1,
            )
        )

    # 4. Full Visible Bare Arms, Forearms & Hands Skin (overlapping ~3 cm inside sleeve cuff)
    arm_vids = group_vertex_indices(
        body_obj,
        (
            "upperarm02.L", "lowerarm01.L", "lowerarm02.L", "wrist.L",
            "upperarm02.R", "lowerarm01.R", "lowerarm02.R", "wrist.R",
            "metacarpal1.L", "metacarpal2.L", "metacarpal3.L", "metacarpal4.L",
            "metacarpal1.R", "metacarpal2.R", "metacarpal3.R", "metacarpal4.R",
            "finger1-1.L", "finger1-2.L", "finger1-3.L",
            "finger2-1.L", "finger2-2.L", "finger2-3.L",
            "finger3-1.L", "finger3-2.L", "finger3-3.L",
            "finger4-1.L", "finger4-2.L", "finger4-3.L",
            "finger5-1.L", "finger5-2.L", "finger5-3.L",
            "finger1-1.R", "finger1-2.R", "finger1-3.R",
            "finger2-1.R", "finger2-2.R", "finger2-3.R",
            "finger3-1.R", "finger3-2.R", "finger3-3.R",
            "finger4-1.R", "finger4-2.R", "finger4-3.R",
            "finger5-1.R", "finger5-2.R", "finger5-3.R",
            "fingernails",
        ),
    )
    arms_skin = duplicate_subset_mesh(
        body_obj,
        "Arms_Skin",
        lambda vid, co: (vid in arm_vids) and (co.z <= sleeve_cut_z + 0.045) and (abs(co.x) > 0.14),
        collection,
        palette["skin"],
    )
    apply_all_modifiers(arms_skin, smooth=True, recalc_normals=False)
    objs.append(arms_skin)

    return objs


def normalize_character_height_and_grounding(objs: Sequence[bpy.types.Object]) -> float:
    """Scale character cleanly to exact TARGET_HEIGHT_M (1.78 m) with min Z = 0.0."""
    min_z = float("inf")
    max_z = float("-inf")
    min_x = float("inf")
    max_x = float("-inf")
    for obj in objs:
        for v in obj.data.vertices:
            wz = (obj.matrix_world @ v.co).z
            wx = (obj.matrix_world @ v.co).x
            min_z = min(min_z, wz)
            max_z = max(max_z, wz)
            min_x = min(min_x, wx)
            max_x = max(max_x, wx)

    cur_height = max(0.01, max_z - min_z)
    scale = TARGET_HEIGHT_M / cur_height
    shift_z = -min_z * scale
    shift_x = -(min_x + max_x) * 0.5 * scale

    for obj in objs:
        for v in obj.data.vertices:
            v.co.x = v.co.x * scale + shift_x
            v.co.y *= scale
            v.co.z = v.co.z * scale + shift_z
        obj.data.update()

    return TARGET_HEIGHT_M


def setup_studio_and_render(render_dir: str) -> None:
    """Create minimal slate-concrete studio backdrop (unselected for GLB) & render 3 views."""
    os.makedirs(render_dir, exist_ok=True)
    studio_col = ensure_collection("Studio_Backdrop")

    floor_mat = make_material("Studio_Floor_Mat", (0.31, 0.36, 0.39), roughness=0.92)
    wall_mat = make_material("Studio_Wall_Mat", (0.29, 0.34, 0.38), roughness=0.95)

    bm_floor = bmesh.new()
    bmesh.ops.create_grid(bm_floor, x_segments=1, y_segments=1, size=12.0)
    mesh_floor = bpy.data.meshes.new("Studio_Floor")
    bm_floor.to_mesh(mesh_floor)
    bm_floor.free()
    floor_obj = bpy.data.objects.new("Studio_Floor", mesh_floor)
    studio_col.objects.link(floor_obj)
    floor_obj.data.materials.append(floor_mat)

    bm_wall = bmesh.new()
    bmesh.ops.create_circle(bm_wall, cap_ends=False, radius=6.5, segments=48)
    bmesh.ops.extrude_edge_only(bm_wall, edges=list(bm_wall.edges))
    bm_wall.verts.ensure_lookup_table()
    for v in bm_wall.verts:
        if v.index >= 48:
            v.co.z = 5.0
    bmesh.ops.recalc_face_normals(bm_wall, faces=bm_wall.faces)
    bmesh.ops.reverse_faces(bm_wall, faces=bm_wall.faces)
    mesh_wall = bpy.data.meshes.new("Studio_Cyclorama")
    bm_wall.to_mesh(mesh_wall)
    bm_wall.free()
    wall_obj = bpy.data.objects.new("Studio_Cyclorama", mesh_wall)
    studio_col.objects.link(wall_obj)
    wall_obj.data.materials.append(wall_mat)
    for p in wall_obj.data.polygons:
        p.use_smooth = True

    def add_area_light(
        name: str,
        loc: tuple[float, float, float],
        target: tuple[float, float, float],
        energy: float,
        color: tuple[float, float, float],
        size: float,
    ) -> None:
        ldata = bpy.data.lights.new(name=name, type="AREA")
        ldata.energy = energy
        ldata.color = color
        ldata.size = size
        lobj = bpy.data.objects.new(name, ldata)
        lobj.location = loc
        direction = Vector(target) - Vector(loc)
        lobj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
        studio_col.objects.link(lobj)

    add_area_light("Studio_Key_Warm", (2.2, -2.6, 2.3), (0.0, 0.0, 1.0), 190.0, (1.0, 0.90, 0.78), 1.6)
    add_area_light("Studio_Fill_Cool", (-2.6, -2.2, 1.5), (0.0, 0.0, 1.05), 105.0, (0.72, 0.85, 0.98), 2.4)
    add_area_light("Studio_Bounce_Front", (0.0, -2.0, 0.85), (0.0, 0.0, 1.50), 26.0, (0.95, 0.92, 0.88), 1.8)
    add_area_light("Studio_Rim_Cool", (-1.4, 2.4, 2.2), (0.0, 0.0, 1.1), 135.0, (0.76, 0.88, 1.0), 1.4)

    world = bpy.data.worlds.new("Studio_World")
    bpy.context.scene.world = world
    world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    if bg is not None:
        bg.inputs["Color"].default_value = (0.14, 0.17, 0.20, 1.0)
        bg.inputs["Strength"].default_value = 0.7

    cam_data = bpy.data.cameras.new("Studio_Camera")
    cam_data.lens = 58.0
    cam_obj = bpy.data.objects.new("Studio_Camera", cam_data)
    studio_col.objects.link(cam_obj)
    bpy.context.scene.camera = cam_obj

    scene = bpy.context.scene
    scene.render.engine = "BLENDER_EEVEE"
    scene.render.resolution_x = 448
    scene.render.resolution_y = 600
    scene.render.resolution_percentage = 100
    scene.render.image_settings.file_format = "PNG"
    scene.view_settings.view_transform = "Standard"
    scene.view_settings.look = "None"
    scene.view_settings.exposure = 0.10

    target = Vector((0.0, 0.0, 0.90))
    dist = 3.25
    cam_z = 0.96
    views = (
        ("render_frontal.png", 0.0),
        ("render_side.png", -90.0),
        ("render_oblique.png", -42.0),
    )
    for filename, yaw_deg in views:
        rad = math.radians(yaw_deg)
        cx = dist * math.sin(rad)
        cy = -dist * math.cos(rad)
        cam_obj.location = (cx, cy, cam_z)
        direction = target - cam_obj.location
        cam_obj.rotation_euler = direction.to_track_quat("-Z", "Y").to_euler()
        scene.render.filepath = os.path.join(render_dir, filename)
        bpy.ops.render.render(write_still=True)


def export_character_glb(objs: Sequence[bpy.types.Object], out_path: str) -> None:
    """Export only the character meshes to a glTF 2.0 Binary (`.glb`)."""
    out_dir = os.path.dirname(os.path.abspath(out_path))
    os.makedirs(out_dir, exist_ok=True)
    bpy.ops.object.select_all(action="DESELECT")
    for obj in objs:
        obj.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    bpy.ops.export_scene.gltf(
        filepath=out_path,
        export_format="GLB",
        use_selection=True,
        export_yup=True,
        export_apply=True,
        export_materials="EXPORT",
        export_texcoords=False,
        export_normals=True,
        export_animations=False,
        export_cameras=False,
        export_lights=False,
    )


def parse_args(argv: list[str]) -> argparse.Namespace:
    if "--" in argv:
        argv = argv[argv.index("--") + 1 :]
    else:
        argv = []
    parser = argparse.ArgumentParser(description="Generate Alex Wen (Set 1) MPFB2 3D model and GLB.")
    parser.add_argument("--export-glb", default="", help="Output path for .glb binary export.")
    parser.add_argument("--render-dir", default="", help="Optional directory for 3-angle PNG previews.")
    return parser.parse_args(argv)


def main() -> None:
    args = parse_args(sys.argv)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    palette = create_palette()
    char_col = ensure_collection("Alex_Wen_Set01")

    body_obj, tights_obj, landmarks, bone_segs = create_posed_mpfb_body()

    objs: list[bpy.types.Object] = []
    tee_objs, hem_z, sleeve_cut_z = build_tshirt_from_tights(
        tights_obj, landmarks, bone_segs, char_col, palette
    )
    objs.extend(tee_objs)
    objs.extend(build_loose_jeans_from_body(body_obj, landmarks, hem_z, char_col, palette))
    objs.extend(build_trail_shoes_at_feet(landmarks, char_col, palette))
    objs.extend(build_eyes_brows_hair_and_skin(body_obj, landmarks, sleeve_cut_z, char_col, palette))

    bpy.data.objects.remove(tights_obj, do_unlink=True)
    bpy.data.objects.remove(body_obj, do_unlink=True)

    height_m = normalize_character_height_and_grounding(objs)
    tri_count = sum(len(p.vertices) - 2 for o in objs for p in o.data.polygons)
    print(f"BUILT MPFB2 Alex_Wen_Set01: objects={len(objs)} triangles={tri_count} height={height_m:.3f}m")

    if args.export_glb:
        export_character_glb(objs, args.export_glb)
        print(f"EXPORTED GLB: {args.export_glb} ({os.path.getsize(args.export_glb)} bytes)")

    if args.render_dir:
        setup_studio_and_render(args.render_dir)
        print(f"RENDERED PREVIEWS: {args.render_dir}")


if __name__ == "__main__":
    main()
