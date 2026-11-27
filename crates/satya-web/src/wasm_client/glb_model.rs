//! Spec-compliant glTF 2.0 Binary (`.glb`) Parser, Encoder, and Procedural Low-Poly Articulated
//! Character & Prop Generator shared by `#notebook-pane-model` and `#notebook-pane-maps`.
//!
//! Built-in presets (`valkyrie_rig`, `recon_mk4`, `secops_sentry`, `operative_light`, `sector4_airlock`)
//! serialize to genuine glTF 2.0 `.glb` byte streams via [`encode_glb_bytes`] and load through
//! [`parse_glb_bytes`], exercising the exact same codepath used when importing external `.glb` files
//! from Blender, Quaternius, Kenney, or KayKit.

use crate::wasm_client::glb_png::{DecodedPng, decode_png_rgba8};
use crate::wasm_client::wgpu_shared::{cross3, normalize3};
use bytemuck::{Pod, Zeroable};
use serde_json::{Value, json};
use std::collections::HashMap;

const GLB_MAGIC: u32 = 0x4654_6C67; // b"glTF"
const GLB_VERSION_2: u32 = 2;
const CHUNK_JSON: u32 = 0x4E4F_534A; // b"JSON"
const CHUNK_BIN: u32 = 0x004E_4942; // b"BIN\0"

/// Anatomical / structural hit-zone tag stored in `GlbVertex::bary_zone[3]` for shader highlighting.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HitZoneTag {
    BaseOrProp = 0,
    HeadOptics = 1,
    TorsoCarapace = 2,
    ArmsWeapon = 3,
    LegsBoots = 4,
    PackThrusters = 5,
}

impl HitZoneTag {
    #[must_use]
    pub fn from_name(name: &str) -> Self {
        let low = name.to_ascii_lowercase();
        let has = |keys: &[&str]| keys.iter().any(|k| low.contains(k));
        if has(&["head", "helm", "visor", "dome"]) {
            Self::HeadOptics
        } else if has(&["pack", "thrust", "crane"]) {
            Self::PackThrusters
        } else if has(&["arm", "hand", "weapon", "rifle", "gun", "valve"]) {
            Self::ArmsWeapon
        } else if has(&["leg", "boot", "thigh", "shin", "foot"]) {
            Self::LegsBoots
        } else if has(&["torso", "chest", "pelvis", "body", "door", "plate"]) {
            Self::TorsoCarapace
        } else {
            Self::BaseOrProp
        }
    }

    #[must_use]
    pub const fn as_f32(self) -> f32 {
        (self as u8) as f32
    }
}

/// Interleaved GPU triangle vertex (`64` bytes) for `.glb` characters, props, and tactical map tiles.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct GlbVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    /// RGB base color (`0..=1`) and `a` = tintable mask (`1.0` = apply faction/primary color tint, `0.0` = fixed material color).
    pub color: [f32; 4],
    /// `xyz` = triangle barycentric coords (`[1,0,0]`, `[0,1,0]`, `[0,0,1]`) for anti-aliased WGSL wireframe; `w` = [`HitZoneTag`].
    pub bary_zone: [f32; 4],
    /// Normalized glTF `TEXCOORD_0` UV (`0..=1`), or `[-1.0, -1.0]` sentinel for untextured vertices.
    pub uv: [f32; 2],
}

/// Packed RGBA8 albedo texture atlas extracted from embedded glTF 2.0 PNG images.
#[derive(Debug, Clone)]
pub struct GlbTextureAtlas {
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
}

/// Named submesh preserved for round-trip `.glb` export (`File -> Import -> glTF 2.0` in Blender).
#[derive(Debug, Clone)]
pub struct GlbSubmesh {
    pub name: String,
    pub base_color: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub tintable: bool,
    pub zone: HitZoneTag,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub vertex_colors: Vec<[f32; 3]>,
    pub texture_index: Option<usize>,
    pub indices: Vec<u32>,
}

/// Parsed, GPU-ready 3D model decoded from `.glb` binary bytes.
#[derive(Debug, Clone)]
pub struct LoadedGlbModel {
    pub name: String,
    pub vertices: Vec<GlbVertex>,
    pub submeshes: Vec<GlbSubmesh>,
    pub albedo_atlas: Option<GlbTextureAtlas>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    pub triangle_count: usize,
}

impl LoadedGlbModel {
    #[must_use]
    pub fn height_m(&self) -> f32 {
        (self.bounds_max[1] - self.bounds_min[1]).max(0.05)
    }

    #[must_use]
    pub fn max_extent_m(&self) -> f32 {
        let dx = self.bounds_max[0] - self.bounds_min[0];
        let dy = self.bounds_max[1] - self.bounds_min[1];
        let dz = self.bounds_max[2] - self.bounds_min[2];
        dx.max(dy).max(dz).max(0.1)
    }
}

const MAT4_IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

fn mat4_mul(a: [f32; 16], b: [f32; 16]) -> [f32; 16] {
    let mut out = [0.0_f32; 16];
    for col in 0..4 {
        for row in 0..4 {
            out[col * 4 + row] = (0..4).map(|k| a[k * 4 + row] * b[col * 4 + k]).sum();
        }
    }
    out
}

fn mat4_from_trs(t: [f32; 3], q: [f32; 4], s: [f32; 3]) -> [f32; 16] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let (x2, y2, z2) = (x + x, y + y, z + z);
    let (xx, xy, xz) = (x * x2, x * y2, x * z2);
    let (yy, yz, zz) = (y * y2, y * z2, z * z2);
    let (wx, wy, wz) = (w * x2, w * y2, w * z2);
    [
        (1.0 - (yy + zz)) * s[0],
        (xy + wz) * s[0],
        (xz - wy) * s[0],
        0.0,
        (xy - wz) * s[1],
        (1.0 - (xx + zz)) * s[1],
        (yz + wx) * s[1],
        0.0,
        (xz + wy) * s[2],
        (yz - wx) * s[2],
        (1.0 - (xx + yy)) * s[2],
        0.0,
        t[0],
        t[1],
        t[2],
        1.0,
    ]
}

fn transform_point(m: &[f32; 16], p: [f32; 3]) -> [f32; 3] {
    [
        m[0] * p[0] + m[4] * p[1] + m[8] * p[2] + m[12],
        m[1] * p[0] + m[5] * p[1] + m[9] * p[2] + m[13],
        m[2] * p[0] + m[6] * p[1] + m[10] * p[2] + m[14],
    ]
}

fn transform_normal(m: &[f32; 16], n: [f32; 3]) -> [f32; 3] {
    normalize3([
        m[0] * n[0] + m[4] * n[1] + m[8] * n[2],
        m[1] * n[0] + m[5] * n[1] + m[9] * n[2],
        m[2] * n[0] + m[6] * n[1] + m[10] * n[2],
    ])
}

fn compute_face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    normalize3(cross3(u, v))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    let s = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn read_f32_le(bytes: &[u8], offset: usize) -> Option<f32> {
    Some(f32::from_bits(read_u32_le(bytes, offset)?))
}

fn read_u16_le(bytes: &[u8], offset: usize) -> Option<u16> {
    let s = bytes.get(offset..offset + 2)?;
    Some(u16::from_le_bytes([s[0], s[1]]))
}

fn accessor_buffer_offset_and_stride(
    gltf: &Value,
    acc_idx: usize,
    min_stride: usize,
) -> Result<(usize, u64, usize, usize), String> {
    let acc = gltf
        .get("accessors")
        .and_then(Value::as_array)
        .and_then(|a| a.get(acc_idx))
        .ok_or_else(|| format!("Missing accessor {acc_idx}"))?;
    let count = acc.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
    let comp_type = acc
        .get("componentType")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let bv_idx = acc
        .get("bufferView")
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("Accessor {acc_idx} missing bufferView"))? as usize;
    let bv = gltf
        .get("bufferViews")
        .and_then(Value::as_array)
        .and_then(|b| b.get(bv_idx))
        .ok_or_else(|| format!("Missing bufferView {bv_idx}"))?;
    let base_offset = (bv.get("byteOffset").and_then(Value::as_u64).unwrap_or(0)
        + acc.get("byteOffset").and_then(Value::as_u64).unwrap_or(0))
        as usize;
    let stride = bv
        .get("byteStride")
        .and_then(Value::as_u64)
        .unwrap_or(min_stride as u64)
        .max(min_stride as u64) as usize;
    Ok((count, comp_type, base_offset, stride))
}

