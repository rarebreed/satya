//! Tactical Map Dressing Wire DTOs: thin **edge walls** (walls, doors, windows on cell boundaries),
//! **multi-cell props** (sofas, bar counters, beds, trees, vehicles) and their **material classes**.
//!
//! Re-exported through [`crate::tactical_map_dto`]. Mechanical behavior (prop footprints, stamped
//! `TacticalTileKind`s, and edge-aware LoS/pathing) lives in `satya-engine::{map_dressing, tactical_map_edges}`.
//!
//! Edge addressing: every cell `(x, y)` owns its **North** edge (shared with `(x, y - 1)`) and its
//! **West** edge (shared with `(x - 1, y)`). The board's South/East borders use `y = grid_height` /
//! `x = grid_width`.

use crate::define_wire_enum;
use serde::{Deserialize, Serialize};

const fn default_one_cell() -> i32 {
    1
}

const fn is_false(b: &bool) -> bool {
    !*b
}

/// Parses a `#rrggbb` or `rrggbb` hex color string into `[r, g, b]`.
#[must_use]
pub fn parse_hex_rgb(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim().strip_prefix('#').unwrap_or(raw.trim());
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([r, g, b])
}

/// Formats `[r, g, b]` into canonical lowercase `#rrggbb`.
#[must_use]
pub fn format_rgb_hex(rgb: [u8; 3]) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

define_wire_enum! {
    /// What occupies a cell-boundary edge.
    pub enum WallEdgeKind [count = 8] {
        Wall => ("WALL", "Wall"),
        DoorClosed => ("DOOR_CLOSED", "Closed Door"),
        DoorOpen => ("DOOR_OPEN", "Open Door"),
        Window => ("WINDOW", "Window (waist-high sill)"),
        FenceWood => ("FENCE_WOOD", "Wooden Fence (post-and-rail)"),
        FenceWire => ("FENCE_WIRE", "Wire / Chain-Link Fence"),
        LowWallStone => ("LOW_WALL_STONE", "Outdoor Stone / Masonry Wall"),
        StallPartition => ("STALL_PARTITION", "Restroom Stall / Privacy Partition"),
    }
}

impl WallEdgeKind {
    /// Whether this edge is a passable doorway (`DoorClosed` or `DoorOpen`).
    #[must_use]
    pub const fn is_door(self) -> bool {
        matches!(self, Self::DoorClosed | Self::DoorOpen)
    }

    /// Whether a combatant cannot step across this edge.
    #[must_use]
    pub const fn blocks_movement(self) -> bool {
        matches!(
            self,
            Self::Wall
                | Self::DoorClosed
                | Self::Window
                | Self::FenceWood
                | Self::FenceWire
                | Self::LowWallStone
                | Self::StallPartition
        )
    }

    /// Whether this edge completely blocks Line of Sight.
    #[must_use]
    pub const fn blocks_sight(self) -> bool {
        matches!(self, Self::Wall | Self::DoorClosed | Self::StallPartition)
    }

    /// Whether shooting across this edge imposes Low Cover (window sill / fence / stone wall).
    #[must_use]
    pub const fn provides_half_cover(self) -> bool {
        matches!(self, Self::Window | Self::FenceWood | Self::LowWallStone)
    }
}

define_wire_enum! {
    /// Which of a cell's two owned edges an edge record refers to.
    pub enum EdgeSide [count = 2] {
        North => ("NORTH", "North edge"),
        West => ("WEST", "West edge"),
    }
}

define_wire_enum! {
    /// Physical construction material of a wall edge or prop, graded against projectile caliber,
    /// impact energy, and penetration in `satya_maps::barrier_ballistics` (`0 PR` concealment through `10 PR` armor steel).
    #[derive(Default)]
    pub enum MaterialClass [count = 16] {
        #[default]
        Drywall => ("DRYWALL", "Drywall / stud partition"),
        Brick => ("BRICK", "Brick masonry"),
        ConcreteBlock => ("CONCRETE_BLOCK", "Concrete block (CMU)", aliases: ["CONCRETE"]),
        ReinforcedConcrete => ("REINFORCED_CONCRETE", "Reinforced concrete"),
        Glass => ("GLASS", "Glass"),
        SolidWood => ("SOLID_WOOD", "Solid hardwood", aliases: ["SOLID_OAK", "OAK", "WOOD"]),
        WoodFurniture => ("WOOD_FURNITURE", "Wood furniture / particleboard"),
        Upholstery => ("UPHOLSTERY", "Upholstered furniture"),
        SheetSteel => ("SHEET_STEEL", "Sheet steel"),
        ArmorSteel => ("ARMOR_STEEL", "Armor-grade steel plate"),
        Appliance => ("APPLIANCE", "Appliance (sheet steel + internals)"),
        Ceramic => ("CERAMIC", "Ceramic / porcelain"),
        Earth => ("EARTH", "Packed earth / soil-filled planter"),
        Foliage => ("FOLIAGE", "Foliage (concealment only)", aliases: ["NONE"]),
        LiveWood => ("LIVE_WOOD", "Living tree trunk"),
        VehicleBody => ("VEHICLE_BODY", "Vehicle body panels"),
    }
}

