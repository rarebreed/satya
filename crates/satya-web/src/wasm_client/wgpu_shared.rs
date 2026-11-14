//! Shared 3D Linear Algebra, Orbit/Pan/Zoom + Orthographic Camera, Unified `.glb` Diorama WGSL
//! Shader, and Browser WebGPU (`wgpu`) Canvas/Depth/Pipeline Bootstrap Helpers.
//!
//! Shared across [`crate::wasm_client::model_3d_wgpu`], [`crate::wasm_client::map_3d_wgpu`],
//! and [`crate::wasm_client::dice_3d_wgpu`].

#[cfg(target_arch = "wasm32")]
use crate::wasm_client::glb_model::GlbVertex;
use bytemuck::{Pod, Zeroable};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

/// Unified WGSL shader used by both `#wgpu-model-canvas` (Tab 3) and `#wgpu-map-canvas` (Tab 1).
///
/// `scene.mode_params.x`:
/// - `0.0`: Clean PBR + Rim Lighting
/// - `1.0`: Wireframe + PBR (`fwidth` anti-aliased barycentric edges)
/// - `2.0`: Anatomical Hit Zones (`Head`, `Torso`, `Arms/Weapon`, `Legs`, `Pack`)
/// - `3.0`: Thermal / FLIR Sensor
pub const SHARED_GLB_DIORAMA_WGSL: &str = r#"
struct DioramaUniforms {
    view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    mode_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> scene: DioramaUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) bary_zone: vec4<f32>,
    @location(4) model_0: vec4<f32>,
    @location(5) model_1: vec4<f32>,
    @location(6) model_2: vec4<f32>,
    @location(7) model_3: vec4<f32>,
    @location(8) inst_tint: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) world_pos: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) base_rgb: vec3<f32>,
    @location(3) alpha: f32,
    @location(4) bary: vec3<f32>,
    @location(5) zone_id: f32,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    let world_p = model * vec4<f32>(in.position, 1.0);
    let rot_m = mat3x3<f32>(in.model_0.xyz, in.model_1.xyz, in.model_2.xyz);

    // Blend tintable armor submeshes (`in.color.a > 0.5`) toward instance primary color
    let tinted = mix(in.color.rgb, in.inst_tint.rgb, in.color.a);

    var out: VertexOutput;
    out.clip_pos = scene.view_proj * world_p;
    out.world_pos = world_p.xyz;
    out.world_normal = normalize(rot_m * in.normal);
    out.base_rgb = tinted;
    out.alpha = in.inst_tint.a;
    out.bary = in.bary_zone.xyz;
    out.zone_id = in.bary_zone.w;
    return out;
}

fn zone_palette(zone_id: f32, fallback: vec3<f32>) -> vec3<f32> {
    let z = i32(round(zone_id));
    if z == 1 {
        return vec3<f32>(0.20, 0.92, 0.62); // Head / Optics (DR 3-5)
    }
    if z == 2 {
        return vec3<f32>(0.22, 0.74, 0.98); // Torso Carapace (DR 4-7)
    }
    if z == 3 {
        return vec3<f32>(0.98, 0.72, 0.18); // Arms & Weapon
    }
    if z == 4 {
        return vec3<f32>(0.52, 0.58, 0.98); // Legs & Mag-Boots
    }
    if z == 5 {
        return vec3<f32>(0.88, 0.42, 0.96); // Backpack / Thruster Rig
    }
    return fallback;
}

fn hash21(p: vec2<f32>) -> f32 {
    let h = dot(p, vec2<f32>(127.1, 311.7));
    return fract(sin(h) * 43758.5453123);
}

