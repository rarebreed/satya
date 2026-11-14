//! # Tactical Map Legacy Migration, Connectivity Verification & Point Hover Inspection
//!
//! Companion module to [`crate::tactical_map_geom`] housing:
//! - [`migrate_legacy_tiles_to_vector`] for upgrading legacy saved maps with populated `tiles`,
//! - [`all_walkable_areas_connected`] for `0.5m` lattice traversability verification,
//! - [`inspect_map_point`] (`MapHoverCategory`, `MapHoverInspection`) powering the UI hover status
//!   bar and inspect tooltip across 2D SVG and 3D WebGPU viewports.

use serde::{Deserialize, Serialize};

use satya_schema::{
    CoverGrade, EdgeSide, GmMapUpdateRequest, MapPropPlacement, MapWallEdge, MaterialClass,
    MessageVisibility, PropKind, TacticalTileKind, TokenPlacement, WallEdgeKind,
};

use crate::map_dressing::{make_prop, prop_spec};
use crate::tactical_map_geom::{
    distance_point_to_segment, edge_endpoints_m, prop_contains_point,
    vehicle_point_is_engine_bay, EDGE_PICK_HALF_THICK_M, TOKEN_PICK_RADIUS_M, TOKEN_RADIUS_M,
};
use crate::tactical_map_spatial::{SpatialObstacleIndex, SpatialWallFilter};

/// High-level map object class reported by [`inspect_map_point`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MapHoverCategory {
    Token,
    EdgeBarrier,
    SolidProp,
    SurfaceProp,
    Ground,
}

/// Unified object & ground inspection snapshot for both 2D SVG and 3D WebGPU viewports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapHoverInspection {
    pub category: MapHoverCategory,
    pub title: String,
    pub subtitle: String,
    pub material_label: String,
    pub cover_label: String,
    pub height_cm: i32,
    pub elevation_cm: i32,
    pub rotation_deg: Option<i32>,
    pub world_x_m: f32,
    pub world_y_m: f32,
    /// Stable highlight reference (`entity_id`, `"edge:x:y:side"`, `"prop:idx"`, or `"ground"`).
    pub target_ref: String,
}

/// Migrates legacy tile-array maps (`tiles` non-empty) into vector `wall_edges` + `props`, leaving
/// empty tiles so legacy saved campaigns upgrade automatically to the vector pipeline.
pub fn migrate_legacy_tiles_to_vector(
    tiles: &[TacticalTileKind],
    width: i32,
    height: i32,
    edges: &mut Vec<MapWallEdge>,
    props: &mut Vec<MapPropPlacement>,
) {
    if width <= 0 || height <= 0 || tiles.len() < (width * height) as usize {
        return;
    }
    let tile_at = |x: i32, y: i32| -> TacticalTileKind {
        if x < 0 || y < 0 || x >= width || y >= height {
            TacticalTileKind::FullWall
        } else {
            tiles[(y * width + x) as usize]
        }
    };
    let mut push_edge_unique = |e: MapWallEdge| {
        if !edges.iter().any(|x| x.x == e.x && x.y == e.y && x.side == e.side) {
            edges.push(e);
        }
    };
    for y in 0..height {
        for x in 0..width {
            let t = tile_at(x, y);
            let covered_by_prop = props.iter().any(|p| p.covers(x, y));
            if covered_by_prop {
                continue;
            }
            match t {
                TacticalTileKind::FullWall => {
                    // Add StructuralPillar if isolated 1x1 block, else boundary edges against walkable floor.
                    let adj_floor = [(x, y - 1), (x, y + 1), (x - 1, y), (x + 1, y)]
                        .into_iter()
                        .all(|(nx, ny)| tile_at(nx, ny) != TacticalTileKind::FullWall);
                    if adj_floor {
                        props.push(make_prop(PropKind::StructuralPillar, x, y, 0, None, 0));
                    } else {
                        for (nx, ny, ex, ey, side) in [
                            (x, y - 1, x, y, EdgeSide::North),
                            (x, y + 1, x, y + 1, EdgeSide::North),
                            (x - 1, y, x, y, EdgeSide::West),
                            (x + 1, y, x + 1, y, EdgeSide::West),
                        ] {
                            if nx >= 0
                                && ny >= 0
                                && nx < width
                                && ny < height
                                && tile_at(nx, ny) != TacticalTileKind::FullWall
                            {
                                push_edge_unique(MapWallEdge {
                                    x: ex,
                                    y: ey,
                                    side,
                                    kind: WallEdgeKind::Wall,
                                    material: MaterialClass::Brick,
                                });
                            }
                        }
                    }
                }
                TacticalTileKind::DoorClosed | TacticalTileKind::DoorOpen => {
                    let kind = if t == TacticalTileKind::DoorClosed {
                        WallEdgeKind::DoorClosed
                    } else {
                        WallEdgeKind::DoorOpen
                    };
                    let side = if tile_at(x - 1, y) == TacticalTileKind::FullWall {
                        EdgeSide::North
                    } else {
                        EdgeSide::West
                    };
                    push_edge_unique(MapWallEdge {
                        x,
                        y,
                        side,
                        kind,
                        material: MaterialClass::SolidWood,
                    });
                }
                TacticalTileKind::HalfCover => {
                    props.push(make_prop(PropKind::JerseyBarrier, x, y, 0, Some(1), 0));
                }
                TacticalTileKind::Concealment => {
                    props.push(make_prop(PropKind::Bush, x, y, 0, None, 0));
                }
                TacticalTileKind::HazardZone => {
                    props.push(make_prop(PropKind::HazardSpill, x, y, 0, Some(1), 0));
                }
                TacticalTileKind::ElevatedPlatform => {
                    props.push(make_prop(PropKind::ElevatedCatwalk, x, y, 0, Some(1), 0));
                }
                TacticalTileKind::Floor => {}
            }
        }
    }
}

