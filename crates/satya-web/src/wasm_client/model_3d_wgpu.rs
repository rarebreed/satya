//! Shared Orbit/Pan/Zoom + Orthographic Camera, Ray-Plane Unprojection, Unified PBR/Wireframe/HitZone
//! WGSL Shader, and Browser WebGPU (`wgpu`) 3D Character & `.glb` Turntable Viewer (`#notebook-pane-model`).

use crate::wasm_client::dice_3d::parse_hex_color;
use crate::wasm_client::glb_model::{LoadedGlbModel, load_preset_glb_model, parse_glb_bytes};
#[cfg(target_arch = "wasm32")]
use crate::wasm_client::glb_model::{GlbVertex, build_preset_glb_bytes, encode_glb_bytes};
use bytemuck::{Pod, Zeroable};
use wasm_bindgen::prelude::*;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let n = normalize(in.world_normal);
    let v = normalize(scene.camera_pos.xyz - in.world_pos);
    let key_l = normalize(vec3<f32>(0.48, 0.84, 0.36));
    let fill_l = normalize(vec3<f32>(-0.62, 0.42, -0.52));

    let ndl = max(dot(n, key_l), 0.0);
    let fill = max(dot(n, fill_l), 0.0) * 0.32;
    let hemi = mix(vec3<f32>(0.07, 0.10, 0.16), vec3<f32>(0.22, 0.28, 0.38), n.y * 0.5 + 0.5);
    let h = normalize(key_l + v);
    let spec = pow(max(dot(n, h), 0.0), 38.0) * 0.42;
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), 3.0) * 0.28;

    let mode = i32(round(scene.mode_params.x));
    var albedo = in.base_rgb;
    if mode == 2 {
        albedo = zone_palette(in.zone_id, in.base_rgb * 0.65);
    }

    // Subtle 1.0m minor / 5.0m major tactical grid lines on deck tiles (`zone_id < -0.5`)
    var grid_boost = vec3<f32>(0.0);
    if in.zone_id < -0.5 {
        let cell_scale = max(scene.mode_params.y, 0.25);
        let uv1 = fract(in.world_pos.xz / cell_scale + vec2<f32>(0.5, 0.5));
        let d1 = min(min(uv1.x, 1.0 - uv1.x), min(uv1.y, 1.0 - uv1.y));
        let line1 = 1.0 - smoothstep(0.0, 0.038, d1);
        let uv5 = fract((in.world_pos.xz / cell_scale + vec2<f32>(0.5, 0.5)) * 0.2);
        let d5 = min(min(uv5.x, 1.0 - uv5.x), min(uv5.y, 1.0 - uv5.y));
        let line5 = 1.0 - smoothstep(0.0, 0.016, d5);
        grid_boost = vec3<f32>(0.09, 0.22, 0.36) * line1 + vec3<f32>(0.14, 0.38, 0.58) * line5;
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
        let fw = fwidth(in.bary);
        let edge_d = min(min(in.bary.x / max(fw.x, 1e-4), in.bary.y / max(fw.y, 1e-4)), in.bary.z / max(fw.z, 1e-4));
        let wire = 1.0 - smoothstep(0.35, 1.25, edge_d);
        let wire_col = select(vec3<f32>(0.22, 0.78, 0.98), vec3<f32>(0.98, 0.32, 0.46), mode == 2);
        rgb = mix(rgb, wire_col, wire * scene.mode_params.z);
    }

    return vec4<f32>(rgb, in.alpha);
}
"#;