fn vnoise2(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var n = normalize(in.world_normal);
    let v = normalize(scene.camera_pos.xyz - in.world_pos);
    let fw = fwidth(in.bary);
    let mode = i32(round(scene.mode_params.x));
    var albedo = in.base_rgb;
    var spec_scale = 1.0;

    if mode == 2 {
        albedo = zone_palette(in.zone_id, in.base_rgb * 0.65);
    } else {
        let wp = in.world_pos.xz;
        if abs(in.zone_id - (-2.0)) < 0.1 {
            let g_patch = vnoise2(wp * 2.2);
            let g_fine = vnoise2(wp * 11.5);
            let g_blade = hash21(floor(wp * 26.0));
            albedo = mix(in.base_rgb * 0.78, in.base_rgb * 1.24, g_patch)
                + vec3<f32>(0.02, 0.05, 0.01) * (g_fine - 0.5)
                + vec3<f32>(0.03, 0.06, 0.02) * (g_blade - 0.5);
            n = normalize(n + vec3<f32>((g_fine - 0.5) * 0.14, 0.0, (g_patch - 0.5) * 0.14));
            spec_scale = 0.28;
        } else if abs(in.zone_id - (-3.0)) < 0.1 {
            let pebbles = vnoise2(wp * 18.0);
            let grit = hash21(floor(wp * 30.0));
            albedo = albedo * (0.78 + 0.36 * pebbles + 0.14 * (grit - 0.5));
            n = normalize(n + vec3<f32>((grit - 0.5) * 0.18, 0.0, (pebbles - 0.5) * 0.18));
            spec_scale = 0.45;
        } else if abs(in.zone_id - (-4.0)) < 0.1 {
            let loam = vnoise2(wp * 4.5);
            let sand = vnoise2(wp * 16.0);
            albedo = albedo * (0.80 + 0.30 * loam) + vec3<f32>(0.03, 0.02, 0.01) * (sand - 0.5);
            spec_scale = 0.22;
        } else if abs(in.zone_id - (-5.0)) < 0.1 {
            let bark = vnoise2(vec2<f32>(wp.x * 20.0, wp.y * 9.0));
            let chip = hash21(floor(vec2<f32>(wp.x * 16.0, wp.y * 28.0)));
            albedo = albedo * (0.72 + 0.45 * bark + 0.16 * (chip - 0.5));
            n = normalize(n + vec3<f32>((chip - 0.5) * 0.16, 0.0, (bark - 0.5) * 0.16));
            spec_scale = 0.25;
        } else if abs(in.zone_id - (-6.0)) < 0.1 {
            let rip = sin(wp.x * 9.0 + wp.y * 3.5 + vnoise2(wp * 4.0) * 4.5);
            albedo = mix(albedo * 0.86, albedo * 1.28, rip * 0.5 + 0.5);
            n = normalize(n + vec3<f32>(rip * 0.08, 0.0, cos(wp.y * 8.0) * 0.06));
            spec_scale = 2.2;
        } else if abs(in.zone_id - (-0.25)) < 0.1 {
            let l_clump = vnoise2(wp * 7.5 + in.world_pos.y * 3.1);
            let l_leaf = vnoise2(wp * 19.0 - in.world_pos.y * 5.0);
            albedo = albedo * (0.72 + 0.48 * l_clump) + vec3<f32>(0.04, 0.07, 0.01) * l_leaf;
            n = normalize(n + vec3<f32>((l_leaf - 0.5) * 0.24, 0.0, (l_clump - 0.5) * 0.24));
            spec_scale = 0.35;
        } else if abs(in.zone_id - (-7.0)) < 0.1 {
            let grain = hash21(floor(wp * 26.0));
            let wear = vnoise2(wp * 2.8);
            albedo = albedo * (0.88 + 0.14 * wear + 0.12 * (grain - 0.5));
            n = normalize(n + vec3<f32>((grain - 0.5) * 0.10, 0.0, (wear - 0.5) * 0.10));
            spec_scale = 0.30;
        }
    }

    let key_l = normalize(vec3<f32>(0.48, 0.84, 0.36));
    let fill_l = normalize(vec3<f32>(-0.62, 0.42, -0.52));

    let ndl = max(dot(n, key_l), 0.0);
    let fill = max(dot(n, fill_l), 0.0) * 0.32;
    let hemi = mix(vec3<f32>(0.07, 0.10, 0.16), vec3<f32>(0.22, 0.28, 0.38), n.y * 0.5 + 0.5);
    let h = normalize(key_l + v);
    let spec = pow(max(dot(n, h), 0.0), 38.0) * 0.42 * spec_scale;
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), 3.0) * 0.28 * spec_scale;

    // Subtle 1.0m minor / 5.0m major tactical grid lines on deck tiles (`zone_id < -0.5`)
    var grid_boost = vec3<f32>(0.0);
    if in.zone_id < -0.5 && scene.mode_params.w > 0.5 {
        let cell_scale = max(scene.mode_params.y, 0.25);
        let uv1 = fract(in.world_pos.xz / cell_scale + vec2<f32>(0.5, 0.5));
        let d1 = min(min(uv1.x, 1.0 - uv1.x), min(uv1.y, 1.0 - uv1.y));
        let line1 = 1.0 - smoothstep(0.0, 0.038, d1);
        let uv5 = fract((in.world_pos.xz / cell_scale + vec2<f32>(0.5, 0.5)) * 0.2);
        let d5 = min(min(uv5.x, 1.0 - uv5.x), min(uv5.y, 1.0 - uv5.y));
        let line5 = 1.0 - smoothstep(0.0, 0.016, d5);
        let minor_c = select(vec3<f32>(0.09, 0.22, 0.36), vec3<f32>(0.04, 0.06, 0.04), in.zone_id < -1.5);
        let major_c = select(vec3<f32>(0.14, 0.38, 0.58), vec3<f32>(0.07, 0.11, 0.06), in.zone_id < -1.5);
        grid_boost = minor_c * line1 + major_c * line5;
    }

    var rgb = hemi * albedo + albedo * (0.76 * ndl + fill) + vec3<f32>(0.85, 0.93, 1.0) * (spec + fresnel * 0.5) + grid_boost;

    if mode == 3 {
        // Thermal / IR false-color sensor view
        let heat = clamp(0.25 + 0.45 * max(dot(n, v), 0.0) + 0.28 * clamp(in.world_pos.y * 0.55, 0.0, 1.0), 0.0, 1.0);
        let cold_c = vec3<f32>(0.08, 0.12, 0.48);
        let mid_c = vec3<f32>(0.88, 0.20, 0.38);
        let hot_c = vec3<f32>(1.0, 0.88, 0.22);
        rgb = select(mix(mid_c, hot_c, (heat - 0.5) * 2.0), mix(cold_c, mid_c, heat * 2.0), heat < 0.5);
    }

    if mode == 1 || mode == 2 {
        let edge_d = min(min(in.bary.x / max(fw.x, 1e-4), in.bary.y / max(fw.y, 1e-4)), in.bary.z / max(fw.z, 1e-4));
        let wire = 1.0 - smoothstep(0.35, 1.25, edge_d);
        let wire_col = select(vec3<f32>(0.22, 0.78, 0.98), vec3<f32>(0.98, 0.32, 0.46), mode == 2);
        rgb = mix(rgb, wire_col, wire * scene.mode_params.z);
    }

    return vec4<f32>(rgb, in.alpha);
}
"#;

