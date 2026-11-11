//! Synchronized 3D WebGPU (`wgpu`) Polyhedral Dice Roller, Procedural Platonic/Trapezohedral Mesh
//! Generator, SDF Numeral Texture Atlas, and Deterministic Physics Trajectory Engine.
//!
//! Companion module [`crate::wasm_client::dice_3d_wgpu`] contains the two-pass translucent WGSL
//! shader, `wgpu` pipeline, camera matrix builder, and DOM slider/canvas event synchronization,
//! all re-exported here for flat access.

pub use crate::wasm_client::dice_3d_wgpu::*;

use crate::wasm_client::wgpu_shared::{cross3, dot3, mul_mat4_cols, normalize3};
use bytemuck::{Pod, Zeroable};
use satya_schema::WgpuDiceSpec;

/// Width in pixels of each face cell in the procedural numeral texture atlas.
pub const ATLAS_CELL_SIZE: u32 = 128;
/// Number of columns in the procedural numeral texture atlas (`5 * 4 = 20` cells).
pub const ATLAS_COLS: u32 = 5;
/// Number of rows in the procedural numeral texture atlas (`5 * 4 = 20` cells).
pub const ATLAS_ROWS: u32 = 4;
/// Total width in pixels of the procedural numeral texture atlas (`640`).
pub const ATLAS_WIDTH: u32 = ATLAS_CELL_SIZE * ATLAS_COLS;
/// Total height in pixels of the procedural numeral texture atlas (`512`).
pub const ATLAS_HEIGHT: u32 = ATLAS_CELL_SIZE * ATLAS_ROWS;

/// Supported polyhedral RPG die shapes (`d4`, `d6`, `d8`, `d10`, `d12`, `d20`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DieKind {
    D4,
    D6,
    D8,
    D10,
    D12,
    D20,
}

impl DieKind {
    pub const fn sides(self) -> i32 {
        match self {
            Self::D4 => 4,
            Self::D6 => 6,
            Self::D8 => 8,
            Self::D10 => 10,
            Self::D12 => 12,
            Self::D20 => 20,
        }
    }

    pub const fn from_sides(sides: i32) -> Self {
        match sides {
            4 => Self::D4,
            6 => Self::D6,
            8 => Self::D8,
            10 => Self::D10,
            12 => Self::D12,
            _ => Self::D20,
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::D4 => "d4",
            Self::D6 => "d6",
            Self::D8 => "d8",
            Self::D10 => "d10",
            Self::D12 => "d12",
            Self::D20 => "d20",
        }
    }
}

/// GPU vertex structure for a polyhedral die facet triangle (`44` bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct DieVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// `x`: distance to polygon edge in `[0, 1]`, `y`: face index, `z`: reserved.
    pub edge_factors: [f32; 3],
}

/// CPU-side procedural mesh for a polyhedral die, including face normals for outcome alignment.
#[derive(Debug, Clone)]
pub struct PolyhedralMesh {
    pub kind: DieKind,
    pub vertices: Vec<DieVertex>,
    pub face_normals: Vec<[f32; 3]>,
}

/// User-customizable color, ink, and translucency settings for the 3D WebGPU dice renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct DiceMaterialConfig {
    pub base_color_hex: String,
    pub base_rgb: [f32; 3],
    pub ink_color_hex: String,
    pub ink_rgb: [f32; 3],
    pub opacity: f32,
}

impl Default for DiceMaterialConfig {
    fn default() -> Self {
        Self::from_hex_and_opacity("#0ea5e9", "#f8fafc", 0.38)
    }
}

impl DiceMaterialConfig {
    /// Constructs a [`DiceMaterialConfig`] from `#RRGGBB` hex strings and an opacity in `[0.08, 1.0]`.
    pub fn from_hex_and_opacity(base_hex: &str, ink_hex: &str, opacity: f32) -> Self {
        let (norm_base, base_rgb) = parse_hex_color(base_hex, [0.055, 0.647, 0.914]);
        let (norm_ink, ink_rgb) = parse_hex_color(ink_hex, [0.973, 0.980, 0.988]);
        Self {
            base_color_hex: norm_base,
            base_rgb,
            ink_color_hex: norm_ink,
            ink_rgb,
            opacity: opacity.clamp(0.08, 1.0),
        }
    }

    /// Returns a named preset material configuration (`"translucent_sapphire"`, `"amber_resin"`,
    /// `"emerald_glass"`, `"crimson_ruby"`, `"amethyst_void"`, or `"obsidian_opaque"`).
    pub fn from_preset(preset: &str) -> Self {
        match preset.trim() {
            "amber_resin" => Self::from_hex_and_opacity("#f59e0b", "#fef3c7", 0.36),
            "emerald_glass" => Self::from_hex_and_opacity("#10b981", "#ecfdf5", 0.32),
            "crimson_ruby" => Self::from_hex_and_opacity("#e11d48", "#ffe4e6", 0.35),
            "amethyst_void" => Self::from_hex_and_opacity("#9333ea", "#f3e8ff", 0.34),
            "obsidian_opaque" => Self::from_hex_and_opacity("#1e293b", "#38bdf8", 1.00),
            _ => Self::from_hex_and_opacity("#0ea5e9", "#f8fafc", 0.38),
        }
    }
}

