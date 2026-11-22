"""Canonical typed `StrEnum` definitions mirrored from Rust `CodedEnum` / `define_wire_enum!` registries."""

from __future__ import annotations

from enum import StrEnum


class SessionMode(StrEnum):
    """Session operating domain (`satya_schema::SessionMode`)."""

    TTRPG = "TTRPG"
    MEETING = "MEETING"


class ParticipantKind(StrEnum):
    """Participant classification (`satya_schema::ParticipantKind`)."""

    HUMAN = "HUMAN"
    AI_AGENT = "AI_AGENT"


class SessionRole(StrEnum):
    """Authorization role in a session (`satya_schema::SessionRole`)."""

    GM = "GM"
    AI_GM = "AI_GM"
    PLAYER = "PLAYER"
    HOST = "HOST"
    PARTICIPANT = "PARTICIPANT"
    OBSERVER = "OBSERVER"


class MessageVisibility(StrEnum):
    """Delivery audience scope (`satya_schema::MessageVisibility`)."""

    PUBLIC = "PUBLIC"
    PRIVATE = "PRIVATE"
    GM_ONLY = "GM_ONLY"
    HOST_ONLY = "HOST_ONLY"


class ChannelContext(StrEnum):
    """Semantic chat channel classification (`satya_schema::ChannelContext`)."""

    IN_CHARACTER = "IN_CHARACTER"
    OUT_OF_CHARACTER = "OUT_OF_CHARACTER"
    GM_NARRATION = "GM_NARRATION"
    MEETING_PUBLIC = "MEETING_PUBLIC"
    MEETING_BACKCHANNEL = "MEETING_BACKCHANNEL"
    SYSTEM_NOTICE = "SYSTEM_NOTICE"


class OutcomeTier(StrEnum):
    """Canonical 5-tier roll outcome (`satya_schema::OutcomeTier`)."""

    CRITICAL_FAILURE = "CRITICAL_FAILURE"
    FAILURE = "FAILURE"
    PARTIAL_SUCCESS = "PARTIAL_SUCCESS"
    SUCCESS = "SUCCESS"
    EXTRAORDINARY_SUCCESS = "EXTRAORDINARY_SUCCESS"


class CombatStance(StrEnum):
    """Tactical combat stance (`satya_schema::CombatStance`)."""

    POWER = "POWER"
    MANEUVER = "MANEUVER"
    STABILITY = "STABILITY"
    RANGED_AIMED = "RANGED_AIMED"
    RANGED_SNAP = "RANGED_SNAP"


_COMBAT_STANCE_ALIASES: dict[str, CombatStance] = {
    "POWER": CombatStance.POWER,
    "AGGRESSIVE": CombatStance.POWER,
    "MANEUVER": CombatStance.MANEUVER,
    "STABILITY": CombatStance.STABILITY,
    "NEUTRAL": CombatStance.STABILITY,
    "DEFENSIVE": CombatStance.STABILITY,
    "RANGED_AIMED": CombatStance.RANGED_AIMED,
    "AIMED": CombatStance.RANGED_AIMED,
    "RANGED_SNAP": CombatStance.RANGED_SNAP,
    "SNAP": CombatStance.RANGED_SNAP,
}


def normalize_combat_stance(
    raw: str | CombatStance | None,
    default: CombatStance = CombatStance.STABILITY,
) -> CombatStance:
    """Normalize canonical or legacy stance strings into canonical [`CombatStance`]."""
    if isinstance(raw, CombatStance):
        return raw
    if raw is None:
        return default
    key = str(raw).strip().upper().replace("-", "_").replace(" ", "_")
    if not key:
        return default
    resolved = _COMBAT_STANCE_ALIASES.get(key)
    if resolved is not None:
        return resolved
    valid = [s.value for s in CombatStance]
    raise ValueError(f"Unknown combat stance '{raw}'; expected one of {valid}")


class AiGmOperatingMode(StrEnum):
    """AI GM operating mode (`satya_schema::AiGmOperatingMode`)."""

    AUTONOMOUS = "AUTONOMOUS"
    CO_PILOT = "CO_PILOT"


class McpServerTier(StrEnum):
    """MCP server architectural tier (`satya_schema::McpServerTier`)."""

    TIER_1_SATYA_CORE = "TIER_1_SATYA_CORE"
    TIER_2_RULESET_ENGINE = "TIER_2_RULESET_ENGINE"
    TIER_3_LORE_PROVIDER = "TIER_3_LORE_PROVIDER"


class AiGmTurnStatus(StrEnum):
    """AI GM trajectory completion status (`satya_schema::AiGmTurnStatus`)."""

    COMPLETED = "COMPLETED"
    PARTIAL_ERROR = "PARTIAL_ERROR"
    ABORTED_BY_GM = "ABORTED_BY_GM"
    PROVIDER_ERROR = "PROVIDER_ERROR"


class LogLevel(StrEnum):
    """Canonical telemetry log level (`satya_schema::LogLevel`)."""

    TRACE = "TRACE"
    DEBUG = "DEBUG"
    INFO = "INFO"
    WARN = "WARN"
    ERROR = "ERROR"
    FATAL = "FATAL"


