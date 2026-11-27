#!/usr/bin/env python3
"""MPFB2-powered Blender 5.3 hero-miniature generator for Alex Wen (Set 1, v3).

Builds Alex Wen (1.78 m Chinese-Malay male, athletic build) on top of MakeHuman /
MPFB2 (`bl_ext.blender_org.mpfb`) with moussed upward-styled textured hair,
relaxed hands, natural crew-neck collar, mid-bicep sleeves, 3D cloth/denim folds,
contoured trail shoes, and a single packed 1024x1024 graphic-novel UV texture atlas.

Usage (requires `--` before script args; run outside sandbox so Blender sees MPFB2):
    blender --background --python build_alex_wen_set01_v3.py -- \
        --export-glb assets/models/characters/alex_wen/alex_wen_set01_v3.glb \
        --render-dir path/to/renders
"""

from __future__ import annotations

import argparse
import importlib
import math
import os
import sys
from typing import Callable, Sequence

import bmesh
import bpy
import numpy as np
from mathutils import Euler, Matrix, Quaternion, Vector

TARGET_HEIGHT_M = 1.78
ATLAS_SIZE = 1024


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
    """Rotate a child pose bone using its current posed transform."""
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
    dict[str, Vector],
]:
    """Spawn Alex Wen in MPFB2, pose naturally, and return body, tights, landmarks, bone_segs, face_refs."""
    HumanService = mpfb_import("mpfb.services.humanservice", "HumanService")
    TargetService = mpfb_import("mpfb.services.targetservice", "TargetService")
    LocationService = mpfb_import("mpfb.services.locationservice", "LocationService")
    HumanObjectProperties = mpfb_import("mpfb.entities.objectproperties", "HumanObjectProperties")

    macro = TargetService.get_default_macro_info_dict()
    macro["gender"] = 1.0
    macro["age"] = 0.45
    macro["muscle"] = 0.72
    macro["weight"] = 0.50
    macro["proportions"] = 0.55
    macro["height"] = 0.64
    macro["cupsize"] = 0.0
    macro["firmness"] = 0.74
    macro["race"] = {"asian": 0.92, "caucasian": 0.04, "african": 0.04}

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
    HumanObjectProperties.set_value("muscle", 0.72, entity_reference=human)
    HumanObjectProperties.set_value("weight", 0.50, entity_reference=human)
    HumanObjectProperties.set_value("asian", 0.92, entity_reference=human)
    HumanObjectProperties.set_value("caucasian", 0.04, entity_reference=human)
    HumanObjectProperties.set_value("african", 0.04, entity_reference=human)
    TargetService.reapply_macro_details(human)

    tdir = LocationService.get_mpfb_data("targets")
    facial_targets = (
        ("head/head-rectangular.target.gz", 0.42),
        ("head/head-square.target.gz", 0.30),
        ("head/head-fat-decr.target.gz", 0.62),
        ("chin/chin-bones-incr.target.gz", 0.48),
        ("chin/chin-width-incr.target.gz", 0.24),
        ("chin/chin-prominent-incr.target.gz", 0.28),
        ("cheek/l-cheek-bones-incr.target.gz", 0.52),
        ("cheek/r-cheek-bones-incr.target.gz", 0.52),
        ("cheek/l-cheek-volume-decr.target.gz", 0.48),
        ("cheek/r-cheek-volume-decr.target.gz", 0.48),
        ("eyebrows/eyebrows-trans-forward.target.gz", 0.20),
        ("eyebrows/eyebrows-trans-down.target.gz", 0.28),
        ("mouth/mouth-lowerlip-volume-decr.target.gz", 0.48),
        ("mouth/mouth-lowerlip-height-decr.target.gz", 0.36),
        ("nose/nose-nostrils-width-decr.target.gz", 0.28),
        ("nose/nose-scale-depth-incr.target.gz", 0.28),
        ("nose/nose-point-width-decr.target.gz", 0.22),
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

    # Relaxed shoulders + arms hanging naturally outside thighs with slight elbow bend
    rotate_pose_bone_world(rig, "clavicle.L", Vector((0.0, 1.0, 0.0)), 2.5)
    rotate_pose_bone_world(rig, "clavicle.R", Vector((0.0, -1.0, 0.0)), 2.5)
    rotate_pose_bone_world(rig, "shoulder01.L", Vector((0.0, 1.0, 0.0)), 3.5)
    rotate_pose_bone_world(rig, "shoulder01.R", Vector((0.0, -1.0, 0.0)), 3.5)

    aim_bone_toward_dir(rig, "upperarm01.L", Vector((0.20, 0.03, -1.0)), blend=0.92)
    aim_bone_toward_dir(rig, "upperarm01.R", Vector((-0.20, 0.03, -1.0)), blend=0.92)
    bpy.context.view_layer.update()

    aim_posed_bone_toward_dir(rig, "lowerarm01.L", Vector((0.05, -0.09, -1.0)), blend=0.95)
    aim_posed_bone_toward_dir(rig, "lowerarm01.R", Vector((-0.05, -0.09, -1.0)), blend=0.95)
    bpy.context.view_layer.update()

    # Bring hands slightly inward toward outer thigh & relax fingers together (no stiff splay)
    rotate_posed_bone_world(rig, "wrist.L", Vector((0.0, 1.0, 0.0)), -5.0)
    rotate_posed_bone_world(rig, "wrist.R", Vector((0.0, -1.0, 0.0)), -5.0)
    bpy.context.view_layer.update()

    for finger_idx, base_curl in ((2, 18.0), (3, 22.0), (4, 25.0), (5, 28.0)):
        for seg, ang in ((1, base_curl), (2, base_curl + 10.0), (3, 14.0)):
            rotate_posed_bone_world(rig, f"finger{finger_idx}-{seg}.L", Vector((0.0, 1.0, 0.0)), ang)
            rotate_posed_bone_world(rig, f"finger{finger_idx}-{seg}.R", Vector((0.0, -1.0, 0.0)), ang)
            bpy.context.view_layer.update()

    # Slight adduction on outer fingers & natural relaxed thumb resting near index finger
    for finger_idx, splay_deg in ((2, -2.5), (4, 3.0), (5, 5.5)):
        rotate_posed_bone_world(rig, f"finger{finger_idx}-1.L", Vector((0.0, 0.0, 1.0)), splay_deg)
        rotate_posed_bone_world(rig, f"finger{finger_idx}-1.R", Vector((0.0, 0.0, -1.0)), splay_deg)
    rotate_posed_bone_world(rig, "finger1-1.L", Vector((1.0, 0.0, 0.0)), 14.0)
    rotate_posed_bone_world(rig, "finger1-1.R", Vector((1.0, 0.0, 0.0)), 14.0)
    rotate_posed_bone_world(rig, "finger1-2.L", Vector((0.0, 1.0, 0.0)), 18.0)
    rotate_posed_bone_world(rig, "finger1-2.R", Vector((0.0, -1.0, 0.0)), 18.0)
    bpy.context.view_layer.update()

    # Natural balanced shoulder-width standing stance (tiny outward foot splay)
    rotate_pose_bone_world(rig, "upperleg01.L", Vector((0.0, 1.0, 0.0)), 5.8)
    rotate_pose_bone_world(rig, "upperleg01.R", Vector((0.0, -1.0, 0.0)), 5.8)
    rotate_pose_bone_world(rig, "lowerleg01.L", Vector((0.0, -1.0, 0.0)), 4.6)
    rotate_pose_bone_world(rig, "lowerleg01.R", Vector((0.0, 1.0, 0.0)), 4.6)
    rotate_pose_bone_world(rig, "foot.L", Vector((0.0, -1.0, 0.0)), 1.2)
    rotate_pose_bone_world(rig, "foot.R", Vector((0.0, 1.0, 0.0)), 1.2)
    bpy.context.view_layer.update()

    bone_segs: dict[str, tuple[Vector, Vector]] = {}
    for bn in (
        "clavicle.L", "clavicle.R",
        "shoulder01.L", "shoulder01.R",
        "upperarm01.L", "upperarm01.R",
        "upperarm02.L", "upperarm02.R",
        "lowerarm01.L", "lowerarm01.R",
        "wrist.L", "wrist.R",
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

    face_refs: dict[str, Vector] = {}
    for ref_grp in ("lips", "nose", "ears", "jaw"):
        if ref_grp in baked_obj.vertex_groups:
            face_refs[ref_grp] = group_bounds(baked_obj, ref_grp)[0]

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

    return baked_obj, tights_obj, landmarks, bone_segs, face_refs


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
    ref_up: Vector | None = None,
) -> bpy.types.Object:
    """Loft an elliptical tube along a 3D spine `(center_vec, rx, ry)`."""
    bm = bmesh.new()
    ring_verts: list[list[bmesh.types.BMVert]] = []
    n = len(points)

    prev_ref = ref_up.copy() if ref_up is not None else Vector((0.0, 0.0, 1.0))
    for idx, (pt, rx, ry) in enumerate(points):
        if idx == 0:
            tangent = (points[1][0] - pt).normalized()
        elif idx == n - 1:
            tangent = (pt - points[n - 2][0]).normalized()
        else:
            tangent = (points[idx + 1][0] - points[idx - 1][0]).normalized()

        if abs(tangent.dot(prev_ref)) > 0.92:
            prev_ref = (
                Vector((0.0, -1.0, 0.0))
                if abs(prev_ref.dot(Vector((0.0, 0.0, 1.0)))) > 0.5
                else Vector((0.0, 0.0, 1.0))
            )
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


def build_tshirt_from_tights(
    tights_obj: bpy.types.Object,
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    bone_segs: dict[str, tuple[Vector, Vector]],
    collection: bpy.types.Collection,
    atlas_mat: bpy.types.Material,
) -> tuple[bpy.types.Object, bpy.types.Object, float, float, float]:
    """Extract a relaxed crew-neck t-shirt with mid-bicep sleeves & 3D drape folds."""
    neck_bot_z = landmarks["neck01"][1].z
    pelvis_z = landmarks["joint-pelvis"][0].z
    hem_z = pelvis_z - 0.008

    ua1_l_h, _ = bone_segs["upperarm01.L"]
    ua2_l_h, ua2_l_t = bone_segs["upperarm02.L"]
    arm_dir_l = (ua2_l_t - ua1_l_h).normalized()
    p_cuff_l = ua2_l_h.lerp(ua2_l_t, 0.56)

    ua1_r_h, _ = bone_segs["upperarm01.R"]
    ua2_r_h, ua2_r_t = bone_segs["upperarm02.R"]
    arm_dir_r = (ua2_r_t - ua1_r_h).normalized()
    p_cuff_r = ua2_r_h.lerp(ua2_r_t, 0.56)
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

    tee = duplicate_subset_mesh(
        tights_obj,
        "Torso_TShirt",
        lambda vid, co: (vid not in exclude_vids) and (co.z >= hem_z - 0.05),
        collection,
        atlas_mat,
    )

    bisect_mesh_horizontal(tee, hem_z, keep_above=True)

    # Lower collar line cleanly onto sternoclavicular notch / C7 base (eliminating turtleneck rise)
    for v in tee.data.vertices:
        if v.co.z > neck_bot_z - 0.068 and abs(v.co.x) < 0.130:
            wx = max(0.0, 1.0 - (abs(v.co.x) / 0.130) ** 1.8)
            front_w = 0.74 + 0.26 * max(0.0, min(1.0, (0.025 - v.co.y) / 0.095))
            wz = max(0.0, min(1.0, (v.co.z - (neck_bot_z - 0.068)) / 0.088))
            dz = 0.050 * wx * front_w * wz
            if dz > 0.0:
                v.co.z -= dz
                v.co.y -= 0.15 * dz * front_w
    tee.data.update()

    sw = tee.modifiers.new("NeckConform", "SHRINKWRAP")
    sw.target = tights_obj
    sw.wrap_method = "NEAREST_SURFACEPOINT"
    sw.wrap_mode = "OUTSIDE_SURFACE"
    sw.offset = 0.0012
    apply_all_modifiers(tee, smooth=False, recalc_normals=False)

    # Cut perpendicular mid-bicep sleeve openings
    bm = bmesh.new()
    bm.from_mesh(tee.data)
    for side_sign, p_cuff, arm_dir in ((1.0, p_cuff_l, arm_dir_l), (-1.0, p_cuff_r, arm_dir_r)):
        arm_faces = [f for f in bm.faces if all(v.co.x * side_sign > 0.172 for v in f.verts)]
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

    # Straight relaxed cotton drape from chest to hip + subtle 3D fold ripples
    chest_z = bone_segs["shoulder01.L"][1].z - 0.11
    for v in tee.data.vertices:
        if v.co.z < chest_z and abs(v.co.x) < 0.215:
            t = max(0.0, min(1.0, (chest_z - v.co.z) / max(0.01, chest_z - hem_z)))
            cx, cy = 0.0, -0.022
            v.co.x = cx + (v.co.x - cx) * (1.0 + 0.105 * t)
            y_scale = 1.095 if v.co.y < -0.018 else 0.995
            v.co.y = cy + (v.co.y - cy) * (1.0 + (y_scale - 1.0) * t)
    tee.data.update()

    # Cotton fabric offset (generous sleeve ease + subtle 3D drape ripples at torso & armpits)
    tee.data.update()
    norms = [v.normal.copy() for v in tee.data.vertices]
    for v, n in zip(tee.data.vertices, norms):
        is_sleeve = abs(v.co.x) > 0.175
        base_off = 0.0135 if is_sleeve else 0.0065
        fold = 0.0
        if not is_sleeve and v.co.z < chest_z + 0.06:
            diag = v.co.z * 28.0 - abs(v.co.x) * 20.0
            vert = v.co.x * 46.0 + v.co.z * 8.0
            armpit_w = math.exp(-((v.co.z - (chest_z + 0.01)) / 0.11) ** 2) * min(1.0, abs(v.co.x) / 0.13)
            lower_w = max(0.0, min(1.0, (chest_z - v.co.z) / max(0.05, chest_z - hem_z)))
            fold = 0.0032 * armpit_w * math.sin(diag) + 0.0024 * lower_w * math.cos(vert)
        v.co += n * (base_off + fold)
    tee.data.update()

    sm = tee.modifiers.new("Smooth", "SMOOTH")
    sm.factor = 0.42
    sm.iterations = 4
    sub = tee.modifiers.new("Subsurf", "SUBSURF")
    sub.levels = 1
    sub.render_levels = 1
    apply_all_modifiers(tee, smooth=True, recalc_normals=False)

    # Extract flat ribbed crew-neck collar ring directly from neckline boundary
    bm_col = bmesh.new()
    bm_col.from_mesh(tee.data)
    top_bound_verts = {
        v.index
        for v in bm_col.verts
        if v.is_boundary and v.co.z > neck_bot_z - 0.095 and abs(v.co.x) < 0.125
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
        atlas_mat,
    )
    inflate_along_normals(collar, lambda _co: 0.0014)
    sol_c = collar.modifiers.new("Solidify", "SOLIDIFY")
    sol_c.thickness = 0.0046
    sol_c.offset = 1.0
    sol_c.use_rim = True
    apply_all_modifiers(collar, smooth=True, recalc_normals=False)

    sol = tee.modifiers.new("Solidify", "SOLIDIFY")
    sol.thickness = 0.0045
    sol.offset = 1.0
    sol.use_rim = True
    apply_all_modifiers(tee, smooth=True, recalc_normals=False)

    return tee, collar, hem_z, sleeve_cut_z, chest_z


def build_loose_jeans_from_body(
    body_obj: bpy.types.Object,
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    hem_z: float,
    collection: bpy.types.Collection,
    atlas_mat: bpy.types.Material,
) -> tuple[bpy.types.Object, list[bpy.types.Object], float, float, float]:
    """Build straight-leg faded denim jeans with 3D knee/ankle folds and clean cuffs."""
    ankle_z = min(landmarks["joint-l-ankle"][0].z, landmarks["joint-r-ankle"][0].z)
    knee_z = 0.5 * (landmarks["joint-l-knee"][0].z + landmarks["joint-r-knee"][0].z)
    cuff_z = ankle_z - 0.006
    waist_z = hem_z + 0.115
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
        atlas_mat,
    )

    bisect_mesh_horizontal(jeans, waist_z, keep_above=False)
    bisect_mesh_horizontal(jeans, cuff_z, keep_above=True)

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

    n_bins = 18
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

    for v in jeans.data.vertices:
        z = v.co.z
        if z > crotch_z:
            t = max(0.0, min(1.0, (z - crotch_z) / max(0.01, waist_z - crotch_z)))
            expand = 0.0085 * (1.0 - 0.85 * t)
            v.co += v.normal * expand
        else:
            side = 1 if v.co.x >= 0.0 else -1
            frac = max(0.0, min(1.0, (z - cuff_z) / z_span))
            b = max(0, min(n_bins - 1, int(frac * n_bins)))
            cx, cy = bin_centers[side][b]
            dx, dy = v.co.x - cx, v.co.y - cy
            r = math.hypot(dx, dy)
            target_r = 0.076 + 0.019 * frac
            if r > 1e-4:
                new_r = max(r + 0.008, target_r)
                blended_r = r * 0.24 + new_r * 0.76
                v.co.x = cx + (dx / r) * blended_r
                v.co.y = cy + (dy / r) * blended_r
            # 3D knee horizontal creases + ankle stacking break folds
            knee_w = math.exp(-((z - knee_z) / 0.07) ** 2)
            ankle_w = math.exp(-((z - (cuff_z + 0.055)) / 0.055) ** 2)
            v.co.y += 0.0038 * knee_w * math.sin((z - knee_z) * 58.0)
            v.co.y += 0.0042 * ankle_w * math.sin((z - cuff_z) * 72.0)

    jeans.data.update()
    sm = jeans.modifiers.new("Smooth", "SMOOTH")
    sm.factor = 0.58
    sm.iterations = 8
    sol = jeans.modifiers.new("Solidify", "SOLIDIFY")
    sol.thickness = 0.0045
    sol.offset = 1.0
    sol.use_rim = True
    apply_all_modifiers(jeans, smooth=True, recalc_normals=False)

    cuffs: list[bpy.types.Object] = []
    for side_tag, sx in (("L", 1.0), ("R", -1.0)):
        cuff = duplicate_subset_mesh(
            jeans,
            f"Legs_Jeans_Cuff_{side_tag}",
            lambda _vid, co, s=sx: (co.x * s > 0.02) and (cuff_z + 0.001 <= co.z <= cuff_z + 0.017),
            collection,
            atlas_mat,
        )
        inflate_along_normals(cuff, lambda _co: 0.0014)
        apply_all_modifiers(cuff, smooth=True, recalc_normals=False)
        cuffs.append(cuff)

    return jeans, cuffs, waist_z, knee_z, cuff_z