/// Uniform buffer (`96` bytes) bound at `@group(0) @binding(0)` for [`SHARED_GLB_DIORAMA_WGSL`].
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct DioramaUniforms {
    pub view_proj: [[f32; 4]; 4],
    pub camera_pos: [f32; 4],
    /// `[shader_mode, cell_size_m, wire_strength, reserved]`
    pub mode_params: [f32; 4],
}

/// Per-instance transform + tint (`80` bytes) bound as vertex buffer slot `1`.
#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub struct GlbInstanceData {
    pub model: [[f32; 4]; 4],
    pub tint_rgba: [f32; 4],
}

impl GlbInstanceData {
    #[must_use]
    pub fn from_trs_and_tint(
        translation: [f32; 3],
        yaw_deg: f32,
        scale: [f32; 3],
        tint_rgba: [f32; 4],
    ) -> Self {
        let (s, c) = yaw_deg.to_radians().sin_cos();
        Self {
            model: [
                [c * scale[0], 0.0, -s * scale[0], 0.0],
                [0.0, scale[1], 0.0, 0.0],
                [s * scale[2], 0.0, c * scale[2], 0.0],
                [translation[0], translation[1], translation[2], 1.0],
            ],
            tint_rgba,
        }
    }
}

/// Shared Orbit / Pan / Zoom + Top-Down Orthographic camera used by both the `3D Model` turntable
/// and the `Maps` tactical diorama.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitOrthoCamera {
    pub yaw_deg: f32,
    pub pitch_deg: f32,
    pub zoom: f32,
    pub pan: [f32; 2],
    pub target_y: f32,
    pub base_distance: f32,
    pub orthographic: bool,
}

