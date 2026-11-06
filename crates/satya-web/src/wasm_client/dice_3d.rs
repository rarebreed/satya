//! Synchronized 3D WebGPU (`wgpu`) Polyhedral Dice Roller, Procedural Platonic/Trapezohedral Mesh
//! Generator, SDF Numeral Texture Atlas, Deterministic Physics Trajectory Engine, and Two-Pass
//! Translucent Shader Pipeline.
//!
//! ## Architecture
//! 1. **Procedural Polyhedral Geometry ([`DieKind`], [`PolyhedralMesh`])**:
//!    Generates exact 3D vertex buffers, flat facet normals, barycentric edge coordinates, and
//!    per-face UV coordinates for standard TTRPG polyhedra (`d4`, `d6`, `d8`, `d10`, `d12`, `d20`)
//!    entirely in Rust without external asset files.
//! 2. **Procedural Numeral Texture Atlas ([`generate_dice_numeral_atlas_rgba8`])**:
//!    Synthesizes a `640x512` RGBA8 texture (`5x4` grid of `128x128` cells for faces `1..=20`)
//!    using anti-aliased signed-distance-field (SDF) seven-segment/vector strokes and orientation
//!    underline bars on `6` and `9`.
//! 3. **Customizable Translucent Material ([`DiceMaterialConfig`])**:
//!    Controls die body RGB color, numeral ink RGB color, and alpha opacity/translucency
//!    (`0.15..=1.0`), with two-pass rendering (`CullMode::Front` interior pass followed by
//!    `CullMode::Back` exterior pass with Fresnel rim lighting and specular highlights).
//! 4. **Deterministic Seeded Physics ([`DicePhysicsScene`])**:
//!    Drives parabolic throw trajectories, floor bounces, and angular spin from
//!    [`satya_schema::WgpuDiceSpec::physics_seed`], aligning each die's final resting orientation
//!    so the face matching [`satya_schema::WgpuDiceSpec::outcomes`] points directly toward `+Y`.

use bytemuck::{Pod, Zeroable};
use satya_schema::WgpuDiceSpec;
use wasm_bindgen::JsCast;

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

/// Supported polyhedral RPG die shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DieKind {
    /// 4-sided regular tetrahedron (`1..=4`).
    D4,
    /// 6-sided regular hexahedron / cube (`1..=6`).
    D6,
    /// 8-sided regular octahedron (`1..=8`).
    D8,
    /// 10-sided pentagonal trapezohedron (`1..=10`).
    D10,
    /// 12-sided regular dodecahedron (`1..=12`).
    D12,
    /// 20-sided regular icosahedron (`1..=20`), primary die for Satya `2d20` cascades.
    D20,
}

impl DieKind {
    /// Returns the number of faces on this polyhedral die.
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

    /// Maps an arbitrary side count (`4, 6, 8, 10, 12, 20`) to the closest [`DieKind`].
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

    /// Short label for UI telemetry pills (`"d4"`, `"d6"`, `"d8"`, `"d10"`, `"d12"`, `"d20"`).
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

/// GPU vertex structure for a polyhedral die facet triangle.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct DieVertex {
    /// 3D model-space vertex position.
    pub position: [f32; 3],
    /// Outward unit normal vector of the facet.
    pub normal: [f32; 3],
    /// Atlas UV coordinates mapping the face's numeral (`[0.0..1.0, 0.0..1.0]`).
    pub uv: [f32; 2],
    /// Barycentric/edge coordinates (`x`: distance to polygon edge in `[0, 1]`, `y`: face index, `z`: reserved).
    pub edge_factors: [f32; 3],
}

/// CPU-side procedural mesh for a polyhedral die, including face normals for outcome alignment.
#[derive(Debug, Clone)]
pub struct PolyhedralMesh {
    /// Die shape represented by this mesh.
    pub kind: DieKind,
    /// Interleaved triangle vertices (`3` per triangle) ready for upload to a `wgpu::Buffer`.
    pub vertices: Vec<DieVertex>,
    /// Outward unit normal vector for each numbered face (`index 0` = face `1`, ..., `index N-1` = face `N`).
    pub face_normals: Vec<[f32; 3]>,
}

/// User-customizable color, ink, and translucency settings for the 3D WebGPU dice renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct DiceMaterialConfig {
    /// Hex color string (`"#RRGGBB"`) for the translucent or opaque die body.
    pub base_color_hex: String,
    /// Linear/sRGB `[r, g, b]` (`0.0..=1.0`) for the die body.
    pub base_rgb: [f32; 3],
    /// Hex color string (`"#RRGGBB"`) for the engraved face numerals.
    pub ink_color_hex: String,
    /// Linear/sRGB `[r, g, b]` (`0.0..=1.0`) for the engraved face numerals.
    pub ink_rgb: [f32; 3],
    /// Alpha opacity (`0.15` glass translucent to `1.0` solid opaque).
    pub opacity: f32,
}

impl Default for DiceMaterialConfig {
    fn default() -> Self {
        Self::from_hex_and_opacity("#0ea5e9", "#f8fafc", 0.68)
    }
}

impl DiceMaterialConfig {
    /// Constructs a [`DiceMaterialConfig`] from `#RRGGBB` hex strings and an opacity in `[0.15, 1.0]`.
    pub fn from_hex_and_opacity(base_hex: &str, ink_hex: &str, opacity: f32) -> Self {
        let (norm_base, base_rgb) = parse_hex_color(base_hex, [0.055, 0.647, 0.914]);
        let (norm_ink, ink_rgb) = parse_hex_color(ink_hex, [0.973, 0.980, 0.988]);
        Self {
            base_color_hex: norm_base,
            base_rgb,
            ink_color_hex: norm_ink,
            ink_rgb,
            opacity: opacity.clamp(0.15, 1.0),
        }
    }

