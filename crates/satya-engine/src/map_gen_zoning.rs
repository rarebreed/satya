//! # Decoupled Zoning Context × Economic Tier Generators (`map_gen_zoning`)
//!
//! Combines building generators ([`crate::map_gen_residential`], [`crate::map_gen_commercial`])
//! with street, yard, and civic context ([`ZoningContext`]) for six playable presets:
//! - [`SectorPreset::UrbanResidential`]
//! - [`SectorPreset::ProjectsHighDensity`]
//! - [`SectorPreset::SuburbanNeighborhood`]
//! - [`SectorPreset::RuralCountryside`]
//! - [`SectorPreset::LuxuryMansion`]
//! - [`SectorPreset::DowntownCommercial`]
//!
//! Emits multi-floor [`MapFloorLevel`] stacks (`B1`, `1F`, `2F`) with vertically registered
//! [`PropKind::Stairwell`] portals, clear spawn pads, and 100% single-floor reachability.

use satya_schema::{
    EconomicTier, EdgeSide, MapFloorLevel, MapPropPlacement, MapSurfaceFeature, MapWallEdge,
    MaterialClass, PropKind, StoreArchetype, SurfaceFeatureKind, WallEdgeKind, ZoningContext,
};

use crate::map_dressing::{make_prop, prop_spec};
use crate::map_gen_commercial::generate_store_unit;
use crate::map_gen_residential::{
    CardinalEntry, FloorContent, GeneratedHome, LotBox, generate_home_in_lot,
};
use crate::map_outdoor_gen::{OutdoorBuilder, RectM};
use crate::map_outdoor_natural::{
    CurveProfile, CurveSpec, route_trail_across_bridge_m, synthesize_natural_curve_m,
};
use crate::map_pool_spa_gen::{OutdoorPoolSpaOpts, try_place_outdoor_pool_or_spa};
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

/// Returns the canonical `(ZoningContext, EconomicTier)` pair for a zoned [`SectorPreset`].
#[must_use]
pub const fn default_zoning_and_tier(preset: SectorPreset) -> (ZoningContext, EconomicTier) {
    match preset {
        SectorPreset::UrbanResidential => {
            (ZoningContext::UrbanResidential, EconomicTier::MiddleClass)
        }
        SectorPreset::ProjectsHighDensity => {
            (ZoningContext::ProjectsHighDensity, EconomicTier::LowIncome)
        }
        SectorPreset::SuburbanNeighborhood => (
            ZoningContext::SuburbanNeighborhood,
            EconomicTier::UpperMiddleClass,
        ),
        SectorPreset::RuralCountryside => {
            (ZoningContext::RuralCountryside, EconomicTier::LowIncome)
        }
        SectorPreset::LuxuryMansion => (
            ZoningContext::SuburbanNeighborhood,
            EconomicTier::LuxuryMansion,
        ),
        SectorPreset::DowntownCommercial => (
            ZoningContext::DowntownCommercial,
            EconomicTier::UpperMiddleClass,
        ),
        _ => (ZoningContext::UrbanResidential, EconomicTier::MiddleClass),
    }
}

/// Accumulates `B1` cellar and `2F` upper levels across one or more buildings in the sector.
#[derive(Default)]
pub(crate) struct FloorStack {
    pub(crate) b1_edges: Vec<MapWallEdge>,
    pub(crate) b1_props: Vec<MapPropPlacement>,
    pub(crate) b1_surfaces: Vec<MapSurfaceFeature>,
    pub(crate) f2_edges: Vec<MapWallEdge>,
    pub(crate) f2_props: Vec<MapPropPlacement>,
    pub(crate) f2_surfaces: Vec<MapSurfaceFeature>,
}

impl FloorStack {
    pub(crate) fn absorb_ground(b: &mut OutdoorBuilder, ground: FloorContent) {
        for e in ground.wall_edges {
            b.put_edge(e.x, e.y, e.side, e.kind, e.material);
        }
        b.props.extend(ground.props);
        b.surfaces.extend(ground.surfaces);
    }

    pub(crate) fn absorb_home(
        &mut self,
        b: &mut OutdoorBuilder,
        home: GeneratedHome,
        allow_basement: bool,
    ) {
        Self::absorb_ground(b, home.ground);
        if allow_basement && let Some(b1) = home.basement {
            self.b1_edges.extend(b1.wall_edges);
            self.b1_props.extend(b1.props);
            self.b1_surfaces.extend(b1.surfaces);
        }
        if let Some(f2) = home.upper {
            self.f2_edges.extend(f2.wall_edges);
            self.f2_props.extend(f2.props);
            self.f2_surfaces.extend(f2.surfaces);
        }
    }

    pub(crate) fn absorb_upper(&mut self, f2: FloorContent) {
        self.f2_edges.extend(f2.wall_edges);
        self.f2_props.extend(f2.props);
        self.f2_surfaces.extend(f2.surfaces);
    }

    pub(crate) fn into_sector(mut self, b: OutdoorBuilder, w: i32, h: i32) -> ProceduralSector {
        let mut sec = b.finalize_connected();
        let mut floors = Vec::new();
        if !self.b1_edges.is_empty() {
            prune_occluded_surfaces(&mut self.b1_surfaces, w, h);
            floors.push(MapFloorLevel {
                floor_index: -1,
                label: "B1 · Cellar & Vault".to_string(),
                elevation_cm: -320,
                wall_edges: self.b1_edges,
                props: self.b1_props,
                surfaces: self.b1_surfaces,
                ..Default::default()
            });
        }
        if !self.f2_edges.is_empty() {
            prune_occluded_surfaces(&mut self.f2_surfaces, w, h);
            floors.push(MapFloorLevel {
                floor_index: 0,
                label: "1F · Ground".to_string(),
                elevation_cm: 0,
                wall_edges: sec.wall_edges.clone(),
                props: sec.props.clone(),
                surfaces: sec.surfaces.clone(),
                ..Default::default()
            });
            floors.push(MapFloorLevel {
                floor_index: 1,
                label: "2F · Upper Level".to_string(),
                elevation_cm: 340,
                wall_edges: self.f2_edges,
                props: self.f2_props,
                surfaces: self.f2_surfaces,
                ..Default::default()
            });
        } else if !floors.is_empty() {
            floors.push(MapFloorLevel {
                floor_index: 0,
                label: "1F · Ground".to_string(),
                elevation_cm: 0,
                wall_edges: sec.wall_edges.clone(),
                props: sec.props.clone(),
                surfaces: sec.surfaces.clone(),
                ..Default::default()
            });
        }
        floors.sort_by_key(|f| f.floor_index);
        sec.floors = floors;
        sec.active_floor = 0;
        sec
    }
}

/// Safe building column envelope reserving East/West spawn pads on standard boards.
fn safe_x_span(w: i32) -> (i32, i32) {
    let pad = if w >= 16 {
        4
    } else if w >= 12 {
        3
    } else {
        2
    };
    (pad, (w - pad).max(pad + 4).min(w - 1))
}