/// World-space `(eye, target, right_vec, true_up_vec, forward_vec)` basis returned by [`OrbitOrthoCamera::basis_vectors`].
pub type CameraBasisVectors = ([f32; 3], [f32; 3], [f32; 3], [f32; 3], [f32; 3]);

impl OrbitOrthoCamera {
    #[must_use]
    pub const fn turntable_default() -> Self {
        Self {
            yaw_deg: 26.0,
            pitch_deg: 22.0,
            zoom: 1.0,
            pan: [0.0, 0.0],
            target_y: 0.92,
            base_distance: 3.35,
            orthographic: false,
        }
    }

    #[must_use]
    pub const fn tactical_map_default() -> Self {
        Self {
            yaw_deg: -24.0,
            pitch_deg: 52.0,
            zoom: 1.0,
            pan: [0.0, 0.0],
            target_y: 0.0,
            base_distance: 19.5,
            orthographic: false,
        }
    }

    pub fn orbit_by(&mut self, dyaw_deg: f32, dpitch_deg: f32) {
        self.yaw_deg += dyaw_deg;
        while self.yaw_deg > 180.0 {
            self.yaw_deg -= 360.0;
        }
        while self.yaw_deg < -180.0 {
            self.yaw_deg += 360.0;
        }
        self.pitch_deg = (self.pitch_deg + dpitch_deg).clamp(8.0, 88.5);
    }

    pub fn pan_by_screen_delta(&mut self, right_delta: f32, forward_delta: f32, limit: f32) {
        let (sy, cy) = self.yaw_deg.to_radians().sin_cos();
        let (rx, rz) = (cy, -sy);
        let (fx, fz) = (-sy, -cy);
        let scale = 1.0 / self.zoom.max(0.15);
        self.pan[0] =
            (self.pan[0] + (rx * right_delta + fx * forward_delta) * scale).clamp(-limit, limit);
        self.pan[1] =
            (self.pan[1] + (rz * right_delta + fz * forward_delta) * scale).clamp(-limit, limit);
    }