def foot_contour(theta: float) -> tuple[float, float]:
    """Asymmetric trail shoe footprint: broader rounded forefoot (-Y), tapered heel (+Y)."""
    ct, st = math.cos(theta), math.sin(theta)
    if st < 0.0:
        return ct * (1.04 - 0.10 * st * st), st * 1.04
    return ct * (0.82 - 0.05 * st), st * 0.96


def build_trail_shoes_at_feet(
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    collection: bpy.types.Collection,
    atlas_mat: bpy.types.Material,
) -> dict[str, list[bpy.types.Object]]:
    """Build contoured hiking/trail shoes grouped by atlas region (`sole`, `upper`, `cap`, `lace`)."""
    grouped: dict[str, list[bpy.types.Object]] = {
        "sole": [],
        "upper": [],
        "cap": [],
        "lace": [],
    }
    for side_tag, s in (("L", 1.0), ("R", -1.0)):
        _, f_min, f_max = landmarks[f"foot.{side_tag}"]
        ankle_center = landmarks[f"joint-{side_tag.lower()}-ankle"][0]
        foot_len = max(0.26, f_max.y - f_min.y + 0.028)
        foot_w = max(0.054, (f_max.x - f_min.x) * 0.57)
        base_z = f_min.z - 0.008
        pivot = Vector((ankle_center.x + s * 0.003, (f_min.y + f_max.y) * 0.5 - 0.018, base_z))
        yaw = s * math.radians(-6.5)

        def place(obj: bpy.types.Object) -> bpy.types.Object:
            rot = Euler((0.0, 0.0, yaw), "XYZ").to_matrix().to_4x4()
            obj.data.transform(Matrix.Translation(pivot) @ rot)
            obj.data.update()
            return obj

        ry = foot_len * 0.52
        sole = loft_profile_rings(
            f"Legs_Shoe_Sole_{side_tag}",
            [
                (0.0, 0.000, 0.001, foot_w * 0.95, ry * 0.97),
                (0.0, -0.002, 0.010, foot_w * 1.03, ry * 1.02),
                (0.0, -0.002, 0.024, foot_w * 1.02, ry * 1.01),
                (0.0, 0.000, 0.032, foot_w * 0.98, ry * 0.97),
            ],
            segments=24,
            collection=collection,
            mat=atlas_mat,
            shape_fn=foot_contour,
            subdiv=1,
        )
        grouped["sole"].append(place(sole))

        for idx, y_off in enumerate((-0.088, -0.044, 0.000, 0.046, 0.086)):
            lug = create_beveled_box(
                f"Legs_Shoe_Lug_{side_tag}_{idx}",
                (0.0, y_off, 0.003),
                (foot_w * 1.32, 0.024, 0.006),
                (0.0, 0.0, 0.0),
                0.0015,
                collection,
                atlas_mat,
                smooth=False,
            )
            grouped["sole"].append(place(lug))

        upper = loft_profile_rings(
            f"Legs_Shoe_Upper_{side_tag}",
            [
                (0.0, 0.000, 0.026, foot_w * 0.96, ry * 0.96),
                (0.0, -0.004, 0.048, foot_w * 0.95, ry * 0.93),
                (0.0, 0.015, 0.068, foot_w * 0.87, ry * 0.74),
                (0.0, 0.038, 0.086, foot_w * 0.78, ry * 0.50),
                (0.0, 0.046, 0.100, foot_w * 0.74, ry * 0.42),
            ],
            segments=24,
            collection=collection,
            mat=atlas_mat,
            shape_fn=foot_contour,
            subdiv=1,
        )
        grouped["upper"].append(place(upper))

        toecap = create_uv_sphere(
            f"Legs_Shoe_ToeCap_{side_tag}",
            (0.0, -ry * 0.71, 0.040),
            (foot_w * 0.82, ry * 0.34, 0.021),
            (math.radians(-7.0), 0.0, 0.0),
            collection,
            atlas_mat,
            u_seg=14,
            v_seg=10,
        )
        grouped["cap"].append(place(toecap))

        heelcap = create_uv_sphere(
            f"Legs_Shoe_HeelCap_{side_tag}",
            (0.0, ry * 0.68, 0.046),
            (foot_w * 0.74, ry * 0.28, 0.026),
            (math.radians(8.0), 0.0, 0.0),
            collection,
            atlas_mat,
            u_seg=14,
            v_seg=10,
        )
        grouped["cap"].append(place(heelcap))

        collar = loft_profile_rings(
            f"Legs_Shoe_Collar_{side_tag}",
            [
                (0.0, 0.044, 0.082, foot_w * 0.80, ry * 0.44),
                (0.0, 0.046, 0.098, foot_w * 0.79, ry * 0.42),
                (0.0, 0.044, 0.108, foot_w * 0.70, ry * 0.36),
            ],
            segments=18,
            collection=collection,
            mat=atlas_mat,
            subdiv=1,
        )
        grouped["cap"].append(place(collar))

        tongue = create_beveled_box(
            f"Legs_Shoe_Tongue_{side_tag}",
            (0.0, -0.020, 0.086),
            (foot_w * 0.74, 0.068, 0.022),
            (math.radians(32.0), 0.0, 0.0),
            0.007,
            collection,
            atlas_mat,
        )
        grouped["lace"].append(place(tongue))

        # Criss-cross X-pattern trail laces
        for i, (ly, lz) in enumerate((
            (-0.058, 0.068),
            (-0.038, 0.080),
            (-0.018, 0.092),
            (0.002, 0.103),
        )):
            for x_ang in (26.0, -26.0):
                lace = create_beveled_box(
                    f"Legs_Shoe_Lace_{side_tag}_{i}_{int(x_ang)}",
                    (0.0, ly, lz),
                    (foot_w * 0.90, 0.006, 0.0055),
                    (math.radians(26.0), 0.0, math.radians(x_ang)),
                    0.0018,
                    collection,
                    atlas_mat,
                )
                grouped["lace"].append(place(lace))

    return grouped