/// Emits a front walk spur from a building entrance to the sidewalk or curb with exposed yard length.
fn add_entry_walk(
    b: &mut OutdoorBuilder,
    lot: LotBox,
    entry: CardinalEntry,
    kind: SurfaceFeatureKind,
    target_m: f32,
) {
    let mx = (lot.x0 + lot.x1) as f32 * 0.5;
    let my = (lot.y0 + lot.y1) as f32 * 0.5;
    let pts = match entry {
        CardinalEntry::North => [(mx, lot.y0 as f32), (mx, target_m)],
        CardinalEntry::South => [(mx, lot.y1 as f32), (mx, target_m)],
        CardinalEntry::West => [(lot.x0 as f32, my), (target_m, my)],
        CardinalEntry::East => [(lot.x1 as f32, my), (target_m, my)],
    };
    b.add_ribbon(kind, &pts, 145);
}

/// Scatters perimeter shrubs, trees, benches, and organic yard patches strictly outside roofs & asphalt.
fn dress_yard_props(b: &mut OutdoorBuilder, tier: EconomicTier, tree_budget: i32, patches: usize) {
    let bounds = RectM {
        x0: 1.5,
        y0: 1.5,
        x1: b.w as f32 - 1.5,
        y1: b.h as f32 - 1.5,
    };
    for i in 0..patches {
        let px = b
            .rng
            .f32_range(bounds.x0 + 0.5, (bounds.x1 - 0.5).max(bounds.x0 + 1.0));
        let py = b
            .rng
            .f32_range(bounds.y0 + 0.5, (bounds.y1 - 0.5).max(bounds.y0 + 1.0));
        let kind = if i % 2 == 0 {
            SurfaceFeatureKind::GardenBedZone
        } else {
            SurfaceFeatureKind::MulchZone
        };
        let rad = if tier == EconomicTier::LuxuryMansion {
            1.5
        } else {
            1.1
        };
        b.try_add_organic_patch(kind, (px, py), (rad, rad * 0.8), 0.30, 28);
    }
    if tree_budget > 0 {
        b.scatter_trees(PropKind::MediumTree, tree_budget, 2.6, bounds);
        b.scatter_turf_prop_rot(PropKind::Bush, (tree_budget + 1).max(2), 35, bounds);
    }
    b.scatter_turf_prop_rot(PropKind::ParkBench, 2, 45, bounds);
    b.scatter_turf_prop_rot(PropKind::TrashCan, 1, 30, bounds);
    if matches!(tier, EconomicTier::LowIncome | EconomicTier::MiddleClass) {
        b.scatter_turf_prop_rot(PropKind::WoodCrate, 2, 30, bounds);
    }
    if !matches!(tier, EconomicTier::LowIncome) {
        b.scatter_turf_prop_rot(PropKind::GardenShed, 1, 0, bounds);
    }
}

/// Places a compound estate perimeter wall around `comp` with two wide vehicular/foot gates.
fn add_estate_perimeter(b: &mut OutdoorBuilder, comp: LotBox, gate_x: i32, gate_south: bool) {
    for x in comp.x0..comp.x1 {
        let is_gate = x == gate_x || x == gate_x + 1;
        let k = if is_gate {
            WallEdgeKind::DoorOpen
        } else {
            WallEdgeKind::Wall
        };
        let m = if is_gate {
            MaterialClass::SheetSteel
        } else {
            MaterialClass::ConcreteBlock
        };
        b.put_edge(
            x,
            comp.y0,
            EdgeSide::North,
            if !gate_south && is_gate {
                k
            } else {
                WallEdgeKind::Wall
            },
            if !gate_south && is_gate {
                m
            } else {
                MaterialClass::ConcreteBlock
            },
        );
        b.put_edge(
            x,
            comp.y1,
            EdgeSide::North,
            if gate_south && is_gate {
                k
            } else {
                WallEdgeKind::Wall
            },
            if gate_south && is_gate {
                m
            } else {
                MaterialClass::ConcreteBlock
            },
        );
    }
    for y in comp.y0..comp.y1 {
        let is_side_door = y == (comp.y0 + comp.y1) / 2;
        let k = if is_side_door {
            WallEdgeKind::DoorOpen
        } else {
            WallEdgeKind::Wall
        };
        let m = if is_side_door {
            MaterialClass::SheetSteel
        } else {
            MaterialClass::ConcreteBlock
        };
        b.put_edge(comp.x0, y, EdgeSide::West, k, m);
        b.put_edge(
            comp.x1,
            y,
            EdgeSide::West,
            WallEdgeKind::Wall,
            MaterialClass::ConcreteBlock,
        );
    }
}

/// Topology family for `UrbanResidential` and `ProjectsHighDensity` city blocks (`1F` + `2F` + opt `B1`).
fn place_unit_prop_checked(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    bay: LotBox,
    cand: MapPropPlacement,
) -> bool {
    if cand.x < bay.x0
        || cand.y < bay.y0
        || cand.x + cand.w_cells > bay.x1
        || cand.y + cand.h_cells > bay.y1
    {
        return false;
    }
    if crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges) {
        return false;
    }
    let blocks_door = edges.iter().any(|e| {
        matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
            && match e.side {
                EdgeSide::West => cand.covers(e.x - 1, e.y) || cand.covers(e.x, e.y),
                EdgeSide::North => cand.covers(e.x, e.y - 1) || cand.covers(e.x, e.y),
            }
    });
    if blocks_door && !prop_spec(cand.kind).floor_layer {
        return false;
    }
    if props.iter().any(|e| {
        (!prop_spec(e.kind).floor_layer || e.kind.is_vertical_portal())
            && crate::tactical_map_geom::props_overlap_continuous(&cand, e)
    }) {
        return false;
    }
    props.push(cand);
    true
}