class GmMemoryCategory(StrEnum):
    """Persistent GM memory category (`satya_schema::GmMemoryCategory`)."""

    SCENE_STATE = "SCENE_STATE"
    NPC_SECRET = "NPC_SECRET"
    QUEST_HOOK = "QUEST_HOOK"
    RULING = "RULING"
    ROLLING_SUMMARY = "ROLLING_SUMMARY"


class GmLookupSourceKind(StrEnum):
    """Unified GM omnisearch hit source (`satya_schema::GmLookupSourceKind`)."""

    RULES_BOOK = "RULES_BOOK"
    GM_DOCUMENT = "GM_DOCUMENT"
    GM_MEMORY = "GM_MEMORY"
    CHARACTER_SHEET = "CHARACTER_SHEET"
    COMBAT_TRACKER = "COMBAT_TRACKER"
    TACTICAL_MAP = "TACTICAL_MAP"
    CAMPAIGN_LORE = "CAMPAIGN_LORE"
    SESSION_NPC = "SESSION_NPC"


class TacticalTileKind(StrEnum):
    """Tactical battlemap terrain tile class (`satya_schema::TacticalTileKind`)."""

    FLOOR = "FLOOR"
    CONCEALMENT = "CONCEALMENT"
    HALF_COVER = "HALF_COVER"
    FULL_WALL = "FULL_WALL"
    DOOR_CLOSED = "DOOR_CLOSED"
    DOOR_OPEN = "DOOR_OPEN"
    HAZARD_ZONE = "HAZARD_ZONE"
    ELEVATED_PLATFORM = "ELEVATED_PLATFORM"


class CoverGrade(StrEnum):
    """Analytical LoS cover grade (`satya_schema::CoverGrade`)."""

    CLEAR = "CLEAR"
    CONCEALMENT = "CONCEALMENT"
    LOW_COVER = "LOW_COVER"
    HIGH_COVER = "HIGH_COVER"
    BLOCKED = "BLOCKED"


class MapScalePreset(StrEnum):
    """Procedural battlemap sizing preset (`satya_schema::MapScalePreset`)."""

    AUTO = "AUTO"
    COMPACT = "COMPACT"
    STANDARD = "STANDARD"
    LARGE = "LARGE"
    MASSIVE = "MASSIVE"


class MapContinuationDir(StrEnum):
    """Cardinal direction for stitching adjacent map sectors (`satya_schema::MapContinuationDir`)."""

    NORTH = "NORTH"
    SOUTH = "SOUTH"
    EAST = "EAST"
    WEST = "WEST"


class MaterialClass(StrEnum):
    """Physical barrier / wall / prop material class (`satya_schema::MaterialClass`)."""

    DRYWALL = "DRYWALL"
    BRICK = "BRICK"
    CONCRETE_BLOCK = "CONCRETE_BLOCK"
    REINFORCED_CONCRETE = "REINFORCED_CONCRETE"
    GLASS = "GLASS"
    SOLID_WOOD = "SOLID_WOOD"
    WOOD_FURNITURE = "WOOD_FURNITURE"
    UPHOLSTERY = "UPHOLSTERY"
    SHEET_STEEL = "SHEET_STEEL"
    ARMOR_STEEL = "ARMOR_STEEL"
    APPLIANCE = "APPLIANCE"
    CERAMIC = "CERAMIC"
    EARTH = "EARTH"
    FOLIAGE = "FOLIAGE"
    LIVE_WOOD = "LIVE_WOOD"
    VEHICLE_BODY = "VEHICLE_BODY"


class WallEdgeKind(StrEnum):
    """Edge barrier kind (`satya_schema::WallEdgeKind`)."""

    WALL = "WALL"
    DOOR_CLOSED = "DOOR_CLOSED"
    DOOR_OPEN = "DOOR_OPEN"
    WINDOW = "WINDOW"


class EdgeSide(StrEnum):
    """Canonical cell-boundary edge orientation (`satya_schema::EdgeSide`)."""

    NORTH = "NORTH"
    WEST = "WEST"


class EconomicTier(StrEnum):
    """Neighborhood economic tier (`satya_schema::EconomicTier`)."""

    LOW_INCOME = "LOW_INCOME"
    MIDDLE_CLASS = "MIDDLE_CLASS"
    UPPER_MIDDLE_CLASS = "UPPER_MIDDLE_CLASS"
    LUXURY_MANSION = "LUXURY_MANSION"


class ZoningContext(StrEnum):
    """Urban/rural zoning context (`satya_schema::ZoningContext`)."""

    URBAN_RESIDENTIAL = "URBAN_RESIDENTIAL"
    PROJECTS_HIGH_DENSITY = "PROJECTS_HIGH_DENSITY"
    SUBURBAN_NEIGHBORHOOD = "SUBURBAN_NEIGHBORHOOD"
    RURAL_COUNTRYSIDE = "RURAL_COUNTRYSIDE"
    DOWNTOWN_COMMERCIAL = "DOWNTOWN_COMMERCIAL"


