//! # *Awakened* 2035 Procedural Venue Generators (`map_gen_awakened`)
//!
//! Four scenario-grounded 2035 near-future battlemap generators pairing bespoke 2035 props
//! (`CryoComputeCluster`, `BioServerVat`, `AirGappedTerminal`, `BiometricTurnstile`,
//! `SecurityCameraMast`, `DroneDockPad`, `HvTransformer`, `DieselGenerator`, `BatteryBankWall`,
//! `WaterPurifierKiosk`) and surfaces (`FaradayCopperFloor`, `AntiStaticCleanroomFloor`,
//! `SubstationGrating`, `FloodedAsphalt`) with Faraday shielding and surveillance metadata:
//! - [`SectorPreset::CommunityClinicAnnex`] (*Episode 1: Cold Storage at St. Jude Annex*, `1F` + `B1` vault)
//! - [`SectorPreset::CorporateServerBunker`] (Hardened AI 2.0 cryo-compute hall + biometric mantrap)
//! - [`SectorPreset::SatyangSafehouse`] (Covert copper-mesh shophouse & `B1` SIGINT sanctuary)
//! - [`SectorPreset::BrownoutLogisticsDepot`] (Logistics bay + fenced `500 kV` substation + wet-bulb shelter)

use satya_schema::{
    EdgeSide, MapFaradayZone, MapFloorLevel, MapPropPlacement, MapSurveillanceNode,
    MapWallEdge, MaterialClass, MessageVisibility, PropKind, SurfaceFeatureKind, WallEdgeKind,
};

use crate::map_dressing::{make_prop, prop_spec};
use crate::map_gen_residential::{LotBox, trim_floor_props_to_connected};
use crate::map_outdoor_gen::OutdoorBuilder;
use crate::map_surface_queries::prune_occluded_surfaces;
use crate::tactical_map::{ProceduralSector, SectorPreset};

struct FloorCanvas<'a> {
    edges: &'a mut Vec<MapWallEdge>,
    w: i32,
    h: i32,
}

impl<'a> FloorCanvas<'a> {
    fn new(edges: &'a mut Vec<MapWallEdge>, w: i32, h: i32) -> Self {
        Self { edges, w, h }
    }

    fn upsert(
        &mut self,
        x: i32,
        y: i32,
        side: EdgeSide,
        kind: WallEdgeKind,
        material: MaterialClass,
    ) {
        let valid = match side {
            EdgeSide::West => x >= 0 && x <= self.w && y >= 0 && y < self.h,
            EdgeSide::North => x >= 0 && x < self.w && y >= 0 && y <= self.h,
        };
        if !valid {
            return;
        }
        let e = MapWallEdge {
            x,
            y,
            side,
            kind,
            material,
        };
        match self
            .edges
            .iter_mut()
            .find(|o| o.x == x && o.y == y && o.side == side)
        {
            Some(slot) => *slot = e,
            None => self.edges.push(e),
        }
    }