fn read_accessor_vec3_f32(
    gltf: &Value,
    bin: &[u8],
    acc_idx: usize,
) -> Result<Vec<[f32; 3]>, String> {
    let (count, comp_type, base_offset, stride) =
        accessor_buffer_offset_and_stride(gltf, acc_idx, 12)?;
    if comp_type != 5126 {
        return Err(format!(
            "Unsupported VEC3 componentType {comp_type} (expected 5126 FLOAT)"
        ));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = base_offset + i * stride;
        let x = read_f32_le(bin, off).ok_or_else(|| "Truncated GLB VEC3 buffer".to_string())?;
        let y = read_f32_le(bin, off + 4).ok_or_else(|| "Truncated GLB VEC3 buffer".to_string())?;
        let z = read_f32_le(bin, off + 8).ok_or_else(|| "Truncated GLB VEC3 buffer".to_string())?;
        out.push([x, y, z]);
    }
    Ok(out)
}

fn read_accessor_vec2_f32(
    gltf: &Value,
    bin: &[u8],
    acc_idx: usize,
) -> Result<Vec<[f32; 2]>, String> {
    let (count, comp_type, base_offset, stride) =
        accessor_buffer_offset_and_stride(gltf, acc_idx, 8)?;
    if comp_type != 5126 {
        return Err(format!(
            "Unsupported VEC2 componentType {comp_type} (expected 5126 FLOAT)"
        ));
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = base_offset + i * stride;
        let u = read_f32_le(bin, off).ok_or_else(|| "Truncated GLB VEC2 buffer".to_string())?;
        let v =
            read_f32_le(bin, off + 4).ok_or_else(|| "Truncated GLB VEC2 buffer".to_string())?;
        out.push([u, v]);
    }
    Ok(out)
}

fn read_accessor_color_rgb(
    gltf: &Value,
    bin: &[u8],
    acc_idx: usize,
) -> Result<Vec<[f32; 3]>, String> {
    let acc = gltf
        .get("accessors")
        .and_then(Value::as_array)
        .and_then(|a| a.get(acc_idx))
        .ok_or_else(|| format!("Missing accessor {acc_idx}"))?;
    let vec_type = acc.get("type").and_then(Value::as_str).unwrap_or("VEC3");
    let comps = if vec_type == "VEC4" { 4_usize } else { 3_usize };
    let comp_type = acc
        .get("componentType")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let comp_bytes = match comp_type {
        5121 => 1_usize,
        5123 => 2_usize,
        5126 => 4_usize,
        other => return Err(format!("Unsupported COLOR_0 componentType {other}")),
    };
    let (count, _, base_offset, stride) =
        accessor_buffer_offset_and_stride(gltf, acc_idx, comps * comp_bytes)?;
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = base_offset + i * stride;
        let read_chan = |c: usize| -> Result<f32, String> {
            let coff = off + c * comp_bytes;
            match comp_type {
                5121 => bin
                    .get(coff)
                    .map(|&b| f32::from(b) / 255.0)
                    .ok_or_else(|| "Truncated u8 COLOR_0 buffer".to_string()),
                5123 => read_u16_le(bin, coff)
                    .map(|v| f32::from(v) / 65535.0)
                    .ok_or_else(|| "Truncated u16 COLOR_0 buffer".to_string()),
                5126 => read_f32_le(bin, coff)
                    .ok_or_else(|| "Truncated f32 COLOR_0 buffer".to_string()),
                _ => Ok(1.0),
            }
        };
        out.push([read_chan(0)?, read_chan(1)?, read_chan(2)?]);
    }
    Ok(out)
}

fn read_accessor_indices(gltf: &Value, bin: &[u8], acc_idx: usize) -> Result<Vec<u32>, String> {
    let (count, comp_type, base_offset, _) = accessor_buffer_offset_and_stride(gltf, acc_idx, 1)?;
    let mut out = Vec::with_capacity(count);
    match comp_type {
        5121 => {
            for i in 0..count {
                let b = *bin
                    .get(base_offset + i)
                    .ok_or_else(|| "Truncated u8 index buffer".to_string())?;
                out.push(u32::from(b));
            }
        }
        5123 => {
            let (_, _, _, stride) = accessor_buffer_offset_and_stride(gltf, acc_idx, 2)?;
            for i in 0..count {
                let v = read_u16_le(bin, base_offset + i * stride)
                    .ok_or_else(|| "Truncated u16 index buffer".to_string())?;
                out.push(u32::from(v));
            }
        }
        5125 => {
            let (_, _, _, stride) = accessor_buffer_offset_and_stride(gltf, acc_idx, 4)?;
            for i in 0..count {
                let v = read_u32_le(bin, base_offset + i * stride)
                    .ok_or_else(|| "Truncated u32 index buffer".to_string())?;
                out.push(v);
            }
        }
        other => return Err(format!("Unsupported glTF index componentType {other}")),
    }
    Ok(out)
}

fn node_local_matrix(node: &Value) -> [f32; 16] {
    if let Some(mat_arr) = node.get("matrix").and_then(Value::as_array)
        && mat_arr.len() == 16
    {
        let mut m = [0.0_f32; 16];
        for (i, v) in mat_arr.iter().enumerate() {
            m[i] = v.as_f64().unwrap_or(0.0) as f32;
        }
        return m;
    }
    let read_arr = |key: &str, def: [f32; 4], len: usize| -> [f32; 4] {
        let mut out = def;
        if let Some(arr) = node.get(key).and_then(Value::as_array)
            && arr.len() == len
        {
            for i in 0..len {
                out[i] = arr[i].as_f64().unwrap_or(f64::from(def[i])) as f32;
            }
        }
        out
    };
    let t = read_arr("translation", [0.0; 4], 3);
    let r = read_arr("rotation", [0.0, 0.0, 0.0, 1.0], 4);
    let s = read_arr("scale", [1.0; 4], 3);
    mat4_from_trs([t[0], t[1], t[2]], r, [s[0], s[1], s[2]])
}

fn wrap_uv01(u: f32) -> f32 {
    if !u.is_finite() {
        0.5
    } else if (0.0..=1.0).contains(&u) {
        u
    } else {
        u.rem_euclid(1.0)
    }
}

fn extract_embedded_png_for_material(
    gltf: &Value,
    bin: &[u8],
    mat: &Value,
    image_cache: &mut HashMap<usize, Option<usize>>,
    decoded_images: &mut Vec<DecodedPng>,
) -> Option<usize> {
    let tex_idx = mat
        .get("pbrMetallicRoughness")?
        .get("baseColorTexture")?
        .get("index")?
        .as_u64()? as usize;
    let img_idx = gltf
        .get("textures")?
        .as_array()?
        .get(tex_idx)?
        .get("source")?
        .as_u64()? as usize;
    if let Some(&cached) = image_cache.get(&img_idx) {
        return cached;
    }

    let decoded = (|| -> Option<DecodedPng> {
        let img = gltf.get("images")?.as_array()?.get(img_idx)?;
        if let Some(mime) = img.get("mimeType").and_then(Value::as_str)
            && mime != "image/png"
        {
            return None;
        }
        let bv_idx = img.get("bufferView")?.as_u64()? as usize;
        let bv = gltf.get("bufferViews")?.as_array()?.get(bv_idx)?;
        let byte_off = bv.get("byteOffset").and_then(Value::as_u64).unwrap_or(0) as usize;
        let byte_len = bv.get("byteLength")?.as_u64()? as usize;
        let slice = bin.get(byte_off..byte_off.saturating_add(byte_len))?;
        decode_png_rgba8(slice).ok()
    })();

    let packed_idx = decoded.map(|png| {
        let idx = decoded_images.len();
        decoded_images.push(png);
        idx
    });
    image_cache.insert(img_idx, packed_idx);
    packed_idx
}