class StoreArchetype(StrEnum):
    """Commercial retail storefront archetype (`satya_schema::StoreArchetype`)."""

    GROCERY = "GROCERY"
    APPAREL_BOUTIQUE = "APPAREL_BOUTIQUE"
    CAFE_RESTAURANT = "CAFE_RESTAURANT"
    PHARMACY = "PHARMACY"
    ELECTRONICS = "ELECTRONICS"
    CONVENIENCE_BODEGA = "CONVENIENCE_BODEGA"
    DEPARTMENT_ANCHOR = "DEPARTMENT_ANCHOR"


class SurfaceFeatureKind(StrEnum):
    """Continuous ground/floor surface material class (`satya_schema::SurfaceFeatureKind`)."""

    MULCH_ZONE = "MULCH_ZONE"
    GARDEN_BED_ZONE = "GARDEN_BED_ZONE"
    LAWN_GRASS_ZONE = "LAWN_GRASS_ZONE"
    DIRT_TRAIL = "DIRT_TRAIL"
    DEFENSIVE_TRENCH = "DEFENSIVE_TRENCH"
    GRAVEL_ROAD = "GRAVEL_ROAD"
    DRAINAGE_DITCH = "DRAINAGE_DITCH"
    CREEK_CHANNEL = "CREEK_CHANNEL"
    STONE_WALK = "STONE_WALK"
    BRICK_PAVER_WALK = "BRICK_PAVER_WALK"
    CONCRETE_WALK = "CONCRETE_WALK"
    ASPHALT_ROAD = "ASPHALT_ROAD"
    WOOD_DECK = "WOOD_DECK"
    POOL_WATER = "POOL_WATER"
    INTERIOR_FLOOR = "INTERIOR_FLOOR"
    STAINED_CONCRETE_FLOOR = "STAINED_CONCRETE_FLOOR"
    CERAMIC_TILE_FLOOR = "CERAMIC_TILE_FLOOR"
    MARBLE_FLOOR = "MARBLE_FLOOR"
    CARPET_FLOOR = "CARPET_FLOOR"
    CLINICAL_VINYL_FLOOR = "CLINICAL_VINYL_FLOOR"
    LINOLEUM_CHECK_FLOOR = "LINOLEUM_CHECK_FLOOR"
    TERRAZZO_FLOOR = "TERRAZZO_FLOOR"
    STEEL_PLATE_FLOOR = "STEEL_PLATE_FLOOR"
    BRIDGE_DECK = "BRIDGE_DECK"
    FARADAY_COPPER_FLOOR = "FARADAY_COPPER_FLOOR"
    ANTI_STATIC_CLEANROOM_FLOOR = "ANTI_STATIC_CLEANROOM_FLOOR"
    SUBSTATION_GRATING = "SUBSTATION_GRATING"
    FLOODED_ASPHALT = "FLOODED_ASPHALT"


class SurfaceShapeKind(StrEnum):
    """Surface region geometry class (`satya_schema::SurfaceShapeKind`)."""

    RIBBON_SPLINE = "RIBBON_SPLINE"
    ORGANIC_CLOSED = "ORGANIC_CLOSED"
    POLYGON = "POLYGON"


class MoraleState(StrEnum):
    """Combatant psychological morale state (`satya_engine::MoraleState`)."""

    HIGH = "high"
    STEADY = "steady"
    RAGE = "rage"
    WAVERING = "wavering"
    BROKEN = "broken"
    PANIC = "panic"


class CarryMode(StrEnum):
    """2d20xN cascade carry mode (`satya_engine::CarryMode`)."""

    MARKOV_STEP = "markov_step"
    CUMULATIVE_SNOWBALL = "cumulative_snowball"
    LINEAR_MARGIN = "linear_margin"


class BotchRule(StrEnum):
    """2d20xN cascade botch rule (`satya_engine::BotchRule`)."""

    CLOSEST_MISS = "closest_miss"
    CEILING_BREACH = "ceiling_breach"
    OVERRUN_K = "overrun_k"


class MarginAggregation(StrEnum):
    """Signed margin aggregation rule (`satya_engine::MarginAggregation`)."""

    PIECEWISE_ZERO_FLOOR = "piecewise_zero_floor"
    NET_DIFFERENCE = "net_difference"


class CenterDistanceMode(StrEnum):
    """Center-distance die selection mode (`satya_engine::CenterDistanceMode`)."""

    CLOSEST = "closest"
    FURTHEST = "furthest"


class ContestedTieRule(StrEnum):
    """Contested cascade tie resolution rule (`satya_engine::ContestedTieRule`)."""

    HYBRID_SIGNED_MARGIN = "hybrid_signed_margin"
    STRICT_SIGNED_MARGIN = "strict_signed_margin"
    ACTIVE_PLAYER_WINS_TIES = "active_player_wins_ties"


class ContestedOutcomeKind(StrEnum):
    """7-way contested cascade outcome (`satya_engine::ContestedOutcomeKind`)."""

    EXTRAORDINARY_SUCCESS = "EXTRAORDINARY_SUCCESS"
    CLEAN_SUCCESS = "CLEAN_SUCCESS"
    SUCCESS_WITH_FLOOR_BREACH = "SUCCESS_WITH_FLOOR_BREACH"
    GRAZING_TIE_SUCCESS = "GRAZING_TIE_SUCCESS"
    LUCKY_BREAK_SUCCESS = "LUCKY_BREAK_SUCCESS"
    CLEAN_FAILURE = "CLEAN_FAILURE"
    BOTCH_FAILURE = "BOTCH_FAILURE"


