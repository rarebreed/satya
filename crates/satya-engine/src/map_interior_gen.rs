//! # Furnished Modern-Day Interior Generator (`map_interior_gen`)
//!
//! Deterministic, seeded generator for edge-walled interiors (`Apartment`, `Bar`, `Office`):
//! 1. **Shell**: masonry perimeter edge walls with West/East entry doors and exterior windows.
//! 2. **Rooms & Props**: BSP split / partitioned rooms with rule-based continuous `[0, 360°)` props;
//!    door thresholds and spawn pads stay clear while tall props preserve full connectivity.

use satya_schema::{
    EconomicTier, EdgeSide, MapWallEdge, MaterialClass, PropKind, SurfaceFeatureKind, WallEdgeKind,
};
use serde::{Deserialize, Serialize};

use crate::map_dressing::{PropObstruction, make_prop_at_cm, prop_spec};
use crate::map_pool_spa_gen::{SpaStyle, spa_variant_seed};
pub(crate) use crate::map_room_furnish::{Furnisher, Role, emit_room_floor_surface};
use crate::map_surface_geom::vehicle_forward_rot_deg;
use crate::map_surface_queries::{prune_occluded_surfaces, surface_contains_point_m};
use crate::tactical_map::ProceduralSector;
use crate::tactical_map_edges::WallEdgeIndex;
use crate::tactical_map_geom::all_walkable_areas_connected;

/// Furnished interior layout family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteriorStyle {
    Apartment,
    Bar,
    Office,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Rng(pub(crate) u64);

impl Rng {
    pub(crate) fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub(crate) fn range(&mut self, lo: i32, hi_excl: i32) -> i32 {
        if hi_excl <= lo {
            lo
        } else {
            lo + (self.next() % (hi_excl - lo) as u64) as i32
        }
    }
    pub(crate) fn chance(&mut self, num: u64, den: u64) -> bool {
        self.next() % den < num
    }
}

/// Half-open cell rectangle `[x0, x1) × [y0, y1)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Room {
    pub(crate) x0: i32,
    pub(crate) y0: i32,
    pub(crate) x1: i32,
    pub(crate) y1: i32,
}

impl Room {
    pub(crate) const fn w(self) -> i32 {
        self.x1 - self.x0
    }
    pub(crate) const fn h(self) -> i32 {
        self.y1 - self.y0
    }
    pub(crate) const fn area(self) -> i32 {
        self.w() * self.h()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    North,
    East,
    South,
    West,
}

impl Side {
    pub(crate) const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];
    /// Rotation that puts a prop's back against this wall.
    pub(crate) const fn rot(self) -> i32 {
        match self {
            Self::North => 0,
            Self::East => 90,
            Self::South => 180,
            Self::West => 270,
        }
    }
}

pub(crate) struct Shell {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) rng: Rng,
    pub(crate) edges: Vec<MapWallEdge>,
}