define_wire_enum! {
    /// Catalog of placeable continuous props & surface zones. Dimensions, heights, and materials are in
    /// `satya_maps::map_dressing::prop_spec`.
    pub enum PropKind [count = 171] {
        Sofa => ("SOFA", "Sofa (3-seat)"),
        Loveseat => ("LOVESEAT", "Loveseat"),
        Armchair => ("ARMCHAIR", "Armchair"),
        CoffeeTable => ("COFFEE_TABLE", "Coffee table"),
        Rug => ("RUG", "Area rug"),
        DiningTable => ("DINING_TABLE", "Dining table & chairs"),
        RoundTable => ("ROUND_TABLE", "Bistro table & chairs"),
        Booth => ("BOOTH", "Booth seating"),
        Bed => ("BED", "Double bed"),
        Desk => ("DESK", "Desk & chair"),
        Bookshelf => ("BOOKSHELF", "Bookshelf"),
        Wardrobe => ("WARDROBE", "Wardrobe"),
        KitchenCounter => ("KITCHEN_COUNTER", "Kitchen counter"),
        Stove => ("STOVE", "Stove / range"),
        Fridge => ("FRIDGE", "Refrigerator"),
        BarCounter => ("BAR_COUNTER", "Bar counter"),
        BackBarShelf => ("BACK_BAR_SHELF", "Back-bar bottle shelf"),
        BarStool => ("BAR_STOOL", "Bar stool"),
        PoolTable => ("POOL_TABLE", "Pool table"),
        Jukebox => ("JUKEBOX", "Jukebox"),
        Bathtub => ("BATHTUB", "Bathtub"),
        Toilet => ("TOILET", "Toilet"),
        Planter => ("PLANTER", "Planter"),
        WoodCrate => ("WOOD_CRATE", "Wooden crate"),
        DrumCluster => ("DRUM_CLUSTER", "Steel drums"),
        SteelShelving => ("STEEL_SHELVING", "Steel shelving"),
        LargeTree => ("LARGE_TREE", "Large tree"),
        Car => ("CAR", "Sedan"),
        MediumTree => ("MEDIUM_TREE", "Tree"),
        Hedge => ("HEDGE", "Trimmed hedge"),
        Bush => ("BUSH", "Shrub / bush"),
        Boulder => ("BOULDER", "Granite boulder"),
        ParkBench => ("PARK_BENCH", "Park bench"),
        PicnicTable => ("PICNIC_TABLE", "Picnic table"),
        Dumpster => ("DUMPSTER", "Commercial dumpster"),
        JerseyBarrier => ("JERSEY_BARRIER", "Concrete Jersey barrier"),
        UtilityVan => ("UTILITY_VAN", "Utility van / SUV"),
        StonePath => ("STONE_PATH", "Flagstone path"),
        AsphaltDrive => ("ASPHALT_DRIVE", "Asphalt road / drive"),
        DeskL => ("DESK_L", "L-shaped corner desk"),
        DeskT => ("DESK_T", "T-shaped workstation desk"),
        FallenLog => ("FALLEN_LOG", "Fallen log"),
        CreekWater => ("CREEK_WATER", "Creek / shallow stream"),
        DrainageDitch => ("DRAINAGE_DITCH", "Drainage ditch / trench"),
        GardenBed => ("GARDEN_BED", "Decorative garden bed"),
        GardenEdging => ("GARDEN_EDGING", "Landscape stone edging"),
        GravelPath => ("GRAVEL_PATH", "Gravel surface / path"),
        DirtPath => ("DIRT_PATH", "Packed dirt surface / trail"),
        MulchBed => ("MULCH_BED", "Bark mulch ground"),
        StructuralPillar => ("STRUCTURAL_PILLAR", "Structural bulkhead / pillar"),
        ElevatedCatwalk => ("ELEVATED_CATWALK", "Elevated gantry / catwalk"),
        HazardSpill => ("HAZARD_SPILL", "Chemical / plasma hazard spill"),
        Stairwell => ("STAIRWELL", "Stairwell"),
        DisplayRack => ("DISPLAY_RACK", "Retail display gondola"),
        CheckoutCounter => ("CHECKOUT_COUNTER", "Checkout counter"),
        HospitalBed => ("HOSPITAL_BED", "Hospital ward bed"),
        SurgicalTable => ("SURGICAL_TABLE", "Surgical operating table"),
        ImagingScanner => ("IMAGING_SCANNER", "MRI / CT diagnostic scanner"),
        PrivacyCurtain => ("PRIVACY_CURTAIN", "Medical privacy curtain"),
        MallKiosk => ("MALL_KIOSK", "Mall kiosk island"),
        ConveyorBelt => ("CONVEYOR_BELT", "Assembly conveyor belt"),
        IndustrialPress => ("INDUSTRIAL_PRESS", "Hydraulic industrial press"),
        PalletRack => ("PALLET_RACK", "High-bay pallet rack"),
        Forklift => ("FORKLIFT", "Warehouse forklift"),
        ShippingContainer => ("SHIPPING_CONTAINER", "ISO shipping container"),
        CropRow => ("CROP_ROW", "Field crop row"),
        PlanterTable => ("PLANTER_TABLE", "Conservatory planter bench"),
        DeskLamp => ("DESK_LAMP", "Desk lamp", aliases: ["TABLE_LAMP"]),
        StandingLamp => ("STANDING_LAMP", "Standing floor lamp", aliases: ["FLOOR_LAMP"]),
        Nightstand => ("NIGHTSTAND", "Bedside nightstand", aliases: ["BEDSIDE_TABLE", "END_TABLE"]),
        Dresser => ("DRESSER", "Bedroom dresser", aliases: ["CHEST_OF_DRAWERS", "BUREAU"]),
        BathroomSink => ("BATHROOM_SINK", "Bathroom vanity & sink", aliases: ["VANITY_SINK", "WASHBASIN", "SINK"]),
        FireExtinguisher => ("FIRE_EXTINGUISHER", "Fire extinguisher", aliases: ["EXTINGUISHER"]),
        TvStand => ("TV_STAND", "Entertainment stand & TV", aliases: ["TELEVISION", "ENTERTAINMENT_CENTER", "MEDIA_CONSOLE"]),
        Stereo => ("STEREO", "Stereo system & speakers", aliases: ["SOUND_SYSTEM", "SPEAKERS"]),
        RunnerRug => ("RUNNER_RUG", "Hallway / bedside runner rug", aliases: ["HALL_RUG", "BATH_MAT"]),
        RoundRug => ("ROUND_RUG", "Circular accent rug", aliases: ["CIRCLE_RUG"]),
        SmallFountain => ("SMALL_FOUNTAIN", "Small courtyard fountain", aliases: ["COURTYARD_FOUNTAIN"]),
        MediumFountain => ("MEDIUM_FOUNTAIN", "Tiered plaza fountain", aliases: ["FOUNTAIN", "PLAZA_FOUNTAIN"]),
        LargeFountain => ("LARGE_FOUNTAIN", "Grand monumental fountain", aliases: ["GRAND_FOUNTAIN"]),
        Gazebo => ("GAZEBO", "Timber garden gazebo", aliases: ["PAVILION", "PERGOLA"]),
        BbqGrill => ("BBQ_GRILL", "Outdoor barbeque grill", aliases: ["GRILL", "BARBECUE"]),
        GardenShed => ("GARDEN_SHED", "Backyard utility shed", aliases: ["SHED", "TOOL_SHED"]),
        TrashCan => ("TRASH_CAN", "Trash bin", aliases: ["GARBAGE_CAN", "WASTE_BIN"]),
        GarbagePile => ("GARBAGE_PILE", "Garbage & refuse pile", aliases: ["TRASH_BAGS", "REFUSE_PILE"]),
        AcUnit => ("AC_UNIT", "Exterior HVAC / AC condenser", aliases: ["AIR_CONDITIONER", "HVAC_CONDENSER"]),
        HotTub => ("HOT_TUB", "Outdoor spa & hot tub", aliases: ["SPA", "JACUZZI"]),
        FloorClothes => ("FLOOR_CLOTHES", "Scattered clothes on floor", aliases: ["LAUNDRY_PILE", "CLOTHES_PILE"]),
        FloorToys => ("FLOOR_TOYS", "Scattered toys on floor", aliases: ["TOYS"]),
        StreetDebris => ("STREET_DEBRIS", "Scattered street litter & debris", aliases: ["LITTER", "TRASH_SCATTER"]),
        Cot => ("COT", "Folding triage / camp cot", aliases: ["CAMP_COT", "FOLDING_COT"]),
        Gurney => ("GURNEY", "Wheeled trauma stretcher", aliases: ["STRETCHER", "MEDICAL_STRETCHER"]),
        ReclinerChair => ("RECLINER_CHAIR", "Visitor sleeper recliner", aliases: ["RECLINER", "SLEEPER_CHAIR", "VISITOR_CHAIR"]),
        MedicalCounter => ("MEDICAL_COUNTER", "Medical counter & scrub sink", aliases: ["SCRUB_SINK", "CLINIC_COUNTER"]),
        NurseStation => ("NURSE_STATION", "Corridor nursing station", aliases: ["NURSING_STATION", "CHARTING_DESK"]),
        TelemetryMonitor => ("TELEMETRY_MONITOR", "Vitals telemetry monitor & IV stand", aliases: ["VITALS_MONITOR", "ECG_MONITOR", "IV_STAND"]),
        BiohazardBin => ("BIOHAZARD_BIN", "Biohazard & soiled-linen waste bins", aliases: ["SHARPS_BIN", "MEDICAL_WASTE", "LINEN_HAMPER"]),
        LinenCart => ("LINEN_CART", "Clean linen & supply cart", aliases: ["CRASH_CART", "SUPPLY_CART"]),
        ElevatorCar => ("ELEVATOR_CAR", "Passenger elevator", aliases: ["ELEVATOR", "PASSENGER_ELEVATOR"]),
        ServiceElevator => ("SERVICE_ELEVATOR", "Hospital gurney / freight elevator", aliases: ["FREIGHT_ELEVATOR", "PATIENT_ELEVATOR"]),
        LabBench => ("LAB_BENCH", "Laboratory workbench", aliases: ["SCIENCE_BENCH", "WORKBENCH"]),
        DeskClutter => ("DESK_CLUTTER", "Scattered desk papers, mug & clutter", aliases: ["CLUTTER", "PAPERS", "TABLETOP_CLUTTER"]),
        SolderingStation => ("SOLDERING_STATION", "Electronics soldering & rework bench", aliases: ["SOLDER_BENCH", "REWORK_STATION"]),
        TestBench => ("TEST_BENCH", "Oscilloscope, wavegen & logic analyzer rig", aliases: ["OSCILLOSCOPE", "LOGIC_ANALYZER", "WAVE_GENERATOR"]),
        Printer3dStation => ("PRINTER3D_STATION", "3D printer & curing station", aliases: ["3D_PRINTER", "PRINTER_3D", "PRINTER_3D_STATION", "RESIN_PRINTER"]),
        AirCompressor => ("AIR_COMPRESSOR", "Lab / shop air compressor", aliases: ["COMPRESSOR"]),
        CncMill => ("CNC_MILL", "Enclosed CNC milling machine", aliases: ["MILLING_MACHINE", "CNC_MACHINE"]),
        ServerRack => ("SERVER_RACK", "42U compute & network server rack", aliases: ["DATA_RACK", "NETWORK_RACK"]),
        FumeHood => ("FUME_HOOD", "Chemical fume hood & biosafety cabinet", aliases: ["BIOSAFETY_CABINET", "EXTRACTION_HOOD"]),
        ShowerStall => ("SHOWER_STALL", "Walk-in glass shower stall", aliases: ["SHOWER", "GLASS_SHOWER"]),
        SpaBath => ("SPA_BATH", "Jetted whirlpool spa bath", aliases: ["JETTED_TUB", "WHIRLPOOL_TUB", "SPA_TUB"]),
        LinenCloset => ("LINEN_CLOSET", "Built-in towel & linen cabinet", aliases: ["TOWEL_CABINET", "BATH_CABINET"]),
        WasherDryer => ("WASHER_DRYER", "Laundry washer & dryer pair", aliases: ["WASHING_MACHINE", "LAUNDRY_MACHINES"]),
        SingleBed => ("SINGLE_BED", "Single / twin bed", aliases: ["TWIN_BED", "BUNK_BED"]),
        FilingCabinet => ("FILING_CABINET", "Steel office filing cabinet", aliases: ["FILE_CABINET"]),
        WaterCooler => ("WATER_COOLER", "Corridor / office water cooler", aliases: ["WATER_DISPENSER"]),
        BrownstoneStoop => ("BROWNSTONE_STOOP", "Carved front entry stoop & rails", aliases: ["STOOP", "ENTRY_STEPS"]),
        AwningCanopy => ("AWNING_CANOPY", "Storefront / market awning & lanterns", aliases: ["AWNING", "SHOP_CANOPY"]),
        CryoComputeCluster => ("CRYO_COMPUTE_CLUSTER", "Cryogenic QPU compute cluster", aliases: ["CRYO_RACK", "QPU_RACK"]),
        BioServerVat => ("BIO_SERVER_VAT", "Cryogenic seed / bio-compute vat", aliases: ["CRYO_VAT", "SEED_VAT"]),
        AirGappedTerminal => ("AIR_GAPPED_TERMINAL", "Optical-diode air-gapped terminal", aliases: ["OFFLINE_TERMINAL", "FARADAY_TERMINAL"]),
        BiometricTurnstile => ("BIOMETRIC_TURNSTILE", "Biometric gait & iris checkpoint gate", aliases: ["TURNSTILE", "SECURITY_GATE"]),
        SecurityCameraMast => ("SECURITY_CAMERA_MAST", "ALPR & acoustic surveillance sensor mast", aliases: ["CAMERA_POLE", "SENSOR_MAST"]),
        DroneDockPad => ("DRONE_DOCK_PAD", "Inductive UAV launch & recharge pad", aliases: ["UAV_PAD", "DRONE_PAD"]),
        HvTransformer => ("HV_TRANSFORMER", "500 kV high-voltage substation transformer", aliases: ["TRANSFORMER", "SUBSTATION_TRANSFORMER"]),
        DieselGenerator => ("DIESEL_GENERATOR", "Backup diesel microgrid generator", aliases: ["GENERATOR", "BACKUP_GENERATOR"]),
        BatteryBankWall => ("BATTERY_BANK_WALL", "Modular LFP microgrid battery cabinet", aliases: ["BATTERY_BANK", "POWER_WALL"]),
        WaterPurifierKiosk => ("WATER_PURIFIER_KIOSK", "Municipal ration water & misting kiosk", aliases: ["WATER_KIOSK", "MISTING_STATION"]),
        EspressoCounter => ("ESPRESSO_COUNTER", "Barista espresso & pastry display counter", aliases: ["COFFEE_BAR", "CAFE_COUNTER"]),
        CommercialPrepTable => ("COMMERCIAL_PREP_TABLE", "Stainless commercial kitchen prep island", aliases: ["PREP_TABLE", "STAINLESS_PREP"]),
        CommercialGrillHood => ("COMMERCIAL_GRILL_HOOD", "Commercial range, fryer & exhaust hood", aliases: ["FLAT_TOP_GRILL", "COMMERCIAL_RANGE"]),
        PatioUmbrellaTable => ("PATIO_UMBRELLA_TABLE", "Outdoor bistro table with parasol umbrella", aliases: ["PATIO_TABLE", "UMBRELLA_TABLE"]),
        BuffetCounter => ("BUFFET_COUNTER", "Self-serve salad, pastry & coffee bar counter", aliases: ["BUFFET", "SALAD_BAR"]),
        RefrigeratedDisplayCase => ("REFRIGERATED_DISPLAY_CASE", "Glass-door beverage & deli cooler wall", aliases: ["BEVERAGE_COOLER", "DELI_CASE"]),
        ClothingRack => ("CLOTHING_RACK", "Retail apparel garment rack", aliases: ["GARMENT_RACK", "APPAREL_RACK"]),
        PharmacyCounter => ("PHARMACY_COUNTER", "Clinical pharmacy dispensary counter", aliases: ["DISPENSARY_COUNTER", "PHARMACY_DESK"]),
        VendingMachine => ("VENDING_MACHINE", "Automated snack & beverage vending machine", aliases: ["SNACK_MACHINE", "DRINK_MACHINE"]),
        AtmKiosk => ("ATM_KIOSK", "Walk-up ATM & teller kiosk", aliases: ["ATM", "CASH_MACHINE"]),
        RoboticAssemblyArm => ("ROBOTIC_ASSEMBLY_ARM", "Articulated industrial welding robot cell", aliases: ["WELDING_ROBOT", "ROBOT_ARM"]),
        ToolChestWorkbench => ("TOOL_CHEST_WORKBENCH", "Rolling red steel tool chest & mechanic bench", aliases: ["TOOL_CHEST", "MECHANIC_BENCH"]),
        CargoPallet => ("CARGO_PALLET", "Shrink-wrapped freight cargo pallet", aliases: ["FREIGHT_PALLET", "PALLET_STACK"]),
        ElectricalSwitchgear => ("ELECTRICAL_SWITCHGEAR", "High-voltage industrial breaker switchgear cabinet", aliases: ["SWITCHGEAR", "BREAKER_CABINET"]),
        Tractor => ("TRACTOR", "Utility agricultural farm tractor", aliases: ["FARM_TRACTOR"]),
        HayBaleStack => ("HAY_BALE_STACK", "Stacked golden agricultural hay bales", aliases: ["HAY_BALES", "STRAW_BALES"]),
        LivestockTrough => ("LIVESTOCK_TROUGH", "Galvanized steel feed & water trough", aliases: ["FEED_TROUGH", "WATER_TROUGH"]),
        ConservatorySpecimenIsland => ("CONSERVATORY_SPECIMEN_ISLAND", "Lush tiered botanical specimen bed & palms", aliases: ["SPECIMEN_BED", "BOTANICAL_ISLAND"]),
        CommandConsole => ("COMMAND_CONSOLE", "Multi-screen flight & security control console", aliases: ["CONTROL_CONSOLE", "OPS_CONSOLE"]),
        EvaSuitLocker => ("EVA_SUIT_LOCKER", "Pressurized EVA suit bay & helmet locker", aliases: ["SUIT_LOCKER", "SPACE_SUIT_RACK"]),
        SciFiCargoPod => ("SCI_FI_CARGO_POD", "Sealed polymer-alloy orbital freight pod", aliases: ["CARGO_POD", "STASIS_POD", "SCIFI_CARGO_POD"]),
        HullDebris => ("HULL_DEBRIS", "Twisted bulkhead plating & severed cabling debris", aliases: ["WRECKAGE", "BULKHEAD_DEBRIS"]),
        ConferenceTable => ("CONFERENCE_TABLE", "Boardroom conference table & chairs", aliases: ["MEETING_TABLE", "BOARD_TABLE"]),
        CopyMachine => ("COPY_MACHINE", "Office multifunction printer & copier station", aliases: ["PRINTER_COPIER", "COPIER"]),
        StreetLamp => ("STREET_LAMP", "Cast-iron sidewalk streetlight standard", aliases: ["LAMP_POST", "STREETLIGHT"]),
        BusShelter => ("BUS_SHELTER", "Glazed transit bus stop shelter & bench", aliases: ["TRANSIT_SHELTER", "BUS_STOP"]),
        Urinal => ("URINAL", "Wall-mounted urinal", aliases: ["PUBLIC_URINAL"]),
        ChangingTable => ("CHANGING_TABLE", "Fold-down baby changing station", aliases: ["BABY_CHANGING_TABLE", "CHANGING_STATION"]),
        SnackVendingMachine => ("SNACK_VENDING_MACHINE", "Glass-front snack vending machine", aliases: ["SPIRAL_VENDING_MACHINE", "COIL_VENDING_MACHINE"]),
        FastFoodCounter => ("FAST_FOOD_COUNTER", "Fast-food order counter & registers", aliases: ["ORDER_COUNTER", "SERVICE_COUNTER"]),
        SodaFountain => ("SODA_FOUNTAIN", "Self-serve soda fountain & ice station", aliases: ["DRINK_STATION", "BEVERAGE_DISPENSER"]),
        DriveThruMenuBoard => ("DRIVE_THRU_MENU_BOARD", "Illuminated drive-thru menu board & speaker", aliases: ["MENU_BOARD", "DRIVE_THRU_SIGN"]),
        HostStand => ("HOST_STAND", "Restaurant host podium & menu stand", aliases: ["HOSTESS_STAND", "MAITRE_D_STAND"]),
        DeepFryerStation => ("DEEP_FRYER_STATION", "Commercial deep fryer & warming station", aliases: ["FRYER", "DEEP_FRYER"]),
        PickupTruck => ("PICKUP_TRUCK", "Utility pickup truck", aliases: ["TRUCK", "FARM_TRUCK"]),
        Horse => ("HORSE", "Horse", aliases: ["FARM_HORSE", "STALLION"]),
        Cow => ("COW", "Dairy / beef cow", aliases: ["CATTLE", "BULL"]),
        Pig => ("PIG", "Farm pig", aliases: ["HOG", "SWINE"]),
        Chicken => ("CHICKEN", "Farm chickens", aliases: ["HEN", "POULTRY"]),
        Goat => ("GOAT", "Farm goat", aliases: ["BILLY_GOAT"]),
        Dog => ("DOG", "Farm / guard dog", aliases: ["HOUND", "SHEPHERD"]),
        Silo => ("SILO", "Agricultural grain silo", aliases: ["GRAIN_SILO", "FARM_SILO"]),
        Wheelbarrow => ("WHEELBARROW", "Farm & garden wheelbarrow", aliases: ["GARDEN_CART"]),
    }
}