fn pack_texture_atlas(images: Vec<DecodedPng>) -> (Option<GlbTextureAtlas>, Vec<[f32; 4]>) {
    if images.is_empty() {
        return (None, Vec::new());
    }
    if images.len() == 1 {
        let img = images.into_iter().next().expect("non-empty");
        return (
            Some(GlbTextureAtlas {
                width: img.width,
                height: img.height,
                rgba8: img.rgba8,
            }),
            vec![[0.0, 0.0, 1.0, 1.0]],
        );
    }

    const MAX_ATLAS: u32 = 2048;
    const PAD: u32 = 2;
    let mut order: Vec<usize> = (0..images.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(images[i].height));

    let mut placements = vec![(0_u32, 0_u32); images.len()];
    let mut cur_x = PAD;
    let mut cur_y = PAD;
    let mut row_h = 0_u32;
    let mut atlas_w = 0_u32;
    let mut atlas_h = 0_u32;

    for &idx in &order {
        let w = images[idx].width;
        let h = images[idx].height;
        if cur_x + w + PAD > MAX_ATLAS {
            cur_x = PAD;
            cur_y = cur_y.saturating_add(row_h + PAD);
            row_h = 0;
        }
        if cur_y + h + PAD > MAX_ATLAS || w + PAD * 2 > MAX_ATLAS {
            // Fallback to first image only if total textures exceed 2048x2048 budget
            let first = images.into_iter().next().expect("non-empty");
            let mut rects = vec![[0.0, 0.0, 1.0, 1.0]; order.len()];
            rects.truncate(order.len());
            return (
                Some(GlbTextureAtlas {
                    width: first.width,
                    height: first.height,
                    rgba8: first.rgba8,
                }),
                rects,
            );
        }
        placements[idx] = (cur_x, cur_y);
        atlas_w = atlas_w.max(cur_x + w + PAD);
        atlas_h = atlas_h.max(cur_y + h + PAD);
        cur_x += w + PAD;
        row_h = row_h.max(h);
    }

    let aw = atlas_w.max(1);
    let ah = atlas_h.max(1);
    let mut buf = vec![0_u8; (aw as usize) * (ah as usize) * 4];
    let mut rects = Vec::with_capacity(images.len());

    for (idx, img) in images.iter().enumerate() {
        let (px, py) = placements[idx];
        for row in 0..img.height {
            let src_off = (row as usize) * (img.width as usize) * 4;
            let dst_off = (((py + row) as usize) * (aw as usize) + (px as usize)) * 4;
            let len = (img.width as usize) * 4;
            buf[dst_off..dst_off + len].copy_from_slice(&img.rgba8[src_off..src_off + len]);
        }
        let inv_w = 1.0 / (aw as f32);
        let inv_h = 1.0 / (ah as f32);
        rects.push([
            (px as f32) * inv_w,
            (py as f32) * inv_h,
            ((px + img.width) as f32) * inv_w,
            ((py + img.height) as f32) * inv_h,
        ]);
    }

    (
        Some(GlbTextureAtlas {
            width: aw,
            height: ah,
            rgba8: buf,
        }),
        rects,
    )
}

/// Parses a spec-compliant glTF 2.0 Binary (`.glb`) file into GPU-ready triangles and submeshes.
pub fn parse_glb_bytes(bytes: &[u8], fallback_name: &str) -> Result<LoadedGlbModel, String> {
    if bytes.len() < 20 {
        return Err("File is too small to be a valid .glb binary".to_string());
    }
    if read_u32_le(bytes, 0).unwrap_or(0) != GLB_MAGIC {
        return Err("Invalid .glb header magic (expected 'glTF')".to_string());
    }
    let version = read_u32_le(bytes, 4).unwrap_or(0);
    if version != GLB_VERSION_2 {
        return Err(format!(
            "Unsupported glTF version {version} (expected glTF 2.0)"
        ));
    }

    let mut offset = 12_usize;
    let mut json_slice: Option<&[u8]> = None;
    let mut bin_slice: &[u8] = &[];
    while offset + 8 <= bytes.len() {
        let chunk_len = read_u32_le(bytes, offset).unwrap_or(0) as usize;
        let chunk_type = read_u32_le(bytes, offset + 4).unwrap_or(0);
        let start = offset + 8;
        let end = start.saturating_add(chunk_len);
        if end > bytes.len() {
            return Err("Truncated .glb chunk payload".to_string());
        }
        if chunk_type == CHUNK_JSON && json_slice.is_none() {
            json_slice = Some(&bytes[start..end]);
        } else if chunk_type == CHUNK_BIN && bin_slice.is_empty() {
            bin_slice = &bytes[start..end];
        }
        offset = end;
    }

    let raw_json = json_slice.ok_or_else(|| "Missing JSON chunk in .glb file".to_string())?;
    let gltf: Value =
        serde_json::from_slice(raw_json).map_err(|e| format!("Invalid glTF JSON chunk: {e}"))?;

    let nodes = gltf
        .get("nodes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let meshes = gltf
        .get("meshes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let materials = gltf
        .get("materials")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let root_indices: Vec<usize> = if let Some(scenes) =
        gltf.get("scenes").and_then(Value::as_array)
        && !scenes.is_empty()
    {
        let sc_idx = gltf.get("scene").and_then(Value::as_u64).unwrap_or(0) as usize;
        scenes
            .get(sc_idx)
            .or_else(|| scenes.first())
            .and_then(|s| s.get("nodes"))
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                    .collect()
            })
            .unwrap_or_default()
    } else {
        (0..nodes.len()).collect()
    };

    let mut submeshes: Vec<GlbSubmesh> = Vec::new();
    let mut image_cache: HashMap<usize, Option<usize>> = HashMap::new();
    let mut decoded_images: Vec<DecodedPng> = Vec::new();
    let mut stack: Vec<(usize, [f32; 16])> = root_indices
        .into_iter()
        .rev()
        .map(|idx| (idx, MAT4_IDENTITY))
        .collect();

    while let Some((node_idx, parent_mat)) = stack.pop() {
        let Some(node) = nodes.get(node_idx) else {
            continue;
        };
        let world_mat = mat4_mul(parent_mat, node_local_matrix(node));
        let node_name = node
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("node")
            .to_string();

        if let Some(mesh_idx) = node.get("mesh").and_then(Value::as_u64)
            && let Some(mesh) = meshes.get(mesh_idx as usize)
        {
            let mesh_name = mesh
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or(&node_name);
            let combined_name = format!("{node_name}_{mesh_name}");
            if let Some(prims) = mesh.get("primitives").and_then(Value::as_array) {
                for prim in prims {
                    if prim.get("mode").and_then(Value::as_u64).unwrap_or(4) != 4 {
                        continue;
                    }
                    let Some(pos_acc) = prim
                        .get("attributes")
                        .and_then(|a| a.get("POSITION"))
                        .and_then(Value::as_u64)
                    else {
                        continue;
                    };
                    let raw_pos = read_accessor_vec3_f32(&gltf, bin_slice, pos_acc as usize)?;
                    if raw_pos.is_empty() {
                        continue;
                    }
                    let raw_norm = prim
                        .get("attributes")
                        .and_then(|a| a.get("NORMAL"))
                        .and_then(Value::as_u64)
                        .and_then(|n_acc| {
                            read_accessor_vec3_f32(&gltf, bin_slice, n_acc as usize).ok()
                        });
                    let raw_uvs = prim
                        .get("attributes")
                        .and_then(|a| a.get("TEXCOORD_0"))
                        .and_then(Value::as_u64)
                        .and_then(|uv_acc| {
                            read_accessor_vec2_f32(&gltf, bin_slice, uv_acc as usize).ok()
                        })
                        .filter(|uvs| uvs.len() == raw_pos.len())
                        .unwrap_or_default();
                    let vertex_colors = prim
                        .get("attributes")
                        .and_then(|a| a.get("COLOR_0"))
                        .and_then(Value::as_u64)
                        .and_then(|c_acc| {
                            read_accessor_color_rgb(&gltf, bin_slice, c_acc as usize).ok()
                        })
                        .filter(|cols| cols.len() == raw_pos.len())
                        .unwrap_or_default();
                    let indices = if let Some(i_acc) = prim.get("indices").and_then(Value::as_u64) {
                        read_accessor_indices(&gltf, bin_slice, i_acc as usize)?
                    } else {
                        (0..raw_pos.len() as u32).collect()
                    };

                    let mut base_color = [0.72_f32, 0.78, 0.86, 1.0];
                    let mut metallic = 0.25_f32;
                    let mut roughness = 0.55_f32;
                    let mut tintable = false;
                    let mut mat_name = String::new();
                    let mut texture_index = None;

                    if let Some(mat_idx) = prim.get("material").and_then(Value::as_u64)
                        && let Some(mat) = materials.get(mat_idx as usize)
                    {
                        mat_name = mat
                            .get("name")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        let low_mat = mat_name.to_ascii_lowercase();
                        if low_mat.contains("tint") || low_mat.contains("primary") {
                            tintable = true;
                        }
                        if let Some(pbr) = mat.get("pbrMetallicRoughness") {
                            if let Some(bcf) = pbr.get("baseColorFactor").and_then(Value::as_array)
                                && bcf.len() == 4
                            {
                                for k in 0..4 {
                                    base_color[k] = bcf[k].as_f64().unwrap_or(1.0) as f32;
                                }
                            }
                            if let Some(m) = pbr.get("metallicFactor").and_then(Value::as_f64) {
                                metallic = m as f32;
                            }
                            if let Some(r) = pbr.get("roughnessFactor").and_then(Value::as_f64) {
                                roughness = r as f32;
                            }
                            if pbr.get("baseColorTexture").is_some() {
                                base_color = [
                                    pbr.get("baseColorFactor")
                                        .and_then(Value::as_array)
                                        .and_then(|a| a.first())
                                        .and_then(Value::as_f64)
                                        .unwrap_or(1.0) as f32,
                                    pbr.get("baseColorFactor")
                                        .and_then(Value::as_array)
                                        .and_then(|a| a.get(1))
                                        .and_then(Value::as_f64)
                                        .unwrap_or(1.0)
                                        as f32,
                                    pbr.get("baseColorFactor")
                                        .and_then(Value::as_array)
                                        .and_then(|a| a.get(2))
                                        .and_then(Value::as_f64)
                                        .unwrap_or(1.0)
                                        as f32,
                                    pbr.get("baseColorFactor")
                                        .and_then(Value::as_array)
                                        .and_then(|a| a.get(3))
                                        .and_then(Value::as_f64)
                                        .unwrap_or(1.0)
                                        as f32,
                                ];
                                texture_index = extract_embedded_png_for_material(
                                    &gltf,
                                    bin_slice,
                                    mat,
                                    &mut image_cache,
                                    &mut decoded_images,
                                );
                            }
                        }
                    }

                    let world_pos: Vec<[f32; 3]> = raw_pos
                        .iter()
                        .map(|&p| transform_point(&world_mat, p))
                        .collect();
                    let world_norm: Vec<[f32; 3]> = if let Some(rn) = raw_norm
                        && rn.len() == world_pos.len()
                    {
                        rn.iter()
                            .map(|&n| transform_normal(&world_mat, n))
                            .collect()
                    } else {
                        let mut gen_nrms = vec![[0.0_f32, 1.0, 0.0]; world_pos.len()];
                        for &[raw0, raw1, raw2] in indices.as_chunks::<3>().0 {
                            let (i0, i1, i2) = (raw0 as usize, raw1 as usize, raw2 as usize);
                            if i0 < world_pos.len() && i1 < world_pos.len() && i2 < world_pos.len()
                            {
                                let fnrm = compute_face_normal(
                                    world_pos[i0],
                                    world_pos[i1],
                                    world_pos[i2],
                                );
                                gen_nrms[i0] = fnrm;
                                gen_nrms[i1] = fnrm;
                                gen_nrms[i2] = fnrm;
                            }
                        }
                        gen_nrms
                    };

                    let zone = HitZoneTag::from_name(&format!("{combined_name}_{mat_name}"));
                    submeshes.push(GlbSubmesh {
                        name: node_name.clone(),
                        base_color,
                        metallic,
                        roughness,
                        tintable,
                        zone,
                        positions: world_pos,
                        normals: world_norm,
                        uvs: raw_uvs,
                        vertex_colors,
                        texture_index,
                        indices,
                    });
                }
            }
        }

        if let Some(children) = node.get("children").and_then(Value::as_array) {
            for child in children.iter().rev() {
                if let Some(c_idx) = child.as_u64() {
                    stack.push((c_idx as usize, world_mat));
                }
            }
        }
    }

    if submeshes.is_empty() {
        return Err("No triangle meshes found in .glb asset".to_string());
    }

    let (albedo_atlas, uv_rects) = pack_texture_atlas(decoded_images);
    Ok(assemble_loaded_model(
        fallback_name,
        submeshes,
        albedo_atlas,
        &uv_rects,
        true,
    ))
}