    /// Returns a named preset material configuration (`"translucent_sapphire"`, `"amber_resin"`,
    /// `"emerald_glass"`, `"crimson_ruby"`, or `"obsidian_opaque"`).
    pub fn from_preset(preset: &str) -> Self {
        match preset.trim() {
            "amber_resin" => Self::from_hex_and_opacity("#f59e0b", "#fef3c7", 0.64),
            "emerald_glass" => Self::from_hex_and_opacity("#10b981", "#ecfdf5", 0.58),
            "crimson_ruby" => Self::from_hex_and_opacity("#e11d48", "#ffe4e6", 0.62),
            "amethyst_void" => Self::from_hex_and_opacity("#9333ea", "#f3e8ff", 0.60),
            "obsidian_opaque" => Self::from_hex_and_opacity("#1e293b", "#38bdf8", 1.00),
            _ => Self::from_hex_and_opacity("#0ea5e9", "#f8fafc", 0.68),
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
        let final_count = outcome_count.clamp(1, 24);
        (DieKind::D20, final_count)
    }
}

// ============================================================================
// Pure-Rust 3D Vector & Matrix Math Helpers
// ============================================================================

#[inline]
fn vec3_add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

#[inline]
fn vec3_sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[inline]
fn vec3_scale(v: [f32; 3], s: f32) -> [f32; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

#[inline]
fn vec3_dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

#[inline]
fn vec3_cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[inline]
fn vec3_len(v: [f32; 3]) -> f32 {
    vec3_dot(v, v).sqrt()
}

#[inline]
fn vec3_normalize(v: [f32; 3]) -> [f32; 3] {
    let l = vec3_len(v);
    if l > 1e-6 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
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

    // Construct an orthonormal tangent frame (tangent_u, tangent_v) in the face plane
    let up_ref = if normal[1].abs() < 0.92 {
        [0.0, 1.0, 0.0]
    } else {
        [0.0, 0.0, 1.0]
    };
    let tangent_u = vec3_normalize(vec3_cross(up_ref, normal));
    let tangent_v = vec3_normalize(vec3_cross(normal, tangent_u));

    let mut max_r = 1e-4_f32;
    for &p in poly {
        let r = vec3_len(vec3_sub(p, centroid));
        if r > max_r {
            max_r = r;
        }
    }

    let (u0, v0, u1, v1) = atlas_uv_bounds(face_number);
    let u_mid = 0.5 * (u0 + u1);
    let v_mid = 0.5 * (v0 + v1);
    let u_half = 0.5 * (u1 - u0) * uv_radius_scale;
    let v_half = 0.5 * (v1 - v0) * uv_radius_scale;

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
        let mut p_a = poly[i];
        let mut p_b = poly[next];
        // Ensure counter-clockwise winding around outward normal
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
            let faces = [[0, 2, 1], [0, 1, 3], [0, 3, 2], [1, 2, 3]];
            for (idx, f) in faces.iter().enumerate() {
                let poly = [v[f[0]], v[f[1]], v[f[2]]];
                append_polygon_face(&mut vertices, &mut face_normals, &poly, idx + 1, 0.62);
            }
        }
        DieKind::D6 => {
            let h = 0.68_f32;
            // Opposite faces sum to 7: (1: +Y, 6: -Y, 2: +Z, 5: -Z, 3: +X, 4: -X)
            let faces: [([[f32; 3]; 4], usize); 6] = [
                ([[-h, h, h], [h, h, h], [h, h, -h], [-h, h, -h]], 1),
                ([[-h, -h, h], [h, -h, h], [h, h, h], [-h, h, h]], 2),
                ([[h, -h, h], [h, -h, -h], [h, h, -h], [h, h, h]], 3),
                ([[-h, -h, -h], [-h, -h, h], [-h, h, h], [-h, h, -h]], 4),
                ([[h, -h, -h], [-h, -h, -h], [-h, h, -h], [h, h, -h]], 5),
                ([[-h, -h, -h], [h, -h, -h], [h, -h, h], [-h, -h, h]], 6),
            ];
            for (poly, num) in faces {
                append_polygon_face(&mut vertices, &mut face_normals, &poly, num, 0.72);
            }
        }
        DieKind::D8 => {
            let r = 1.05_f32;
            let top = [0.0, r, 0.0];
            let bot = [0.0, -r, 0.0];
            let px = [r, 0.0, 0.0];
            let nx = [-r, 0.0, 0.0];
            let pz = [0.0, 0.0, r];
            let nz = [0.0, 0.0, -r];
            let faces = [
                [top, pz, px],
                [top, px, nz],
                [top, nz, nx],
                [top, nx, pz],
                [bot, px, pz],
                [bot, nz, px],
                [bot, nx, nz],
                [bot, pz, nx],
            ];
            for (idx, poly) in faces.iter().enumerate() {
                append_polygon_face(&mut vertices, &mut face_normals, poly, idx + 1, 0.65);
            }
        }
        DieKind::D10 => {
            // Pentagonal trapezohedron: 2 apices + 2 staggered rings of 5 vertices
            let apex_top = [0.0, 1.05_f32, 0.0];
            let apex_bot = [0.0, -1.05_f32, 0.0];
            let h_ring = 0.19_f32;
            let r_ring = 0.96_f32;
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
                let kite_top = [apex_top, upper[i], lower[i], upper[next]];
                append_polygon_face(&mut vertices, &mut face_normals, &kite_top, i * 2 + 1, 0.62);
                let kite_bot = [apex_bot, lower[i], upper[next], lower[next]];
                append_polygon_face(&mut vertices, &mut face_normals, &kite_bot, i * 2 + 2, 0.62);
            }
        }
        DieKind::D12 => {
            // Regular dodecahedron constructed as the dual of the regular icosahedron
            let ico = raw_icosahedron_triangles();
            let ico_verts = raw_icosahedron_vertices();
            // Each vertex of the icosahedron corresponds to a pentagonal face of the dodecahedron
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
                // Sort the 5 centers counter-clockwise around v_dir
                let n = vec3_normalize(v_dir);
                let up_ref = if n[1].abs() < 0.9 {
                    [0.0, 1.0, 0.0]
                } else {
                    [1.0, 0.0, 0.0]
                };
                let tu = vec3_normalize(vec3_cross(up_ref, n));
                let tv = vec3_normalize(vec3_cross(n, tu));
                incident_centers.sort_by(|a, b| {
                    let ang_a = vec3_dot(*a, tv).atan2(vec3_dot(*a, tu));
                    let ang_b = vec3_dot(*b, tv).atan2(vec3_dot(*b, tu));
                    ang_a
                        .partial_cmp(&ang_b)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                append_polygon_face(
                    &mut vertices,
                    &mut face_normals,
                    &incident_centers,
                    v_idx + 1,
                    0.68,
                );
            }
        }
        DieKind::D20 => {
            let v = raw_icosahedron_vertices();
            let tris = raw_icosahedron_triangles();
            for (idx, tri) in tris.iter().enumerate() {
                let poly = [v[tri[0]], v[tri[1]], v[tri[2]]];
                append_polygon_face(&mut vertices, &mut face_normals, &poly, idx + 1, 0.65);
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
        [-1.0, phi, 0.0],
        [1.0, phi, 0.0],
        [-1.0, -phi, 0.0],
        [1.0, -phi, 0.0],
        [0.0, -1.0, phi],
        [0.0, 1.0, phi],
        [0.0, -1.0, -phi],
        [0.0, 1.0, -phi],
        [phi, 0.0, -1.0],
        [phi, 0.0, 1.0],
        [-phi, 0.0, -1.0],
        [-phi, 0.0, 1.0],
    ];
    let mut out = [[0.0_f32; 3]; 12];
    for (i, r) in raw.iter().enumerate() {
        out[i] = vec3_scale(vec3_normalize(*r), 1.04);
    }
    out
}

fn raw_icosahedron_triangles() -> [[usize; 3]; 20] {
    [
        [0, 11, 5],
        [0, 5, 1],
        [0, 1, 7],
        [0, 7, 10],
        [0, 10, 11],
        [1, 5, 9],
        [5, 11, 4],
        [11, 10, 2],
        [10, 7, 6],
        [7, 1, 8],
        [3, 9, 4],
        [3, 4, 2],
        [3, 2, 6],
        [3, 6, 8],
        [3, 8, 9],
        [4, 9, 5],
        [2, 4, 11],
        [6, 2, 10],
        [8, 6, 7],
        [9, 8, 1],
    ]
}

// ============================================================================
// Procedural Vector-SDF Numeral Texture Atlas (`1..=20`)
// ============================================================================

/// Generates an anti-aliased RGBA8 texture (`ATLAS_WIDTH x ATLAS_HEIGHT`) containing crisp vector
/// numerals `1..=20` across a `5x4` grid of `128x128` cells.
///
/// The alpha channel (`a`) encodes the numeral stroke coverage (`0..=255`) so the WGSL shader can
/// tint the numerals dynamically using any user-selected `ink_rgb` color without regenerating the texture.
pub fn generate_dice_numeral_atlas_rgba8() -> Vec<u8> {
    let mut pixels = vec![0_u8; (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize];

    for face_num in 1..=20_u32 {
        let cell_idx = face_num - 1;
        let col = cell_idx % ATLAS_COLS;
        let row = cell_idx / ATLAS_COLS;
        let base_x = col * ATLAS_CELL_SIZE;
        let base_y = row * ATLAS_CELL_SIZE;

        let digits: Vec<u8> = if face_num >= 10 {
            vec![(face_num / 10) as u8, (face_num % 10) as u8]
        } else {
            vec![face_num as u8]
        };
        let needs_underline = face_num == 6 || face_num == 9;

        for py in 0..ATLAS_CELL_SIZE {
            for px in 0..ATLAS_CELL_SIZE {
                // Normalized cell coordinates in [-1.0, 1.0]
                let nx = ((px as f32 + 0.5) / (ATLAS_CELL_SIZE as f32)) * 2.0 - 1.0;
                let ny = ((py as f32 + 0.5) / (ATLAS_CELL_SIZE as f32)) * 2.0 - 1.0;

                let mut min_dist = 10.0_f32;
                if digits.len() == 1 {
                    min_dist = min_dist.min(digit_sdf(digits[0], nx / 0.36, ny / 0.42));
                } else {
                    let d0 = digit_sdf(digits[0], (nx + 0.24) / 0.27, ny / 0.38);
                    let d1 = digit_sdf(digits[1], (nx - 0.24) / 0.27, ny / 0.38);
                    min_dist = min_dist.min(d0).min(d1);
                }

                if needs_underline {
                    let ul = segment_sdf(nx, ny, -0.24, 0.54, 0.24, 0.54) - 0.045;
                    min_dist = min_dist.min(ul);
                }

                // Smooth anti-aliased stroke coverage
                let edge_softness = 0.035_f32;
                let alpha_f = ((edge_softness - min_dist) / (2.0 * edge_softness)).clamp(0.0, 1.0);
                if alpha_f > 0.0 {
                    let a = (alpha_f * 255.0).round() as u8;
                    let idx = (((base_y + py) * ATLAS_WIDTH + (base_x + px)) * 4) as usize;
                    pixels[idx] = 255;
                    pixels[idx + 1] = 255;
                    pixels[idx + 2] = 255;
                    pixels[idx + 3] = a;
                }
            }
        }
    }

    pixels
}

fn segment_sdf(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let pax = px - ax;
    let pay = py - ay;
    let bax = bx - ax;
    let bay = by - ay;
    let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay).max(1e-6)).clamp(0.0, 1.0);
    let dx = pax - bax * h;
    let dy = pay - bay * h;
    (dx * dx + dy * dy).sqrt()
}

/// Evaluates the signed distance to crisp vector strokes for a digit `0..=9` in local `[-1, 1]` space.
fn digit_sdf(digit: u8, x: f32, y: f32) -> f32 {
    let stroke_r = 0.145_f32;
    // Standard 7-segment + diagonal coordinates in [-0.55..0.55, -0.75..0.75]
    let l = -0.50_f32;
    let r = 0.50_f32;
    let t = -0.72_f32;
    let m = 0.00_f32;
    let b = 0.72_f32;

    let segs: &[([f32; 2], [f32; 2])] = match digit {
        0 => &[
            ([l, t], [r, t]),
            ([r, t], [r, b]),
            ([r, b], [l, b]),
            ([l, b], [l, t]),
        ],
        1 => &[
            ([-0.22, -0.42], [0.05, t]),
            ([0.05, t], [0.05, b]),
            ([-0.30, b], [0.38, b]),
        ],
        2 => &[
            ([l, t], [r, t]),
            ([r, t], [r, m]),
            ([r, m], [l, m]),
            ([l, m], [l, b]),
            ([l, b], [r, b]),
        ],
        3 => &[
            ([l, t], [r, t]),
            ([r, t], [r, b]),
            ([l, m], [r, m]),
            ([l, b], [r, b]),
        ],
        4 => &[([l, t], [l, m]), ([l, m], [r, m]), ([r, t], [r, b])],
        5 => &[
            ([r, t], [l, t]),
            ([l, t], [l, m]),
            ([l, m], [r, m]),
            ([r, m], [r, b]),
            ([r, b], [l, b]),
        ],
        6 => &[
            ([r, t], [l, t]),
            ([l, t], [l, b]),
            ([l, b], [r, b]),
            ([r, b], [r, m]),
            ([r, m], [l, m]),
        ],
        7 => &[([l, t], [r, t]), ([r, t], [-0.10, b])],
        8 => &[
            ([l, t], [r, t]),
            ([r, t], [r, b]),
            ([r, b], [l, b]),
            ([l, b], [l, t]),
            ([l, m], [r, m]),
        ],
        9 => &[
            ([l, m], [l, t]),
            ([l, t], [r, t]),
            ([r, t], [r, b]),
            ([r, b], [l, b]),
            ([l, m], [r, m]),
        ],
        _ => &[],
    };

    let mut d = 10.0_f32;
    for &(a, b_pt) in segs {
        d = d.min(segment_sdf(x, y, a[0], a[1], b_pt[0], b_pt[1]));
    }
    d - stroke_r
}

// ============================================================================
// Deterministic 3D Physics Trajectory & Outcome Alignment
// ============================================================================

/// Deterministic SplitMix64 pseudo-random number generator used to synchronize 3D dice physics
/// across all peers from [`WgpuDiceSpec::physics_seed`].
#[derive(Debug, Clone)]
pub struct DeterministicPrng {
    state: u64,
}

impl DeterministicPrng {
    /// Creates a new [`DeterministicPrng`] from a signed 64-bit seed.
    pub fn new(seed: i64) -> Self {
        Self {
            state: (seed as u64) ^ 0x9E37_79B9_7F4A_7C15,
        }
    }