/// Parses a `#RRGGBB` or `RRGGBB` hex color string into a normalized `"#rrggbb"` string and `[f32; 3]`.
pub fn parse_hex_color(raw: &str, fallback: [f32; 3]) -> (String, [f32; 3]) {
    let trimmed = raw.trim().trim_start_matches('#');
    if trimmed.len() == 6
        && let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&trimmed[0..2], 16),
            u8::from_str_radix(&trimmed[2..4], 16),
            u8::from_str_radix(&trimmed[4..6], 16),
        )
    {
        return (
            format!("#{r:02x}{g:02x}{b:02x}"),
            [
                f32::from(r) / 255.0,
                f32::from(g) / 255.0,
                f32::from(b) / 255.0,
            ],
        );
    }
    let fr = (fallback[0].clamp(0.0, 1.0) * 255.0).round() as u8;
    let fg = (fallback[1].clamp(0.0, 1.0) * 255.0).round() as u8;
    let fb = (fallback[2].clamp(0.0, 1.0) * 255.0).round() as u8;
    (format!("#{fr:02x}{fg:02x}{fb:02x}"), fallback)
}

/// Parses a dice notation string (e.g. `"2d20"`, `"4d20"`, `"3d6+2"`, `"1d8"`) into `(DieKind, count)`.
pub fn parse_dice_notation_kind(notation: &str, outcome_count: usize) -> (DieKind, usize) {
    let lower = notation.trim().to_ascii_lowercase();
    let token = lower.split_whitespace().next().unwrap_or("2d20");
    let core = token.split('|').next().unwrap_or(token);
    let core = core.split(['+', '-']).next().unwrap_or(core);
    if let Some((cnt_str, rest)) = core.split_once('d') {
        let count = cnt_str.trim().parse::<usize>().unwrap_or(1).clamp(1, 24);
        let sides_digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let sides = sides_digits.parse::<i32>().unwrap_or(20);
        let final_count = if outcome_count > 0 {
            outcome_count.clamp(1, 24)
        } else {
            count
        };
        (DieKind::from_sides(sides), final_count)
    } else {
        (DieKind::D20, outcome_count.clamp(1, 24))
    }
}

#[inline]
pub(crate) fn vec3_add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
pub(crate) fn vec3_sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
pub(crate) fn vec3_scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[inline]
pub(crate) fn vec3_dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    dot3(a, b)
}

#[inline]
pub(crate) fn vec3_cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    cross3(a, b)
}

#[inline]
pub(crate) fn vec3_len(v: [f32; 3]) -> f32 {
    dot3(v, v).sqrt()
}

#[inline]
pub(crate) fn vec3_normalize(v: [f32; 3]) -> [f32; 3] {
    normalize3(v)
}

/// Returns the UV bounding rectangle `(u0, v0, u1, v1)` in the `5x4` numeral atlas for `face_number` (`1..=20`).
pub fn atlas_uv_bounds(face_number: usize) -> (f32, f32, f32, f32) {
    let idx = face_number.saturating_sub(1).min(19) as u32;
    let col = idx % ATLAS_COLS;
    let row = idx / ATLAS_COLS;
    let u0 = (col as f32) / (ATLAS_COLS as f32);
    let v0 = (row as f32) / (ATLAS_ROWS as f32);
    let u1 = ((col + 1) as f32) / (ATLAS_COLS as f32);
    let v1 = ((row + 1) as f32) / (ATLAS_ROWS as f32);
    (u0, v0, u1, v1)
}