/// Checks that all traversable `1.0m` cell centers form one connected region on a `0.5m` lattice
/// (treating closed doors as traversable passages and blocking only `Wall`/`Window` edges and
/// movement-blocking props).
#[must_use]
pub fn all_walkable_areas_connected(
    width: i32,
    height: i32,
    edges: &[MapWallEdge],
    props: &[MapPropPlacement],
) -> bool {
    if width <= 0 || height <= 0 {
        return false;
    }
    let bw = width as f32;
    let bh = height as f32;
    let spatial = SpatialObstacleIndex::new(bw, bh, edges, props);
    let cols = (width * 2) as usize;
    let rows = (height * 2) as usize;
    let node_pos = |ix: usize, iy: usize| -> (f32, f32) {
        ((ix as f32 + 0.5) * 0.5, (iy as f32 + 0.5) * 0.5)
    };
    let mut open = vec![false; cols * rows];
    let mut first = None;
    for iy in 0..rows {
        for ix in 0..cols {
            let (px, py) = node_pos(ix, iy);
            if !spatial.any_full_obstacle_prop_intersects_circle(px, py, 0.20) {
                open[iy * cols + ix] = true;
                if first.is_none() {
                    first = Some((ix, iy));
                }
            }
        }
    }
    // Prefer starting near the West deployment pad `(2.5m, h/2 + 0.5m)` when unobstructed.
    let pref_ix = ((2.5_f32 / 0.5).floor() as usize).min(cols.saturating_sub(1));
    let pref_iy = ((((height / 2) as f32 + 0.5) / 0.5).floor() as usize).min(rows.saturating_sub(1));
    let start = if open[pref_iy * cols + pref_ix] {
        (pref_ix, pref_iy)
    } else if let Some(f) = first {
        f
    } else {
        return false;
    };
    let mut visited = vec![false; cols * rows];
    let mut queue = std::collections::VecDeque::new();
    visited[start.1 * cols + start.0] = true;
    queue.push_back(start);
    while let Some((cx, cy)) = queue.pop_front() {
        for (dx, dy) in [(-1isize, 0isize), (1, 0), (0, -1), (0, 1)] {
            let nx = cx as isize + dx;
            let ny = cy as isize + dy;
            if nx < 0 || ny < 0 || nx >= cols as isize || ny >= rows as isize {
                continue;
            }
            let (ux, uy) = (nx as usize, ny as usize);
            let nidx = uy * cols + ux;
            if !open[nidx] || visited[nidx] {
                continue;
            }
            if spatial.half_meter_step_blocked_by_wall_or_window(cx, cy, ux, uy) {
                continue;
            }
            visited[nidx] = true;
            queue.push_back((ux, uy));
        }
    }
    for gy in 0..height {
        for gx in 0..width {
            let cx = gx as f32 + 0.5;
            let cy = gy as f32 + 0.5;
            if spatial.circle_hits_wall(cx, cy, TOKEN_RADIUS_M, SpatialWallFilter::WallsAndWindowsOnly)
                || spatial.any_full_obstacle_prop_intersects_circle(cx, cy, TOKEN_RADIUS_M - 1e-4)
            {
                continue;
            }
            let bx = (gx * 2) as usize;
            let by = (gy * 2) as usize;
            let reached = [
                (bx, by),
                ((bx + 1).min(cols - 1), by),
                (bx, (by + 1).min(rows - 1)),
                ((bx + 1).min(cols - 1), (by + 1).min(rows - 1)),
            ]
            .into_iter()
            .any(|(sx, sy)| visited[sy * cols + sx]);
            if !reached {
                return false;
            }
        }
    }
    true
}