    /// Generates the next pseudo-random `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Generates a uniform `f32` in `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        ((self.next_u64() >> 40) as u32) as f32 / ((1_u32 << 24) as f32)
    }

    /// Generates a uniform `f32` in `[min_val, max_val)`.
    pub fn range_f32(&mut self, min_val: f32, max_val: f32) -> f32 {
        min_val + (max_val - min_val) * self.next_f32()
    }
}

/// Computes a 3x3 column-major rotation matrix that rotates `from_unit` onto `[0.0, 1.0, 0.0]` (`+Y`),
/// followed by an azimuthal yaw rotation `yaw_rad` around `+Y`.
pub fn rotation_aligning_normal_to_up(from_unit: [f32; 3], yaw_rad: f32) -> [[f32; 3]; 3] {
    let f = vec3_normalize(from_unit);
    let to = [0.0_f32, 1.0, 0.0];
    let dot = vec3_dot(f, to).clamp(-1.0, 1.0);

    let align_mat = if dot > 0.9999 {
        [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]
    } else if dot < -0.9999 {
        // 180-degree flip around X axis
        [[1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0]]
    } else {
        let axis = vec3_normalize(vec3_cross(f, to));
        let angle = dot.acos();
        axis_angle_matrix(axis, angle)
    };

    let yaw_mat = axis_angle_matrix([0.0, 1.0, 0.0], yaw_rad);
    mat3_mul(yaw_mat, align_mat)
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
    /// Polyhedral shape of this die.
    pub kind: DieKind,
    /// Authoritative face outcome (`1..=sides`) that lands facing `+Y` when settled.
    pub outcome: i32,
    /// Starting 3D position at `t = 0.0`.
    pub start_pos: [f32; 3],
    /// Final resting 3D position on the tray floor at `t = 1.0`.
    pub target_pos: [f32; 3],
    /// Unit spin axis during the throw tumble.
    pub spin_axis: [f32; 3],
    /// Total radians of tumble spin before settling.
    pub total_spin_rad: f32,
    /// Final resting 3x3 rotation matrix that places `outcome`'s face normal pointing straight up (`+Y`).
    pub resting_rotation: [[f32; 3]; 3],
}