/// Triangulates a convex 3D polygon face (`3`, `4`, or `5` vertices in counter-clockwise order),
/// computing outward normals, tangent-plane numeral UVs, and edge distance factors.
fn append_polygon_face(
    vertices_out: &mut Vec<DieVertex>,
    face_normals_out: &mut Vec<[f32; 3]>,
    poly: &[[f32; 3]],
    face_number: usize,
    uv_radius_scale: f32,
) {
    let n_verts = poly.len();
    if n_verts < 3 {
        return;
    }
    let mut centroid = [0.0_f32; 3];
    for &p in poly {
        centroid = vec3_add(centroid, p);
    }
    centroid = vec3_scale(centroid, 1.0 / (n_verts as f32));

    let e0 = vec3_sub(poly[1], poly[0]);
    let e1 = vec3_sub(poly[2], poly[0]);
    let mut normal = vec3_normalize(vec3_cross(e0, e1));
    if vec3_dot(normal, centroid) < 0.0 {
        normal = vec3_scale(normal, -1.0);
    }
    face_normals_out.push(normal);

    let up_ref = if normal[1].abs() < 0.92 {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let tangent_u = vec3_normalize(vec3_cross(up_ref, normal));
    let tangent_v = vec3_normalize(vec3_cross(normal, tangent_u));

    let mut max_r = 1e-4_f32;
    for &p in poly {
        max_r = max_r.max(vec3_len(vec3_sub(p, centroid)));
    }

    let (u0, v0, u1, v1) = atlas_uv_bounds(face_number);
    let (u_mid, v_mid) = (0.5 * (u0 + u1), 0.5 * (v0 + v1));
    let (u_half, v_half) = (0.5 * (u1 - u0) * uv_radius_scale, 0.5 * (v1 - v0) * uv_radius_scale);

    let project_uv = |p: [f32; 3]| -> [f32; 2] {
        let rel = vec3_sub(p, centroid);
        let lx = vec3_dot(rel, tangent_u) / max_r;
        let ly = -vec3_dot(rel, tangent_v) / max_r;
        [u_mid + lx * u_half, v_mid + ly * v_half]
    };

    let c_uv = [u_mid, v_mid];
    let f_idx = face_number as f32;

    // Fan-triangulate from the polygon centroid so edge_factors[0] is 1.0 at the center
    // and 0.0 along every outer polygon edge (giving uniform chamfer bevels on triangles, quads, and pentagons).
    for i in 0..n_verts {
        let next = (i + 1) % n_verts;
        let (mut p_a, mut p_b) = (poly[i], poly[next]);
        let tri_n = vec3_cross(vec3_sub(p_a, centroid), vec3_sub(p_b, centroid));
        if vec3_dot(tri_n, normal) < 0.0 {
            std::mem::swap(&mut p_a, &mut p_b);
        }
        vertices_out.push(DieVertex {
            position: centroid,
            normal,
            uv: c_uv,
            edge_factors: [1.0, f_idx, 1.0],
        });
        vertices_out.push(DieVertex {
            position: p_a,
            normal,
            uv: project_uv(p_a),
            edge_factors: [0.0, f_idx, 0.0],
        });
        vertices_out.push(DieVertex {
            position: p_b,
            normal,
            uv: project_uv(p_b),
            edge_factors: [0.0, f_idx, 1.0],
        });
    }
}

/// Builds the procedural [`PolyhedralMesh`] for a specified [`DieKind`] normalized to unit radius (`~1.0`).
pub fn build_polyhedral_mesh(kind: DieKind) -> PolyhedralMesh {
    let mut vertices = Vec::new();
    let mut face_normals = Vec::new();

    match kind {
        DieKind::D4 => {
            let s = 1.0 / 3.0_f32.sqrt();
            let v = [[s, s, s], [s, -s, -s], [-s, s, -s], [-s, -s, s]];
            for (idx, f) in [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]].iter().enumerate() {
                append_polygon_face(&mut vertices, &mut face_normals, &[v[f[0]], v[f[1]], v[f[2]]], idx + 1, 0.82);
            }
        }
        DieKind::D6 => {
            let h = 0.68_f32;
            let faces: [([[f32; 3]; 4], usize); 6] = [
                ([[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]], 1),
                ([[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]], 2),
                ([[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]], 3),
                ([[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]], 4),
                ([[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]], 5),
                ([[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]], 6),
            ];
            for (poly, num) in faces {
                append_polygon_face(&mut vertices, &mut face_normals, &poly, num, 0.90);
            }
        }
        DieKind::D8 => {
            let r = 1.05_f32;
            let (top, bot) = ([0.0, r, 0.0], [0.0, -r, 0.0]);
            let (px, nx, pz, nz) = ([r, 0.0, 0.0], [-r, 0.0, 0.0], [0.0, 0.0, r], [0.0, 0.0, -r]);
            let faces = [
                [top, pz, px], [top, px, nz], [top, nz, nx], [top, nx, pz],
                [bot, px, pz], [bot, nz, px], [bot, nx, nz], [bot, pz, nx],
            ];
            for (idx, poly) in faces.iter().enumerate() {
                append_polygon_face(&mut vertices, &mut face_normals, poly, idx + 1, 0.84);
            }
        }
        DieKind::D10 => {
            let (apex_top, apex_bot) = ([0.0, 1.05_f32, 0.0], [0.0, -1.05_f32, 0.0]);
            let (h_ring, r_ring) = (0.19_f32, 0.96_f32);
            let mut upper = [[0.0_f32; 3]; 5];
            let mut lower = [[0.0_f32; 3]; 5];
            for i in 0..5 {
                let a_up = (i as f32) * std::f32::consts::TAU / 5.0;
                let a_lo = ((i as f32) + 0.5) * std::f32::consts::TAU / 5.0;
                upper[i] = [r_ring * a_up.cos(), h_ring, r_ring * a_up.sin()];
                lower[i] = [r_ring * a_lo.cos(), -h_ring, r_ring * a_lo.sin()];
            }
            for i in 0..5 {
                let next = (i + 1) % 5;
                append_polygon_face(&mut vertices, &mut face_normals, &[apex_top, upper[i], lower[i], upper[next]], i * 2 + 1, 0.82);
                append_polygon_face(&mut vertices, &mut face_normals, &[apex_bot, lower[i], upper[next], lower[next]], i * 2 + 2, 0.82);
            }
        }
        DieKind::D12 => {
            let ico = raw_icosahedron_triangles();
            let ico_verts = raw_icosahedron_vertices();
            for (v_idx, &v_dir) in ico_verts.iter().enumerate() {
                let mut incident_centers = Vec::with_capacity(5);
                for tri in &ico {
                    if tri[0] == v_idx || tri[1] == v_idx || tri[2] == v_idx {
                        let c = vec3_normalize(vec3_add(
                            vec3_add(ico_verts[tri[0]], ico_verts[tri[1]]),
                            ico_verts[tri[2]],
                        ));
                        incident_centers.push(vec3_scale(c, 1.02));
                    }
                }
                let n = vec3_normalize(v_dir);
                let up_ref = if n[1].abs() < 0.9 { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
                let tu = vec3_normalize(vec3_cross(up_ref, n));
                let tv = vec3_normalize(vec3_cross(n, tu));
                incident_centers.sort_by(|a, b| {
                    let ang_a = vec3_dot(*a, tv).atan2(vec3_dot(*a, tu));
                    let ang_b = vec3_dot(*b, tv).atan2(vec3_dot(*b, tu));
                    ang_a.partial_cmp(&ang_b).unwrap_or(std::cmp::Ordering::Equal)
                });
                append_polygon_face(&mut vertices, &mut face_normals, &incident_centers, v_idx + 1, 0.88);
            }
        }
        DieKind::D20 => {
            let v = raw_icosahedron_vertices();
            for (idx, tri) in raw_icosahedron_triangles().iter().enumerate() {
                append_polygon_face(&mut vertices, &mut face_normals, &[v[tri[0]], v[tri[1]], v[tri[2]]], idx + 1, 0.85);
            }
        }
    }

    PolyhedralMesh {
        kind,
        vertices,
        face_normals,
    }
}

fn raw_icosahedron_vertices() -> [[f32; 3]; 12] {
    let phi = 0.5 * (1.0 + 5.0_f32.sqrt());
    let raw = [
        [-1.0, phi, 0.0], [1.0, phi, 0.0], [-1.0, -phi, 0.0], [1.0, -phi, 0.0],
        [0.0, -1.0, phi], [0.0, 1.0, phi], [0.0, -1.0, -phi], [0.0, 1.0, -phi],
        [phi, 0.0, -1.0], [phi, 0.0, 1.0], [-phi, 0.0, -1.0], [-phi, 0.0, 1.0],
    ];
    let mut out = [[0.0_f32; 3]; 12];
    for (i, r) in raw.iter().enumerate() {
        out[i] = vec3_scale(vec3_normalize(*r), 1.04);
    }
    out
}

fn raw_icosahedron_triangles() -> [[usize; 3]; 20] {
    [
        [0, 11, 5], [0, 5, 1], [0, 1, 7], [0, 7, 10], [0, 10, 11],
        [1, 5, 9], [5, 11, 4], [11, 10, 2], [10, 7, 6], [7, 1, 8],
        [3, 9, 4], [3, 4, 2], [3, 2, 6], [3, 6, 8], [3, 8, 9],
        [4, 9, 5], [2, 4, 11], [6, 2, 10], [8, 6, 7], [9, 8, 1],
    ]
}

// ============================================================================
// Procedural Vector-SDF Numeral Texture Atlas (`1..=20`)
// ============================================================================

/// Generates an anti-aliased RGBA8 texture (`ATLAS_WIDTH x ATLAS_HEIGHT`) containing crisp vector
/// numerals `1..=20` across a `5x4` grid of `128x128` cells.
pub fn generate_dice_numeral_atlas_rgba8() -> Vec<u8> {
    let mut pixels = vec![0_u8; (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize];

    for face_num in 1..=20_u32 {
        let cell_idx = face_num - 1;
        let base_x = (cell_idx % ATLAS_COLS) * ATLAS_CELL_SIZE;
        let base_y = (cell_idx / ATLAS_COLS) * ATLAS_CELL_SIZE;
        let needs_underline = face_num == 6 || face_num == 9;

        for py in 0..ATLAS_CELL_SIZE {
            for px in 0..ATLAS_CELL_SIZE {
                let nx = ((px as f32 + 0.5) / (ATLAS_CELL_SIZE as f32)) * 2.0 - 1.0;
                let ny = ((py as f32 + 0.5) / (ATLAS_CELL_SIZE as f32)) * 2.0 - 1.0;

                let mut min_dist = if face_num < 10 {
                    digit_sdf(face_num as u8, nx / 0.20, ny / 0.23)
                } else {
                    let d0 = digit_sdf((face_num / 10) as u8, (nx + 0.135) / 0.15, ny / 0.21);
                    let d1 = digit_sdf((face_num % 10) as u8, (nx - 0.135) / 0.15, ny / 0.21);
                    d0.min(d1)
                };

                if needs_underline {
                    min_dist = min_dist.min(segment_sdf(nx, ny, -0.12, 0.27, 0.12, 0.27) - 0.025);
                }

                let edge_softness = 0.028_f32;
                let alpha_f = ((edge_softness - min_dist) / (2.0 * edge_softness)).clamp(0.0, 1.0);
                if alpha_f > 0.0 {
                    let a = (alpha_f * 255.0).round() as u8;
                    let idx = (((base_y + py) * ATLAS_WIDTH + (base_x + px)) * 4) as usize;
                    pixels[idx..idx + 4].copy_from_slice(&[255, 255, 255, a]);
                }
            }
        }
    }

    pixels
}

fn segment_sdf(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let (pax, pay) = (px - ax, py - ay);
    let (bax, bay) = (bx - ax, by - ay);
    let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay).max(1e-6)).clamp(0.0, 1.0);
    let (dx, dy) = (pax - bax * h, pay - bay * h);
    (dx * dx + dy * dy).sqrt()
}