fn h_slab_wall(
    edges: &mut Vec<MapWallEdge>,
    y: i32,
    x0: i32,
    x1: i32,
    doors: &[i32],
    wall_mat: MaterialClass,
) {
    for x in x0..x1 {
        let (k, m) = if doors.contains(&x) {
            (WallEdgeKind::DoorClosed, MaterialClass::SolidWood)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        if !edges
            .iter()
            .any(|e| e.x == x && e.y == y && e.side == EdgeSide::North)
        {
            edges.push(MapWallEdge {
                x,
                y,
                side: EdgeSide::North,
                kind: k,
                material: m,
            });
        }
    }
}

fn v_slab_wall(
    edges: &mut Vec<MapWallEdge>,
    x: i32,
    y0: i32,
    y1: i32,
    doors: &[i32],
    wall_mat: MaterialClass,
) {
    for y in y0..y1 {
        let (k, m) = if doors.contains(&y) {
            (WallEdgeKind::DoorClosed, MaterialClass::SolidWood)
        } else {
            (WallEdgeKind::Wall, wall_mat)
        };
        if !edges
            .iter()
            .any(|e| e.x == x && e.y == y && e.side == EdgeSide::West)
        {
            edges.push(MapWallEdge {
                x,
                y,
                side: EdgeSide::West,
                kind: k,
                material: m,
            });
        }
    }
}

fn dress_compact_apartment_unit(
    bay: LotBox,
    idx: usize,
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    s: u32,
) {
    let bed_kind = if bay.w() >= 4 && bay.h() >= 4 && idx.is_multiple_of(2) {
        PropKind::Bed
    } else {
        PropKind::SingleBed
    };
    let _ = place_unit_prop_checked(
        edges,
        props,
        bay,
        make_prop(bed_kind, bay.x0, bay.y0, 0, None, s + 1),
    );
    let _ = place_unit_prop_checked(
        edges,
        props,
        bay,
        make_prop(PropKind::Toilet, bay.x1 - 1, bay.y0, 0, None, s + 2),
    );
    let _ = place_unit_prop_checked(
        edges,
        props,
        bay,
        make_prop(PropKind::BathroomSink, bay.x1 - 1, (bay.y0 + 1).min(bay.y1 - 1), 90, None, s + 3),
    );
    let _ = place_unit_prop_checked(
        edges,
        props,
        bay,
        make_prop(
            PropKind::KitchenCounter,
            bay.x0,
            bay.y1 - 1,
            0,
            Some(bay.w().clamp(1, 2)),
            s + 4,
        ),
    );
    if bay.w() >= 4 {
        let _ = place_unit_prop_checked(
            edges,
            props,
            bay,
            make_prop(PropKind::Sofa, (bay.x0 + 1).min(bay.x1 - 2), bay.y1 - 1, 180, None, s + 5),
        );
    }
    let _ = place_unit_prop_checked(
        edges,
        props,
        bay,
        make_prop(PropKind::Desk, bay.x1 - 1, bay.y1 - 1, 90, None, s + 6),
    );
    let _ = place_unit_prop_checked(
        edges,
        props,
        bay,
        make_prop(PropKind::TrashCan, bay.x0, (bay.y0 + 1).min(bay.y1 - 1), 0, None, s + 7),
    );
}

#[allow(clippy::too_many_arguments)]
fn build_apartment_slab_floor(
    w: i32,
    h: i32,
    slab: LotBox,
    floor_idx: i32,
    seed: u64,
) -> FloorContent {
    let mut edges = Vec::new();
    let mut props = Vec::new();
    let s = (seed as u32).wrapping_add(floor_idx as u32 * 53);

    // Concrete exterior perimeter with stairwell/courtyard double entries on 1F.
    for x in slab.x0..slab.x1 {
        let win = (x - slab.x0) % 3 == 1 && x + 1 < slab.x1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::ConcreteBlock)
        };
        edges.push(MapWallEdge { x, y: slab.y0, side: EdgeSide::North, kind: k, material: m });
        edges.push(MapWallEdge { x, y: slab.y1, side: EdgeSide::North, kind: k, material: m });
    }
    for y in slab.y0..slab.y1 {
        let win = (y - slab.y0) % 3 == 1 && y + 1 < slab.y1;
        let (k, m) = if win {
            (WallEdgeKind::Window, MaterialClass::Glass)
        } else {
            (WallEdgeKind::Wall, MaterialClass::ConcreteBlock)
        };
        edges.push(MapWallEdge { x: slab.x0, y, side: EdgeSide::West, kind: k, material: m });
        edges.push(MapWallEdge { x: slab.x1, y, side: EdgeSide::West, kind: k, material: m });
    }

    let horiz_corr = slab.w() >= slab.h() && slab.h() >= 6;
    let corr_box = if horiz_corr {
        let cy0 = (slab.y0 + slab.h() / 2 - 1).clamp(slab.y0 + 2, slab.y1 - 3);
        let cy1 = (cy0 + 2).min(slab.y1 - 2);
        if floor_idx == 0 {
            for e in edges.iter_mut() {
                if (e.x == slab.x0 || e.x == slab.x1)
                    && e.side == EdgeSide::West
                    && (e.y == cy0 || e.y == cy0 + 1)
                {
                    e.kind = WallEdgeKind::DoorOpen;
                    e.material = MaterialClass::Glass;
                }
            }
        }
        let n_cols = (slab.w() / 4).clamp(2, 4);
        let mut xs = vec![slab.x0];
        for i in 1..n_cols {
            xs.push(slab.x0 + (slab.w() * i) / n_cols);
        }
        xs.push(slab.x1);
        let mut ndoors = Vec::new();
        let mut sdoors = Vec::new();
        for win in xs.windows(2) {
            ndoors.push((win[0] + win[1]) / 2);
            sdoors.push(((win[0] + win[1]) / 2).clamp(win[0], win[1] - 1));
        }
        h_slab_wall(&mut edges, cy0, slab.x0, slab.x1, &ndoors, MaterialClass::Drywall);
        h_slab_wall(&mut edges, cy1, slab.x0, slab.x1, &sdoors, MaterialClass::Drywall);
        for &dx in xs.iter().skip(1).take(xs.len().saturating_sub(2)) {
            v_slab_wall(&mut edges, dx, slab.y0, cy0, &[], MaterialClass::ConcreteBlock);
            v_slab_wall(&mut edges, dx, cy1, slab.y1, &[], MaterialClass::ConcreteBlock);
        }
        let stair_x = slab.x0 + 1;
        let elev_x = (slab.x1 - 3).max(stair_x + 2);
        props.push(make_prop(PropKind::Stairwell, stair_x, cy0, 0, None, s));
        props.push(make_prop(PropKind::ElevatorCar, elev_x, cy0, 0, None, s + 1));
        for (i, win) in xs.windows(2).enumerate() {
            dress_compact_apartment_unit(
                LotBox::new(win[0], slab.y0, win[1], cy0),
                i * 2,
                &edges,
                &mut props,
                s + 20 + i as u32 * 13,
            );
            dress_compact_apartment_unit(
                LotBox::new(win[0], cy1, win[1], slab.y1),
                i * 2 + 1,
                &edges,
                &mut props,
                s + 90 + i as u32 * 13,
            );
        }
        LotBox::new(slab.x0, cy0, slab.x1, cy1)
    } else {
        let cx0 = (slab.x0 + slab.w() / 2 - 1).clamp(slab.x0 + 2, slab.x1 - 4);
        let cx1 = (cx0 + 2).min(slab.x1 - 2);
        if floor_idx == 0 {
            for e in edges.iter_mut() {
                if (e.y == slab.y0 || e.y == slab.y1)
                    && e.side == EdgeSide::North
                    && (e.x == cx0 || e.x == cx0 + 1)
                {
                    e.kind = WallEdgeKind::DoorOpen;
                    e.material = MaterialClass::Glass;
                }
            }
        }
        let n_rows = (slab.h() / 4).clamp(2, 4);
        let mut ys = vec![slab.y0];
        for i in 1..n_rows {
            ys.push(slab.y0 + (slab.h() * i) / n_rows);
        }
        ys.push(slab.y1);
        let mut wdoors = Vec::new();
        let mut edoors = Vec::new();
        for win in ys.windows(2) {
            wdoors.push((win[0] + win[1]) / 2);
            edoors.push(((win[0] + win[1]) / 2).clamp(win[0], win[1] - 1));
        }
        v_slab_wall(&mut edges, cx0, slab.y0, slab.y1, &wdoors, MaterialClass::Drywall);
        v_slab_wall(&mut edges, cx1, slab.y0, slab.y1, &edoors, MaterialClass::Drywall);
        for &dy in ys.iter().skip(1).take(ys.len().saturating_sub(2)) {
            h_slab_wall(&mut edges, dy, slab.x0, cx0, &[], MaterialClass::ConcreteBlock);
            h_slab_wall(&mut edges, dy, cx1, slab.x1, &[], MaterialClass::ConcreteBlock);
        }
        let stair_y = slab.y0 + 1;
        let elev_y = (slab.y1 - 3).max(stair_y + 2);
        props.push(make_prop(PropKind::Stairwell, cx0, stair_y, 0, None, s));
        props.push(make_prop(PropKind::ElevatorCar, cx0, elev_y, 0, None, s + 1));
        for (i, win) in ys.windows(2).enumerate() {
            dress_compact_apartment_unit(
                LotBox::new(slab.x0, win[0], cx0, win[1]),
                i * 2,
                &edges,
                &mut props,
                s + 20 + i as u32 * 13,
            );
            dress_compact_apartment_unit(
                LotBox::new(cx1, win[0], slab.x1, win[1]),
                i * 2 + 1,
                &edges,
                &mut props,
                s + 90 + i as u32 * 13,
            );
        }
        LotBox::new(cx0, slab.y0, cx1, slab.y1)
    };

    crate::map_gen_residential::trim_floor_props_to_connected(w, h, slab, &edges, &mut props);
    let surfaces = vec![
        slab.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 200),
        corr_box.polygon_feature(SurfaceFeatureKind::LinoleumCheckFloor, 0, s + 201),
    ];
    FloorContent {
        wall_edges: edges,
        props,
        surfaces,
    }
}