/// Active multi-die physics scene driven by a [`WgpuDiceSpec`].
#[derive(Debug, Clone)]
pub struct DicePhysicsScene {
    /// Deterministic seed used to generate this roll.
    pub physics_seed: i64,
    /// Original dice notation string.
    pub dice_notation: String,
    /// Individual simulated dice in the tray.
    pub dice: Vec<SimulatedDie>,
    /// Normalized animation progress `t` in `[0.0, 1.0]` (`1.0` = settled at final outcomes).
    pub progress: f32,
}

impl DicePhysicsScene {
    /// Constructs a deterministic [`DicePhysicsScene`] from a [`WgpuDiceSpec`].
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
            let col = i % cols;
            let row = i / cols;
            let grid_x = ((col as f32) - ((cols.saturating_sub(1)) as f32) * 0.5) * spacing;
            let grid_z = ((row as f32) - ((rows.saturating_sub(1)) as f32) * 0.5) * spacing;
            let jitter_x = rng.range_f32(-0.18, 0.18);
            let jitter_z = rng.range_f32(-0.18, 0.18);

            let target_pos = [grid_x + jitter_x, 0.0, grid_z + jitter_z];
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
        // Cubic ease-out for horizontal travel
        let ease = 1.0 - (1.0 - u).powi(3);
        let x = die.start_pos[0] + (die.target_pos[0] - die.start_pos[0]) * ease;
        let z = die.start_pos[2] + (die.target_pos[2] - die.start_pos[2]) * ease;

        // Multi-bounce vertical trajectory damping to 0.0 at u = 1.0
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

        // Decaying tumble rotation that converges smoothly to `die.resting_rotation` at u = 1.0
        let remaining_spin = die.total_spin_rad * (1.0 - u).powi(2);
        let tumble = axis_angle_matrix(die.spin_axis, remaining_spin);
        let rot = mat3_mul(tumble, die.resting_rotation);