fn digit_sdf(digit: u8, x: f32, y: f32) -> f32 {
    let (l, r, t, m, b) = (-0.50_f32, 0.50_f32, -0.72_f32, 0.00_f32, 0.72_f32);
    let segs: &[([f32; 2], [f32; 2])] = match digit {
        0 => &[([l, t], [r, t]), ([r, t], [r, b]), ([r, b], [l, b]), ([l, b], [l, t])],
        1 => &[([-0.22, -0.42], [0.05, t]), ([0.05, t], [0.05, b]), ([-0.30, b], [0.38, b])],
        2 => &[([l, t], [r, t]), ([r, t], [r, m]), ([r, m], [l, m]), ([l, m], [l, b]), ([l, b], [r, b])],
        3 => &[([l, t], [r, t]), ([r, t], [r, b]), ([l, m], [r, m]), ([l, b], [r, b])],
        4 => &[([l, t], [l, m]), ([l, m], [r, m]), ([r, t], [r, b])],
        5 => &[([r, t], [l, t]), ([l, t], [l, m]), ([l, m], [r, m]), ([r, m], [r, b]), ([r, b], [l, b])],
        6 => &[([r, t], [l, t]), ([l, t], [l, b]), ([l, b], [r, b]), ([r, b], [r, m]), ([r, m], [l, m])],
        7 => &[([l, t], [r, t]), ([r, t], [-0.10, b])],
        8 => &[([l, t], [r, t]), ([r, t], [r, b]), ([r, b], [l, b]), ([l, b], [l, t]), ([l, m], [r, m])],
        9 => &[([l, m], [l, t]), ([l, t], [r, t]), ([r, t], [r, b]), ([r, b], [l, b]), ([l, m], [r, m])],
        _ => &[],
    };
    let mut d = 10.0_f32;
    for &(a, b_pt) in segs {
        d = d.min(segment_sdf(x, y, a[0], a[1], b_pt[0], b_pt[1]));
    }
    d - 0.125
}