impl PropKind {
    /// Whether this prop provides cross-floor vertical transit (`Stairwell`, `ElevatorCar`, `ServiceElevator`).
    #[must_use]
    pub const fn is_vertical_portal(self) -> bool {
        matches!(
            self,
            Self::Stairwell | Self::ElevatorCar | Self::ServiceElevator
        )
    }

    /// Whether this prop represents tabletop clutter placed on top of desks and workbenches.
    #[must_use]
    pub const fn is_tabletop_clutter(self) -> bool {
        matches!(self, Self::DeskClutter)
    }

    /// Whether this prop renders an overhanging roof/canopy layer above walls and tokens.
    #[must_use]
    pub const fn is_overhead_canopy(self) -> bool {
        matches!(
            self,
            Self::LargeTree | Self::MediumTree | Self::Gazebo | Self::AwningCanopy
        )
    }
}

/// One wall, door, or window on a cell-boundary edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapWallEdge {
    pub x: i32,
    pub y: i32,
    pub side: EdgeSide,
    pub kind: WallEdgeKind,
    #[serde(default)]
    pub material: MaterialClass,
}

/// Address of a single cell-boundary edge (`(x, y)` + owned `side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MapEdgeRef {
    pub x: i32,
    pub y: i32,
    pub side: EdgeSide,
}