        ([x, y, z], rot)
    }
}

// ============================================================================
// Uniform Buffer Structures & WGSL Two-Pass Translucent Shader
// ============================================================================

/// Scene-wide uniform buffer uploaded once per frame (`144` bytes, 16-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct SceneUniforms {
    /// 4x4 column-major view-projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Camera eye position in world space (`xyz`, `w = time_sec`).
    pub camera_pos: [f32; 4],
    /// Translucent die body `[r, g, b, opacity]`.
    pub base_color: [f32; 4],
    /// Engraved face numeral ink `[r, g, b, 1.0]`.
    pub ink_color: [f32; 4],
}

/// Per-die instance model matrix uniform (`64` bytes, 16-byte aligned).
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DieInstanceData {
    /// 4x4 column-major model transformation matrix for a single die.
    pub model: [[f32; 4]; 4],
}

/// Perspective look-at view-projection matrix builder for the 3D dice tray camera.
pub fn compute_view_proj_matrix(aspect: f32, die_count: usize) -> ([[f32; 4]; 4], [f32; 3]) {
    let dist_scale = if die_count <= 2 {
        1.0
    } else if die_count <= 6 {
        1.28
    } else {
        1.65
    };
    let eye = [0.0_f32, 6.2 * dist_scale, 4.4 * dist_scale];
    let target = [0.0_f32, 0.0, 0.15];
    let up = [0.0_f32, 1.0, 0.0];

    let f = vec3_normalize(vec3_sub(target, eye));
    let s = vec3_normalize(vec3_cross(f, up));
    let u = vec3_cross(s, f);

    let view = [
        [s[0], u[0], -f[0], 0.0],
        [s[1], u[1], -f[1], 0.0],
        [s[2], u[2], -f[2], 0.0],
        [-vec3_dot(s, eye), -vec3_dot(u, eye), vec3_dot(f, eye), 1.0],
    ];

    let fov_y = 40.0_f32.to_radians();
    let f_cot = 1.0 / (0.5 * fov_y).tan();
    let z_near = 0.2_f32;
    let z_far = 50.0_f32;
    let safe_aspect = aspect.max(0.25);

    // WebGPU [0, 1] depth clip space perspective matrix
    let proj = [
        [f_cot / safe_aspect, 0.0, 0.0, 0.0],
        [0.0, f_cot, 0.0, 0.0],
        [0.0, 0.0, z_far / (z_near - z_far), -1.0],
        [0.0, 0.0, (z_near * z_far) / (z_near - z_far), 0.0],
    ];

    (mat4_mul(proj, view), eye)
}

fn mat4_mul(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0_f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
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

/// WGSL shader source implementing two-pass translucent polyhedral rendering:
/// - `fs_back`: interior back-face pass (`CullMode::Front`) rendering refracted internal facets and edges.
/// - `fs_front`: exterior front-face pass (`CullMode::Back`) rendering Blinn-Phong + Fresnel rim lighting,
///   chamfered facet edges, and high-contrast numeral ink sampled from the procedural atlas.
pub const DICE_WGSL_SHADER: &str = r#"
struct SceneUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    base_color: vec4<f32>,
    ink_color: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> scene: SceneUniforms;

@group(0) @binding(1)
var numeral_tex: texture_2d<f32>;

@group(0) @binding(2)
var numeral_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) edge_factors: vec3<f32>,
    @location(4) model_0: vec4<f32>,
    @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>,
    @location(7) model_3: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) edge_dist: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    let world_p = model * vec4<f32>(in.position, 1.0);
    let rot_m = mat3x3<f32>(in.model_0.xyz, in.model_1.xyz, in.model_2.xyz);
    var out: VertexOutput;
    out.clip_pos = scene.view_proj * world_p;
    out.world_pos = world_p.xyz;
    out.world_normal = normalize(rot_m * in.normal);
    out.uv = in.uv;
    out.edge_dist = in.edge_factors.x;
    return out;
}

@fragment
fn fs_back(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(-in.world_normal);
    let v = normalize(scene.camera_pos.xyz - in.world_pos);
    let l1 = normalize(vec3<f32>(0.45, 0.85, 0.35));
    let ndl = max(dot(n, l1), 0.2);

    // Subtle internal facet edge glow visible through translucent dice
    let edge_highlight = 1.0 - smoothstep(0.0, 0.09, in.edge_dist);
    let ink_mask = textureSample(numeral_tex, numeral_sampler, in.uv).a;

    let body_rgb = scene.base_color.rgb * (0.38 + 0.35 * ndl)
        + scene.base_color.rgb * edge_highlight * 0.28;
    let mixed_rgb = mix(body_rgb, scene.ink_color.rgb * 0.65, ink_mask * 0.45);
    let back_alpha = clamp(scene.base_color.a * 0.52 + edge_highlight * 0.14, 0.10, 0.85);
    return vec4<f32>(mixed_rgb, back_alpha);
}

@fragment
fn fs_front(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(scene.camera_pos.xyz - in.world_pos);
    let l1 = normalize(vec3<f32>(0.42, 0.86, 0.40));
    let l2 = normalize(vec3<f32>(-0.55, 0.45, -0.45));

    let diff1 = max(dot(n, l1), 0.0);
    let diff2 = max(dot(n, l2), 0.0) * 0.35;
    let h = normalize(l1 + v);
    let spec = pow(max(dot(n, h), 0.0), 48.0) * 0.65;
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), 2.6);

    // Chamfered polygon edge bevel highlight
    let edge_bevel = 1.0 - smoothstep(0.0, 0.075, in.edge_dist);
    let ink_mask = textureSample(numeral_tex, numeral_sampler, in.uv).a;

    let lit_body = scene.base_color.rgb * (0.26 + 0.68 * diff1 + diff2)
        + vec3<f32>(1.0, 1.0, 1.0) * spec
        + mix(scene.base_color.rgb, vec3<f32>(1.0, 1.0, 1.0), 0.55) * (fresnel * 0.42 + edge_bevel * 0.26);

    let final_rgb = mix(lit_body, scene.ink_color.rgb, ink_mask);
    let body_alpha = clamp(scene.base_color.a + fresnel * 0.22 + spec * 0.30 + edge_bevel * 0.16, 0.15, 1.0);
    let final_alpha = max(body_alpha, ink_mask * 0.98);
    return vec4<f32>(final_rgb, final_alpha);
}
"#;