    pub fn zoom_by_factor(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.18, 4.0);
    }

    /// Computes `(eye, target, right_vec, true_up_vec, forward_vec)` in world space.
    #[must_use]
    pub fn basis_vectors(&self) -> CameraBasisVectors {
        let dist = self.base_distance / self.zoom.max(0.15);
        let pitch_eff = if self.orthographic {
            88.5_f32.to_radians()
        } else {
            self.pitch_deg.clamp(8.0, 88.5).to_radians()
        };
        let yaw_rad = self.yaw_deg.to_radians();
        let horiz = dist * pitch_eff.cos();
        let target = [self.pan[0], self.target_y, self.pan[1]];
        let eye = [
            target[0] + horiz * yaw_rad.sin(),
            target[1] + dist * pitch_eff.sin(),
            target[2] + horiz * yaw_rad.cos(),
        ];
        let fwd = normalize3([
            target[0] - eye[0],
            target[1] - eye[1],
            target[2] - eye[2],
        ]);
        let right = normalize3(cross3(fwd, [0.0, 1.0, 0.0]));
        let up = normalize3(cross3(right, fwd));
        (eye, target, right, up, fwd)
    }

    /// Builds column-major `4x4` `view_proj` (`wgpu` depth `0..1`) and `eye` world position.
    #[must_use]
    pub fn build_view_proj_and_eye(&self, aspect: f32) -> ([[f32; 4]; 4], [f32; 3]) {
        let asp = aspect.max(0.2);
        let (eye, _target, right, up, fwd) = self.basis_vectors();
        let view = [
            [right[0], up[0], -fwd[0], 0.0],
            [right[1], up[1], -fwd[1], 0.0],
            [right[2], up[2], -fwd[2], 0.0],
            [-dot3(right, eye), -dot3(up, eye), dot3(fwd, eye), 1.0],
        ];

        let proj = if self.orthographic {
            let half_h = (self.base_distance * 0.46) / self.zoom.max(0.15);
            let half_w = half_h * asp;
            let (near, far) = (0.1_f32, 240.0_f32);
            [
                [1.0 / half_w, 0.0, 0.0, 0.0],
                [0.0, 1.0 / half_h, 0.0, 0.0],
                [0.0, 0.0, -1.0 / (far - near), 0.0],
                [0.0, 0.0, -near / (far - near), 1.0],
            ]
        } else {
            let f = 1.0 / (0.5 * 42.0_f32.to_radians()).tan();
            let (near, far) = (0.1_f32, 240.0_f32);
            [
                [f / asp, 0.0, 0.0, 0.0],
                [0.0, f, 0.0, 0.0],
                [0.0, 0.0, far / (near - far), -1.0],
                [0.0, 0.0, (near * far) / (near - far), 0.0],
            ]
        };

        (mul_mat4_cols(proj, view), eye)
    }

    /// Intersects a Normalized Device Coordinate (`ndc_x, ndc_y` in `-1.0..=1.0`, `+Y` up) ray
    /// with the horizontal plane `y = plane_y`, returning world `[x, z]`.
    #[must_use]
    pub fn screen_ndc_to_plane_xz(
        &self,
        ndc_x: f32,
        ndc_y: f32,
        aspect: f32,
        plane_y: f32,
    ) -> Option<[f32; 2]> {
        let asp = aspect.max(0.2);
        let (eye, _target, right, up, fwd) = self.basis_vectors();
        let (ray_origin, ray_dir) = if self.orthographic {
            let half_h = (self.base_distance * 0.46) / self.zoom.max(0.15);
            let half_w = half_h * asp;
            let ox = eye[0] + right[0] * (ndc_x * half_w) + up[0] * (ndc_y * half_h);
            let oy = eye[1] + right[1] * (ndc_x * half_w) + up[1] * (ndc_y * half_h);
            let oz = eye[2] + right[2] * (ndc_x * half_w) + up[2] * (ndc_y * half_h);
            ([ox, oy, oz], fwd)
        } else {
            let tan_half = (0.5 * 42.0_f32.to_radians()).tan();
            let dx = fwd[0] + right[0] * (ndc_x * asp * tan_half) + up[0] * (ndc_y * tan_half);
            let dy = fwd[1] + right[1] * (ndc_x * asp * tan_half) + up[1] * (ndc_y * tan_half);
            let dz = fwd[2] + right[2] * (ndc_x * asp * tan_half) + up[2] * (ndc_y * tan_half);
            (eye, normalize3([dx, dy, dz]))
        };

        if ray_dir[1].abs() < 1e-4 {
            return None;
        }
        let t = (plane_y - ray_origin[1]) / ray_dir[1];
        if t <= 0.0 {
            return None;
        }
        Some([
            ray_origin[0] + ray_dir[0] * t,
            ray_origin[2] + ray_dir[2] * t,
        ])
    }
}

