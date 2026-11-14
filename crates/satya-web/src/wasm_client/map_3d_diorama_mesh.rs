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

/// Builds the static/overlay tactical diorama vertex list for `ui`.
#[must_use]
pub fn build_tactical_diorama_vertices(ui: &TacticalMapUiState) -> Vec<GlbVertex> {
    let w = ui.map.grid_width.max(4);
    let h = ui.map.grid_height.max(4);
    let cell_m = ui.map.cell_size_m();
    let half_w_m = (w as f32) * cell_m * 0.5;
    let half_h_m = (h as f32) * cell_m * 0.5;

    let mut verts = Vec::with_capacity((w * h * 18) as usize);

    // 1. Continuous 1.0m-gridded tactical deck floor (`zone_id = -2.0` grass / `-1.0` interior deck)
    let active_floor = ui.map.active_floor;
    let preset_code = ui.map.theme_preset.as_deref().unwrap_or_default();
    let (deck_rgba, deck_zone_id) = if active_floor < 0 {
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
            "residential_apartment" | "corner_bar" | "modern_interior" | "bar_interior" => {
                ([0.23, 0.17, 0.12, 0.0], -1.0_f32)
            }
            _ => ([0.075, 0.115, 0.185, 0.0], -1.0_f32),
        }
    };
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
                && satya_engine::tactical_map_geom::prop_contains_point(p, xm, ym, false)
        }) {
            1.48
        } else if matches!(
            satya_engine::map_surface_geom::topmost_surface_at_m(&ui.map.surfaces, xm, ym)
                .map(|s| s.kind),
            Some(satya_schema::SurfaceFeatureKind::BridgeDeck)
        ) {
            0.12
        } else {
            0.0
        }
    };

    // 2. Vector walls / doors / windows and 360°-rotated 3D props
    crate::wasm_client::map_3d_dressing_geom::push_dressing_geometry(&mut verts, ui, wall_h, door_h);

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
        push_box(&mut verts, sx, sz, sy, sy + 0.024, 0.42, 0.42, [0.96, 0.78, 0.14], 0.0);
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

    verts
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
        ui.map.props.push(satya_engine::map_dressing::make_prop_at_cm(
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
}