/// Uniform buffer (`80` bytes) bound at `@group(0) @binding(0)` for [`SHARED_GLB_DIORAMA_WGSL`].
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
        let rad = yaw_deg.to_radians();
        let (s, c) = rad.sin_cos();
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
        let yaw_rad = self.yaw_deg.to_radians();
        let (sy, cy) = yaw_rad.sin_cos();
        // Camera right = `[cy, -sy]` in `(x, z)` when eye sits at `[sin(yaw), cos(yaw)]`
        let rx = cy;
        let rz = -sy;
        let fx = -sy;
        let fz = -cy;
        let scale = 1.0 / self.zoom.max(0.35);
        self.pan[0] = (self.pan[0] + (rx * right_delta + fx * forward_delta) * scale)
            .clamp(-limit, limit);
        self.pan[1] = (self.pan[1] + (rz * right_delta + fz * forward_delta) * scale)
            .clamp(-limit, limit);
    }

    pub fn zoom_by_factor(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(0.35, 3.5);
    }

    /// Computes `(eye, target, right_vec, true_up_vec, forward_vec)` in world space.
    #[must_use]
    pub fn basis_vectors(&self) -> CameraBasisVectors {
        let dist = self.base_distance / self.zoom.max(0.25);
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
        let world_up = [0.0_f32, 1.0, 0.0];
        let right = normalize3(cross3(fwd, world_up));
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
            [
                -dot3(right, eye),
                -dot3(up, eye),
                dot3(fwd, eye),
                1.0,
            ],
        ];

        let proj = if self.orthographic {
            let half_h = (self.base_distance * 0.46) / self.zoom.max(0.25);
            let half_w = half_h * asp;
            let near = 0.1_f32;
            let far = 120.0_f32;
            [
                [1.0 / half_w, 0.0, 0.0, 0.0],
                [0.0, 1.0 / half_h, 0.0, 0.0],
                [0.0, 0.0, -1.0 / (far - near), 0.0],
                [0.0, 0.0, -near / (far - near), 1.0],
            ]
        } else {
            let fov_y = 42.0_f32.to_radians();
            let f = 1.0 / (0.5 * fov_y).tan();
            let near = 0.1_f32;
            let far = 140.0_f32;
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
            let half_h = (self.base_distance * 0.46) / self.zoom.max(0.25);
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

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let l = dot3(v, v).sqrt();
    if l > 1e-6 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        [0.0, 1.0, 0.0]
    }
}

fn mul_mat4_cols(a: [[f32; 4]; 4], b: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0_f32; 4]; 4];
    for c in 0..4 {
        for r in 0..4 {
            out[c][r] =
                a[0][r] * b[c][0] + a[1][r] * b[c][1] + a[2][r] * b[c][2] + a[3][r] * b[c][3];
        }
    }
    out
}

pub fn shader_mode_code(mode_str: &str) -> f32 {
    match mode_str.trim() {
        "pbr" => 0.0,
        "hit_zones" => 2.0,
        "thermal" => 3.0,
        _ => 1.0, // "wireframe_pbr"
    }
}

// ============================================================================
// Shared Custom `.glb` Asset Registry (Accessible by Both Tab 3 and Tab 1)
// ============================================================================

thread_local! {
    static CUSTOM_GLB_MODEL: std::cell::RefCell<Option<(LoadedGlbModel, Vec<u8>)>> =
        const { std::cell::RefCell::new(None) };
}

/// Stores user-imported `.glb` bytes in the browser session so both the `3D Model` viewer
/// and `"custom_glb"` miniatures on the Tactical Map can render it immediately.
pub fn set_custom_glb_asset(name: &str, raw_bytes: Vec<u8>) -> Result<(String, usize, f32), String> {
    let parsed = parse_glb_bytes(&raw_bytes, name)?;
    let summary = (
        parsed.name.clone(),
        parsed.triangle_count,
        parsed.height_m(),
    );
    CUSTOM_GLB_MODEL.with(|cell| {
        *cell.borrow_mut() = Some((parsed, raw_bytes));
    });
    Ok(summary)
}