fn cover_label_for_grade(grade: CoverGrade) -> &'static str {
    match grade {
        CoverGrade::Clear => "No Cover",
        CoverGrade::Concealment => "Concealment",
        CoverGrade::LowCover => "Half Cover",
        CoverGrade::HighCover => "High / Corner Cover",
        CoverGrade::Blocked => "Full Cover",
    }
}

fn theme_is_interior(theme_name: &str) -> bool {
    let lower = theme_name.to_ascii_lowercase();
    ["apartment", "office", "subway", "warehouse", "diner", "bar", "interior", "room"]
        .iter()
        .any(|kw| lower.contains(kw))
}

/// Inspects any continuous `(x_m, y_m)` cursor position on the battlemap and classifies the topmost
/// object or underlying ground surface in priority order:
/// Tokens -> Wall/Door/Window Edges -> Solid Props (incl. engine block & trunk vs. canopy) ->
/// Surface/Catwalk/Hazard Props -> Base Ground/Floor.
#[must_use]
pub fn inspect_map_point(
    req: &GmMapUpdateRequest,
    theme_name: &str,
    x_m: f32,
    y_m: f32,
) -> Option<MapHoverInspection> {
    let (bw, bh) = req.board_size_m();
    if x_m < 0.0 || y_m < 0.0 || x_m > bw || y_m > bh {
        return None;
    }

    // 1. Living / placed tokens.
    let mut best_tok: Option<(&TokenPlacement, f32)> = None;
    for tok in &req.tokens {
        let (tx, ty) = req.token_pos_m(tok);
        let d = ((x_m - tx).powi(2) + (y_m - ty).powi(2)).sqrt();
        if d <= TOKEN_PICK_RADIUS_M && best_tok.is_none_or(|(_, bd)| d < bd) {
            best_tok = Some((tok, d));
        }
    }
    if let Some((tok, _)) = best_tok {
        let vis = req.visual_for(&tok.entity_id);
        let vis_tag = match tok.visibility {
            MessageVisibility::Public => "Public",
            MessageVisibility::GmOnly => "GM Only",
            _ => "Scoped",
        };
        let stance = vis.map_or("ready", |v| v.stance_pose.as_str());
        let elev_cm = vis.map_or(0, |v| v.elevation_cm);
        let facing = vis.map(|v| v.facing_deg);
        return Some(MapHoverInspection {
            category: MapHoverCategory::Token,
            title: tok.label.clone(),
            subtitle: format!("{vis_tag} · {stance} · ({x_m:.2}m, {y_m:.2}m)"),
            material_label: "Actor".to_owned(),
            cover_label: "Mobile Miniature".to_owned(),
            height_cm: 175,
            elevation_cm: elev_cm,
            rotation_deg: facing,
            world_x_m: x_m,
            world_y_m: y_m,
            target_ref: tok.entity_id.clone(),
        });
    }

    // 2. Thin edge walls / doors / windows.
    let mut best_edge: Option<(&MapWallEdge, f32)> = None;
    for edge in &req.wall_edges {
        let (a, b) = edge_endpoints_m(edge);
        let (d, _) = distance_point_to_segment(x_m, y_m, a, b);
        if d <= EDGE_PICK_HALF_THICK_M && best_edge.is_none_or(|(_, bd)| d < bd) {
            best_edge = Some((edge, d));
        }
    }
    if let Some((edge, _)) = best_edge {
        let (title, cover_label, height_cm) = match edge.kind {
            WallEdgeKind::Wall => (
                format!("{} Wall", edge.material.label()),
                "Full Cover / Blocks LoS".to_owned(),
                270,
            ),
            WallEdgeKind::DoorClosed => (
                "Closed Door".to_owned(),
                "Full Cover / Blocks LoS (Interactable)".to_owned(),
                215,
            ),
            WallEdgeKind::DoorOpen => (
                "Open Doorway".to_owned(),
                "Open Passage".to_owned(),
                215,
            ),
            WallEdgeKind::Window => (
                "Glazed Window".to_owned(),
                "Half Cover / Clear Sightline".to_owned(),
                180,
            ),
        };
        let orient_deg = match edge.side {
            EdgeSide::North => 0,
            EdgeSide::West => 90,
        };
        return Some(MapHoverInspection {
            category: MapHoverCategory::EdgeBarrier,
            title,
            subtitle: "Structural Edge".to_owned(),
            material_label: edge.material.label().to_owned(),
            cover_label,
            height_cm,
            elevation_cm: 0,
            rotation_deg: Some(orient_deg),
            world_x_m: x_m,
            world_y_m: y_m,
            target_ref: format!("edge:{}:{}:{:?}", edge.x, edge.y, edge.side),
        });
    }

    // 3. Solid / three-dimensional props (newest on top; vehicle & tree parts broken out).
    for (idx, p) in req.props.iter().enumerate().rev() {
        let spec = prop_spec(p.kind);
        if spec.floor_layer || !prop_contains_point(p, x_m, y_m, true) {
            continue;
        }
        if matches!(p.kind, PropKind::LargeTree | PropKind::MediumTree) {
            let in_trunk = prop_contains_point(p, x_m, y_m, false);
            return Some(MapHoverInspection {
                category: MapHoverCategory::SolidProp,
                title: if in_trunk {
                    format!("{} — Trunk", p.kind.label())
                } else {
                    format!("{} — Canopy", p.kind.label())
                },
                subtitle: format!("{} · {}°", p.kind.label(), p.rotation_deg),
                material_label: if in_trunk {
                    MaterialClass::LiveWood.label().to_owned()
                } else {
                    MaterialClass::Foliage.label().to_owned()
                },
                cover_label: if in_trunk {
                    "Full Cover (Solid Trunk)".to_owned()
                } else {
                    "Overhead Concealment".to_owned()
                },
                height_cm: spec.height_cm,
                elevation_cm: if in_trunk { 0 } else { 210 },
                rotation_deg: Some(p.rotation_deg),
                world_x_m: x_m,
                world_y_m: y_m,
                target_ref: format!("prop:{idx}"),
            });
        }
        if matches!(p.kind, PropKind::Car | PropKind::UtilityVan) && vehicle_point_is_engine_bay(p, x_m, y_m) {
            return Some(MapHoverInspection {
                category: MapHoverCategory::SolidProp,
                title: format!("{} — Engine Block", p.kind.label()),
                subtitle: "Front Hood Hard Cover".to_owned(),
                material_label: MaterialClass::ArmorSteel.label().to_owned(),
                cover_label: "Half Cover (Engine Armor)".to_owned(),
                height_cm: spec.height_cm.min(115),
                elevation_cm: 0,
                rotation_deg: Some(p.rotation_deg),
                world_x_m: x_m,
                world_y_m: y_m,
                target_ref: format!("prop:{idx}"),
            });
        }
        let mat_label = spec.material.label().to_owned();
        let cover = if matches!(p.kind, PropKind::BarStool) {
            "Seat / Low Obstacle"
        } else {
            cover_label_for_grade(spec.obstruction.cover_grade())
        };
        return Some(MapHoverInspection {
            category: MapHoverCategory::SolidProp,
            title: p.kind.label().to_owned(),
            subtitle: format!("Solid Prop · {}°", p.rotation_deg),
            material_label: mat_label,
            cover_label: cover.to_owned(),
            height_cm: spec.height_cm,
            elevation_cm: 0,
            rotation_deg: Some(p.rotation_deg),
            world_x_m: x_m,
            world_y_m: y_m,
            target_ref: format!("prop:{idx}"),
        });
    }

    // 4. Elevated bridge decks override underlying ground channels; then floor-layer props; then vector surfaces.
    let top_surf = crate::map_surface_geom::topmost_surface_at_m(&req.surfaces, x_m, y_m);
    if let Some(surf) = top_surf
        && surf.kind == satya_schema::SurfaceFeatureKind::BridgeDeck
    {
        return Some(MapHoverInspection {
            category: MapHoverCategory::SurfaceProp,
            title: surf.kind.label().to_owned(),
            subtitle: "Elevated Bridge / Overpass (+15 cm)".to_owned(),
            material_label: surf.kind.material().label().to_owned(),
            cover_label: "Walkable Surface".to_owned(),
            height_cm: 15,
            elevation_cm: 15,
            rotation_deg: None,
            world_x_m: x_m,
            world_y_m: y_m,
            target_ref: "surface:bridge".to_owned(),
        });
    }

    for (idx, p) in req.props.iter().enumerate().rev() {
        let spec = prop_spec(p.kind);
        if !spec.floor_layer || !prop_contains_point(p, x_m, y_m, false) {
            continue;
        }
        let (subtitle, cover_label, elevation_cm) = if p.kind == PropKind::ElevatedCatwalk {
            ("Elevated Walkway (+15 cm)".to_owned(), "Elevated Surface".to_owned(), 15)
        } else if spec.move_cost_mult > 1.1 {
            (
                format!("Difficult Ground ({:.1}x Move Cost)", spec.move_cost_mult),
                "Hazardous Footing".to_owned(),
                0,
            )
        } else {
            ("Surface Layer".to_owned(), "Walkable Surface".to_owned(), 0)
        };
        return Some(MapHoverInspection {
            category: MapHoverCategory::SurfaceProp,
            title: p.kind.label().to_owned(),
            subtitle,
            material_label: spec.material.label().to_owned(),
            cover_label,
            height_cm: spec.height_cm,
            elevation_cm,
            rotation_deg: Some(p.rotation_deg),
            world_x_m: x_m,
            world_y_m: y_m,
            target_ref: format!("prop:{idx}"),
        });
    }

    if let Some(surf) = top_surf {
        let kind = surf.kind;
        let cost = kind.move_cost_mult();
        let (subtitle, cover_label, elevation_cm) = match kind {
            satya_schema::SurfaceFeatureKind::DefensiveTrench => (
                format!("Excavated Trench ({cost:.1}x Move Cost)"),
                "Low Cover (Trench)".to_owned(),
                -40,
            ),
            satya_schema::SurfaceFeatureKind::CreekChannel
            | satya_schema::SurfaceFeatureKind::DrainageDitch => (
                format!("Difficult Ground ({cost:.1}x Move Cost)"),
                "Hazardous Footing".to_owned(),
                -25,
            ),
            satya_schema::SurfaceFeatureKind::GardenBedZone => (
                format!("Planted Soil ({cost:.2}x Move Cost)"),
                "Soft Ground".to_owned(),
                8,
            ),
            _ => ("Vector Surface Layer".to_owned(), "Walkable Surface".to_owned(), 0),
        };
        return Some(MapHoverInspection {
            category: MapHoverCategory::SurfaceProp,
            title: kind.label().to_owned(),
            subtitle,
            material_label: kind.material().label().to_owned(),
            cover_label,
            height_cm: 0,
            elevation_cm,
            rotation_deg: None,
            world_x_m: x_m,
            world_y_m: y_m,
            target_ref: format!("surface:{}", kind.as_str()),
        });
    }

    // 5. Bare ground / floor fallback.
    let theme_label = if theme_name.trim().is_empty() {
        "Tactical Floor"
    } else {
        theme_name.trim()
    };
    let (title, material_label) = if theme_is_interior(theme_label) {
        (format!("Interior Floor — {theme_label}"), "Finished Flooring".to_owned())
    } else {
        (format!("Open Ground — {theme_label}"), "Natural / Paved Ground".to_owned())
    };
    Some(MapHoverInspection {
        category: MapHoverCategory::Ground,
        title,
        subtitle: format!("Clear Terrain · ({x_m:.2}m, {y_m:.2}m)"),
        material_label,
        cover_label: "No Cover".to_owned(),
        height_cm: 0,
        elevation_cm: 0,
        rotation_deg: None,
        world_x_m: x_m,
        world_y_m: y_m,
        target_ref: "ground".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map_dressing::make_prop_at_cm;
    use crate::tactical_map_geom::{intersect_segment_prop, resolve_token_destination_cm};
    use satya_schema::TokenVisualSpec;

    fn sample_req() -> GmMapUpdateRequest {
        let mut req = GmMapUpdateRequest {
            map_id: "m1".to_owned(),
            title: "Suburban Park".to_owned(),
            image_url: String::new(),
            grid_width: 12,
            grid_height: 10,
            cell_size_cm: 100,
            theme_preset: Some("wooded_park".to_owned()),
            props: vec![
                make_prop_at_cm(PropKind::Car, 500, 450, 30, None, 0),
                make_prop_at_cm(PropKind::LargeTree, 200, 200, 0, None, 0),
                make_prop(PropKind::HazardSpill, 8, 7, 0, Some(2), 0),
                make_prop(PropKind::ElevatedCatwalk, 8, 2, 0, Some(3), 0),
            ],
            wall_edges: vec![MapWallEdge {
                x: 1,
                y: 5,
                side: EdgeSide::West,
                kind: WallEdgeKind::Wall,
                material: MaterialClass::Brick,
            }],
            tokens: vec![TokenPlacement {
                entity_id: "pc1".to_owned(),
                label: "Vance".to_owned(),
                x: 4,
                y: 8,
                visibility: MessageVisibility::Public,
            }],
            token_visuals: vec![TokenVisualSpec {
                entity_id: "pc1".to_owned(),
                model_preset: "valkyrie_rig".to_owned(),
                color_hex: "#38bdf8".to_owned(),
                facing_deg: 90,
                elevation_cm: 0,
                stance_pose: "aimed".to_owned(),
                portrait_url: None,
                pos_x_cm: Some(450),
                pos_y_cm: Some(850),
            }],
            ..GmMapUpdateRequest::default()
        };
        req.set_token_pos_cm("pc1", 450, 850);
        req
    }

    #[test]
    fn car_30_deg_engine_bay_ray_and_hover_classify_armor_steel() {
        let car = make_prop_at_cm(PropKind::Car, 500, 450, 30, None, 0);
        let (dx, dy) = car.footprint_m().local_dir_to_world(0.0, 2.0);
        let hit = intersect_segment_prop(&car, (3.0, 4.5 + dy), (7.0, 4.5 + dy)).unwrap();
        assert!(hit.is_engine_bay);
        assert_eq!(hit.material, MaterialClass::ArmorSteel);

        let mut req = sample_req();
        req.props = vec![car];
        let eng = inspect_map_point(&req, "Suburban Park", 5.0 + dx, 4.5 + dy).unwrap();
        assert!(eng.title.contains("Engine Block"));
        assert_eq!(eng.material_label, MaterialClass::ArmorSteel.label());

        let cab = inspect_map_point(&req, "Suburban Park", 5.0, 4.5).unwrap();
        assert_eq!(cab.material_label, MaterialClass::VehicleBody.label());
    }

    #[test]
    fn hover_inspects_tokens_edges_trees_surfaces_and_ground() {
        let req = sample_req();
        let tok = inspect_map_point(&req, "Suburban Park", 4.6, 8.55).unwrap();
        assert_eq!(tok.category, MapHoverCategory::Token);
        assert_eq!(tok.title, "Vance");

        let edge = inspect_map_point(&req, "Suburban Park", 1.05, 5.5).unwrap();
        assert_eq!(edge.category, MapHoverCategory::EdgeBarrier);
        assert!(edge.title.contains("Brick masonry Wall"), "title={}", edge.title);

        let trunk = inspect_map_point(&req, "Suburban Park", 2.0, 2.0).unwrap();
        assert!(trunk.title.contains("Trunk"));
        let canopy = inspect_map_point(&req, "Suburban Park", 2.75, 2.0).unwrap();
        assert!(canopy.title.contains("Canopy"));

        let spill = inspect_map_point(&req, "Suburban Park", 8.8, 7.5).unwrap();
        assert_eq!(spill.category, MapHoverCategory::SurfaceProp);
        assert!(spill.subtitle.contains("2.0x"));

        let catwalk = inspect_map_point(&req, "Suburban Park", 9.5, 2.5).unwrap();
        assert_eq!(catwalk.elevation_cm, 15);

        let ground = inspect_map_point(&req, "Suburban Park", 0.5, 0.5).unwrap();
        assert_eq!(ground.category, MapHoverCategory::Ground);
        assert!(ground.title.contains("Open Ground"));
    }

    #[test]
    fn token_slide_snap_near_wall_and_reject_deep_obstacle() {
        let req = sample_req();
        // Clicking at x=1.20 next to West brick wall at x=1.00 nudges right to >= 1.375m.
        let snapped = resolve_token_destination_cm(&req, "pc1", 120, 550).unwrap();
        assert!(snapped.0 >= 137 && snapped.0 <= 145, "snapped x={}", snapped.0);
        // Clicking deep inside the car cabin (500, 450) rejects.
        assert!(resolve_token_destination_cm(&req, "pc1", 500, 450).is_err());
    }
}