/// Multi-unit high-density concrete apartment slabs around a paved courtyard (`ProjectsHighDensity`).
fn build_projects_complex(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let street_north = variant == 1;
    let (street_y, walk_y, lot_y0, lot_y1) = if street_north {
        (1.4, 3.45, 5, (h - 2).max(11))
    } else {
        (hf - 1.4, hf - 3.45, 2, (h - 5).max(8))
    };

    b.add_polygon_surface(
        SurfaceFeatureKind::LawnGrassZone,
        &[(0.0, 0.0), (wf, 0.0), (wf, hf), (0.0, hf)],
        0,
        (seed as u32).wrapping_add(7),
    );
    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(0.0, street_y), (wf, street_y)],
        300,
    );
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(0.0, walk_y), (wf, walk_y)],
        160,
    );

    let (sx0, sx1) = safe_x_span(w);
    let span = sx1 - sx0;
    let slabs: Vec<LotBox> = if span >= 24 && lot_y1 - lot_y0 >= 10 {
        let slab_w = ((span - 8) / 2).clamp(9, 13);
        let court = LotBox::new(sx0 + slab_w, lot_y0 + 1, sx1 - slab_w, lot_y1 - 1);
        b.add_polygon_surface(
            SurfaceFeatureKind::ConcreteWalk,
            &[
                (court.x0 as f32, court.y0 as f32),
                (court.x1 as f32, court.y0 as f32),
                (court.x1 as f32, court.y1 as f32),
                (court.x0 as f32, court.y1 as f32),
            ],
            16,
            (seed as u32).wrapping_add(41),
        );
        vec![
            LotBox::new(sx0, lot_y0, sx0 + slab_w, lot_y1),
            LotBox::new(sx1 - slab_w, lot_y0, sx1, lot_y1),
        ]
    } else {
        vec![LotBox::new(sx0, lot_y0, sx1, lot_y1)]
    };

    for (i, slab) in slabs.iter().copied().enumerate() {
        let f1 = build_apartment_slab_floor(w, h, slab, 0, seed + i as u64 * 101);
        let f2 = build_apartment_slab_floor(w, h, slab, 1, seed + i as u64 * 101);
        FloorStack::absorb_ground(b, f1);
        stack.absorb_upper(f2);
        let walk_entry = if street_north {
            CardinalEntry::North
        } else {
            CardinalEntry::South
        };
        add_entry_walk(b, slab, walk_entry, SurfaceFeatureKind::ConcreteWalk, walk_y);
    }

    let _ = b.place_vehicle_along_ribbon(PropKind::Car, street_idx, 0.25, 0.20, 5);
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, street_idx, 0.72, -0.20, 7);
    let _ = b.try_place_off_road_cm(
        PropKind::Dumpster,
        OutdoorBuilder::cm((sx0 as f32 - 1.5).max(1.5)),
        OutdoorBuilder::cm(if street_north { hf - 1.5 } else { 1.5 }),
        4,
        None,
    );
    let _ = b.try_place_cm(
        PropKind::StreetDebris,
        OutdoorBuilder::cm(wf * 0.35),
        OutdoorBuilder::cm(walk_y),
        8,
        None,
        true,
    );
    dress_yard_props(b, tier, 2, 2);
}