/// Resolves a preset slug (`"valkyrie_rig"`, `"recon_mk4"`, `"secops_sentry"`, `"operative_light"`,
/// `"sector4_airlock"`, or `"custom_glb"`) into a [`LoadedGlbModel`].
#[must_use]
pub fn resolve_glb_model(preset: &str) -> LoadedGlbModel {
    if preset.trim() == "custom_glb"
        && let Some(custom) =
            CUSTOM_GLB_MODEL.with(|cell| cell.borrow().as_ref().map(|(m, _)| m.clone()))
    {
        return custom;
    }
    load_preset_glb_model(preset)
}

// ============================================================================
// Browser WebGPU (`wgpu`) Runtime for Tab 3 (`#wgpu-model-canvas`)
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod model_wgpu_runtime {
    use super::*;
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};

    pub struct ModelViewerPipeline {
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
        depth_view: wgpu::TextureView,
        uniform_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
        render_pipeline: wgpu::RenderPipeline,
        vertex_buffer: wgpu::Buffer,
        vertex_count: u32,
        instance_buffer: wgpu::Buffer,
        instance_count: u32,
    }

    pub struct ModelViewerController {
        pub pipeline: Option<ModelViewerPipeline>,
        pub initializing: bool,
        pub listeners_attached: bool,
        pub camera: OrbitOrthoCamera,
        pub active_drag: Option<(bool, i32, i32)>,
        pub active_preset: String,
        pub shader_mode: String,
        pub tint_rgb: [f32; 3],
        pub loaded_model: Option<LoadedGlbModel>,
        pub mesh_dirty: bool,
    }

    impl Default for ModelViewerController {
        fn default() -> Self {
            Self {
                pipeline: None,
                initializing: false,
                listeners_attached: false,
                camera: OrbitOrthoCamera::turntable_default(),
                active_drag: None,
                active_preset: "valkyrie_rig".to_string(),
                shader_mode: "wireframe_pbr".to_string(),
                tint_rgb: [0.06, 0.65, 0.91],
                loaded_model: None,
                mesh_dirty: true,
            }
        }
    }

    thread_local! {
        static MODEL_CTRL: RefCell<ModelViewerController> =
            RefCell::new(ModelViewerController::default());
    }

    pub fn with_model_ctrl<F, R>(f: F) -> R
    where
        F: FnOnce(&mut ModelViewerController) -> R,
    {
        MODEL_CTRL.with(|c| f(&mut c.borrow_mut()))
    }

    fn create_depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("satya-model-depth"),
            size: wgpu::Extent3d {
                width: w.max(1),
                height: h.max(1),
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

    fn build_pedestal_and_model_vertices(model: &LoadedGlbModel) -> Vec<GlbVertex> {
        let mut verts = Vec::with_capacity(model.vertices.len() + 6);
        // 3m x 3m turntable floor quad (`zone_id = -1.0` activates 1.0m shader grid lines)
        let r = 1.6_f32;
        let col = [0.07_f32, 0.11, 0.18, 0.0];
        let n = [0.0_f32, 1.0, 0.0];
        let corners = [
            [-r, -0.002, -r],
            [-r, -0.002, r],
            [r, -0.002, r],
            [r, -0.002, -r],
        ];
        let bary = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        for tri in [[0_usize, 1, 2], [0, 2, 3]] {
            for (ci, &vi) in tri.iter().enumerate() {
                verts.push(GlbVertex {
                    position: corners[vi],
                    normal: n,
                    color: col,
                    bary_zone: [bary[ci][0], bary[ci][1], bary[ci][2], -1.0],
                });
            }
        }
        verts.extend_from_slice(&model.vertices);
        verts
    }

    fn upload_active_mesh(ctrl: &mut ModelViewerController) {
        let model = resolve_glb_model(&ctrl.active_preset);
        let h = model.height_m().clamp(0.4, 6.0);
        ctrl.camera.target_y = h * 0.48;
        ctrl.camera.base_distance = (model.max_extent_m() * 1.85).clamp(2.4, 9.0);
        let all_verts = build_pedestal_and_model_vertices(&model);
        ctrl.loaded_model = Some(model);
        ctrl.mesh_dirty = false;

        let Some(pipe) = ctrl.pipeline.as_mut() else {
            return;
        };
        let byte_slice: &[u8] = bytemuck::cast_slice(&all_verts);
        let vb = pipe.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-model-vb"),
            size: (byte_slice.len().max(64)) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        pipe.queue.write_buffer(&vb, 0, byte_slice);
        pipe.vertex_buffer = vb;
        pipe.vertex_count = all_verts.len() as u32;
    }

    pub fn render_model_frame(ctrl: &mut ModelViewerController) {
        if ctrl.mesh_dirty || ctrl.loaded_model.is_none() {
            upload_active_mesh(ctrl);
        }
        let Some(pipe) = ctrl.pipeline.as_mut() else {
            return;
        };

        if let Some(el) = crate::wasm_client::dom::get_element("wgpu-model-canvas")
            && let Ok(canvas) = el.dyn_into::<HtmlCanvasElement>()
        {
            let cw = canvas.client_width().max(320) as u32;
            let ch = canvas.client_height().max(260) as u32;
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
        let (view_proj, eye) = ctrl.camera.build_view_proj_and_eye(aspect);
        let uniforms = DioramaUniforms {
            view_proj,
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            mode_params: [shader_mode_code(&ctrl.shader_mode), 1.0, 0.55, 0.0],
        };
        pipe.queue
            .write_buffer(&pipe.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let inst = GlbInstanceData::from_trs_and_tint(
            [0.0, 0.0, 0.0],
            0.0,
            [1.0, 1.0, 1.0],
            [ctrl.tint_rgb[0], ctrl.tint_rgb[1], ctrl.tint_rgb[2], 1.0],
        );
        pipe.queue
            .write_buffer(&pipe.instance_buffer, 0, bytemuck::bytes_of(&inst));
        pipe.instance_count = 1;

        let Ok(frame) = pipe.surface.get_current_texture() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = pipe
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("satya-model-enc"),
            });
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("satya-model-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
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
            rpass.set_pipeline(&pipe.render_pipeline);
            rpass.set_bind_group(0, &pipe.bind_group, &[]);
            rpass.set_vertex_buffer(0, pipe.vertex_buffer.slice(..));
            rpass.set_vertex_buffer(1, pipe.instance_buffer.slice(..));
            rpass.draw(0..pipe.vertex_count, 0..pipe.instance_count);
        }
        pipe.queue.submit(std::iter::once(enc.finish()));
        frame.present();
    }

    fn attach_listeners_if_needed() {
        let already = with_model_ctrl(|c| {
            if c.listeners_attached {
                true
            } else {
                c.listeners_attached = true;
                false
            }
        });
        if already {
            return;
        }
        let Some(canvas) = crate::wasm_client::dom::get_element("wgpu-model-canvas") else {
            with_model_ctrl(|c| c.listeners_attached = false);
            return;
        };
        let Some(win) = crate::wasm_client::dom::window() else {
            return;
        };

        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            ev.prevent_default();
            let is_pan = ev.shift_key() || ev.button() == 1 || ev.button() == 2;
            with_model_ctrl(|c| c.active_drag = Some((is_pan, ev.client_x(), ev.client_y())));
        });
        let _ =
            canvas.add_event_listener_with_callback("mousedown", on_down.as_ref().unchecked_ref());
        on_down.forget();

        let on_ctx = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            ev.prevent_default();
        });
        let _ =
            canvas.add_event_listener_with_callback("contextmenu", on_ctx.as_ref().unchecked_ref());
        on_ctx.forget();

        let on_dbl = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            ev.prevent_default();
            with_model_ctrl(|c| {
                c.camera = OrbitOrthoCamera::turntable_default();
                render_model_frame(c);
            });
        });
        let _ =
            canvas.add_event_listener_with_callback("dblclick", on_dbl.as_ref().unchecked_ref());
        on_dbl.forget();

        let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |ev: WheelEvent| {
            ev.prevent_default();
            let factor = if ev.delta_y() < 0.0 { 1.12 } else { 1.0 / 1.12 };
            with_model_ctrl(|c| {
                c.camera.zoom_by_factor(factor);
                render_model_frame(c);
            });
        });
        let _ = canvas.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
        on_wheel.forget();

        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            with_model_ctrl(|c| {
                let Some((is_pan, lx, ly)) = c.active_drag else {
                    return;
                };
                let (cx, cy) = (ev.client_x(), ev.client_y());
                let dx = (cx - lx) as f32;
                let dy = (cy - ly) as f32;
                if dx.abs() < 0.5 && dy.abs() < 0.5 {
                    return;
                }
                c.active_drag = Some((is_pan, cx, cy));
                if is_pan {
                    c.camera.pan_by_screen_delta(-dx * 0.008, dy * 0.008, 3.0);
                } else {
                    c.camera.orbit_by(-dx * 0.5, dy * 0.35);
                }
                render_model_frame(c);
            });
        });
        let _ = win.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        on_move.forget();

        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |_ev: MouseEvent| {
            with_model_ctrl(|c| c.active_drag = None);
        });
        let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_up.forget();
    }

    pub fn ensure_model_viewer_initialized_and_render() {
        attach_listeners_if_needed();
        let should_init = with_model_ctrl(|c| {
            if c.pipeline.is_some() {
                render_model_frame(c);
                false
            } else if c.initializing {
                false
            } else {
                c.initializing = true;
                true
            }
        });
        if !should_init {
            return;
        }
        spawn_local(async move {
            let res = init_model_pipeline().await;
            with_model_ctrl(|c| {
                c.initializing = false;
                match res {
                    Ok(pipe) => {
                        c.pipeline = Some(pipe);
                        c.mesh_dirty = true;
                        crate::wasm_client::dom::set_display("wgpu-model-canvas-wrap", "block");
                        crate::wasm_client::dom::set_text(
                            "model-wgpu-status-pill",
                            "WebGPU Active · glTF 2.0 (.glb)",
                        );
                        render_model_frame(c);
                    }
                    Err(err) => {
                        crate::wasm_client::dom::set_text("model-wgpu-status-pill", &err);
                    }
                }
            });
        });
    }

    async fn init_model_pipeline() -> Result<ModelViewerPipeline, String> {
        let Some(el) = crate::wasm_client::dom::get_element("wgpu-model-canvas") else {
            return Err("Canvas #wgpu-model-canvas not found".to_string());
        };
        let canvas: HtmlCanvasElement = el
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Not a canvas".to_string())?;
        let width = canvas.client_width().max(320) as u32;
        let height = canvas.client_height().max(260) as u32;
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });
        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas))
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
                    label: Some("satya-model-device"),
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
            .ok_or_else(|| "No surface format".to_string())?;
        let alpha_mode = caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(wgpu::CompositeAlphaMode::Auto);
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

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-model-uniforms"),
            size: std::mem::size_of::<DioramaUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("satya-model-bgl"),
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
            label: Some("satya-model-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("satya-model-wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHARED_GLB_DIORAMA_WGSL.into()),
        });
        let playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("satya-model-playout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });

        let vbuffers = shared_glb_vertex_buffer_layouts();
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("satya-model-pipeline"),
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
                    format,
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

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-model-empty-vb"),
            size: 64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-model-ib"),
            size: std::mem::size_of::<GlbInstanceData>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(ModelViewerPipeline {
            device,
            queue,
            surface,
            surface_config,
            depth_view,
            uniform_buffer,
            bind_group,
            render_pipeline,
            vertex_buffer,
            vertex_count: 0,
            instance_buffer,
            instance_count: 1,
        })
    }
}

