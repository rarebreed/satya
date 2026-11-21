//! # Macro-World Zone Classification & Neighbor Transition Matrix (`map_world_transitions`)
//!
//! Extracted from [`crate::map_world_graph`] to keep each module focused (`~330` lines):
//! classifies all 25 [`SectorPreset`]s into [`MacroZoneFamily`] buckets and selects
//! realistic road/water-gated neighbor presets (`Hospital`, `ShoppingPlaza`, `ShoppingMall`,
//! `Farms`, `BotanicalGarden`, `ManufacturingPlant`, `Warehouse`, and legacy zones).

use satya_schema::{EconomicTier, MapContinuationDir, WorldSectorCell, ZoningContext};

use crate::map_outdoor_gen::Rng;
use crate::tactical_map::SectorPreset;

/// Broad land-use / structural family used for run-depth decay and neighbor transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacroZoneFamily {
    CommercialCore,
    UrbanResidential,
    SuburbanEstate,
    CivicCampus,
    IndustrialDistrict,
    NaturalWoodland,
    RuralFarmland,
    IndoorWing,
    FacilityWing,
}

impl MacroZoneFamily {
    /// Classifies a [`SectorPreset`] (plus optional [`ZoningContext`]) into its macro zone family.
    #[must_use]
    pub const fn for_preset(preset: SectorPreset, zoning: Option<ZoningContext>) -> Self {
        if let Some(z) = zoning {
            return match z {
                ZoningContext::DowntownCommercial => Self::CommercialCore,
                ZoningContext::UrbanResidential | ZoningContext::ProjectsHighDensity => {
                    Self::UrbanResidential
                }
                ZoningContext::SuburbanNeighborhood => Self::SuburbanEstate,
                ZoningContext::RuralCountryside => Self::RuralFarmland,
            };
        }
        match preset {
            SectorPreset::DowntownCommercial
            | SectorPreset::UrbanCrossroads
            | SectorPreset::ShoppingPlaza
            | SectorPreset::ShoppingMall => Self::CommercialCore,
            SectorPreset::SuburbanNeighborhood
            | SectorPreset::LuxuryMansion
            | SectorPreset::GardenEstate => Self::SuburbanEstate,
            SectorPreset::Hospital | SectorPreset::CommunityClinicAnnex => Self::CivicCampus,
            SectorPreset::ManufacturingPlant
            | SectorPreset::Warehouse
            | SectorPreset::BrownoutLogisticsDepot => Self::IndustrialDistrict,
            SectorPreset::WoodedPark
            | SectorPreset::WoodlandCreek
            | SectorPreset::BotanicalGarden
            | SectorPreset::RoadsideDitch => Self::NaturalWoodland,
            SectorPreset::RuralCountryside | SectorPreset::Farms => Self::RuralFarmland,
            SectorPreset::UrbanResidential
            | SectorPreset::ProjectsHighDensity
            | SectorPreset::SatyangSafehouse => Self::UrbanResidential,
            SectorPreset::ResidentialApartment
            | SectorPreset::OfficeFloor
            | SectorPreset::CornerBar => Self::IndoorWing,
            SectorPreset::DockingBay
            | SectorPreset::MedLabSector
            | SectorPreset::DerelictCorridor
            | SectorPreset::OrbitalAirlock
            | SectorPreset::CorporateServerBunker => Self::FacilityWing,
        }
    }
}

/// Resolved procedural specification for one newly stitched neighbor sector chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborSectorSpec {
    pub preset: SectorPreset,
    pub zoning: Option<ZoningContext>,
    pub economic_tier: Option<EconomicTier>,
    pub zone_depth: u8,
    /// When `Some(side)`, mirrors the generated chunk if needed so its main road faces `side`.
    pub preferred_road_side: Option<MapContinuationDir>,
}