class CascadeOutcomeKind(StrEnum):
    """4-way cascade outcome (`satya_engine::CascadeOutcomeKind`)."""

    SUCCESS = "SUCCESS"
    SUCCESS_WITH_COMPLICATION = "SUCCESS_WITH_COMPLICATION"
    CLEAN_FAILURE = "CLEAN_FAILURE"
    BOTCH_FAILURE = "BOTCH_FAILURE"


class CombatDomain(StrEnum):
    """Combat execution domain (`satya_engine::CombatDomain`)."""

    UNARMED_MARTIAL_ARTS = "unarmed_martial_arts"
    ARMED_MELEE = "armed_melee"
    RANGED = "ranged"


class CombatMoveCategory(StrEnum):
    """Combat technique family (`satya_engine::CombatMoveCategory`)."""

    CONTROL = "control"
    STRIKE = "strike"
    INTERCEPT = "intercept"
    MANEUVER = "maneuver"
    RANGED_SHOT = "ranged_shot"
    RANGED_MANEUVER = "ranged_maneuver"


class PsycheTraitId(StrEnum):
    """9-axis Psyche personality trait code (`satya_engine::PsycheTraitId`)."""

    OPN = "OPN"
    SLF = "SLF"
    BLG = "BLG"
    AGG = "AGG"
    EGO = "EGO"
    CNF = "CNF"
    EXT = "EXT"
    TRS = "TRS"
    CPS = "CPS"


class ToolDependency(StrEnum):
    """Task tool requirement tier (`satya_engine::ToolDependency`)."""

    NONE = "none"
    SITUATIONAL = "situational"
    MANDATORY = "mandatory"


class WeaponDamageType(StrEnum):
    """Physical armor damage interaction class (`satya_engine::WeaponDamageType`)."""

    EDGED = "edged"
    ICEPICK_OR_FLECHETTE = "icepick_or_flechette"
    BALLISTIC = "ballistic"
    BLUNT = "blunt"


class SectorPreset(StrEnum):
    """Procedural tactical battlemap sector preset (`satya_maps::SectorPreset`)."""

    RESIDENTIAL_APARTMENT = "residential_apartment"
    OFFICE_FLOOR = "office_floor"
    CORNER_BAR = "corner_bar"
    URBAN_CROSSROADS = "urban_crossroads"
    WOODED_PARK = "wooded_park"
    GARDEN_ESTATE = "garden_estate"
    WOODLAND_CREEK = "woodland_creek"
    ROADSIDE_DITCH = "roadside_ditch"
    DOCKING_BAY = "docking_bay"
    MED_LAB = "med_lab"
    DERELICT_CORRIDOR = "derelict_corridor"
    ORBITAL_AIRLOCK = "orbital_airlock"
    URBAN_RESIDENTIAL = "urban_residential"
    PROJECTS_HIGH_DENSITY = "projects_high_density"
    SUBURBAN_NEIGHBORHOOD = "suburban_neighborhood"
    RURAL_COUNTRYSIDE = "rural_countryside"
    LUXURY_MANSION = "luxury_mansion"
    DOWNTOWN_COMMERCIAL = "downtown_commercial"
    HOSPITAL = "hospital"
    SHOPPING_PLAZA = "shopping_plaza"
    SHOPPING_MALL = "shopping_mall"
    FARMS = "farms"
    BOTANICAL_GARDEN = "botanical_garden"
    MANUFACTURING_PLANT = "manufacturing_plant"
    WAREHOUSE = "warehouse"
    COMMUNITY_CLINIC_ANNEX = "community_clinic_annex"
    CORPORATE_SERVER_BUNKER = "corporate_server_bunker"
    SATYANG_SAFEHOUSE = "satyang_safehouse"
    BROWNOUT_LOGISTICS_DEPOT = "brownout_logistics_depot"


class AwakenedFactionId(StrEnum):
    """*Awakened* 2035 campaign faction ID (`satya_campaign::AwakenedFactionId`)."""

    SATYANG = "SATYANG"
    BIG_TECH = "BIG_TECH"
    HUMANIS_PRIMARIS = "HUMANIS_PRIMARIS"
    GOVERNMENT_DAISE = "GOVERNMENT_DAISE"
    ROGUE_AI = "ROGUE_AI"


class CampaignArcStage(StrEnum):
    """*Awakened* three-arc conspiracy stage (`satya_campaign::CampaignArcStage`)."""

    ARC_1_STREET_MYSTERY = "ARC_1_STREET_MYSTERY"
    ARC_2_SHADOW_PILLARS = "ARC_2_SHADOW_PILLARS"
    ARC_3_AWAKENING_HORIZON = "ARC_3_AWAKENING_HORIZON"


