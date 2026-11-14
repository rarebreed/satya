//! Browser WebGPU (`wgpu`) 2.5D/3D Tactical Map Diorama & 2D Top-Down Orthographic Renderer
//! (`#wgpu-map-canvas`), Procedural Terrain/Cover/Cutaway-Bulkhead Geometry, and `.glb` Miniatures.

pub use crate::wasm_client::map_3d_diorama_mesh::*;
pub use crate::wasm_client::wgpu_shared::*;

use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
mod map_wgpu_runtime {
    use super::*;
    use crate::wasm_client::dice_3d::parse_hex_color;
    use crate::wasm_client::glb_model::LoadedGlbModel;
    use crate::wasm_client::map_3d::{
        metric_to_world_xz, resolve_token_visual, with_map_ui, world_xz_to_metric,
    };
    use crate::wasm_client::model_3d_wgpu::{custom_glb_generation, resolve_glb_model};
    use satya_schema::{MessageVisibility, PropKind};
    use std::cell::RefCell;
    use std::collections::HashMap;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{MouseEvent, WheelEvent};

    struct GpuMeshSlice {
        vertex_buffer: wgpu::Buffer,
        vertex_count: u32,
    }

    pub struct MapDioramaPipeline {
        device: wgpu::Device,
        queue: wgpu::Queue,
        surface: wgpu::Surface<'static>,
        surface_config: wgpu::SurfaceConfiguration,
        depth_view: wgpu::TextureView,
        uniform_buffer: wgpu::Buffer,
        bind_group: wgpu::BindGroup,
        render_pipeline: wgpu::RenderPipeline,
        diorama_vb: wgpu::Buffer,
        diorama_vcount: u32,
        diorama_ib: wgpu::Buffer,
        miniature_meshes: HashMap<String, GpuMeshSlice>,
        cached_custom_glb_gen: u64,
        miniature_ib: wgpu::Buffer,
    }

    pub struct MapDioramaController {
        pub pipeline: Option<MapDioramaPipeline>,
        pub initializing: bool,
        pub listeners_attached: bool,
        pub camera: OrbitOrthoCamera,
        /// `(is_pan, start_x, start_y, last_x, last_y, moved_beyond_click)`
        pub drag_state: Option<(bool, i32, i32, i32, i32, bool)>,
        pub scene_dirty: bool,
        pub shader_mode: String,
    }

    impl Default for MapDioramaController {
        fn default() -> Self {
            Self {
                pipeline: None,
                initializing: false,
                listeners_attached: false,
                camera: OrbitOrthoCamera::tactical_map_default(),
                drag_state: None,
                scene_dirty: true,
                shader_mode: "pbr".to_string(),
            }
        }
    }

    thread_local! {
        static MAP_CTRL: RefCell<MapDioramaController> =
            RefCell::new(MapDioramaController::default());
    }

    pub fn with_map_ctrl<F, R>(f: F) -> R
    where
        F: FnOnce(&mut MapDioramaController) -> R,
    {
        MAP_CTRL.with(|c| f(&mut c.borrow_mut()))
    }

    fn ensure_miniature_gpu_mesh(pipe: &mut MapDioramaPipeline, preset: &str) {
        let cur_gen = custom_glb_generation();
        if preset == "custom_glb" {
            if pipe.miniature_meshes.contains_key(preset)
                && pipe.cached_custom_glb_gen == cur_gen
            {
                return;
            }
        } else if pipe.miniature_meshes.contains_key(preset) {
            return;
        }
        let model: LoadedGlbModel = resolve_glb_model(preset);
        let bytes: &[u8] = bytemuck::cast_slice(&model.vertices);
        let vb = pipe.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-map-mini-vb"),
            size: bytes.len().max(64) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        pipe.queue.write_buffer(&vb, 0, bytes);
        pipe.miniature_meshes.insert(
            preset.to_string(),
            GpuMeshSlice {
                vertex_buffer: vb,
                vertex_count: model.vertices.len() as u32,
            },
        );
        if preset == "custom_glb" {
            pipe.cached_custom_glb_gen = cur_gen;
        }
    }