// ============================================================================
// Deterministic 3D Physics Trajectory & Outcome Alignment
// ============================================================================

/// Deterministic SplitMix64 PRNG synchronizing 3D dice physics across peers from `physics_seed`.
#[derive(Debug, Clone)]
pub struct DeterministicPrng {
    state: u64,
}

impl DeterministicPrng {
    pub fn new(seed: i64) -> Self {
        Self {
            state: (seed as u64) ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / ((1_u32 << 24) as f32)
    }

    pub fn range_f32(&mut self, min_val: f32, max_val: f32) -> f32 {
        min_val + (max_val - min_val) * self.next_f32()
    }
}

/// Computes a 3x3 column-major rotation matrix rotating `from_unit` onto `+Y` (`[0, 1, 0]`) plus yaw `yaw_rad`.
pub fn rotation_aligning_normal_to_up(from_unit: [f32; 3], yaw_rad: f32) -> [[f32; 3]; 3] {
    let f = vec3_normalize(from_unit);
    let to = [0.0_f32, 1.0, 0.0];
    let dot = vec3_dot(f, to).clamp(-1.0, 1.0);

    let align_mat = if dot > 0.9999 {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    } else if dot < -0.9999 {
        [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]
    } else {
        axis_angle_matrix(vec3_normalize(vec3_cross(f, to)), dot.acos())
    };
    mat3_mul(axis_angle_matrix([0.0, 1.0, 0.0], yaw_rad), align_mat)
}

/// Constructs a 3x3 column-major rotation matrix from a unit `axis` and `angle_rad`.
pub fn axis_angle_matrix(axis: [f32; 3], angle_rad: f32) -> [[f32; 3]; 3] {
    let [x, y, z] = vec3_normalize(axis);
    let (s, c) = angle_rad.sin_cos();
    let t = 1.0 - c;
    [
        [t * x * x + c, t * x * y + s * z, t * x * z - s * y],
        [t * x * y - s * z, t * y * y + c, t * y * z + s * x],
        [t * x * z + s * y, t * y * z - s * x, t * z * z + c],
    ]
}

/// Multiplies two 3x3 column-major matrices `a * b`.
pub fn mat3_mul(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut out = [[0.0_f32; 3]; 3];
    for col in 0..3 {
        for row in 0..3 {
            out[col][row] = a[0][row] * b[col][0] + a[1][row] * b[col][1] + a[2][row] * b[col][2];
        }
    }
    out
}

/// Transforms a 3D vector `v` by a 3x3 column-major matrix `m`.
pub fn mat3_mul_vec3(m: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
        m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
        m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
    ]
}