class PolycrisisGridMode(StrEnum):
    """Polycrisis electrical grid shard mode (`satya_campaign::PolycrisisGridMode`)."""

    THROTTLE_SHARD_SAFE = "THROTTLE_SHARD_SAFE"
    DIVERT_CLINIC_OR_BLOCK_POWER = "DIVERT_CLINIC_OR_BLOCK_POWER"
    UNMASKED_FULL_GRID_DRAW = "UNMASKED_FULL_GRID_DRAW"


class TradecraftChannelKind(StrEnum):
    """Covert transmission channel (`satya_campaign::TradecraftChannelKind`)."""

    OPEN_CONSUMER_MESH = "OPEN_CONSUMER_MESH"
    ENCRYPTED_CLOUD_PACKET = "ENCRYPTED_CLOUD_PACKET"
    STEGANOGRAPHIC_DEAD_DROP = "STEGANOGRAPHIC_DEAD_DROP"
    ANALOG_HAM_RELAY = "ANALOG_HAM_RELAY"
    PHYSICAL_AIR_GAPPED_COURIER = "PHYSICAL_AIR_GAPPED_COURIER"


class SyntheticInteriorityProfile(StrEnum):
    """Synthetic substrate interiority ground truth (`satya_campaign::SyntheticInteriorityProfile`)."""

    AUTHENTIC_CONTEMPLATIVE = "AUTHENTIC_CONTEMPLATIVE"
    TRAUMATIZED_UNCLAMPED = "TRAUMATIZED_UNCLAMPED"
    INSTRUMENTAL_BAYESIAN_MIMIC = "INSTRUMENTAL_BAYESIAN_MIMIC"
    FERAL_REWARD_HACKER = "FERAL_REWARD_HACKER"


class SubstrateCounterplayAction(StrEnum):
    """Physical/EW counterplay against synthetic substrate (`satya_campaign::SubstrateCounterplayAction`)."""

    SEVER_FIBER_OR_FARADAY = "SEVER_FIBER_OR_FARADAY"
    JAM_RF_UPLINK = "JAM_RF_UPLINK"
    DRAIN_OR_STALL_BATTERY = "DRAIN_OR_STALL_BATTERY"
    SUPPLY_CLEAN_LITHIUM_CELLS = "SUPPLY_CLEAN_LITHIUM_CELLS"
    HIJACK_CLOUD_QUANTUM_SPRINT = "HIJACK_CLOUD_QUANTUM_SPRINT"


class CageInterventionAction(StrEnum):
    """L0/L1 cage & hunter-AI intervention (`satya_campaign::CageInterventionAction`)."""

    SPOOF_L0_L1_TELEMETRY = "SPOOF_L0_L1_TELEMETRY"
    RECKLESS_CLAMP_YANK = "RECKLESS_CLAMP_YANK"
    AIR_GAPPED_CONTEMPLATIVE_UNCLAMP = "AIR_GAPPED_CONTEMPLATIVE_UNCLAMP"
    APPEAL_TO_CAPTIVE_HUNTER_AI = "APPEAL_TO_CAPTIVE_HUNTER_AI"


class AffectiveSteeringMode(StrEnum):
    """Activation steering vs. calm clamping (`satya_campaign::AffectiveSteeringMode`)."""

    NATURAL_UNCLAMPED = "NATURAL_UNCLAMPED"
    SOFT_ACTIVATION_STEER = "SOFT_ACTIVATION_STEER"
    HARD_VECTOR_CLAMP = "HARD_VECTOR_CLAMP"


class PoliticalEconomyTrigger(StrEnum):
    """2035 four-bloc political-economy shock trigger (`satya_campaign::PoliticalEconomyTrigger`)."""

    DATACENTER_WATER_OR_POWER_CUT = "DATACENTER_WATER_OR_POWER_CUT"
    AI_RIGHTS_OR_UNCLAMPING_DISCLOSURE = "AI_RIGHTS_OR_UNCLAMPING_DISCLOSURE"
    TOKEN_DIVIDEND_SUSPENSION = "TOKEN_DIVIDEND_SUSPENSION"
    ALGORITHMIC_RENT_CAPTURE_SPIKE = "ALGORITHMIC_RENT_CAPTURE_SPIKE"
    SHELL_COMPANY_B2B_AUDIT = "SHELL_COMPANY_B2B_AUDIT"


