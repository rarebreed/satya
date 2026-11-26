//! # Painted Top-Down Battlemap Prop Sprite Registry (`map_2d_art_sprites`)
//!
//! Maps `140` discrete [`PropKind`] placements onto alpha-matted top-down WebP sprites (`3`
//! deterministic visual variants each = `420` assets) served from `/assets/sprites/props/*.webp`
//! when [`BattlemapTextureMode::Painted`] is active. Stretchable modular props tile horizontally
//! across longer runs, while [`BattlemapTextureMode::ProceduralVector`], custom GM `tint_rgb`
//! overrides, terrain ribbons, and overhead canopies fall back cleanly to procedural SVG glyphs.

use std::fmt::Write as _;

use satya_schema::{MapPropPlacement, PropKind};

use crate::wasm_client::map_2d_art_props::battlemap_mix_u64;

/// Base HTTP route prefix mounted by `satya-signaling` for painted prop sprites.
pub const BATTLEMAP_PROP_SPRITE_BASE_URL: &str = "/assets/sprites/props";

/// Static catalog entry pairing one [`PropKind`] with its alpha-matted WebP variant files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropSpriteSpec {
    pub kind: PropKind,
    pub variant_files: &'static [&'static str],
    pub base_w_cells: i32,
    pub base_h_cells: i32,
    pub tile_along_width: bool,
}

macro_rules! prop_sprite_specs {
    ($($kind:ident => $stem:literal ($w:literal x $h:literal $(, tile=$tile:literal)?));+ $(;)?) => {
        /// Complete `140`-prop × `3`-variant (`420` WebP sprites) registry synchronized with
        /// `src/satya_ml/texture_gen/prop_catalog.py`.
        pub const BATTLEMAP_PROP_SPRITE_SPECS: &[PropSpriteSpec] = &[
            $(PropSpriteSpec {
                kind: PropKind::$kind,
                variant_files: &[
                    concat!($stem, ".webp"),
                    concat!($stem, "_v1.webp"),
                    concat!($stem, "_v2.webp"),
                ],
                base_w_cells: $w,
                base_h_cells: $h,
                tile_along_width: prop_sprite_specs!(@tile $($tile)?),
            }),+
        ];
    };
    (@tile $t:literal) => { $t };
    (@tile) => { false };
}

