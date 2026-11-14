//! Continuous Vector Terrain Surface Features (`tactical_map_surfaces_dto`)
//!
//! Stores curved splines (`RibbonSpline`) and smooth closed ground contours (`OrganicClosed`,
//! `Polygon`) in integer centimeter (`100 cm = 1.00 m`) wire coordinates (`Eq`-safe) so roads,
//! creeks, drainage ditches, trenches, mulch beds, and garden beds render smoothly in both 2D SVG
//! and 3D WebGPU without `1m` row-run stair-stepping.
//!
//! Re-exported through [`crate::tactical_map_dto`]. Geometry evaluation, spline extrusion,
//! triangulation, and priority queries live in `satya_engine::map_surface_geom`.

use crate::define_wire_enum;
use crate::tactical_map_dressing_dto::MaterialClass;
use serde::{Deserialize, Serialize};

define_wire_enum! {
    /// Classification of a continuous vector terrain surface feature.
    #[derive(Default)]
    pub enum SurfaceFeatureKind [count = 12] {
        #[default]
        MulchZone => ("MULCH_ZONE", "Organic Bark Mulch"),
        GardenBedZone => ("GARDEN_BED_ZONE", "Curved Garden Bed"),
        DirtTrail => ("DIRT_TRAIL", "Packed Dirt Trail"),
        DrainageDitch => ("DRAINAGE_DITCH", "Drainage Ditch"),
        DefensiveTrench => ("DEFENSIVE_TRENCH", "Excavated Trench"),
        CreekChannel => ("CREEK_CHANNEL", "Creek / Shallow Stream"),
        GravelRoad => ("GRAVEL_ROAD", "Gravel Lane / Shoal"),
        StoneWalk => ("STONE_WALK", "Flagstone Walkway"),
        ConcreteWalk => ("CONCRETE_WALK", "Concrete Sidewalk / Plaza"),
        AsphaltRoad => ("ASPHALT_ROAD", "Asphalt Road / Drive"),
        InteriorFloor => ("INTERIOR_FLOOR", "Building Interior Floor"),
        BridgeDeck => ("BRIDGE_DECK", "Bridge / Culvert Deck"),
    }
}

impl SurfaceFeatureKind {
    /// Deterministic compositing & masking z-priority (`BridgeDeck > InteriorFloor > AsphaltRoad > ... > MulchZone`).
    #[must_use]
    pub const fn layer_priority(self) -> u8 {
        match self {
            Self::MulchZone => 14,
            Self::GardenBedZone => 16,
            Self::DirtTrail => 24,
            Self::DrainageDitch => 28,
            Self::DefensiveTrench => 29,
            Self::CreekChannel => 30,
            Self::GravelRoad => 34,
            Self::StoneWalk => 38,
            Self::ConcreteWalk => 45,
            Self::AsphaltRoad => 50,
            Self::InteriorFloor => 60,
            Self::BridgeDeck => 70,
        }
    }

    /// Movement gait cost multiplier when crossing this surface (`> 1.0` = difficult footing).
    #[must_use]
    pub const fn move_cost_mult(self) -> f32 {
        match self {
            Self::CreekChannel | Self::DrainageDitch => 2.0,
            Self::DefensiveTrench => 1.5,
            Self::GardenBedZone => 1.25,
            _ => 1.0,
        }
    }

    /// Representative physical [`MaterialClass`] for hover inspection and acoustic/cover labels.
    #[must_use]
    pub const fn material(self) -> MaterialClass {
        match self {
            Self::MulchZone | Self::DirtTrail | Self::DrainageDitch | Self::DefensiveTrench | Self::CreekChannel => {
                MaterialClass::Earth
            }
            Self::GardenBedZone => MaterialClass::Foliage,
            Self::GravelRoad | Self::StoneWalk => MaterialClass::Ceramic,
            Self::ConcreteWalk | Self::AsphaltRoad => MaterialClass::ReinforcedConcrete,
            Self::InteriorFloor => MaterialClass::SolidWood,
            Self::BridgeDeck => MaterialClass::ConcreteBlock,
        }
    }

    /// Whether this surface represents a paved, gravel, or stone travel/building surface where organic
    /// mulch patches and planting beds should never intrude.
    #[must_use]
    pub const fn is_road_or_pavement(self) -> bool {
        matches!(
            self,
            Self::BridgeDeck
                | Self::InteriorFloor
                | Self::AsphaltRoad
                | Self::ConcreteWalk
                | Self::StoneWalk
                | Self::GravelRoad
        )
    }

    /// Whether this surface is a hydrological channel or excavated trench.
    #[must_use]
    pub const fn is_water_or_channel(self) -> bool {
        matches!(
            self,
            Self::CreekChannel | Self::DrainageDitch | Self::DefensiveTrench
        )
    }

    /// Whether this surface forms a bridge or continuous travel deck that masks ditch rims or road curbs beneath it.
    #[must_use]
    pub const fn is_bridge_or_road_overpass(self) -> bool {
        matches!(
            self,
            Self::BridgeDeck
                | Self::AsphaltRoad
                | Self::GravelRoad
                | Self::StoneWalk
                | Self::ConcreteWalk
        )
    }
}