    fn enclose(&mut self, lot: LotBox, wall_mat: MaterialClass, win_step: i32) {
        for x in lot.x0..lot.x1 {
            let win = win_step > 0 && (x - lot.x0) % win_step == 1 && x + 1 < lot.x1;
            let (k, m) = if win {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, wall_mat)
            };
            self.upsert(x, lot.y0, EdgeSide::North, k, m);
            self.upsert(x, lot.y1, EdgeSide::North, k, m);
        }
        for y in lot.y0..lot.y1 {
            let win = win_step > 0 && (y - lot.y0) % win_step == 1 && y + 1 < lot.y1;
            let (k, m) = if win {
                (WallEdgeKind::Window, MaterialClass::Glass)
            } else {
                (WallEdgeKind::Wall, wall_mat)
            };
            self.upsert(lot.x0, y, EdgeSide::West, k, m);
            self.upsert(lot.x1, y, EdgeSide::West, k, m);
        }
    }

    fn v_partition(
        &mut self,
        x: i32,
        y0: i32,
        y1: i32,
        door_y: i32,
        open: bool,
        wall_mat: MaterialClass,
    ) {
        let dk = if open {
            WallEdgeKind::DoorOpen
        } else {
            WallEdgeKind::DoorClosed
        };
        for y in y0..y1 {
            if y == door_y {
                self.upsert(x, y, EdgeSide::West, dk, MaterialClass::SheetSteel);
            } else {
                self.upsert(x, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
            }
        }
    }

    fn h_partition(
        &mut self,
        y: i32,
        x0: i32,
        x1: i32,
        door_x: i32,
        open: bool,
        wall_mat: MaterialClass,
    ) {
        let dk = if open {
            WallEdgeKind::DoorOpen
        } else {
            WallEdgeKind::DoorClosed
        };
        for x in x0..x1 {
            if x == door_x {
                self.upsert(x, y, EdgeSide::North, dk, MaterialClass::SheetSteel);
            } else {
                self.upsert(x, y, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
            }
        }
    }
}

fn safe_interior_lot(w: i32, h: i32, has_south_street: bool) -> LotBox {
    let lx0 = if w >= 16 { 4 } else if w >= 12 { 3 } else { 1 };
    let lx1 = if w >= 16 { w - 4 } else if w >= 12 { w - 3 } else { w - 1 };
    let ly0 = if h >= 14 { 2 } else { 1 };
    let ly1 = if has_south_street && h >= 14 {
        h - 4
    } else {
        h - 1
    };
    LotBox::new(lx0, ly0, lx1.max(lx0 + 4), ly1.max(ly0 + 4))
}

fn blocks_any_door(edges: &[MapWallEdge], cand: &MapPropPlacement) -> bool {
    edges.iter().any(|e| {
        matches!(e.kind, WallEdgeKind::DoorClosed | WallEdgeKind::DoorOpen)
            && match e.side {
                EdgeSide::West => cand.covers(e.x - 1, e.y) || cand.covers(e.x, e.y),
                EdgeSide::North => cand.covers(e.x, e.y - 1) || cand.covers(e.x, e.y),
            }
    })
}

fn try_push_prop(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    spawn_pads: &[(f32, f32)],
    w: i32,
    h: i32,
    cand: MapPropPlacement,
) -> bool {
    let fp = cand.footprint_m();
    let (sw_cm, sh_cm) = cand.local_span_cm();
    let (hw_cm, hh_cm) = crate::map_dressing::rotated_aabb_half_extents_cm(
        sw_cm,
        sh_cm,
        cand.rotation_deg,
    );
    let (hw, hh) = (hw_cm / 100.0, hh_cm / 100.0);
    if fp.cx_m - hw < 0.05
        || fp.cy_m - hh < 0.05
        || fp.cx_m + hw > w as f32 - 0.05
        || fp.cy_m + hh > h as f32 - 0.05
    {
        return false;
    }
    if crate::tactical_map_geom::prop_crosses_wall_edge(&cand, edges) {
        return false;
    }
    if !prop_spec(cand.kind).floor_layer || cand.kind.is_vertical_portal() {
        if blocks_any_door(edges, &cand) {
            return false;
        }
        for &(sx, sy) in spawn_pads {
            if crate::tactical_map_geom::prop_intersects_circle(&cand, sx, sy, 0.85) {
                return false;
            }
        }
        for ex in props.iter() {
            if (!prop_spec(ex.kind).floor_layer || ex.kind.is_vertical_portal())
                && crate::tactical_map_geom::props_overlap_continuous(&cand, ex)
            {
                return false;
            }
        }
    }
    props.push(cand);
    true
}

fn push_props_checked(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    spawn_pads: &[(f32, f32)],
    w: i32,
    h: i32,
    cands: impl IntoIterator<Item = MapPropPlacement>,
) {
    for cand in cands {
        let _ = try_push_prop(edges, props, spawn_pads, w, h, cand);
    }
}

fn try_push_first(
    edges: &[MapWallEdge],
    props: &mut Vec<MapPropPlacement>,
    spawn_pads: &[(f32, f32)],
    w: i32,
    h: i32,
    cands: impl IntoIterator<Item = MapPropPlacement>,
) {
    for cand in cands {
        if try_push_prop(edges, props, spawn_pads, w, h, cand) {
            break;
        }
    }
}

fn furnish_clinic_pharmacy_and_restroom(
    b: &mut OutdoorBuilder,
    w: i32,
    h: i32,
    pharm: LotBox,
    bath: Option<LotBox>,
    s: u32,
) {
    let p_cands = [
        make_prop(PropKind::MedicalCounter, pharm.x0 + 1, pharm.y0, 0, Some(2), s + 100),
        make_prop(PropKind::Fridge, pharm.x1 - 1, pharm.y0, 0, None, s + 101),
        make_prop(PropKind::SteelShelving, pharm.x1 - 1, pharm.y0 + 2, 90, None, s + 102),
        make_prop(PropKind::LabBench, pharm.x0 + 1, pharm.y0 + 3, 0, None, s + 103),
        make_prop(PropKind::DeskClutter, pharm.x0 + 1, pharm.y0 + 3, 0, None, s + 104),
        make_prop(PropKind::FilingCabinet, pharm.x0, pharm.y1 - 1, 0, None, s + 105),
        make_prop(PropKind::BiohazardBin, pharm.x1 - 1, pharm.y1 - 1, 0, None, s + 106),
        make_prop(PropKind::TrashCan, pharm.x0 + 1, pharm.y1 - 2, 0, None, s + 107),
    ];
    push_props_checked(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, p_cands);

    if let Some(br) = bath {
        let b_cands = [
            make_prop(PropKind::Toilet, br.x0, br.y0, 0, None, s + 110),
            make_prop(PropKind::BathroomSink, br.x0, br.y1 - 1, 90, None, s + 111),
            make_prop(PropKind::TrashCan, br.x1 - 1, br.y0, 0, None, s + 112),
        ];
        push_props_checked(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, b_cands);
    }
}

#[allow(clippy::too_many_arguments)]
fn furnish_clinic_intake_lobby(
    b: &mut OutdoorBuilder,
    w: i32,
    h: i32,
    lot: LotBox,
    split_x: i32,
    front_door_x: i32,
    lobby_top_y: i32,
    has_pharm: bool,
    s: u32,
) {
    let lobby_w = split_x - lot.x0;
    let lobby_h = lot.y1 - lobby_top_y;

    // Reception desk & intake checkpoint (kept >= 1m clear of front door & spine doors).
    try_push_first(
        &b.edges,
        &mut b.props,
        &b.spawn_pads_m,
        w,
        h,
        [
            make_prop(PropKind::NurseStation, lot.x0 + 1, lot.y1 - 4, 0, None, s + 120),
            make_prop(PropKind::NurseStation, lot.x0 + 1, lot.y1 - 3, 0, None, s + 121),
        ],
    );
    push_props_checked(
        &b.edges,
        &mut b.props,
        &b.spawn_pads_m,
        w,
        h,
        [
            make_prop(PropKind::DeskClutter, lot.x0 + 2, lot.y1 - 4, 0, None, s + 122),
            make_prop(PropKind::BiometricTurnstile, front_door_x + 1, lot.y1 - 1, 0, None, s + 123),
            make_prop(PropKind::WaterPurifierKiosk, split_x - 1, lot.y1 - 1, 0, None, s + 124),
            make_prop(PropKind::WaterCooler, split_x - 2, lot.y0 + 4, 0, None, s + 125),
            make_prop(PropKind::FireExtinguisher, split_x - 1, lot.y0 + 3, 0, None, s + 126),
            make_prop(PropKind::FilingCabinet, lot.x0, lot.y1 - 4, 270, None, s + 127),
            make_prop(PropKind::TrashCan, lot.x0, lot.y1 - 1, 0, None, s + 128),
            make_prop(PropKind::Planter, front_door_x - 1, lot.y1 - 1, 0, None, s + 129),
            make_prop(PropKind::Planter, split_x - 1, lot.y1 - 2, 0, None, s + 130),
        ],
    );

    if !has_pharm {
        // Compact 24x18 lobby lacks a walled pharmacy, so put intake workstation & supply storage in NW corner.
        push_props_checked(
            &b.edges,
            &mut b.props,
            &b.spawn_pads_m,
            w,
            h,
            [
                make_prop(PropKind::Desk, lot.x0 + 1, lot.y0, 0, None, s + 131),
                make_prop(PropKind::DeskClutter, lot.x0 + 1, lot.y0, 0, None, s + 132),
                make_prop(PropKind::SteelShelving, lot.x0, lot.y0 + 2, 270, None, s + 133),
                make_prop(PropKind::Fridge, lot.x0 + 3, lot.y0, 0, None, s + 134),
                make_prop(PropKind::MedicalCounter, lot.x0 + 4, lot.y0, 0, Some(2), s + 135),
            ],
        );
    }

    // Waiting lounge area in the east/central half of the intake lobby.
    let lounge_x = lot.x0 + (lobby_w / 2).max(3);
    let lounge_y = (lobby_top_y + 1).max(lot.y0 + 4);
    let mut lounge = vec![
        make_prop(PropKind::Rug, (lounge_x - 1).max(lot.x0 + 1), lounge_y, 0, None, s + 140),
        make_prop(PropKind::Sofa, split_x - 1, lounge_y, 90, None, s + 141),
        make_prop(PropKind::CoffeeTable, lounge_x, lounge_y + 1, 90, None, s + 142),
        make_prop(PropKind::Loveseat, lounge_x, (lot.y1 - 3).max(lounge_y + 3), 180, None, s + 143),
        make_prop(PropKind::ReclinerChair, lot.x0, lot.y0 + 2, 90, None, s + 144),
        make_prop(PropKind::ReclinerChair, lounge_x - 1, lounge_y + 2, 0, None, s + 145),
        make_prop(PropKind::StandingLamp, split_x - 1, lot.y1 - 3, 0, None, s + 146),
        make_prop(PropKind::LinenCloset, split_x - 1, lot.y0, 0, None, s + 147),
    ];
    if lobby_w >= 11 && lobby_h >= 8 {
        lounge.extend([
            make_prop(PropKind::TvStand, split_x - 3, lot.y1 - 1, 180, None, s + 148),
            make_prop(PropKind::Sofa, lounge_x - 2, lounge_y + 3, 0, None, s + 149),
            make_prop(PropKind::CoffeeTable, lounge_x - 1, lounge_y + 5, 0, None, s + 150),
            make_prop(PropKind::Armchair, lounge_x - 3, lounge_y + 3, 90, None, s + 151),
            make_prop(PropKind::Armchair, lounge_x - 3, lounge_y + 5, 90, None, s + 152),
            make_prop(PropKind::Desk, lot.x0 + 4, lot.y1 - 1, 180, None, s + 153),
            make_prop(PropKind::DeskClutter, lot.x0 + 4, lot.y1 - 1, 0, None, s + 154),
            make_prop(PropKind::RunnerRug, split_x - 3, lot.y0 + 1, 0, Some(2), s + 155),
            make_prop(PropKind::SteelShelving, lot.x0 + 7, lot.y0, 0, None, s + 156),
        ]);
    }
    push_props_checked(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, lounge);
}

fn furnish_clinic_east_wards(
    b: &mut OutdoorBuilder,
    lot: LotBox,
    split_x: i32,
    split_y: i32,
    diag_x: Option<i32>,
    s: u32,
) {
    let (w, h) = (b.w, b.h);
    // NE Trauma & Urgent Care Ward (split_x..lot.x1, lot.y0..split_y).
    let mut ne_props = Vec::new();
    let bay_start_x = (split_x + 4).min(lot.x1 - 3);
    let bay_end_x = (lot.x1 - 2).max(bay_start_x);
    let mut bx = bay_start_x;
    let mut idx = 0_u32;
    while bx <= bay_end_x {
        let bed_kind = match idx % 3 {
            0 => PropKind::HospitalBed,
            1 => PropKind::Gurney,
            _ => PropKind::SurgicalTable,
        };
        let rot = if bed_kind == PropKind::Gurney { 0 } else { 90 };
        ne_props.push(make_prop(bed_kind, bx, lot.y0, rot, None, s + 170 + idx * 4));
        if bx + 1 < lot.x1 - 1 {
            ne_props.push(make_prop(
                PropKind::TelemetryMonitor,
                bx + 1,
                lot.y0,
                0,
                None,
                s + 171 + idx * 4,
            ));
        }
        if bx + 2 < lot.x1 - 2 {
            ne_props.push(make_prop(
                PropKind::PrivacyCurtain,
                bx + 2,
                lot.y0 + 1,
                90,
                Some(2),
                s + 172 + idx * 4,
            ));
        }
        bx += 4;
        idx += 1;
    }

    ne_props.extend([
        make_prop(PropKind::HospitalBed, lot.x1 - 2, split_y - 3, 90, None, s + 190),
        make_prop(PropKind::TelemetryMonitor, lot.x1 - 1, split_y - 4, 0, None, s + 191),
        make_prop(PropKind::MedicalCounter, split_x + 1, split_y - 1, 180, Some(2), s + 192),
        make_prop(PropKind::LinenCart, lot.x1 - 3, split_y - 1, 180, None, s + 193),
        make_prop(PropKind::BiohazardBin, split_x + 3, split_y - 1, 0, None, s + 194),
        make_prop(PropKind::TrashCan, lot.x1 - 1, split_y - 1, 0, None, s + 195),
        make_prop(PropKind::FireExtinguisher, split_x, split_y - 1, 0, None, s + 196),
        make_prop(PropKind::SteelShelving, lot.x1 - 2, lot.y0, 0, None, s + 197),
    ]);
    if lot.x1 - split_x >= 10 && split_y - lot.y0 >= 8 {
        let mid_x = split_x + (lot.x1 - split_x) / 2;
        let mid_y = lot.y0 + (split_y - lot.y0) / 2;
        ne_props.extend([
            make_prop(PropKind::SurgicalTable, mid_x - 1, mid_y, 0, None, s + 200),
            make_prop(PropKind::TelemetryMonitor, mid_x + 1, mid_y, 0, None, s + 201),
            make_prop(PropKind::LinenCart, mid_x - 1, mid_y + 2, 0, None, s + 202),
            make_prop(PropKind::Desk, split_x, lot.y0 + 3, 270, None, s + 203),
            make_prop(PropKind::DeskClutter, split_x, lot.y0 + 3, 0, None, s + 204),
            make_prop(PropKind::MedicalCounter, lot.x1 - 4, lot.y0 + 4, 90, Some(2), s + 205),
        ]);
    }
    push_props_checked(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, ne_props);

    // SE Recovery Ward & Diagnostics Suite (split_x..lot.x1, split_y..lot.y1).
    let mut se_props = Vec::new();
    if let Some(dx) = diag_x {
        // Recovery / Cot Ward (split_x..dx)
        se_props.extend([
            make_prop(PropKind::Cot, split_x + 1, split_y, 0, None, s + 210),
            make_prop(PropKind::PrivacyCurtain, split_x + 2, split_y + 1, 90, Some(2), s + 211),
            make_prop(PropKind::Cot, split_x + 4, split_y, 0, None, s + 212),
            make_prop(PropKind::TelemetryMonitor, split_x + 5, split_y, 0, None, s + 213),
            make_prop(PropKind::HospitalBed, split_x + 1, lot.y1 - 2, 270, None, s + 214),
            make_prop(PropKind::TelemetryMonitor, split_x + 2, lot.y1 - 1, 0, None, s + 215),
            make_prop(PropKind::ReclinerChair, split_x + 3, lot.y1 - 1, 180, None, s + 216),
            make_prop(PropKind::ReclinerChair, dx - 2, split_y, 0, None, s + 217),
            make_prop(PropKind::MedicalCounter, dx - 3, lot.y1 - 1, 180, Some(2), s + 218),
            make_prop(PropKind::LinenCart, split_x + 1, split_y + 3, 0, None, s + 219),
            make_prop(PropKind::BiohazardBin, dx - 1, lot.y1 - 1, 0, None, s + 220),
            make_prop(PropKind::WaterCooler, split_x, lot.y1 - 1, 0, None, s + 221),
        ]);
        // Diagnostics & Lab Suite (dx..lot.x1)
        se_props.extend([
            make_prop(PropKind::ImagingScanner, lot.x1 - 3, split_y + 1, 0, None, s + 230),
            make_prop(PropKind::FumeHood, dx + 1, split_y, 0, None, s + 231),
            make_prop(PropKind::LabBench, dx + 1, lot.y1 - 1, 180, Some(2), s + 232),
            make_prop(PropKind::DeskClutter, dx + 1, lot.y1 - 1, 0, None, s + 233),
            make_prop(PropKind::AirGappedTerminal, dx + 4, lot.y1 - 1, 180, None, s + 234),
            make_prop(PropKind::SteelShelving, lot.x1 - 1, split_y + 4, 90, None, s + 235),
            make_prop(PropKind::MedicalCounter, lot.x1 - 3, lot.y1 - 1, 180, Some(2), s + 236),
            make_prop(PropKind::BiohazardBin, lot.x1 - 1, lot.y1 - 1, 0, None, s + 237),
            make_prop(PropKind::TrashCan, dx, split_y, 0, None, s + 238),
        ]);
    } else {
        se_props.extend([
            make_prop(PropKind::Cot, split_x + 1, split_y + 1, 0, None, s + 240),
            make_prop(PropKind::PrivacyCurtain, split_x + 2, split_y + 1, 90, Some(2), s + 241),
            make_prop(PropKind::HospitalBed, lot.x1 - 2, split_y + 1, 90, None, s + 242),
            make_prop(PropKind::TelemetryMonitor, lot.x1 - 1, split_y, 0, None, s + 243),
            make_prop(PropKind::MedicalCounter, lot.x1 - 3, lot.y1 - 1, 180, Some(2), s + 244),
            make_prop(PropKind::LabBench, split_x + 1, lot.y1 - 1, 180, None, s + 245),
            make_prop(PropKind::DeskClutter, split_x + 1, lot.y1 - 1, 0, None, s + 246),
            make_prop(PropKind::LinenCart, split_x + 3, lot.y1 - 1, 180, None, s + 247),
            make_prop(PropKind::ReclinerChair, lot.x1 - 4, lot.y1 - 1, 180, None, s + 248),
            make_prop(PropKind::BiohazardBin, lot.x1 - 1, lot.y1 - 1, 0, None, s + 249),
            make_prop(PropKind::SteelShelving, split_x + 4, split_y, 0, None, s + 250),
        ]);
    }
    push_props_checked(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, se_props);
}

fn furnish_clinic_exterior(
    b: &mut OutdoorBuilder,
    w: i32,
    h: i32,
    lot: LotBox,
    front_door_x: i32,
    has_street: bool,
) {
    if w >= 12 && lot.x0 >= 3 {
        let alley_x = 110.min(lot.x0 * 100 - 80).max(70);
        let _ = b.try_place_cm(PropKind::Dumpster, alley_x, lot.y0 * 100 + 300, 90, None, true);
        let _ = b.try_place_cm(PropKind::DrumCluster, alley_x, lot.y0 * 100 + 120, 0, None, true);
        let _ = b.try_place_cm(PropKind::GarbagePile, alley_x, lot.y1 * 100 - 280, 90, None, true);
        let _ = b.try_place_cm(PropKind::WoodCrate, alley_x, lot.y1 * 100 - 120, 15, None, true);
        let _ = b.try_place_cm(PropKind::StreetDebris, 160, lot.y1 * 100 - 110, 15, None, true);
        let _ = b.try_place_cm(PropKind::SecurityCameraMast, 70, lot.y1 * 100 - 70, 0, None, true);
    }
    if w >= 12 && lot.x1 + 3 <= w {
        let pad_x = (lot.x1 * 100 + 200).min(w * 100 - 100);
        let _ = b.try_place_cm(PropKind::AcUnit, lot.x1 * 100 + 100, lot.y0 * 100 + 520, 0, None, true);
        let _ = b.try_place_cm(PropKind::AcUnit, lot.x1 * 100 + 100, lot.y0 * 100 + 660, 0, None, true);
        if !b.try_place_cm(PropKind::DieselGenerator, pad_x, lot.y0 * 100 + 260, 90, None, true) {
            let _ = b.try_place_cm(PropKind::DieselGenerator, pad_x, lot.y1 * 100 - 220, 90, None, true);
        }
        let _ = b.try_place_cm(PropKind::BatteryBankWall, lot.x1 * 100 + 100, lot.y1 * 100 - 200, 90, None, true);
        let _ = b.try_place_cm(PropKind::DrumCluster, pad_x, lot.y1 * 100 - 90, 0, None, true);
        let _ = b.try_place_cm(
            PropKind::SecurityCameraMast,
            (w * 100 - 70).max(pad_x),
            lot.y1 * 100 - 70,
            0,
            None,
            true,
        );
    }
    if has_street {
        let walk_y = lot.y1 * 100 + 50;
        let _ = b.try_place_cm(
            PropKind::JerseyBarrier,
            (front_door_x - 2).max(1) * 100 + 50,
            walk_y,
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::WaterPurifierKiosk,
            (front_door_x + 2).min(w - 2) * 100 + 50,
            walk_y,
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::ParkBench,
            (front_door_x + 5).min(lot.x1 - 1) * 100,
            walk_y,
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::Planter,
            (lot.x0 * 100 + 80).min((front_door_x - 4) * 100),
            walk_y,
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::TrashCan,
            (front_door_x + 7).min(lot.x1) * 100 + 50,
            walk_y,
            0,
            None,
            true,
        );
        let _ = b.try_place_cm(
            PropKind::SecurityCameraMast,
            lot.x0 * 100 + 50,
            walk_y,
            0,
            None,
            true,
        );
        if lot.y1 + 2 <= h {
            let road_y = (lot.y1 + 2).min(h - 1) * 100;
            let _ = b.try_place_cm(
                PropKind::UtilityVan,
                (lot.x1 - 3).max(4) * 100 + 50,
                road_y,
                90,
                None,
                true,
            );
            if w >= 26 {
                let _ = b.try_place_cm(
                    PropKind::Car,
                    (lot.x0 + 2) * 100 + 50,
                    road_y,
                    270,
                    None,
                    true,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn build_clinic_b1_vault_floor(
    w: i32,
    h: i32,
    lot: LotBox,
    split_x: i32,
    portal_x: i32,
    portal_y: i32,
    spawn_pads: &[(f32, f32)],
    s: u32,
) -> (MapFloorLevel, MapFaradayZone) {
    let mut b1_edges = Vec::new();
    {
        let mut b1_fc = FloorCanvas::new(&mut b1_edges, w, h);
        b1_fc.enclose(lot, MaterialClass::ReinforcedConcrete, 0);
        b1_fc.v_partition(split_x, lot.y0, lot.y1, lot.y0 + lot.h() / 2, false, MaterialClass::ArmorSteel);
    }
    let mut b1_props = vec![
        make_prop(PropKind::Stairwell, portal_x.max(lot.x0), portal_y, 0, None, s + 40),
        make_prop(PropKind::ServiceElevator, split_x.min(lot.x1 - 3), lot.y0, 0, None, s + 41),
    ];
    let mut cands = vec![
        // West cleanroom prep, power backup & cold-storage shelving
        make_prop(PropKind::DieselGenerator, lot.x0, lot.y1 - 1, 0, None, s + 42),
        make_prop(PropKind::BatteryBankWall, lot.x0, lot.y0 + 1, 270, Some(2), s + 43),
        make_prop(PropKind::AirCompressor, lot.x0 + 3, lot.y1 - 1, 0, None, s + 44),
        make_prop(PropKind::FumeHood, lot.x0 + 2, lot.y0, 0, None, s + 45),
        make_prop(PropKind::Fridge, lot.x0 + 5, lot.y0, 0, None, s + 46),
        make_prop(PropKind::SteelShelving, lot.x0 + 2, lot.y0 + 3, 0, None, s + 47),
        make_prop(PropKind::LabBench, lot.x0 + 2, lot.y1 - 3, 0, Some(2), s + 48),
        make_prop(PropKind::DeskClutter, lot.x0 + 2, lot.y1 - 3, 0, None, s + 49),
        make_prop(PropKind::HazardSpill, split_x - 2, lot.y1 - 2, 0, None, s + 50),
        make_prop(PropKind::DrumCluster, lot.x0 + 1, lot.y0 + 6, 0, None, s + 51),
        make_prop(PropKind::WoodCrate, lot.x0 + 5, lot.y1 - 1, 0, None, s + 52),
        make_prop(PropKind::FireExtinguisher, split_x - 1, lot.y0 + 4, 0, None, s + 53),
        make_prop(PropKind::TrashCan, split_x - 1, lot.y1 - 1, 0, None, s + 54),
        // East Faraday cryo-vault compute clusters, bio-vats & air-gapped terminals
        make_prop(PropKind::CryoComputeCluster, lot.x1 - 2, lot.y0 + 1, 0, None, s + 60),
        make_prop(PropKind::BioServerVat, lot.x1 - 1, lot.y1 - 2, 0, None, s + 61),
        make_prop(PropKind::AirGappedTerminal, split_x + 1, lot.y1 - 1, 180, None, s + 62),
        make_prop(PropKind::BatteryBankWall, split_x + 1, lot.y0 + 3, 270, Some(2), s + 63),
        make_prop(PropKind::ServerRack, lot.x1 - 1, lot.y0 + 4, 90, Some(2), s + 64),
        make_prop(PropKind::LabBench, split_x + 4, lot.y1 - 1, 180, None, s + 65),
        make_prop(PropKind::DeskClutter, split_x + 4, lot.y1 - 1, 0, None, s + 66),
        make_prop(PropKind::FireExtinguisher, split_x, lot.y1 - 1, 0, None, s + 67),
    ];
    if lot.w() >= 20 && lot.h() >= 16 {
        cands.extend([
            make_prop(PropKind::SteelShelving, lot.x0 + 6, lot.y0 + 3, 0, None, s + 70),
            make_prop(PropKind::SteelShelving, lot.x0 + 2, lot.y0 + 9, 0, None, s + 71),
            make_prop(PropKind::LabBench, lot.x0 + 7, lot.y0 + 9, 0, Some(2), s + 72),
            make_prop(PropKind::DeskClutter, lot.x0 + 7, lot.y0 + 9, 0, None, s + 73),
            make_prop(PropKind::CryoComputeCluster, split_x + 5, lot.y0 + 1, 0, None, s + 74),
            make_prop(PropKind::CryoComputeCluster, split_x + 5, lot.y0 + 5, 0, None, s + 75),
            make_prop(PropKind::CryoComputeCluster, lot.x1 - 4, lot.y0 + 5, 0, None, s + 76),
            make_prop(PropKind::BioServerVat, split_x + 3, lot.y0 + 8, 0, None, s + 77),
            make_prop(PropKind::BioServerVat, split_x + 6, lot.y0 + 8, 0, None, s + 78),
            make_prop(PropKind::BioServerVat, lot.x1 - 4, lot.y0 + 8, 0, None, s + 79),
            make_prop(PropKind::BioServerVat, lot.x1 - 2, lot.y0 + 11, 0, None, s + 80),
            make_prop(PropKind::AirGappedTerminal, lot.x1 - 3, lot.y1 - 1, 180, None, s + 81),
            make_prop(PropKind::ServerRack, lot.x1 - 1, lot.y0 + 8, 90, Some(2), s + 82),
            make_prop(PropKind::SteelShelving, split_x + 2, lot.y0 + 12, 0, None, s + 83),
            make_prop(PropKind::Desk, lot.x0 + 6, lot.y1 - 1, 180, None, s + 84),
            make_prop(PropKind::DeskClutter, lot.x0 + 6, lot.y1 - 1, 0, None, s + 85),
        ]);
    }
    // Keep vertical portals and validate all additional props cleanly.
    let portal_count = b1_props.len();
    for cand in cands {
        let _ = try_push_prop(&b1_edges, &mut b1_props, spawn_pads, w, h, cand);
    }
    if b1_props.len() > portal_count {
        trim_floor_props_to_connected(w, h, lot, &b1_edges, &mut b1_props);
    }
    let mut b1_surfaces = vec![
        LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
            .polygon_feature(SurfaceFeatureKind::AntiStaticCleanroomFloor, 0, s + 50),
        LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
            .polygon_feature(SurfaceFeatureKind::FaradayCopperFloor, 0, s + 51),
    ];
    prune_occluded_surfaces(&mut b1_surfaces, w, h);
    let floor = MapFloorLevel {
        floor_index: -1,
        label: "B1 · Cold Storage Vault".to_string(),
        elevation_cm: -320,
        wall_edges: b1_edges,
        props: b1_props,
        surfaces: b1_surfaces,
        ground_floor_override: Some("faraday_copper_mesh".to_string()),
        ground_tint_rgb: None,
    };
    let faraday = MapFaradayZone {
        zone_id: "faraday-clinic-b1-vault".to_string(),
        label: "B1 Cryogenic Seed Vault (Reinforced Faraday Bay)".to_string(),
        floor_index: -1,
        x0_cm: split_x * 100,
        y0_cm: lot.y0 * 100,
        x1_cm: lot.x1 * 100,
        y1_cm: lot.y1 * 100,
        attenuation_db: 90,
        forces_severed_faraday_reflex: true,
    };
    (floor, faraday)
}

fn generate_community_clinic_annex(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_street = h >= 14;
    let lot = safe_interior_lot(w, h, has_street);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if w >= 12 && lot.x0 >= 3 {
        let alley = LotBox::new(0, lot.y0, lot.x0, lot.y1);
        b.surfaces.push(alley.polygon_feature(SurfaceFeatureKind::BrickPaverWalk, 0, s + 1));
        if lot.h() >= 5 {
            b.surfaces.push(
                LotBox::new(0, lot.y1 - 2, 2.min(lot.x0), lot.y1)
                    .polygon_feature(SurfaceFeatureKind::FloodedAsphalt, 0, s + 12),
            );
        }
    }
    if w >= 12 && lot.x1 + 3 <= w {
        let util = LotBox::new(lot.x1, lot.y0, w, lot.y1);
        b.surfaces.push(util.polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 2));
    }
    if has_street {
        let walk = LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h));
        b.surfaces.push(walk.polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 3));
        if lot.y1 + 2 <= h {
            let road = LotBox::new(0, lot.y1 + 1, w, h);
            b.surfaces.push(road.polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 4));
        }
    }

    let split_x = lot.x0 + lot.w() / 2;
    let split_y = lot.y0 + lot.h() / 2;
    let has_two_wings = lot.w() >= 10 && lot.h() >= 7;
    let portal_x = if has_two_wings { split_x - 2 } else { lot.x0 };
    let portal_y = lot.y0;
    let front_door_x = lot.x0 + (lot.w() / 4).max(1);

    let pharm_box = (has_two_wings && split_x - lot.x0 >= 9 && lot.h() >= 14).then(|| {
        let pw = ((split_x - lot.x0) / 2).clamp(4, 6).min(portal_x - 1 - lot.x0);
        let ph = (lot.h() / 3).clamp(4, 6);
        LotBox::new(lot.x0, lot.y0, lot.x0 + pw, lot.y0 + ph)
    });
    let bath_box = pharm_box.and_then(|p| {
        (p.y1 + 5 <= lot.y1 && p.w() >= 4).then(|| LotBox::new(lot.x0, p.y1, lot.x0 + 3, p.y1 + 3))
    });
    let diag_x = (has_two_wings && lot.x1 - split_x >= 10 && lot.y1 - split_y >= 8)
        .then(|| split_x + (lot.x1 - split_x) / 2);

    if has_two_wings {
        if let Some(pharm) = pharm_box {
            b.surfaces.push(pharm.polygon_feature(SurfaceFeatureKind::ClinicalVinylFloor, 0, s + 5));
            b.surfaces.push(
                LotBox::new(pharm.x1, lot.y0, split_x, pharm.y1)
                    .polygon_feature(SurfaceFeatureKind::TerrazzoFloor, 0, s + 7),
            );
            let lobby_top_y = if let Some(bath) = bath_box {
                b.surfaces.push(bath.polygon_feature(SurfaceFeatureKind::CeramicTileFloor, 0, s + 8));
                b.surfaces.push(
                    LotBox::new(bath.x1, pharm.y1, split_x, bath.y1)
                        .polygon_feature(SurfaceFeatureKind::TerrazzoFloor, 0, s + 9),
                );
                bath.y1
            } else {
                pharm.y1
            };
            b.surfaces.push(
                LotBox::new(lot.x0, lobby_top_y, split_x, lot.y1)
                    .polygon_feature(SurfaceFeatureKind::TerrazzoFloor, 0, s + 13),
            );
        } else {
            b.surfaces.push(
                LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                    .polygon_feature(SurfaceFeatureKind::TerrazzoFloor, 0, s + 5),
            );
        }
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ClinicalVinylFloor, 0, s + 6),
        );
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::ClinicalVinylFloor, 0, s + 6));
    }

    {
        let mut fc = FloorCanvas::new(&mut b.edges, w, h);
        fc.enclose(lot, MaterialClass::Brick, 3);
        fc.upsert(front_door_x, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::Glass);
        fc.upsert(lot.x0, lot.y0 + 1, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SheetSteel);
        fc.upsert(lot.x1, lot.y0 + 1, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SheetSteel);
        if has_two_wings {
            if lot.h() >= 10 {
                let ne_door_y = lot.y0 + (split_y - lot.y0) / 2;
                let se_door_y = split_y + (lot.y1 - split_y) / 2;
                for y in lot.y0..lot.y1 {
                    let is_door = y == ne_door_y || y == se_door_y;
                    let (kind, mat) = if is_door {
                        (WallEdgeKind::DoorOpen, MaterialClass::SheetSteel)
                    } else {
                        (WallEdgeKind::Wall, MaterialClass::Drywall)
                    };
                    fc.upsert(split_x, y, EdgeSide::West, kind, mat);
                }
            } else {
                fc.v_partition(split_x, lot.y0, lot.y1, lot.y0 + lot.h() / 2, true, MaterialClass::Drywall);
            }
            let h_door_x = diag_x.map_or(split_x + (lot.x1 - split_x) / 2, |dx| split_x + (dx - split_x) / 2);
            fc.h_partition(split_y, split_x, lot.x1, h_door_x, true, MaterialClass::Drywall);
            if let Some(pharm) = pharm_box {
                for y in pharm.y0..pharm.y1 {
                    fc.upsert(pharm.x1, y, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::Drywall);
                }
                let p_door_x = (pharm.x0 + pharm.w() / 2).max(lot.x0 + 3).min(pharm.x1 - 1);
                fc.h_partition(pharm.y1, pharm.x0, pharm.x1, p_door_x, true, MaterialClass::Drywall);
            }
            if let Some(bath) = bath_box {
                for y in bath.y0..bath.y1 {
                    fc.upsert(lot.x0, y, EdgeSide::West, WallEdgeKind::Wall, MaterialClass::Brick);
                }
                for x in bath.x0..bath.x1 {
                    fc.upsert(x, bath.y1, EdgeSide::North, WallEdgeKind::Wall, MaterialClass::Drywall);
                }
                fc.v_partition(bath.x1, bath.y0, bath.y1, bath.y0 + 1, false, MaterialClass::Drywall);
            }
            if let Some(dx) = diag_x {
                fc.v_partition(dx, split_y, lot.y1, split_y + (lot.y1 - split_y) / 2, true, MaterialClass::Drywall);
            }
        }
    }

    let has_multi_floor = w >= 20 && h >= 14 && has_two_wings;
    let _ = try_push_prop(
        &b.edges,
        &mut b.props,
        &b.spawn_pads_m,
        w,
        h,
        make_prop(PropKind::Stairwell, portal_x.max(lot.x0), portal_y, 0, None, s + 10),
    );
    let _ = try_push_prop(
        &b.edges,
        &mut b.props,
        &b.spawn_pads_m,
        w,
        h,
        make_prop(PropKind::ServiceElevator, split_x.min(lot.x1 - 3), lot.y0, 0, None, s + 11),
    );

    if has_two_wings {
        if let Some(pharm) = pharm_box {
            furnish_clinic_pharmacy_and_restroom(&mut b, w, h, pharm, bath_box, s);
        }
        let lobby_top_y = bath_box.or(pharm_box).map_or(lot.y0, |r| r.y1);
        furnish_clinic_intake_lobby(
            &mut b,
            w,
            h,
            lot,
            split_x,
            front_door_x,
            lobby_top_y,
            pharm_box.is_some(),
            s,
        );
        furnish_clinic_east_wards(&mut b, lot, split_x, split_y, diag_x, s);
    } else {
        let compact = [
            make_prop(PropKind::MedicalCounter, lot.x0 + 1, lot.y0 + 1, 0, Some(2), s + 20),
            make_prop(PropKind::HospitalBed, lot.x1 - 2, lot.y0 + 1, 90, None, s + 21),
            make_prop(PropKind::Cot, lot.x1 - 2, lot.y1 - 2, 0, None, s + 22),
            make_prop(PropKind::TelemetryMonitor, lot.x1 - 1, lot.y0 + 1, 0, None, s + 23),
            make_prop(PropKind::Desk, lot.x0 + 1, lot.y1 - 2, 180, None, s + 24),
            make_prop(PropKind::DeskClutter, lot.x0 + 1, lot.y1 - 2, 0, None, s + 25),
            make_prop(PropKind::ReclinerChair, lot.x0 + 3, lot.y1 - 1, 180, None, s + 26),
            make_prop(PropKind::BiohazardBin, lot.x0 + 1, lot.y1 - 1, 0, None, s + 27),
            make_prop(PropKind::LinenCart, lot.x0 + 3, lot.y0 + 1, 0, None, s + 28),
            make_prop(PropKind::SteelShelving, lot.x0, lot.y0 + 2, 270, None, s + 29),
            make_prop(PropKind::FireExtinguisher, lot.x1 - 1, lot.y1 - 1, 0, None, s + 30),
        ];
        push_props_checked(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, compact);
    }

    furnish_clinic_exterior(&mut b, w, h, lot, front_door_x, has_street);

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let mut floors = Vec::new();
    let faraday_zones = if has_multi_floor {
        let (b1_floor, faraday) = build_clinic_b1_vault_floor(
            w,
            h,
            lot,
            split_x,
            portal_x,
            portal_y,
            &b.spawn_pads_m,
            s,
        );
        floors.push(b1_floor);
        floors.push(MapFloorLevel {
            floor_index: 0,
            label: "1F · Mutual-Aid Clinic".to_string(),
            elevation_cm: 0,
            wall_edges: b.edges.clone(),
            props: b.props.clone(),
            surfaces: b.surfaces.clone(),
            ground_floor_override: None,
            ground_tint_rgb: None,
        });
        vec![faraday]
    } else {
        Vec::new()
    };

    let surveillance_nodes = vec![
        MapSurveillanceNode {
            node_id: "cam-clinic-vestibule".to_string(),
            label: "Clinic Entry Biometric Scanner".to_string(),
            node_kind_code: "BIOMETRIC_GAIT_CHECKPOINT".to_string(),
            floor_index: 0,
            x_cm: front_door_x * 100 + 50,
            y_cm: (lot.y1 - 1).max(lot.y0) * 100 + 50,
            facing_deg: 180,
            fov_deg: 110,
            range_cm: 900,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "cam-clinic-street-alpr".to_string(),
            label: "Municipal ALPR Street Camera".to_string(),
            node_kind_code: "ALPR_TRAFFIC_CAMERA".to_string(),
            floor_index: 0,
            x_cm: (lot.x0 * 100 + 50).min(w * 100 - 150),
            y_cm: (lot.y1 * 100 + 50).min(h * 100 - 100),
            facing_deg: 135,
            fov_deg: 120,
            range_cm: 1600,
            powered: true,
            visibility: MessageVisibility::Public,
        },
    ];

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors,
        active_floor: 0,
        faraday_zones,
        surveillance_nodes,
    }
}