/// Per-die deterministic trajectory state inside a [`DicePhysicsScene`].
#[derive(Debug, Clone)]
pub struct SimulatedDie {
    pub kind: DieKind,
    pub outcome: i32,
    pub start_pos: [f32; 3],
    pub target_pos: [f32; 3],
    pub spin_axis: [f32; 3],
    pub total_spin_rad: f32,
    pub resting_rotation: [[f32; 3]; 3],
}

/// Active multi-die physics scene driven by a [`WgpuDiceSpec`].
#[derive(Debug, Clone)]
pub struct DicePhysicsScene {
    pub physics_seed: i64,
    pub dice_notation: String,
    pub dice: Vec<SimulatedDie>,
    pub progress: f32,
}

impl DicePhysicsScene {
    pub fn from_spec(spec: &WgpuDiceSpec) -> Self {
        let outcomes = if spec.outcomes.is_empty() {
            vec![1, 20]
        } else {
            spec.outcomes.clone()
        };
        let (kind, count) = parse_dice_notation_kind(&spec.dice_notation, outcomes.len());
        let mesh = build_polyhedral_mesh(kind);
        let mut rng = DeterministicPrng::new(spec.physics_seed);

        let cols = (count as f32).sqrt().ceil().max(1.0) as usize;
        let rows = count.div_ceil(cols);
        let spacing = if count <= 2 {
            2.45
        } else if count <= 6 {
            2.15
        } else {
            1.75
        };

        let mut dice = Vec::with_capacity(count);
        for i in 0..count {
            let outcome = outcomes.get(i).copied().unwrap_or(1).clamp(1, kind.sides());
            let (col, row) = (i % cols, i / cols);
            let grid_x = ((col as f32) - ((cols.saturating_sub(1)) as f32) * 0.5) * spacing;
            let grid_z = ((row as f32) - ((rows.saturating_sub(1)) as f32) * 0.5) * spacing;
            let target_pos = [
                grid_x + rng.range_f32(-0.18, 0.18),
                0.0,
                grid_z + rng.range_f32(-0.18, 0.18),
            ];
            let start_pos = [
                grid_x * 0.55 + rng.range_f32(-2.2, -1.1),
                rng.range_f32(2.6, 3.8),
                grid_z * 0.55 + rng.range_f32(1.4, 2.5),
            ];
            let spin_axis = vec3_normalize([
                rng.range_f32(-1.0, 1.0),
                rng.range_f32(-0.8, 0.8),
                rng.range_f32(-1.0, 1.0),
            ]);
            let total_spin_rad = std::f32::consts::TAU * rng.range_f32(2.0, 4.0);
            let yaw = rng.range_f32(-0.35, 0.35);
            let face_normal = mesh
                .face_normals
                .get((outcome - 1) as usize)
                .copied()
                .unwrap_or([0.0, 1.0, 0.0]);
            let resting_rotation = rotation_aligning_normal_to_up(face_normal, yaw);

            dice.push(SimulatedDie {
                kind,
                outcome,
                start_pos,
                target_pos,
                spin_axis,
                total_spin_rad,
                resting_rotation,
            });
        }

        Self {
            physics_seed: spec.physics_seed,
            dice_notation: spec.dice_notation.clone(),
            dice,
            progress: 0.0,
        }
    }

    /// Evaluates the 3D position and 3x3 rotation matrix of die `index` at normalized time `t` in `[0.0, 1.0]`.
    pub fn sample_die_transform(&self, index: usize, t: f32) -> ([f32; 3], [[f32; 3]; 3]) {
        let Some(die) = self.dice.get(index) else {
            return (
                [0.0, 0.0, 0.0],
                [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            );
        };
        let u = t.clamp(0.0, 1.0);
        let ease = 1.0 - (1.0 - u).powi(3);
        let x = die.start_pos[0] + (die.target_pos[0] - die.start_pos[0]) * ease;
        let z = die.start_pos[2] + (die.target_pos[2] - die.start_pos[2]) * ease;

        let y = if u < 0.45 {
            let local = u / 0.45;
            die.start_pos[1] * (1.0 - local * local)
        } else if u < 0.75 {
            let local = (u - 0.45) / 0.30;
            1.15 * 4.0 * local * (1.0 - local)
        } else if u < 0.92 {
            let local = (u - 0.75) / 0.17;
            0.38 * 4.0 * local * (1.0 - local)
        } else {
            let local = (u - 0.92) / 0.08;
            0.08 * 4.0 * local * (1.0 - local)
        };

        let remaining_spin = die.total_spin_rad * (1.0 - u).powi(2);
        let tumble = axis_angle_matrix(die.spin_axis, remaining_spin);
        ([x, y, z], mat3_mul(tumble, die.resting_rotation))
    }
}

/// Scene-wide uniform buffer uploaded once per frame (`112` bytes, 16-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SceneUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    pub base_color: [f32; 4],
    pub ink_color: [f32; 4],
}