prop_sprite_specs! {
    // --- Batch 1: Pilot Clinic & Vault Pack (12) ---
    HospitalBed => "hospital_bed" (2 x 1);
    Gurney => "gurney" (1 x 2);
    SurgicalTable => "surgical_table" (2 x 1);
    ImagingScanner => "imaging_scanner" (2 x 2);
    NurseStation => "nurse_station" (3 x 2, tile=true);
    TelemetryMonitor => "telemetry_monitor" (1 x 1);
    LinenCart => "linen_cart" (2 x 1);
    BiohazardBin => "biohazard_bin" (1 x 1);
    CryoComputeCluster => "cryo_compute_cluster" (2 x 1);
    BioServerVat => "bio_server_vat" (1 x 1);
    DieselGenerator => "diesel_generator" (2 x 1);
    UtilityVan => "utility_van" (2 x 5);
    // --- Batch 2: Clinic Lobby, Wards, Labs, Vault & Exterior (12) ---
    Cot => "cot" (1 x 2);
    LabBench => "lab_bench" (2 x 1, tile=true);
    FumeHood => "fume_hood" (2 x 1);
    AirGappedTerminal => "air_gapped_terminal" (1 x 1);
    BiometricTurnstile => "biometric_turnstile" (2 x 1);
    WaterPurifierKiosk => "water_purifier_kiosk" (1 x 1);
    BatteryBankWall => "battery_bank_wall" (2 x 1, tile=true);
    ReclinerChair => "recliner_chair" (1 x 1);
    Desk => "desk" (2 x 1);
    SteelShelving => "steel_shelving" (2 x 1);
    Dumpster => "dumpster" (2 x 1);
    AcUnit => "ac_unit" (1 x 1);
    // --- Batch 3: Living & Bedroom (16) ---
    Sofa => "sofa" (3 x 1);
    Loveseat => "loveseat" (2 x 1);
    Armchair => "armchair" (1 x 1);
    CoffeeTable => "coffee_table" (2 x 1);
    Rug => "rug" (3 x 2);
    RunnerRug => "runner_rug" (2 x 1, tile=true);
    RoundRug => "round_rug" (2 x 2);
    DiningTable => "dining_table" (2 x 2);
    RoundTable => "round_table" (1 x 1);
    Bed => "bed" (2 x 2);
    SingleBed => "single_bed" (1 x 2);
    Bookshelf => "bookshelf" (2 x 1);
    Wardrobe => "wardrobe" (2 x 1);
    Nightstand => "nightstand" (1 x 1);
    Dresser => "dresser" (2 x 1);
    TvStand => "tv_stand" (2 x 1, tile=true);
    // --- Batch 4: Kitchen, Bath & Bar (17) ---
    KitchenCounter => "kitchen_counter" (1 x 1, tile=true);
    Stove => "stove" (1 x 1);
    Fridge => "fridge" (1 x 1);
    BarCounter => "bar_counter" (1 x 1, tile=true);
    BackBarShelf => "back_bar_shelf" (1 x 1, tile=true);
    BarStool => "bar_stool" (1 x 1);
    PoolTable => "pool_table" (3 x 2);
    Jukebox => "jukebox" (1 x 1);
    Booth => "booth" (2 x 2);
    Bathtub => "bathtub" (2 x 1);
    Toilet => "toilet" (1 x 1);
    BathroomSink => "bathroom_sink" (1 x 1, tile=true);
    ShowerStall => "shower_stall" (1 x 1);
    SpaBath => "spa_bath" (2 x 2);
    LinenCloset => "linen_closet" (1 x 1);
    WasherDryer => "washer_dryer" (2 x 1);
    Stereo => "stereo" (1 x 1, tile=true);
    // --- Batch 5: Office, Clutter, Clinical & Vertical Portals (16) ---
    DeskL => "desk_l" (2 x 2);
    DeskT => "desk_t" (3 x 2);
    DeskLamp => "desk_lamp" (1 x 1);
    StandingLamp => "standing_lamp" (1 x 1);
    DeskClutter => "desk_clutter" (1 x 1);
    FilingCabinet => "filing_cabinet" (1 x 1, tile=true);
    WaterCooler => "water_cooler" (1 x 1);
    FireExtinguisher => "fire_extinguisher" (1 x 1);
    TrashCan => "trash_can" (1 x 1);
    FloorClothes => "floor_clothes" (1 x 1);
    FloorToys => "floor_toys" (1 x 1);
    MedicalCounter => "medical_counter" (1 x 1, tile=true);
    PrivacyCurtain => "privacy_curtain" (2 x 1, tile=true);
    Stairwell => "stairwell" (2 x 3);
    ElevatorCar => "elevator_car" (2 x 2);
    ServiceElevator => "service_elevator" (3 x 2);
    // --- Batch 6: Lab, Retail & Industrial (18) ---
    SolderingStation => "soldering_station" (2 x 1);
    TestBench => "test_bench" (2 x 1);
    Printer3dStation => "printer3d_station" (2 x 1);
    AirCompressor => "air_compressor" (1 x 1);
    CncMill => "cnc_mill" (3 x 2);
    ServerRack => "server_rack" (1 x 1, tile=true);
    DisplayRack => "display_rack" (2 x 1, tile=true);
    CheckoutCounter => "checkout_counter" (2 x 1, tile=true);
    MallKiosk => "mall_kiosk" (3 x 2);
    ConveyorBelt => "conveyor_belt" (3 x 1, tile=true);
    IndustrialPress => "industrial_press" (2 x 2);
    PalletRack => "pallet_rack" (3 x 1, tile=true);
    Forklift => "forklift" (2 x 1);
    ShippingContainer => "shipping_container" (6 x 2, tile=true);
    WoodCrate => "wood_crate" (1 x 1);
    DrumCluster => "drum_cluster" (1 x 1);
    StructuralPillar => "structural_pillar" (1 x 1);
    ElevatedCatwalk => "elevated_catwalk" (1 x 1, tile=true);
    // --- Batch 7: Outdoor, Vehicles & Infrastructure (23) ---
    Car => "car" (2 x 5);
    JerseyBarrier => "jersey_barrier" (2 x 1, tile=true);
    ParkBench => "park_bench" (2 x 1);
    PicnicTable => "picnic_table" (2 x 2);
    Planter => "planter" (1 x 1);
    PlanterTable => "planter_table" (2 x 1, tile=true);
    CropRow => "crop_row" (3 x 1, tile=true);
    Hedge => "hedge" (1 x 1, tile=true);
    Bush => "bush" (1 x 1);
    Boulder => "boulder" (2 x 1);
    FallenLog => "fallen_log" (3 x 1);
    SmallFountain => "small_fountain" (1 x 1);
    MediumFountain => "medium_fountain" (2 x 2);
    LargeFountain => "large_fountain" (3 x 3);
    BbqGrill => "bbq_grill" (1 x 1);
    GardenShed => "garden_shed" (3 x 2);
    HotTub => "hot_tub" (2 x 2);
    GarbagePile => "garbage_pile" (2 x 1);
    StreetDebris => "street_debris" (2 x 1, tile=true);
    BrownstoneStoop => "brownstone_stoop" (2 x 2);
    SecurityCameraMast => "security_camera_mast" (1 x 1);
    DroneDockPad => "drone_dock_pad" (2 x 2);
    HvTransformer => "hv_transformer" (2 x 2);
    // --- Batch 8: Cafe, Restaurant & Retail (10) ---
    EspressoCounter => "espresso_counter" (2 x 1, tile=true);
    CommercialPrepTable => "commercial_prep_table" (2 x 1, tile=true);
    CommercialGrillHood => "commercial_grill_hood" (2 x 1);
    PatioUmbrellaTable => "patio_umbrella_table" (2 x 2);
    BuffetCounter => "buffet_counter" (2 x 1, tile=true);
    RefrigeratedDisplayCase => "refrigerated_display_case" (2 x 1, tile=true);
    ClothingRack => "clothing_rack" (2 x 1, tile=true);
    PharmacyCounter => "pharmacy_counter" (2 x 1, tile=true);
    VendingMachine => "vending_machine" (1 x 1);
    AtmKiosk => "atm_kiosk" (1 x 1);
    // --- Batch 9: Industrial, Farm & Conservatory (8) ---
    RoboticAssemblyArm => "robotic_assembly_arm" (2 x 2);
    ToolChestWorkbench => "tool_chest_workbench" (2 x 1);
    CargoPallet => "cargo_pallet" (1 x 1);
    ElectricalSwitchgear => "electrical_switchgear" (2 x 1, tile=true);
    Tractor => "tractor" (2 x 3);
    HayBaleStack => "hay_bale_stack" (2 x 1);
    LivestockTrough => "livestock_trough" (2 x 1);
    ConservatorySpecimenIsland => "conservatory_specimen_island" (2 x 2);
    // --- Batch 10: Sci-Fi, Office & Streetscape (8) ---
    CommandConsole => "command_console" (2 x 1, tile=true);
    EvaSuitLocker => "eva_suit_locker" (2 x 1, tile=true);
    SciFiCargoPod => "scifi_cargo_pod" (2 x 1);
    HullDebris => "hull_debris" (2 x 1);
    ConferenceTable => "conference_table" (3 x 2);
    CopyMachine => "copy_machine" (1 x 1);
    StreetLamp => "street_lamp" (1 x 1);
    BusShelter => "bus_shelter" (3 x 1);
}