/// Precomputed continuous world-space center, canonical unrotated span/half-extents,
/// and `0..360°` clockwise rotation basis for a [`MapPropPlacement`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropFootprintM {
    pub cx_m: f32,
    pub cy_m: f32,
    pub span_w_m: f32,
    pub span_h_m: f32,
    pub hx_m: f32,
    pub hy_m: f32,
    pub rot_deg: i32,
    pub rot_rad: f32,
    pub sin_rot: f32,
    pub cos_rot: f32,
}

impl PropFootprintM {
    /// Transforms a world point `(wx_m, wy_m)` into this prop's unrotated local frame (`cx_m, cy_m = 0, 0`).
    #[must_use]
    pub fn world_to_local(self, wx_m: f32, wy_m: f32) -> (f32, f32) {
        let dx = wx_m - self.cx_m;
        let dy = wy_m - self.cy_m;
        match self.rot_deg {
            0 => (dx, dy),
            90 => (dy, -dx),
            180 => (-dx, -dy),
            270 => (-dy, dx),
            _ => (
                dx * self.cos_rot + dy * self.sin_rot,
                -dx * self.sin_rot + dy * self.cos_rot,
            ),
        }
    }

    /// Rotates a local direction vector `(lx_m, ly_m)` by `rot_deg` clockwise into world space.
    #[must_use]
    pub fn local_dir_to_world(self, lx_m: f32, ly_m: f32) -> (f32, f32) {
        match self.rot_deg {
            0 => (lx_m, ly_m),
            90 => (-ly_m, lx_m),
            180 => (-lx_m, -ly_m),
            270 => (ly_m, -lx_m),
            _ => (
                lx_m * self.cos_rot - ly_m * self.sin_rot,
                lx_m * self.sin_rot + ly_m * self.cos_rot,
            ),
        }
    }