#[cfg(target_arch = "wasm32")]
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

/// Synchronizes `#model-preset-select`, `#model-render-mode`, and `#model-tint-color` into the
/// live `wgpu` `.glb` turntable viewer (as well as the subsystem hit-zone readout).
pub fn sync_3d_model_viewer_from_dom() {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    let preset = crate::wasm_client::dom::select_value("model-preset-select");
    let mode = crate::wasm_client::dom::select_value("model-render-mode");
    let tint_hex = crate::wasm_client::dom::input_value("model-tint-color");
    let (_, tint_rgb) = parse_hex_color(
        if tint_hex.is_empty() {
            "#0ea5e9"
        } else {
            &tint_hex
        },
        [0.06, 0.65, 0.91],
    );

    #[cfg(target_arch = "wasm32")]
    {
        let p_eff = if preset.is_empty() {
            "valkyrie_rig".to_string()
        } else {
            preset.clone()
        };
        let m_eff = if mode.is_empty() {
            "wireframe_pbr".to_string()
        } else {
            mode
        };
        model_wgpu_runtime::with_model_ctrl(|c| {
            if c.active_preset != p_eff {
                c.active_preset = p_eff;
                c.mesh_dirty = true;
            }
            c.shader_mode = m_eff;
            c.tint_rgb = tint_rgb;
        });
        model_wgpu_runtime::ensure_model_viewer_initialized_and_render();
    }
    let _ = (preset, mode, tint_rgb);
}