fn urban_tier_shift(base: Option<EconomicTier>, roll: u32) -> EconomicTier {
    match base.unwrap_or(EconomicTier::MiddleClass) {
        EconomicTier::UpperMiddleClass | EconomicTier::LuxuryMansion => {
            if roll < 70 {
                EconomicTier::UpperMiddleClass
            } else {
                EconomicTier::MiddleClass
            }
        }
        EconomicTier::MiddleClass => {
            if roll < 65 {
                EconomicTier::MiddleClass
            } else {
                EconomicTier::UpperMiddleClass
            }
        }
        EconomicTier::LowIncome => {
            if roll < 65 {
                EconomicTier::LowIncome
            } else {
                EconomicTier::MiddleClass
            }
        }
    }
}

fn zoned(
    preset: SectorPreset,
    z: ZoningContext,
    t: EconomicTier,
    zd: u8,
    road: Option<MapContinuationDir>,
) -> NeighborSectorSpec {
    NeighborSectorSpec {
        preset,
        zoning: Some(z),
        economic_tier: Some(t),
        zone_depth: zd,
        preferred_road_side: road,
    }
}

fn plain(preset: SectorPreset, zd: u8, road: Option<MapContinuationDir>) -> NeighborSectorSpec {
    NeighborSectorSpec {
        preset,
        zoning: None,
        economic_tier: None,
        zone_depth: zd,
        preferred_road_side: road,
    }
}