    /// Maps a local-frame offset `(ox_m, oy_m)` into world coordinates `(wx_m, wy_m)`.
    #[must_use]
    pub fn local_offset_to_world(self, ox_m: f32, oy_m: f32) -> (f32, f32) {
        let (dwx, dwy) = self.local_dir_to_world(ox_m, oy_m);
        (self.cx_m + dwx, self.cy_m + dwy)
    }
}

/// A placed continuous prop. `(x, y)` and `(w_cells, h_cells)` represent the coarse integer AABB in
/// meter cells for quick spatial indexing and legacy consumers, while optional `center_x_cm` /
/// `center_y_cm` and canonical unrotated `span_w_cm` / `span_h_cm` store exact metric coordinates.
/// `rotation_deg` supports full `0..360°` clockwise orientation where back faces North (`-Y`) at `0°`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapPropPlacement {
    pub kind: PropKind,
    #[serde(default, alias = "origin_x")]
    pub x: i32,
    #[serde(default, alias = "origin_y")]
    pub y: i32,
    #[serde(default = "default_one_cell", alias = "span_w")]
    pub w_cells: i32,
    #[serde(default = "default_one_cell", alias = "span_h")]
    pub h_cells: i32,
    #[serde(default)]
    pub rotation_deg: i32,
    #[serde(default)]
    pub variant_seed: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_x_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub center_y_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_w_cm: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span_h_cm: Option<i32>,
    /// Optional custom tint RGB `[r, g, b]` applied in 2D/3D renderers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tint_rgb: Option<[u8; 3]>,
    /// When `true`, this prop is purely visual (never blocks movement, LoS, or provides cover).
    #[serde(default, skip_serializing_if = "is_false")]
    pub decorative: bool,
}