def build_eyes_brows_hair_and_skin(
    body_obj: bpy.types.Object,
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    sleeve_cut_z: float,
    collection: bpy.types.Collection,
    atlas_mat: bpy.types.Material,
) -> dict[str, list[bpy.types.Object]]:
    """Build head skin, 4-layer eyes, sculpted brows, layered swept hair, and arms/hands skin."""
    grouped: dict[str, list[bpy.types.Object]] = {
        "head_skin": [],
        "arms_skin": [],
        "sclera": [],
        "iris": [],
        "pupil": [],
        "catchlight": [],
        "hair": [],
    }

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
            (co.z > neck_z + 0.028)
            or ((vid in head_vids) and co.z >= neck_z - 0.018 and abs(co.x) < 0.085)
        ),
        collection,
        atlas_mat,
    )
    apply_all_modifiers(head_skin, smooth=True, recalc_normals=False)
    grouped["head_skin"].append(head_skin)

    for side_tag in ("L", "R"):
        eye_c, eye_mn, eye_mx = landmarks[f"helper-{side_tag.lower()}-eye"]
        eye_r = max(0.011, (eye_mx.x - eye_mn.x) * 0.46)
        eye_cy = eye_c.y + 0.0008
        grouped["sclera"].append(
            create_uv_sphere(
                f"Head_Eye_Sclera_{side_tag}",
                (eye_c.x, eye_cy, eye_c.z),
                (eye_r, eye_r, eye_r),
                (0.0, 0.0, 0.0),
                collection,
                atlas_mat,
                u_seg=16,
                v_seg=12,
            )
        )
        grouped["iris"].append(
            create_uv_sphere(
                f"Head_Eye_Iris_{side_tag}",
                (eye_c.x, eye_cy - eye_r * 0.76, eye_c.z),
                (eye_r * 0.56, eye_r * 0.28, eye_r * 0.56),
                (0.0, 0.0, 0.0),
                collection,
                atlas_mat,
                u_seg=14,
                v_seg=10,
            )
        )
        grouped["pupil"].append(
            create_uv_sphere(
                f"Head_Eye_Pupil_{side_tag}",
                (eye_c.x, eye_cy - eye_r * 0.90, eye_c.z),
                (eye_r * 0.26, eye_r * 0.14, eye_r * 0.26),
                (0.0, 0.0, 0.0),
                collection,
                atlas_mat,
                u_seg=12,
                v_seg=8,
            )
        )
        grouped["catchlight"].append(
            create_uv_sphere(
                f"Head_Eye_Catchlight_{side_tag}",
                (eye_c.x + eye_r * 0.18, eye_cy - eye_r * 0.96, eye_c.z + eye_r * 0.20),
                (eye_r * 0.065, eye_r * 0.04, eye_r * 0.065),
                (0.0, 0.0, 0.0),
                collection,
                atlas_mat,
                u_seg=10,
                v_seg=6,
            )
        )

        # Bold masculine arched eyebrows hugging the brow ridge
        brow_z = eye_c.z + 0.0148
        brow_verts = [
            v.co
            for v in head_skin.data.vertices
            if abs(v.co.x - eye_c.x) < 0.019
            and abs(v.co.z - brow_z) < 0.008
            and v.co.y < eye_c.y
        ]
        brow_surface_y = min((p.y for p in brow_verts), default=(eye_c.y - 0.022))
        s = 1.0 if side_tag == "L" else -1.0
        brow_cx = eye_c.x + s * 0.0038
        brow_w, brow_h = 0.047, 0.0078
        bm_brow = bmesh.new()
        cols = []
        for ix in range(9):
            u = ix / 8.0
            x = brow_cx + s * (u - 0.45) * brow_w
            inner_full = 1.02 + 0.14 * math.sin(min(1.0, u / 0.65) * math.pi)
            outer_taper = max(0.28, 1.0 - max(0.0, (u - 0.58) / 0.42) ** 1.35 * 0.72)
            h_scale = inner_full * outer_taper
            tail_drop = 0.0014 * max(0.0, (u - 0.72) / 0.28)
            z_arch = 0.0024 * math.sin(u * math.pi * 0.85) - tail_drop
            y = (brow_surface_y - 0.004) + 0.0075 * (u ** 1.7)
            v_bot = bm_brow.verts.new((x, y, brow_z + z_arch - 0.5 * brow_h * h_scale))
            v_top = bm_brow.verts.new((x, y, brow_z + z_arch + 0.5 * brow_h * h_scale))
            cols.append((v_bot, v_top))
        for ix in range(8):
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
        brow_obj.data.materials.append(atlas_mat)

        sw_brow = brow_obj.modifiers.new("BrowConform", "SHRINKWRAP")
        sw_brow.target = head_skin
        sw_brow.wrap_method = "NEAREST_SURFACEPOINT"
        sw_brow.wrap_mode = "OUTSIDE_SURFACE"
        sw_brow.offset = 0.0008

        sol_brow = brow_obj.modifiers.new("BrowSolidify", "SOLIDIFY")
        sol_brow.thickness = 0.0022
        sol_brow.offset = 1.0
        sol_brow.use_rim = True
        apply_all_modifiers(brow_obj, smooth=True, recalc_normals=True)
        grouped["hair"].append(brow_obj)

    # Tight tapered lower sides + boxy structured upper crown + swept front fringe locks
    scalp_vids = group_vertex_indices(body_obj, ("scalp",))
    hairline_z = max(landmarks["helper-l-eye"][0].z, landmarks["helper-r-eye"][0].z) + 0.044
    hair_cap = duplicate_subset_mesh(
        body_obj,
        "Head_Hair_Cap",
        lambda vid, co: (not (co.y < -0.042 and abs(co.x) < 0.064 and co.z < hairline_z - 0.002))
        and ((vid in scalp_vids) or (vid in head_vids and co.z >= hairline_z and co.y > -0.148)),
        collection,
        atlas_mat,
    )
    head_top_z = max(v.co.z for v in hair_cap.data.vertices)
    inflate_along_normals(
        hair_cap,
        lambda co: 0.0028 + 0.0125 * max(0.0, min(1.0, (co.z - (head_top_z - 0.085)) / 0.085)),
    )
    for v in hair_cap.data.vertices:
        dz_top = head_top_z - v.co.z
        # Flare upper temples & parietal corners outward/upward for a boxier top silhouette
        if -0.012 <= dz_top <= 0.058:
            z_band = math.sin(max(0.0, min(1.0, (0.058 - dz_top) / 0.068)) * math.pi * 0.78)
            ax = abs(v.co.x)
            if ax > 0.032:
                x_factor = min(1.0, (ax - 0.032) / 0.042)
                v.co.x += math.copysign(0.0110 * z_band * x_factor, v.co.x)
                v.co.z += 0.0115 * z_band * x_factor
            if v.co.y < -0.035:
                v.co.y -= 0.0065 * z_band
        # Smooth front-to-mid crest lift so upward moussed locks sit on a continuous sculpted base
        if v.co.z > hairline_z + 0.015 and v.co.y < 0.025:
            front_w = max(0.0, min(1.0, (0.025 - v.co.y) / 0.105))
            top_w = max(0.0, min(1.0, (v.co.z - (hairline_z + 0.015)) / 0.040))
            center_w = max(0.25, 1.0 - (abs(v.co.x) / 0.085) ** 2)
            v.co.z += 0.005 * (front_w ** 1.2) * top_w * center_w
    hair_cap.data.update()

    # Snap open front boundary vertices onto one smooth hairline arch before Smooth/Solidify
    bm_cap = bmesh.new()
    bm_cap.from_mesh(hair_cap.data)
    cap_xs = [v.co.x for v in bm_cap.verts]
    cap_ys = [v.co.y for v in bm_cap.verts]
    cap_cx = 0.5 * (min(cap_xs) + max(cap_xs))
    cap_cy = 0.5 * (min(cap_ys) + max(cap_ys))
    for v in bm_cap.verts:
        if (
            v.is_boundary
            and v.co.y < cap_cy - 0.025
            and abs(v.co.x - cap_cx) < 0.062
            and v.co.z < hairline_z + 0.022
        ):
            v.co.z = hairline_z + 0.004 + 0.005 * ((v.co.x - cap_cx) / 0.062) ** 2
    bm_cap.to_mesh(hair_cap.data)
    bm_cap.free()
    hair_cap.data.update()

    sm_h = hair_cap.modifiers.new("Smooth", "SMOOTH")
    sm_h.factor = 0.48
    sm_h.iterations = 4
    apply_all_modifiers(hair_cap, smooth=True, recalc_normals=False)

    sub_h = hair_cap.modifiers.new("Subsurf", "SUBSURF")
    sub_h.levels = 1
    sub_h.render_levels = 1
    apply_all_modifiers(hair_cap, smooth=True, recalc_normals=False)

    # Subtle upward-and-angled surface fluting across front hair cap (fades smoothly to zero at edges)
    for v in hair_cap.data.vertices:
        if v.co.y < cap_cy - 0.010 and abs(v.co.x - cap_cx) < 0.058 and v.co.z > hairline_z + 0.007:
            x_fade = max(0.0, 1.0 - (abs(v.co.x - cap_cx) / 0.058) ** 2)
            z_fade = max(0.0, min(1.0, (v.co.z - (hairline_z + 0.007)) / 0.014))
            y_fade = max(0.0, min(1.0, (cap_cy - 0.010 - v.co.y) / 0.055))
            phase = ((v.co.x - cap_cx) - 0.24 * (v.co.z - hairline_z)) * 220.0
            ridge = 0.0012 * (0.65 * math.sin(phase) + 0.35 * math.sin(phase * 1.85 + 0.9))
            weight = x_fade * z_fade * y_fade
            v.co.y -= ridge * weight
            v.co.z += ridge * 0.45 * weight
    hair_cap.data.update()

    sol_h = hair_cap.modifiers.new("Solidify", "SOLIDIFY")
    sol_h.thickness = 0.0056
    sol_h.offset = 1.0
    sol_h.use_rim = True
    apply_all_modifiers(hair_cap, smooth=True, recalc_normals=False)
    grouped["hair"].append(hair_cap)

    h_xs = [v.co.x for v in hair_cap.data.vertices]
    h_ys = [v.co.y for v in hair_cap.data.vertices]
    cx = (min(h_xs) + max(h_xs)) * 0.5
    cy = (min(h_ys) + max(h_ys)) * 0.5

    tuft_specs = (
        # Rear crown / nape transition layers (top-middle crown kept clean & boxy via Head_Hair_Cap)
        ((-0.042, 0.028, head_top_z - 0.005), (-0.036, 0.064, head_top_z - 0.016), (-0.028, 0.080, head_top_z - 0.046), 0.022),
        ((-0.012, 0.034, head_top_z - 0.002), (-0.010, 0.068, head_top_z - 0.013), (-0.008, 0.086, head_top_z - 0.044), 0.024),
        ((0.016, 0.034, head_top_z - 0.002), (0.016, 0.068, head_top_z - 0.013), (0.012, 0.086, head_top_z - 0.044), 0.024),
        ((0.042, 0.028, head_top_z - 0.005), (0.040, 0.064, head_top_z - 0.016), (0.030, 0.080, head_top_z - 0.046), 0.022),
        # Clean sideburns tapering down in front of ears
        ((-0.065, -0.034, head_top_z - 0.048), (-0.069, -0.016, head_top_z - 0.068), (-0.067, -0.008, head_top_z - 0.094), 0.012),
        ((0.065, -0.034, head_top_z - 0.048), (0.069, -0.016, head_top_z - 0.068), (0.067, -0.008, head_top_z - 0.094), 0.012),
    )
    for idx, (p0, p1, p2, r) in enumerate(tuft_specs):
        pts = [
            (Vector((cx + p0[0], cy + p0[1], p0[2])), r * 0.85, r * 0.24),
            (Vector((cx + p1[0], cy + p1[1], p1[2])), r * 1.05, r * 0.28),
            (Vector((cx + p2[0], cy + p2[1], p2[2])), r * 0.40, r * 0.14),
        ]
        grouped["hair"].append(
            loft_tube_along_points(
                f"Head_Hair_Tuft_{idx}",
                pts,
                segments=10,
                collection=collection,
                mat=atlas_mat,
                subdiv=1,
            )
        )

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
        atlas_mat,
    )
    apply_all_modifiers(arms_skin, smooth=True, recalc_normals=False)
    grouped["arms_skin"].append(arms_skin)

    return grouped