fn assemble_loaded_model(
    name: &str,
    mut submeshes: Vec<GlbSubmesh>,
    albedo_atlas: Option<GlbTextureAtlas>,
    uv_rects: &[[f32; 4]],
    ground_and_center_xz: bool,
) -> LoadedGlbModel {
    let mut bmin = [f32::INFINITY; 3];
    let mut bmax = [f32::NEG_INFINITY; 3];
    for sm in &submeshes {
        for p in &sm.positions {
            for k in 0..3 {
                bmin[k] = bmin[k].min(p[k]);
                bmax[k] = bmax[k].max(p[k]);
            }
        }
    }

    if ground_and_center_xz && bmin[0].is_finite() {
        let (cx, cz, min_y) = (
            0.5 * (bmin[0] + bmax[0]),
            0.5 * (bmin[2] + bmax[2]),
            bmin[1],
        );
        for sm in &mut submeshes {
            for p in &mut sm.positions {
                p[0] -= cx;
                p[1] -= min_y;
                p[2] -= cz;
            }
        }
        bmax[0] -= cx;
        bmin[0] -= cx;
        bmax[2] -= cz;
        bmin[2] -= cz;
        bmax[1] -= min_y;
        bmin[1] = 0.0;
    }

    let bary = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    let mut vertices = Vec::new();
    for sm in &submeshes {
        let base_lin = [sm.base_color[0], sm.base_color[1], sm.base_color[2]];
        let tint_alpha = if sm.tintable { 1.0 } else { 0.0 };
        let zf = sm.zone.as_f32();
        let has_vc = sm.vertex_colors.len() == sm.positions.len();
        let tex_rect = sm
            .texture_index
            .and_then(|ti| uv_rects.get(ti).copied())
            .filter(|_| sm.uvs.len() == sm.positions.len());

        for &[raw0, raw1, raw2] in sm.indices.as_chunks::<3>().0 {
            let (i0, i1, i2) = (raw0 as usize, raw1 as usize, raw2 as usize);
            if i0 >= sm.positions.len() || i1 >= sm.positions.len() || i2 >= sm.positions.len() {
                continue;
            }
            for (corner, &vi) in [i0, i1, i2].iter().enumerate() {
                let b = bary[corner];
                let vc = if has_vc {
                    sm.vertex_colors[vi]
                } else {
                    [1.0, 1.0, 1.0]
                };
                let col = [
                    linear_to_srgb(base_lin[0] * vc[0]),
                    linear_to_srgb(base_lin[1] * vc[1]),
                    linear_to_srgb(base_lin[2] * vc[2]),
                    tint_alpha,
                ];
                let uv = if let Some([u0, v0, u1, v1]) = tex_rect {
                    let [ru, rv] = sm.uvs[vi];
                    [
                        u0 + wrap_uv01(ru) * (u1 - u0),
                        v0 + wrap_uv01(rv) * (v1 - v0),
                    ]
                } else {
                    [-1.0, -1.0]
                };
                vertices.push(GlbVertex {
                    position: sm.positions[vi],
                    normal: sm.normals[vi],
                    color: col,
                    bary_zone: [b[0], b[1], b[2], zf],
                    uv,
                });
            }
        }
    }

    let triangle_count = vertices.len() / 3;
    LoadedGlbModel {
        name: name.to_string(),
        vertices,
        submeshes,
        albedo_atlas,
        bounds_min: bmin,
        bounds_max: bmax,
        triangle_count,
    }
}