/// Per-die instance model matrix uniform (`64` bytes, 16-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DieInstanceData {
    pub model: [[f32; 4]; 4],
}

/// Interactive orbit (`yaw_deg`, `pitch_deg`), `pan`, and `zoom` camera configuration for the 3D WebGPU dice tray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiceCameraConfig {
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub zoom: f32,
    pub pan: [f32; 2],
}

impl Default for DiceCameraConfig {
    fn default() -> Self {
        Self {
            yaw_deg: 0.0,
            pitch_deg: 55.0,
            zoom: 1.0,
            pan: [0.0, 0.0],
        }
    }
}

impl DiceCameraConfig {
    pub fn orbit_by(&mut self, delta_yaw_deg: f32, delta_pitch_deg: f32) {
        let mut y = (self.yaw_deg + delta_yaw_deg) % 360.0;
        if y > 180.0 {
            y -= 360.0;
        } else if y < -180.0 {
            y += 360.0;
        }
        self.yaw_deg = y;
        self.pitch_deg = (self.pitch_deg + delta_pitch_deg).clamp(12.0, 88.0);
    }

    pub fn pan_by_screen_delta(&mut self, right_delta: f32, forward_delta: f32) {
        let (sin_y, cos_y) = self.yaw_deg.to_radians().sin_cos();
        let inv_zoom = 1.0 / self.zoom.clamp(0.40, 2.50);
        let dx = (right_delta * cos_y - forward_delta * sin_y) * inv_zoom;
        let dz = (-right_delta * sin_y - forward_delta * cos_y) * inv_zoom;
        self.pan[0] = (self.pan[0] + dx).clamp(-6.0, 6.0);
        self.pan[1] = (self.pan[1] + dz).clamp(-6.0, 6.0);
    }

    pub fn zoom_by_factor(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.40, 2.50);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Perspective look-at view-projection matrix builder using default camera parameters.
pub fn compute_view_proj_matrix(aspect: f32, die_count: usize) -> ([[f32; 4]; 4], [f32; 3]) {
    compute_view_proj_matrix_with_camera(aspect, die_count, &DiceCameraConfig::default())
}

/// Perspective look-at view-projection matrix builder with user orbit (`yaw_deg`, `pitch_deg`), `zoom`, and `pan`.
pub fn compute_view_proj_matrix_with_camera(
    aspect: f32,
    die_count: usize,
    camera: &DiceCameraConfig,
) -> ([[f32; 4]; 4], [f32; 3]) {
    let dist_scale = if die_count <= 2 {
        1.0
    } else if die_count <= 6 {
        1.28
    } else {
        1.65
    };
    let zoom = camera.zoom.clamp(0.40, 2.50);
    let radius = (7.52_f32 * dist_scale) / zoom;
    let pitch_rad = camera.pitch_deg.clamp(12.0, 88.0).to_radians();
    let yaw_rad = camera.yaw_deg.to_radians();

    let target = [
        camera.pan[0].clamp(-6.0, 6.0),
        0.0_f32,
        0.15 + camera.pan[1].clamp(-6.0, 6.0),
    ];
    let horiz = radius * pitch_rad.cos();
    let eye = [
        target[0] + horiz * yaw_rad.sin(),
        target[1] + radius * pitch_rad.sin(),
        target[2] + horiz * yaw_rad.cos(),
    ];
    let f = vec3_normalize(vec3_sub(target, eye));
    let s = vec3_normalize(vec3_cross(f, [0.0, 1.0, 0.0]));
    let u = vec3_cross(s, f);

    let view = [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-vec3_dot(s, eye), -vec3_dot(u, eye), vec3_dot(f, eye), 1.0],
    ];
    let f_cot = 1.0 / (0.5 * 40.0_f32.to_radians()).tan();
    let (z_near, z_far) = (0.2_f32, 65.0_f32);
    let proj = [
        [f_cot / aspect.max(0.25), 0.0, 0.0, 0.0],
        [0.0, f_cot, 0.0, 0.0],
        [0.0, 0.0, z_far / (z_near - z_far), -1.0],
        [0.0, 0.0, (z_near * z_far) / (z_near - z_far), 0.0],
    ];
    (mul_mat4_cols(proj, view), eye)
}