// ============================================================================
// Browser WebGPU (`wgpu`) Runtime State
// ============================================================================

mod webgpu_runtime {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::HtmlCanvasElement;

    #[cfg(target_arch = "wasm32")]
    fn create_canvas_surface(
        instance: &wgpu::Instance,
        canvas: HtmlCanvasElement,
    ) -> Result<wgpu::Surface<'static>, String> {
        instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
            .map_err(|e| format!("WebGPU surface error: {e}"))
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn create_canvas_surface(
        _instance: &wgpu::Instance,
        _canvas: HtmlCanvasElement,
    ) -> Result<wgpu::Surface<'static>, String> {
        Err("Browser WebGPU canvas surface requires wasm32 target".to_string())
    }

    struct GpuMeshBuffer {
        vertex_buffer: wgpu::Buffer,
        vertex_count: u32,
    }

    pub struct WebGpuDicePipeline {
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
        depth_view: wgpu::TextureView,
        uniform_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
        instance_buffer: wgpu::Buffer,
        back_pipeline: wgpu::RenderPipeline,
        front_pipeline: wgpu::RenderPipeline,
        meshes: HashMap<DieKind, GpuMeshBuffer>,
    }

    #[derive(Default)]
    pub struct DiceRendererController {
        pub pipeline: Option<WebGpuDicePipeline>,
        pub initializing: bool,
        pub material: DiceMaterialConfig,
        pub scene: Option<DicePhysicsScene>,
        pub anim_start_ms: f64,
        pub anim_loop_active: bool,
    }

    thread_local! {
        static DICE_CONTROLLER: Rc<RefCell<DiceRendererController>> =
            Rc::new(RefCell::new(DiceRendererController::default()));
    }

    pub fn with_dice_controller<F, R>(f: F) -> R
    where
        F: FnOnce(&mut DiceRendererController) -> R,
    {
        DICE_CONTROLLER.with(|cell| f(&mut cell.borrow_mut()))
    }

    fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("satya-dice-depth"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth24Plus,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        tex.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn ensure_webgpu_initialized_and_render() {
        let should_init = with_dice_controller(|ctrl| {
            if ctrl.pipeline.is_some() {
                render_current_frame(ctrl);
                false
            } else if ctrl.initializing {
                false
            } else {
                ctrl.initializing = true;
                true
            }
        });

        if !should_init {
            return;
        }

        spawn_local(async move {
            let init_res = init_webgpu_pipeline().await;
            with_dice_controller(|ctrl| {
                ctrl.initializing = false;
                match init_res {
                    Ok(pipe) => {
                        crate::wasm_client::dom::set_text(
                            "wgpu-status-pill",
                            "WebGPU Active · 2-Pass Translucent",
                        );
                        crate::wasm_client::dom::set_class("wgpu-status-pill", "pill pill-active");
                        ctrl.pipeline = Some(pipe);
                        if ctrl.scene.is_none() {
                            ctrl.scene = Some(DicePhysicsScene::from_spec(&WgpuDiceSpec {
                                dice_notation: "2d20".to_string(),
                                physics_seed: 42,
                                outcomes: vec![1, 20],
                                visibility: satya_schema::MessageVisibility::Public,
                            }));
                            ctrl.anim_start_ms = js_sys::Date::now();
                        }
                        render_current_frame(ctrl);
                        start_animation_loop_if_needed(ctrl);
                    }
                    Err(err_msg) => {
                        crate::wasm_client::dom::set_text("wgpu-status-pill", &err_msg);
                        crate::wasm_client::dom::set_class("wgpu-status-pill", "pill pill-muted");
                    }
                }
            });
        });
    }

    async fn init_webgpu_pipeline() -> Result<WebGpuDicePipeline, String> {
        let Some(el) = crate::wasm_client::dom::get_element("wgpu-dice-canvas") else {
            return Err("Canvas #wgpu-dice-canvas not found".to_string());
        };
        let canvas: HtmlCanvasElement = el
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Element is not an HtmlCanvasElement".to_string())?;

        let width = canvas.client_width().max(320) as u32;
        let height = canvas.client_height().max(260) as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface = create_canvas_surface(&instance, canvas)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| "WebGPU adapter unavailable in this browser".to_string())?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("satya-dice-device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_webgl2_defaults()
                        .using_resolution(adapter.limits()),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| format!("WebGPU device error: {e}"))?;

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .copied()
            .find(|f| !f.is_srgb())
            .or_else(|| caps.formats.first().copied())
            .ok_or_else(|| "No supported WebGPU surface format".to_string())?;

        let alpha_mode = if caps
            .alpha_modes
            .contains(&wgpu::CompositeAlphaMode::PreMultiplied)
        {
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            caps.alpha_modes
                .first()
                .copied()
                .unwrap_or(wgpu::CompositeAlphaMode::Auto)
        };

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let depth_view = create_depth_view(&device, width, height);

        // Upload procedural 5x4 numeral texture atlas (faces 1..=20)
        let atlas_rgba = generate_dice_numeral_atlas_rgba8();
        let atlas_size = wgpu::Extent3d {
            width: ATLAS_WIDTH,
            height: ATLAS_HEIGHT,
            depth_or_array_layers: 1,
        };
        let atlas_tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("satya-dice-numeral-atlas"),
            size: atlas_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &atlas_tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &atlas_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * ATLAS_WIDTH),
                rows_per_image: Some(ATLAS_HEIGHT),
            },
            atlas_size,
        );
        let atlas_view = atlas_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("satya-dice-sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-dice-scene-uniforms"),
            size: std::mem::size_of::<SceneUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-dice-instances"),
            size: (std::mem::size_of::<DieInstanceData>() * 24) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("satya-dice-bgl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("satya-dice-bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&atlas_sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("satya-dice-wgsl"),
            source: wgpu::ShaderSource::Wgsl(DICE_WGSL_SHADER.into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("satya-dice-pipeline-layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let vertex_buffers = [
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<DieVertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 0,
                        shader_location: 0,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 12,
                        shader_location: 1,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x2,
                        offset: 24,
                        shader_location: 2,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x3,
                        offset: 32,
                        shader_location: 3,
                    },
                ],
            },
            wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<DieInstanceData>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &[
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 0,
                        shader_location: 4,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 16,
                        shader_location: 5,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 32,
                        shader_location: 6,
                    },
                    wgpu::VertexAttribute {
                        format: wgpu::VertexFormat::Float32x4,
                        offset: 48,
                        shader_location: 7,
                    },
                ],
            },
        ];

        // Pass 1: Back-facing interior facets (`CullMode::Front`, depth write off, alpha blend)
        let back_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("satya-dice-back-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_back"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Front),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Pass 2: Front-facing exterior facets + numerals (`CullMode::Back`, depth write on, alpha blend)
        let front_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("satya-dice-front-pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_front"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                cull_mode: Some(wgpu::Face::Back),
                front_face: wgpu::FrontFace::Ccw,
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth24Plus,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Upload all 6 polyhedral meshes (`d4`, `d6`, `d8`, `d10`, `d12`, `d20`)
        let mut meshes = HashMap::new();
        for kind in [
            DieKind::D4,
            DieKind::D6,
            DieKind::D8,
            DieKind::D10,
            DieKind::D12,
            DieKind::D20,
        ] {
            let cpu_mesh = build_polyhedral_mesh(kind);
            let bytes = bytemuck::cast_slice(&cpu_mesh.vertices);
            let vb = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(kind.label()),
                size: bytes.len() as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&vb, 0, bytes);
            meshes.insert(
                kind,
                GpuMeshBuffer {
                    vertex_buffer: vb,
                    vertex_count: cpu_mesh.vertices.len() as u32,
                },
            );
        }

        Ok(WebGpuDicePipeline {
            device,
            queue,
            surface,
            surface_config,
            depth_view,
            uniform_buffer,
            bind_group,
            instance_buffer,
            back_pipeline,
            front_pipeline,
            meshes,
        })
    }

    pub fn render_current_frame(ctrl: &mut DiceRendererController) {
        let Some(pipe) = ctrl.pipeline.as_mut() else {
            return;
        };
        let Some(scene) = ctrl.scene.as_ref() else {
            return;
        };

        // Resize surface if canvas dimensions changed
        if let Some(el) = crate::wasm_client::dom::get_element("wgpu-dice-canvas")
            && let Ok(canvas) = el.dyn_into::<HtmlCanvasElement>()
        {
            let cw = (canvas.client_width().max(320)) as u32;
            let ch = (canvas.client_height().max(260)) as u32;
            if cw != pipe.surface_config.width || ch != pipe.surface_config.height {
                canvas.set_width(cw);
                canvas.set_height(ch);
                pipe.surface_config.width = cw;
                pipe.surface_config.height = ch;
                pipe.surface.configure(&pipe.device, &pipe.surface_config);
                pipe.depth_view = create_depth_view(&pipe.device, cw, ch);
            }
        }

        let aspect =
            (pipe.surface_config.width as f32) / (pipe.surface_config.height.max(1) as f32);
        let die_count = scene.dice.len().max(1);
        let (view_proj, eye) = compute_view_proj_matrix(aspect, die_count);

        let uniforms = SceneUniforms {
            view_proj,
            camera_pos: [eye[0], eye[1], eye[2], scene.progress],
            base_color: [
                ctrl.material.base_rgb[0],
                ctrl.material.base_rgb[1],
                ctrl.material.base_rgb[2],
                ctrl.material.opacity,
            ],
            ink_color: [
                ctrl.material.ink_rgb[0],
                ctrl.material.ink_rgb[1],
                ctrl.material.ink_rgb[2],
                1.0,
            ],
        };
        pipe.queue
            .write_buffer(&pipe.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let active_kind = scene.dice.first().map(|d| d.kind).unwrap_or(DieKind::D20);
        let Some(gpu_mesh) = pipe.meshes.get(&active_kind) else {
            return;
        };

        let mut instances = Vec::with_capacity(scene.dice.len());
        for i in 0..scene.dice.len() {
            let (pos, rot) = scene.sample_die_transform(i, scene.progress);
            instances.push(DieInstanceData {
                model: pack_model_matrix(pos, rot),
            });
        }
        pipe.queue
            .write_buffer(&pipe.instance_buffer, 0, bytemuck::cast_slice(&instances));

        let Ok(frame) = pipe.surface.get_current_texture() else {
            return;
        };
        let color_view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = pipe
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("satya-dice-encoder"),
            });

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("satya-dice-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.035,
                            g: 0.051,
                            b: 0.086,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &pipe.depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_bind_group(0, &pipe.bind_group, &[]);
            rpass.set_vertex_buffer(0, gpu_mesh.vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, pipe.instance_buffer.slice(..));

            let inst_count = instances.len() as u32;
            // Pass 1: Back faces (translucent interior depth & facets)
            if ctrl.material.opacity < 0.99 {
                rpass.set_pipeline(&pipe.back_pipeline);
                rpass.draw(0..gpu_mesh.vertex_count, 0..inst_count);
            }
            // Pass 2: Front faces (lit exterior facets, specular, and numeral ink)
            rpass.set_pipeline(&pipe.front_pipeline);
            rpass.draw(0..gpu_mesh.vertex_count, 0..inst_count);
        }

        pipe.queue.submit(std::iter::once(encoder.finish()));
        frame.present();
    }

    pub fn start_animation_loop_if_needed(ctrl: &mut DiceRendererController) {
        if ctrl.anim_loop_active {
            return;
        }
        ctrl.anim_loop_active = true;
        schedule_next_animation_frame();
    }

    fn schedule_next_animation_frame() {
        let Some(win) = crate::wasm_client::dom::window() else {
            return;
        };
        let cb = Closure::once_into_js(move || {
            let keep_going = with_dice_controller(|ctrl| {
                let now = js_sys::Date::now();
                let elapsed_ms = (now - ctrl.anim_start_ms).max(0.0);
                let duration_ms = 1250.0;
                let t = (elapsed_ms / duration_ms).clamp(0.0, 1.0) as f32;
                if let Some(sc) = ctrl.scene.as_mut() {
                    sc.progress = t;
                }
                render_current_frame(ctrl);
                if t < 1.0 {
                    true
                } else {
                    ctrl.anim_loop_active = false;
                    false
                }
            });
            if keep_going {
                schedule_next_animation_frame();
            }
        });
        let _ = win.request_animation_frame(cb.unchecked_ref());
    }
}