fn generate_corporate_server_bunker(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_apron = h >= 14;
    let lot = safe_interior_lot(w, h, has_apron);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if w >= 12 && lot.x0 >= 3 {
        b.surfaces.push(
            LotBox::new(0, lot.y0, lot.x0, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 1),
        );
    }
    if w >= 12 && lot.x1 + 3 <= w {
        b.surfaces.push(
            LotBox::new(lot.x1, lot.y0, w, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 2),
        );
    }
    if has_apron {
        b.surfaces.push(
            LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h))
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 3),
        );
        if lot.y1 + 2 <= h {
            b.surfaces.push(
                LotBox::new(0, lot.y1 + 1, w, h)
                    .polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 4),
            );
        }
    }

    let split_x = lot.x0 + (lot.w() * 2 / 5).max(2);
    let breaker_y = lot.y0 + (lot.h() / 2).max(2);
    let has_wings = lot.w() >= 10 && lot.h() >= 7;

    if has_wings {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::AntiStaticCleanroomFloor, 0, s + 5),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::FaradayCopperFloor, 0, s + 6),
        );
        if lot.x1 - split_x >= 4 && lot.h() >= 5 {
            b.surfaces.push(
                LotBox::new(lot.x1 - 1, lot.y0 + 1, lot.x1, lot.y1 - 1)
                    .polygon_feature(SurfaceFeatureKind::SubstationGrating, 0, s + 7),
            );
        }
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::FaradayCopperFloor, 0, s + 6));
    }

    {
        let mut fc = FloorCanvas::new(&mut b.edges, w, h);
        fc.enclose(lot, MaterialClass::ReinforcedConcrete, 0);
        fc.upsert(lot.x0, lot.y0 + lot.h() / 2, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::ArmorSteel);
        fc.upsert(lot.x1, lot.y0 + lot.h() / 2, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::ArmorSteel);
        fc.upsert(lot.x0 + lot.w() / 2, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        if has_wings {
            for y in lot.y0..lot.y1 {
                if y == breaker_y {
                    fc.upsert(split_x, y, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::ArmorSteel);
                } else {
                    fc.upsert(split_x, y, EdgeSide::West, WallEdgeKind::Window, MaterialClass::Glass);
                }
            }
            fc.h_partition(breaker_y, split_x + 2, lot.x1, (split_x + 2).min(lot.x1 - 1), true, MaterialClass::ArmorSteel);
        }
    }

    let cands = if has_wings {
        vec![
            make_prop(PropKind::DeskL, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::BiometricTurnstile, lot.x0, lot.y0 + lot.h() / 2 + 1, 0, None, s + 11),
            make_prop(PropKind::FilingCabinet, split_x - 1, lot.y1 - 1, 0, None, s + 12),
            make_prop(PropKind::FireExtinguisher, split_x - 1, lot.y0, 0, None, s + 13),
            make_prop(PropKind::CryoComputeCluster, split_x + 1, lot.y0 + 1, 0, None, s + 14),
            make_prop(PropKind::CryoComputeCluster, lot.x1 - 2, lot.y0 + 1, 0, None, s + 15),
            make_prop(PropKind::BioServerVat, split_x + 2, lot.y0 + 3, 0, None, s + 16),
            make_prop(PropKind::BatteryBankWall, lot.x1 - 2, breaker_y + 1, 0, None, s + 17),
            make_prop(PropKind::AirGappedTerminal, split_x + 1, lot.y1 - 1, 0, None, s + 18),
            make_prop(PropKind::ServerRack, lot.x1 - 1, lot.y1 - 1, 0, None, s + 19),
            make_prop(PropKind::SteelShelving, lot.x0 + 1, lot.y1 - 1, 0, None, s + 20),
        ]
    } else {
        vec![
            make_prop(PropKind::AirGappedTerminal, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::CryoComputeCluster, lot.x1 - 2, lot.y0 + 1, 0, None, s + 11),
            make_prop(PropKind::BioServerVat, lot.x0 + 1, lot.y1 - 2, 0, None, s + 12),
        ]
    };
    for cand in cands {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    if w >= 16 && lot.x1 + 3 <= w {
        let _ = b.try_place_cm(PropKind::DroneDockPad, lot.x1 * 100 + 150, lot.y0 * 100 + 200, 0, None, true);
    }
    if has_apron {
        let _ = b.try_place_cm(
            PropKind::JerseyBarrier,
            (lot.x0 * 100 + 150).min(w * 100 - 150),
            lot.y1 * 100 + 50,
            0,
            None,
            true,
        );
        if lot.y1 + 2 <= h {
            let _ = b.try_place_cm(
                PropKind::Car,
                (lot.x1 - 2).max(4) * 100,
                (lot.y1 + 2).min(h - 1) * 100,
                90,
                None,
                true,
            );
        }
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let fz = MapFaradayZone {
        zone_id: "faraday-qpu-hall".to_string(),
        label: "Cryogenic QPU Faraday Vault".to_string(),
        floor_index: 0,
        x0_cm: (if has_wings { split_x } else { lot.x0 }) * 100,
        y0_cm: lot.y0 * 100,
        x1_cm: lot.x1 * 100,
        y1_cm: lot.y1 * 100,
        attenuation_db: 95,
        forces_severed_faraday_reflex: true,
    };

    let surveillance_nodes = vec![
        MapSurveillanceNode {
            node_id: "node-bunker-biometric".to_string(),
            label: "Mantrap Biometric Gait Scanner".to_string(),
            node_kind_code: "BIOMETRIC_GAIT_CHECKPOINT".to_string(),
            floor_index: 0,
            x_cm: (lot.x0 + 1).min(w - 2) * 100 + 50,
            y_cm: (lot.y0 + lot.h() / 2) * 100 + 50,
            facing_deg: 270,
            fov_deg: 100,
            range_cm: 800,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "node-bunker-scada".to_string(),
            label: "Air-Gapped Chiller SCADA Terminal".to_string(),
            node_kind_code: "AIR_GAPPED_SCADA".to_string(),
            floor_index: 0,
            x_cm: (lot.x1 - 1).max(lot.x0) * 100 + 50,
            y_cm: (lot.y1 - 1).max(lot.y0) * 100 + 50,
            facing_deg: 0,
            fov_deg: 360,
            range_cm: 500,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "node-bunker-turret".to_string(),
            label: "Concealed Apron Thermal Sentry".to_string(),
            node_kind_code: "THERMAL_PERIMETER_TURRET".to_string(),
            floor_index: 0,
            x_cm: (lot.x0 + lot.w() / 2) * 100 + 50,
            y_cm: lot.y1 * 100 + 30,
            facing_deg: 180,
            fov_deg: 90,
            range_cm: 1400,
            powered: true,
            visibility: MessageVisibility::GmOnly,
        },
    ];

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors: Vec::new(),
        active_floor: 0,
        faraday_zones: vec![fz],
        surveillance_nodes,
    }
}

fn generate_satyang_safehouse(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_street = h >= 14;
    let lot = safe_interior_lot(w, h, has_street);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if w >= 12 && lot.x0 >= 3 {
        b.surfaces.push(
            LotBox::new(0, lot.y0, lot.x0, lot.y1)
                .polygon_feature(SurfaceFeatureKind::BrickPaverWalk, 0, s + 1),
        );
        if lot.h() >= 5 {
            b.surfaces.push(
                LotBox::new(0, lot.y1 - 2, 2.min(lot.x0), lot.y1)
                    .polygon_feature(SurfaceFeatureKind::FloodedAsphalt, 0, s + 8),
            );
        }
    }
    if has_street {
        b.surfaces.push(
            LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h))
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 2),
        );
        if lot.y1 + 2 <= h {
            b.surfaces.push(
                LotBox::new(0, lot.y1 + 1, w, h)
                    .polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 3),
            );
        }
    }

    let split_x = lot.x0 + lot.w() / 2;
    let has_wings = lot.w() >= 10 && lot.h() >= 7;
    if has_wings {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 4),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::CarpetFloor, 0, s + 5),
        );
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::InteriorFloor, 0, s + 4));
    }

    let front_door_x = lot.x0 + 1;
    {
        let mut fc = FloorCanvas::new(&mut b.edges, w, h);
        fc.enclose(lot, MaterialClass::Brick, 3);
        fc.upsert(front_door_x, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SolidWood);
        fc.upsert(lot.x0, lot.y0 + 1, EdgeSide::West, WallEdgeKind::DoorClosed, MaterialClass::SolidWood);
        if has_wings {
            fc.v_partition(split_x, lot.y0, lot.y1, lot.y0 + lot.h() / 2, false, MaterialClass::Drywall);
        }
    }

    let portal_x = lot.x1 - 2;
    let portal_y = lot.y0;
    let has_multi_floor = w >= 20 && h >= 14 && has_wings;
    let cands = if has_wings {
        vec![
            make_prop(PropKind::Stairwell, portal_x.max(split_x + 1), portal_y, 0, None, s + 10),
            make_prop(PropKind::Rug, lot.x0 + 1, lot.y0 + 1, 0, None, s + 11),
            make_prop(PropKind::Desk, lot.x0, lot.y0 + 1, 0, None, s + 12),
            make_prop(PropKind::Bookshelf, split_x - 1, lot.y0, 0, None, s + 13),
            make_prop(PropKind::Sofa, lot.x0 + 1, lot.y1 - 2, 0, None, s + 14),
            make_prop(PropKind::KitchenCounter, split_x + 1, lot.y0, 0, None, s + 15),
            make_prop(PropKind::Fridge, split_x, lot.y0, 0, None, s + 16),
            make_prop(PropKind::Cot, lot.x1 - 2, lot.y1 - 2, 0, None, s + 17),
            make_prop(PropKind::DiningTable, split_x + 1, lot.y1 - 2, 0, None, s + 18),
        ]
    } else {
        vec![
            make_prop(PropKind::Desk, lot.x0 + 1, lot.y0 + 1, 0, None, s + 12),
            make_prop(PropKind::Sofa, lot.x1 - 3, lot.y1 - 2, 0, None, s + 14),
        ]
    };
    for cand in cands {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    if w >= 12 && lot.x0 >= 3 {
        let _ = b.try_place_cm(PropKind::Dumpster, 120, lot.y0 * 100 + 100, 90, None, true);
    }
    if has_street {
        let _ = b.try_place_cm(PropKind::AwningCanopy, front_door_x * 100 + 100, lot.y1 * 100 + 40, 0, None, true);
        if lot.y1 + 2 <= h {
            let _ = b.try_place_cm(PropKind::Car, (lot.x1 - 2).max(4) * 100, (lot.y1 + 2).min(h - 1) * 100, 90, None, true);
        }
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let mut floors = Vec::new();
    let faraday_zone = if has_multi_floor {
        let mut b1_edges = Vec::new();
        {
            let mut b1_fc = FloorCanvas::new(&mut b1_edges, w, h);
            b1_fc.enclose(lot, MaterialClass::ReinforcedConcrete, 0);
            b1_fc.v_partition(split_x, lot.y0, lot.y1, lot.y0 + lot.h() / 2, false, MaterialClass::ReinforcedConcrete);
        }
        let mut b1_props = Vec::new();
        for cand in [
            make_prop(PropKind::Stairwell, portal_x.max(split_x + 1), portal_y, 0, None, s + 30),
            make_prop(PropKind::Rug, lot.x0 + 1, lot.y0 + 1, 0, None, s + 31),
            make_prop(PropKind::AirGappedTerminal, lot.x0, lot.y0 + 1, 0, None, s + 32),
            make_prop(PropKind::SolderingStation, lot.x0, lot.y1 - 2, 0, None, s + 33),
            make_prop(PropKind::BatteryBankWall, split_x - 2, lot.y1 - 1, 0, None, s + 34),
            make_prop(PropKind::ServerRack, split_x - 1, lot.y0 + 1, 0, None, s + 35),
            make_prop(PropKind::SteelShelving, lot.x1 - 2, lot.y1 - 1, 0, None, s + 36),
            make_prop(PropKind::WoodCrate, split_x + 1, lot.y1 - 2, 0, None, s + 37),
        ] {
            let _ = try_push_prop(&b1_edges, &mut b1_props, &b.spawn_pads_m, w, h, cand);
        }
        trim_floor_props_to_connected(w, h, lot, &b1_edges, &mut b1_props);
        let mut b1_surfaces = vec![
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::FaradayCopperFloor, 0, s + 39),
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 40),
        ];
        prune_occluded_surfaces(&mut b1_surfaces, w, h);
        floors.push(MapFloorLevel {
            floor_index: -1,
            label: "B1 · Copper-Mesh Cellar".to_string(),
            elevation_cm: -300,
            wall_edges: b1_edges,
            props: b1_props,
            surfaces: b1_surfaces,
            ground_floor_override: Some("faraday_copper_mesh".to_string()),
            ground_tint_rgb: None,
        });
        floors.push(MapFloorLevel {
            floor_index: 0,
            label: "1F · Cover Shophouse".to_string(),
            elevation_cm: 0,
            wall_edges: b.edges.clone(),
            props: b.props.clone(),
            surfaces: b.surfaces.clone(),
            ground_floor_override: None,
            ground_tint_rgb: None,
        });
        MapFaradayZone {
            zone_id: "faraday-safehouse-cellar".to_string(),
            label: "B1 Copper-Mesh SIGINT Sanctuary".to_string(),
            floor_index: -1,
            x0_cm: lot.x0 * 100,
            y0_cm: lot.y0 * 100,
            x1_cm: split_x * 100,
            y1_cm: lot.y1 * 100,
            attenuation_db: 85,
            forces_severed_faraday_reflex: true,
        }
    } else {
        MapFaradayZone {
            zone_id: "faraday-safehouse-backroom".to_string(),
            label: "Copper-Mesh Shielded Backroom".to_string(),
            floor_index: 0,
            x0_cm: lot.x0 * 100,
            y0_cm: lot.y0 * 100,
            x1_cm: lot.x1 * 100,
            y1_cm: lot.y1 * 100,
            attenuation_db: 85,
            forces_severed_faraday_reflex: true,
        }
    };

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors,
        active_floor: 0,
        faraday_zones: vec![faraday_zone],
        surveillance_nodes: Vec::new(),
    }
}