/// Resets the `3D Model` turntable camera to its default front-quarter angle.
#[wasm_bindgen]
pub fn satya_reset_model_camera() {
    #[cfg(target_arch = "wasm32")]
    model_wgpu_runtime::with_model_ctrl(|c| {
        c.camera = OrbitOrthoCamera::turntable_default();
        model_wgpu_runtime::render_model_frame(c);
    });
}

/// Reads a user-selected `.glb` file from `#model-glb-file-input`, parses its glTF 2.0 binary
/// hierarchy and materials, and activates it in the `3D Model` viewer (and Tactical Map `"custom_glb"`).
#[wasm_bindgen]
pub fn satya_import_custom_glb_model() {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Function, Promise, Reflect, Uint8Array};
        use wasm_bindgen::JsCast;
        use wasm_bindgen_futures::{JsFuture, spawn_local};

        let Some(input_el) = crate::wasm_client::dom::get_element("model-glb-file-input") else {
            return;
        };
        let Ok(files) = Reflect::get(&input_el, &JsValue::from_str("files")) else {
            return;
        };
        let Ok(file) = Reflect::get(&files, &JsValue::from_f64(0.0)) else {
            return;
        };
        if file.is_null() || file.is_undefined() {
            return;
        }
        let file_name = Reflect::get(&file, &JsValue::from_str("name"))
            .ok()
            .and_then(|v| v.as_string())
            .unwrap_or_else(|| "custom_model.glb".to_string());
        let Ok(ab_fn_val) = Reflect::get(&file, &JsValue::from_str("arrayBuffer")) else {
            return;
        };
        let Ok(ab_fn) = ab_fn_val.dyn_into::<Function>() else {
            return;
        };
        let Ok(promise_val) = ab_fn.call0(&file) else {
            return;
        };
        let promise = Promise::from(promise_val);

        spawn_local(async move {
            let Ok(buf_val) = JsFuture::from(promise).await else {
                crate::wasm_client::dom::set_text(
                    "model-telemetry-pill",
                    "Failed to read .glb file bytes",
                );
                return;
            };
            let bytes = Uint8Array::new(&buf_val).to_vec();
            match set_custom_glb_asset(&file_name, bytes) {
                Ok((name, tris, h_m)) => {
                    crate::wasm_client::dom::set_select_value("model-preset-select", "custom_glb");
                    crate::wasm_client::dom::set_text(
                        "model-telemetry-pill",
                        &format!("Custom GLB: {name} · {tris} tris ({h_m:.2}m)"),
                    );
                    model_wgpu_runtime::with_model_ctrl(|c| {
                        c.active_preset = "custom_glb".to_string();
                        c.mesh_dirty = true;
                    });
                    model_wgpu_runtime::ensure_model_viewer_initialized_and_render();
                    crate::wasm_client::map_3d::refresh_map_wgpu_if_active();
                }
                Err(err) => {
                    crate::wasm_client::dom::set_text(
                        "model-telemetry-pill",
                        &format!("GLB Error: {err}"),
                    );
                }
            }
        });
    }
}