/// Distance-weighted and road/water-gated neighbor transition selector for stitching `dir` from `source`.
#[must_use]
pub fn select_neighbor_sector_transition(
    source: &WorldSectorCell,
    all_sectors: &[WorldSectorCell],
    target_coord: (i32, i32),
    dir: MapContinuationDir,
    seed: i64,
) -> NeighborSectorSpec {
    let src_preset = SectorPreset::from_code_str(&source.preset).unwrap_or_default();
    let src_edge = source.boundary.edge(dir);
    let family = MacroZoneFamily::for_preset(src_preset, source.zoning);
    let depth = source.zone_depth;
    let road_opp = src_edge.road_parallel.then_some(dir.opposite());

    let mut rng = Rng((seed as u64)
        ^ ((target_coord.0 as i64 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
        ^ ((target_coord.1 as i64 as u64).wrapping_mul(0xBF58_476D_1CE4_E5B9))
        ^ ((dir as u64) << 40));
    let roll = (rng.next() % 100) as u32;
    let sub_roll = (rng.next() % 100) as u32;

    let orth_has_commercial = all_sectors.iter().any(|c| {
        (c.sector_x - target_coord.0).abs() + (c.sector_y - target_coord.1).abs() == 1
            && SectorPreset::from_code_str(&c.preset).is_some_and(|p| {
                matches!(
                    p,
                    SectorPreset::DowntownCommercial
                        | SectorPreset::UrbanCrossroads
                        | SectorPreset::ShoppingPlaza
                        | SectorPreset::ShoppingMall
                )
            })
    });

    match family {
        MacroZoneFamily::CommercialCore => {
            let is_plaza_or_mall = matches!(
                src_preset,
                SectorPreset::ShoppingPlaza | SectorPreset::ShoppingMall
            );
            if depth == 0 {
                if !is_plaza_or_mall && roll < 82 {
                    let next_p = if src_preset == SectorPreset::UrbanCrossroads && sub_roll < 55 {
                        SectorPreset::UrbanCrossroads
                    } else if roll < 58 {
                        SectorPreset::DowntownCommercial
                    } else if roll < 76 {
                        SectorPreset::UrbanCrossroads
                    } else {
                        SectorPreset::ShoppingPlaza
                    };
                    if next_p.is_zoned_preset() {
                        zoned(
                            next_p,
                            ZoningContext::DowntownCommercial,
                            EconomicTier::UpperMiddleClass,
                            1,
                            road_opp,
                        )
                    } else {
                        plain(next_p, 1, road_opp)
                    }
                } else if is_plaza_or_mall && roll < 55 {
                    let next_p = if sub_roll < 36 {
                        SectorPreset::DowntownCommercial
                    } else if sub_roll < 64 {
                        SectorPreset::ShoppingPlaza
                    } else if sub_roll < 86 {
                        SectorPreset::ShoppingMall
                    } else {
                        SectorPreset::Hospital
                    };
                    if next_p.is_zoned_preset() {
                        zoned(
                            next_p,
                            ZoningContext::DowntownCommercial,
                            EconomicTier::UpperMiddleClass,
                            1,
                            road_opp,
                        )
                    } else {
                        plain(next_p, 1, road_opp)
                    }
                } else if !is_plaza_or_mall && (82..90).contains(&roll) {
                    plain(SectorPreset::Hospital, 0, road_opp)
                } else if !is_plaza_or_mall && (90..96).contains(&roll) {
                    plain(SectorPreset::Warehouse, 0, road_opp)
                } else if !is_plaza_or_mall && roll >= 96 {
                    plain(SectorPreset::ShoppingMall, 0, road_opp)
                } else if sub_roll < 70 {
                    zoned(
                        SectorPreset::UrbanResidential,
                        ZoningContext::UrbanResidential,
                        EconomicTier::UpperMiddleClass,
                        0,
                        road_opp,
                    )
                } else {
                    zoned(
                        SectorPreset::SuburbanNeighborhood,
                        ZoningContext::SuburbanNeighborhood,
                        EconomicTier::UpperMiddleClass,
                        0,
                        road_opp,
                    )
                }
            } else if depth == 1 {
                if !is_plaza_or_mall && roll < 45 {
                    let next_p = if sub_roll < 60 {
                        SectorPreset::DowntownCommercial
                    } else {
                        SectorPreset::UrbanCrossroads
                    };
                    if next_p.is_zoned_preset() {
                        zoned(
                            next_p,
                            ZoningContext::DowntownCommercial,
                            EconomicTier::UpperMiddleClass,
                            2,
                            road_opp,
                        )
                    } else {
                        plain(next_p, 2, road_opp)
                    }
                } else if is_plaza_or_mall && roll < 30 {
                    plain(SectorPreset::ShoppingPlaza, 2, road_opp)
                } else if roll < 78 {
                    zoned(
                        SectorPreset::UrbanResidential,
                        ZoningContext::UrbanResidential,
                        EconomicTier::UpperMiddleClass,
                        0,
                        road_opp,
                    )
                } else {
                    zoned(
                        SectorPreset::SuburbanNeighborhood,
                        ZoningContext::SuburbanNeighborhood,
                        EconomicTier::UpperMiddleClass,
                        0,
                        road_opp,
                    )
                }
            } else if roll < 65 {
                zoned(
                    SectorPreset::UrbanResidential,
                    ZoningContext::UrbanResidential,
                    EconomicTier::UpperMiddleClass,
                    0,
                    road_opp,
                )
            } else if roll < 92 {
                zoned(
                    SectorPreset::SuburbanNeighborhood,
                    ZoningContext::SuburbanNeighborhood,
                    EconomicTier::UpperMiddleClass,
                    0,
                    road_opp,
                )
            } else {
                zoned(
                    SectorPreset::UrbanResidential,
                    ZoningContext::UrbanResidential,
                    EconomicTier::MiddleClass,
                    0,
                    road_opp,
                )
            }
        }

        MacroZoneFamily::CivicCampus => {
            if src_edge.road_parallel {
                if roll < 45 {
                    zoned(
                        SectorPreset::DowntownCommercial,
                        ZoningContext::DowntownCommercial,
                        EconomicTier::UpperMiddleClass,
                        0,
                        road_opp,
                    )
                } else if roll < 80 {
                    zoned(
                        SectorPreset::UrbanResidential,
                        ZoningContext::UrbanResidential,
                        EconomicTier::UpperMiddleClass,
                        0,
                        road_opp,
                    )
                } else {
                    plain(SectorPreset::UrbanCrossroads, 0, road_opp)
                }
            } else if depth == 0 && roll < 35 {
                plain(SectorPreset::Hospital, 1, None)
            } else if roll < 68 {
                zoned(
                    SectorPreset::UrbanResidential,
                    ZoningContext::UrbanResidential,
                    EconomicTier::UpperMiddleClass,
                    0,
                    None,
                )
            } else {
                plain(SectorPreset::BotanicalGarden, 0, None)
            }
        }

        MacroZoneFamily::IndustrialDistrict => {
            if depth == 0 && roll < 65 {
                let wh_thresh = if src_preset == SectorPreset::ManufacturingPlant {
                    60
                } else {
                    55
                };
                let next_p = if sub_roll < wh_thresh {
                    SectorPreset::Warehouse
                } else {
                    SectorPreset::ManufacturingPlant
                };
                plain(next_p, 1, road_opp)
            } else if roll < 42 {
                zoned(
                    SectorPreset::ProjectsHighDensity,
                    ZoningContext::ProjectsHighDensity,
                    EconomicTier::LowIncome,
                    0,
                    road_opp,
                )
            } else if roll < 72 {
                plain(SectorPreset::UrbanCrossroads, 0, road_opp)
            } else {
                plain(SectorPreset::RoadsideDitch, 0, None)
            }
        }

        MacroZoneFamily::NaturalWoodland => {
            if src_preset == SectorPreset::BotanicalGarden {
                if src_edge.road_parallel {
                    if roll < 50 {
                        zoned(
                            SectorPreset::UrbanResidential,
                            ZoningContext::UrbanResidential,
                            EconomicTier::UpperMiddleClass,
                            0,
                            road_opp,
                        )
                    } else if roll < 82 {
                        zoned(
                            SectorPreset::SuburbanNeighborhood,
                            ZoningContext::SuburbanNeighborhood,
                            EconomicTier::UpperMiddleClass,
                            0,
                            road_opp,
                        )
                    } else {
                        zoned(
                            SectorPreset::LuxuryMansion,
                            ZoningContext::SuburbanNeighborhood,
                            EconomicTier::LuxuryMansion,
                            0,
                            road_opp,
                        )
                    }
                } else if roll < 50 {
                    plain(SectorPreset::WoodedPark, depth.saturating_add(1), None)
                } else if roll < 82 {
                    plain(SectorPreset::WoodlandCreek, depth.saturating_add(1), None)
                } else {
                    zoned(
                        SectorPreset::SuburbanNeighborhood,
                        ZoningContext::SuburbanNeighborhood,
                        EconomicTier::UpperMiddleClass,
                        0,
                        None,
                    )
                }
            } else if src_edge.road_parallel {
                if src_preset == SectorPreset::WoodedPark {
                    if roll < 40 {
                        let tier = if sub_roll < 55 {
                            EconomicTier::UpperMiddleClass
                        } else {
                            EconomicTier::MiddleClass
                        };
                        zoned(
                            SectorPreset::UrbanResidential,
                            ZoningContext::UrbanResidential,
                            tier,
                            0,
                            road_opp,
                        )
                    } else if roll < 75 {
                        let tier = if sub_roll < 60 {
                            EconomicTier::UpperMiddleClass
                        } else {
                            EconomicTier::MiddleClass
                        };
                        zoned(
                            SectorPreset::SuburbanNeighborhood,
                            ZoningContext::SuburbanNeighborhood,
                            tier,
                            0,
                            road_opp,
                        )
                    } else {
                        zoned(
                            SectorPreset::RuralCountryside,
                            ZoningContext::RuralCountryside,
                            EconomicTier::MiddleClass,
                            0,
                            road_opp,
                        )
                    }
                } else if roll < 55 {
                    zoned(
                        SectorPreset::RuralCountryside,
                        ZoningContext::RuralCountryside,
                        EconomicTier::LowIncome,
                        0,
                        road_opp,
                    )
                } else if roll < 85 {
                    zoned(
                        SectorPreset::SuburbanNeighborhood,
                        ZoningContext::SuburbanNeighborhood,
                        EconomicTier::MiddleClass,
                        0,
                        road_opp,
                    )
                } else {
                    plain(SectorPreset::WoodedPark, 0, road_opp)
                }
            } else if src_edge.has_water() {
                if roll < 55 {
                    plain(SectorPreset::WoodlandCreek, depth.saturating_add(1), None)
                } else if roll < 85 {
                    zoned(
                        SectorPreset::RuralCountryside,
                        ZoningContext::RuralCountryside,
                        EconomicTier::MiddleClass,
                        0,
                        None,
                    )
                } else {
                    plain(SectorPreset::WoodedPark, depth.saturating_add(1), None)
                }
            } else if !src_edge.crossing_roads_cm.is_empty() {
                if depth < 2 {
                    if roll < 45 {
                        zoned(
                            SectorPreset::RuralCountryside,
                            ZoningContext::RuralCountryside,
                            EconomicTier::MiddleClass,
                            0,
                            None,
                        )
                    } else if roll < 75 {
                        plain(SectorPreset::RoadsideDitch, depth.saturating_add(1), None)
                    } else {
                        plain(SectorPreset::WoodedPark, depth.saturating_add(1), None)
                    }
                } else if roll < 65 {
                    zoned(
                        SectorPreset::RuralCountryside,
                        ZoningContext::RuralCountryside,
                        EconomicTier::MiddleClass,
                        0,
                        None,
                    )
                } else {
                    plain(SectorPreset::RoadsideDitch, 0, None)
                }
            } else if roll < 45 {
                zoned(
                    SectorPreset::RuralCountryside,
                    ZoningContext::RuralCountryside,
                    EconomicTier::LowIncome,
                    0,
                    None,
                )
            } else if roll < 78 {
                plain(SectorPreset::WoodedPark, depth.saturating_add(1), None)
            } else {
                plain(SectorPreset::WoodlandCreek, depth.saturating_add(1), None)
            }
        }

        MacroZoneFamily::UrbanResidential => {
            let tier = urban_tier_shift(source.economic_tier, sub_roll);
            if depth < 2 {
                if src_preset == SectorPreset::ProjectsHighDensity && roll < 55 {
                    zoned(
                        SectorPreset::ProjectsHighDensity,
                        ZoningContext::ProjectsHighDensity,
                        EconomicTier::LowIncome,
                        depth + 1,
                        road_opp,
                    )
                } else if roll < 50 {
                    zoned(
                        SectorPreset::UrbanResidential,
                        ZoningContext::UrbanResidential,
                        tier,
                        depth + 1,
                        road_opp,
                    )
                } else if roll < 72 || (orth_has_commercial && roll < 82) {
                    if sub_roll < 30 {
                        plain(SectorPreset::ShoppingPlaza, 0, road_opp)
                    } else {
                        zoned(
                            SectorPreset::DowntownCommercial,
                            ZoningContext::DowntownCommercial,
                            EconomicTier::UpperMiddleClass,
                            0,
                            road_opp,
                        )
                    }
                } else if roll < 88 {
                    zoned(
                        SectorPreset::SuburbanNeighborhood,
                        ZoningContext::SuburbanNeighborhood,
                        tier,
                        0,
                        road_opp,
                    )
                } else {
                    plain(SectorPreset::WoodedPark, 0, road_opp)
                }
            } else if roll < 45 {
                zoned(
                    SectorPreset::SuburbanNeighborhood,
                    ZoningContext::SuburbanNeighborhood,
                    tier,
                    0,
                    road_opp,
                )
            } else if roll < 75 {
                plain(SectorPreset::WoodedPark, 0, road_opp)
            } else {
                zoned(
                    SectorPreset::DowntownCommercial,
                    ZoningContext::DowntownCommercial,
                    EconomicTier::UpperMiddleClass,
                    0,
                    road_opp,
                )
            }
        }

        MacroZoneFamily::SuburbanEstate => {
            if matches!(
                src_preset,
                SectorPreset::LuxuryMansion | SectorPreset::GardenEstate
            ) {
                if src_edge.road_parallel {
                    if roll < 60 {
                        zoned(
                            SectorPreset::SuburbanNeighborhood,
                            ZoningContext::SuburbanNeighborhood,
                            EconomicTier::UpperMiddleClass,
                            0,
                            road_opp,
                        )
                    } else if roll < 85 {
                        zoned(
                            SectorPreset::LuxuryMansion,
                            ZoningContext::SuburbanNeighborhood,
                            EconomicTier::LuxuryMansion,
                            depth.saturating_add(1),
                            road_opp,
                        )
                    } else {
                        zoned(
                            SectorPreset::RuralCountryside,
                            ZoningContext::RuralCountryside,
                            EconomicTier::UpperMiddleClass,
                            0,
                            road_opp,
                        )
                    }
                } else if roll < 45 {
                    plain(SectorPreset::WoodedPark, 0, None)
                } else if roll < 75 {
                    zoned(
                        SectorPreset::RuralCountryside,
                        ZoningContext::RuralCountryside,
                        EconomicTier::MiddleClass,
                        0,
                        None,
                    )
                } else {
                    plain(SectorPreset::WoodlandCreek, 0, None)
                }
            } else if depth < 2 {
                let tier = urban_tier_shift(source.economic_tier, sub_roll);
                if roll < 55 {
                    zoned(
                        SectorPreset::SuburbanNeighborhood,
                        ZoningContext::SuburbanNeighborhood,
                        tier,
                        depth + 1,
                        road_opp,
                    )
                } else if roll < 70 {
                    let next_p = if sub_roll < 65 {
                        SectorPreset::LuxuryMansion
                    } else {
                        SectorPreset::GardenEstate
                    };
                    zoned(
                        next_p,
                        ZoningContext::SuburbanNeighborhood,
                        EconomicTier::LuxuryMansion,
                        0,
                        road_opp,
                    )
                } else if roll < 76 {
                    plain(SectorPreset::BotanicalGarden, 0, road_opp)
                } else if roll < 86 {
                    plain(SectorPreset::WoodedPark, 0, road_opp)
                } else {
                    zoned(
                        SectorPreset::UrbanResidential,
                        ZoningContext::UrbanResidential,
                        tier,
                        0,
                        road_opp,
                    )
                }
            } else if roll < 45 {
                zoned(
                    SectorPreset::RuralCountryside,
                    ZoningContext::RuralCountryside,
                    EconomicTier::MiddleClass,
                    0,
                    None,
                )
            } else if roll < 80 {
                plain(SectorPreset::WoodedPark, 0, road_opp)
            } else {
                plain(SectorPreset::RoadsideDitch, 0, None)
            }
        }

        MacroZoneFamily::RuralFarmland => {
            if src_edge.has_water() && roll < 45 {
                plain(SectorPreset::WoodlandCreek, depth.saturating_add(1), None)
            } else if src_edge.has_any_road() && (45..75).contains(&roll) {
                plain(SectorPreset::RoadsideDitch, 0, None)
            } else if src_edge.road_parallel && (75..90).contains(&roll) {
                zoned(
                    SectorPreset::SuburbanNeighborhood,
                    ZoningContext::SuburbanNeighborhood,
                    EconomicTier::MiddleClass,
                    0,
                    road_opp,
                )
            } else if roll < 40 {
                plain(SectorPreset::Farms, (depth + 1).min(4), None)
            } else if roll < 68 {
                zoned(
                    SectorPreset::RuralCountryside,
                    ZoningContext::RuralCountryside,
                    EconomicTier::LowIncome,
                    (depth + 1).min(4),
                    None,
                )
            } else if roll < 86 {
                plain(SectorPreset::WoodedPark, 0, None)
            } else {
                plain(SectorPreset::WoodlandCreek, 0, None)
            }
        }

        MacroZoneFamily::IndoorWing => {
            let next_p = if roll < 72 {
                src_preset
            } else if src_preset == SectorPreset::ResidentialApartment {
                if sub_roll < 60 {
                    SectorPreset::OfficeFloor
                } else {
                    SectorPreset::CornerBar
                }
            } else if src_preset == SectorPreset::OfficeFloor {
                if sub_roll < 65 {
                    SectorPreset::ResidentialApartment
                } else {
                    SectorPreset::CornerBar
                }
            } else if sub_roll < 55 {
                SectorPreset::ResidentialApartment
            } else {
                SectorPreset::OfficeFloor
            };
            plain(next_p, depth.saturating_add(1), None)
        }

        MacroZoneFamily::FacilityWing => {
            let wings = [
                SectorPreset::DockingBay,
                SectorPreset::MedLabSector,
                SectorPreset::DerelictCorridor,
                SectorPreset::OrbitalAirlock,
            ];
            let next_p = if roll < 65 {
                src_preset
            } else {
                let candidates: Vec<SectorPreset> =
                    wings.into_iter().filter(|&p| p != src_preset).collect();
                candidates[(sub_roll as usize) % candidates.len()]
            };
            plain(next_p, depth.saturating_add(1), None)
        }
    }
}