impl Shell {
    pub(crate) fn put(
        &mut self,
        x: i32,
        y: i32,
        side: EdgeSide,
        kind: WallEdgeKind,
        material: MaterialClass,
    ) {
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
            Some(o) => *o = e,
            None => self.edges.push(e),
        }
    }

    pub(crate) fn door_kind(&mut self) -> WallEdgeKind {
        if self.rng.chance(1, 2) {
            WallEdgeKind::DoorOpen
        } else {
            WallEdgeKind::DoorClosed
        }
    }

    /// Styled vertical partition on West edges of column `x` for rows `y0..y1`, with doors at `door_rows`.
    pub(crate) fn partition_v_styled(
        &mut self,
        x: i32,
        y0: i32,
        y1: i32,
        door_rows: &[i32],
        wall: (WallEdgeKind, MaterialClass),
    ) {
        for y in y0..y1 {
            if door_rows.contains(&y) {
                let kind = self.door_kind();
                self.put(x, y, EdgeSide::West, kind, MaterialClass::Drywall);
            } else {
                self.put(x, y, EdgeSide::West, wall.0, wall.1);
            }
        }
    }

    /// Styled horizontal partition on North edges of row `y` for columns `x0..x1`, with doors at `door_cols`.
    pub(crate) fn partition_h_styled(
        &mut self,
        y: i32,
        x0: i32,
        x1: i32,
        door_cols: &[i32],
        wall: (WallEdgeKind, MaterialClass),
    ) {
        for x in x0..x1 {
            if door_cols.contains(&x) {
                let kind = self.door_kind();
                self.put(x, y, EdgeSide::North, kind, MaterialClass::Drywall);
            } else {
                self.put(x, y, EdgeSide::North, wall.0, wall.1);
            }
        }
    }

    /// Drywall partition on West edges of column `x` for rows `y0..y1`, with doors at `door_rows`.
    pub(crate) fn partition_v(&mut self, x: i32, y0: i32, y1: i32, door_rows: &[i32]) {
        self.partition_v_styled(
            x,
            y0,
            y1,
            door_rows,
            (WallEdgeKind::Wall, MaterialClass::Drywall),
        );
    }

    /// Drywall partition on North edges of row `y` for columns `x0..x1`, with doors at `door_cols`.
    pub(crate) fn partition_h(&mut self, y: i32, x0: i32, x1: i32, door_cols: &[i32]) {
        self.partition_h_styled(
            y,
            x0,
            x1,
            door_cols,
            (WallEdgeKind::Wall, MaterialClass::Drywall),
        );
    }

    fn perimeter_mat(&mut self, wall_mat: MaterialClass) {
        let (w, h, mid_y) = (self.w, self.h, self.h / 2);
        for x in 0..w {
            self.put(x, 0, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
            self.put(x, h, EdgeSide::North, WallEdgeKind::Wall, wall_mat);
        }
        for y in 0..h {
            self.put(0, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
            self.put(w, y, EdgeSide::West, WallEdgeKind::Wall, wall_mat);
        }
        self.put(
            0,
            mid_y,
            EdgeSide::West,
            WallEdgeKind::DoorClosed,
            MaterialClass::SolidWood,
        );
        self.put(
            w,
            mid_y,
            EdgeSide::West,
            WallEdgeKind::DoorClosed,
            MaterialClass::SolidWood,
        );
    }

    fn perimeter(&mut self) {
        self.perimeter_mat(MaterialClass::Brick);
    }

    /// Seeded exterior windows on perimeter walls, keeping clear of corners, doors, and partition junctions.
    fn windows(&mut self, spacing: i32) {
        let idx = WallEdgeIndex::from_edges(self.w, self.h, &self.edges);
        let (w, h) = (self.w, self.h);
        let off = self.rng.range(0, spacing);
        let mut picks: Vec<(i32, i32, EdgeSide)> = Vec::new();
        let partition_v =
            |x: i32, y: i32| idx.get(x, y, EdgeSide::West).is_some() && x > 0 && x < w;
        let partition_h =
            |x: i32, y: i32| idx.get(x, y, EdgeSide::North).is_some() && y > 0 && y < h;
        for x in 1..(w - 1) {
            if (x + off) % spacing == 0 {
                if !partition_v(x, 0) && !partition_v(x + 1, 0) {
                    picks.push((x, 0, EdgeSide::North));
                }
                if !partition_v(x, h - 1) && !partition_v(x + 1, h - 1) {
                    picks.push((x, h, EdgeSide::North));
                }
            }
        }
        for y in 1..(h - 1) {
            if (y + off) % spacing == 0 && (y - h / 2).abs() > 1 {
                if !partition_h(0, y) && !partition_h(0, y + 1) {
                    picks.push((0, y, EdgeSide::West));
                }
                if !partition_h(w - 1, y) && !partition_h(w - 1, y + 1) {
                    picks.push((w, y, EdgeSide::West));
                }
            }
        }
        for (x, y, side) in picks {
            self.put(x, y, side, WallEdgeKind::Window, MaterialClass::Glass);
        }
    }
}

/// Generates a furnished edge-walled interior sector (`width × height` cells, already clamped by caller).
#[must_use]
pub fn generate_furnished_interior(
    style: InteriorStyle,
    width: i32,
    height: i32,
    seed: i64,
) -> ProceduralSector {
    let (w, h) = (width, height);
    let mid_y = h / 2;
    let salt = match style {
        InteriorStyle::Apartment => 0xA9A7,
        InteriorStyle::Bar => 0xBA77,
        InteriorStyle::Office => 0x0FF1,
    };
    let mut shell = Shell {
        w,
        h,
        rng: Rng((seed as u64) ^ salt),
        edges: Vec::new(),
    };
    let whole = Room {
        x0: 0,
        y0: 0,
        x1: w,
        y1: h,
    };
    let mut rooms_roles: Vec<(Room, Role)> = Vec::new();
    let mut patio_rooms: Vec<Room> = Vec::new();
    let mut surfaces = if w >= 24 && h >= 18 {
        let (r_roles, ext_surfaces, patios) =
            crate::map_interior_topologies::build_scaled_interior_layout(style, &mut shell);
        rooms_roles = r_roles;
        patio_rooms = patios;
        ext_surfaces
    } else {
        match style {
            InteriorStyle::Office => shell.perimeter_mat(MaterialClass::ConcreteBlock),
            InteriorStyle::Apartment | InteriorStyle::Bar => shell.perimeter(),
        }
        match style {
            InteriorStyle::Apartment => {
                crate::map_interior_topologies::partition_apartment_zone(
                    &mut shell,
                    whole,
                    true,
                    false,
                    Some(mid_y),
                    &mut rooms_roles,
                );
                shell.windows(4);
            }
            InteriorStyle::Bar => {
                let strip_w = (w * 3 / 10).max(4);
                let sx = w - strip_w;
                let n_back: usize = if h >= 12 { 3 } else { 2 };
                let mut cuts = vec![0];
                for i in 1..n_back {
                    let base = h * i as i32 / n_back as i32;
                    cuts.push((base + shell.rng.range(-1, 2)).clamp(3, h - 3));
                }
                cuts.push(h);
                let mut doors = Vec::new();
                const BACK_ROLES: [Role; 4] = [
                    Role::Storage,
                    Role::Restroom,
                    Role::BackKitchen,
                    Role::Office,
                ];
                for i in 0..n_back {
                    let (y0, y1) = (cuts[i], cuts[i + 1]);
                    doors.push(shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2)));
                    if i > 0 {
                        shell.partition_h(y0, sx, w, &[]);
                    }
                    rooms_roles.push((
                        Room {
                            x0: sx,
                            y0,
                            x1: w,
                            y1,
                        },
                        BACK_ROLES[i % BACK_ROLES.len()],
                    ));
                }
                shell.partition_v(sx, 0, h, &doors);
                rooms_roles.insert(
                    0,
                    (
                        Room {
                            x0: 0,
                            y0: 0,
                            x1: sx,
                            y1: h,
                        },
                        Role::BarHall,
                    ),
                );
                shell.windows(3);
            }
            InteriorStyle::Office => {
                if w >= 12 {
                    let sx = (w * 6 / 10).clamp(6, w - 4);
                    let n_east: usize = if h >= 12 { 3 } else { 2 };
                    let mut cuts = vec![0];
                    for i in 1..n_east {
                        let base = h * i as i32 / n_east as i32;
                        cuts.push((base + shell.rng.range(-1, 2)).clamp(3, h - 3));
                    }
                    cuts.push(h);
                    const EAST_ROLES: [Role; 3] =
                        [Role::ConferenceRoom, Role::Breakroom, Role::ServerCloset];
                    for i in 0..n_east {
                        let (y0, y1) = (cuts[i], cuts[i + 1]);
                        let door = shell.rng.range(y0 + 1, (y1 - 1).max(y0 + 2));
                        if i > 0 {
                            shell.partition_h_styled(
                                y0,
                                sx,
                                w,
                                &[],
                                (WallEdgeKind::Wall, MaterialClass::Drywall),
                            );
                        }
                        if i == 0 {
                            shell.partition_v_styled(
                                sx,
                                y0,
                                y1,
                                &[door],
                                (WallEdgeKind::Window, MaterialClass::Glass),
                            );
                            shell.put(
                                sx,
                                y0,
                                EdgeSide::West,
                                WallEdgeKind::Wall,
                                MaterialClass::Drywall,
                            );
                            if y1 - y0 >= 3 {
                                let kind = if door == y1 - 1 {
                                    WallEdgeKind::DoorOpen
                                } else {
                                    WallEdgeKind::Wall
                                };
                                shell.put(sx, y1 - 1, EdgeSide::West, kind, MaterialClass::Drywall);
                            }
                        } else {
                            shell.partition_v(sx, y0, y1, &[door]);
                        }
                        rooms_roles.push((
                            Room {
                                x0: sx,
                                y0,
                                x1: w,
                                y1,
                            },
                            EAST_ROLES[i % EAST_ROLES.len()],
                        ));
                    }
                    rooms_roles.insert(
                        0,
                        (
                            Room {
                                x0: 0,
                                y0: 0,
                                x1: sx,
                                y1: h,
                            },
                            Role::Bullpen,
                        ),
                    );
                } else {
                    let sx = (w / 2).clamp(4, w - 3);
                    let door = mid_y.clamp(1, h - 2);
                    shell.partition_v(sx, 0, h, &[door]);
                    rooms_roles.push((
                        Room {
                            x0: 0,
                            y0: 0,
                            x1: sx,
                            y1: h,
                        },
                        Role::Bullpen,
                    ));
                    rooms_roles.push((
                        Room {
                            x0: sx,
                            y0: 0,
                            x1: w,
                            y1: h,
                        },
                        Role::ServerCloset,
                    ));
                }
                shell.windows(3);
            }
        }
        vec![satya_schema::MapSurfaceFeature {
            kind: satya_schema::SurfaceFeatureKind::InteriorFloor,
            shape: satya_schema::SurfaceShapeKind::Polygon,
            points_cm: vec![
                satya_schema::MapPointCm::new(0, 0),
                satya_schema::MapPointCm::new(w * 100, 0),
                satya_schema::MapPointCm::new(w * 100, h * 100),
                satya_schema::MapPointCm::new(0, h * 100),
            ],
            width_cm: 0,
            border_style_cm: 0,
            variant_seed: (seed as u32).wrapping_add(7),
            tint_rgb: None,
        }]
    };

    let interior_count = surfaces
        .iter()
        .filter(|s| s.kind == satya_schema::SurfaceFeatureKind::InteriorFloor)
        .count();
    let mut wood_kept_per_poly = vec![false; interior_count.max(1)];
    let mut room_overlays = Vec::new();
    for &(r, role) in &rooms_roles {
        let poly_idx = surfaces
            .iter()
            .filter(|s| s.kind == satya_schema::SurfaceFeatureKind::InteriorFloor)
            .position(|s| {
                let cx = (r.x0 + r.x1) as f32 * 0.5;
                let cy = (r.y0 + r.y1) as f32 * 0.5;
                crate::map_surface_queries::surface_contains_point_m(s, cx, cy)
            })
            .unwrap_or(0);
        if let Some(sf) =
            emit_room_floor_surface(r, role, EconomicTier::MiddleClass, &mut shell.rng)
        {
            room_overlays.push((poly_idx, sf));
        } else {
            wood_kept_per_poly[poly_idx] = true;
        }
    }
    for (poly_idx, sf) in room_overlays {
        if !wood_kept_per_poly[poly_idx] {
            wood_kept_per_poly[poly_idx] = true;
            continue;
        }
        surfaces.push(sf);
    }
    prune_occluded_surfaces(&mut surfaces, w, h);

    // Reserve spawn pads and both sides of every door so thresholds stay walkable.
    let mut furn = Furnisher::new(w, h, shell.rng.next());
    for &(sx, sy) in &[
        (2, mid_y),
        (2, (mid_y - 1).max(1)),
        (w - 3, mid_y),
        (w - 3, (mid_y + 1).min(h - 2)),
    ] {
        furn.reserve_cell(sx, sy);
    }
    furn.reserve_doors(&shell.edges);
    for &(r, role) in &rooms_roles {
        furn.furnish(role, r);
    }
    if style == InteriorStyle::Office
        && w >= 24
        && h >= 18
        && let Some((er, _)) = rooms_roles
            .iter()
            .find(|(r, role)| *role == Role::Hall && r.w() >= 3 && r.h() >= 2)
            .or_else(|| {
                rooms_roles
                    .iter()
                    .find(|(r, role)| *role == Role::Bullpen && r.w() >= 4 && r.h() >= 3)
            })
    {
        let _ = furn.try_place(
            satya_schema::PropKind::ElevatorCar,
            er.x0 + 1,
            er.y0,
            0,
            None,
            *er,
        );
    }
    let pool_surfs: Vec<_> = surfaces
        .iter()
        .filter(|s| s.kind == SurfaceFeatureKind::PoolWater)
        .cloned()
        .collect();
    let has_pool = !pool_surfs.is_empty();
    let has_spa_deck = surfaces
        .iter()
        .any(|s| s.kind == SurfaceFeatureKind::WoodDeck);
    for pr in patio_rooms {
        for yy in pr.y0..pr.y1 {
            for xx in pr.x0..pr.x1 {
                let cx = xx as f32 + 0.5;
                let cy = yy as f32 + 0.5;
                let in_pool = pool_surfs.iter().any(|ps| {
                    [
                        (0.0, 0.0),
                        (-0.35, -0.35),
                        (0.35, -0.35),
                        (-0.35, 0.35),
                        (0.35, 0.35),
                    ]
                    .iter()
                    .any(|&(ox, oy)| surface_contains_point_m(ps, cx + ox, cy + oy))
                });
                if in_pool {
                    furn.reserve_cell(xx, yy);
                }
            }
        }
        if style == InteriorStyle::Apartment && (has_pool || has_spa_deck) {
            let pcx = (pr.x0 + pr.x1) as f32 * 0.5;
            let pcy = (pr.y0 + pr.y1) as f32 * 0.5;
            let spa_style = SpaStyle::ALL[(furn.rng.next() as usize) % SpaStyle::ALL.len()];
            let spa_seed = spa_variant_seed(furn.rng.next() as u32, spa_style);
            if has_spa_deck && !has_pool {
                let hx = pr.x0 + (pr.w() - 2).max(0) / 2;
                let hy = pr.y0 + (pr.h() - 2).max(0) / 2;
                if furn
                    .try_place(PropKind::HotTub, hx, hy, 0, None, pr)
                    .is_some()
                    && let Some(last) = furn.props.last_mut()
                {
                    last.variant_seed = spa_seed;
                }
            } else if has_pool && pr.w() >= 6 && pr.h() >= 5 {
                let mut placed_tub = false;
                for yy in pr.y0..=(pr.y1 - 2) {
                    if placed_tub {
                        break;
                    }
                    for xx in pr.x0..=(pr.x1 - 2) {
                        if furn
                            .try_place(PropKind::HotTub, xx, yy, 0, None, pr)
                            .is_some()
                        {
                            if let Some(last) = furn.props.last_mut() {
                                last.variant_seed = spa_seed;
                            }
                            placed_tub = true;
                            break;
                        }
                    }
                }
            }
            let mut loungers = 0usize;
            for yy in (pr.y0 + 1)..(pr.y1 - 1) {
                for xx in (pr.x0 + 1)..(pr.x1 - 1) {
                    if loungers >= 2 {
                        break;
                    }
                    if (xx + yy) % 3 != 0 || furn.blocked[(yy * furn.w + xx) as usize] {
                        continue;
                    }
                    let cx = xx as f32 + 0.5;
                    let cy = yy as f32 + 0.5;
                    let rot = vehicle_forward_rot_deg(pcx - cx, pcy - cy);
                    let cand = make_prop_at_cm(
                        PropKind::ReclinerChair,
                        (cx * 100.0).round() as i32,
                        (cy * 100.0).round() as i32,
                        rot,
                        None,
                        furn.rng.next() as u32,
                    );
                    let fits = cand.x >= pr.x0
                        && cand.y >= pr.y0
                        && cand.x + cand.w_cells <= pr.x1
                        && cand.y + cand.h_cells <= pr.y1
                        && (cand.y..(cand.y + cand.h_cells)).all(|cy_i| {
                            (cand.x..(cand.x + cand.w_cells)).all(|cx_i| {
                                !cand.covers(cx_i, cy_i)
                                    || !furn.blocked[(cy_i * furn.w + cx_i) as usize]
                            })
                        });
                    if fits {
                        for cy_i in cand.y..(cand.y + cand.h_cells) {
                            for cx_i in cand.x..(cand.x + cand.w_cells) {
                                if cand.covers(cx_i, cy_i) {
                                    furn.reserve_cell(cx_i, cy_i);
                                }
                            }
                        }
                        furn.props.push(cand);
                        loungers += 1;
                    }
                }
            }
        }
        furn.furnish_patio(pr);
    }

    // Connectivity safeguard: drop solid obstacles (tallest/newest first) until every open area is reachable.
    let mut props = furn.props;
    loop {
        if all_walkable_areas_connected(w, h, &shell.edges, &props) {
            return ProceduralSector {
                wall_edges: shell.edges,
                props,
                surfaces: surfaces.clone(),
                ..ProceduralSector::default()
            };
        }
        match props
            .iter()
            .rposition(|p| prop_spec(p.kind).obstruction == PropObstruction::FullObstacle)
        {
            Some(i) => {
                props.remove(i);
            }
            None => {
                return ProceduralSector {
                    wall_edges: shell.edges,
                    props,
                    surfaces,
                    ..ProceduralSector::default()
                };
            }
        }
    }
}