fn generate_brownout_logistics_depot(w: i32, h: i32, seed: i64) -> ProceduralSector {
    let mut b = OutdoorBuilder::new(w, h, seed);
    let has_street = h >= 14;
    let lot = safe_interior_lot(w, h, has_street);
    let s = (b.rng.next() & 0xffff_ffff) as u32;

    if w >= 12 && lot.x0 >= 3 {
        b.surfaces.push(
            LotBox::new(0, lot.y0, lot.x0, lot.y1)
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 9),
        );
        if lot.h() >= 5 {
            b.surfaces.push(
                LotBox::new(0, lot.y1 - 2, 2.min(lot.x0), lot.y1)
                    .polygon_feature(SurfaceFeatureKind::FloodedAsphalt, 0, s + 8),
            );
        }
    }
    if has_street {
        b.surfaces.push(
            LotBox::new(0, lot.y1, w, (lot.y1 + 1).min(h))
                .polygon_feature(SurfaceFeatureKind::ConcreteWalk, 0, s + 1),
        );
        if lot.y1 + 2 <= h {
            b.surfaces.push(
                LotBox::new(0, lot.y1 + 1, w, h)
                    .polygon_feature(SurfaceFeatureKind::AsphaltRoad, 0, s + 2),
            );
        }
    }

    let split_x = lot.x0 + (lot.w() * 3 / 5).max(2);
    let has_substation = lot.w() >= 10 && lot.h() >= 7;
    if has_substation {
        b.surfaces.push(
            LotBox::new(lot.x0, lot.y0, split_x, lot.y1)
                .polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 3),
        );
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, lot.y1)
                .polygon_feature(SurfaceFeatureKind::GravelRoad, 0, s + 4),
        );
        let pad_y1 = lot.y0 + (lot.h() / 2).max(2);
        b.surfaces.push(
            LotBox::new(split_x, lot.y0, lot.x1, pad_y1)
                .polygon_feature(SurfaceFeatureKind::SubstationGrating, 0, s + 5),
        );
        let door_y = lot.y0 + lot.h() / 2;
        let mut fc = FloorCanvas::new(&mut b.edges, w, h);
        fc.enclose(LotBox::new(lot.x0, lot.y0, split_x, lot.y1), MaterialClass::ConcreteBlock, 4);
        fc.enclose(LotBox::new(split_x, lot.y0, lot.x1, lot.y1), MaterialClass::SheetSteel, 0);
        fc.upsert(lot.x0, door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        fc.upsert(split_x, door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        fc.upsert(lot.x1, door_y, EdgeSide::West, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
        fc.upsert(lot.x0 + 2, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
    } else {
        b.surfaces.push(lot.polygon_feature(SurfaceFeatureKind::StainedConcreteFloor, 0, s + 3));
        let mut fc = FloorCanvas::new(&mut b.edges, w, h);
        fc.enclose(lot, MaterialClass::ConcreteBlock, 4);
        fc.upsert(lot.x0 + lot.w() / 2, lot.y1, EdgeSide::North, WallEdgeKind::DoorOpen, MaterialClass::SheetSteel);
    }

    let cands = if has_substation {
        vec![
            make_prop(PropKind::PalletRack, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::ConveyorBelt, lot.x0 + 1, lot.y1 - 2, 0, Some(3), s + 11),
            make_prop(PropKind::Forklift, split_x - 2, lot.y0 + 1, 90, None, s + 12),
            make_prop(PropKind::WoodCrate, split_x - 2, lot.y1 - 2, 0, None, s + 13),
            make_prop(PropKind::Desk, lot.x0 + 1, lot.y0 + 3, 0, None, s + 14),
            make_prop(PropKind::FilingCabinet, lot.x0, lot.y1 - 1, 0, None, s + 15),
            make_prop(PropKind::HvTransformer, split_x + 1, lot.y0, 0, None, s + 16),
            make_prop(PropKind::DieselGenerator, lot.x1 - 2, lot.y0 + 1, 0, None, s + 17),
            make_prop(PropKind::BatteryBankWall, lot.x1 - 2, lot.y1 - 1, 0, None, s + 18),
            make_prop(PropKind::SecurityCameraMast, split_x, lot.y1 - 1, 0, None, s + 19),
            make_prop(PropKind::JerseyBarrier, split_x + 2, lot.y1 - 1, 0, None, s + 20),
            make_prop(PropKind::HazardSpill, split_x + 2, lot.y0 + 2, 0, None, s + 21),
        ]
    } else {
        vec![
            make_prop(PropKind::PalletRack, lot.x0 + 1, lot.y0 + 1, 0, None, s + 10),
            make_prop(PropKind::DieselGenerator, lot.x1 - 2, lot.y1 - 2, 0, None, s + 11),
            make_prop(PropKind::WoodCrate, lot.x0 + 1, lot.y1 - 2, 0, None, s + 12),
        ]
    };
    for cand in cands {
        let _ = try_push_prop(&b.edges, &mut b.props, &b.spawn_pads_m, w, h, cand);
    }

    if w >= 16 && h >= 14 {
        let _ = b.try_place_cm(PropKind::ShippingContainer, (w - 3) * 100 + 50, lot.y0 * 100 + 200, 90, None, true);
        let _ = b.try_place_cm(PropKind::DroneDockPad, (w - 3) * 100 + 50, (lot.y0 + 4).min(lot.y1 - 2) * 100, 0, None, true);
    }
    if has_street {
        let can_x = (lot.x0 + 2).min(w - 3) * 100;
        let _ = b.try_place_cm(PropKind::AwningCanopy, can_x, lot.y1 * 100 + 40, 0, None, true);
        let _ = b.try_place_cm(PropKind::ParkBench, can_x + 200, lot.y1 * 100 + 45, 0, None, true);
        let _ = b.try_place_cm(PropKind::WaterPurifierKiosk, can_x - 100, lot.y1 * 100 + 45, 0, None, true);
        if lot.y1 + 2 <= h {
            let _ = b.try_place_cm(
                PropKind::UtilityVan,
                (lot.x1 - 3).max(5) * 100,
                (lot.y1 + 2).min(h - 1) * 100,
                90,
                None,
                true,
            );
        }
    }

    trim_floor_props_to_connected(w, h, LotBox::new(0, 0, w, h), &b.edges, &mut b.props);
    prune_occluded_surfaces(&mut b.surfaces, w, h);

    let surveillance_nodes = vec![
        MapSurveillanceNode {
            node_id: "cam-depot-alpr".to_string(),
            label: "Depot Gatehouse ALPR Camera".to_string(),
            node_kind_code: "ALPR_TRAFFIC_CAMERA".to_string(),
            floor_index: 0,
            x_cm: lot.x0 * 100 + 50,
            y_cm: (lot.y1 * 100 + 50).min(h * 100 - 100),
            facing_deg: 180,
            fov_deg: 120,
            range_cm: 1800,
            powered: true,
            visibility: MessageVisibility::Public,
        },
        MapSurveillanceNode {
            node_id: "arr-depot-acoustic".to_string(),
            label: "500 kV Substation Acoustic Array".to_string(),
            node_kind_code: "ACOUSTIC_GUNSHOT_ARRAY".to_string(),
            floor_index: 0,
            x_cm: (if has_substation { split_x + 1 } else { lot.x0 + 1 }) * 100 + 50,
            y_cm: (lot.y0 + 1) * 100 + 50,
            facing_deg: 0,
            fov_deg: 360,
            range_cm: 1800,
            powered: true,
            visibility: MessageVisibility::Public,
        },
    ];

    ProceduralSector {
        wall_edges: b.edges,
        props: b.props,
        surfaces: b.surfaces,
        floors: Vec::new(),
        active_floor: 0,
        faraday_zones: Vec::new(),
        surveillance_nodes,
    }
}

/// Generates one of the 4 canonical *Awakened* 2035 procedural tactical battlemaps.
#[must_use]
pub(crate) fn generate_awakened_sector(
    preset: SectorPreset,
    w: i32,
    h: i32,
    seed: i64,
) -> ProceduralSector {
    match preset {
        SectorPreset::CommunityClinicAnnex => generate_community_clinic_annex(w, h, seed),
        SectorPreset::CorporateServerBunker => generate_corporate_server_bunker(w, h, seed),
        SectorPreset::SatyangSafehouse => generate_satyang_safehouse(w, h, seed),
        SectorPreset::BrownoutLogisticsDepot => generate_brownout_logistics_depot(w, h, seed),
        _ => ProceduralSector::default(),
    }
}
