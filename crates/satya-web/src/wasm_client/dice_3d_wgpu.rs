//! Browser WebGPU (`wgpu`) Two-Pass Translucent Dice Pipeline, WGSL Shader, Orbit/Pan/Zoom Camera
//! Matrix Builder, and DOM Slider/Canvas Event Synchronization.

#[cfg(target_arch = "wasm32")]
use crate::wasm_client::dice_3d::{
    ATLAS_HEIGHT, ATLAS_WIDTH, DieInstanceData, DieKind, DieVertex, SceneUniforms,
    build_polyhedral_mesh, compute_view_proj_matrix_with_camera, generate_dice_numeral_atlas_rgba8,
    pack_model_matrix, vec3_dot, vec3_sub,
};
use crate::wasm_client::dice_3d::{DiceCameraConfig, DiceMaterialConfig, DicePhysicsScene};
use satya_schema::WgpuDiceSpec;
use wasm_bindgen::JsCast;

/// WGSL shader source implementing two-pass translucent polyhedral rendering:
/// - `fs_back`: interior back-face pass (`CullMode::Front`) rendering refracted internal facets,
///   tabletop grid lines, and opposite-side numerals visible through the glass die body.
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
    let ndl = max(dot(n, l1), 0.22);

    // Internal facet edge refraction & tabletop grid visible through translucent dice
    let edge_highlight = 1.0 - smoothstep(0.0, 0.10, in.edge_dist);
    let ink_mask = textureSample(numeral_tex, numeral_sampler, in.uv).a;

    let grid_uv = fract(in.world_pos.xz * 0.85);
    let grid_line = (1.0 - smoothstep(0.0, 0.06, min(grid_uv.x, 1.0 - grid_uv.x)))
        + (1.0 - smoothstep(0.0, 0.06, min(grid_uv.y, 1.0 - grid_uv.y)));
    let tray_refract = vec3<f32>(0.08, 0.18, 0.30) * clamp(grid_line, 0.0, 1.0) * (1.0 - scene.base_color.a);

    let body_rgb = scene.base_color.rgb * (0.30 + 0.38 * ndl)
        + mix(scene.base_color.rgb, vec3<f32>(1.0, 1.0, 1.0), 0.38) * edge_highlight * 0.44
        + tray_refract;
    let mixed_rgb = mix(body_rgb, scene.ink_color.rgb * 0.72, ink_mask * 0.52);
    let back_alpha = clamp(scene.base_color.a * 0.40 + edge_highlight * 0.24 + ink_mask * 0.20, 0.05, 0.80);
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
    let spec = pow(max(dot(n, h), 0.0), 54.0) * 0.70;
    let fresnel = pow(1.0 - max(dot(n, v), 0.0), 2.8);

    // Chamfered polygon edge bevel highlight
    let edge_bevel = 1.0 - smoothstep(0.0, 0.07, in.edge_dist);
    let ink_mask = textureSample(numeral_tex, numeral_sampler, in.uv).a;

    let lit_body = scene.base_color.rgb * (0.24 + 0.64 * diff1 + diff2)
        + vec3<f32>(1.0, 1.0, 1.0) * spec
        + mix(scene.base_color.rgb, vec3<f32>(1.0, 1.0, 1.0), 0.55) * (fresnel * 0.40 + edge_bevel * 0.28);

    let final_rgb = mix(lit_body, scene.ink_color.rgb, ink_mask);
    // Keep front-surface glass transmission low when translucent so back-facing facets and
    // refracted edges shine clearly through the die body, while still reaching 1.0 when opaque.
    let glass_base = mix(scene.base_color.a * 0.50, 1.0, pow(scene.base_color.a, 3.2));
    let body_alpha = clamp(glass_base + fresnel * 0.16 + spec * 0.35 + edge_bevel * 0.14, 0.06, 1.0);
    let final_alpha = max(body_alpha, ink_mask * 0.96);
    return vec4<f32>(final_rgb, final_alpha);
}
"#;

