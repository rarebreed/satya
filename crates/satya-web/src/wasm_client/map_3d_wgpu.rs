//! Browser WebGPU (`wgpu`) 2.5D/3D Tactical Map Diorama & 2D Top-Down Orthographic Renderer
//! (`#wgpu-map-canvas`), Procedural Terrain/Cover/Cutaway-Bulkhead Geometry, and `.glb` Miniatures.

#[cfg(target_arch = "wasm32")]
use crate::wasm_client::dice_3d::parse_hex_color;
use crate::wasm_client::glb_model::GlbVertex;
#[cfg(target_arch = "wasm32")]
use crate::wasm_client::map_3d::{
    TacticalMapUiState, format_map_tactical_hud_html, grid_cell_to_world_xz,
    handle_map_grid_cell_clicked, resolve_token_visual, with_map_ui, with_map_ui_mut,
    world_xz_to_grid_cell,
};
#[cfg(target_arch = "wasm32")]
use crate::wasm_client::model_3d_wgpu::{
    DioramaUniforms, GlbInstanceData, OrbitOrthoCamera, SHARED_GLB_DIORAMA_WGSL,
    resolve_glb_model, shared_glb_vertex_buffer_layouts,
};
#[cfg(target_arch = "wasm32")]
use satya_schema::{CoverGrade, MessageVisibility, TacticalTileKind};
use wasm_bindgen::prelude::*;