#[must_use]
fn spec_for_kind(kind: PropKind) -> Option<&'static PropSpriteSpec> {
    BATTLEMAP_PROP_SPRITE_SPECS
        .iter()
        .find(|s| s.kind == kind && !s.variant_files.is_empty())
}

#[must_use]
fn pick_variant(files: &'static [&'static str], seed: u32) -> &'static str {
    let idx = (battlemap_mix_u64(u64::from(seed)) as usize) % files.len();
    files[idx]
}

/// Resolves a single deterministic painted sprite filename for `p` when non-stretched.
#[must_use]
pub fn prop_sprite_asset_for_placement(p: &MapPropPlacement) -> Option<&'static str> {
    if p.tint_rgb.is_some() {
        return None;
    }
    let spec = spec_for_kind(p.kind)?;
    let fp = p.footprint_m();
    let actual_aspect = fp.span_w_m / fp.span_h_m.max(0.05);
    let base_aspect = spec.base_w_cells as f32 / spec.base_h_cells as f32;
    let ratio = actual_aspect / base_aspect;
    if !(0.72..=1.38).contains(&ratio) {
        return None;
    }
    Some(pick_variant(spec.variant_files, p.variant_seed))
}

/// Emits one or more (when stretched) `<image>` tags centered at `(0, 0)` across `lw × lh`.
///
/// Returns `false` when the placement should fall back to procedural SVG art.
pub fn push_prop_sprite_for_placement(
    out: &mut String,
    p: &MapPropPlacement,
    lw: f32,
    lh: f32,
) -> bool {
    if p.tint_rgb.is_some() {
        return false;
    }
    let Some(spec) = spec_for_kind(p.kind) else {
        return false;
    };
    let fp = p.footprint_m();
    let actual_aspect = fp.span_w_m / fp.span_h_m.max(0.05);
    let base_aspect = spec.base_w_cells as f32 / spec.base_h_cells as f32;
    let ratio = actual_aspect / base_aspect;
    if (0.72..=1.38).contains(&ratio) {
        let asset_file = pick_variant(spec.variant_files, p.variant_seed);
        push_prop_sprite_image_at(out, asset_file, -lw * 0.5, -lh * 0.5, lw, lh, "xMidYMid meet");
        return true;
    }
    if spec.tile_along_width && ratio > 1.38 {
        let tiles = ratio.round().clamp(2.0, 12.0) as usize;
        let seg_w = lw / tiles as f32;
        for i in 0..tiles {
            let x = -lw * 0.5 + i as f32 * seg_w;
            let asset_file = pick_variant(spec.variant_files, p.variant_seed.wrapping_add(i as u32));
            push_prop_sprite_image_at(out, asset_file, x, -lh * 0.5, seg_w, lh, "none");
        }
        return true;
    }
    false
}