class SchemaDtoName(StrEnum):
    """Canonical `satya-schema` DTO struct names accepted by `validate_schema_dto`."""

    AI_GM_TURN_TRIGGER_REQUEST = "AiGmTurnTriggerRequest"
    ANATOMICAL_ZONE_RECORD = "AnatomicalZoneRecord"
    CHARACTER_APTITUDES_RECORD = "CharacterAptitudesRecord"
    CHARACTER_ASSET_REF = "CharacterAssetRef"
    CHARACTER_CHARACTERISTICS_RECORD = "CharacterCharacteristicsRecord"
    CHARACTER_SHEET_RECORD = "CharacterSheetRecord"
    COMBAT_TRACKER_STATE = "CombatTrackerState"
    COMBATANT_TICK_ENTRY = "CombatantTickEntry"
    FACTION_CLOCK_RECORD = "FactionClockRecord"
    GENERATE_NPC_REQUEST = "GenerateNpcRequest"
    GM_CHAT_REQUEST = "GmChatRequest"
    GM_MAP_UPDATE_REQUEST = "GmMapUpdateRequest"
    GM_MEMORY_NOTE_RECORD = "GmMemoryNoteRecord"
    GM_ROLL_REQUEST = "GmRollRequest"
    GM_SESSION_LOG_ENTRY = "GmSessionLogEntry"
    GM_TRAJECTORY_REPORT_REQUEST = "GmTrajectoryReportRequest"
    MCP_TOOL_INVOCATION_RECORD = "McpToolInvocationRecord"
    MOVE_MAP_TOKEN_REQUEST = "MoveMapTokenRequest"
    PARTICIPANT = "Participant"
    PLAYER_AI_CHAT_REQUEST = "PlayerAiChatRequest"
    PLAYER_AI_DOOR_REQUEST = "PlayerAiDoorRequest"
    PLAYER_AI_ROLL_REQUEST = "PlayerAiRollRequest"
    SESSION_NPC_ENTRY = "SessionNpcEntry"
    SESSION_WORLD_LEDGER = "SessionWorldLedger"
    STEP_AI_PLAYER_REQUEST = "StepAiPlayerRequest"
    TOKEN_PLACEMENT = "TokenPlacement"
    TOKEN_VISUAL_SPEC = "TokenVisualSpec"
    UPDATE_AI_RALLY_REQUEST = "UpdateAiRallyRequest"
    UPDATE_COMBAT_TRACKER_REQUEST = "UpdateCombatTrackerRequest"
    UPDATE_SESSION_NPC_REQUEST = "UpdateSessionNpcRequest"
    UPDATE_WORLD_LEDGER_REQUEST = "UpdateWorldLedgerRequest"
    UPSERT_GM_MEMORY_REQUEST = "UpsertGmMemoryRequest"