/// Urban street neighborhoods deterministically varying by seed between Greenwich Village
/// brick brownstone terraces (`variant != 2`) and Chinatown mixed-use shophouses (`variant == 2`).
fn build_urban_rowhouse_neighborhood(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let chinatown = variant == 2;
    let street_north = !chinatown && variant == 1;
    let (street_y, walk_y, entry, lot_y0, lot_y1) = if street_north {
        let y0 = 6.min((h - 6).max(5));
        let y1 = (y0 + 11).min(h - 2).max(y0 + 6);
        (1.4, 3.55, CardinalEntry::North, y0, y1)
    } else {
        let y1 = (h - 6).max(8);
        let y0 = (y1 - 11).max(2).min(y1 - 5);
        (hf - 1.4, hf - 3.55, CardinalEntry::South, y0, y1)
    };

    b.add_polygon_surface(
        SurfaceFeatureKind::LawnGrassZone,
        &[(0.0, 0.0), (wf, 0.0), (wf, hf), (0.0, hf)],
        0,
        (seed as u32).wrapping_add(13),
    );
    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y),
            (wf * 0.45, street_y + 0.08),
            (wf, street_y - 0.06),
        ],
        300,
    );
    b.add_ribbon(
        SurfaceFeatureKind::BrickPaverWalk,
        &[(0.0, walk_y), (wf, walk_y)],
        170,
    );

    let (sx0, sx1) = safe_x_span(w);
    let span = sx1 - sx0;
    let pads = b.spawn_pads_m.clone();

    if chinatown {
        // Rear cobblestone/gravel delivery alley + striped market awnings + upstairs apartments.
        b.add_ribbon(
            SurfaceFeatureKind::GravelRoad,
            &[(0.0, 0.85), (wf, 0.85)],
            145,
        );
        let gap = 1;
        let count = (span / 7).clamp(2, 4);
        let each_w = ((span - gap * (count - 1)) / count).max(5);
        let arch_base = (seed as usize) % StoreArchetype::ALL.len();
        const AWNING_COLORS: [[u8; 3]; 4] = [
            [212, 48, 38],
            [32, 138, 90],
            [222, 162, 42],
            [30, 114, 140],
        ];
        for i in 0..count {
            let lx0 = sx0 + i * (each_w + gap);
            let lx1 = (lx0 + each_w).min(sx1);
            if lx1 - lx0 < 5 || lot_y1 - lot_y0 < 5 {
                continue;
            }
            let bay = LotBox::new(lx0, lot_y0, lx1, lot_y1);
            let arch =
                StoreArchetype::ALL[(arch_base + i as usize * 2) % StoreArchetype::ALL.len()];
            let store = generate_store_unit(
                w,
                h,
                bay,
                arch,
                CardinalEntry::South,
                true,
                &pads,
                seed.wrapping_add(i as u64 * 173),
            );
            FloorStack::absorb_ground(b, store);
            add_entry_walk(b, bay, CardinalEntry::South, SurfaceFeatureKind::BrickPaverWalk, walk_y);

            let mut awning = make_prop(
                PropKind::AwningCanopy,
                lx0,
                lot_y1,
                0,
                Some((bay.w() - 1).clamp(2, 6)),
                seed as u32 + i as u32,
            );
            awning.tint_rgb = Some(AWNING_COLORS[i as usize % AWNING_COLORS.len()]);
            b.props.push(awning);

            if bay.w() >= 5 && bay.h() >= 5 {
                let upstairs = generate_home_in_lot(
                    w,
                    h,
                    bay,
                    tier,
                    CardinalEntry::South,
                    true,
                    false,
                    false,
                    &[],
                    seed.wrapping_add(0x2F00 + i as u64 * 89),
                );
                for st in &upstairs.stair_anchors {
                    let stair_prop = make_prop(PropKind::Stairwell, st.x, st.y, st.rot_deg, None, 0);
                    b.props.retain(|existing| {
                        prop_spec(existing.kind).floor_layer
                            || !crate::tactical_map_geom::props_overlap_continuous(
                                existing,
                                &stair_prop,
                            )
                    });
                    b.props.push(stair_prop);
                }
                if let Some(f2) = upstairs.upper {
                    stack.absorb_upper(f2);
                }
            }
        }
    } else {
        // Greenwich Village Brick Brownstone terrace rowhouses sharing solid Brick party walls.
        let count = if span >= 28 {
            4
        } else if span >= 18 {
            3
        } else {
            2
        };
        let each_w = (span / count).max(5);
        let allow_basement = matches!(
            tier,
            EconomicTier::MiddleClass
                | EconomicTier::UpperMiddleClass
                | EconomicTier::LuxuryMansion
        );
        for i in 0..count {
            let lx0 = sx0 + i * each_w;
            let lx1 = if i + 1 == count {
                sx1
            } else {
                (lx0 + each_w).min(sx1)
            };
            if lx1 - lx0 < 5 || lot_y1 - lot_y0 < 5 {
                continue;
            }
            let unit_lot = LotBox::new(lx0, lot_y0, lx1, lot_y1);
            let home = generate_home_in_lot(
                w,
                h,
                unit_lot,
                tier,
                entry,
                unit_lot.w() >= 5 && unit_lot.h() >= 5,
                allow_basement && i == 0,
                true,
                &pads,
                seed.wrapping_add(i as u64 * 97),
            );
            stack.absorb_home(b, home, allow_basement && i == 0);
            add_entry_walk(b, unit_lot, entry, SurfaceFeatureKind::BrickPaverWalk, walk_y);

            let mx = unit_lot.x0 + unit_lot.w() / 2;
            let stoop_x = (mx - 1).clamp(unit_lot.x0, unit_lot.x1 - 2);
            let stoop_y = if street_north {
                (unit_lot.y0 - 2).max(1)
            } else {
                unit_lot.y1
            };
            b.props.push(make_prop(
                PropKind::BrownstoneStoop,
                stoop_x,
                stoop_y,
                if street_north { 180 } else { 0 },
                None,
                seed as u32 + i as u32,
            ));
        }

        // Convert shared neighbor party seams into solid brick fire-walls with zero interior windows.
        for i in 1..count {
            let party_x = sx0 + i * each_w;
            for edges in [&mut b.edges, &mut stack.f2_edges, &mut stack.b1_edges] {
                for e in edges.iter_mut() {
                    if e.x == party_x && e.side == EdgeSide::West && e.y >= lot_y0 && e.y < lot_y1 {
                        e.kind = WallEdgeKind::Wall;
                        e.material = MaterialClass::Brick;
                    }
                }
            }
        }

        if matches!(
            tier,
            EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
        ) {
            let (rear_y0, rear_y1, rear_depth) = if street_north {
                (lot_y1 as f32 + 0.6, (hf - 1.2).max(lot_y1 as f32 + 0.6), h - lot_y1)
            } else {
                (1.2_f32, (lot_y0 as f32 - 0.6).max(1.2), lot_y0)
            };
            if rear_depth >= 4 && b.rng.chance(9, 20) {
                let rear_rect = RectM {
                    x0: sx0 as f32 + 0.5,
                    y0: rear_y0,
                    x1: sx1 as f32 - 0.5,
                    y1: rear_y1,
                };
                let prefer_spa = rear_depth < 6 || b.rng.chance(2, 3);
                let _ = try_place_outdoor_pool_or_spa(
                    b,
                    rear_rect,
                    OutdoorPoolSpaOpts {
                        allow_pool: tier == EconomicTier::LuxuryMansion && rear_depth >= 6,
                        allow_spa: true,
                        prefer_spa_only: prefer_spa,
                    },
                );
            }
        }
    }

    let _ = b.place_vehicle_along_ribbon(PropKind::Car, street_idx, 0.28, 0.20, 6);
    if w >= 24 {
        let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, street_idx, 0.72, -0.20, 8);
    }
    let _ = b.try_place_off_road_cm(
        PropKind::Dumpster,
        OutdoorBuilder::cm((sx0 as f32 - 1.5).max(1.5)),
        OutdoorBuilder::cm(if street_north { hf - 1.5 } else { 1.5 }),
        4,
        None,
    );
    let _ = b.try_place_cm(
        PropKind::StreetDebris,
        OutdoorBuilder::cm(wf * 0.35),
        OutdoorBuilder::cm(walk_y),
        8,
        None,
        true,
    );
    dress_yard_props(b, tier, 4, 3);
}

