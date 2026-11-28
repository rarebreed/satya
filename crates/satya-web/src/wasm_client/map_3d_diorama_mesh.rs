//! Tactical diorama vertex builder (`map_3d_diorama_mesh`), companion to `map_3d_wgpu`.
//! Emits the 1.0m deck surface, wall/prop OBB dressing, continuous gait reach strips,
//! cursor/selected-token reticles, and vertical cover/LoS sightline beam.

use crate::wasm_client::glb_model::GlbVertex;
use crate::wasm_client::map_3d::{TacticalMapUiState, metric_to_world_xz};
use crate::wasm_client::wgpu_shared::{cross3, normalize3};
use satya_schema::{CoverGrade, PropKind};

pub fn push_quad(verts: &mut Vec<GlbVertex>, p: [[f32; 3]; 4], col: [f32; 4], zone_id: f32) {
    let u = [p[1][0] - p[0][0], p[1][1] - p[0][1], p[1][2] - p[0][2]];
    let v = [p[2][0] - p[0][0], p[2][1] - p[0][1], p[2][2] - p[0][2]];
    let n = normalize3(cross3(u, v));
    let bary = [[1.0_f32, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    for tri in [[0_usize, 1, 2], [0, 2, 3]] {
        for (ci, &vi) in tri.iter().enumerate() {
            verts.push(GlbVertex {
                position: p[vi],
                normal: n,
                color: col,
                bary_zone: [bary[ci][0], bary[ci][1], bary[ci][2], zone_id],
                uv: [-1.0, -1.0],
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

pub fn push_beam(
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

fn push_editor_overlay_3d(
    verts: &mut Vec<GlbVertex>,
    ui: &TacticalMapUiState,
    bw_m: f32,
    bh_m: f32,
    catwalk_elevation: &dyn Fn(f32, f32) -> f32,
) {
    use crate::wasm_client::map_3d_dressing_geom::push_obb;
    use crate::wasm_client::map_editor::{
        EditorSelection, EditorTool, SurfaceBrushShape, rasterize_wall_stroke_edges, with_editor,
    };
    with_editor(|ed| {
        if !ed.active {
            return;
        }
        match ed.selection {
            EditorSelection::Prop(idx) => {
                if let Some(p) = ui.map.props.get(idx) {
                    let fp = p.footprint_m();
                    let [cx, cz] = metric_to_world_xz(fp.cx_m, fp.cy_m, bw_m, bh_m);
                    let base_y = catwalk_elevation(fp.cx_m, fp.cy_m) + 0.022;
                    push_obb(
                        verts,
                        cx,
                        cz,
                        base_y,
                        base_y + 0.026,
                        fp.hx_m + 0.08,
                        fp.hy_m + 0.08,
                        fp.rot_deg,
                        [0.98, 0.80, 0.16],
                        0.0,
                    );
                }
            }
            EditorSelection::Surface(idx) => {
                if let Some(s) = ui.map.surfaces.get(idx) {
                    for pt in &s.points_cm {
                        let [vx, vz] = metric_to_world_xz(
                            pt.x_cm as f32 / 100.0,
                            pt.y_cm as f32 / 100.0,
                            bw_m,
                            bh_m,
                        );
                        push_box(
                            verts,
                            vx,
                            vz,
                            0.025,
                            0.09,
                            0.10,
                            0.10,
                            [0.76, 0.36, 0.96],
                            0.0,
                        );
                    }
                }
            }
            EditorSelection::None => {}
        }

        if ed.tool == EditorTool::AddProp
            && let Some((hx_m, hy_m)) = ui.hovered_point_m
        {
            let (px_m, py_m) = if ed.snap_half_m {
                ((hx_m * 2.0).round() * 0.5, (hy_m * 2.0).round() * 0.5)
            } else {
                (hx_m, hy_m)
            };
            let p = satya_maps::map_dressing::make_prop_at_cm(
                ed.prop_kind,
                (px_m * 100.0).round() as i32,
                (py_m * 100.0).round() as i32,
                ed.prop_rotation_deg,
                None,
                0,
            );
            let fp = p.footprint_m();
            let [cx, cz] = metric_to_world_xz(fp.cx_m, fp.cy_m, bw_m, bh_m);
            let base_y = catwalk_elevation(fp.cx_m, fp.cy_m) + 0.024;
            push_obb(
                verts,
                cx,
                cz,
                base_y,
                base_y + 0.026,
                fp.hx_m,
                fp.hy_m,
                fp.rot_deg,
                [0.20, 0.82, 0.98],
                0.0,
            );
        } else if ed.tool == EditorTool::PaintWall
            && let (Some(st), Some(en)) = (ed.wall_drag_start, ed.wall_drag_cur)
        {
            let cell_m = ui.map.cell_size_m().max(0.25);
            for r in rasterize_wall_stroke_edges(&ui.map, st, en) {
                let (xm, ym, hx, hz) = match r.side {
                    satya_schema::EdgeSide::North => (
                        (r.x as f32 + 0.5) * cell_m,
                        r.y as f32 * cell_m,
                        cell_m * 0.5,
                        0.08,
                    ),
                    satya_schema::EdgeSide::West => (
                        r.x as f32 * cell_m,
                        (r.y as f32 + 0.5) * cell_m,
                        0.08,
                        cell_m * 0.5,
                    ),
                };
                let [wx, wz] = metric_to_world_xz(xm, ym, bw_m, bh_m);
                push_box(verts, wx, wz, 0.0, 0.68, hx, hz, [0.98, 0.72, 0.16], 0.0);
            }
        } else if ed.tool == EditorTool::PaintSurface {
            if ed.surface_shape == SurfaceBrushShape::Rect
                && let (Some(a), Some(b)) = (ed.surface_rect_start_cm, ed.surface_rect_cur_cm)
            {
                let x_m = ((a.0 + b.0) as f32 * 0.5) / 100.0;
                let y_m = ((a.1 + b.1) as f32 * 0.5) / 100.0;
                let hx = (((a.0 - b.0).abs().max(20)) as f32 * 0.5) / 100.0;
                let hz = (((a.1 - b.1).abs().max(20)) as f32 * 0.5) / 100.0;
                let [wx, wz] = metric_to_world_xz(x_m, y_m, bw_m, bh_m);
                push_box(verts, wx, wz, 0.022, 0.055, hx, hz, [0.76, 0.36, 0.96], 0.0);
            } else {
                for (i, pt) in ed.surface_draft_pts_cm.iter().enumerate() {
                    let [vx, vz] = metric_to_world_xz(
                        pt.x_cm as f32 / 100.0,
                        pt.y_cm as f32 / 100.0,
                        bw_m,
                        bh_m,
                    );
                    let col = if i == 0 {
                        [0.98, 0.80, 0.16]
                    } else {
                        [0.76, 0.36, 0.96]
                    };
                    push_box(verts, vx, vz, 0.02, 0.12, 0.12, 0.12, col, 0.0);
                }
            }
        }
    });
}

/// Builds the layered tactical diorama vertex buffer `(verts, opaque_vertex_count)`, placing
/// translucent overhanging tree/gazebo canopies after opaque geometry so miniatures draw in between.
#[must_use]
pub fn build_tactical_diorama_layered(ui: &TacticalMapUiState) -> (Vec<GlbVertex>, u32) {
    let w = ui.map.grid_width.max(4);
    let h = ui.map.grid_height.max(4);
    let cell_m = ui.map.cell_size_m();
    let half_w_m = (w as f32) * cell_m * 0.5;
    let half_h_m = (h as f32) * cell_m * 0.5;

    let mut verts = Vec::with_capacity((w * h * 18) as usize);

    // 1. Continuous 1.0m-gridded tactical deck floor (`zone_id = -2.0` grass / `-1.0` interior deck)
    let active_floor = ui.map.active_floor;
    let preset_code = ui.map.theme_preset.as_deref().unwrap_or_default();
    let (mut deck_rgba, mut deck_zone_id) = if let Some(override_key) = ui
        .map
        .ground_floor_override
        .as_deref()
        .filter(|s| !s.trim().is_empty())
    {
        if let Some(spec) =
            crate::wasm_client::map_2d_art_textures::ground_override_texture_spec(override_key)
        {
            let rgb = satya_schema::parse_hex_rgb(spec.fallback_hex).unwrap_or([100, 108, 118]);
            let zid = match spec.pattern_id {
                "bm-grass" | "bm-grass-lush" | "bm-grass-dry" | "bm-leaf-tex" => -2.0_f32,
                "bm-dirt" | "bm-mulch" | "bm-carpet" => -4.0_f32,
                "bm-asphalt" | "bm-concrete" | "bm-stained-concrete" => -7.0_f32,
                "bm-clinical-vinyl" | "bm-linoleum-check" | "bm-terrazzo" | "bm-ceramic"
                | "bm-marble" | "bm-vinyl" => -8.0_f32,
                "bm-stone"
                | "bm-gravel"
                | "bm-gravel-pea"
                | "bm-gravel-slate"
                | "bm-brick"
                | "bm-herringbone-brick"
                | "bm-wood-deck" => -3.0_f32,
                _ => -1.0_f32,
            };
            (
                [
                    rgb[0] as f32 / 255.0,
                    rgb[1] as f32 / 255.0,
                    rgb[2] as f32 / 255.0,
                    0.0,
                ],
                zid,
            )
        } else {
            ([0.16, 0.18, 0.21, 0.0], -1.0_f32)
        }
    } else if active_floor < 0 {
        ([0.16, 0.18, 0.21, 0.0], -1.0_f32)
    } else if active_floor > 0 {
        ([0.23, 0.17, 0.12, 0.0], -1.0_f32)
    } else {
        match preset_code {
            "wooded_park"
            | "garden_estate"
            | "woodland_creek"
            | "roadside_ditch"
            | "park_grounds"
            | "suburban_neighborhood"
            | "rural_countryside"
            | "luxury_mansion" => ([0.24, 0.36, 0.17, 0.0], -2.0_f32),
            "urban_residential" | "projects_high_density" | "downtown_commercial" => {
                ([0.14, 0.16, 0.19, 0.0], -1.0_f32)
            }
            "residential_apartment"
            | "corner_bar"
            | "bistro_diner"
            | "fine_dining_restaurant"
            | "modern_interior"
            | "bar_interior" => {
                ([0.23, 0.17, 0.12, 0.0], -1.0_f32)
            }
            "fast_food_restaurant" => ([0.20, 0.21, 0.23, 0.0], -1.0_f32),
            _ => ([0.075, 0.115, 0.185, 0.0], -1.0_f32),
        }
    };
    if let Some([tr, tg, tb]) = ui.map.ground_tint_rgb {
        let tint = [tr as f32 / 255.0, tg as f32 / 255.0, tb as f32 / 255.0];
        deck_rgba[0] = deck_rgba[0] * 0.65 + tint[0] * 0.35;
        deck_rgba[1] = deck_rgba[1] * 0.65 + tint[1] * 0.35;
        deck_rgba[2] = deck_rgba[2] * 0.65 + tint[2] * 0.35;
        if deck_zone_id < -0.5 {
            deck_zone_id = -1.0;
        }
    }
    push_quad(
        &mut verts,
        [
            [-half_w_m, 0.0, -half_h_m],
            [-half_w_m, 0.0, half_h_m],
            [half_w_m, 0.0, half_h_m],
            [half_w_m, 0.0, -half_h_m],
        ],
        deck_rgba,
        deck_zone_id,
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
    let (bw_m, bh_m) = (half_w_m * 2.0, half_h_m * 2.0);
    let catwalk_elevation = |xm: f32, ym: f32| -> f32 {
        if ui.map.props.iter().any(|p| {
            p.kind == PropKind::ElevatedCatwalk
                && satya_maps::tactical_map_geom::prop_contains_point(p, xm, ym, false)
        }) {
            1.48
        } else if matches!(
            satya_maps::map_surface_geom::topmost_surface_at_m(&ui.map.surfaces, xm, ym)
                .map(|s| s.kind),
            Some(satya_schema::SurfaceFeatureKind::BridgeDeck)
        ) {
            0.12
        } else {
            0.0
        }
    };

    // 2. Vector walls / doors / windows and 360°-rotated opaque 3D props
    crate::wasm_client::map_3d_dressing_geom::push_dressing_geometry(
        &mut verts, ui, wall_h, door_h,
    );

    // 3. Continuous 0.25m gait reach strips (`y = 0.016m`)
    for s in &ui.reachable_strips {
        let mid_x_m = (s.x_min_m + s.x_max_m) * 0.5;
        let [wx, wz] = metric_to_world_xz(mid_x_m, s.y_m, bw_m, bh_m);
        let hx = ((s.x_max_m - s.x_min_m) * 0.5).max(0.05);
        let hz = (s.step_m * 0.46).max(0.05);
        let base_y = catwalk_elevation(mid_x_m, s.y_m).max(0.016);
        push_box(
            &mut verts,
            wx,
            wz,
            base_y,
            base_y + 0.018,
            hx,
            hz,
            [0.09, 0.58, 0.42],
            0.0,
        );
    }

    // 4. Selected miniature ring & continuous cursor hover reticle (active floor only)
    if let Some(sel_id) = ui.selected_entity_id.as_deref()
        && ui.map.token_floor(sel_id) == ui.map.active_floor
        && let Some(tok) = ui.map.tokens.iter().find(|t| t.entity_id == sel_id)
    {
        let (tx_m, ty_m) = ui.map.token_pos_m(tok);
        let [sx, sz] = metric_to_world_xz(tx_m, ty_m, bw_m, bh_m);
        let sy = catwalk_elevation(tx_m, ty_m) + 0.022;
        push_box(
            &mut verts,
            sx,
            sz,
            sy,
            sy + 0.024,
            0.42,
            0.42,
            [0.96, 0.78, 0.14],
            0.0,
        );
    }
    if let Some((hx_m, hy_m)) = ui.hovered_point_m {
        let [wx, wz] = metric_to_world_xz(hx_m, hy_m, bw_m, bh_m);
        let hy_m = catwalk_elevation(hx_m, hy_m) + 0.025;
        let col = if ui.interaction_mode == "edit_terrain" && ui.edit_prop_brush == "__erase__" {
            [0.95, 0.25, 0.38]
        } else {
            [0.22, 0.78, 0.98]
        };
        push_box(&mut verts, wx, wz, hy_m, hy_m + 0.022, 0.34, 0.34, col, 0.0);
    }

    // 4b. Interactive Battlemap Editor selection frame, AddProp ghost & wall/surface drafts
    push_editor_overlay_3d(&mut verts, ui, bw_m, bh_m, &catwalk_elevation);

    // 5. 3D Line-of-Sight & Cover laser ribbon (`y = 1.05m` chest/rifle sightline)
    if let Some(los) = ui.last_los.as_ref()
        && los.distance_m > 0.05
    {
        let a_xz = metric_to_world_xz(los.attacker_m.0, los.attacker_m.1, bw_m, bh_m);
        let t_xz = metric_to_world_xz(los.target_m.0, los.target_m.1, bw_m, bh_m);
        let blind_pen = ui
            .last_ballistic
            .as_ref()
            .is_some_and(|b| b.is_blind_fire && b.can_reach_target);
        let beam_rgb = match los.cover_grade {
            CoverGrade::Clear => [0.14, 0.86, 0.96],
            CoverGrade::Concealment => [0.18, 0.80, 0.38],
            CoverGrade::LowCover => [0.98, 0.76, 0.14],
            CoverGrade::HighCover => [0.98, 0.48, 0.16],
            CoverGrade::Blocked if blind_pen => [0.68, 0.32, 0.96],
            CoverGrade::Blocked => [0.96, 0.22, 0.36],
        };
        push_beam(&mut verts, a_xz, t_xz, 1.05, 0.045, beam_rgb);
    }

    let opaque_vcount = verts.len() as u32;
    crate::wasm_client::map_3d_props_extra::push_translucent_canopies_3d(&mut verts, ui);
    crate::wasm_client::map_3d_props_awakened::push_spatial_overlays_3d(&mut verts, ui);
    (verts, opaque_vcount)
}

/// Builds the static/overlay tactical diorama vertex list for `ui`.
#[must_use]
pub fn build_tactical_diorama_vertices(ui: &TacticalMapUiState) -> Vec<GlbVertex> {
    build_tactical_diorama_layered(ui).0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::glb_model::load_preset_glb_model;
    use crate::wasm_client::model_3d_wgpu::build_pedestal_and_model_vertices;

    #[test]
    fn test_build_tactical_diorama_vertices_cutaway_and_elevated_platform() {
        let mut ui = TacticalMapUiState {
            wall_cutaway: true,
            ..Default::default()
        };
        let verts_cut = build_tactical_diorama_vertices(&ui);
        assert!(!verts_cut.is_empty());
        assert!(verts_cut.len().is_multiple_of(3));

        ui.wall_cutaway = false;
        let verts_full = build_tactical_diorama_vertices(&ui);
        assert!(
            verts_full.len() >= verts_cut.len(),
            "Full walls emit window lintel geometry above cutaway height ({} vs {})",
            verts_full.len(),
            verts_cut.len()
        );

        let max_y_cut = verts_cut
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let max_y_full = verts_full
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_y_cut < max_y_full,
            "Cutaway walls (0.58m) must reduce max Y ({max_y_cut}) below full bulkheads (2.35m -> {max_y_full})"
        );
        assert!(
            (2.375..=2.405).contains(&max_y_full),
            "Full edge/cell walls must reach 2.38m..2.39m (got {max_y_full})"
        );

        // Verify isolated cutaway edge wall height (0.58m + 0.03m cap = 0.61m) and elevated catwalk (1.48m)
        ui.map.tiles.clear();
        ui.map.wall_edges.clear();
        ui.map.props.clear();
        ui.map.tokens.clear();
        ui.reachable_strips.clear();
        ui.hovered_point_m = None;
        ui.last_los = None;

        ui.map.wall_edges.push(satya_schema::MapWallEdge {
            x: 0,
            y: 0,
            side: satya_schema::EdgeSide::North,
            kind: satya_schema::WallEdgeKind::Wall,
            material: satya_schema::MaterialClass::Drywall,
        });
        ui.wall_cutaway = true;
        let wall_cut_verts = build_tactical_diorama_vertices(&ui);
        let wall_cut_max_y = wall_cut_verts
            .iter()
            .map(|v| v.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((wall_cut_max_y - 0.61).abs() < 1e-3);

        ui.map.wall_edges.clear();
        ui.map.props.push(satya_maps::map_dressing::make_prop_at_cm(
            satya_schema::PropKind::ElevatedCatwalk,
            200,
            200,
            0,
            None,
            1,
        ));
        let plat_verts = build_tactical_diorama_vertices(&ui);
        assert!(
            plat_verts
                .iter()
                .any(|v| (v.position[1] - 1.48).abs() < 1e-4 && (v.bary_zone[3] + 1.0).abs() < 1e-4),
            "ElevatedCatwalk must emit 1.48m gridded catwalk top geometry"
        );
    }

    #[test]
    fn test_layered_diorama_puts_translucent_canopies_after_opaque_vcount() {
        let mut ui = TacticalMapUiState {
            wall_cutaway: false,
            ..Default::default()
        };
        ui.map.props.push(satya_maps::map_dressing::make_prop_at_cm(
            PropKind::LargeTree,
            450,
            450,
            0,
            None,
            1,
        ));
        ui.map.props.push(satya_maps::map_dressing::make_prop_at_cm(
            PropKind::MediumTree,
            850,
            450,
            0,
            None,
            2,
        ));
        ui.map.props.push(satya_maps::map_dressing::make_prop_at_cm(
            PropKind::Gazebo,
            1300,
            700,
            0,
            None,
            3,
        ));
        let (verts_full, opaque_full) = build_tactical_diorama_layered(&ui);
        assert!(verts_full.len() as u32 > opaque_full);
        assert!(
            verts_full[opaque_full as usize..]
                .iter()
                .all(|v| (v.bary_zone[3] + 0.35).abs() < 1e-4)
        );
        assert!(
            verts_full[..opaque_full as usize]
                .iter()
                .all(|v| (v.bary_zone[3] + 0.35).abs() > 0.05)
        );

        ui.wall_cutaway = true;
        let (verts_cut, opaque_cut) = build_tactical_diorama_layered(&ui);
        assert_eq!(verts_cut.len() as u32, opaque_cut);
    }

    #[test]
    fn test_build_pedestal_and_model_vertices_prepends_gridded_turntable_quad() {
        let model = load_preset_glb_model("valkyrie_rig");
        let combined = build_pedestal_and_model_vertices(&model);
        assert_eq!(combined.len(), model.vertices.len() + 6);
        for v in &combined[0..6] {
            assert!((v.position[1] + 0.002).abs() < 1e-5);
            assert!((v.bary_zone[3] + 1.0).abs() < 1e-5);
        }
        assert_eq!(&combined[6..], model.vertices.as_slice());
    }

    #[test]
    fn wave6_awakened_props_and_translucent_spatial_overlays_render_cleanly() {
        use satya_maps::map_dressing::make_prop;
        use satya_maps::tactical_map::{SectorPreset, build_procedural_map_request};
        use satya_schema::{MapSurveillanceNode, MessageVisibility, PropKind};

        let mut map = build_procedural_map_request(
            "w6-3d",
            SectorPreset::CorporateServerBunker,
            None,
            24,
            18,
            91,
        );
        let awakened_kinds = [
            PropKind::CryoComputeCluster,
            PropKind::BioServerVat,
            PropKind::AirGappedTerminal,
            PropKind::BiometricTurnstile,
            PropKind::SecurityCameraMast,
            PropKind::DroneDockPad,
            PropKind::HvTransformer,
            PropKind::DieselGenerator,
            PropKind::BatteryBankWall,
            PropKind::WaterPurifierKiosk,
        ];
        for (i, &kind) in awakened_kinds.iter().enumerate() {
            map.props
                .push(make_prop(kind, 2 + (i as i32) * 2, 2, (i as i32 * 45) % 360, None, i as u32));
        }
        map.surveillance_nodes.push(MapSurveillanceNode {
            node_id: "cam-offline".to_string(),
            label: "Offline Cam".to_string(),
            node_kind_code: "DOME_CAMERA".to_string(),
            floor_index: map.active_floor,
            x_cm: 650,
            y_cm: 450,
            facing_deg: 90,
            fov_deg: 90,
            range_cm: 800,
            powered: false,
            visibility: MessageVisibility::Public,
        });

        let ui = TacticalMapUiState {
            map,
            wall_cutaway: true,
            ..Default::default()
        };
        let (verts, opaque_vcount) = build_tactical_diorama_layered(&ui);
        assert!(opaque_vcount > 0);
        assert!(
            verts.len() > opaque_vcount as usize,
            "expected translucent Faraday/surveillance vertices after opaque_vcount"
        );
        for v in &verts[opaque_vcount as usize..] {
            assert!(
                (v.bary_zone[3] + 0.35).abs() < 1e-4,
                "post-opaque vertex must carry TRANSLUCENT_CANOPY_ZONE (-0.35), got {}",
                v.bary_zone[3]
            );
            assert!(v.position.iter().all(|c| c.is_finite()));
        }
    }
}