define_wire_enum! {
    /// Geometric curve interpretation for [`MapSurfaceFeature::points_cm`].
    #[derive(Default)]
    pub enum SurfaceShapeKind [count = 3] {
        #[default]
        RibbonSpline => ("RIBBON_SPLINE", "Smooth Open Spline Ribbon"),
        OrganicClosed => ("ORGANIC_CLOSED", "Smooth Closed Organic Contour"),
        Polygon => ("POLYGON", "Rounded / Crisp Polygon"),
    }
}

/// 2D point in continuous map centimeters (`100 cm = 1.00 m`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct MapPointCm {
    pub x_cm: i32,
    pub y_cm: i32,
}

impl MapPointCm {
    /// Constructs a centimeter map coordinate.
    #[must_use]
    pub const fn new(x_cm: i32, y_cm: i32) -> Self {
        Self { x_cm, y_cm }
    }

    /// Converts to continuous meters `(x_m, y_m)`.
    #[must_use]
    pub fn to_m(self) -> (f32, f32) {
        (self.x_cm as f32 / 100.0, self.y_cm as f32 / 100.0)
    }

    /// Returns a translated copy shifted by `(dx_cm, dy_cm)`.
    #[must_use]
    pub const fn translated(self, dx_cm: i32, dy_cm: i32) -> Self {
        Self {
            x_cm: self.x_cm + dx_cm,
            y_cm: self.y_cm + dy_cm,
        }
    }
}

/// A smooth continuous vector terrain ribbon or closed ground zone on [`crate::protocol::GmMapUpdateRequest`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSurfaceFeature {
    pub kind: SurfaceFeatureKind,
    #[serde(default)]
    pub shape: SurfaceShapeKind,
    /// Control points in continuous centimeters (`100 cm = 1.00 m`).
    #[serde(default)]
    pub points_cm: Vec<MapPointCm>,
    /// Full ribbon width in centimeters for [`SurfaceShapeKind::RibbonSpline`] (`0` for closed shapes).
    #[serde(default)]
    pub width_cm: i32,
    /// Stone edging block width in cm for [`SurfaceFeatureKind::GardenBedZone`], or corner fillet radius in cm for [`SurfaceShapeKind::Polygon`].
    #[serde(default)]
    pub border_style_cm: i32,
    #[serde(default)]
    pub variant_seed: u32,
}

impl MapSurfaceFeature {
    /// Returns a copy whose control points are shifted by `(dx_cm, dy_cm)`.
    #[must_use]
    pub fn translated(&self, dx_cm: i32, dy_cm: i32) -> Self {
        let mut out = self.clone();
        for pt in &mut out.points_cm {
            *pt = pt.translated(dx_cm, dy_cm);
        }
        out
    }

    /// Mirrors all control points across X (`flip_x = true`) or Y (`flip_x = false`) within `(span_w_cm, span_h_cm)`.
    #[must_use]
    pub fn mirrored(&self, span_w_cm: i32, span_h_cm: i32, flip_x: bool) -> Self {
        let mut out = self.clone();
        for pt in &mut out.points_cm {
            if flip_x {
                pt.x_cm = span_w_cm - pt.x_cm;
            } else {
                pt.y_cm = span_h_cm - pt.y_cm;
            }
        }
        if matches!(out.shape, SurfaceShapeKind::OrganicClosed | SurfaceShapeKind::Polygon) {
            out.points_cm.reverse();
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_feature_serde_priority_and_transforms() {
        let feat = MapSurfaceFeature {
            kind: SurfaceFeatureKind::CreekChannel,
            shape: SurfaceShapeKind::RibbonSpline,
            points_cm: vec![
                MapPointCm::new(400, 0),
                MapPointCm::new(520, 700),
                MapPointCm::new(450, 1400),
            ],
            width_cm: 190,
            border_style_cm: 0,
            variant_seed: 17,
        };
        let json = serde_json::to_string(&feat).unwrap();
        let back: MapSurfaceFeature = serde_json::from_str(&json).unwrap();
        assert_eq!(feat, back);

        assert!(
            SurfaceFeatureKind::BridgeDeck.layer_priority()
                > SurfaceFeatureKind::AsphaltRoad.layer_priority()
        );
        assert!(
            SurfaceFeatureKind::AsphaltRoad.layer_priority()
                > SurfaceFeatureKind::CreekChannel.layer_priority()
        );
        assert!(
            SurfaceFeatureKind::CreekChannel.layer_priority()
                > SurfaceFeatureKind::GardenBedZone.layer_priority()
        );
        assert!(
            SurfaceFeatureKind::GardenBedZone.layer_priority()
                > SurfaceFeatureKind::MulchZone.layer_priority()
        );

        let moved = feat.translated(200, -100);
        assert_eq!(moved.points_cm[0], MapPointCm::new(600, -100));
        let flipped = feat.mirrored(2000, 1400, true);
        assert_eq!(flipped.points_cm[0], MapPointCm::new(1600, 0));
    }
}