/// Packs a 3D translation `pos` and 3x3 rotation `rot` into a 4x4 column-major model matrix.
pub fn pack_model_matrix(pos: [f32; 3], rot: [[f32; 3]; 3]) -> [[f32; 4]; 4] {
    [
        [rot[0][0], rot[0][1], rot[0][2], 0.0],
        [rot[1][0], rot[1][1], rot[1][2], 0.0],
        [rot[2][0], rot[2][1], rot[2][2], 0.0],
        [pos[0], pos[1], pos[2], 1.0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use satya_schema::MessageVisibility;

    #[test]
    fn test_all_polyhedral_meshes_generate_valid_normals_and_uvs() {
        for kind in [
            DieKind::D4,
            DieKind::D6,
            DieKind::D8,
            DieKind::D10,
            DieKind::D12,
            DieKind::D20,
        ] {
            let mesh = build_polyhedral_mesh(kind);
            assert_eq!(mesh.face_normals.len(), kind.sides() as usize);
            assert!(
                !mesh.vertices.is_empty() && mesh.vertices.len().is_multiple_of(3),
                "Mesh for {:?} must contain triangle triplets",
                kind
            );
            for n in &mesh.face_normals {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                assert!((len - 1.0).abs() < 1e-4, "Face normal must be unit length");
            }
            for v in &mesh.vertices {
                assert!(v.uv[0] >= 0.0 && v.uv[0] <= 1.0);
                assert!(v.uv[1] >= 0.0 && v.uv[1] <= 1.0);
            }
        }
    }

    #[test]
    fn test_numeral_atlas_generates_compact_strokes_for_all_20_cells() {
        let rgba = generate_dice_numeral_atlas_rgba8();
        assert_eq!(rgba.len(), (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize);

        for face_num in 1..=20_u32 {
            let idx = face_num - 1;
            let col = idx % ATLAS_COLS;
            let row = idx / ATLAS_COLS;
            let mut non_zero_alpha = 0_usize;
            let mut max_radius_from_center = 0.0_f32;
            for py in 0..ATLAS_CELL_SIZE {
                for px in 0..ATLAS_CELL_SIZE {
                    let x = col * ATLAS_CELL_SIZE + px;
                    let y = row * ATLAS_CELL_SIZE + py;
                    let a = rgba[((y * ATLAS_WIDTH + x) * 4 + 3) as usize];
                    if a > 128 {
                        non_zero_alpha += 1;
                        let nx = ((px as f32 + 0.5) / (ATLAS_CELL_SIZE as f32)) * 2.0 - 1.0;
                        let ny = ((py as f32 + 0.5) / (ATLAS_CELL_SIZE as f32)) * 2.0 - 1.0;
                        max_radius_from_center =
                            max_radius_from_center.max((nx * nx + ny * ny).sqrt());
                    }
                }
            }
            assert!(
                non_zero_alpha > 40,
                "Atlas cell for face {face_num} must contain rendered numeral strokes (got {non_zero_alpha})"
            );
            assert!(
                max_radius_from_center < 0.40,
                "Numeral for face {face_num} must stay compact within cell center (max_r = {max_radius_from_center})"
            );
        }
    }

    #[test]
    fn test_deterministic_physics_aligns_top_face_normal_to_outcome() {
        let spec = WgpuDiceSpec {
            dice_notation: "3d20".to_string(),
            physics_seed: 1337,
            outcomes: vec![1, 12, 20],
            visibility: MessageVisibility::Public,
        };
        let scene_a = DicePhysicsScene::from_spec(&spec);
        let scene_b = DicePhysicsScene::from_spec(&spec);
        let mesh = build_polyhedral_mesh(DieKind::D20);

        for i in 0..3 {
            let (pos_a, rot_a) = scene_a.sample_die_transform(i, 1.0);
            let (pos_b, rot_b) = scene_b.sample_die_transform(i, 1.0);
            assert_eq!(pos_a, pos_b);
            assert_eq!(rot_a, rot_b);
            assert!(pos_a[1].abs() < 1e-5);

            let outcome = spec.outcomes[i] as usize;
            let world_n = mat3_mul_vec3(rot_a, mesh.face_normals[outcome - 1]);
            assert!(
                (world_n[1] - 1.0).abs() < 1e-4,
                "Outcome {outcome} face normal must point straight up (+Y) at t=1.0, got {world_n:?}"
            );
        }
    }

    #[test]
    fn test_material_config_and_camera_orbit_pan_zoom() {
        let def = DiceMaterialConfig::default();
        assert!(def.opacity <= 0.45);

        let mat = DiceMaterialConfig::from_hex_and_opacity("#38BDF8", "#FDE047", 0.35);
        assert_eq!(mat.base_color_hex, "#38bdf8");
        assert_eq!(mat.ink_color_hex, "#fde047");
        assert!((mat.opacity - 0.35).abs() < 1e-5);

        let amber = DiceMaterialConfig::from_preset("amber_resin");
        assert_eq!(amber.base_color_hex, "#f59e0b");
        assert!(amber.opacity < 0.45);

        let mut cam = DiceCameraConfig::default();
        let (_, eye_default) = compute_view_proj_matrix_with_camera(2.0, 2, &cam);
        cam.zoom_by_factor(1.5);
        let (_, eye_zoomed) = compute_view_proj_matrix_with_camera(2.0, 2, &cam);
        assert!(vec3_len(eye_zoomed) < vec3_len(eye_default));

        cam.orbit_by(90.0, 15.0);
        assert!((cam.yaw_deg - 90.0).abs() < 1e-4);
        assert!((cam.pitch_deg - 70.0).abs() < 1e-4);

        cam.pan_by_screen_delta(1.2, -0.8);
        assert!(cam.pan[0].abs() > 0.01 || cam.pan[1].abs() > 0.01);

        cam.reset();
        assert_eq!(cam, DiceCameraConfig::default());
    }
}