/// Spawns a new 3D physics roll in the WebGPU dice canvas and starts the throw animation.
pub fn spawn_wgpu_dice_scene(spec: &WgpuDiceSpec) {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    webgpu_runtime::with_dice_controller(|ctrl| {
        ctrl.scene = Some(DicePhysicsScene::from_spec(spec));
        ctrl.anim_start_ms = js_sys::Date::now();
        if ctrl.pipeline.is_some() {
            webgpu_runtime::render_current_frame(ctrl);
            webgpu_runtime::start_animation_loop_if_needed(ctrl);
        }
    });
    webgpu_runtime::ensure_webgpu_initialized_and_render();
}

/// Updates the active [`DiceMaterialConfig`] (base color, ink color, and opacity/translucency)
/// from the DOM controls and immediately re-renders the 3D WebGPU dice canvas.
pub fn sync_dice_material_from_dom_and_render() {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    let base_hex = crate::wasm_client::dom::input_value("dice-base-color");
    let ink_hex = crate::wasm_client::dom::input_value("dice-ink-color");
    let opacity_raw = crate::wasm_client::dom::input_value("dice-opacity-slider");
    let opacity_pct = opacity_raw
        .parse::<f32>()
        .unwrap_or(68.0)
        .clamp(15.0, 100.0);
    let opacity = opacity_pct / 100.0;

    // Sync custom property `--control-value` for WebKit/Blink range slider fill track per modern-web-guidance
    if let Some(slider_el) = crate::wasm_client::dom::get_element("dice-opacity-slider")
        && let Ok(html_el) = slider_el.clone().dyn_into::<web_sys::HtmlElement>()
    {
        let _ = html_el.style().set_property("--attr-min", "15");
        let _ = html_el.style().set_property("--attr-max", "100");
        let _ = html_el
            .style()
            .set_property("--control-value", &format!("{opacity_pct:.0}"));
        let _ = slider_el.set_attribute("aria-valuetext", &format!("{opacity_pct:.0}% opacity"));
    }
    crate::wasm_client::dom::set_text("dice-opacity-readout", &format!("{opacity_pct:.0}%"));

    let mat = DiceMaterialConfig::from_hex_and_opacity(
        if base_hex.is_empty() {
            "#0ea5e9"
        } else {
            &base_hex
        },
        if ink_hex.is_empty() {
            "#f8fafc"
        } else {
            &ink_hex
        },
        opacity,
    );

    webgpu_runtime::with_dice_controller(|ctrl| {
        ctrl.material = mat;
        webgpu_runtime::render_current_frame(ctrl);
    });
    webgpu_runtime::ensure_webgpu_initialized_and_render();
}