/// Topology family for `SuburbanNeighborhood` (compact homes inside generous landscaped yards).
fn build_suburban_neighborhood(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let curved_top = variant == 1;
    let street_y = if curved_top { 1.45 } else { hf - 1.45 };
    let entry = if curved_top {
        CardinalEntry::North
    } else {
        CardinalEntry::South
    };
    let walk_y = if curved_top { 3.55 } else { hf - 3.55 };

    b.add_polygon_surface(
        SurfaceFeatureKind::LawnGrassZone,
        &[(0.0, 0.0), (wf, 0.0), (wf, hf), (0.0, hf)],
        0,
        (seed as u32).wrapping_add(19),
    );
    let bend = if variant == 2 { 0.45 } else { -0.35 };
    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y),
            (wf * 0.33, street_y + bend * 0.5),
            (wf * 0.67, street_y - bend * 0.4),
            (wf, street_y),
        ],
        300,
    );
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(0.0, walk_y), (wf, walk_y)],
        140,
    );

    let (sx0, sx1) = safe_x_span(w);
    let house_h = if h >= 24 {
        (h * 34 / 100).clamp(8, 12)
    } else if h >= 18 {
        (h * 44 / 100).clamp(6, 9)
    } else {
        (h - 7).clamp(5, 8)
    };
    let front_setback = if h >= 22 { 3 } else { 1 };
    let (lot_y0, lot_y1) = if curved_top {
        let y0 = (5 + front_setback).min(h - house_h - 2);
        (y0, (y0 + house_h).min(h - 2))
    } else {
        let y1 = (h - 5 - front_setback).max(6 + house_h.min(h - 7));
        ((y1 - house_h).max(2), y1)
    };

    let span = sx1 - sx0;
    let lots: Vec<LotBox> = if span >= 20 && variant != 2 {
        let mid = (sx0 + sx1) / 2;
        let max_w = if w >= 36 { 13 } else { 11 };
        let lw = ((span - 5) / 2).clamp(6, max_w);
        let gap = (span - lw * 2).max(4);
        let x_left = sx0 + ((span - (lw * 2 + gap)) / 2).max(0);
        let x_right = (mid + gap / 2).min(sx1 - lw);
        vec![
            LotBox::new(x_left, lot_y0, x_left + lw, lot_y1),
            LotBox::new(x_right, lot_y0, x_right + lw, lot_y1),
        ]
    } else {
        let single_w = if w >= 36 { 14 } else { (span - 4).clamp(7, 13) };
        let pad_x = ((span - single_w) / 2).max(0);
        vec![LotBox::new(sx0 + pad_x, lot_y0, sx1 - pad_x, lot_y1)]
    };

    let allow_basement = matches!(
        tier,
        EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
    );
    for (idx, lot) in lots.iter().copied().enumerate() {
        if lot.w() < 5 || lot.h() < 5 {
            continue;
        }
        let pads = b.spawn_pads_m.clone();
        let home = generate_home_in_lot(
            w,
            h,
            lot,
            tier,
            entry,
            lot.w() >= 6 && lot.h() >= 5,
            allow_basement && idx == 0,
            true,
            &pads,
            seed.wrapping_add(idx as u64 * 131),
        );
        stack.absorb_home(b, home, allow_basement && idx == 0);
        add_entry_walk(b, lot, entry, SurfaceFeatureKind::StoneWalk, walk_y);
        let drive_x = (lot.x0 as f32 - 1.5).max(1.5);
        let y_end = if curved_top {
            lot.y0 as f32
        } else {
            lot.y1 as f32
        };
        let drive_idx = b.add_ribbon(
            SurfaceFeatureKind::AsphaltRoad,
            &[(drive_x, street_y), (drive_x, y_end)],
            210,
        );
        if idx == 0 {
            let _ = b.place_vehicle_along_ribbon(PropKind::Car, drive_idx, 0.50, 0.0, 5);
        }
    }

    let _ = b.place_vehicle_along_ribbon(PropKind::Car, street_idx, 0.62, 0.20, 7);
    if !matches!(tier, EconomicTier::LowIncome) {
        let has_pool = b
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
        let want_amenity = if matches!(
            tier,
            EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
        ) {
            b.rng.chance(3, 5)
        } else {
            b.rng.chance(3, 10)
        };
        if want_amenity {
            let yard_bounds = RectM {
                x0: 1.5,
                y0: 1.5,
                x1: wf - 1.5,
                y1: hf - 1.5,
            };
            let prefer_spa = has_pool || b.rng.chance(2, 5);
            let _ = try_place_outdoor_pool_or_spa(
                b,
                yard_bounds,
                OutdoorPoolSpaOpts {
                    allow_pool: !has_pool,
                    allow_spa: true,
                    prefer_spa_only: prefer_spa,
                },
            );
        }
    }
    let rich_landscape = !matches!(tier, EconomicTier::LowIncome);
    let tree_budget = if rich_landscape {
        (w * h / 65).clamp(4, 16)
    } else {
        (w * h / 90).clamp(3, 12)
    };
    let patches = if rich_landscape { 6 } else { 4 };
    dress_yard_props(b, tier, tree_budget, patches);
}

/// Topology family for `LuxuryMansion` (walled motor-court compound, formal beds, `B1`+`1F`+`2F`).
fn build_luxury_mansion_site(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let gate_south = variant != 1;
    let street_y = if gate_south { hf - 1.45 } else { 1.45 };
    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y),
            (wf * 0.4, street_y + 0.12),
            (wf, street_y - 0.10),
        ],
        300,
    );

    let (sx0, sx1) = safe_x_span(w);
    let comp = LotBox::new(
        sx0.max(3),
        if gate_south { 2 } else { 3 },
        sx1.min(w - 3),
        if gate_south {
            (h - 3).max(10)
        } else {
            (h - 2).max(10)
        },
    );
    let manor_pad_x = if comp.w() >= 18 { 3 } else { 2 };
    let rear_lawn = if h >= 28 {
        8
    } else if h >= 26 {
        7
    } else if h >= 24 {
        6
    } else {
        0
    };
    let (manor_y0, manor_y1) = if gate_south {
        let y0 = (comp.y0 + 1 + rear_lawn).min((comp.y1 - 13).max(comp.y0 + 1));
        let y1 = (comp.y1 - 2).max(y0 + 5);
        (y0, y1)
    } else {
        let y0 = comp.y0 + 2;
        let y1 = (comp.y1 - 1 - rear_lawn).max((y0 + 11).min(comp.y1 - 1)).max(y0 + 5);
        (y0, y1)
    };
    let manor = LotBox::new(
        (comp.x0 + manor_pad_x).min(comp.x1 - 7),
        manor_y0,
        (comp.x1 - manor_pad_x).max(comp.x0 + 8),
        manor_y1,
    );
    let entry = if gate_south {
        CardinalEntry::South
    } else {
        CardinalEntry::North
    };
    let gate_x = (manor.x0 + manor.x1) / 2;

    let approach_y = if gate_south {
        manor.y1 as f32
    } else {
        manor.y0 as f32
    };
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[
            (gate_x as f32 + 0.5, street_y),
            (gate_x as f32 + 0.5, approach_y),
        ],
        210,
    );
    if variant == 2 && manor.x0 >= 6 {
        b.add_ribbon(
            SurfaceFeatureKind::GravelRoad,
            &[
                (comp.x0 as f32 - 0.5, street_y),
                (comp.x0 as f32 - 0.5, (manor.y0 + manor.y1) as f32 * 0.5),
            ],
            200,
        );
    }

    add_estate_perimeter(b, comp, gate_x, gate_south);
    let pads = b.spawn_pads_m.clone();
    let home = generate_home_in_lot(
        w,
        h,
        manor,
        tier,
        entry,
        true,
        true,
        true,
        &pads,
        seed ^ 0x00E5_7A7E,
    );
    stack.absorb_home(b, home, true);

    let has_pool = b
        .surfaces
        .iter()
        .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
    let rear_court = if gate_south {
        RectM {
            x0: comp.x0 as f32 + 0.8,
            y0: comp.y0 as f32 + 0.65,
            x1: comp.x1 as f32 - 0.8,
            y1: (manor.y0 as f32 - 0.65).max(comp.y0 as f32 + 0.65),
        }
    } else {
        RectM {
            x0: comp.x0 as f32 + 0.8,
            y0: manor.y1 as f32 + 0.65,
            x1: comp.x1 as f32 - 0.8,
            y1: (comp.y1 as f32 - 0.65).max(manor.y1 as f32 + 0.65),
        }
    };
    let comp_bounds = RectM {
        x0: comp.x0 as f32 + 0.8,
        y0: comp.y0 as f32 + 0.8,
        x1: comp.x1 as f32 - 0.8,
        y1: comp.y1 as f32 - 0.8,
    };
    let opts = OutdoorPoolSpaOpts {
        allow_pool: !has_pool,
        allow_spa: true,
        prefer_spa_only: has_pool,
    };
    if !try_place_outdoor_pool_or_spa(b, rear_court, opts) {
        let _ = try_place_outdoor_pool_or_spa(b, comp_bounds, opts);
    }

    let _ = b.place_vehicle_along_ribbon(PropKind::Car, street_idx, 0.35, 0.20, 4);
    dress_yard_props(b, tier, (w * h / 80).clamp(4, 14), 5);
}