    /// Continuous metric `(x_m, y_m)` under the pointer on the `y = 0` deck plane (inside the board).
    fn pick_metric_point_from_mouse(ev: &MouseEvent, cam: &OrbitOrthoCamera) -> Option<(f32, f32)> {
        let el = crate::wasm_client::dom::get_element("wgpu-map-canvas")?;
        let rect = el.get_bounding_client_rect();
        let rw = rect.width().max(1.0) as f32;
        let rh = rect.height().max(1.0) as f32;
        let local_x = (ev.client_x() as f64 - rect.left()) as f32;
        let local_y = (ev.client_y() as f64 - rect.top()) as f32;
        if local_x < 0.0 || local_y < 0.0 || local_x > rw || local_y > rh {
            return None;
        }
        let ndc_x = (local_x / rw) * 2.0 - 1.0;
        let ndc_y = 1.0 - (local_y / rh) * 2.0;
        let [wx, wz] = cam.screen_ndc_to_plane_xz(ndc_x, ndc_y, rw / rh, 0.0)?;
        with_map_ui(|ui| {
            let (bw_m, bh_m) = ui.map.board_size_m();
            world_xz_to_metric(wx, wz, bw_m, bh_m)
        })
    }

    pub fn render_map_frame(ctrl: &mut MapDioramaController) {
        let Some(pipe) = ctrl.pipeline.as_mut() else {
            return;
        };

        resize_canvas_surface_if_needed(
            "wgpu-map-canvas",
            &pipe.device,
            &pipe.surface,
            &mut pipe.surface_config,
            &mut pipe.depth_view,
            "satya-map-depth",
        );

        if ctrl.scene_dirty {
            let verts = with_map_ui(build_tactical_diorama_vertices);
            let bytes: &[u8] = bytemuck::cast_slice(&verts);
            let vb = pipe.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("satya-map-diorama-vb"),
                size: bytes.len().max(64) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            pipe.queue.write_buffer(&vb, 0, bytes);
            pipe.diorama_vb = vb;
            pipe.diorama_vcount = verts.len() as u32;
            let world_inst =
                GlbInstanceData::from_trs_and_tint([0.0, 0.0, 0.0], 0.0, [1.0, 1.0, 1.0], [1.0; 4]);
            pipe.queue
                .write_buffer(&pipe.diorama_ib, 0, bytemuck::bytes_of(&world_inst));
            ctrl.scene_dirty = false;
        }

        // Gather token miniature draw calls grouped in order
        let (cell_m, show_grid, draws): (f32, bool, Vec<(String, GlbInstanceData)>) = with_map_ui(|ui| {
            let cm = ui.map.cell_size_m();
            let (bw_m, bh_m) = ui.map.board_size_m();
            let mut list = Vec::with_capacity(ui.map.tokens.len());
            for tok in &ui.map.tokens {
                if ui.map.token_floor(&tok.entity_id) != ui.map.active_floor {
                    continue;
                }
                let (preset, hex_col, facing_deg) = resolve_token_visual(&ui.map, tok);
                let (_, rgb) = parse_hex_color(&hex_col, [0.22, 0.74, 0.98]);
                let (tx_m, ty_m) = ui.map.token_pos_m(tok);
                let [wx, wz] = metric_to_world_xz(tx_m, ty_m, bw_m, bh_m);
                let on_catwalk = ui.map.props.iter().any(|p| {
                    p.kind == PropKind::ElevatedCatwalk
                        && satya_engine::tactical_map_geom::prop_contains_point(p, tx_m, ty_m, false)
                });
                let on_bridge = !on_catwalk
                    && matches!(
                        satya_engine::map_surface_geom::topmost_surface_at_m(&ui.map.surfaces, tx_m, ty_m)
                            .map(|s| s.kind),
                        Some(satya_schema::SurfaceFeatureKind::BridgeDeck)
                    );
                let active_floor_elev_cm = ui.map.floor_elevation_cm(ui.map.active_floor);
                let base_y = if on_catwalk {
                    1.48
                } else if on_bridge {
                    0.12
                } else {
                    ui.map.visual_for(&tok.entity_id).map_or(0.0, |v| {
                        ((v.elevation_cm - active_floor_elev_cm) as f32 / 100.0).max(0.0)
                    })
                };
                let alpha = if tok.visibility == MessageVisibility::GmOnly {
                    0.55
                } else {
                    1.0
                };
                // Convert `0 deg = North (-Z)` vs `.glb` forward `-Z`: `-facing_deg` yaw around `+Y`
                let inst = GlbInstanceData::from_trs_and_tint(
                    [wx, base_y, wz],
                    -facing_deg,
                    [0.92, 0.92, 0.92],
                    [rgb[0], rgb[1], rgb[2], alpha],
                );
                list.push((preset, inst));
            }
            (cm, ui.show_grid_overlay, list)
        });

        for (preset, _) in &draws {
            ensure_miniature_gpu_mesh(pipe, preset);
        }
        let all_insts: Vec<GlbInstanceData> = draws.iter().map(|(_, d)| *d).collect();
        let max_inst_bytes = (all_insts.len().max(1) * std::mem::size_of::<GlbInstanceData>()) as u64;
        if pipe.miniature_ib.size() < max_inst_bytes {
            pipe.miniature_ib = pipe.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("satya-map-mini-ib"),
                size: max_inst_bytes * 2,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if !all_insts.is_empty() {
            pipe.queue
                .write_buffer(&pipe.miniature_ib, 0, bytemuck::cast_slice(&all_insts));
        }

        let aspect =
            (pipe.surface_config.width as f32) / (pipe.surface_config.height.max(1) as f32);
        let (view_proj, eye) = ctrl.camera.build_view_proj_and_eye(aspect);
        let mode_f = shader_mode_code(&ctrl.shader_mode);
        let uniforms = DioramaUniforms {
            view_proj,
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            mode_params: [mode_f, cell_m, 0.42, if show_grid { 1.0 } else { 0.0 }],
        };
        pipe.queue
            .write_buffer(&pipe.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let Ok(frame) = pipe.surface.get_current_texture() else {
            return;
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = pipe
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("satya-map-enc"),
            });
        {
            let mut rpass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("satya-map-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.032,
                            g: 0.048,
                            b: 0.082,
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

            // Draw floor, cover, bulkheads, reach envelope, and LoS laser
            rpass.set_vertex_buffer(0, pipe.diorama_vb.slice(..));
            rpass.set_vertex_buffer(1, pipe.diorama_ib.slice(..));
            rpass.draw(0..pipe.diorama_vcount, 0..1);

            // Draw each `.glb` character miniature at its world cell transform
            let stride = std::mem::size_of::<GlbInstanceData>() as u64;
            for (idx, (preset, _)) in draws.iter().enumerate() {
                if let Some(mesh) = pipe.miniature_meshes.get(preset)
                    && mesh.vertex_count > 0
                {
                    let off = (idx as u64) * stride;
                    rpass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    rpass.set_vertex_buffer(1, pipe.miniature_ib.slice(off..off + stride));
                    rpass.draw(0..mesh.vertex_count, 0..1);
                }
            }
        }
        pipe.queue.submit(std::iter::once(enc.finish()));
        frame.present();
    }

    fn attach_map_listeners_if_needed() {
        let already = with_map_ctrl(|c| {
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
        let Some(canvas) = crate::wasm_client::dom::get_element("wgpu-map-canvas") else {
            with_map_ctrl(|c| c.listeners_attached = false);
            return;
        };
        let Some(win) = crate::wasm_client::dom::window() else {
            return;
        };

        let on_down = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            ev.prevent_default();
            let is_pan = ev.shift_key() || ev.button() == 1 || ev.button() == 2;
            let (cx, cy) = (ev.client_x(), ev.client_y());
            with_map_ctrl(|c| {
                c.drag_state = Some((is_pan, cx, cy, cx, cy, false));
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

        let on_wheel = Closure::<dyn FnMut(WheelEvent)>::new(move |ev: WheelEvent| {
            ev.prevent_default();
            let factor = if ev.delta_y() < 0.0 { 1.12 } else { 1.0 / 1.12 };
            with_map_ctrl(|c| {
                c.camera.zoom_by_factor(factor);
                render_map_frame(c);
            });
            crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
        });
        let _ = canvas.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
        on_wheel.forget();

        let on_canvas_hover = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            let cam_opt = with_map_ctrl(|c| {
                if c.drag_state.is_some() {
                    None
                } else {
                    Some(c.camera)
                }
            });
            let Some(cam) = cam_opt else {
                return;
            };
            if let Some((mx, my)) = pick_metric_point_from_mouse(&ev, &cam) {
                crate::wasm_client::map_hover_inspect::satya_hover_map_point(
                    mx,
                    my,
                    ev.client_x() as f32,
                    ev.client_y() as f32,
                );
            } else {
                crate::wasm_client::map_hover_inspect::satya_leave_map_hover();
            }
        });
        let _ = canvas
            .add_event_listener_with_callback("mousemove", on_canvas_hover.as_ref().unchecked_ref());
        on_canvas_hover.forget();

        let on_canvas_leave = Closure::<dyn FnMut(MouseEvent)>::new(move |_ev: MouseEvent| {
            crate::wasm_client::map_hover_inspect::satya_leave_map_hover();
        });
        let _ = canvas
            .add_event_listener_with_callback("mouseleave", on_canvas_leave.as_ref().unchecked_ref());
        on_canvas_leave.forget();

        let on_win_move = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            let panned = with_map_ctrl(|c| {
                let Some((is_pan, sx, sy, lx, ly, mut moved)) = c.drag_state else {
                    return false;
                };
                let (cx, cy) = (ev.client_x(), ev.client_y());
                if (cx - sx).abs() > 4 || (cy - sy).abs() > 4 {
                    moved = true;
                }
                let (dx, dy) = ((cx - lx) as f32, (cy - ly) as f32);
                c.drag_state = Some((is_pan, sx, sy, cx, cy, moved));
                if !moved || (dx.abs() < 0.5 && dy.abs() < 0.5) {
                    return false;
                }
                if is_pan || c.camera.orthographic {
                    let max_pan = crate::wasm_client::map_3d::with_map_ui(|ui| {
                        let (w, h) = ui.map.board_size_m();
                        w.max(h) * 0.55 + 6.0
                    });
                    c.camera.pan_by_screen_delta(-dx * 0.032, dy * 0.032, max_pan);
                    render_map_frame(c);
                    return true;
                }
                c.camera.orbit_by(-dx * 0.42, dy * 0.32);
                render_map_frame(c);
                false
            });
            if panned {
                crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
            }
        });
        let _ =
            win.add_event_listener_with_callback("mousemove", on_win_move.as_ref().unchecked_ref());
        on_win_move.forget();

        let on_up = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            let clicked_cam = with_map_ctrl(|c| {
                let st = c.drag_state.take();
                match st {
                    Some((false, _, _, _, _, false)) if ev.button() == 0 => Some(c.camera),
                    _ => None,
                }
            });
            if let Some(cam) = clicked_cam
                && let Some((mx, my)) = pick_metric_point_from_mouse(&ev, &cam)
            {
                crate::wasm_client::map_edge_edit::satya_click_map_point(mx, my);
            }
        });
        let _ = win.add_event_listener_with_callback("mouseup", on_up.as_ref().unchecked_ref());
        on_up.forget();
    }

