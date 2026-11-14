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
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

/// Returns the canonical `(ZoningContext, EconomicTier)` pair for a zoned [`SectorPreset`].
#[must_use]
pub const fn default_zoning_and_tier(preset: SectorPreset) -> (ZoningContext, EconomicTier) {
    match preset {
        SectorPreset::UrbanResidential => (ZoningContext::UrbanResidential, EconomicTier::MiddleClass),
        SectorPreset::ProjectsHighDensity => (ZoningContext::ProjectsHighDensity, EconomicTier::LowIncome),
        SectorPreset::SuburbanNeighborhood => {
            (ZoningContext::SuburbanNeighborhood, EconomicTier::UpperMiddleClass)
        }
        SectorPreset::RuralCountryside => (ZoningContext::RuralCountryside, EconomicTier::LowIncome),
        SectorPreset::LuxuryMansion => (ZoningContext::SuburbanNeighborhood, EconomicTier::LuxuryMansion),
        SectorPreset::DowntownCommercial => {
            (ZoningContext::DowntownCommercial, EconomicTier::UpperMiddleClass)
        }
        _ => (ZoningContext::UrbanResidential, EconomicTier::MiddleClass),
    }
}

/// Accumulates `B1` cellar and `2F` upper levels across one or more buildings in the sector.
#[derive(Default)]
struct FloorStack {
    b1_edges: Vec<MapWallEdge>,
    b1_props: Vec<MapPropPlacement>,
    b1_surfaces: Vec<MapSurfaceFeature>,
    f2_edges: Vec<MapWallEdge>,
    f2_props: Vec<MapPropPlacement>,
    f2_surfaces: Vec<MapSurfaceFeature>,
}

impl FloorStack {
    fn absorb_ground(b: &mut OutdoorBuilder, ground: FloorContent) {
        for e in ground.wall_edges {
            b.put_edge(e.x, e.y, e.side, e.kind, e.material);
        }
        b.props.extend(ground.props);
        b.surfaces.extend(ground.surfaces);
    }