/// 3D dot product `a . b`.
#[inline]
#[must_use]
pub fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// 3D right-handed cross product `a x b`.
#[inline]
#[must_use]
pub fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// Normalizes a 3D vector to unit length, falling back to `+Y` (`[0.0, 1.0, 0.0]`) near zero.
#[inline]
#[must_use]
pub fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = dot3(v, v).sqrt();
    if l > 1e-6 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

/// Multiplies two column-major `4x4` matrices `a * b`.
#[inline]
#[must_use]
pub fn mul_mat4_cols(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0_f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

/// Maps UI shader mode slugs (`"pbr"`, `"wireframe_pbr"`, `"hit_zones"`, `"thermal"`) to `mode_params.x`.
#[must_use]
pub fn shader_mode_code(mode_str: &str) -> f32 {
    match mode_str.trim() {
        "pbr" => 0.0,
        "hit_zones" => 2.0,
        "thermal" => 3.0,
        _ => 1.0, // "wireframe_pbr"
    }
}

// ============================================================================
// Browser WebGPU (`wgpu`) Canvas, Depth, Resize & Pipeline Helpers (`wasm32`)
// ============================================================================

/// Initialized WebGPU `(canvas, surface, device, queue, surface_config)` tuple for a DOM `<canvas>`.
#[cfg(target_arch = "wasm32")]
pub type WgpuCanvasContext = (
    web_sys::HtmlCanvasElement,
    wgpu::Surface<'static>,
    wgpu::Device,
    wgpu::Queue,
    wgpu::SurfaceConfiguration,
);

/// Initializes a browser WebGPU surface, adapter, device, queue, and surface configuration for `#canvas_id`.
#[cfg(target_arch = "wasm32")]
pub async fn init_wgpu_canvas_context(canvas_id: &str) -> Result<WgpuCanvasContext, String> {
    let Some(el) = crate::wasm_client::dom::get_element(canvas_id) else {
        return Err(format!("Canvas #{canvas_id} not found"));
    };
    let canvas: web_sys::HtmlCanvasElement = el
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| format!("Element #{canvas_id} is not an HtmlCanvasElement"))?;

    let width = canvas.client_width().max(320) as u32;
    let height = canvas.client_height().max(260) as u32;
    canvas.set_width(width);
    canvas.set_height(height);

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU,
        ..Default::default()
    });
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| format!("WebGPU surface error: {e}"))?;
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .ok_or_else(|| "WebGPU unavailable (showing SVG fallback)".to_string())?;
    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: Some(canvas_id),
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
    Ok((canvas, surface, device, queue, surface_config))
}

/// Creates a `Depth24Plus` attachment view sized to `width.max(1) x height.max(1)`.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn create_depth_view(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    label: &str,
) -> wgpu::TextureView {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
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

/// Checks `#canvas_id`'s CSS client dimensions and reconfigures `surface` + `depth_view` if resized,
/// returning `Some((new_width, new_height))` when a resize occurred.
#[cfg(target_arch = "wasm32")]
pub fn resize_canvas_surface_if_needed(
    canvas_id: &str,
    device: &wgpu::Device,
    surface: &wgpu::Surface<'static>,
    surface_config: &mut wgpu::SurfaceConfiguration,
    depth_view: &mut wgpu::TextureView,
    depth_label: &str,
) -> Option<(u32, u32)> {
    let el = crate::wasm_client::dom::get_element(canvas_id)?;
    let canvas = el.dyn_into::<web_sys::HtmlCanvasElement>().ok()?;
    let cw = canvas.client_width().max(320) as u32;
    let ch = canvas.client_height().max(260) as u32;
    if cw != surface_config.width || ch != surface_config.height {
        canvas.set_width(cw);
        canvas.set_height(ch);
        surface_config.width = cw;
        surface_config.height = ch;
        surface.configure(device, surface_config);
        *depth_view = create_depth_view(device, cw, ch, depth_label);
        Some((cw, ch))
    } else {
        None
    }
}

/// Returns the `[GlbVertex, GlbInstanceData]` vertex buffer layouts for [`SHARED_GLB_DIORAMA_WGSL`].
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn shared_glb_vertex_buffer_layouts() -> [wgpu::VertexBufferLayout<'static>; 2] {
    const V_ATTRS: [wgpu::VertexAttribute; 4] = [
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
            format: wgpu::VertexFormat::Float32x4,
            offset: 24,
            shader_location: 2,
        },
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 40,
            shader_location: 3,
        },
    ];
    const I_ATTRS: [wgpu::VertexAttribute; 5] = [
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
        wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Float32x4,
            offset: 64,
            shader_location: 8,
        },
    ];
    [
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlbVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &V_ATTRS,
        },
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GlbInstanceData>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &I_ATTRS,
        },
    ]
}