# ============================================================================
# Graphic-Novel UV Assignment & 1024x1024 Procedural Texture Atlas
# ============================================================================


def clamp01(val: float) -> float:
    return max(0.002, min(0.998, val))


def assign_uv_loops(obj: bpy.types.Object, uv_fn: Callable[[bmesh.types.BMFace, Vector], tuple[float, float]]) -> None:
    """Assign clean clamped [0, 1] per-loop UVs to `obj`."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    uv_layer = bm.loops.layers.uv.verify()
    for face in bm.faces:
        for loop in face.loops:
            u, v = uv_fn(face, loop.vert.co)
            loop[uv_layer].uv = (clamp01(u), clamp01(v))
    bm.to_mesh(obj.data)
    bm.free()
    obj.data.update()


def assign_constant_uv_box(obj: bpy.types.Object, u0: float, v0: float, u1: float, v1: float) -> None:
    """Map an object uniformly onto a dedicated swatch box in the atlas."""
    xs = [v.co.x for v in obj.data.vertices]
    zs = [v.co.z for v in obj.data.vertices]
    min_x, max_x = min(xs), max(xs)
    min_z, max_z = min(zs), max(zs)
    dx = max(1e-4, max_x - min_x)
    dz = max(1e-4, max_z - min_z)
    assign_uv_loops(
        obj,
        lambda _f, co: (
            u0 + ((co.x - min_x) / dx) * (u1 - u0),
            v0 + ((co.z - min_z) / dz) * (v1 - v0),
        ),
    )


def assign_all_uvs_and_build_atlas(
    *,
    head_skin: bpy.types.Object,
    arms_skin: bpy.types.Object,
    tee: bpy.types.Object,
    collar: bpy.types.Object,
    jeans: bpy.types.Object,
    cuffs: Sequence[bpy.types.Object],
    shoe_groups: dict[str, list[bpy.types.Object]],
    eye_groups: dict[str, list[bpy.types.Object]],
    hair_objs: Sequence[bpy.types.Object],
    landmarks: dict[str, tuple[Vector, Vector, Vector]],
    face_refs: dict[str, Vector],
    hem_z: float,
    waist_z: float,
    knee_z: float,
    cuff_z: float,
    chest_z: float,
) -> bpy.types.Image:
    """Unwrap every character part cleanly into 4 quadrants and paint a 1024x1024 PNG atlas."""
    # --- 1. Q1 (u: 0.00..0.50, v: 0.50..1.00): Head/Face/Neck + Arms/Hands + Eye Swatches ---
    h_zs = [v.co.z for v in head_skin.data.vertices]
    h_z0, h_z1 = min(h_zs) - 0.004, max(h_zs) + 0.004
    ear_y = face_refs.get("ears", Vector((0.0, -0.02, 1.6))).y

    def head_proj(co: Vector, force_side: float = 0.0) -> tuple[float, float]:
        front_y = ear_y
        wrap = max(0.0, co.y - front_y)
        sx = force_side if force_side != 0.0 else (1.0 if co.x >= 0.0 else -1.0)
        x_unwrapped = co.x + sx * wrap * 0.58
        u = 0.180 + (x_unwrapped / 0.165) * 0.155
        v = 0.515 + ((co.z - h_z0) / max(0.05, h_z1 - h_z0)) * 0.468
        return max(0.015, min(0.345, u)), max(0.515, min(0.985, v))

    def head_uv_fn(face: bmesh.types.BMFace, co: Vector) -> tuple[float, float]:
        cent = face.calc_center_median()
        force_s = (1.0 if cent.x >= 0.0 else -1.0) if cent.y > ear_y + 0.02 else 0.0
        return head_proj(co, force_s)

    assign_uv_loops(head_skin, head_uv_fn)

    # Arms/Hands in Q1 right strip (u: 0.360..0.485, v: 0.545..0.985)
    a_zs = [v.co.z for v in arms_skin.data.vertices]
    a_z0, a_z1 = min(a_zs) - 0.004, max(a_zs) + 0.004
    a_ys = [v.co.y for v in arms_skin.data.vertices]
    a_y0, a_y1 = min(a_ys) - 0.004, max(a_ys) + 0.004
    assign_uv_loops(
        arms_skin,
        lambda _f, co: (
            0.362 + ((co.y - a_y0) / max(0.04, a_y1 - a_y0)) * 0.120,
            0.550 + ((co.z - a_z0) / max(0.05, a_z1 - a_z0)) * 0.432,
        ),
    )

    # Eye swatches tucked cleanly at bottom of Q1 right strip (u: 0.36..0.48, v: 0.512..0.540)
    for o in eye_groups["sclera"]:
        assign_constant_uv_box(o, 0.365, 0.514, 0.388, 0.538)
    for o in eye_groups["iris"]:
        assign_constant_uv_box(o, 0.394, 0.514, 0.418, 0.538)
    for o in eye_groups["pupil"]:
        assign_constant_uv_box(o, 0.424, 0.514, 0.448, 0.538)
    for o in eye_groups["catchlight"]:
        assign_constant_uv_box(o, 0.454, 0.514, 0.482, 0.538)

    # --- 2. Q2 (u: 0.50..1.00, v: 0.50..1.00): T-Shirt Front + Back + Collar ---
    t_zs = [v.co.z for v in tee.data.vertices]
    t_z0, t_z1 = min(t_zs) - 0.004, max(t_zs) + 0.004
    t_xs = [abs(v.co.x) for v in tee.data.vertices]
    t_xmax = max(0.28, max(t_xs) + 0.01)
    tee_mid_y = -0.010

    def tee_uv_fn(face: bmesh.types.BMFace, co: Vector) -> tuple[float, float]:
        cent = face.calc_center_median()
        u_norm = (co.x / t_xmax) * 0.5 + 0.5
        v = 0.545 + ((co.z - t_z0) / max(0.05, t_z1 - t_z0)) * 0.440
        if cent.y <= tee_mid_y:
            u = 0.515 + max(0.0, min(1.0, u_norm)) * 0.228
        else:
            u = 0.757 + max(0.0, min(1.0, u_norm)) * 0.228
        return u, v

    assign_uv_loops(tee, tee_uv_fn)
    assign_constant_uv_box(collar, 0.518, 0.512, 0.982, 0.535)

    # --- 3. Q3 (u: 0.00..0.50, v: 0.00..0.50): Faded Blue Jeans Front + Back ---
    j_xs = [abs(v.co.x) for v in jeans.data.vertices]
    j_xmax = max(0.24, max(j_xs) + 0.01)
    jeans_mid_y = -0.012

    def jeans_uv_fn(face: bmesh.types.BMFace, co: Vector) -> tuple[float, float]:
        cent = face.calc_center_median()
        u_norm = (co.x / j_xmax) * 0.5 + 0.5
        v = 0.016 + ((co.z - cuff_z) / max(0.05, waist_z - cuff_z)) * 0.468
        if cent.y <= jeans_mid_y:
            u = 0.016 + max(0.0, min(1.0, u_norm)) * 0.226
        else:
            u = 0.258 + max(0.0, min(1.0, u_norm)) * 0.226
        return u, v

    assign_uv_loops(jeans, jeans_uv_fn)
    for cuff_obj in cuffs:
        assign_constant_uv_box(cuff_obj, 0.025, 0.016, 0.235, 0.030)

    # --- 4. Q4 (u: 0.50..1.00, v: 0.00..0.50): Hair + Trail Shoes ---
    for h_obj in hair_objs:
        if h_obj.name.startswith("Head_Eyebrow"):
            assign_constant_uv_box(h_obj, 0.504, 0.256, 0.520, 0.282)
        elif h_obj.name.startswith("Head_Hair_Cap"):
            hc_xs = [v.co.x for v in h_obj.data.vertices]
            hc_ys = [v.co.y for v in h_obj.data.vertices]
            hc_zs = [v.co.z for v in h_obj.data.vertices]
            h_cx = 0.5 * (min(hc_xs) + max(hc_xs))
            h_cy = 0.5 * (min(hc_ys) + max(hc_ys))
            hc_z0, hc_z1 = min(hc_zs), max(hc_zs)

            def hair_cap_uv_fn(face: bmesh.types.BMFace, co: Vector) -> tuple[float, float]:
                cent = face.calc_center_median()
                angle = math.atan2(co.x - h_cx, -(co.y - h_cy))
                if cent.y > h_cy and (co.x - h_cx) * (cent.x - h_cx) < 0.0:
                    angle = math.pi if cent.x >= h_cx else -math.pi
                u = max(0.528, min(0.722, 0.625 + (angle / math.pi) * 0.095))
                v = max(
                    0.262,
                    min(
                        0.488,
                        0.264 + max(0.0, min(1.0, (co.z - hc_z0) / max(0.04, hc_z1 - hc_z0))) * 0.222,
                    ),
                )
                return u, v

            assign_uv_loops(h_obj, hair_cap_uv_fn)
        else:
            assign_constant_uv_box(h_obj, 0.528, 0.264, 0.562, 0.340)
    for s_obj in shoe_groups["upper"]:
        assign_constant_uv_box(s_obj, 0.736, 0.256, 0.984, 0.484)
    for c_obj in shoe_groups["cap"]:
        assign_constant_uv_box(c_obj, 0.516, 0.016, 0.722, 0.242)
    for l_obj in shoe_groups["lace"]:
        assign_constant_uv_box(l_obj, 0.736, 0.016, 0.858, 0.242)
    for r_obj in shoe_groups["sole"]:
        assign_constant_uv_box(r_obj, 0.870, 0.016, 0.984, 0.242)

    # --- Paint 1024x1024 Graphic-Novel Texture Atlas (bottom-up rows, sRGB [0..1]) ---
    img_arr = np.zeros((ATLAS_SIZE, ATLAS_SIZE, 4), dtype=np.float32)
    img_arr[:, :, 3] = 1.0

    yy, xx = np.mgrid[0:ATLAS_SIZE, 0:ATLAS_SIZE]
    u_grid = (xx + 0.5) / ATLAS_SIZE
    v_grid = (yy + 0.5) / ATLAS_SIZE

    def fill_rect(u0: float, v0: float, u1: float, v1: float, rgb: tuple[float, float, float]) -> None:
        x0, x1 = max(0, int(u0 * ATLAS_SIZE)), min(ATLAS_SIZE, int(math.ceil(u1 * ATLAS_SIZE)))
        y0, y1 = max(0, int(v0 * ATLAS_SIZE)), min(ATLAS_SIZE, int(math.ceil(v1 * ATLAS_SIZE)))
        img_arr[y0:y1, x0:x1, :3] = rgb

    def draw_soft_ellipse(
        uc: float,
        vc: float,
        ru: float,
        rv: float,
        rgb: tuple[float, float, float],
        alpha: float = 1.0,
        feather: float = 0.35,
    ) -> None:
        pad_u, pad_v = ru * 1.4, rv * 1.4
        x0 = max(0, int((uc - pad_u) * ATLAS_SIZE))
        x1 = min(ATLAS_SIZE, int(math.ceil((uc + pad_u) * ATLAS_SIZE)))
        y0 = max(0, int((vc - pad_v) * ATLAS_SIZE))
        y1 = min(ATLAS_SIZE, int(math.ceil((vc + pad_v) * ATLAS_SIZE)))
        if x1 <= x0 or y1 <= y0:
            return
        du = (u_grid[y0:y1, x0:x1] - uc) / max(1e-5, ru)
        dv = (v_grid[y0:y1, x0:x1] - vc) / max(1e-5, rv)
        dist = np.sqrt(du * du + dv * dv)
        w = np.clip((1.0 - dist) / max(1e-3, feather), 0.0, 1.0) * alpha
        patch = img_arr[y0:y1, x0:x1, :3]
        col = np.array(rgb, dtype=np.float32)
        img_arr[y0:y1, x0:x1, :3] = patch * (1.0 - w[..., None]) + col * w[..., None]

    def draw_ink_curve(
        pts_uv: Sequence[tuple[float, float]],
        width_px: float,
        rgb: tuple[float, float, float],
        alpha: float = 0.85,
        dashed: bool = False,
    ) -> None:
        pts_arr = np.asarray(pts_uv, dtype=np.float32) * ATLAS_SIZE
        margin = int(width_px * 2 + 4)
        x0 = max(0, int(pts_arr[:, 0].min()) - margin)
        x1 = min(ATLAS_SIZE, int(pts_arr[:, 0].max()) + margin + 1)
        y0 = max(0, int(pts_arr[:, 1].min()) - margin)
        y1 = min(ATLAS_SIZE, int(pts_arr[:, 1].max()) + margin + 1)
        if x1 <= x0 or y1 <= y0:
            return
        gx = xx[y0:y1, x0:x1].astype(np.float32) + 0.5
        gy = yy[y0:y1, x0:x1].astype(np.float32) + 0.5
        min_d = np.full(gx.shape, 1e6, dtype=np.float32)
        cum_len = 0.0
        best_s = np.zeros(gx.shape, dtype=np.float32)
        for i in range(len(pts_arr) - 1):
            ax, ay = pts_arr[i]
            bx, by = pts_arr[i + 1]
            vx, vy = bx - ax, by - ay
            seg_len2 = vx * vx + vy * vy
            if seg_len2 < 1e-5:
                continue
            t = np.clip(((gx - ax) * vx + (gy - ay) * vy) / seg_len2, 0.0, 1.0)
            px = ax + t * vx
            py = ay + t * vy
            d = np.hypot(gx - px, gy - py)
            mask = d < min_d
            min_d = np.where(mask, d, min_d)
            seg_len = math.sqrt(float(seg_len2))
            best_s = np.where(mask, cum_len + t * seg_len, best_s)
            cum_len += seg_len
        w = np.clip(1.0 - (min_d - width_px * 0.5) / max(0.8, width_px * 0.6), 0.0, 1.0) * alpha
        if dashed:
            dash_mask = (np.sin(best_s * 0.9) > 0.1).astype(np.float32)
            w *= dash_mask
        col = np.array(rgb, dtype=np.float32)
        patch = img_arr[y0:y1, x0:x1, :3]
        img_arr[y0:y1, x0:x1, :3] = patch * (1.0 - w[..., None]) + col * w[..., None]

    # ==== Paint Q1: Warm Olive-Tan Skin + Landmark-Aligned Graphic-Novel Face Ink ====
    skin_base = (0.76, 0.55, 0.40)
    skin_mid = (0.68, 0.47, 0.33)
    skin_shadow = (0.56, 0.37, 0.25)
    skin_high = (0.83, 0.64, 0.49)
    ink_dark = (0.12, 0.08, 0.07)
    lip_upper = (0.57, 0.36, 0.27)
    lip_lower = (0.67, 0.45, 0.34)

    fill_rect(0.0, 0.5, 0.5, 1.0, skin_base)
    neck_uv = head_proj(landmarks["neck01"][0])
    jaw_uv = head_proj(landmarks["joint-jaw"][0])
    # Soft chin/neck shadow below jaw (v decreases downward toward collarbones)
    draw_soft_ellipse(0.180, neck_uv[1] - 0.015, 0.070, 0.045, skin_shadow, alpha=0.40, feather=0.80)
    draw_soft_ellipse(0.180, jaw_uv[1] + 0.018, 0.036, 0.025, skin_high, alpha=0.22, feather=0.75)

    # Landmark-aligned eyes, eyelid creases, cheekbones, nose, and lips
    l_eye_uv = head_proj(landmarks["helper-l-eye"][0])
    r_eye_uv = head_proj(landmarks["helper-r-eye"][0])
    for side_s, e_uv in ((1.0, l_eye_uv), (-1.0, r_eye_uv)):
        draw_soft_ellipse(e_uv[0], e_uv[1] + 0.004, 0.022, 0.012, skin_mid, alpha=0.40, feather=0.70)
        draw_soft_ellipse(e_uv[0] + side_s * 0.024, e_uv[1] - 0.036, 0.026, 0.022, skin_mid, alpha=0.30, feather=0.78)
        draw_ink_curve(
            (
                (e_uv[0] - side_s * 0.014, e_uv[1] + 0.001),
                (e_uv[0], e_uv[1] + 0.006),
                (e_uv[0] + side_s * 0.015, e_uv[1] + 0.002),
            ),
            1.7,
            ink_dark,
            0.78,
        )
        draw_ink_curve(
            (
                (e_uv[0] - side_s * 0.009, e_uv[1] - 0.006),
                (e_uv[0] + side_s * 0.009, e_uv[1] - 0.006),
            ),
            0.9,
            ink_dark,
            0.28,
        )

    nose_c = face_refs.get("nose", Vector((0.0, -0.10, 0.5 * (l_eye_uv[1] + jaw_uv[1]))))
    nose_uv = head_proj(nose_c)
    draw_soft_ellipse(nose_uv[0], nose_uv[1] - 0.004, 0.013, 0.014, skin_mid, alpha=0.32, feather=0.70)
    draw_ink_curve(((nose_uv[0] - 0.006, nose_uv[1] + 0.022), (nose_uv[0] - 0.005, nose_uv[1] - 0.002)), 0.9, ink_dark, 0.22)
    draw_ink_curve(((nose_uv[0] - 0.009, nose_uv[1] - 0.007), (nose_uv[0], nose_uv[1] - 0.010), (nose_uv[0] + 0.009, nose_uv[1] - 0.007)), 1.2, ink_dark, 0.45)

    lips_c = face_refs.get("lips", Vector((0.0, -0.10, jaw_uv[1] + 0.025)))
    lips_uv = head_proj(lips_c)
    draw_soft_ellipse(lips_uv[0], lips_uv[1] + 0.004, 0.017, 0.005, lip_upper, alpha=0.70, feather=0.45)
    draw_soft_ellipse(lips_uv[0], lips_uv[1] - 0.005, 0.015, 0.006, lip_lower, alpha=0.68, feather=0.45)
    draw_soft_ellipse(lips_uv[0], lips_uv[1] - 0.015, 0.010, 0.004, skin_shadow, alpha=0.32, feather=0.70)
    draw_ink_curve(
        (
            (lips_uv[0] - 0.018, lips_uv[1] + 0.000),
            (lips_uv[0] - 0.006, lips_uv[1] + 0.001),
            (lips_uv[0] + 0.006, lips_uv[1] + 0.001),
            (lips_uv[0] + 0.018, lips_uv[1] + 0.000),
        ),
        1.3,
        ink_dark,
        0.65,
    )

    # Ear inner creases
    draw_ink_curve(((0.068, l_eye_uv[1] - 0.002), (0.064, l_eye_uv[1] - 0.016), (0.069, l_eye_uv[1] - 0.028)), 1.2, ink_dark, 0.40)
    draw_ink_curve(((0.292, r_eye_uv[1] - 0.002), (0.296, r_eye_uv[1] - 0.016), (0.291, r_eye_uv[1] - 0.028)), 1.2, ink_dark, 0.40)

    # Arms/Hands gentle tonal warmth (no artificial forearm lines)
    draw_soft_ellipse(0.422, 0.78, 0.042, 0.16, skin_mid, alpha=0.22, feather=0.80)
    draw_soft_ellipse(0.422, 0.60, 0.036, 0.055, skin_mid, alpha=0.18, feather=0.75)

    # Eye swatches in Q1 bottom-right
    fill_rect(0.360, 0.510, 0.391, 0.542, (0.91, 0.90, 0.88))
    fill_rect(0.391, 0.510, 0.421, 0.542, (0.18, 0.10, 0.06))
    fill_rect(0.421, 0.510, 0.451, 0.542, (0.02, 0.02, 0.025))
    fill_rect(0.451, 0.510, 0.490, 0.542, (0.98, 0.99, 1.0))

    # ==== Paint Q2: Olive-Grey Cotton T-Shirt + Soft Painterly Drape Folds + Stitches ====
    tee_base = (0.39, 0.37, 0.30)
    tee_shadow = (0.27, 0.25, 0.20)
    tee_high = (0.47, 0.45, 0.37)
    tee_ink = (0.15, 0.14, 0.11)

    fill_rect(0.5, 0.5, 1.0, 1.0, tee_base)
    rng = np.random.default_rng(42)
    q2_noise = (rng.normal(0.0, 0.011, (512, 512, 1))).astype(np.float32)
    img_arr[512:1024, 512:1024, :3] = np.clip(img_arr[512:1024, 512:1024, :3] + q2_noise, 0.0, 1.0)

    # Front T-shirt soft chest highlights, underarm drape shadows, subtle thin crease ticks
    draw_soft_ellipse(0.629, 0.855, 0.068, 0.052, tee_high, alpha=0.45, feather=0.75)
    draw_soft_ellipse(0.578, 0.800, 0.024, 0.060, tee_shadow, alpha=0.52, feather=0.70)
    draw_soft_ellipse(0.680, 0.800, 0.024, 0.060, tee_shadow, alpha=0.52, feather=0.70)
    draw_soft_ellipse(0.629, 0.625, 0.075, 0.070, tee_shadow, alpha=0.32, feather=0.80)
    for sx in (-1.0, 1.0):
        cx_arm = 0.629 + sx * 0.050
        draw_soft_ellipse(cx_arm - sx * 0.012, 0.795, 0.014, 0.032, tee_shadow, alpha=0.25, feather=0.75)
        draw_ink_curve(((cx_arm, 0.825), (cx_arm - sx * 0.020, 0.778)), 0.9, tee_ink, 0.16)
        draw_ink_curve(((0.629 + sx * 0.064, 0.746), (0.629 + sx * 0.105, 0.754)), 0.9, tee_ink, 0.28, dashed=True)
    draw_ink_curve(((0.522, 0.556), (0.736, 0.556)), 0.9, tee_ink, 0.30, dashed=True)

    # Back T-shirt
    draw_soft_ellipse(0.871, 0.850, 0.072, 0.055, tee_high, alpha=0.35, feather=0.75)
    for sx in (-1.0, 1.0):
        draw_ink_curve(((0.871 + sx * 0.045, 0.825), (0.871 + sx * 0.022, 0.768)), 0.9, tee_ink, 0.16)
    draw_ink_curve(((0.764, 0.556), (0.978, 0.556)), 0.9, tee_ink, 0.30, dashed=True)

    # Ribbed crew-neck collar strip (v: 0.510..0.538)
    fill_rect(0.505, 0.505, 0.995, 0.540, (0.31, 0.30, 0.24))
    draw_ink_curve(((0.510, 0.524), (0.990, 0.524)), 1.0, tee_ink, 0.35, dashed=True)

    # ==== Paint Q3: Muted Slate-Blue Washed Denim + Soft Crease Shading ====
    denim_base = (0.29, 0.40, 0.50)
    denim_fade = (0.40, 0.52, 0.62)
    denim_dark = (0.19, 0.28, 0.37)
    denim_ink = (0.11, 0.17, 0.24)
    stitch_gold = (0.60, 0.49, 0.33)

    fill_rect(0.0, 0.0, 0.5, 0.5, denim_base)
    twill = (rng.normal(0.0, 0.014, (512, 512, 1))).astype(np.float32)
    diag_weave = (0.009 * np.sin((xx[0:512, 0:512] + yy[0:512, 0:512]) * 1.15))[..., None].astype(np.float32)
    img_arr[0:512, 0:512, :3] = np.clip(img_arr[0:512, 0:512, :3] + twill + diag_weave, 0.0, 1.0)

    # Front Jeans (u: 0.016..0.242, v: 0.016..0.484)
    for leg_u in (0.088, 0.170):
        draw_soft_ellipse(leg_u, 0.305, 0.026, 0.082, denim_fade, alpha=0.62, feather=0.75)
        draw_soft_ellipse(leg_u, 0.215, 0.022, 0.040, denim_fade, alpha=0.50, feather=0.72)
        draw_soft_ellipse(leg_u, 0.230, 0.024, 0.016, denim_dark, alpha=0.45, feather=0.70)
        draw_soft_ellipse(leg_u, 0.052, 0.025, 0.016, denim_dark, alpha=0.42, feather=0.70)
        draw_ink_curve(((leg_u - 0.018, 0.234), (leg_u, 0.228), (leg_u + 0.018, 0.232)), 1.0, denim_ink, 0.34)
        draw_ink_curve(((leg_u - 0.018, 0.054), (leg_u, 0.048), (leg_u + 0.018, 0.052)), 1.1, denim_ink, 0.36)
    draw_ink_curve(((0.129, 0.445), (0.129, 0.382), (0.121, 0.370)), 1.2, stitch_gold, 0.60, dashed=True)
    draw_ink_curve(((0.129, 0.445), (0.129, 0.365)), 1.2, denim_ink, 0.48)
    draw_ink_curve(((0.058, 0.425), (0.072, 0.398), (0.092, 0.390)), 1.2, denim_ink, 0.44)
    draw_ink_curve(((0.200, 0.425), (0.186, 0.398), (0.166, 0.390)), 1.2, denim_ink, 0.44)
    draw_ink_curve(((0.044, 0.420), (0.052, 0.025)), 1.2, denim_ink, 0.46)
    draw_ink_curve(((0.214, 0.420), (0.206, 0.025)), 1.2, denim_ink, 0.46)

    # Back Jeans (u: 0.258..0.484, v: 0.016..0.484)
    for leg_u in (0.330, 0.412):
        draw_soft_ellipse(leg_u, 0.310, 0.026, 0.075, denim_fade, alpha=0.40, feather=0.78)
        draw_soft_ellipse(leg_u, 0.228, 0.026, 0.022, denim_dark, alpha=0.50, feather=0.70)
        draw_ink_curve(((leg_u - 0.018, 0.230), (leg_u, 0.224), (leg_u + 0.018, 0.228)), 1.1, denim_ink, 0.34)
        draw_ink_curve(((leg_u - 0.018, 0.052), (leg_u + 0.018, 0.050)), 1.1, denim_ink, 0.34)
    for pk_u in (0.336, 0.406):
        draw_ink_curve(
            (
                (pk_u - 0.022, 0.425),
                (pk_u + 0.022, 0.425),
                (pk_u + 0.020, 0.376),
                (pk_u, 0.366),
                (pk_u - 0.020, 0.376),
                (pk_u - 0.022, 0.425),
            ),
            1.2,
            denim_ink,
            0.52,
        )
        draw_ink_curve(((pk_u - 0.017, 0.398), (pk_u, 0.391), (pk_u + 0.017, 0.398)), 1.1, stitch_gold, 0.62)

    # ==== Paint Q4: Hair + Trail Shoes ====
    # Hair swatch (u: 0.50..0.728, v: 0.25..0.50)
    fill_rect(0.500, 0.250, 0.728, 0.500, (0.044, 0.048, 0.060))
    # Isolated dark ink corner for eyebrows (never bleeds hair sheen strokes)
    fill_rect(0.500, 0.250, 0.524, 0.290, (0.032, 0.035, 0.042))

    # Fine brushed hair grain + directional striations across cap region (tilted ~up-right in front)
    hx0, hx1 = int(0.526 * ATLAS_SIZE), int(0.724 * ATLAS_SIZE)
    hy0, hy1 = int(0.260 * ATLAS_SIZE), int(0.492 * ATLAS_SIZE)
    hu = u_grid[hy0:hy1, hx0:hx1]
    hv = v_grid[hy0:hy1, hx0:hx1]
    front_zone = np.clip(1.0 - np.abs(hu - 0.625) / 0.056, 0.0, 1.0)
    u_tilted = hu - (0.14 * front_zone) * (hv - 0.34)
    hair_grain = (
        0.022 * np.sin(u_tilted * 460.0)
        + 0.014 * np.cos(u_tilted * 920.0 + hv * 18.0)
    )[..., None].astype(np.float32)
    img_arr[hy0:hy1, hx0:hx1, :3] = np.clip(
        img_arr[hy0:hy1, hx0:hx1, :3]
        + hair_grain * np.array([0.55, 0.65, 0.85], dtype=np.float32),
        0.0,
        1.0,
    )

    # Explicit upward-brushed front hair strands (18 alternating deep-ink grooves & slate sheen strokes)
    for i in range(18):
        t = i / 17.0
        u_base = 0.576 + t * 0.098
        side_tilt = (u_base - 0.625) * 0.14
        du = 0.011 + 0.004 * math.sin(i * 1.7) + side_tilt
        v0_s = 0.335 + 0.008 * ((i * 3) % 3)
        v1_s = 0.470 + 0.012 * ((i + 1) % 2)
        vm_s = 0.5 * (v0_s + v1_s)
        pts_s = (
            (u_base, v0_s),
            (u_base + du * 0.42, vm_s),
            (u_base + du, v1_s),
        )
        if i % 2 == 0:
            draw_ink_curve(pts_s, 1.6, (0.012, 0.014, 0.020), 0.80)
        else:
            col_hi = (0.22, 0.25, 0.31) if i % 4 == 1 else (0.14, 0.16, 0.21)
            draw_ink_curve(pts_s, 1.3, col_hi, 0.64)

    # Broken graphic-novel brushed sheen band along upper-front crest (v: 0.42..0.465)
    for k in range(7):
        uk = 0.587 + k * 0.0125
        vk = 0.434 + 0.008 * math.sin(k * 1.4)
        draw_soft_ellipse(uk, vk, 0.0042, 0.016, (0.19, 0.22, 0.28), alpha=0.40, feather=0.45)
        draw_ink_curve(
            ((uk + 0.004, vk - 0.014), (uk + 0.007, vk + 0.016)),
            1.2,
            (0.014, 0.016, 0.022),
            0.72,
        )

    # Subtle darker vertical grain strokes on sides & rear so sides stay neat
    for u_s in (0.538, 0.548, 0.558, 0.566, 0.684, 0.692, 0.702, 0.712):
        draw_ink_curve(((u_s, 0.280), (u_s, 0.435)), 1.0, (0.018, 0.020, 0.028), 0.50)

    # Trail Shoe Suede Upper (u: 0.728..1.000, v: 0.25..0.50)
    fill_rect(0.728, 0.250, 1.000, 0.500, (0.50, 0.36, 0.24))
    suede_grain = (rng.normal(0.0, 0.018, (256, 278, 1))).astype(np.float32)
    img_arr[256:512, 746:1024, :3] = np.clip(img_arr[256:512, 746:1024, :3] + suede_grain, 0.0, 1.0)
    draw_ink_curve(((0.745, 0.365), (0.860, 0.340), (0.975, 0.365)), 1.5, (0.20, 0.13, 0.08), 0.72)
    draw_ink_curve(((0.745, 0.358), (0.860, 0.333), (0.975, 0.358)), 1.2, (0.72, 0.58, 0.42), 0.70, dashed=True)

    # Trail Shoe Mudguard Cap & Collar (u: 0.500..0.728, v: 0.00..0.250)
    fill_rect(0.500, 0.000, 0.728, 0.250, (0.37, 0.25, 0.16))
    draw_ink_curve(((0.520, 0.120), (0.710, 0.120)), 1.3, (0.68, 0.53, 0.38), 0.65, dashed=True)

    # Tongue & Laces (u: 0.728..0.864, v: 0.00..0.250)
    fill_rect(0.728, 0.000, 0.864, 0.250, (0.28, 0.19, 0.12))
    # Sole & Lugs (u: 0.864..1.000, v: 0.00..0.250)
    fill_rect(0.864, 0.000, 1.000, 0.250, (0.11, 0.095, 0.085))
    draw_ink_curve(((0.870, 0.145), (0.990, 0.145)), 2.0, (0.22, 0.19, 0.16), 0.80)

    img = bpy.data.images.new("Alex_Wen_V3_Atlas", width=ATLAS_SIZE, height=ATLAS_SIZE, alpha=True)
    img.file_format = "PNG"
    img.colorspace_settings.name = "sRGB"
    img.pixels.foreach_set(img_arr.ravel())
    img.pack()
    return img


def create_atlas_material(atlas_img: bpy.types.Image) -> bpy.types.Material:
    """Create the single shared Principled BSDF material referencing the packed PNG atlas."""
    mat = bpy.data.materials.new(name="Alex_V3_Atlas_Mat")
    if mat.node_tree is None:
        mat.use_nodes = True
    nt = mat.node_tree
    bsdf = nt.nodes.get("Principled BSDF") or nt.nodes.new(type="ShaderNodeBsdfPrincipled")
    bsdf.inputs["Base Color"].default_value = (1.0, 1.0, 1.0, 1.0)
    bsdf.inputs["Roughness"].default_value = 0.85
    bsdf.inputs["Metallic"].default_value = 0.0
    if "Specular IOR Level" in bsdf.inputs:
        bsdf.inputs["Specular IOR Level"].default_value = 0.18

    tex_node = nt.nodes.new(type="ShaderNodeTexImage")
    tex_node.image = atlas_img
    tex_node.extension = "CLIP"
    nt.links.new(tex_node.outputs["Color"], bsdf.inputs["Base Color"])
    return mat


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


def make_studio_material(name: str, srgb: tuple[float, float, float], roughness: float) -> bpy.types.Material:
    """Simple solid material for studio backdrop only (excluded from GLB export)."""
    def s2l(c: float) -> float:
        return c / 12.92 if c <= 0.04045 else ((c + 0.055) / 1.055) ** 2.4

    mat = bpy.data.materials.new(name=name)
    if mat.node_tree is None:
        mat.use_nodes = True
    bsdf = mat.node_tree.nodes.get("Principled BSDF") or mat.node_tree.nodes.new(type="ShaderNodeBsdfPrincipled")
    bsdf.inputs["Base Color"].default_value = (s2l(srgb[0]), s2l(srgb[1]), s2l(srgb[2]), 1.0)
    bsdf.inputs["Roughness"].default_value = roughness
    return mat


def setup_studio_and_render(render_dir: str) -> None:
    """Create minimal slate-concrete studio backdrop & render 4 verification views."""
    os.makedirs(render_dir, exist_ok=True)
    studio_col = ensure_collection("Studio_Backdrop")

    floor_mat = make_studio_material("Studio_Floor_Mat", (0.31, 0.36, 0.39), 0.92)
    wall_mat = make_studio_material("Studio_Wall_Mat", (0.29, 0.34, 0.38), 0.95)

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

    def add_area_light(name, loc, target, energy, color, size):
        ldata = bpy.data.lights.new(name=name, type="AREA")
        ldata.energy = energy
        ldata.color = color
        ldata.size = size
        lobj = bpy.data.objects.new(name, ldata)
        lobj.location = loc
        lobj.rotation_euler = (Vector(target) - Vector(loc)).to_track_quat("-Z", "Y").to_euler()
        studio_col.objects.link(lobj)

    add_area_light("Studio_Key_Warm", (2.2, -2.6, 2.3), (0.0, 0.0, 1.0), 190.0, (1.0, 0.90, 0.78), 1.6)
    add_area_light("Studio_Fill_Cool", (-2.6, -2.2, 1.5), (0.0, 0.0, 1.05), 110.0, (0.72, 0.85, 0.98), 2.4)
    add_area_light("Studio_Bounce_Front", (0.0, -2.0, 0.85), (0.0, 0.0, 1.50), 28.0, (0.95, 0.92, 0.88), 1.8)
    add_area_light("Studio_Rim_Cool", (-1.4, 2.4, 2.2), (0.0, 0.0, 1.1), 135.0, (0.76, 0.88, 1.0), 1.4)

    world = bpy.data.worlds.new("Studio_World")
    bpy.context.scene.world = world
    if world.node_tree is None:
        world.use_nodes = True
    bg = world.node_tree.nodes.get("Background")
    if bg is not None:
        bg.inputs["Color"].default_value = (0.14, 0.17, 0.20, 1.0)
        bg.inputs["Strength"].default_value = 0.75

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
    scene.view_settings.exposure = 0.12

    views = (
        ("render_frontal.png", 0.0, 3.25, 0.96, 0.90),
        ("render_side.png", -90.0, 3.25, 0.96, 0.90),
        ("render_oblique.png", -42.0, 3.25, 0.96, 0.90),
        ("render_face_closeup.png", -14.0, 0.72, 1.62, 1.58),
    )
    for filename, yaw_deg, dist, cam_z, tgt_z in views:
        rad = math.radians(yaw_deg)
        cam_obj.location = (dist * math.sin(rad), -dist * math.cos(rad), cam_z)
        target = Vector((0.0, 0.0, tgt_z))
        cam_obj.rotation_euler = (target - cam_obj.location).to_track_quat("-Z", "Y").to_euler()
        scene.render.filepath = os.path.join(render_dir, filename)
        bpy.ops.render.render(write_still=True)


def export_character_glb(objs: Sequence[bpy.types.Object], out_path: str) -> None:
    """Export character meshes + packed UV atlas to a glTF 2.0 Binary (`.glb`)."""
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
        export_image_format="AUTO",
        export_texcoords=True,
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
    parser = argparse.ArgumentParser(description="Generate Alex Wen (Set 1 v3) UV-textured MPFB2 GLB.")
    parser.add_argument("--export-glb", default="", help="Output path for .glb binary export.")
    parser.add_argument("--render-dir", default="", help="Optional directory for 4-angle PNG previews.")
    return parser.parse_args(argv)


def main() -> None:
    args = parse_args(sys.argv)
    bpy.ops.wm.read_factory_settings(use_empty=True)
    char_col = ensure_collection("Alex_Wen_Set01_V3")

    # Placeholder material replaced with packed atlas material once UVs + image are created
    placeholder_mat = bpy.data.materials.new("Alex_V3_Atlas_Temp")

    body_obj, tights_obj, landmarks, bone_segs, face_refs = create_posed_mpfb_body()

    tee, collar, hem_z, sleeve_cut_z, chest_z = build_tshirt_from_tights(
        tights_obj, landmarks, bone_segs, char_col, placeholder_mat
    )
    jeans, cuffs, waist_z, knee_z, cuff_z = build_loose_jeans_from_body(
        body_obj, landmarks, hem_z, char_col, placeholder_mat
    )
    shoe_groups = build_trail_shoes_at_feet(landmarks, char_col, placeholder_mat)
    face_groups = build_eyes_brows_hair_and_skin(
        body_obj, landmarks, sleeve_cut_z, char_col, placeholder_mat
    )

    bpy.data.objects.remove(tights_obj, do_unlink=True)
    bpy.data.objects.remove(body_obj, do_unlink=True)

    atlas_img = assign_all_uvs_and_build_atlas(
        head_skin=face_groups["head_skin"][0],
        arms_skin=face_groups["arms_skin"][0],
        tee=tee,
        collar=collar,
        jeans=jeans,
        cuffs=cuffs,
        shoe_groups=shoe_groups,
        eye_groups=face_groups,
        hair_objs=face_groups["hair"],
        landmarks=landmarks,
        face_refs=face_refs,
        hem_z=hem_z,
        waist_z=waist_z,
        knee_z=knee_z,
        cuff_z=cuff_z,
        chest_z=chest_z,
    )
    atlas_mat = create_atlas_material(atlas_img)

    objs: list[bpy.types.Object] = [
        tee,
        collar,
        jeans,
        *cuffs,
        *shoe_groups["sole"],
        *shoe_groups["upper"],
        *shoe_groups["cap"],
        *shoe_groups["lace"],
        *face_groups["head_skin"],
        *face_groups["arms_skin"],
        *face_groups["sclera"],
        *face_groups["iris"],
        *face_groups["pupil"],
        *face_groups["catchlight"],
        *face_groups["hair"],
    ]
    for obj in objs:
        obj.data.materials.clear()
        obj.data.materials.append(atlas_mat)
    bpy.data.materials.remove(placeholder_mat)

    height_m = normalize_character_height_and_grounding(objs)
    tri_count = sum(len(p.vertices) - 2 for o in objs for p in o.data.polygons)
    print(f"BUILT MPFB2 Alex_Wen_Set01_V3: objects={len(objs)} triangles={tri_count} height={height_m:.3f}m")

    if args.export_glb:
        export_character_glb(objs, args.export_glb)
        print(f"EXPORTED GLB: {args.export_glb} ({os.path.getsize(args.export_glb)} bytes)")

    if args.render_dir:
        setup_studio_and_render(args.render_dir)
        print(f"RENDERED PREVIEWS: {args.render_dir}")


if __name__ == "__main__":
    main()