/// Applies a material preset selected in `#dice-material-preset` to the color inputs and opacity slider,
/// then re-renders the WebGPU dice canvas.
pub fn apply_dice_material_preset_from_dom() {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    let preset = crate::wasm_client::dom::select_value("dice-material-preset");
    let mat = DiceMaterialConfig::from_preset(&preset);
    let pct = (mat.opacity * 100.0).round() as i32;
    crate::wasm_client::dom::set_input_value("dice-base-color", &mat.base_color_hex);
    crate::wasm_client::dom::set_input_value("dice-ink-color", &mat.ink_color_hex);
    crate::wasm_client::dom::set_input_value("dice-opacity-slider", &pct.to_string());
    sync_dice_material_from_dom_and_render();
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
    fn test_numeral_atlas_generates_non_empty_strokes_for_all_20_cells() {
        let rgba = generate_dice_numeral_atlas_rgba8();
        assert_eq!(rgba.len(), (ATLAS_WIDTH * ATLAS_HEIGHT * 4) as usize);

        for face_num in 1..=20_u32 {
            let idx = face_num - 1;
            let col = idx % ATLAS_COLS;
            let row = idx / ATLAS_COLS;
            let mut non_zero_alpha = 0_usize;
            for py in 0..ATLAS_CELL_SIZE {
                for px in 0..ATLAS_CELL_SIZE {
                    let x = col * ATLAS_CELL_SIZE + px;
                    let y = row * ATLAS_CELL_SIZE + py;
                    let a = rgba[((y * ATLAS_WIDTH + x) * 4 + 3) as usize];
                    if a > 128 {
                        non_zero_alpha += 1;
                    }
                }
            }
            assert!(
                non_zero_alpha > 40,
                "Atlas cell for face {face_num} must contain rendered numeral strokes (got {non_zero_alpha})"
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
            assert_eq!(
                pos_a, pos_b,
                "Same physics_seed must produce identical resting positions"
            );
            assert_eq!(
                rot_a, rot_b,
                "Same physics_seed must produce identical resting rotations"
            );
            assert!(
                pos_a[1].abs() < 1e-5,
                "Resting die must sit on the tray floor (y = 0)"
            );

            // Verify the outcome face normal points straight up (+Y = [0, 1, 0]) at t = 1.0
            let outcome = spec.outcomes[i] as usize;
            let local_n = mesh.face_normals[outcome - 1];
            let world_n = mat3_mul_vec3(rot_a, local_n);
            assert!(
                (world_n[1] - 1.0).abs() < 1e-4,
                "Outcome {outcome} face normal must point straight up (+Y) at t=1.0, got {world_n:?}"
            );
        }
    }

    #[test]
    fn test_material_config_and_hex_color_parsing() {
        let mat = DiceMaterialConfig::from_hex_and_opacity("#38BDF8", "#FDE047", 0.65);
        assert_eq!(mat.base_color_hex, "#38bdf8");
        assert_eq!(mat.ink_color_hex, "#fde047");
        assert!((mat.opacity - 0.65).abs() < 1e-5);

        let amber = DiceMaterialConfig::from_preset("amber_resin");
        assert_eq!(amber.base_color_hex, "#f59e0b");
        assert!(amber.opacity < 0.9);
    }
}