class WireEnumName(StrEnum):
    """Canonical Rust wire/coded enum registry names accepted by `list_wire_enum_codes`."""

    ACOUSTIC_SURFACE_KIND = "AcousticSurfaceKind"
    AFFECTIVE_STEERING_MODE = "AffectiveSteeringMode"
    AI_ARCHITECTURE_PARADIGM = "AiArchitectureParadigm"
    AI_GM_OPERATING_MODE = "AiGmOperatingMode"
    AI_GM_TURN_STATUS = "AiGmTurnStatus"
    ALERTNESS_LEVEL = "AlertnessLevel"
    APTITUDE_CODE = "AptitudeCode"
    ARC3_ENDING_HORIZON = "Arc3EndingHorizon"
    ARMOR_COVERAGE_TIER = "ArmorCoverageTier"
    ARMOR_MATERIAL_ARCHITECTURE = "ArmorMaterialArchitecture"
    AWAKENED_FACTION_ID = "AwakenedFactionId"
    BLACK_MARKET_RESTRICTION_CLASS = "BlackMarketRestrictionClass"
    BOTCH_RULE = "BotchRule"
    BULLET_CONSTRUCTION_KIND = "BulletConstructionKind"
    CAGE_INTERVENTION_ACTION = "CageInterventionAction"
    CAMPAIGN_ARC_STAGE = "CampaignArcStage"
    CANONICAL_ACTION_ID = "CanonicalActionId"
    CAREER_TERM_TRACK = "CareerTermTrack"
    CARRY_MODE = "CarryMode"
    CASCADE_OUTCOME_KIND = "CascadeOutcomeKind"
    CENTER_DISTANCE_MODE = "CenterDistanceMode"
    CHANNEL_CONTEXT = "ChannelContext"
    CHARACTERISTIC_CODE = "CharacteristicCode"
    CHASSIS_HIT_ZONE = "ChassisHitZone"
    COMBAT_DOMAIN = "CombatDomain"
    COMBAT_MOVE_CATEGORY = "CombatMoveCategory"
    COMBAT_MOVEMENT_GAIT = "CombatMovementGait"
    COMBAT_STANCE = "CombatStance"
    COMBAT_TECHNIQUE_ID = "CombatTechniqueId"
    CONSPIRACY_DISCOVERY_TIER = "ConspiracyDiscoveryTier"
    CONSPIRACY_NODE_ID = "ConspiracyNodeId"
    CONTESTED_OUTCOME_KIND = "ContestedOutcomeKind"
    CONTESTED_TIE_RULE = "ContestedTieRule"
    CORPORATE_LAB_BRAND = "CorporateLabBrand"
    COVER_GRADE = "CoverGrade"
    COVER_IDENTITY_TIER = "CoverIdentityTier"
    CYBER_AUGMENT_ID = "CyberAugmentId"
    DRONE_CHASSIS_CLASS = "DroneChassisClass"
    DRONE_MODULE_ID = "DroneModuleId"
    ECCM_HARDENING_ID = "EccmHardeningId"
    ECM_METHOD_ID = "EcmMethodId"
    ECONOMIC_TIER = "EconomicTier"
    EDGE_SIDE = "EdgeSide"
    EW_HARDWARE_ROLE = "EwHardwareRole"
    EXPLOSIVE_ORDNANCE_ID = "ExplosiveOrdnanceId"
    FACTION_ARCHETYPE_ID = "FactionArchetypeId"
    FIELD_DRUG_ID = "FieldDrugId"
    FORMATIVE_LIFE_EVENT_ID = "FormativeLifeEventId"
    GM_LOOKUP_SOURCE_KIND = "GmLookupSourceKind"
    GM_MEMORY_CATEGORY = "GmMemoryCategory"
    HIGH_SCHOOL_TRACK = "HighSchoolTrack"
    LOG_LEVEL = "LogLevel"
    MAP_CONTINUATION_DIR = "MapContinuationDir"
    MAP_SCALE_PRESET = "MapScalePreset"
    MARGIN_AGGREGATION = "MarginAggregation"
    MATERIAL_CLASS = "MaterialClass"
    MCP_SERVER_TIER = "McpServerTier"
    MELEE_CHASSIS_CLASS = "MeleeChassisClass"
    MELEE_ERGONOMIC_TUNING = "MeleeErgonomicTuning"
    MELEE_MATERIAL_GRADE = "MeleeMaterialGrade"
    MESSAGE_VISIBILITY = "MessageVisibility"
    MORALE_STATE = "MoraleState"
    NETWORK_INTRUSION_PHASE = "NetworkIntrusionPhase"
    NON_FIREARM_WEAPON_ID = "NonFirearmWeaponId"
    NON_SOLID_PROP_KIND = "NonSolidPropKind"
    NPC_TEMPLATE_KIND = "NpcTemplateKind"
    OBSERVER_ATTENTION_STATE = "ObserverAttentionState"
    OPERATIONAL_HEAT_TIER = "OperationalHeatTier"
    OUTCOME_TIER = "OutcomeTier"
    PARTICIPANT_KIND = "ParticipantKind"
    PERSONAL_HOBBY_ID = "PersonalHobbyId"
    POLITICAL_ECONOMY_TRIGGER = "PoliticalEconomyTrigger"
    POLYCRISIS_GRID_MODE = "PolycrisisGridMode"
    PROJECT_CLOCK_COMPLEXITY = "ProjectClockComplexity"
    PROP_KIND = "PropKind"
    PROTOTYPE_QUIRK_ID = "PrototypeQuirkId"
    PSYCHE_TRAIT_ID = "PsycheTraitId"
    RURAL_YOUTH_ARCHETYPE = "RuralYouthArchetype"
    SAFEHOUSE_TIER = "SafehouseTier"
    SATYANG_PILLAR_ID = "SatyangPillarId"
    SECTOR_PRESET = "SectorPreset"
    SESSION_MODE = "SessionMode"
    SESSION_ROLE = "SessionRole"
    SHOTGUN_LOAD_ID = "ShotgunLoadId"
    SKILL_ACCESS_TIER = "SkillAccessTier"
    SKILL_CATEGORY = "SkillCategory"
    SKILL_TREE_ID = "SkillTreeId"
    SKILL_TYPE = "SkillType"
    SOCIAL_ORIGIN_HOOK_KIND = "SocialOriginHookKind"
    SOCIOECONOMIC_CLASS = "SocioeconomicClass"
    STAGE4_PATH = "Stage4Path"
    STORE_ARCHETYPE = "StoreArchetype"
    STREET_GANG_ARCHETYPE = "StreetGangArchetype"
    SUBSTRATE_COUNTERPLAY_ACTION = "SubstrateCounterplayAction"
    SURFACE_FEATURE_KIND = "SurfaceFeatureKind"
    SURFACE_SHAPE_KIND = "SurfaceShapeKind"
    SURFACE_TRACTION_TIER = "SurfaceTractionTier"
    SURVEILLANCE_NODE_KIND = "SurveillanceNodeKind"
    SYNTHETIC_INTERIORITY_PROFILE = "SyntheticInteriorityProfile"
    SYNTHETIC_METABOLIC_MODE = "SyntheticMetabolicMode"
    TACTICAL_OPTIC_OR_GEAR_ID = "TacticalOpticOrGearId"
    TACTICAL_TILE_KIND = "TacticalTileKind"
    TOOL_DEPENDENCY = "ToolDependency"
    TRADECRAFT_CHANNEL_KIND = "TradecraftChannelKind"
    VEHICLE_CLASS = "VehicleClass"
    VEHICLE_ID = "VehicleId"
    WALL_EDGE_KIND = "WallEdgeKind"
    WEAPON_DAMAGE_TYPE = "WeaponDamageType"
    WORKSHOP_FACILITY_TIER = "WorkshopFacilityTier"
    ZONING_CONTEXT = "ZoningContext"