/// Topology family for `RuralCountryside` (gravel road or winding creek/bridge, farmhouse + yard props).
fn build_rural_countryside(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let lane_y = if variant == 1 { 1.65 } else { hf - 1.65 };
    let lane_rect = RectM {
        x0: 1.0,
        y0: if variant == 1 { 1.15 } else { hf - 2.45 },
        x1: wf - 1.0,
        y1: if variant == 1 { 2.45 } else { hf - 1.15 },
    };
    let lane_y0 = lane_y + b.rng.f32_range(-0.25, 0.25);
    let lane_y1 = lane_y + b.rng.f32_range(-0.25, 0.25);
    let lane_pts = synthesize_natural_curve_m(
        &mut b.rng,
        (0.0, lane_y0),
        (wf, lane_y1),
        CurveSpec {
            profile: CurveProfile::CountryRoad,
            max_lateral_m: 0.65,
            ribbon_half_w_m: 1.25,
            clamp_rect: lane_rect,
        },
    );
    let lane_idx = b.add_ribbon(SurfaceFeatureKind::GravelRoad, &lane_pts, 250);

    let lot_y1_est = (h - 4).max(10) as f32;
    let mut creek_bridge = None;
    let mut creek_pts_opt = None;
    if variant == 1 && h >= 14 {
        let creek_y = hf - 2.15;
        let creek_rect = RectM {
            x0: 1.2,
            y0: (lot_y1_est + 1.15).min(creek_y - 0.15),
            x1: wf - 1.2,
            y1: (hf - 2.05).max(creek_y + 0.15),
        };
        let cy0 = creek_y + b.rng.f32_range(-0.25, 0.25);
        let cy1 = creek_y + b.rng.f32_range(-0.25, 0.25);
        let creek_pts = synthesize_natural_curve_m(
            &mut b.rng,
            (0.0, cy0),
            (wf, cy1),
            CurveSpec {
                profile: CurveProfile::WildStream,
                max_lateral_m: 0.75,
                ribbon_half_w_m: 1.0,
                clamp_rect: creek_rect,
            },
        );
        let creek = b.add_ribbon(SurfaceFeatureKind::CreekChannel, &creek_pts, 200);
        let br_frac = b.rng.f32_range(0.35, 0.65);
        creek_bridge = b.place_perpendicular_bridge(creek, br_frac, 2.2, 3.0);
        creek_pts_opt = Some(creek_pts);
    } else if variant == 2 {
        // Flanking roadside ditch on left wing, safely clear of center farmhouse drive.
        let ditch_y = (lane_y - 2.2).max(1.2);
        let ditch_rect = RectM {
            x0: 0.5,
            y0: 0.8,
            x1: wf * 0.35,
            y1: (lane_y - 1.9).max(1.5),
        };
        let dy1 = ditch_y + b.rng.f32_range(-0.25, 0.25);
        let ditch_pts = synthesize_natural_curve_m(
            &mut b.rng,
            (0.0, ditch_y),
            (wf * 0.28, dy1),
            CurveSpec {
                profile: CurveProfile::DrainageSwale,
                max_lateral_m: 0.45,
                ribbon_half_w_m: 0.68,
                clamp_rect: ditch_rect,
            },
        );
        b.add_ribbon(SurfaceFeatureKind::DrainageDitch, &ditch_pts, 135);
    }

    let (sx0, sx1) = safe_x_span(w);
    let lot_w = ((sx1 - sx0) * 62 / 100).clamp(6, 18);
    let lx0 = (sx0 + ((sx1 - sx0) - lot_w) / 2).max(sx0);
    let lot = if variant == 1 {
        LotBox::new(lx0, 4, lx0 + lot_w, (h - 4).max(10))
    } else {
        LotBox::new(lx0, 2, lx0 + lot_w, (h - 5).max(8))
    };
    let entry = if variant == 1 {
        CardinalEntry::North
    } else {
        CardinalEntry::South
    };
    let entry_m = (lot.x0 + lot.x1) as f32 * 0.5;
    let drive_end_y = if variant == 1 {
        lot.y0 as f32
    } else {
        lot.y1 as f32
    };
    let drive_pts = synthesize_natural_curve_m(
        &mut b.rng,
        (entry_m, lane_y),
        (entry_m, drive_end_y),
        CurveSpec {
            profile: CurveProfile::ForestTrail,
            max_lateral_m: 0.75,
            ribbon_half_w_m: 0.72,
            clamp_rect: RectM {
                x0: 1.0,
                y0: 0.8,
                x1: wf - 1.0,
                y1: hf - 0.8,
            },
        },
    );
    b.add_ribbon(SurfaceFeatureKind::DirtTrail, &drive_pts, 145);
    if let (Some(br), Some(creek_pts)) = (creek_bridge, creek_pts_opt.as_ref()) {
        let foot_start = (br.center_m.0, lot.y1 as f32 + 0.15);
        let foot_end_x = (br.center_m.0 + b.rng.f32_range(-1.0, 1.0)).clamp(1.0, wf - 1.0);
        let foot_pts = route_trail_across_bridge_m(
            &mut b.rng,
            foot_start,
            br,
            (foot_end_x, hf - 0.35),
            RectM {
                x0: 0.8,
                y0: lot.y1 as f32,
                x1: wf - 0.8,
                y1: hf - 0.35,
            },
            Some((creek_pts, 1.3)),
        );
        b.add_ribbon(SurfaceFeatureKind::DirtTrail, &foot_pts, 130);
    }

    let pads = b.spawn_pads_m.clone();
    let home = generate_home_in_lot(
        w,
        h,
        lot,
        tier,
        entry,
        lot.w() >= 8 && lot.h() >= 6,
        tier == EconomicTier::LuxuryMansion,
        true,
        &pads,
        seed ^ 0x00A9_41CE,
    );
    stack.absorb_home(b, home, tier == EconomicTier::LuxuryMansion);

    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, lane_idx, 0.30, 0.1, 10);
    let bounds = RectM {
        x0: 1.5,
        y0: 1.5,
        x1: wf - 1.5,
        y1: hf - 1.5,
    };
    if !matches!(tier, EconomicTier::LowIncome) {
        let has_pool = b
            .surfaces
            .iter()
            .any(|s| s.kind == SurfaceFeatureKind::PoolWater);
        let want_amenity = if matches!(
            tier,
            EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion
        ) {
            b.rng.chance(11, 20)
        } else {
            b.rng.chance(3, 10)
        };
        if want_amenity {
            let prefer_spa = has_pool || b.rng.chance(1, 2);
            let _ = try_place_outdoor_pool_or_spa(
                b,
                bounds,
                OutdoorPoolSpaOpts {
                    allow_pool: !has_pool,
                    allow_spa: true,
                    prefer_spa_only: prefer_spa,
                },
            );
        }
    }
    b.scatter_trees(PropKind::LargeTree, (w * h / 110).clamp(2, 8), 3.2, bounds);
    b.scatter_trees(PropKind::MediumTree, (w * h / 90).clamp(3, 10), 2.5, bounds);
    b.scatter_turf_prop_rot(PropKind::Boulder, 3, 25, bounds);
    dress_yard_props(b, tier, 2, 3);
}