// ============================================================================
// Browser WebGPU (`wgpu`) Runtime State
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod webgpu_runtime {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::prelude::*;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};

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

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CanvasDragMode {
        Orbit,
        Pan,
    }

    #[derive(Default)]
    pub struct DiceRendererController {
        pub pipeline: Option<WebGpuDicePipeline>,
        pub initializing: bool,
        pub canvas_listeners_attached: bool,
        pub material: DiceMaterialConfig,
        pub camera: DiceCameraConfig,
        pub active_drag: Option<(CanvasDragMode, i32, i32)>,
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

    pub fn attach_canvas_camera_listeners_if_needed() {
        let already_attached = with_dice_controller(|ctrl| {
            if ctrl.canvas_listeners_attached {
                true
            } else {
                ctrl.canvas_listeners_attached = true;
                false
            }
        });
        if already_attached {
            return;
        }
        let Some(canvas) = crate::wasm_client::dom::get_element("wgpu-dice-canvas") else {
            with_dice_controller(|ctrl| {
                ctrl.canvas_listeners_attached = false;
            });
            return;
        };
        let Some(win) = crate::wasm_client::dom::window() else {
            return;
        };

        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            ev.prevent_default();
            let mode = if ev.shift_key() || ev.button() == 1 || ev.button() == 2 {
                CanvasDragMode::Pan
            } else {
                CanvasDragMode::Orbit
            };
            with_dice_controller(|ctrl| {
                ctrl.active_drag = Some((mode, ev.client_x(), ev.client_y()));
            });
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
            with_dice_controller(|ctrl| {
                ctrl.camera.reset();
                super::sync_camera_dom_from_config(&ctrl.camera);
                render_current_frame(ctrl);
            });
        });
        let _ =
            canvas.add_event_listener_with_callback("dblclick", on_dbl.as_ref().unchecked_ref());
        on_dbl.forget();

        let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |ev: WheelEvent| {
            ev.prevent_default();
            let dy = ev.delta_y();
            let factor = if dy < 0.0 {
                1.10_f32
            } else if dy > 0.0 {
                1.0 / 1.10_f32
            } else {
                1.0
            };
            with_dice_controller(|ctrl| {
                ctrl.camera.zoom_by_factor(factor);
                super::sync_camera_dom_from_config(&ctrl.camera);
                render_current_frame(ctrl);
            });
        });
        let _ = canvas.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
        on_wheel.forget();

        let on_move = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            with_dice_controller(|ctrl| {
                let Some((mode, last_x, last_y)) = ctrl.active_drag else {
                    return;
                };
                let cx = ev.client_x();
                let cy = ev.client_y();
                let dx = (cx - last_x) as f32;
                let dy = (cy - last_y) as f32;
                if dx.abs() < 0.5 && dy.abs() < 0.5 {
                    return;
                }
                ctrl.active_drag = Some((mode, cx, cy));
                match mode {
                    CanvasDragMode::Orbit => {
                        ctrl.camera.orbit_by(-dx * 0.45, dy * 0.35);
                    }
                    CanvasDragMode::Pan => {
                        ctrl.camera.pan_by_screen_delta(-dx * 0.015, -dy * 0.015);
                    }
                }
                super::sync_camera_dom_from_config(&ctrl.camera);
                render_current_frame(ctrl);
            });
        });
        let _ = win.add_event_listener_with_callback("mousemove", on_move.as_ref().unchecked_ref());
        on_move.forget();

        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |_ev: MouseEvent| {
            with_dice_controller(|ctrl| {
                ctrl.active_drag = None;
            });
        });
        let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_up.forget();
    }

    pub fn ensure_webgpu_initialized_and_render() {
        attach_canvas_camera_listeners_if_needed();
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
                        super::sync_camera_dom_from_config(&ctrl.camera);
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
        let (view_proj, eye) =
            compute_view_proj_matrix_with_camera(aspect, die_count, &ctrl.camera);

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

        // Sort dice back-to-front relative to camera eye for accurate translucent compositing
        let mut instances_with_dist: Vec<(f32, DieInstanceData)> =
            Vec::with_capacity(scene.dice.len());
        for i in 0..scene.dice.len() {
            let (pos, rot) = scene.sample_die_transform(i, scene.progress);
            let diff = vec3_sub(pos, eye);
            let dist_sq = vec3_dot(diff, diff);
            instances_with_dist.push((
                dist_sq,
                DieInstanceData {
                    model: pack_model_matrix(pos, rot),
                },
            ));
        }
        instances_with_dist
            .sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let instances: Vec<DieInstanceData> =
            instances_with_dist.into_iter().map(|(_, d)| d).collect();
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

#[cfg(not(target_arch = "wasm32"))]
mod webgpu_runtime {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    pub struct DiceRendererController {
        pub pipeline: Option<()>,
        pub material: DiceMaterialConfig,
        pub camera: DiceCameraConfig,
        pub scene: Option<DicePhysicsScene>,
        pub anim_start_ms: f64,
    }

    thread_local! {
        static DICE_CONTROLLER: RefCell<DiceRendererController> =
            RefCell::new(DiceRendererController::default());
    }

    pub fn with_dice_controller<F, R>(f: F) -> R
    where
        F: FnOnce(&mut DiceRendererController) -> R,
    {
        DICE_CONTROLLER.with(|cell| f(&mut cell.borrow_mut()))
    }

    pub fn render_current_frame(_ctrl: &mut DiceRendererController) {}
    pub fn start_animation_loop_if_needed(_ctrl: &mut DiceRendererController) {}
    pub fn ensure_webgpu_initialized_and_render() {}
}

fn sync_range_slider_css_vars(
    id: &str,
    min_str: &str,
    max_str: &str,
    val_str: &str,
    aria_text: &str,
) {
    if let Some(slider_el) = crate::wasm_client::dom::get_element(id)
        && let Ok(html_el) = slider_el.clone().dyn_into::<web_sys::HtmlElement>()
    {
        let _ = html_el.style().set_property("--attr-min", min_str);
        let _ = html_el.style().set_property("--attr-max", max_str);
        let _ = html_el.style().set_property("--control-value", val_str);
        let _ = slider_el.set_attribute("aria-valuetext", aria_text);
    }
}

/// Synchronizes the DOM camera sliders and readouts (`#dice-camera-yaw`, `#dice-camera-pitch`,
/// `#dice-camera-zoom`, `#dice-camera-pan-readout`) with `cam`.
pub fn sync_camera_dom_from_config(cam: &DiceCameraConfig) {
    let yaw_i = cam.yaw_deg.round() as i32;
    let pitch_i = cam.pitch_deg.round() as i32;
    let zoom_pct = (cam.zoom * 100.0).round() as i32;

    crate::wasm_client::dom::set_input_value("dice-camera-yaw", &yaw_i.to_string());
    crate::wasm_client::dom::set_input_value("dice-camera-pitch", &pitch_i.to_string());
    crate::wasm_client::dom::set_input_value("dice-camera-zoom", &zoom_pct.to_string());

    sync_range_slider_css_vars(
        "dice-camera-yaw",
        "-180",
        "180",
        &yaw_i.to_string(),
        &format!("{yaw_i} degrees yaw"),
    );
    sync_range_slider_css_vars(
        "dice-camera-pitch",
        "12",
        "88",
        &pitch_i.to_string(),
        &format!("{pitch_i} degrees pitch"),
    );
    sync_range_slider_css_vars(
        "dice-camera-zoom",
        "40",
        "250",
        &zoom_pct.to_string(),
        &format!("{zoom_pct}% zoom"),
    );

    crate::wasm_client::dom::set_text("dice-camera-yaw-readout", &format!("{yaw_i}°"));
    crate::wasm_client::dom::set_text("dice-camera-pitch-readout", &format!("{pitch_i}°"));
    crate::wasm_client::dom::set_text("dice-camera-zoom-readout", &format!("{zoom_pct}%"));
    crate::wasm_client::dom::set_text(
        "dice-camera-pan-readout",
        &format!("({:+.1}, {:+.1})", cam.pan[0], cam.pan[1]),
    );
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
    let opacity_pct = opacity_raw.parse::<f32>().unwrap_or(38.0).clamp(8.0, 100.0);
    let opacity = opacity_pct / 100.0;

    sync_range_slider_css_vars(
        "dice-opacity-slider",
        "8",
        "100",
        &format!("{opacity_pct:.0}"),
        &format!("{opacity_pct:.0}% opacity"),
    );
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

/// Reads camera orbit angle (`#dice-camera-yaw`, `#dice-camera-pitch`) and zoom (`#dice-camera-zoom`)
/// sliders from the DOM and immediately re-renders the 3D WebGPU dice canvas.
pub fn sync_dice_camera_from_dom_and_render() {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    let yaw = crate::wasm_client::dom::input_value("dice-camera-yaw")
        .parse::<f32>()
        .unwrap_or(0.0)
        .clamp(-180.0, 180.0);
    let pitch = crate::wasm_client::dom::input_value("dice-camera-pitch")
        .parse::<f32>()
        .unwrap_or(55.0)
        .clamp(12.0, 88.0);
    let zoom_pct = crate::wasm_client::dom::input_value("dice-camera-zoom")
        .parse::<f32>()
        .unwrap_or(100.0)
        .clamp(40.0, 250.0);

    webgpu_runtime::with_dice_controller(|ctrl| {
        ctrl.camera.yaw_deg = yaw;
        ctrl.camera.pitch_deg = pitch;
        ctrl.camera.zoom = zoom_pct / 100.0;
        sync_camera_dom_from_config(&ctrl.camera);
        webgpu_runtime::render_current_frame(ctrl);
    });
    webgpu_runtime::ensure_webgpu_initialized_and_render();
}

/// Pans the 3D WebGPU dice camera across the tray floor by `(dx, dz)` and re-renders.
pub fn pan_dice_camera_and_render(dx: f32, dz: f32) {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    webgpu_runtime::with_dice_controller(|ctrl| {
        ctrl.camera.pan_by_screen_delta(dx, -dz);
        sync_camera_dom_from_config(&ctrl.camera);
        webgpu_runtime::render_current_frame(ctrl);
    });
    webgpu_runtime::ensure_webgpu_initialized_and_render();
}

/// Resets the 3D WebGPU dice camera orbit angle, pan offset, and zoom factor to defaults and re-renders.
pub fn reset_dice_camera_and_render() {
    if crate::wasm_client::dom::window().is_none() {
        return;
    }
    webgpu_runtime::with_dice_controller(|ctrl| {
        ctrl.camera.reset();
        sync_camera_dom_from_config(&ctrl.camera);
        webgpu_runtime::render_current_frame(ctrl);
    });
    webgpu_runtime::ensure_webgpu_initialized_and_render();
}