/// Exports and downloads the currently selected 3D asset as a spec-compliant `.glb` binary file
/// so it can be opened and sculpted in Blender (`File -> Import -> glTF 2.0`).
#[wasm_bindgen]
pub fn satya_export_current_glb_model() {
    #[cfg(target_arch = "wasm32")]
    {
        use js_sys::{Array, Function, Object, Reflect, Uint8Array};
        use wasm_bindgen::JsCast;

        let preset = crate::wasm_client::dom::select_value("model-preset-select");
        let p_slug = if preset.is_empty() {
            "valkyrie_rig"
        } else {
            preset.as_str()
        };
        let bytes = if p_slug == "custom_glb" {
            CUSTOM_GLB_MODEL
                .with(|c| {
                    c.borrow()
                        .as_ref()
                        .map(|(m, raw)| {
                            if raw.is_empty() {
                                encode_glb_bytes(&m.submeshes)
                            } else {
                                raw.clone()
                            }
                        })
                })
                .unwrap_or_else(|| build_preset_glb_bytes("valkyrie_rig"))
        } else {
            build_preset_glb_bytes(p_slug)
        };

        let Some(win) = crate::wasm_client::dom::window() else {
            return;
        };
        let Some(doc) = crate::wasm_client::dom::document() else {
            return;
        };
        let u8_arr = Uint8Array::from(bytes.as_slice());
        let parts = Array::of1(&u8_arr);
        let opts = Object::new();
        let _ = Reflect::set(
            &opts,
            &JsValue::from_str("type"),
            &JsValue::from_str("model/gltf-binary"),
        );
        let Ok(blob_ctor) = Reflect::get(&win, &JsValue::from_str("Blob")) else {
            return;
        };
        let Ok(blob_fn) = blob_ctor.dyn_into::<Function>() else {
            return;
        };
        let Ok(blob) = Reflect::construct(&blob_fn, &Array::of2(&parts, &opts)) else {
            return;
        };
        let Ok(url_obj) = Reflect::get(&win, &JsValue::from_str("URL")) else {
            return;
        };
        let Ok(create_fn_val) = Reflect::get(&url_obj, &JsValue::from_str("createObjectURL")) else {
            return;
        };
        let Ok(create_fn) = create_fn_val.dyn_into::<Function>() else {
            return;
        };
        let Ok(href_val) = create_fn.call1(&url_obj, &blob) else {
            return;
        };
        let Some(href) = href_val.as_string() else {
            return;
        };

        if let Ok(a_el) = doc.create_element("a") {
            let _ = a_el.set_attribute("href", &href);
            let _ = a_el.set_attribute("download", &format!("{p_slug}.glb"));
            if let Ok(html_a) = a_el.dyn_into::<web_sys::HtmlElement>() {
                html_a.click();
            }
        }
        if let Ok(rev_val) = Reflect::get(&url_obj, &JsValue::from_str("revokeObjectURL"))
            && let Ok(rev_fn) = rev_val.dyn_into::<Function>()
        {
            let _ = rev_fn.call1(&url_obj, &JsValue::from_str(&href));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orbit_ortho_camera_ray_plane_unprojection_perspective_and_ortho() {
        let mut cam = OrbitOrthoCamera::tactical_map_default();
        // Center of screen `(0.0, 0.0)` must hit camera pan target `(0.0, 0.0)` on ground `y = 0`
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