/// Encodes a slice of [`GlbSubmesh`]es into a spec-compliant glTF 2.0 Binary (`.glb`) file
/// ready for direct import into Blender (`File -> Import -> glTF 2.0`) or `wgpu`.
#[must_use]
pub fn encode_glb_bytes(submeshes: &[GlbSubmesh]) -> Vec<u8> {
    let mut bin: Vec<u8> = Vec::new();
    let mut buffer_views = Vec::new();
    let mut accessors = Vec::new();
    let mut materials = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    let mut scene_nodes = Vec::new();

    for (idx, sm) in submeshes.iter().enumerate() {
        if sm.positions.is_empty() || sm.indices.is_empty() {
            continue;
        }
        let mat_suffix = if sm.tintable { "primary_tint" } else { "mat" };
        materials.push(json!({
            "name": format!("{}_{mat_suffix}", sm.name),
            "pbrMetallicRoughness": {
                "baseColorFactor": sm.base_color,
                "metallicFactor": sm.metallic,
                "roughnessFactor": sm.roughness
            }
        }));
        let mat_idx = materials.len() - 1;

        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let pos_off = bin.len();
        let mut pmin = [f32::INFINITY; 3];
        let mut pmax = [f32::NEG_INFINITY; 3];
        for p in &sm.positions {
            for k in 0..3 {
                pmin[k] = pmin[k].min(p[k]);
                pmax[k] = pmax[k].max(p[k]);
                bin.extend_from_slice(&p[k].to_le_bytes());
            }
        }
        let pos_bv = buffer_views.len();
        buffer_views.push(json!({ "buffer": 0, "byteOffset": pos_off, "byteLength": bin.len() - pos_off, "target": 34962 }));
        let pos_acc = accessors.len();
        accessors.push(json!({
            "bufferView": pos_bv, "componentType": 5126, "count": sm.positions.len(),
            "type": "VEC3", "min": pmin, "max": pmax
        }));

        let norm_off = bin.len();
        for n in &sm.normals {
            for &comp in n {
                bin.extend_from_slice(&comp.to_le_bytes());
            }
        }
        let norm_bv = buffer_views.len();
        buffer_views.push(json!({ "buffer": 0, "byteOffset": norm_off, "byteLength": bin.len() - norm_off, "target": 34962 }));
        let norm_acc = accessors.len();
        accessors.push(json!({ "bufferView": norm_bv, "componentType": 5126, "count": sm.normals.len(), "type": "VEC3" }));

        let mut attrs = json!({ "POSITION": pos_acc, "NORMAL": norm_acc });
        if sm.uvs.len() == sm.positions.len() && !sm.uvs.is_empty() {
            let uv_off = bin.len();
            for &[u, v] in &sm.uvs {
                bin.extend_from_slice(&u.to_le_bytes());
                bin.extend_from_slice(&v.to_le_bytes());
            }
            let uv_bv = buffer_views.len();
            buffer_views.push(json!({ "buffer": 0, "byteOffset": uv_off, "byteLength": bin.len() - uv_off, "target": 34962 }));
            let uv_acc = accessors.len();
            accessors.push(json!({ "bufferView": uv_bv, "componentType": 5126, "count": sm.uvs.len(), "type": "VEC2" }));
            attrs["TEXCOORD_0"] = json!(uv_acc);
        }

        let idx_off = bin.len();
        for &ix in &sm.indices {
            bin.extend_from_slice(&ix.to_le_bytes());
        }
        let idx_bv = buffer_views.len();
        buffer_views.push(json!({ "buffer": 0, "byteOffset": idx_off, "byteLength": bin.len() - idx_off, "target": 34963 }));
        let idx_acc = accessors.len();
        accessors.push(json!({ "bufferView": idx_bv, "componentType": 5125, "count": sm.indices.len(), "type": "SCALAR" }));

        let mesh_idx = meshes.len();
        meshes.push(json!({
            "name": format!("{}_mesh_{idx}", sm.name),
            "primitives": [{
                "attributes": attrs,
                "indices": idx_acc, "material": mat_idx, "mode": 4
            }]
        }));
        let node_idx = nodes.len();
        nodes.push(json!({ "name": sm.name, "mesh": mesh_idx }));
        scene_nodes.push(node_idx);
    }

    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }

    let root_json = json!({
        "asset": { "version": "2.0", "generator": "satya-web-glb-encoder" },
        "scene": 0,
        "scenes": [{ "name": "SatyaAsset", "nodes": scene_nodes }],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "bufferViews": buffer_views,
        "accessors": accessors,
        "buffers": [{ "byteLength": bin.len() }]
    });

    let mut json_bytes = serde_json::to_vec(&root_json).unwrap_or_default();
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }

    let total_len = (12 + 8 + json_bytes.len() + 8 + bin.len()) as u32;
    let mut out = Vec::with_capacity(total_len as usize);
    out.extend_from_slice(&GLB_MAGIC.to_le_bytes());
    out.extend_from_slice(&GLB_VERSION_2.to_le_bytes());
    out.extend_from_slice(&total_len.to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_JSON.to_le_bytes());
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(&CHUNK_BIN.to_le_bytes());
    out.extend_from_slice(&bin);
    out
}

// ============================================================================
// Procedural Low-Poly Articulated Character & Prop Builder
// ============================================================================

struct SubmeshBuilder {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl SubmeshBuilder {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            normals: Vec::new(),
            indices: Vec::new(),
        }
    }

    fn add_quad(&mut self, p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], p3: [f32; 3]) {
        let n = compute_face_normal(p0, p1, p2);
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&[p0, p1, p2, p3]);
        self.normals.extend_from_slice(&[n, n, n, n]);
        self.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    #[allow(clippy::too_many_arguments)]
    fn add_tapered_box(
        &mut self,
        cx: f32,
        cz: f32,
        y0: f32,
        y1: f32,
        hx_bot: f32,
        hz_bot: f32,
        hx_top: f32,
        hz_top: f32,
    ) {
        let b0 = [cx - hx_bot, y0, cz - hz_bot];
        let b1 = [cx + hx_bot, y0, cz - hz_bot];
        let b2 = [cx + hx_bot, y0, cz + hz_bot];
        let b3 = [cx - hx_bot, y0, cz + hz_bot];
        let t0 = [cx - hx_top, y1, cz - hz_top];
        let t1 = [cx + hx_top, y1, cz - hz_top];
        let t2 = [cx + hx_top, y1, cz + hz_top];
        let t3 = [cx - hx_top, y1, cz + hz_top];

        self.add_quad(b3, b2, t2, t3);
        self.add_quad(b1, b0, t0, t1);
        self.add_quad(b2, b1, t1, t2);
        self.add_quad(b0, b3, t3, t0);
        self.add_quad(t3, t2, t1, t0);
        self.add_quad(b0, b1, b2, b3);
    }

    #[allow(clippy::too_many_arguments)]
    fn add_frustum(
        &mut self,
        cx: f32,
        cz: f32,
        y0: f32,
        y1: f32,
        r_bot: f32,
        r_top: f32,
        sides: usize,
    ) {
        let n = sides.max(4);
        let step = std::f32::consts::TAU / (n as f32);
        for i in 0..n {
            let (s0, c0) = ((i as f32) * step).sin_cos();
            let (s1, c1) = (((i + 1) as f32) * step).sin_cos();
            let b0 = [cx + r_bot * c0, y0, cz + r_bot * s0];
            let b1 = [cx + r_bot * c1, y0, cz + r_bot * s1];
            let t1 = [cx + r_top * c1, y1, cz + r_top * s1];
            let t0 = [cx + r_top * c0, y1, cz + r_top * s0];
            self.add_quad(b1, b0, t0, t1);

            let base_idx = self.positions.len() as u32;
            self.positions.extend_from_slice(&[[cx, y1, cz], t0, t1]);
            self.normals.extend_from_slice(&[[0.0, 1.0, 0.0]; 3]);
            self.indices
                .extend_from_slice(&[base_idx, base_idx + 1, base_idx + 2]);

            let b_idx = self.positions.len() as u32;
            self.positions.extend_from_slice(&[[cx, y0, cz], b1, b0]);
            self.normals.extend_from_slice(&[[0.0, -1.0, 0.0]; 3]);
            self.indices
                .extend_from_slice(&[b_idx, b_idx + 1, b_idx + 2]);
        }
    }

    fn finish(
        self,
        name: &str,
        base_color: [f32; 4],
        metallic: f32,
        roughness: f32,
        tintable: bool,
        zone: HitZoneTag,
    ) -> GlbSubmesh {
        GlbSubmesh {
            name: name.to_string(),
            base_color,
            metallic,
            roughness,
            tintable,
            zone,
            positions: self.positions,
            normals: self.normals,
            uvs: Vec::new(),
            vertex_colors: Vec::new(),
            texture_index: None,
            indices: self.indices,
        }
    }
}

struct HumanoidProfile {
    scale_h: f32,
    shoulder_w: f32,
    chest_depth: f32,
    primary_rgba: [f32; 4],
    visor_rgba: [f32; 4],
    rifle_len: f32,
    has_thruster_fins: bool,
    heavy_dome: bool,
}