/// Builds the shared `(uniform_buffer, bind_group, render_pipeline)` for [`SHARED_GLB_DIORAMA_WGSL`].
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn create_shared_diorama_pipeline(
    device: &wgpu::Device,
    surface_format: wgpu::TextureFormat,
    label_prefix: &str,
) -> (wgpu::Buffer, wgpu::BindGroup, wgpu::RenderPipeline) {
    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label_prefix),
        size: std::mem::size_of::<DioramaUniforms>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some(label_prefix),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label_prefix),
        layout: &bgl,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label_prefix),
        source: wgpu::ShaderSource::Wgsl(SHARED_GLB_DIORAMA_WGSL.into()),
    });
    let playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label_prefix),
        bind_group_layouts: &[&bgl],
        push_constant_ranges: &[],
    });
    let vbuffers = shared_glb_vertex_buffer_layouts();
    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label_prefix),
        layout: Some(&playout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &vbuffers,
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            cull_mode: None,
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
    (uniform_buffer, bind_group, render_pipeline)
}

#[cfg(test)]
mod tests {
    use super::*;
    use naga::front::wgsl::parse_str;
    use naga::valid::{Capabilities, ValidationFlags, Validator};

    fn assert_valid_wgsl(label: &str, wgsl_src: &str, expected_entry_points: &[&str]) {
        let module = parse_str(wgsl_src)
            .unwrap_or_else(|e| panic!("{label} WGSL parse error:\n{}", e.emit_to_string(wgsl_src)));
        let mut validator = Validator::new(ValidationFlags::all(), Capabilities::empty());
        validator
            .validate(&module)
            .unwrap_or_else(|e| panic!("{label} WGSL validation error: {e:?}"));
        for ep_name in expected_entry_points {
            assert!(
                module.entry_points.iter().any(|ep| ep.name == *ep_name),
                "{label} missing expected entry point '{ep_name}'"
            );
        }
    }

    #[test]
    fn test_shared_diorama_and_dice_wgsl_shaders_validate_offline() {
        assert_valid_wgsl(
            "SHARED_GLB_DIORAMA_WGSL",
            SHARED_GLB_DIORAMA_WGSL,
            &["vs_main", "fs_main"],
        );
        assert_valid_wgsl(
            "DICE_WGSL_SHADER",
            crate::wasm_client::dice_3d_wgpu::DICE_WGSL_SHADER,
            &["vs_main", "fs_back", "fs_front"],
        );
    }

    #[test]
    fn test_orbit_ortho_camera_ray_plane_unprojection_perspective_and_ortho() {
        let mut cam = OrbitOrthoCamera::tactical_map_default();
        let hit_persp = cam
            .screen_ndc_to_plane_xz(0.0, 0.0, 1.6, 0.0)
            .expect("Perspective center ray should hit ground");
        assert!(hit_persp[0].abs() < 1e-3 && hit_persp[1].abs() < 1e-3);

        cam.pan = [3.5, -2.0];
        cam.orthographic = true;
        let hit_ortho = cam
            .screen_ndc_to_plane_xz(0.0, 0.0, 1.6, 0.0)
            .expect("Ortho center ray should hit pan target");
        assert!((hit_ortho[0] - 3.5).abs() < 1e-2);
        assert!((hit_ortho[1] + 2.0).abs() < 1e-2);
    }
}