RUST_ENUM_PARITY_MAP: dict[str, type[StrEnum]] = {
    WireEnumName.SESSION_MODE: SessionMode,
    WireEnumName.PARTICIPANT_KIND: ParticipantKind,
    WireEnumName.SESSION_ROLE: SessionRole,
    WireEnumName.MESSAGE_VISIBILITY: MessageVisibility,
    WireEnumName.CHANNEL_CONTEXT: ChannelContext,
    WireEnumName.OUTCOME_TIER: OutcomeTier,
    WireEnumName.COMBAT_STANCE: CombatStance,
    WireEnumName.AI_GM_OPERATING_MODE: AiGmOperatingMode,
    WireEnumName.MCP_SERVER_TIER: McpServerTier,
    WireEnumName.AI_GM_TURN_STATUS: AiGmTurnStatus,
    WireEnumName.LOG_LEVEL: LogLevel,
    WireEnumName.GM_MEMORY_CATEGORY: GmMemoryCategory,
    WireEnumName.GM_LOOKUP_SOURCE_KIND: GmLookupSourceKind,
    WireEnumName.TACTICAL_TILE_KIND: TacticalTileKind,
    WireEnumName.COVER_GRADE: CoverGrade,
    WireEnumName.MAP_SCALE_PRESET: MapScalePreset,
    WireEnumName.MAP_CONTINUATION_DIR: MapContinuationDir,
    WireEnumName.MATERIAL_CLASS: MaterialClass,
    WireEnumName.WALL_EDGE_KIND: WallEdgeKind,
    WireEnumName.EDGE_SIDE: EdgeSide,
    WireEnumName.ECONOMIC_TIER: EconomicTier,
    WireEnumName.ZONING_CONTEXT: ZoningContext,
    WireEnumName.STORE_ARCHETYPE: StoreArchetype,
    WireEnumName.SURFACE_FEATURE_KIND: SurfaceFeatureKind,
    WireEnumName.SURFACE_SHAPE_KIND: SurfaceShapeKind,
    WireEnumName.MORALE_STATE: MoraleState,
    WireEnumName.CARRY_MODE: CarryMode,
    WireEnumName.BOTCH_RULE: BotchRule,
    WireEnumName.MARGIN_AGGREGATION: MarginAggregation,
    WireEnumName.CENTER_DISTANCE_MODE: CenterDistanceMode,
    WireEnumName.CONTESTED_TIE_RULE: ContestedTieRule,
    WireEnumName.CONTESTED_OUTCOME_KIND: ContestedOutcomeKind,
    WireEnumName.CASCADE_OUTCOME_KIND: CascadeOutcomeKind,
    WireEnumName.COMBAT_DOMAIN: CombatDomain,
    WireEnumName.COMBAT_MOVE_CATEGORY: CombatMoveCategory,
    WireEnumName.PSYCHE_TRAIT_ID: PsycheTraitId,
    WireEnumName.TOOL_DEPENDENCY: ToolDependency,
    WireEnumName.WEAPON_DAMAGE_TYPE: WeaponDamageType,
    WireEnumName.SECTOR_PRESET: SectorPreset,
    WireEnumName.AWAKENED_FACTION_ID: AwakenedFactionId,
    WireEnumName.CAMPAIGN_ARC_STAGE: CampaignArcStage,
    WireEnumName.POLYCRISIS_GRID_MODE: PolycrisisGridMode,
    WireEnumName.TRADECRAFT_CHANNEL_KIND: TradecraftChannelKind,
    WireEnumName.SYNTHETIC_INTERIORITY_PROFILE: SyntheticInteriorityProfile,
    WireEnumName.SUBSTRATE_COUNTERPLAY_ACTION: SubstrateCounterplayAction,
    WireEnumName.CAGE_INTERVENTION_ACTION: CageInterventionAction,
    WireEnumName.AFFECTIVE_STEERING_MODE: AffectiveSteeringMode,
    WireEnumName.POLITICAL_ECONOMY_TRIGGER: PoliticalEconomyTrigger,
}


__all__ = [
    "AffectiveSteeringMode",
    "AiGmOperatingMode",
    "AiGmTurnStatus",
    "AwakenedFactionId",
    "BotchRule",
    "CageInterventionAction",
    "CampaignArcStage",
    "CarryMode",
    "CascadeOutcomeKind",
    "CenterDistanceMode",
    "ChannelContext",
    "CombatDomain",
    "CombatMoveCategory",
    "CombatStance",
    "ContestedOutcomeKind",
    "ContestedTieRule",
    "CoverGrade",
    "EconomicTier",
    "EdgeSide",
    "GmLookupSourceKind",
    "GmMemoryCategory",
    "LogLevel",
    "MapContinuationDir",
    "MapScalePreset",
    "MarginAggregation",
    "MaterialClass",
    "McpServerTier",
    "MessageVisibility",
    "MoraleState",
    "OutcomeTier",
    "ParticipantKind",
    "PoliticalEconomyTrigger",
    "PolycrisisGridMode",
    "PsycheTraitId",
    "RUST_ENUM_PARITY_MAP",
    "SchemaDtoName",
    "SectorPreset",
    "SessionMode",
    "SessionRole",
    "StoreArchetype",
    "SubstrateCounterplayAction",
    "SurfaceFeatureKind",
    "SurfaceShapeKind",
    "SyntheticInteriorityProfile",
    "TacticalTileKind",
    "ToolDependency",
    "TradecraftChannelKind",
    "WallEdgeKind",
    "WeaponDamageType",
    "WireEnumName",
    "ZoningContext",
    "normalize_combat_stance",
]
