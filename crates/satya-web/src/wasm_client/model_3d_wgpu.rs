//! Browser WebGPU (`wgpu`) 3D Character & `.glb` Turntable Viewer (`#notebook-pane-model`),
//! Shared Custom `.glb` Session Asset Registry, and Pedestal Geometry Builder.

pub use crate::wasm_client::wgpu_shared::*;

use crate::wasm_client::dice_3d::parse_hex_color;
use crate::wasm_client::glb_model::{
    GlbVertex, LoadedGlbModel, build_preset_glb_bytes, encode_glb_bytes, load_preset_glb_model,
    parse_glb_bytes,
};
use std::cell::{Cell, RefCell};
use wasm_bindgen::prelude::*;

// ============================================================================
// Shared Custom `.glb` Asset Registry (Accessible by Both Tab 3 and Tab 1)
// ============================================================================

thread_local! {
    static CUSTOM_GLB_MODEL: RefCell<Option<(LoadedGlbModel, Vec<u8>, u64)>> =
        const { RefCell::new(None) };
    static CUSTOM_GLB_GEN: Cell<u64> = const { Cell::new(0) };
}

/// Returns the monotonic generation counter for user-imported `"custom_glb"` assets (`0` when unset).
#[must_use]
pub fn custom_glb_generation() -> u64 {
    CUSTOM_GLB_MODEL.with(|cell| cell.borrow().as_ref().map_or(0, |(_, _, g)| *g))
}

/// Stores user-imported `.glb` bytes in the browser session (incrementing [`custom_glb_generation`])
/// so both the `3D Model` viewer and `"custom_glb"` miniatures on the Tactical Map render it.
pub fn set_custom_glb_asset(name: &str, raw_bytes: Vec<u8>) -> Result<(String, usize, f32), String> {
    let parsed = parse_glb_bytes(&raw_bytes, name)?;
    let summary = (
        parsed.name.clone(),
        parsed.triangle_count,
        parsed.height_m(),
    );
    let next_gen = CUSTOM_GLB_GEN.with(|g| {
        let n = g.get().wrapping_add(1).max(1);
        g.set(n);
        n
    });
    CUSTOM_GLB_MODEL.with(|cell| {
        *cell.borrow_mut() = Some((parsed, raw_bytes, next_gen));
    });
    Ok(summary)
}

/// Resolves a preset slug (`"valkyrie_rig"`, `"recon_mk4"`, `"secops_sentry"`, `"operative_light"`,
/// `"sector4_airlock"`, or `"custom_glb"`) into a [`LoadedGlbModel`].
#[must_use]
pub fn resolve_glb_model(preset: &str) -> LoadedGlbModel {
    if preset.trim() == "custom_glb"
        && let Some(custom) =
            CUSTOM_GLB_MODEL.with(|cell| cell.borrow().as_ref().map(|(m, _, _)| m.clone()))
    {
        return custom;
    }
    load_preset_glb_model(preset)
}

/// Builds the `3.2m x 3.2m` (`[-1.6, 1.6]`) gridded turntable pedestal quad (`6` vertices,
/// `zone_id = -1.0`) followed by all triangle vertices of `model`.
#[must_use]
pub fn build_pedestal_and_model_vertices(model: &LoadedGlbModel) -> Vec<GlbVertex> {
    let mut verts = Vec::with_capacity(model.vertices.len() + 6);
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

// ============================================================================
// Browser WebGPU (`wgpu`) Runtime for Tab 3 (`#wgpu-model-canvas`)
// ============================================================================

#[cfg(target_arch = "wasm32")]
mod model_wgpu_runtime {
    use super::*;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{MouseEvent, WheelEvent};

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

        resize_canvas_surface_if_needed(
            "wgpu-model-canvas",
            &pipe.device,
            &pipe.surface,
            &mut pipe.surface_config,
            &mut pipe.depth_view,
            "satya-model-depth",
        );

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
                let (dx, dy) = ((cx - lx) as f32, (cy - ly) as f32);
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
        let (_canvas, surface, device, queue, surface_config) =
            init_wgpu_canvas_context("wgpu-model-canvas").await?;
        let depth_view = create_depth_view(
            &device,
            surface_config.width,
            surface_config.height,
            "satya-model-depth",
        );
        let (uniform_buffer, bind_group, render_pipeline) =
            create_shared_diorama_pipeline(&device, surface_config.format, "satya-model");

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
            preset
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
    #[cfg(not(target_arch = "wasm32"))]
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
    wasm_bindgen_futures::spawn_local(async move {
        let Some((file_name, _mime, bytes)) =
            crate::wasm_client::dom::read_first_file_bytes("model-glb-file-input").await
        else {
            crate::wasm_client::dom::set_text(
                "model-telemetry-pill",
                "Failed to read .glb file bytes",
            );
            return;
        };
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

/// Serializes the `.glb` binary payload for `preset` (using raw or re-encoded bytes for `"custom_glb"`).
#[must_use]
pub fn export_glb_bytes_for_preset(preset: &str) -> Vec<u8> {
    let p_slug = if preset.trim().is_empty() {
        "valkyrie_rig"
    } else {
        preset.trim()
    };
    if p_slug == "custom_glb" {
        CUSTOM_GLB_MODEL
            .with(|c| {
                c.borrow().as_ref().map(|(m, raw, _)| {
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
    }
}

/// Exports and downloads the currently selected 3D asset as a spec-compliant `.glb` binary file
/// so it can be opened and sculpted in Blender (`File -> Import -> glTF 2.0`).
#[wasm_bindgen]
pub fn satya_export_current_glb_model() {
    let preset = crate::wasm_client::dom::select_value("model-preset-select");
    let p_slug = if preset.is_empty() {
        "valkyrie_rig"
    } else {
        preset.as_str()
    };
    let bytes = export_glb_bytes_for_preset(p_slug);
    let filename = format!("{p_slug}.glb");
    let _ = crate::wasm_client::dom::trigger_browser_download_bytes(&filename, &bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_glb_generation_counter_and_export_roundtrip() {
        let gen_before = custom_glb_generation();
        let sample_bytes = build_preset_glb_bytes("recon_mk4");
        let (name, tris, h_m) =
            set_custom_glb_asset("custom_recon.glb", sample_bytes.clone()).expect("valid glb");
        assert_eq!(name, "custom_recon.glb");
        assert!(tris >= 60);
        assert!(h_m > 1.2);
        let gen_after_first = custom_glb_generation();
        assert!(gen_after_first > gen_before);

        let _ = set_custom_glb_asset("custom_recon_v2.glb", sample_bytes.clone())
            .expect("second upload valid");
        let gen_after_second = custom_glb_generation();
        assert_eq!(gen_after_second, gen_after_first + 1);

        let resolved = resolve_glb_model("custom_glb");
        assert_eq!(resolved.name, "custom_recon_v2.glb");
        assert_eq!(export_glb_bytes_for_preset("custom_glb"), sample_bytes);
    }
}