fn build_humanoid_submeshes(p: &HumanoidProfile) -> Vec<GlbSubmesh> {
    let s = p.scale_h;
    let mut base = SubmeshBuilder::new();
    base.add_frustum(0.0, 0.0, 0.0, 0.055, 0.38, 0.35, 14);
    base.add_tapered_box(0.0, -0.34, 0.02, 0.075, 0.05, 0.05, 0.03, 0.03);

    let mut legs = SubmeshBuilder::new();
    for sign in [-1.0_f32, 1.0] {
        let (lx, z_off) = (sign * 0.115 * s, sign * 0.03);
        legs.add_tapered_box(lx, z_off - 0.03, 0.055, 0.18 * s, 0.075, 0.11, 0.065, 0.085);
        legs.add_tapered_box(lx, z_off, 0.18 * s, 0.50 * s, 0.065, 0.075, 0.075, 0.080);
        legs.add_tapered_box(
            lx,
            z_off * 0.5,
            0.50 * s,
            0.86 * s,
            0.078,
            0.082,
            0.088,
            0.092,
        );
    }

    let mut torso = SubmeshBuilder::new();
    torso.add_tapered_box(0.0, 0.0, 0.82 * s, 0.98 * s, 0.17 * s, 0.11, 0.15 * s, 0.10);
    torso.add_tapered_box(
        0.0,
        0.0,
        0.98 * s,
        1.43 * s,
        0.155 * s,
        p.chest_depth * 0.82,
        p.shoulder_w,
        p.chest_depth,
    );
    for sign in [-1.0_f32, 1.0] {
        torso.add_tapered_box(
            sign * (p.shoulder_w + 0.055),
            0.0,
            1.30 * s,
            1.46 * s,
            0.075,
            0.105,
            0.055,
            0.085,
        );
    }

    let mut pack = SubmeshBuilder::new();
    pack.add_tapered_box(
        0.0,
        p.chest_depth + 0.075,
        1.02 * s,
        1.42 * s,
        0.13 * s,
        0.075,
        0.11 * s,
        0.065,
    );
    if p.has_thruster_fins {
        for sign in [-1.0_f32, 1.0] {
            pack.add_tapered_box(
                sign * 0.15 * s,
                p.chest_depth + 0.11,
                1.18 * s,
                1.56 * s,
                0.03,
                0.05,
                0.018,
                0.035,
            );
        }
    }

    let mut arms = SubmeshBuilder::new();
    for sign in [-1.0_f32, 1.0] {
        let ax = sign * (p.shoulder_w + 0.045);
        arms.add_tapered_box(ax, -0.02, 1.10 * s, 1.34 * s, 0.055, 0.058, 0.062, 0.062);
        arms.add_tapered_box(
            ax * 0.72,
            -0.16,
            1.04 * s,
            1.16 * s,
            0.048,
            0.13,
            0.048,
            0.11,
        );
    }
    if p.rifle_len > 0.05 {
        arms.add_tapered_box(
            0.05,
            -0.28 - p.rifle_len * 0.35,
            1.06 * s,
            1.16 * s,
            0.032,
            p.rifle_len * 0.5,
            0.026,
            p.rifle_len * 0.48,
        );
    }

    let mut head = SubmeshBuilder::new();
    if p.heavy_dome {
        head.add_frustum(0.0, -0.01, 1.43 * s, 1.66 * s, 0.145 * s, 0.105 * s, 10);
    } else {
        head.add_frustum(0.0, 0.0, 1.43 * s, 1.50 * s, 0.075, 0.08, 8);
        head.add_tapered_box(0.0, -0.01, 1.49 * s, 1.72 * s, 0.105, 0.118, 0.090, 0.102);
    }
    let mut visor = SubmeshBuilder::new();
    visor.add_tapered_box(0.0, -0.115, 1.55 * s, 1.64 * s, 0.088, 0.022, 0.082, 0.020);

    vec![
        base.finish(
            "miniature_base",
            [0.14, 0.19, 0.27, 1.0],
            0.4,
            0.6,
            false,
            HitZoneTag::BaseOrProp,
        ),
        legs.finish(
            "legs_boots",
            [0.22, 0.28, 0.38, 1.0],
            0.35,
            0.55,
            false,
            HitZoneTag::LegsBoots,
        ),
        torso.finish(
            "torso_carapace",
            p.primary_rgba,
            0.45,
            0.38,
            true,
            HitZoneTag::TorsoCarapace,
        ),
        pack.finish(
            "pack_thrusters",
            [0.26, 0.33, 0.44, 1.0],
            0.5,
            0.45,
            false,
            HitZoneTag::PackThrusters,
        ),
        arms.finish(
            "arms_weapon",
            [0.18, 0.22, 0.30, 1.0],
            0.65,
            0.35,
            false,
            HitZoneTag::ArmsWeapon,
        ),
        head.finish(
            "head_helm",
            p.primary_rgba,
            0.45,
            0.35,
            true,
            HitZoneTag::HeadOptics,
        ),
        visor.finish(
            "visor_optics",
            p.visor_rgba,
            0.85,
            0.15,
            false,
            HitZoneTag::HeadOptics,
        ),
    ]
}

fn build_sector4_airlock_submeshes() -> Vec<GlbSubmesh> {
    let mut frame = SubmeshBuilder::new();
    frame.add_tapered_box(0.0, 0.0, 0.0, 0.10, 1.35, 0.95, 1.32, 0.92);
    frame.add_tapered_box(-1.05, 0.0, 0.10, 2.05, 0.16, 0.26, 0.14, 0.24);
    frame.add_tapered_box(1.05, 0.0, 0.10, 2.05, 0.16, 0.26, 0.14, 0.24);
    frame.add_tapered_box(0.0, 0.0, 1.85, 2.18, 1.22, 0.28, 1.14, 0.24);

    let mut doors = SubmeshBuilder::new();
    doors.add_tapered_box(-0.44, 0.0, 0.10, 1.85, 0.42, 0.09, 0.42, 0.09);
    doors.add_tapered_box(0.44, 0.0, 0.10, 1.85, 0.42, 0.09, 0.42, 0.09);

    let mut crane = SubmeshBuilder::new();
    crane.add_tapered_box(0.0, -0.35, 1.92, 2.04, 0.95, 0.10, 0.95, 0.10);
    let mut valve = SubmeshBuilder::new();
    valve.add_frustum(-1.05, -0.28, 0.88, 1.08, 0.16, 0.16, 10);

    vec![
        frame.finish(
            "base_bulkhead",
            [0.22, 0.29, 0.39, 1.0],
            0.55,
            0.50,
            false,
            HitZoneTag::BaseOrProp,
        ),
        doors.finish(
            "torso_airlock_doors",
            [0.08, 0.58, 0.84, 1.0],
            0.50,
            0.38,
            true,
            HitZoneTag::TorsoCarapace,
        ),
        crane.finish(
            "pack_mag_crane",
            [0.92, 0.62, 0.12, 1.0],
            0.60,
            0.42,
            false,
            HitZoneTag::PackThrusters,
        ),
        valve.finish(
            "arms_valve_wheel",
            [0.94, 0.27, 0.36, 1.0],
            0.50,
            0.40,
            false,
            HitZoneTag::ArmsWeapon,
        ),
    ]
}

fn build_quad_drone_submeshes() -> Vec<GlbSubmesh> {
    let mut body = SubmeshBuilder::new();
    body.add_tapered_box(0.0, 0.0, 0.28, 0.48, 0.16, 0.28, 0.14, 0.25);
    let mut legs = SubmeshBuilder::new();
    for (lx, lz) in [(-0.17, -0.21), (0.17, -0.21), (-0.17, 0.21), (0.17, 0.21)] {
        legs.add_tapered_box(lx, lz, 0.0, 0.34, 0.038, 0.045, 0.045, 0.052);
    }
    let mut sensor = SubmeshBuilder::new();
    sensor.add_tapered_box(0.0, -0.29, 0.30, 0.44, 0.09, 0.06, 0.08, 0.05);
    let mut base = SubmeshBuilder::new();
    base.add_frustum(0.0, 0.0, 0.0, 0.03, 0.38, 0.38, 12);
    vec![
        base.finish("miniature_base", [0.14, 0.19, 0.27, 1.0], 0.4, 0.6, false, HitZoneTag::BaseOrProp),
        legs.finish("legs_boots", [0.22, 0.26, 0.32, 1.0], 0.45, 0.45, false, HitZoneTag::LegsBoots),
        body.finish("torso_carapace", [0.36, 0.42, 0.36, 1.0], 0.45, 0.42, true, HitZoneTag::TorsoCarapace),
        sensor.finish("visor_optics", [0.15, 0.18, 0.22, 1.0], 0.70, 0.25, false, HitZoneTag::HeadOptics),
    ]
}

fn build_aero_drone_submeshes() -> Vec<GlbSubmesh> {
    let mut base = SubmeshBuilder::new();
    base.add_frustum(0.0, 0.0, 0.0, 0.025, 0.34, 0.34, 12);
    let mut hull = SubmeshBuilder::new();
    hull.add_tapered_box(0.0, 0.0, 1.18, 1.32, 0.09, 0.20, 0.07, 0.16);
    let mut arms = SubmeshBuilder::new();
    for (rx, rz) in [(-0.24, -0.24), (0.24, -0.24), (-0.24, 0.24), (0.24, 0.24)] {
        arms.add_tapered_box(rx * 0.55, rz * 0.55, 1.22, 1.26, 0.028, 0.14, 0.028, 0.14);
        arms.add_frustum(rx, rz, 1.26, 1.30, 0.13, 0.13, 10);
    }
    let mut sensor = SubmeshBuilder::new();
    sensor.add_tapered_box(0.0, -0.18, 1.12, 1.22, 0.05, 0.05, 0.04, 0.04);
    vec![
        base.finish("miniature_base", [0.14, 0.19, 0.27, 1.0], 0.4, 0.6, false, HitZoneTag::BaseOrProp),
        hull.finish("torso_carapace", [0.22, 0.26, 0.32, 1.0], 0.55, 0.35, true, HitZoneTag::TorsoCarapace),
        arms.finish("arms_weapon", [0.32, 0.38, 0.46, 1.0], 0.60, 0.35, false, HitZoneTag::ArmsWeapon),
        sensor.finish("visor_optics", [0.12, 0.15, 0.18, 1.0], 0.80, 0.20, false, HitZoneTag::HeadOptics),
    ]
}

