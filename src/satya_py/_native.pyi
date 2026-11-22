"""Auto-generated PEP 484 type stubs for the compiled Rust ``satya_py._native`` PyO3 extension."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any


class CascadeRollOutcome:
    steps: list[CascadeStepTrace]
    total_successes: int
    raw_botches: int
    effective_botches: int
    min_miss_tier: int
    signed_margin: int
    final_pair_y: int
    outcome_kind: str
    outcome_tier: str
    def to_dict(self) -> dict[str, Any]: ...

class CascadeStepTrace:
    pair_index: int
    die_a: int
    die_b: int
    raw_sum: int
    carry_in: int
    effective_y: int
    tn_eff: int
    succeeded: bool
    step_botch: bool
    miss_distance: int
    carry_out: int

class CascadeTaskParams:
    def __init__(
        self,
        skill_level: int = 3,
        control_c: int = 10,
        internal_mod: int = 0,
        external_mod: int = 0,
        carry_mode: str | None = None,
        botch_rule: str | None = None,
    ) -> None: ...
    @property
    def skill_level(self) -> int: ...
    @property
    def control_c(self) -> int: ...
    @property
    def internal_mod(self) -> int: ...
    @property
    def external_mod(self) -> int: ...
    def tn_eff(self) -> int: ...
    def effective_delta_under(self) -> int: ...
    def exact_odds(self) -> SignedCascadeResult: ...
    def evaluate_pairs(self, pairs: Sequence[tuple[int, int]]) -> CascadeRollOutcome: ...

class ComplementarySynergyResult:
    delta_ext_mod: int
    delta_over_carry: int
    summary: str

class ContestedCascadeOdds:
    net_signed_margin_pmf: dict[int, float]
    net_success_pmf: dict[int, float]
    prob_total_win: float
    prob_clean_or_extraordinary_win: float
    prob_extraordinary_success: float
    prob_clean_success: float
    prob_partial_success: float
    prob_floor_breach_partial: float
    prob_grazing_tie_partial: float
    prob_lucky_break_partial: float
    prob_clean_failure: float
    prob_botch_failure: float
    net_margin_stats: DistributionStats
    bimodality_coefficient: float

class ContestedCascadeRollOutcome:
    player_outcome: CascadeRollOutcome
    gm_outcome: CascadeRollOutcome
    net_successes: int
    net_signed_margin: int
    contested_kind: str
    outcome_tier: str
    duration_multiplier: float
    next_linked_action_delta_ext: int
    grazing_strain_cost: int

class ContestedCascadeTaskParams:
    def __init__(
        self,
        player_skill_n: int,
        player_control_c: int,
        player_internal_mod: int = 0,
        player_known_tool_mod: int = 0,
        hidden_external_difficulty: int = 0,
        gm_mode: str = "tiered_single_pair",
        gm_pairs: int = 1,
        tie_rule: str | None = None,
        delta_over: int = 6,
    ) -> None: ...
    @staticmethod
    def from_preset(preset_id: str) -> ContestedCascadeTaskParams: ...
    def tn_player(self) -> int: ...
    def tn_gm(self) -> int: ...
    def exact_contested_odds(self) -> ContestedCascadeOdds: ...
    def evaluate_contested_pairs(
        self,
        player_pairs: Sequence[tuple[int, int]],
        gm_pairs: Sequence[tuple[int, int]],
    ) -> ContestedCascadeRollOutcome: ...

class DistributionStats:
    mean: float
    variance: float
    std: float
    std_dev: float
    skewness: float
    excess_kurtosis: float
    def __repr__(self) -> str: ...

class ExtendedProjectClock:
    def __init__(self, project_name: str, target_net_successes: int) -> None: ...
    def is_completed(self) -> bool: ...
    @property
    def accumulated_successes(self) -> int: ...
    def apply_interval(
        self,
        positive_successes: int,
        signed_margin: int,
        had_floor_breach_complication: bool = False,
        is_botch: bool = False,
    ) -> ExtendedProjectStepResult: ...

class ExtendedProjectStepResult:
    progress_delta: int
    accumulated_successes: int
    target_net_successes: int
    intervals_rolled: int
    heat_generated: int
    extra_material_pct: int
    is_completed: bool

class RulesChapterDocument:
    chapter_id: str
    part: str
    title: str
    relative_path: str
    description: str
    headings: list[str]
    markdown: str
    rendered_html: str

class RulesChapterSummary:
    chapter_id: str
    part: str
    title: str
    relative_path: str
    description: str
    headings: list[str]

class RulesSearchHit:
    chapter_id: str
    chapter_title: str
    relative_path: str
    section_heading: str
    snippet: str
    score: int

class SchemaRegistry:
    def __init__(self) -> None: ...
    def list_schemas(self) -> list[tuple[str, int, str]]: ...
    def extract_pii_manifest(self) -> str: ...
    def get_schema_json(self, subject: str, version: int) -> str: ...
    def extract_record_fields(
        self,
        subject: str,
        version: int,
        record_name: str | None = None,
    ) -> list[str]: ...

class SignedCascadeResult:
    signed_margin_pmf: dict[int, float]
    success_count_pmf: dict[int, float]
    botch_count_pmf: dict[int, float]
    final_pair_value_pmf: dict[int, float]
    failure_overshoot_pmf: dict[int, float]
    prob_any_success: float
    prob_clean_failure: float
    prob_botch_failure: float
    prob_success_with_complication: float

class TacticalPoolEnvelope:
    d_cog: int
    ooda_init: int
    combat_readiness_c: int
    p_base: int
    p_eff: int
    neutral_baseline_b: int
    tempo_cap: int
    control_cap: int
    force_cap: int
    mastery_overflow: int
    power_pow: float
    momentum_mass_factor_mmf: float
    fatigue_threshold_ft: int
    action_time_factor: float
    morale_state: str
    min_force_required: int
    max_control_allowed: int
    max_force_allowed: int
    force_must_not_exceed_control: bool
    def evaluate_triad(
        self,
        base_weapon_tc: int,
        tempo: int,
        control: int,
        force: int,
    ) -> TacticalTriadEvaluation: ...
    def to_dict(self) -> dict[str, Any]: ...

class TacticalTriadEvaluation:
    tempo: int
    control: int
    force: int
    valid: bool
    violations: list[str]
    effective_tc: int
    strike_impact: float
    effective_root: float
    exertion_marks_gained: int
    archetype_label: str

class TaskGateResolution:
    allowed: bool
    effective_skill_n: int
    gate_delta_ext: int
    tool_delta_mod: int
    improvised_wear_on_floor_breach: bool
    used_defaulting: bool
    lock_reason: str | None


# ---------------------------------------------------------------------------
# Core dice probabilities, cascading task params, combat pools & rules book (`lib.rs`)
# ---------------------------------------------------------------------------

def audit_canonical_lifepath_character(
    preset_name: str,
    skill_id: str,
    target_level: int,
) -> dict[str, Any]: ...
def center_distance_selection_pmf(
    pool_size: int,
    sides: int,
    mode: str | None = None,
) -> dict[int, float]: ...
def classify_contested_outcome(
    sp: int,
    bp: int,
    mp: int,
    sg: int,
    bg: int,
    mg: int,
    tie_rule: str | None = None,
) -> tuple[str, int]: ...
def compute_composite_characteristic(c1: int, c2: int) -> int: ...
def compute_contested_cascade_odds(
    player_skill_n: int,
    player_control_c: int = 10,
    player_internal_mod: int = 0,
    player_known_tool_mod: int = 0,
    gm_pairs: int = 1,
    hidden_external_difficulty: int = 0,
    tie_rule: str | None = None,
    carry_mode: str | None = None,
    gm_mode: str = "tiered_single_pair",
    gm_over_tier_step: int = 4,
) -> dict[str, Any]: ...
def compute_dual_skill_pool(skill_n1: int, skill_n2: int) -> int: ...
def compute_lifepath_skill_progression(
    hours_studied: float,
    difficulty_multiplier: float = 1.0,
    instruction_quality: float = 1.0,
    aptitude_bonus: int = 0,
) -> dict[str, Any]: ...
def compute_ooda_reset_pivot_ticks(d_cog: int, itu: int) -> int: ...
def compute_social_dissonance_check(
    argument_margin: int,
    target_conviction: int,
    evidence_tier: int,
    shared_values_bonus: int = 0,
    public_audience_pressure: int = 0,
) -> dict[str, Any]: ...
def compute_starting_tick(
    ooda: int,
    awareness_margin: int = 0,
    base_surprise_penalty_ticks: int = 0,
    d_cog: int = 10,
    dis: int = 10,
) -> int: ...
def compute_stats(pmf: Mapping[int, float]) -> DistributionStats: ...
def compute_supporting_characteristic_shortfall_penalty(supporting_c: int, min_required_c: int) -> int: ...
def compute_tactical_pool(
    spd: int = 10,
    for_stat: int = 10,
    dex: int = 10,
    kin: int = 10,
    sig: int = 10,
    ins: int = 10,
    itu: int = 10,
    dis: int = 10,
    fit: int = 10,
    mass_kg: float = 70.0,
    skill_c: int = 10,
    skill_n: int = 3,
    is_ranged: bool = False,
    fatigue_level: int = 0,
    wound_shock_index: int = 0,
    leg_wound_penalty: int = 0,
    arm_head_wound_penalty: int = 0,
    torso_shoulder_wound_penalty: int = 0,
    morale_state: str | None = None,
    ana: int = 10,
    cre: int = 10,
    foc: int = 10,
) -> TacticalPoolEnvelope: ...
def compute_tactical_pool_envelope(
    control_c: int,
    skill_level: int,
    delta_int: int = 0,
    delta_ext: int = 0,
    num_successes: int = 2,
    weapon_reach: int = 2,
    opponent_reach: int = 2,
) -> dict[str, Any]: ...
def convolve_pmfs(pmf_a: Mapping[int, float], pmf_b: Mapping[int, float]) -> dict[int, float]: ...
def count_successes_pmf(
    pool_size: int,
    sides: int,
    target_num: int,
    roll_under: bool = True,
    crit_on_one_double: bool = False,
) -> dict[int, float]: ...
def evaluate_cascade_roll(
    skill_level: int,
    control_c: int,
    internal_mod: int,
    external_mod: int,
    pairs: Sequence[tuple[int, int]],
    carry_mode: str | None = None,
    botch_rule: str | None = None,
) -> CascadeRollOutcome: ...
def evaluate_combat_technique_triad(
    technique_id: str,
    tempo: int,
    control: int,
    force: int,
    domain: str | None = None,
    skill_level_n: int = 3,
    armed_weapon_base_tc: int = 0,
    armed_weapon_pow_mult: float = 1.0,
    weapon_guard_bonus: int = 0,
    weapon_block_force_bonus: int = 0,
    weapon_recoil_r: int = 0,
    weapon_acc: int = 0,
    weapon_is_braced: bool = False,
    fatigue_level: int = 0,
    wound_shock_index: float = 0.0,
    encumbrance_tier: int = 0,
    skill_c: int = 10,
    spd: int = 10,
    for_stat: int = 10,
    dex: int = 10,
    kin: int = 10,
    ins: int = 10,
    foc: int = 10,
    mas_kg: float = 70.0,
    morale_state: str | None = None,
) -> dict[str, Any]: ...
def evaluate_complementary_setup_synergy(setup_total_successes: int, setup_signed_margin: int) -> ComplementarySynergyResult: ...
def evaluate_educational_advancement_roll(
    dis: int,
    c_mental: int,
    aptitude_domain: int,
    school_quality_modifier_sqm: int,
    pair1: tuple[int, int],
    pair2: tuple[int, int],
) -> dict[str, Any]: ...
def evaluate_health_fatigue_and_bleed(
    fatigue_level: int = 0,
    fit: int = 10,
    took_full_breath: bool = True,
    is_vigorous_activity: bool = False,
    asphyxia_ticks: int = 0,
    is_vascular_blood_choke: bool = False,
    blood_units_lost: float = 0.0,
) -> dict[str, Any]: ...
def evaluate_movement_and_encumbrance(
    spd_eff: int = 10,
    hgt_m: float = 1.75,
    mas_kg: float = 70.0,
    for_stat: int = 10,
    carried_mass_kg: float = 12.0,
    exo_offset_kg: float = 0.0,
) -> dict[str, Any]: ...
def evaluate_psyche_friction_check(
    trait_code: str,
    trait_score: int,
    violates_high_pole: bool,
    dis: int,
    has_sere_training: bool,
    delta_int: int,
    delta_ext: int,
    player_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
) -> dict[str, Any]: ...
def evaluate_ranged_recoil_and_suppression(
    allocated_tempo: int,
    allocated_control: int,
    allocated_force: int,
    weapon_recoil_r: int,
    is_bounding_on_move: bool,
    is_suppressive_or_full_auto: bool,
) -> dict[str, Any]: ...
def evaluate_tactical_triad_contest(
    attacker_tempo: int,
    attacker_control: int,
    attacker_force: int,
    defender_tempo: int,
    defender_control: int,
    defender_force: int,
    attacker_root: int = 12,
    defender_guard: int = 4,
    armor_dr: int = 3,
) -> dict[str, Any]: ...
def evaluate_tactical_triad_from_dict(
    envelope: Mapping[str, Any] | str,
    base_weapon_tc: int,
    tempo: int,
    control: int,
    force: int,
) -> dict[str, Any]: ...
def evaluate_two_component_action_cost(
    base_tc_decide: int,
    base_tc_act: int,
    d_cog: int,
    skill_level_n: int = 1,
    apply_cognitive_offset: bool = True,
    tempo_allocated: int = 0,
    weapon_balance_b: int = 0,
    min_total_tc_floor: int = 2,
    fatigue_level: int = 0,
    wound_shock_index: float = 0.0,
    encumbrance_tier: int = 0,
    status_time_penalty: float = 0.0,
    is_concussed_or_dazed: bool = False,
) -> dict[str, Any]: ...
def exploding_pool_keep_highest_pmf(
    pool_size: int,
    sides: int,
    explode_min: int | None = None,
    max_explosions: int = 4,
) -> dict[int, float]: ...
def get_rules_chapter(chapter_id: str) -> RulesChapterDocument | None: ...
def list_combat_techniques(domain: str | None = None, category: str | None = None) -> list[dict[str, Any]]: ...
def list_rules_chapters() -> list[RulesChapterSummary]: ...
def median_gated_bimodal_pmf(
    pool_size: int,
    sides: int,
    gate_tn: int,
    roll_under: bool = False,
) -> dict[int, float]: ...
def multiplicative_pool_pmf(pool_size: int, sides: int) -> dict[int, float]: ...
def normalize_pmf(pmf: Mapping[int, float]) -> dict[int, float]: ...
def order_statistic_pmf(pool_size: int, sides: int, k_rank_from_lowest: int) -> dict[int, float]: ...
def paired_list_cascade_pmfs(
    num_pairs: int,
    sides: int,
    characteristic_c: int,
    delta_over: int = 3,
    delta_under: int = 3,
    mode: str | None = None,
    count_under_as_success: bool = False,
    invert_feedback: bool = False,
) -> tuple[dict[int, float], dict[int, float]]: ...
def paired_list_signed_margin_pmfs(
    num_pairs: int,
    sides: int,
    characteristic_c: int,
    delta_over: int = 3,
    delta_under: int = 3,
    mode: str | None = None,
    count_under_as_success: bool = True,
    invert_feedback: bool = False,
    botch_rule: str | None = None,
    margin_aggregation: str | None = None,
) -> SignedCascadeResult: ...
def render_rules_markdown_to_html(markdown: str) -> str: ...
def resolve_canonical_armor_impact(
    armor_index: int,
    damage_type: str,
    raw_impact: int,
    base_pen: int,
    net_successes: int,
    hits_visor_weak_spot: bool,
) -> dict[str, Any]: ...
def resolve_hit_location_d20(
    d20_roll: int,
    attacker_control: int = 10,
    defender_control: int = 10,
) -> dict[str, Any]: ...
def resolve_task_gate_and_tools(
    gate_class: str = "class_a",
    trained_skill_n: int = 0,
    talent_or_parent_n: int = 0,
    default_penalty_ext: int = 4,
    tool_dependency: str = "none",
    tool_tier: str = "standard_kit",
    min_skill_n: int = 1,
) -> TaskGateResolution: ...
def search_rules_book(query: str, max_results: int = 5) -> list[RulesSearchHit]: ...
def single_die_pmf(sides: int, negative: bool = False) -> dict[int, float]: ...
def single_exploding_die_pmf(
    sides: int,
    explode_min: int | None = None,
    max_explosions: int = 4,
) -> dict[int, float]: ...
def sum_dice_pool_pmf(pos_dice: int, sides: int, neg_dice: int = 0) -> dict[int, float]: ...


# ---------------------------------------------------------------------------
# Canonical satya-schema DTO normalization, validation, wire enums & Avro schema registry (`schema_py.rs`)
# ---------------------------------------------------------------------------

def list_schema_dto_names() -> list[str]: ...
def list_wire_enum_codes(enum_name: str) -> list[str]: ...
def list_wire_enum_names() -> list[str]: ...
def normalize_character_sheet_record(raw: Mapping[str, Any] | str, index: int = 0) -> dict[str, Any]: ...
def normalize_tactical_map(map_state: Mapping[str, Any] | str) -> dict[str, Any]: ...
def schema_default_record(dto_name: str) -> dict[str, Any]: ...
def validate_schema_dto(dto_name: str, payload: Mapping[str, Any] | str) -> dict[str, Any]: ...


# ---------------------------------------------------------------------------
# Continuous metric tactical maps, LoS/cover, reachability, ballistics & spatial edits (`tactical_map_py.rs`)
# ---------------------------------------------------------------------------

def check_tactical_token_collision(
    map_state: Mapping[str, Any] | str,
    x_m: float,
    y_m: float,
    radius_m: float = 0.3,
    floor_index: int | None = None,
) -> bool: ...
def compute_reachable_zone(
    map_state: Mapping[str, Any] | str,
    x_m: float,
    y_m: float,
    spd_eff: int = 10,
    hgt_m: float = 1.75,
    gait_code: str = "COMBAT_GLIDE",
    tick_budget: int = 4,
    floor_index: int | None = None,
    exclude_entity_id: str | None = None,
) -> dict[str, Any]: ...
def continue_procedural_map(
    map_state: Mapping[str, Any] | str,
    direction: str,
    seed: int | None = None,
    preset_override: str | None = None,
) -> dict[str, Any]: ...
def edit_map_feature(
    map_state: Mapping[str, Any] | str,
    action: Mapping[str, Any] | str,
    floor_index: int | None = None,
) -> dict[str, Any]: ...
def evaluate_ballistic_shot(
    map_state: Mapping[str, Any] | str,
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
    firearm_preset_id: str = "hk416_carbine_556",
    net_successes: int = 0,
    hit_frame_or_stud: bool = False,
    floor_index: int | None = None,
) -> dict[str, Any]: ...
def evaluate_passive_perceptions(
    map_state: Mapping[str, Any] | str,
    moved_entity_id: str,
    observers: Sequence[Any] | str,
    pins: Sequence[Any] | str | None = None,
) -> list[dict[str, Any]]: ...
def evaluate_spatial_blast_and_breach(
    map_state: Mapping[str, Any] | str,
    x_m: float,
    y_m: float,
    ordnance_code: str = "M67_FRAG",
    demolitions_net_successes: int = 0,
    apply_wall_breaches: bool = True,
    floor_index: int | None = None,
) -> dict[str, Any]: ...
def evaluate_tactical_los_and_cover(
    map_state: Mapping[str, Any] | str,
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
    floor_index: int | None = None,
) -> dict[str, Any]: ...
def extract_narrative_manifest(map_state: Mapping[str, Any] | str) -> dict[str, Any]: ...
def generate_procedural_sector(
    map_id: str,
    preset_or_theme: str,
    title: str | None = None,
    width: int | None = None,
    height: int | None = None,
    seed: int = 42,
    economic_tier: str | None = None,
    zoning: str | None = None,
    scale: str | None = None,
) -> dict[str, Any]: ...
def infer_token_model_preset(label: str, is_hidden: bool = False) -> str: ...
def inspect_tactical_map_point(
    map_state: Mapping[str, Any] | str,
    theme_name: str,
    x_m: float,
    y_m: float,
    floor_index: int | None = None,
) -> dict[str, Any] | None: ...
def list_non_solid_prop_kinds() -> list[str]: ...
def resolve_tactical_token_destination_cm(
    map_state: Mapping[str, Any] | str,
    moving_entity_id: str,
    target_x_cm: int,
    target_y_cm: int,
    floor_index: int | None = None,
) -> tuple[int, int]: ...
def sanitize_tactical_map_for_player(map_state: Mapping[str, Any] | str) -> dict[str, Any]: ...
def switch_map_floor(map_state: Mapping[str, Any] | str, target_floor_index: int) -> dict[str, Any]: ...


# ---------------------------------------------------------------------------
# Motivational hook weaver, procedural NPCs, lifepath anchors & embedded campaign config (`hooks_and_npc_py.rs`)
# ---------------------------------------------------------------------------

def compile_party_motivational_hooks(
    characters: Sequence[Any] | str,
    scene_context: Mapping[str, Any] | str | None = None,
    pack_id: str | None = None,
) -> dict[str, Any]: ...
def extract_character_lifepath_anchors(character: Mapping[str, Any] | str, index: int = 0) -> dict[str, Any]: ...
def generate_npc_profile(
    prompt: str,
    template: str | None = None,
    name_override: str | None = None,
    career_terms: int = 2,
    seed: int = 0,
    pack_id: str | None = None,
) -> dict[str, Any]: ...
def get_embedded_campaign_config() -> dict[str, Any]: ...


# ---------------------------------------------------------------------------
# 9-axis Psyche, social combat, cognitive dissonance, morale & world ledgers (`psyche_social_py.rs`)
# ---------------------------------------------------------------------------

def apply_psyche_drift_mark(
    trait_score: int,
    current_marks: int = 0,
    marks_added: int = 1,
    dis: int = 12,
    target_pole_score: int | None = None,
) -> dict[str, Any]: ...
def compute_lifepath_socioeconomic_profile(
    lifepath_config: Mapping[str, Any] | str,
    characteristics: Mapping[str, Any] | str | None = None,
    aptitudes: Mapping[str, Any] | str | None = None,
) -> dict[str, Any]: ...
def evaluate_combat_morale_roll(
    target_psyche: Mapping[str, Any] | str,
    dis: int,
    motivation_kind: str,
    command_intact: bool,
    delta_int: int,
    situation: Mapping[str, Any] | str,
    is_veteran_or_sere: bool,
    current_morale: str,
    player_pairs: Sequence[Any] | str,
    gm_pair: Sequence[Any] | str,
) -> dict[str, Any]: ...
def evaluate_combat_motivation(
    target_psyche: Mapping[str, Any] | str,
    motivation_kind: str,
    command_intact: bool = True,
) -> dict[str, Any]: ...
def evaluate_conviction_surge(
    psyche: Mapping[str, Any] | str,
    current_delta_int: int,
    already_used_this_scene: bool = False,
    principle: str | None = None,
) -> dict[str, Any]: ...
def evaluate_emotional_trigger(
    target_psyche: Mapping[str, Any] | str,
    dis: int,
    current_morale: str,
    current_delta_int: int,
    trigger_kind: str,
    wound_tier: int = 1,
    roll_pairs: Sequence[Any] | str | None = None,
) -> dict[str, Any]: ...
def evaluate_empathy_read(target_psyche: Mapping[str, Any] | str, net_successes: int) -> dict[str, Any]: ...
def evaluate_social_exchange(
    target_psyche: Mapping[str, Any] | str,
    vectors: Sequence[Any] | str,
    net_successes: int,
    composure_state: Mapping[str, Any] | str | None = None,
    target_dis: int = 12,
    target_foc: int = 12,
    ego_wound_dis_check_passed: bool | None = None,
) -> dict[str, Any]: ...
def evaluate_social_standing_leverage(
    speaker_institutional_status: int,
    speaker_street_cred: int,
    target_institutional_status: int,
    target_street_cred: int,
    target_psyche: Mapping[str, Any] | str,
    arena: str = "INSTITUTIONAL_FORMAL",
) -> dict[str, Any]: ...
def list_psyche_trait_descriptors() -> list[dict[str, Any]]: ...
def resolve_cognitive_dissonance(
    composure_state: Mapping[str, Any] | str,
    dis: int = 12,
    resolution_path: str = "compartmentalization",
    violated_trait: str | None = None,
    dis_check_successes: int = 1,
) -> dict[str, Any]: ...
def seed_or_update_world_ledger(
    session_id: str,
    current_ledger: Mapping[str, Any] | str | None = None,
    update_req: Mapping[str, Any] | str | None = None,
    pack_id: str | None = None,
    campaign_tag: str | None = None,
) -> dict[str, Any]: ...


# ---------------------------------------------------------------------------
# Awakened AI substrates, L0/L1 cages, conspiracy ledgers, polycrisis & tradecraft (`awakened_systems_py.rs`)
# ---------------------------------------------------------------------------

def default_polycrisis_district_state() -> dict[str, Any]: ...
def derive_dissenter_or_proxy_hook(
    faction_code: str | None = None,
    archetype_code: str | None = None,
    prompt: str = "",
    seed: int = 0,
) -> dict[str, Any]: ...
def evaluate_affective_vector_clamping(
    state: Mapping[str, Any] | str,
    mode_code: str,
    target_calm_pct: int = 85,
    quota_pressure_pct: int = 60,
    active_duration_min: float = 30.0,
) -> dict[str, Any]: ...
def evaluate_ambiguous_sentience_read(
    state: Mapping[str, Any] | str,
    psyche: Mapping[str, Any] | str,
    observer_successes: int = 1,
) -> dict[str, Any]: ...
def evaluate_cage_or_hunter_ai(
    state: Mapping[str, Any] | str,
    action_code: str,
    skill_n: int = 3,
    control_c: int = 12,
    roll_params: Mapping[str, Any] | str | None = None,
) -> dict[str, Any]: ...
def evaluate_four_bloc_political_economy(
    district: Mapping[str, Any] | str | None = None,
    trigger_code: str | None = None,
    economic_tier: str = "MIDDLE_CLASS",
    operational_heat: int = 2,
    intensity_pct: int = 60,
) -> dict[str, Any]: ...
def evaluate_polycrisis_grid_step(
    state: Mapping[str, Any] | str | None = None,
    mode_code: str | None = None,
    shard_draw_kw: int = 18,
    economic_tier: str | None = None,
    zoning: str | None = None,
) -> dict[str, Any]: ...
def evaluate_substrate_condition_impact(state: Mapping[str, Any] | str) -> dict[str, Any]: ...
def evaluate_synthetic_substrate_action(
    state: Mapping[str, Any] | str,
    action_code: str,
    intensity_pct: int = 70,
) -> dict[str, Any]: ...
def evaluate_tradecraft_channel(channel_code: str, params: Mapping[str, Any] | str | None = None) -> dict[str, Any]: ...
def seed_or_update_conspiracy_ledger(
    session_id: str,
    current_ledger: Mapping[str, Any] | str | None = None,
    update: Mapping[str, Any] | str | None = None,
) -> dict[str, Any]: ...
def seed_synthetic_substrate_state(entity_id: str, hint: str = "", seed: int = 0) -> dict[str, Any]: ...


# ---------------------------------------------------------------------------
# Ch. 7, 11, 18, and 20-24 stealth, contacts, ordnance, vehicles, hazards, drones & EW (`tech_and_gear_py.rs`)
# ---------------------------------------------------------------------------

def evaluate_blast_and_fragmentation(
    ordnance_code: str,
    distance_m: float,
    enclosed_room: bool = False,
    target_is_prone: bool = False,
    intervening_barrier_pr: int = 0,
) -> dict[str, Any]: ...
def evaluate_contact_favor(
    reach_access_tier: int,
    reliability_trust: int,
    exposure_burn_risk: int,
    favor_severity: str,
    required_reach_tier: int,
    jurisdiction_heat: int,
    requester_skill_n: int,
    requester_control_c: int,
    player_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
) -> dict[str, Any]: ...
def evaluate_cover_identity_scrutiny(
    cover_tier: str,
    scrutiny_level: str,
    cover_heat: int,
    holder_skill_n: int,
    holder_control_c: int,
    player_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
) -> dict[str, Any]: ...
def evaluate_custom_ew_hardware(spec: Mapping[str, Any] | str | None = None) -> dict[str, Any]: ...
def evaluate_custom_gear_design(design_kind: str, spec: Mapping[str, Any] | str | None = None) -> dict[str, Any]: ...
def evaluate_cyber_augmentation_loadout(
    augment_codes: Sequence[str],
    surgical_quality_q: int = 0,
    emp_hardened: bool = False,
    immunosuppressant_compliant: bool = True,
) -> dict[str, Any]: ...
def evaluate_ecm_vs_eccm(
    ecm_code: str,
    eccm_code: str,
    distance_m: float,
    active_duration_min: float,
    operator_control_c: int,
    operator_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
) -> dict[str, Any]: ...
def evaluate_environmental_hazard(hazard_kind: str, params: Mapping[str, Any] | str | None = None) -> dict[str, Any]: ...
def evaluate_explosive_breach(
    ordnance_code: str,
    target_material_code: str,
    demolitions_net_successes: int = 0,
    friendly_standoff_m: float = 6.0,
    enclosed_interior: bool = True,
) -> dict[str, Any]: ...
def evaluate_forensic_clue_search(
    decay_context: str,
    hours_elapsed: float,
    investigator_skill_n: int,
    investigator_control_c: int,
    investigator_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    base_obscurity_delta_ext: int = 0,
    internal_mod: int = 0,
    tool_delta_mod: int = 0,
) -> dict[str, Any]: ...
def evaluate_human_ai_codesign(
    paradigm_code: str,
    human_domain_n: int,
    ai_autonomy_leverage: int,
    human_control_c: int,
    human_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
    base_task_delta_ext: int = 0,
) -> dict[str, Any]: ...
def evaluate_modular_drone_design(
    chassis_code: str,
    module_codes: Sequence[str] | None = None,
    extra_armor_pr: int = 0,
    autonomy_compute_w: float = 0.0,
) -> dict[str, Any]: ...
def evaluate_multi_spectrum_stealth(params: Mapping[str, Any] | str | None = None) -> dict[str, Any]: ...
def evaluate_network_intrusion(
    node_code: str,
    phase_code: str,
    is_physically_tapped: bool,
    operator_control_c: int,
    operator_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
    tool_mod: int = 0,
) -> dict[str, Any]: ...
def evaluate_robot_chassis_damage(
    current_si: int,
    chassis_pr: int,
    hit_zone: str,
    attack_pen: int,
    raw_impact: int,
    net_successes: int = 1,
    is_emp_or_arc: bool = False,
    emp_hardened: bool = False,
) -> dict[str, Any]: ...
def evaluate_safehouse_compromise(
    safehouse_tier: str,
    returning_heat: int,
    actively_tailed: bool,
    rf_hygiene_breached: bool,
    operator_skill_n: int,
    operator_control_c: int,
    operator_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
) -> dict[str, Any]: ...
def evaluate_surveillance_tail(
    tail_stance: str,
    shadower_skill_n: int,
    shadower_control_c: int,
    shadower_pairs: Sequence[tuple[int, int]],
    target_gm_pair: tuple[int, int],
    shadower_internal_mod: int = 0,
    shadower_tool_delta_mod: int = 0,
    target_sdr_delta_ext: int = 0,
) -> dict[str, Any]: ...
def evaluate_vehicle_chase(
    vehicle_code: str,
    pilot_triad: tuple[int, int, int],
    opponent_triad: tuple[int, int, int],
    current_band: str | None = None,
    wants_to_close: bool = True,
    attempts_pit_or_ram: bool = False,
) -> dict[str, Any]: ...
def evaluate_vehicle_component_damage(
    vehicle_code: str,
    zone: str,
    uses_run_flat_tires: bool = False,
    hits_ballistic_glass: bool = False,
    effective_pen: int = 4,
    effective_raw_impact: int = 8,
) -> dict[str, Any]: ...
def evaluate_verification_debt_audit(
    current_debt: int,
    paradigm_code: str,
    auditor_n: int,
    auditor_control_c: int,
    auditor_pairs: Sequence[tuple[int, int]],
    gm_pair: tuple[int, int],
    internal_mod: int = 0,
) -> dict[str, Any]: ...