/// Topology family for `DowntownCommercial` (multi-store strip + `2F` lofts/offices + vehicles).
fn build_downtown_commercial(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let street_north = variant == 1;
    let (street_y, walk_y, front_side, bay_y0, bay_y1) = if street_north {
        (1.45, 3.60, CardinalEntry::North, 5, (h - 2).max(11))
    } else {
        (
            hf - 1.45,
            hf - 3.60,
            CardinalEntry::South,
            2,
            (h - 5).max(8),
        )
    };

    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[
            (0.0, street_y),
            (wf * 0.5, street_y + 0.08),
            (wf, street_y - 0.06),
        ],
        300,
    );
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(0.0, walk_y), (wf, walk_y)],
        165,
    );
    if variant == 2 {
        let alley_y = if street_north { hf - 0.85 } else { 0.85 };
        b.add_ribbon(
            SurfaceFeatureKind::GravelRoad,
            &[(0.0, alley_y), (wf, alley_y)],
            150,
        );
    }

    let (sx0, sx1) = safe_x_span(w);
    let total_span = sx1 - sx0;
    let gap = 2;
    let store_count = if total_span >= 32 {
        3
    } else if total_span >= 14 {
        2
    } else {
        1
    };
    let each_w = ((total_span - gap * (store_count - 1)) / store_count).max(5);
    let arch_base = (seed as usize) % StoreArchetype::ALL.len();

    for i in 0..store_count {
        let bx0 = sx0 + i * (each_w + gap);
        let bx1 = (bx0 + each_w).min(sx1);
        if bx1 - bx0 < 5 || bay_y1 - bay_y0 < 5 {
            continue;
        }
        let bay = LotBox::new(bx0, bay_y0, bx1, bay_y1);
        let arch = StoreArchetype::ALL[(arch_base + i as usize * 2) % StoreArchetype::ALL.len()];
        let pads = b.spawn_pads_m.clone();
        let store = generate_store_unit(
            w,
            h,
            bay,
            arch,
            front_side,
            true,
            &pads,
            seed.wrapping_add(i as u64 * 173),
        );
        FloorStack::absorb_ground(b, store);
        add_entry_walk(b, bay, front_side, SurfaceFeatureKind::ConcreteWalk, walk_y);

        if i == 0 && bay.w() >= 6 && bay.h() >= 5 {
            let upstairs = generate_home_in_lot(
                w,
                h,
                bay,
                tier,
                front_side,
                true,
                false,
                false,
                &[],
                seed.wrapping_add(0x2F00 + i as u64),
            );
            for st in &upstairs.stair_anchors {
                let stair_prop = make_prop(PropKind::Stairwell, st.x, st.y, st.rot_deg, None, 0);
                // Clear any ground-floor solid prop overlapped by the vertical stairwell landing.
                b.props.retain(|existing| {
                    prop_spec(existing.kind).floor_layer
                        || !crate::tactical_map_geom::props_overlap_continuous(
                            existing,
                            &stair_prop,
                        )
                });
                b.props.push(stair_prop);
            }
            if let Some(f2) = upstairs.upper {
                stack.absorb_upper(f2);
            }
        }
    }

    let _ = b.place_vehicle_along_ribbon(PropKind::Car, street_idx, 0.25, 0.20, 5);
    let _ = b.place_vehicle_along_ribbon(PropKind::UtilityVan, street_idx, 0.72, -0.20, 6);
    let bounds = RectM {
        x0: 1.5,
        y0: 1.5,
        x1: wf - 1.5,
        y1: hf - 1.5,
    };
    b.scatter_turf_prop_rot(PropKind::ParkBench, 2, 45, bounds);
    b.scatter_turf_prop_rot(PropKind::Planter, 2, 45, bounds);
    let _ = b.try_place_off_road_cm(
        PropKind::Dumpster,
        OutdoorBuilder::cm((sx0 as f32 - 1.4).max(1.4)),
        OutdoorBuilder::cm(if street_north { hf - 1.5 } else { 1.5 }),
        2,
        None,
    );
}

/// Generates a decoupled zoned [`ProceduralSector`] for `preset` honoring optional `tier_override`
/// and `zoning_override` across multiple seeded topologies and vertical floor stacks (`B1`/`1F`/`2F`).
#[must_use]
pub fn generate_zoned_sector(
    preset: SectorPreset,
    width: i32,
    height: i32,
    seed: i64,
    tier_override: Option<EconomicTier>,
    zoning_override: Option<ZoningContext>,
) -> ProceduralSector {
    let w = width.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    let h = height.clamp(
        crate::tactical_map::MIN_SECTOR_DIM,
        crate::tactical_map::MAX_SECTOR_DIM,
    );
    let (def_zone, def_tier) = default_zoning_and_tier(preset);
    let zone = zoning_override.unwrap_or(def_zone);
    let tier = tier_override.unwrap_or(def_tier);
    let variant = (seed.unsigned_abs() % 3) as usize;

    let mut b = OutdoorBuilder::new(w, h, seed);
    let mut stack = FloorStack::default();

    if preset == SectorPreset::LuxuryMansion && zoning_override.is_none() {
        build_luxury_mansion_site(&mut b, &mut stack, tier, variant, seed as u64);
    } else {
        match zone {
            ZoningContext::UrbanResidential => {
                build_urban_rowhouse_neighborhood(&mut b, &mut stack, tier, variant, seed as u64);
            }
            ZoningContext::ProjectsHighDensity => {
                build_projects_complex(&mut b, &mut stack, tier, variant, seed as u64);
            }
            ZoningContext::SuburbanNeighborhood => {
                if tier == EconomicTier::LuxuryMansion {
                    build_luxury_mansion_site(&mut b, &mut stack, tier, variant, seed as u64);
                } else {
                    build_suburban_neighborhood(&mut b, &mut stack, tier, variant, seed as u64);
                }
            }
            ZoningContext::RuralCountryside => {
                build_rural_countryside(&mut b, &mut stack, tier, variant, seed as u64);
            }
            ZoningContext::DowntownCommercial => {
                build_downtown_commercial(&mut b, &mut stack, tier, variant, seed as u64);
            }
        }
    }

    stack.into_sector(b, w, h)
}