fn push_quad(verts: &mut Vec<GlbVertex>, p: [[f32; 3]; 4], col: [f32; 4], zone_id: f32) {
    let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
    let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    let nx = u[1] * v[2] - u[2] * v[1];
    let ny = u[2] * v[0] - u[0] * v[2];
    let nz = u[0] * v[1] - u[1] * v[0];
    let len = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
    let n = [nx / len, ny / len, nz / len];
    let bary = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for tri in [[0_usize, 1, 2], [0, 2, 3]] {
        for (ci, &vi) in tri.iter().enumerate() {
            verts.push(GlbVertex {
                position: p[vi],
                normal: n,
                color: col,
                bary_zone: [bary[ci][0], bary[ci][1], bary[ci][2], zone_id],
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn push_box(
    verts: &mut Vec<GlbVertex>,
    cx: f32,
    cz: f32,
    y0: f32,
    y1: f32,
    hx: f32,
    hz: f32,
    rgb: [f32; 3],
    zone_id: f32,
) {
    let col = [rgb[0], rgb[1], rgb[2], 0.0];
    let b0 = [cx - hx, y0, cz - hz];
    let b1 = [cx + hx, y0, cz - hz];
    let b2 = [cx + hx, y0, cz + hz];
    let b3 = [cx - hx, y0, cz + hz];
    let t0 = [cx - hx, y1, cz - hz];
    let t1 = [cx + hx, y1, cz - hz];
    let t2 = [cx + hx, y1, cz + hz];
    let t3 = [cx - hx, y1, cz + hz];
    push_quad(verts, [b3, b2, t2, t3], col, zone_id);
    push_quad(verts, [b1, b0, t0, t1], col, zone_id);
    push_quad(verts, [b2, b1, t1, t2], col, zone_id);
    push_quad(verts, [b0, b3, t3, t0], col, zone_id);
    push_quad(verts, [t3, t2, t1, t0], col, zone_id);
}

#[cfg(target_arch = "wasm32")]
fn push_beam(
    verts: &mut Vec<GlbVertex>,
    start_xz: [f32; 2],
    end_xz: [f32; 2],
    y: f32,
    half_w: f32,
    rgb: [f32; 3],
) {
    let dx = end_xz[0] - start_xz[0];
    let dz = end_xz[1] - start_xz[1];
    let len = (dx * dx + dz * dz).sqrt();
    if len < 0.05 {
        return;
    }
    let px = (-dz / len) * half_w;
    let pz = (dx / len) * half_w;
    let p0 = [start_xz[0] - px, y, start_xz[1] - pz];
    let p1 = [start_xz[0] + px, y, start_xz[1] + pz];
    let p2 = [end_xz[0] + px, y, end_xz[1] + pz];
    let p3 = [end_xz[0] - px, y, end_xz[1] - pz];
    push_quad(verts, [p0, p3, p2, p1], [rgb[0], rgb[1], rgb[2], 0.0], 0.0);
}

/// Builds the static/overlay tactical diorama vertex list for `ui`.
#[cfg(target_arch = "wasm32")]
fn build_tactical_diorama_vertices(ui: &TacticalMapUiState) -> Vec<GlbVertex> {
    let w = ui.map.grid_width.max(4);
    let h = ui.map.grid_height.max(4);
    let cell_m = ui.map.cell_size_m();
    let half_w_m = (w as f32) * cell_m * 0.5;
    let half_h_m = (h as f32) * cell_m * 0.5;

    let mut verts = Vec::with_capacity((w * h * 18) as usize);

    // 1. Continuous 1.0m-gridded tactical deck floor (`zone_id = -1.0` enables WGSL 1m/5m grid lines)
    push_quad(
        &mut verts,
        [
            [-half_w_m, 0.0, -half_h_m],
            [-half_w_m, 0.0, half_h_m],
            [half_w_m, 0.0, half_h_m],
            [half_w_m, 0.0, -half_h_m],
        ],
        [0.075, 0.115, 0.185, 0.0],
        -1.0,
    );
    // Subtle outer table rim
    push_box(
        &mut verts,
        0.0,
        0.0,
        -0.14,
        -0.005,
        half_w_m + 0.12,
        half_h_m + 0.12,
        [0.05, 0.08, 0.13],
        0.0,
    );

    let wall_h = if ui.wall_cutaway { 0.58 } else { 2.35 };
    let door_h = if ui.wall_cutaway { 0.68 } else { 2.25 };

    // 2. Per-cell terrain, cover crates (`0.90m`), bulkheads, doors, hazards, and elevated catwalks
    for gy in 0..h {
        for gx in 0..w {
            let tile = ui.map.tile_at(gx, gy);
            if tile == TacticalTileKind::Floor {
                continue;
            }
            let [wx, wz] = grid_cell_to_world_xz(gx, gy, w, h, cell_m);
            let hc = cell_m * 0.48;
            match tile {
                TacticalTileKind::Floor => {}
                TacticalTileKind::HalfCover => {
                    push_box(&mut verts, wx, wz, 0.0, 0.86, hc * 0.86, hc * 0.86, [0.16, 0.34, 0.52], 0.0);
                    push_box(&mut verts, wx, wz, 0.86, 0.91, hc * 0.90, hc * 0.90, [0.22, 0.68, 0.92], 0.0);
                }
                TacticalTileKind::FullWall => {
                    push_box(&mut verts, wx, wz, 0.0, wall_h, hc, hc, [0.23, 0.29, 0.38], 0.0);
                    push_box(&mut verts, wx, wz, wall_h, wall_h + 0.04, hc * 0.94, hc * 0.94, [0.36, 0.45, 0.58], 0.0);
                }
                TacticalTileKind::DoorClosed => {
                    push_box(&mut verts, wx, wz, 0.0, door_h, hc * 0.92, hc * 0.42, [0.78, 0.42, 0.09], 0.0);
                }
                TacticalTileKind::DoorOpen => {
                    push_box(&mut verts, wx - hc * 0.78, wz, 0.0, 0.35, hc * 0.16, hc * 0.45, [0.12, 0.65, 0.48], 0.0);
                    push_box(&mut verts, wx + hc * 0.78, wz, 0.0, 0.35, hc * 0.16, hc * 0.45, [0.12, 0.65, 0.48], 0.0);
                }
                TacticalTileKind::HazardZone => {
                    push_box(&mut verts, wx, wz, 0.004, 0.032, hc * 0.92, hc * 0.92, [0.68, 0.14, 0.24], 0.0);
                }
                TacticalTileKind::ElevatedPlatform => {
                    push_box(&mut verts, wx, wz, 0.0, 1.48, hc, hc, [0.15, 0.21, 0.31], -1.0);
                }
            }
        }
    }

    // 3. Movement gait reachable cell pads (`y = 0.018m`)
    for r in &ui.reachable_cells {
        let [wx, wz] = grid_cell_to_world_xz(r.x, r.y, w, h, cell_m);
        let base_y = if ui.map.tile_at(r.x, r.y) == TacticalTileKind::ElevatedPlatform {
            1.49
        } else {
            0.016
        };
        let pad = cell_m * 0.41;
        push_box(
            &mut verts,
            wx,
            wz,
            base_y,
            base_y + 0.018,
            pad,
            pad,
            [0.09, 0.58, 0.42],
            0.0,
        );
    }

    // 4. Selected miniature ring & hovered cell reticle
    if let Some(sel_id) = ui.selected_entity_id.as_deref()
        && let Some(tok) = ui.map.tokens.iter().find(|t| t.entity_id == sel_id)
    {
        let [sx, sz] = grid_cell_to_world_xz(tok.x, tok.y, w, h, cell_m);
        let sy = if ui.map.tile_at(tok.x, tok.y) == TacticalTileKind::ElevatedPlatform {
            1.49
        } else {
            0.022
        };
        push_box(&mut verts, sx, sz, sy, sy + 0.024, cell_m * 0.46, cell_m * 0.46, [0.96, 0.78, 0.14], 0.0);
    }
    if let Some((hx, hy)) = ui.hovered_cell {
        let [wx, wz] = grid_cell_to_world_xz(hx, hy, w, h, cell_m);
        let hy_m = if ui.map.tile_at(hx, hy) == TacticalTileKind::ElevatedPlatform {
            1.50
        } else {
            0.025
        };
        push_box(&mut verts, wx, wz, hy_m, hy_m + 0.022, cell_m * 0.44, cell_m * 0.44, [0.22, 0.78, 0.98], 0.0);
    }

    // 5. 3D Line-of-Sight & Cover laser ribbon (`y = 1.05m` chest/rifle sightline)
    if let Some(los) = ui.last_los.as_ref()
        && los.distance_m > 0.05
    {
        let a_xz = grid_cell_to_world_xz(los.attacker_xy.0, los.attacker_xy.1, w, h, cell_m);
        let t_xz = grid_cell_to_world_xz(los.target_xy.0, los.target_xy.1, w, h, cell_m);
        let beam_rgb = match los.cover_grade {
            CoverGrade::Clear => [0.14, 0.86, 0.96],
            CoverGrade::LowCover => [0.98, 0.76, 0.14],
            CoverGrade::HighCover => [0.98, 0.48, 0.16],
            CoverGrade::Blocked => [0.96, 0.22, 0.36],
        };
        push_beam(&mut verts, a_xz, t_xz, 1.05, 0.045, beam_rgb);
    }

    verts
}

#[cfg(target_arch = "wasm32")]
mod map_wgpu_runtime {
    use super::*;
    use crate::wasm_client::glb_model::LoadedGlbModel;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::spawn_local;
    use web_sys::{HtmlCanvasElement, MouseEvent, WheelEvent};

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

    fn create_depth_view(device: &wgpu::Device, w: u32, h: u32) -> wgpu::TextureView {
        let tex = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("satya-map-depth"),
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

    fn ensure_miniature_gpu_mesh(pipe: &mut MapDioramaPipeline, preset: &str) {
        if preset != "custom_glb" && pipe.miniature_meshes.contains_key(preset) {
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
    }

    fn pick_grid_cell_from_mouse(ev: &MouseEvent, cam: &OrbitOrthoCamera) -> Option<(i32, i32)> {
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
            world_xz_to_grid_cell(
                wx,
                wz,
                ui.map.grid_width,
                ui.map.grid_height,
                ui.map.cell_size_m(),
            )
        })
    }

    pub fn render_map_frame(ctrl: &mut MapDioramaController) {
        let Some(pipe) = ctrl.pipeline.as_mut() else {
            return;
        };

        if let Some(el) = crate::wasm_client::dom::get_element("wgpu-map-canvas")
            && let Ok(canvas) = el.dyn_into::<HtmlCanvasElement>()
        {
            let cw = canvas.client_width().max(320) as u32;
            let ch = canvas.client_height().max(280) as u32;
            if cw != pipe.surface_config.width || ch != pipe.surface_config.height {
                canvas.set_width(cw);
                canvas.set_height(ch);
                pipe.surface_config.width = cw;
                pipe.surface_config.height = ch;
                pipe.surface.configure(&pipe.device, &pipe.surface_config);
                pipe.depth_view = create_depth_view(&pipe.device, cw, ch);
            }
        }

        if ctrl.scene_dirty {
            let verts = with_map_ui(|ui| {
                let max_dim = ui.map.grid_width.max(ui.map.grid_height) as f32 * ui.map.cell_size_m();
                let _ = max_dim;
                build_tactical_diorama_vertices(ui)
            });
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
        let (cell_m, draws): (f32, Vec<(String, GlbInstanceData)>) = with_map_ui(|ui| {
            let cm = ui.map.cell_size_m();
            let mut list = Vec::with_capacity(ui.map.tokens.len());
            for tok in &ui.map.tokens {
                let (preset, hex_col, facing_deg) = resolve_token_visual(&ui.map, tok);
                let (_, rgb) = parse_hex_color(&hex_col, [0.22, 0.74, 0.98]);
                let [wx, wz] = grid_cell_to_world_xz(
                    tok.x,
                    tok.y,
                    ui.map.grid_width,
                    ui.map.grid_height,
                    cm,
                );
                let base_y = if ui.map.tile_at(tok.x, tok.y) == TacticalTileKind::ElevatedPlatform {
                    1.48
                } else {
                    0.0
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
            (cm, list)
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
        let mode_f = crate::wasm_client::model_3d_wgpu::shader_mode_code(&ctrl.shader_mode);
        let uniforms = DioramaUniforms {
            view_proj,
            camera_pos: [eye[0], eye[1], eye[2], 1.0],
            mode_params: [mode_f, cell_m, 0.42, 0.0],
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
            let cell = pick_grid_cell_from_mouse(&ev, &cam);
            let changed = with_map_ui_mut(|ui| {
                if ui.hovered_cell != cell {
                    ui.hovered_cell = cell;
                    ui.recompute_overlays();
                    true
                } else {
                    false
                }
            });
            if changed {
                let hud = with_map_ui(format_map_tactical_hud_html);
                if let Some(el) = crate::wasm_client::dom::get_element("tabletop-map-los-hud") {
                    el.set_inner_html(&hud);
                }
                with_map_ctrl(|c| {
                    c.scene_dirty = true;
                    render_map_frame(c);
                });
            }
        });
        let _ = canvas
            .add_event_listener_with_callback("mousemove", on_canvas_hover.as_ref().unchecked_ref());
        on_canvas_hover.forget();

        let on_win_move = Closure::<dyn FnMut(MouseEvent)>::new(move |ev: MouseEvent| {
            with_map_ctrl(|c| {
                let Some((is_pan, sx, sy, lx, ly, mut moved)) = c.drag_state else {
                    return;
                };
                let (cx, cy) = (ev.client_x(), ev.client_y());
                if (cx - sx).abs() > 4 || (cy - sy).abs() > 4 {
                    moved = true;
                }
                let dx = (cx - lx) as f32;
                let dy = (cy - ly) as f32;
                c.drag_state = Some((is_pan, sx, sy, cx, cy, moved));
                if !moved || (dx.abs() < 0.5 && dy.abs() < 0.5) {
                    return;
                }
                if is_pan || c.camera.orthographic {
                    c.camera.pan_by_screen_delta(-dx * 0.032, dy * 0.032, 24.0);
                } else {
                    c.camera.orbit_by(-dx * 0.42, dy * 0.32);
                }
                render_map_frame(c);
            });
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
                && let Some((gx, gy)) = pick_grid_cell_from_mouse(&ev, &cam)
            {
                handle_map_grid_cell_clicked(gx, gy);
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
                        crate::wasm_client::dom::set_display("wgpu-map-canvas-wrap", "block");
                        crate::wasm_client::dom::set_display("tabletop-map-svg-fallback", "none");
                        render_map_frame(c);
                    }
                    Err(_err) => {
                        crate::wasm_client::dom::set_display("wgpu-map-canvas-wrap", "none");
                        crate::wasm_client::dom::set_display("tabletop-map-svg-fallback", "block");
                    }
                }
            });
        });
    }

    async fn init_map_pipeline() -> Result<MapDioramaPipeline, String> {
        let Some(el) = crate::wasm_client::dom::get_element("wgpu-map-canvas") else {
            return Err("Canvas #wgpu-map-canvas not found".to_string());
        };
        let canvas: HtmlCanvasElement = el
            .dyn_into::<HtmlCanvasElement>()
            .map_err(|_| "Not a canvas".to_string())?;
        let width = canvas.client_width().max(320) as u32;
        let height = canvas.client_height().max(280) as u32;
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
            .ok_or_else(|| "WebGPU unavailable".to_string())?;
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("satya-map-device"),
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
            label: Some("satya-map-uniforms"),
            size: std::mem::size_of::<DioramaUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("satya-map-bgl"),
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
            label: Some("satya-map-bg"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("satya-map-wgsl"),
            source: wgpu::ShaderSource::Wgsl(SHARED_GLB_DIORAMA_WGSL.into()),
        });
        let playout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("satya-map-playout"),
            bind_group_layouts: &[&bgl],
            push_constant_ranges: &[],
        });
        let vbuffers = shared_glb_vertex_buffer_layouts();
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("satya-map-pipeline"),
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
    if crate::wasm_client::dom::window().is_some() {
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

/// Resets the tactical map camera orbit, pan, and zoom to the centered overview angle.
#[wasm_bindgen]
pub fn satya_reset_map_camera() {
    #[cfg(target_arch = "wasm32")]
    map_wgpu_runtime::with_map_ctrl(|c| {
        let ortho = c.camera.orthographic;
        c.camera = OrbitOrthoCamera::tactical_map_default();
        c.camera.orthographic = ortho;
        if ortho {
            c.camera.yaw_deg = 0.0;
        }
        map_wgpu_runtime::render_map_frame(c);
    });
}