    pub fn ensure_map_wgpu_initialized_and_render() {
        attach_map_listeners_if_needed();
        let should_init = with_map_ctrl(|c| {
            if c.pipeline.is_some() {
                render_map_frame(c);
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
            let res = init_map_pipeline().await;
            with_map_ctrl(|c| {
                c.initializing = false;
                match res {
                    Ok(pipe) => {
                        c.pipeline = Some(pipe);
                        c.scene_dirty = true;
                        // The user may have switched to `Mode: 2D Battlemap` while the adapter was initializing.
                        if !with_map_ui(|ui| ui.view_battlemap) {
                            crate::wasm_client::dom::set_display("wgpu-map-canvas-wrap", "block");
                            crate::wasm_client::dom::set_display("tabletop-map-svg-fallback", "none");
                            render_map_frame(c);
                        }
                    }
                    Err(_err) => {
                        crate::wasm_client::dom::set_display("wgpu-map-canvas-wrap", "none");
                        crate::wasm_client::dom::set_display("tabletop-map-svg-fallback", "block");
                        crate::wasm_client::map_2d_art::sync_battlemap_svg_host();
                    }
                }
            });
        });
    }

    async fn init_map_pipeline() -> Result<MapDioramaPipeline, String> {
        let (_canvas, surface, device, queue, surface_config) =
            init_wgpu_canvas_context("wgpu-map-canvas").await?;
        let depth_view = create_depth_view(
            &device,
            surface_config.width,
            surface_config.height,
            "satya-map-depth",
        );
        let (uniform_buffer, bind_group, render_pipeline) =
            create_shared_diorama_pipeline(&device, surface_config.format, "satya-map");

        let diorama_vb = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-map-empty-vb"),
            size: 64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let diorama_ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-map-world-ib"),
            size: std::mem::size_of::<GlbInstanceData>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let miniature_ib = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("satya-map-mini-ib"),
            size: (std::mem::size_of::<GlbInstanceData>() * 16) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(MapDioramaPipeline {
            device,
            queue,
            surface,
            surface_config,
            depth_view,
            uniform_buffer,
            bind_group,
            render_pipeline,
            diorama_vb,
            diorama_vcount: 0,
            diorama_ib,
            miniature_meshes: HashMap::new(),
            cached_custom_glb_gen: 0,
            miniature_ib,
        })
    }
}

pub fn mark_map_wgpu_dirty() {
    #[cfg(target_arch = "wasm32")]
    map_wgpu_runtime::with_map_ctrl(|c| c.scene_dirty = true);
}

pub fn refresh_map_wgpu_if_active() {
    #[cfg(target_arch = "wasm32")]
    if crate::wasm_client::dom::window().is_some()
        && !crate::wasm_client::map_3d::with_map_ui(|ui| ui.view_battlemap)
    {
        map_wgpu_runtime::ensure_map_wgpu_initialized_and_render();
    }
}

/// Toggles between **3D Perspective Diorama** (`Orbit/Pan/Zoom`) and **2D Top-Down Orthographic** tactical mode.
#[wasm_bindgen]
pub fn satya_toggle_map_projection() {
    #[cfg(target_arch = "wasm32")]
    map_wgpu_runtime::with_map_ctrl(|c| {
        c.camera.orthographic = !c.camera.orthographic;
        if c.camera.orthographic {
            c.camera.yaw_deg = 0.0;
            crate::wasm_client::dom::set_text("btn-map-toggle-proj", "View: 2D Top-Down");
        } else {
            c.camera.yaw_deg = -24.0;
            c.camera.pitch_deg = 52.0;
            crate::wasm_client::dom::set_text("btn-map-toggle-proj", "View: 3D Diorama");
        }
        map_wgpu_runtime::render_map_frame(c);
    });
}

/// Toggles wall cutaway height (`0.58m` tactical cutaway vs. `2.35m` full bulkhead height).
#[wasm_bindgen]
pub fn satya_toggle_map_wall_cutaway() {
    let is_cut = crate::wasm_client::map_3d::with_map_ui_mut(|ui| {
        ui.wall_cutaway = !ui.wall_cutaway;
        ui.wall_cutaway
    });
    crate::wasm_client::dom::set_text(
        "btn-map-wall-cutaway",
        if is_cut { "Walls: Cutaway" } else { "Walls: Full 2.4m" },
    );
    mark_map_wgpu_dirty();
    refresh_map_wgpu_if_active();
}

/// Returns `(zoom, pan_xz)` for the live WebGPU tactical camera (`(1.0, [0.0, 0.0])` on host targets).
#[must_use]
pub fn current_3d_map_zoom_and_pan() -> (f32, [f32; 2]) {
    #[cfg(target_arch = "wasm32")]
    {
        map_wgpu_runtime::with_map_ctrl(|c| (c.camera.zoom, c.camera.pan))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        (1.0, [0.0, 0.0])
    }
}

/// Applies multiplicative zoom factor to the WebGPU 3D/Ortho tactical map camera and re-renders.
pub fn zoom_3d_map_camera_by(factor: f32) {
    #[cfg(target_arch = "wasm32")]
    map_wgpu_runtime::with_map_ctrl(|c| {
        c.camera.zoom_by_factor(factor);
        map_wgpu_runtime::render_map_frame(c);
    });
    #[cfg(not(target_arch = "wasm32"))]
    let _ = factor;
}

/// Shifts the 3D/Ortho camera pan offset when a continuation strip is stitched so world view stays anchored.
pub fn shift_3d_camera_on_stitch(
    old_size_m: (f32, f32),
    new_size_m: (f32, f32),
    dir: satya_schema::MapContinuationDir,
    nudge_into_new: bool,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let added_w = (new_size_m.0 - old_size_m.0).max(0.0);
        let added_h = (new_size_m.1 - old_size_m.1).max(0.0);
        let (shift_x, shift_z) = match dir {
            satya_schema::MapContinuationDir::West => (added_w, 0.0),
            satya_schema::MapContinuationDir::North => (0.0, added_h),
            _ => (0.0, 0.0),
        };
        let (nudge_x, nudge_z) = if nudge_into_new {
            let (ux, uy) = dir.unit_step();
            (ux as f32 * 5.5, uy as f32 * 4.5)
        } else {
            (0.0, 0.0)
        };
        let max_pan = (new_size_m.0.max(new_size_m.1) * 0.55 + 6.0).max(24.0);
        map_wgpu_runtime::with_map_ctrl(|c| {
            c.camera.pan[0] = (c.camera.pan[0] + shift_x + 0.5 * (old_size_m.0 - new_size_m.0) + nudge_x)
                .clamp(-max_pan, max_pan);
            c.camera.pan[1] = (c.camera.pan[1] + shift_z + 0.5 * (old_size_m.1 - new_size_m.1) + nudge_z)
                .clamp(-max_pan, max_pan);
            map_wgpu_runtime::render_map_frame(c);
        });
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (old_size_m, new_size_m, dir, nudge_into_new);
}

/// Resets the 3D tactical map camera to overview-fit `board_size_m` while preserving ortho mode.
pub fn reset_3d_map_camera_for_board(board_size_m: (f32, f32)) {
    let fit_zoom = OrbitOrthoCamera::fit_zoom_for_board(board_size_m);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = fit_zoom;
    #[cfg(target_arch = "wasm32")]
    map_wgpu_runtime::with_map_ctrl(|c| {
        let ortho = c.camera.orthographic;
        c.camera = OrbitOrthoCamera::tactical_map_default();
        c.camera.zoom = fit_zoom;
        c.camera.orthographic = ortho;
        if ortho {
            c.camera.yaw_deg = 0.0;
        }
        map_wgpu_runtime::render_map_frame(c);
    });
}

/// Resets the tactical map camera orbit, pan, and zoom (both 2D SVG and 3D WebGPU) to overview framing.
#[wasm_bindgen]
pub fn satya_reset_map_camera() {
    let board = crate::wasm_client::map_3d::with_map_ui_mut(|ui| {
        let board = ui.map.board_size_m();
        ui.camera_2d.reset_for_board(board);
        board
    });
    reset_3d_map_camera_for_board(board);
    crate::wasm_client::map_2d_camera::sync_map_camera_and_edge_controls_dom();
}

#[cfg(test)]
mod scale_camera_tests {
    use super::OrbitOrthoCamera;

    #[test]
    fn orbit_ortho_fit_zoom_scales_from_legacy_to_massive_stitched_boards() {
        assert!((OrbitOrthoCamera::fit_zoom_for_board((20.0, 14.0)) - 1.0).abs() < 1e-6);
        let z_64x48 = OrbitOrthoCamera::fit_zoom_for_board((64.0, 48.0));
        assert!((z_64x48 - (14.0 / 48.0)).abs() < 1e-5, "z_64x48={z_64x48}");
        let z_240x180 = OrbitOrthoCamera::fit_zoom_for_board((240.0, 180.0));
        assert!(
            (z_240x180 - (14.0 / 180.0)).abs() < 1e-5 && z_240x180 >= 0.07,
            "z_240x180={z_240x180}"
        );
    }
}