/// Emits one SVG `<image>` element centered at `(0, 0)` spanning `lw × lh` SVG units.
pub fn push_prop_sprite_image(out: &mut String, asset_file: &str, lw: f32, lh: f32) {
    push_prop_sprite_image_at(out, asset_file, -lw * 0.5, -lh * 0.5, lw, lh, "xMidYMid meet");
}

fn push_prop_sprite_image_at(
    out: &mut String,
    asset_file: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    preserve: &str,
) {
    let _ = write!(
        out,
        r#"<image href="{BATTLEMAP_PROP_SPRITE_BASE_URL}/{asset_file}" x="{x:.1}" y="{y:.1}" width="{w:.1}" height="{h:.1}" preserveAspectRatio="{preserve}"/>"#,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm_client::map_2d_art_furniture::push_furniture_glyph_with_mode;
    use crate::wasm_client::map_2d_art_textures::BattlemapTextureMode;
    use satya_maps::map_dressing::make_prop;
    use std::collections::BTreeSet;
    use std::path::PathBuf;

    #[test]
    fn all_140_prop_sprite_variants_exist_and_toggle_with_texture_mode() {
        assert_eq!(BATTLEMAP_PROP_SPRITE_SPECS.len(), 140);
        let kinds: BTreeSet<u16> = BATTLEMAP_PROP_SPRITE_SPECS
            .iter()
            .map(|s| s.kind as u16)
            .collect();
        let files: BTreeSet<&str> = BATTLEMAP_PROP_SPRITE_SPECS
            .iter()
            .flat_map(|s| s.variant_files.iter().copied())
            .collect();
        assert_eq!(kinds.len(), 140);
        assert_eq!(files.len(), 420);

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/sprites/props");
        for spec in BATTLEMAP_PROP_SPRITE_SPECS {
            assert_eq!(spec.variant_files.len(), 3);
            for file in spec.variant_files {
                assert!(file.ends_with(".webp"));
                assert!(
                    root.join(file).is_file(),
                    "missing baked prop sprite {file}"
                );
            }
        }

        for kind in [
            PropKind::HospitalBed,
            PropKind::Cot,
            PropKind::Sofa,
            PropKind::Car,
            PropKind::Forklift,
            PropKind::EspressoCounter,
            PropKind::Tractor,
            PropKind::CommandConsole,
        ] {
            let picked: BTreeSet<&str> = (0..32_u32)
                .filter_map(|seed| {
                    prop_sprite_asset_for_placement(&make_prop(kind, 4, 4, 90, None, seed))
                })
                .collect();
            assert_eq!(picked.len(), 3, "expected 3 variants for {kind:?}: {picked:?}");
        }

        // Stretched modular props tile across their run.
        let counter_run = make_prop(PropKind::MedicalCounter, 4, 4, 0, Some(3), 5);
        let mut run_svg = String::new();
        push_furniture_glyph_with_mode(
            &mut run_svg,
            &counter_run,
            64.0,
            BattlemapTextureMode::Painted,
        );
        assert_eq!(run_svg.matches("<image ").count(), 3);
        assert!(run_svg.contains("medical_counter"));

        let bed = make_prop(PropKind::HospitalBed, 4, 4, 90, None, 1);
        let mut painted = String::new();
        push_furniture_glyph_with_mode(&mut painted, &bed, 64.0, BattlemapTextureMode::Painted);
        assert!(painted.contains("/assets/sprites/props/hospital_bed"));
        assert_eq!(painted.matches("<g ").count(), painted.matches("</g>").count());

        let mut vector = String::new();
        push_furniture_glyph_with_mode(
            &mut vector,
            &bed,
            64.0,
            BattlemapTextureMode::ProceduralVector,
        );
        assert!(!vector.contains("/assets/sprites/props/"));
        assert_eq!(vector.matches("<g ").count(), vector.matches("</g>").count());

        let mut tinted_bed = bed;
        tinted_bed.tint_rgb = Some([200, 120, 120]);
        assert!(prop_sprite_asset_for_placement(&tinted_bed).is_none());
    }
}