    fn absorb_home(&mut self, b: &mut OutdoorBuilder, home: GeneratedHome, allow_basement: bool) {
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

    fn absorb_upper(&mut self, f2: FloorContent) {
        self.f2_edges.extend(f2.wall_edges);
        self.f2_props.extend(f2.props);
        self.f2_surfaces.extend(f2.surfaces);
    }

    fn into_sector(mut self, b: OutdoorBuilder, w: i32, h: i32) -> ProceduralSector {
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
            });
            floors.push(MapFloorLevel {
                floor_index: 1,
                label: "2F · Upper Level".to_string(),
                elevation_cm: 340,
                wall_edges: self.f2_edges,
                props: self.f2_props,
                surfaces: self.f2_surfaces,
            });
        } else if !floors.is_empty() {
            floors.push(MapFloorLevel {
                floor_index: 0,
                label: "1F · Ground".to_string(),
                elevation_cm: 0,
                wall_edges: sec.wall_edges.clone(),
                props: sec.props.clone(),
                surfaces: sec.surfaces.clone(),
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
    let pad = if w >= 16 { 4 } else if w >= 12 { 3 } else { 2 };
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
    let bounds = RectM { x0: 1.5, y0: 1.5, x1: b.w as f32 - 1.5, y1: b.h as f32 - 1.5 };
    for i in 0..patches {
        let px = b.rng.f32_range(bounds.x0 + 0.5, (bounds.x1 - 0.5).max(bounds.x0 + 1.0));
        let py = b.rng.f32_range(bounds.y0 + 0.5, (bounds.y1 - 0.5).max(bounds.y0 + 1.0));
        let kind = if i % 2 == 0 {
            SurfaceFeatureKind::GardenBedZone
        } else {
            SurfaceFeatureKind::MulchZone
        };
        let rad = if tier == EconomicTier::LuxuryMansion { 1.5 } else { 1.1 };
        b.try_add_organic_patch(kind, (px, py), (rad, rad * 0.8), 0.30, 28);
    }
    if tree_budget > 0 {
        b.scatter_trees(PropKind::MediumTree, tree_budget, 2.6, bounds);
        b.scatter_turf_prop_rot(PropKind::Bush, (tree_budget + 1).max(2), 35, bounds);
    }
    b.scatter_turf_prop_rot(PropKind::ParkBench, 2, 45, bounds);
    if matches!(tier, EconomicTier::LowIncome | EconomicTier::MiddleClass) {
        b.scatter_turf_prop_rot(PropKind::WoodCrate, 2, 30, bounds);
    }
}

/// Places a compound estate perimeter wall around `comp` with two wide vehicular/foot gates.
fn add_estate_perimeter(b: &mut OutdoorBuilder, comp: LotBox, gate_x: i32, gate_south: bool) {
    for x in comp.x0..comp.x1 {
        let is_gate = x == gate_x || x == gate_x + 1;
        let k = if is_gate { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
        let m = if is_gate { MaterialClass::SheetSteel } else { MaterialClass::ConcreteBlock };
        b.put_edge(
            x,
            comp.y0,
            EdgeSide::North,
            if !gate_south && is_gate { k } else { WallEdgeKind::Wall },
            if !gate_south && is_gate { m } else { MaterialClass::ConcreteBlock },
        );
        b.put_edge(
            x,
            comp.y1,
            EdgeSide::North,
            if gate_south && is_gate { k } else { WallEdgeKind::Wall },
            if gate_south && is_gate { m } else { MaterialClass::ConcreteBlock },
        );
    }
    for y in comp.y0..comp.y1 {
        let is_side_door = y == (comp.y0 + comp.y1) / 2;
        let k = if is_side_door { WallEdgeKind::DoorOpen } else { WallEdgeKind::Wall };
        let m = if is_side_door { MaterialClass::SheetSteel } else { MaterialClass::ConcreteBlock };
        b.put_edge(comp.x0, y, EdgeSide::West, k, m);
        b.put_edge(comp.x1, y, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::ConcreteBlock);
    }
}

/// Topology family for `UrbanResidential` and `ProjectsHighDensity` city blocks (`1F` + `2F` + opt `B1`).
fn build_urban_block(
    b: &mut OutdoorBuilder,
    stack: &mut FloorStack,
    zone: ZoningContext,
    tier: EconomicTier,
    variant: usize,
    seed: u64,
) {
    let (w, h) = (b.w, b.h);
    let (wf, hf) = (w as f32, h as f32);
    let high_density = zone == ZoningContext::ProjectsHighDensity;
    let street_north = variant == 1;
    let split_plaza = variant == 2 && w >= 24;

    // Street band ~3.0m wide at map edge, 1.4m sidewalk beside it, 1.0m front setback -> lot >= 7m deep.
    let (street_y, walk_y, entry, lot_y0, lot_y1) = if street_north {
        (1.4, 3.55, CardinalEntry::North, 5, (h - 2).max(11))
    } else {
        (hf - 1.4, hf - 3.55, CardinalEntry::South, 2, (h - 5).max(8))
    };

    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(0.0, street_y), (wf * 0.45, street_y + 0.10), (wf, street_y - 0.08)],
        300,
    );
    b.add_ribbon(
        SurfaceFeatureKind::ConcreteWalk,
        &[(0.0, walk_y), (wf, walk_y)],
        150,
    );

    let (sx0, sx1) = safe_x_span(w);
    let span = sx1 - sx0;
    let lot_gap = if high_density { 2 } else { 3 };
    let count = if span >= 34 {
        3
    } else if span >= 16 || split_plaza {
        2
    } else {
        1
    };
    let each_w = ((span - lot_gap * (count - 1)) / count).max(5);
    let allow_basement = !high_density
        && matches!(tier, EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion);

    for i in 0..count {
        let lx0 = sx0 + i * (each_w + lot_gap);
        let lx1 = (lx0 + each_w).min(sx1);
        if lx1 - lx0 < 5 || lot_y1 - lot_y0 < 5 {
            continue;
        }
        let lot = LotBox::new(lx0, lot_y0, lx1, lot_y1);
        let pads = b.spawn_pads_m.clone();
        let home = generate_home_in_lot(
            w,
            h,
            lot,
            tier,
            entry,
            lot.w() >= 6 && lot.h() >= 5,
            allow_basement && i == 0,
            true,
            &pads,
            seed.wrapping_add(i as u64 * 97),
        );
        stack.absorb_home(b, home, allow_basement && i == 0);
        add_entry_walk(b, lot, entry, SurfaceFeatureKind::StoneWalk, walk_y);
    }

    if variant == 2 {
        let alley_y = if street_north { hf - 0.85 } else { 0.85 };
        b.add_ribbon(SurfaceFeatureKind::GravelRoad, &[(0.0, alley_y), (wf, alley_y)], 150);
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
    dress_yard_props(b, tier, if high_density { 2 } else { 4 }, if high_density { 2 } else { 3 });
}

/// Topology family for `SuburbanNeighborhood` (detached 1–2 house yards, driveways, curved road).
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
    let entry = if curved_top { CardinalEntry::North } else { CardinalEntry::South };
    let walk_y = if curved_top { 3.55 } else { hf - 3.55 };

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
    b.add_ribbon(SurfaceFeatureKind::ConcreteWalk, &[(0.0, walk_y), (wf, walk_y)], 140);

    let (sx0, sx1) = safe_x_span(w);
    let lot_y0 = if curved_top { 5 } else { 2 };
    let lot_y1 = if curved_top { (h - 2).max(11) } else { (h - 5).max(8) };
    let lots: Vec<LotBox> = if sx1 - sx0 >= 20 && variant != 2 {
        let mid = (sx0 + sx1) / 2;
        let gap = if tier == EconomicTier::LuxuryMansion { 4 } else { 3 };
        vec![
            LotBox::new(sx0, lot_y0, mid - gap / 2, lot_y1),
            LotBox::new(mid + (gap + 1) / 2, lot_y0, sx1, lot_y1),
        ]
    } else {
        let inset = if sx1 - sx0 >= 16 { 2 } else { 0 };
        vec![LotBox::new(sx0 + inset, lot_y0, sx1 - inset, lot_y1)]
    };

    let allow_basement = matches!(tier, EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion);
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
        let y_end = if curved_top { lot.y0 as f32 } else { lot.y1 as f32 };
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
    dress_yard_props(b, tier, (w * h / 90).clamp(3, 12), 4);
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
        &[(0.0, street_y), (wf * 0.4, street_y + 0.12), (wf, street_y - 0.10)],
        300,
    );

    let (sx0, sx1) = safe_x_span(w);
    let comp = LotBox::new(
        sx0.max(3),
        if gate_south { 2 } else { 3 },
        sx1.min(w - 3),
        if gate_south { (h - 3).max(10) } else { (h - 2).max(10) },
    );
    let manor_pad_x = if comp.w() >= 18 { 3 } else { 2 };
    let manor_y0 = if gate_south { comp.y0 + 1 } else { comp.y0 + 2 };
    let manor_y1 = if gate_south { (comp.y1 - 2).max(manor_y0 + 5) } else { (comp.y1 - 1).max(manor_y0 + 5) };
    let manor = LotBox::new(
        (comp.x0 + manor_pad_x).min(comp.x1 - 7),
        manor_y0,
        (comp.x1 - manor_pad_x).max(comp.x0 + 8),
        manor_y1,
    );
    let entry = if gate_south { CardinalEntry::South } else { CardinalEntry::North };
    let gate_x = (manor.x0 + manor.x1) / 2;

    let approach_y = if gate_south { manor.y1 as f32 } else { manor.y0 as f32 };
    b.add_ribbon(
        SurfaceFeatureKind::StoneWalk,
        &[(gate_x as f32 + 0.5, street_y), (gate_x as f32 + 0.5, approach_y)],
        210,
    );
    if variant == 2 && manor.x0 >= 6 {
        b.add_ribbon(
            SurfaceFeatureKind::GravelRoad,
            &[(comp.x0 as f32 - 0.5, street_y), (comp.x0 as f32 - 0.5, (manor.y0 + manor.y1) as f32 * 0.5)],
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
    let lane_idx = b.add_ribbon(
        SurfaceFeatureKind::GravelRoad,
        &[
            (0.0, lane_y + 0.20),
            (wf * 0.32, lane_y - 0.28),
            (wf * 0.68, lane_y + 0.25),
            (wf, lane_y - 0.12),
        ],
        250,
    );

    if variant == 1 && h >= 14 {
        let creek_y = hf - 1.75;
        let creek = b.add_ribbon(
            SurfaceFeatureKind::CreekChannel,
            &[
                (0.0, creek_y - 0.15),
                (wf * 0.35, creek_y + 0.45),
                (wf * 0.70, creek_y - 0.35),
                (wf, creek_y + 0.15),
            ],
            200,
        );
        b.add_bridge_deck_across(creek, 0.45, 2.2, 3.0);
    } else if variant == 2 {
        // Flanking roadside ditch on left wing, safely clear of center farmhouse drive.
        let ditch_y = (lane_y - 2.2).max(1.2);
        b.add_ribbon(
            SurfaceFeatureKind::DrainageDitch,
            &[(0.0, ditch_y), (wf * 0.28, ditch_y + 0.2)],
            135,
        );
    }

    let (sx0, sx1) = safe_x_span(w);
    let lot_w = ((sx1 - sx0) * 62 / 100).clamp(6, 18);
    let lx0 = (sx0 + ((sx1 - sx0) - lot_w) / 2).max(sx0);
    let lot = if variant == 1 {
        LotBox::new(lx0, 4, lx0 + lot_w, (h - 4).max(10))
    } else {
        LotBox::new(lx0, 2, lx0 + lot_w, (h - 5).max(8))
    };
    let entry = if variant == 1 { CardinalEntry::North } else { CardinalEntry::South };
    let entry_m = (lot.x0 + lot.x1) as f32 * 0.5;
    b.add_ribbon(
        SurfaceFeatureKind::DirtTrail,
        &[
            (entry_m, lane_y),
            (entry_m + 0.25, (lane_y + if variant == 1 { 1.1 } else { -1.2 })),
            (entry_m, if variant == 1 { lot.y0 as f32 } else { lot.y1 as f32 }),
        ],
        145,
    );

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
    let bounds = RectM { x0: 1.5, y0: 1.5, x1: wf - 1.5, y1: hf - 1.5 };
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
        (hf - 1.45, hf - 3.60, CardinalEntry::South, 2, (h - 5).max(8))
    };

    let street_idx = b.add_ribbon(
        SurfaceFeatureKind::AsphaltRoad,
        &[(0.0, street_y), (wf * 0.5, street_y + 0.08), (wf, street_y - 0.06)],
        300,
    );
    b.add_ribbon(SurfaceFeatureKind::ConcreteWalk, &[(0.0, walk_y), (wf, walk_y)], 165);
    if variant == 2 {
        let alley_y = if street_north { hf - 0.85 } else { 0.85 };
        b.add_ribbon(SurfaceFeatureKind::GravelRoad, &[(0.0, alley_y), (wf, alley_y)], 150);
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
                        || !crate::tactical_map_geom::props_overlap_continuous(existing, &stair_prop)
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
    let bounds = RectM { x0: 1.5, y0: 1.5, x1: wf - 1.5, y1: hf - 1.5 };
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
    let w = width.clamp(crate::tactical_map::MIN_SECTOR_DIM, crate::tactical_map::MAX_SECTOR_DIM);
    let h = height.clamp(crate::tactical_map::MIN_SECTOR_DIM, crate::tactical_map::MAX_SECTOR_DIM);
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
            ZoningContext::UrbanResidential | ZoningContext::ProjectsHighDensity => {
                build_urban_block(&mut b, &mut stack, zone, tier, variant, seed as u64);
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