/// Metadata summary `(display_title, scale_desc, hit_zones_desc)` for built-in 3D presets.
#[must_use]
pub fn preset_model_metadata(preset: &str) -> (&'static str, &'static str, &'static str) {
    match preset.trim() {
        "civilian_unarmed" => ("Civilian (Empty-Handed)", "1.74m · Everyday 2035", "Street Coat (DR 0)"),
        "civilian_handgun" => ("Civilian (Handgun)", "1.75m · Concealed Carry", "Hoodie (DR 0) · Compact 9mm"),
        "civilian_rifle" => ("Civilian (Rifle)", "1.78m · Frontier Ranger", "Utility Vest (DR 1) · Carbine"),
        "civilian_knife" => ("Civilian (Knife)", "1.78m · Close-Quarters", "Field Coat (DR 1) · Kerambit"),
        "security_handgun" => ("Police / Security (Handgun)", "1.80m · Duty Vest", "Kevlar Vest (DR 3) · Service Pistol"),
        "soldier_rifle" => ("Security / Soldier (Rifle)", "1.82m · Optical Camo", "Plate Carrier (DR 5) · Carbine"),
        "sniper_rifle" => ("Military / Security (Sniper)", "1.80m · Marksman", "Camo Mantle (DR 3) · Precision Rifle"),
        "drone_quad" => ("Quadruped Drone (Echo-9)", "0.48m · Agile UGV", "Armored Chassis (DR 4) · Optics Bar"),
        "drone_aero" => ("Aero Drone (Quadrotor UAV)", "1.32m Hover · Recon UAV", "Carbon Fuselage (DR 2) · 4 Rotors"),
        "android_domestic" => ("Android — Domestic", "1.70m · Polymer Bipedal", "Polymer Shell (DR 2) · Non-Lethal"),
        "android_industrial" => ("Android — Industrial", "1.85m · Utility Bipedal", "Alloy Frame (DR 4) · Work Grippers"),
        "android_security" => ("Android — Military / Security", "1.86m · Tactical Bipedal", "Ballistic Plating (DR 6) · Carbine"),
        "recon_mk4" => (
            "Recon Suit Mk-IV (Marcus)",
            "1.85m · Light EVA",
            "Optics Visor (DR 3) · Chest Carapace (DR 4) · Thruster Pack (DR 2)",
        ),
        "secops_sentry" => (
            "SecOps Heavy Sentry",
            "2.10m · Armored Bipedal",
            "Sensor Dome (DR 5) · Reactive Plating (DR 7) · Servo Joints (DR 3)",
        ),
        "operative_light" => (
            "Station Operative (Light)",
            "1.78m · Tactical Rig",
            "Comms Headset (DR 1) · Kevlar Vest (DR 3) · Utility Boots (DR 2)",
        ),
        "sector4_airlock" => (
            "Sector 4 Orbital Airlock",
            "2.4m Bulkhead · Zero-G Threshold",
            "Inner Bulkhead · Decompression Valve · Mag-Rail Crane",
        ),
        _ => (
            "Valkyrie Exo-Rig (Sara)",
            "1.92m · Med-Assault Exo",
            "Ballistic Helm (DR 4) · Trauma Auto-Injector · Mag-Boots (DR 4)",
        ),
    }
}

/// Generates a spec-compliant `.glb` byte buffer for any built-in Satya character or prop preset.
#[must_use]
pub fn build_preset_glb_bytes(preset: &str) -> Vec<u8> {
    let submeshes = match preset.trim() {
        "civilian_unarmed" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 0.97,
            shoulder_w: 0.165,
            chest_depth: 0.105,
            primary_rgba: [0.36, 0.42, 0.34, 1.0],
            visor_rgba: [0.22, 0.18, 0.15, 1.0],
            rifle_len: 0.0,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "civilian_handgun" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 0.98,
            shoulder_w: 0.17,
            chest_depth: 0.11,
            primary_rgba: [0.31, 0.43, 0.54, 1.0],
            visor_rgba: [0.22, 0.18, 0.15, 1.0],
            rifle_len: 0.24,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "civilian_rifle" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.0,
            shoulder_w: 0.175,
            chest_depth: 0.115,
            primary_rgba: [0.54, 0.45, 0.33, 1.0],
            visor_rgba: [0.24, 0.20, 0.16, 1.0],
            rifle_len: 0.56,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "civilian_knife" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.0,
            shoulder_w: 0.175,
            chest_depth: 0.115,
            primary_rgba: [0.61, 0.48, 0.33, 1.0],
            visor_rgba: [0.25, 0.26, 0.28, 1.0],
            rifle_len: 0.14,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "security_handgun" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.01,
            shoulder_w: 0.185,
            chest_depth: 0.12,
            primary_rgba: [0.15, 0.22, 0.33, 1.0],
            visor_rgba: [0.16, 0.20, 0.26, 1.0],
            rifle_len: 0.24,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "soldier_rifle" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.03,
            shoulder_w: 0.20,
            chest_depth: 0.13,
            primary_rgba: [0.36, 0.42, 0.37, 1.0],
            visor_rgba: [0.18, 0.22, 0.20, 1.0],
            rifle_len: 0.56,
            has_thruster_fins: false,
            heavy_dome: true,
        }),
        "sniper_rifle" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.01,
            shoulder_w: 0.185,
            chest_depth: 0.12,
            primary_rgba: [0.30, 0.37, 0.32, 1.0],
            visor_rgba: [0.16, 0.20, 0.18, 1.0],
            rifle_len: 0.82,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "drone_quad" => build_quad_drone_submeshes(),
        "drone_aero" => build_aero_drone_submeshes(),
        "android_domestic" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 0.96,
            shoulder_w: 0.16,
            chest_depth: 0.095,
            primary_rgba: [0.88, 0.90, 0.92, 1.0],
            visor_rgba: [0.14, 0.16, 0.19, 1.0],
            rifle_len: 0.0,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "android_industrial" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.05,
            shoulder_w: 0.21,
            chest_depth: 0.13,
            primary_rgba: [0.58, 0.63, 0.68, 1.0],
            visor_rgba: [0.85, 0.58, 0.12, 1.0],
            rifle_len: 0.0,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "android_security" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.06,
            shoulder_w: 0.215,
            chest_depth: 0.135,
            primary_rgba: [0.28, 0.33, 0.39, 1.0],
            visor_rgba: [0.16, 0.19, 0.22, 1.0],
            rifle_len: 0.54,
            has_thruster_fins: false,
            heavy_dome: true,
        }),
        "recon_mk4" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.05,
            shoulder_w: 0.19,
            chest_depth: 0.12,
            primary_rgba: [0.13, 0.82, 0.93, 1.0],
            visor_rgba: [0.98, 0.75, 0.15, 1.0],
            rifle_len: 0.62,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "secops_sentry" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.18,
            shoulder_w: 0.27,
            chest_depth: 0.17,
            primary_rgba: [0.92, 0.28, 0.34, 1.0],
            visor_rgba: [1.0, 0.18, 0.22, 1.0],
            rifle_len: 0.54,
            has_thruster_fins: false,
            heavy_dome: true,
        }),
        "operative_light" => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.0,
            shoulder_w: 0.175,
            chest_depth: 0.11,
            primary_rgba: [0.38, 0.65, 0.98, 1.0],
            visor_rgba: [0.20, 0.90, 0.65, 1.0],
            rifle_len: 0.32,
            has_thruster_fins: false,
            heavy_dome: false,
        }),
        "sector4_airlock" => build_sector4_airlock_submeshes(),
        _ => build_humanoid_submeshes(&HumanoidProfile {
            scale_h: 1.08,
            shoulder_w: 0.22,
            chest_depth: 0.14,
            primary_rgba: [0.06, 0.65, 0.91, 1.0],
            visor_rgba: [0.20, 0.95, 0.68, 1.0],
            rifle_len: 0.48,
            has_thruster_fins: true,
            heavy_dome: false,
        }),
    };
    encode_glb_bytes(&submeshes)
}

/// Converts a glTF 2.0 linear RGB channel (`0..=1`) to sRGB for non-sRGB swapchain rendering.
#[must_use]
fn linear_to_srgb(c: f32) -> f32 {
    let clamped = c.clamp(0.0, 1.0);
    if clamped <= 0.003_130_8 {
        clamped * 12.92
    } else {
        1.055 * clamped.powf(1.0 / 2.4) - 0.055
    }
}