impl MapPropPlacement {
    /// Exact center coordinate `(cx_cm, cy_cm)` in centimeters (`100 cm = 1.00 m`).
    #[must_use]
    pub const fn center_cm(&self) -> (i32, i32) {
        let cx = match self.center_x_cm {
            Some(v) => v,
            None => self.x * 100 + self.w_cells * 50,
        };
        let cy = match self.center_y_cm {
            Some(v) => v,
            None => self.y * 100 + self.h_cells * 50,
        };
        (cx, cy)
    }

    /// Exact center coordinate `(cx_m, cy_m)` in meters.
    #[must_use]
    pub fn center_m(&self) -> (f32, f32) {
        let (cx_cm, cy_cm) = self.center_cm();
        (cx_cm as f32 / 100.0, cy_cm as f32 / 100.0)
    }

    /// Canonical unrotated dimensions `(w_cm, h_cm)` in centimeters.
    #[must_use]
    pub const fn local_span_cm(&self) -> (i32, i32) {
        match (self.span_w_cm, self.span_h_cm) {
            (Some(w), Some(h)) => (if w > 10 { w } else { 10 }, if h > 10 { h } else { 10 }),
            _ => {
                let w = if self.w_cells > 0 {
                    self.w_cells * 100
                } else {
                    100
                };
                let h = if self.h_cells > 0 {
                    self.h_cells * 100
                } else {
                    100
                };
                if self.rotation_deg.rem_euclid(180) == 90 {
                    (h, w)
                } else {
                    (w, h)
                }
            }
        }
    }

    /// Canonical unrotated dimensions `(w_m, h_m)` in meters.
    #[must_use]
    pub fn local_span_m(&self) -> (f32, f32) {
        let (w_cm, h_cm) = self.local_span_cm();
        (w_cm as f32 / 100.0, h_cm as f32 / 100.0)
    }

    /// Computes canonical center, unrotated metric span/half-extents, and rotation basis for this prop.
    #[must_use]
    pub fn footprint_m(&self) -> PropFootprintM {
        let (cx_m, cy_m) = self.center_m();
        let (span_w_m, span_h_m) = self.local_span_m();
        let rot_deg = self.rotation_deg.rem_euclid(360);
        let rot_rad = (rot_deg as f32).to_radians();
        let (sin_rot, cos_rot) = rot_rad.sin_cos();
        PropFootprintM {
            cx_m,
            cy_m,
            span_w_m,
            span_h_m,
            hx_m: span_w_m * 0.5,
            hy_m: span_h_m * 0.5,
            rot_deg,
            rot_rad,
            sin_rot,
            cos_rot,
        }
    }

    /// Whether cell `(cx, cy)` lies inside this prop's axis-aligned bounding box.
    #[must_use]
    pub const fn covers_bbox(&self, cx: i32, cy: i32) -> bool {
        cx >= self.x && cy >= self.y && cx < self.x + self.w_cells && cy < self.y + self.h_cells
    }