/// Builds and parses a built-in `.glb` preset through the full binary `.glb` round-trip pipeline.
#[must_use]
pub fn load_preset_glb_model(preset: &str) -> LoadedGlbModel {
    let (title, _, _) = preset_model_metadata(preset);
    let bytes = build_preset_glb_bytes(preset);
    parse_glb_bytes(&bytes, title).expect("Built-in procedural .glb preset must parse cleanly")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_procedural_glb_presets_encode_and_parse_roundtrip() {
        assert!((linear_to_srgb(0.214) - 0.50).abs() < 0.01);
        for preset in [
            "civilian_unarmed",
            "civilian_handgun",
            "civilian_rifle",
            "civilian_knife",
            "security_handgun",
            "soldier_rifle",
            "sniper_rifle",
            "drone_quad",
            "drone_aero",
            "android_domestic",
            "android_industrial",
            "android_security",
            "valkyrie_rig",
            "recon_mk4",
            "secops_sentry",
            "operative_light",
            "sector4_airlock",
        ] {
            let glb = build_preset_glb_bytes(preset);
            assert_eq!(&glb[0..4], b"glTF");
            let parsed = parse_glb_bytes(&glb, preset).expect("GLB should parse");
            assert!(
                parsed.triangle_count >= 60,
                "{preset} had too few triangles"
            );
            let min_h = if preset == "drone_quad" { 0.4 } else { 1.2 };
            assert!(
                parsed.height_m() > min_h,
                "{preset} height was {}",
                parsed.height_m()
            );
            assert!((parsed.bounds_min[1]).abs() < 1e-4);
        }
    }

    #[test]
    fn test_parse_glb_with_embedded_png_uvs_and_vertex_colors() {
        // Minimal 1x1 uncompressed RGBA PNG (red pixel: 255, 0, 0, 255)
        let mut raw_scan = vec![0_u8, 255, 0, 0, 255];
        let mut s1 = 1_u32;
        let mut s2 = 0_u32;
        for &b in &raw_scan {
            s1 = (s1 + u32::from(b)) % 65521;
            s2 = (s2 + s1) % 65521;
        }
        let adler = (s2 << 16) | s1;
        let mut zlib = vec![0x78, 0x01, 0x01];
        let len = raw_scan.len() as u16;
        zlib.extend_from_slice(&len.to_le_bytes());
        zlib.extend_from_slice(&(!len).to_le_bytes());
        zlib.append(&mut raw_scan);
        zlib.extend_from_slice(&adler.to_be_bytes());

        fn crc32(data: &[u8]) -> u32 {
            let mut crc = 0xFFFF_FFFF_u32;
            for &b in data {
                crc ^= u32::from(b);
                for _ in 0..8 {
                    let mask = (crc & 1).wrapping_neg();
                    crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
                }
            }
            !crc
        }
        fn push_chunk(out: &mut Vec<u8>, tag: &[u8; 4], payload: &[u8]) {
            out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
            out.extend_from_slice(tag);
            out.extend_from_slice(payload);
            let mut c = Vec::from(*tag);
            c.extend_from_slice(payload);
            out.extend_from_slice(&crc32(&c).to_be_bytes());
        }
        let mut png = vec![137, 80, 78, 71, 13, 10, 26, 10];
        let mut ihdr = Vec::new();
        ihdr.extend_from_slice(&1_u32.to_be_bytes());
        ihdr.extend_from_slice(&1_u32.to_be_bytes());
        ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
        push_chunk(&mut png, b"IHDR", &ihdr);
        push_chunk(&mut png, b"IDAT", &zlib);
        push_chunk(&mut png, b"IEND", &[]);

        let mut bin = Vec::new();
        let positions: [[f32; 3]; 3] = [[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.5, 1.5, 0.0]];
        let pos_off = bin.len();
        for p in &positions {
            for &c in p {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let pos_len = bin.len() - pos_off;

        let uvs: [[f32; 2]; 3] = [[0.1, 0.2], [0.9, 0.2], [0.5, 0.8]];
        let uv_off = bin.len();
        for uv in &uvs {
            for &c in uv {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let uv_len = bin.len() - uv_off;

        let cols: [[u8; 4]; 3] = [[255, 255, 255, 255], [128, 255, 255, 255], [255, 128, 255, 255]];
        let col_off = bin.len();
        for c in &cols {
            bin.extend_from_slice(c);
        }
        let col_len = bin.len() - col_off;

        let img_off = bin.len();
        bin.extend_from_slice(&png);
        let img_len = bin.len() - img_off;
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }

        let gltf = json!({
            "asset": { "version": "2.0" },
            "scene": 0,
            "scenes": [{ "nodes": [0] }],
            "nodes": [{ "name": "head_uv_test", "mesh": 0 }],
            "meshes": [{
                "name": "tri_mesh",
                "primitives": [{
                    "attributes": { "POSITION": 0, "TEXCOORD_0": 1, "COLOR_0": 2 },
                    "material": 0
                }]
            }],
            "materials": [{
                "name": "textured_mat",
                "pbrMetallicRoughness": {
                    "baseColorFactor": [1.0, 1.0, 1.0, 1.0],
                    "baseColorTexture": { "index": 0 }
                }
            }],
            "textures": [{ "source": 0 }],
            "images": [{ "bufferView": 3, "mimeType": "image/png" }],
            "bufferViews": [
                { "buffer": 0, "byteOffset": pos_off, "byteLength": pos_len },
                { "buffer": 0, "byteOffset": uv_off, "byteLength": uv_len },
                { "buffer": 0, "byteOffset": col_off, "byteLength": col_len },
                { "buffer": 0, "byteOffset": img_off, "byteLength": img_len }
            ],
            "accessors": [
                { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3" },
                { "bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2" },
                { "bufferView": 2, "componentType": 5121, "count": 3, "type": "VEC4" }
            ],
            "buffers": [{ "byteLength": bin.len() }]
        });
        let mut json_bytes = serde_json::to_vec(&gltf).unwrap();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let total_len = (12 + 8 + json_bytes.len() + 8 + bin.len()) as u32;
        let mut glb = Vec::with_capacity(total_len as usize);
        glb.extend_from_slice(&GLB_MAGIC.to_le_bytes());
        glb.extend_from_slice(&GLB_VERSION_2.to_le_bytes());
        glb.extend_from_slice(&total_len.to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(&CHUNK_JSON.to_le_bytes());
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(&CHUNK_BIN.to_le_bytes());
        glb.extend_from_slice(&bin);

        let parsed = parse_glb_bytes(&glb, "uv_test").expect("UV GLB should parse");
        let atlas = parsed.albedo_atlas.as_ref().expect("Atlas must be decoded");
        assert_eq!((atlas.width, atlas.height), (1, 1));
        assert_eq!(atlas.rgba8, vec![255, 0, 0, 255]);
        assert!((parsed.vertices[0].uv[0] - 0.1).abs() < 1e-5);
        assert!((parsed.vertices[1].uv[0] - 0.9).abs() < 1e-5);
        assert!(parsed.vertices[1].color[0] < 0.8);
    }

    #[test]
    fn test_parse_alex_wen_set01_v2_glb_asset() {
        let glb_bytes =
            include_bytes!("../../../../assets/models/characters/alex_wen/alex_wen_set01_v2.glb");
        let parsed = parse_glb_bytes(glb_bytes, "alex_wen_set01_v2.glb")
            .expect("alex_wen_set01_v2.glb must parse cleanly");
        let atlas = parsed
            .albedo_atlas
            .as_ref()
            .expect("alex_wen_set01_v2.glb must contain embedded PNG atlas");
        assert_eq!((atlas.width, atlas.height), (1024, 1024));
        assert!(parsed.triangle_count > 10_000);
        assert!((parsed.height_m() - 1.78).abs() < 0.02);
        assert!(parsed.bounds_min[1].abs() < 1e-4);
        assert!(
            parsed
                .vertices
                .iter()
                .all(|v| (0.0..=1.0).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1]))
        );
    }

    #[test]
    fn test_parse_alex_wen_set01_v3_glb_asset() {
        let glb_bytes =
            include_bytes!("../../../../assets/models/characters/alex_wen/alex_wen_set01_v3.glb");
        let parsed = parse_glb_bytes(glb_bytes, "alex_wen_set01_v3.glb")
            .expect("alex_wen_set01_v3.glb must parse cleanly");
        let atlas = parsed
            .albedo_atlas
            .as_ref()
            .expect("alex_wen_set01_v3.glb must contain embedded PNG atlas");
        assert_eq!((atlas.width, atlas.height), (1024, 1024));
        assert!(parsed.triangle_count > 10_000);
        assert!((parsed.height_m() - 1.78).abs() < 0.02);
        assert!(parsed.bounds_min[1].abs() < 1e-4);
        assert!(
            parsed
                .vertices
                .iter()
                .all(|v| (0.0..=1.0).contains(&v.uv[0]) && (0.0..=1.0).contains(&v.uv[1]))
        );
    }
}