    /// Whether cell `(cx, cy)` is physically occupied by this prop, honoring arbitrary `0..360°`
    /// rotations as well as legacy diagonal (`45°/135°`) and asymmetric compound shapes (`DeskL`, `DeskT`).
    #[must_use]
    pub fn covers(&self, cx: i32, cy: i32) -> bool {
        if !self.covers_bbox(cx, cy) {
            return false;
        }
        let rot = self.rotation_deg.rem_euclid(360);
        if rot % 90 != 0
            && self.span_w_cm.is_none()
            && self.w_cells == self.h_cells
            && self.w_cells >= 2
        {
            let rx = cx - self.x;
            let ry = cy - self.y;
            return if rot == 45 || rot == 225 {
                rx == ry
            } else {
                rx + ry == self.w_cells - 1
            };
        }
        if rot % 90 == 0 && self.span_w_cm.is_none() {
            let rx = cx - self.x;
            let ry = cy - self.y;
            let (lx, ly, bw, _bh) = match rot {
                90 => (ry, self.w_cells - 1 - rx, self.h_cells, self.w_cells),
                180 => (
                    self.w_cells - 1 - rx,
                    self.h_cells - 1 - ry,
                    self.w_cells,
                    self.h_cells,
                ),
                270 => (self.h_cells - 1 - ry, rx, self.h_cells, self.w_cells),
                _ => (rx, ry, self.w_cells, self.h_cells),
            };
            return match self.kind {
                PropKind::DeskL => ly == 0 || lx == 0,
                PropKind::DeskT => ly == 0 || lx == bw / 2,
                _ => true,
            };
        }

        // Continuous rotated local-frame check against cell center `(cx + 0.5, cy + 0.5)`.
        let fp = self.footprint_m();
        let (lx, ly) = fp.world_to_local(cx as f32 + 0.5, cy as f32 + 0.5);
        if lx.abs() > fp.hx_m + 0.05 || ly.abs() > fp.hy_m + 0.05 {
            return false;
        }
        match self.kind {
            PropKind::DeskL => lx <= -fp.hx_m + 1.02 || ly <= -fp.hy_m + 1.02,
            PropKind::DeskT => ly <= -fp.hy_m + 1.02 || lx.abs() <= 0.52,
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn mk_prop(
        kind: PropKind,
        x: i32,
        y: i32,
        w_cells: i32,
        h_cells: i32,
        rotation_deg: i32,
    ) -> MapPropPlacement {
        MapPropPlacement {
            kind,
            x,
            y,
            w_cells,
            h_cells,
            rotation_deg,
            variant_seed: 0,
            center_x_cm: None,
            center_y_cm: None,
            span_w_cm: None,
            span_h_cm: None,
            tint_rgb: None,
            decorative: false,
        }
    }

    #[test]
    fn dressing_wire_codes_roundtrip_and_edge_semantics() {
        let edge = MapWallEdge {
            x: 3,
            y: 4,
            side: EdgeSide::West,
            kind: WallEdgeKind::Window,
            material: MaterialClass::Glass,
        };
        let json = serde_json::to_string(&edge).unwrap();
        assert!(json.contains(r#""side":"WEST""#) && json.contains(r#""kind":"WINDOW""#));
        assert_eq!(serde_json::from_str::<MapWallEdge>(&json).unwrap(), edge);
        let legacy: MapWallEdge =
            serde_json::from_str(r#"{"x":0,"y":0,"side":"NORTH","kind":"WALL"}"#).unwrap();
        assert_eq!(legacy.material, MaterialClass::Drywall);
        assert!(WallEdgeKind::Window.blocks_movement() && !WallEdgeKind::Window.blocks_sight());
        assert!(!WallEdgeKind::DoorOpen.blocks_movement());
        assert!(WallEdgeKind::FenceWood.blocks_movement() && WallEdgeKind::FenceWood.provides_half_cover() && !WallEdgeKind::FenceWood.blocks_sight());
        assert!(WallEdgeKind::FenceWire.blocks_movement() && !WallEdgeKind::FenceWire.provides_half_cover() && !WallEdgeKind::FenceWire.blocks_sight());
        assert!(WallEdgeKind::LowWallStone.blocks_movement() && WallEdgeKind::LowWallStone.provides_half_cover() && !WallEdgeKind::LowWallStone.blocks_sight());
        assert!(WallEdgeKind::StallPartition.blocks_movement() && WallEdgeKind::StallPartition.blocks_sight());
        for k in PropKind::ALL {
            assert_eq!(
                serde_json::to_string(&k).unwrap(),
                format!("\"{}\"", k.as_str())
            );
            assert_eq!(
                PropKind::from_code_str(&k.as_str().to_ascii_lowercase()),
                Some(k)
            );
        }
        assert_eq!(PropKind::ALL.len(), 171);
        assert_eq!(parse_hex_rgb("#38bdf8"), Some([0x38, 0xbd, 0xf8]));
        assert_eq!(format_rgb_hex([0x38, 0xbd, 0xf8]), "#38bdf8");
        assert!(PropKind::ElevatorCar.is_vertical_portal());
        assert!(PropKind::ServiceElevator.is_vertical_portal());
        assert!(PropKind::DeskClutter.is_tabletop_clutter());
        assert!(PropKind::AwningCanopy.is_overhead_canopy());
        assert_eq!(PropKind::from_code_str("sofa"), Some(PropKind::Sofa));
        assert_eq!(
            PropKind::from_code_str("nightstand"),
            Some(PropKind::Nightstand)
        );
        assert_eq!(
            PropKind::from_code_str("table_lamp"),
            Some(PropKind::DeskLamp)
        );
        assert_eq!(
            PropKind::from_code_str("floor_lamp"),
            Some(PropKind::StandingLamp)
        );
        assert_eq!(
            PropKind::from_code_str("vanity_sink"),
            Some(PropKind::BathroomSink)
        );
        assert_eq!(
            PropKind::from_code_str("television"),
            Some(PropKind::TvStand)
        );
        assert_eq!(
            PropKind::from_code_str("fountain"),
            Some(PropKind::MediumFountain)
        );
        assert_eq!(PropKind::from_code_str("gazebo"), Some(PropKind::Gazebo));
        assert_eq!(PropKind::from_code_str("hot_tub"), Some(PropKind::HotTub));
        assert_eq!(
            PropKind::from_code_str("street_debris"),
            Some(PropKind::StreetDebris)
        );
        assert_eq!(
            PropKind::from_code_str("structural_pillar"),
            Some(PropKind::StructuralPillar)
        );
        assert_eq!(
            PropKind::from_code_str("elevated_catwalk"),
            Some(PropKind::ElevatedCatwalk)
        );
        assert_eq!(
            PropKind::from_code_str("hazard_spill"),
            Some(PropKind::HazardSpill)
        );
        assert_eq!(
            PropKind::from_code_str("hospital_bed"),
            Some(PropKind::HospitalBed)
        );
        assert_eq!(
            PropKind::from_code_str("shipping_container"),
            Some(PropKind::ShippingContainer)
        );
        assert_eq!(PropKind::from_code_str("crop_row"), Some(PropKind::CropRow));
        assert_eq!(
            PropKind::from_code_str("planter_table"),
            Some(PropKind::PlanterTable)
        );
        assert_eq!(PropKind::from_code_str("nonexistent_prop"), None);
        let p = mk_prop(PropKind::BarCounter, 2, 1, 5, 1, 0);
        assert!(p.covers(6, 1) && !p.covers(7, 1) && !p.covers(2, 2));
        assert_eq!(p.center_cm(), (450, 150));
        assert_eq!(p.local_span_cm(), (500, 100));
    }

    #[test]
    fn compound_l_t_desks_and_diagonal_props_cover_only_shape_cells() {
        // DeskL 2x2: at 0° SE corner (1,1) is open; at 90° SW (0,1); at 180° NW (0,0); at 270° NE (1,0)
        for (rot, open_xy) in [(0, (1, 1)), (90, (0, 1)), (180, (0, 0)), (270, (1, 0))] {
            let l_desk = mk_prop(PropKind::DeskL, 0, 0, 2, 2, rot);
            let covered: Vec<(i32, i32)> = (0..2)
                .flat_map(|y| (0..2).map(move |x| (x, y)))
                .filter(|&(x, y)| l_desk.covers(x, y))
                .collect();
            assert_eq!(covered.len(), 3, "rot={rot}");
            assert!(
                !l_desk.covers(open_xy.0, open_xy.1),
                "rot={rot} open={open_xy:?}"
            );
        }

        // DeskT 3x2: at 0° (0,1) & (2,1) open; at 90° (2x3) (0,0) & (0,2) open
        let t0 = mk_prop(PropKind::DeskT, 0, 0, 3, 2, 0);
        assert!(t0.covers(0, 0) && t0.covers(1, 0) && t0.covers(2, 0) && t0.covers(1, 1));
        assert!(!t0.covers(0, 1) && !t0.covers(2, 1));

        let t90 = mk_prop(PropKind::DeskT, 0, 0, 2, 3, 90);
        assert!(t90.covers(1, 0) && t90.covers(1, 1) && t90.covers(1, 2) && t90.covers(0, 1));
        assert!(!t90.covers(0, 0) && !t90.covers(0, 2));

        let t180 = mk_prop(PropKind::DeskT, 0, 0, 3, 2, 180);
        assert!(t180.covers(0, 1) && t180.covers(1, 1) && t180.covers(2, 1) && t180.covers(1, 0));
        assert!(!t180.covers(0, 0) && !t180.covers(2, 0));

        let t270 = mk_prop(PropKind::DeskT, 0, 0, 2, 3, 270);
        assert!(t270.covers(0, 0) && t270.covers(0, 1) && t270.covers(0, 2) && t270.covers(1, 1));
        assert!(!t270.covers(1, 0) && !t270.covers(1, 2));

        // FallenLog 3x3 at 45° covers main diagonal; at 135° covers anti-diagonal
        let log45 = mk_prop(PropKind::FallenLog, 1, 1, 3, 3, 45);
        assert!(log45.covers(1, 1) && log45.covers(2, 2) && log45.covers(3, 3));
        assert!(!log45.covers(3, 1) && !log45.covers(1, 3) && log45.covers_bbox(3, 1));

        let log135 = mk_prop(PropKind::FallenLog, 1, 1, 3, 3, 135);
        assert!(log135.covers(3, 1) && log135.covers(2, 2) && log135.covers(1, 3));
        assert!(!log135.covers(1, 1) && !log135.covers(3, 3));

        // Free 37° continuous rotated sofa preserves unrotated canonical span
        let free_sofa = MapPropPlacement {
            kind: PropKind::Sofa,
            x: 3,
            y: 3,
            w_cells: 4,
            h_cells: 4,
            rotation_deg: 37,
            variant_seed: 9,
            center_x_cm: Some(525),
            center_y_cm: Some(475),
            span_w_cm: Some(300),
            span_h_cm: Some(100),
            tint_rgb: Some([120, 180, 220]),
            decorative: true,
        };
        assert_eq!(free_sofa.center_cm(), (525, 475));
        assert_eq!(free_sofa.local_span_cm(), (300, 100));
        assert!(free_sofa.decorative);
        assert_eq!(free_sofa.tint_rgb, Some([120, 180, 220]));
    }

    #[test]
    fn prop_footprint_roundtrip_and_python_json_aliases() {
        for rot in [0, 37, 90, 215] {
            let p = MapPropPlacement {
                kind: PropKind::Sofa,
                x: 3,
                y: 2,
                w_cells: 3,
                h_cells: 2,
                rotation_deg: rot,
                variant_seed: 0,
                center_x_cm: Some(450),
                center_y_cm: Some(325),
                span_w_cm: Some(260),
                span_h_cm: Some(110),
                tint_rgb: None,
                decorative: false,
            };
            let fp = p.footprint_m();
            let (wx, wy) = fp.local_offset_to_world(0.85, -0.35);
            let (lx, ly) = fp.world_to_local(wx, wy);
            assert!(
                (lx - 0.85).abs() < 1e-4 && (ly - (-0.35)).abs() < 1e-4,
                "rot={rot}: ({lx}, {ly})"
            );
        }

        let py_json = r#"{
            "kind": "JERSEY_BARRIER",
            "origin_x": 6,
            "origin_y": 4,
            "span_w": 2,
            "span_h": 1,
            "center_x_cm": 700,
            "center_y_cm": 475,
            "span_w_cm": 140,
            "span_h_cm": 80,
            "rotation_deg": 15
        }"#;
        let parsed: MapPropPlacement = serde_json::from_str(py_json).unwrap();
        assert_eq!(parsed.kind, PropKind::JerseyBarrier);
        assert_eq!(
            (parsed.x, parsed.y, parsed.w_cells, parsed.h_cells),
            (6, 4, 2, 1)
        );
        assert_eq!(parsed.center_cm(), (700, 475));
        assert_eq!(parsed.local_span_cm(), (140, 80));
        assert_eq!(
            MaterialClass::from_code_str("SOLID_OAK"),
            Some(MaterialClass::SolidWood)
        );
    }
}
