"""PEP 484 type stubs for the compiled Rust `satya_py` PyO3 extension (`crates/satya-py`)."""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

_PmfOut = dict[int, float]
_PmfIn = Mapping[int, float]
_DicePair = tuple[int, int]


class DistributionStats:
    mean: float
    variance: float
    std: float
    std_dev: float
    skewness: float
    excess_kurtosis: float

    def __repr__(self) -> str: ...


class SignedCascadeResult:
    signed_margin_pmf: _PmfOut
    success_count_pmf: _PmfOut
    botch_count_pmf: _PmfOut
    final_pair_value_pmf: _PmfOut
    failure_overshoot_pmf: _PmfOut
    prob_any_success: float
    prob_clean_failure: float
    prob_botch_failure: float
    prob_success_with_complication: float


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


class CascadeTaskParams:
    def __init__(
        self,
        skill_level: int = 3,
        control_c: int = 10,
        internal_mod: int = 0,
        external_mod: int = 0,
        carry_mode: str = "markov_step",
        botch_rule: str = "closest_miss",
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
    def evaluate_pairs(self, pairs: Sequence[_DicePair]) -> CascadeRollOutcome: ...


class ContestedCascadeOdds:
    net_signed_margin_pmf: _PmfOut
    net_success_pmf: _PmfOut
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
        tie_rule: str = "hybrid_signed_margin",
        delta_over: int = 6,
    ) -> None: ...
    @staticmethod
    def from_preset(preset_id: str) -> ContestedCascadeTaskParams: ...
    def tn_player(self) -> int: ...
    def tn_gm(self) -> int: ...
    def exact_contested_odds(self) -> ContestedCascadeOdds: ...
    def evaluate_contested_pairs(
        self,
        player_pairs: Sequence[_DicePair],
        gm_pairs: Sequence[_DicePair],
    ) -> ContestedCascadeRollOutcome: ...


class TaskGateResolution:
    allowed: bool
    effective_skill_n: int
    gate_delta_ext: int
    tool_delta_mod: int
    improvised_wear_on_floor_breach: bool
    used_defaulting: bool
    lock_reason: str | None


class ComplementarySynergyResult:
    delta_ext_mod: int
    delta_over_carry: int
    summary: str


class ExtendedProjectStepResult:
    progress_delta: int
    accumulated_successes: int
    target_net_successes: int
    intervals_rolled: int
    heat_generated: int
    extra_material_pct: int
    is_completed: bool


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


class RulesChapterSummary:
    chapter_id: str
    part: str
    title: str
    relative_path: str
    description: str
    headings: list[str]


class RulesChapterDocument:
    chapter_id: str
    part: str
    title: str
    relative_path: str
    description: str
    headings: list[str]
    markdown: str
    rendered_html: str


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


# ---------------------------------------------------------------------------
# Dice probability, cascading task & combat pool functions (`lib.rs`)
# ---------------------------------------------------------------------------

def normalize_pmf(pmf: _PmfIn) -> _PmfOut: ...
def compute_stats(pmf: _PmfIn) -> DistributionStats: ...
def convolve_pmfs(pmf_a: _PmfIn, pmf_b: _PmfIn) -> _PmfOut: ...
def single_die_pmf(sides: int, negative: bool = False) -> _PmfOut: ...
def sum_dice_pool_pmf(pos_dice: int, sides: int, neg_dice: int = 0) -> _PmfOut: ...
def count_successes_pmf(
    pool_size: int,
    sides: int,
    target_num: int,
    roll_under: bool = True,
    crit_on_one_double: bool = False,
) -> _PmfOut: ...
def single_exploding_die_pmf(
    sides: int,
    explode_min: int | None = None,
    max_explosions: int = 4,
) -> _PmfOut: ...
def exploding_pool_keep_highest_pmf(
    pool_size: int,
    sides: int,
    explode_min: int | None = None,
    max_explosions: int = 4,
) -> _PmfOut: ...
def order_statistic_pmf(
    pool_size: int,
    sides: int,
    k_rank_from_lowest: int,
) -> _PmfOut: ...
def center_distance_selection_pmf(
    pool_size: int,
    sides: int,
    mode: str = "furthest",
) -> _PmfOut: ...
def median_gated_bimodal_pmf(
    pool_size: int,
    sides: int,
    gate_tn: int,
    roll_under: bool = False,
) -> _PmfOut: ...
def multiplicative_pool_pmf(pool_size: int, sides: int) -> _PmfOut: ...
def paired_list_cascade_pmfs(
    num_pairs: int,
    sides: int,
    characteristic_c: int,
    delta_over: int = 3,
    delta_under: int = 3,
    mode: str = "markov_step",
    count_under_as_success: bool = False,
    invert_feedback: bool = False,
) -> tuple[_PmfOut, _PmfOut]: ...
def paired_list_signed_margin_pmfs(
    num_pairs: int,
    sides: int,
    characteristic_c: int,
    delta_over: int = 3,
    delta_under: int = 3,
    mode: str = "markov_step",
    count_under_as_success: bool = True,
    invert_feedback: bool = False,
    botch_rule: str = "closest_miss",
    margin_aggregation: str = "piecewise_zero_floor",
) -> SignedCascadeResult: ...
def evaluate_cascade_roll(
    skill_level: int,
    control_c: int,
    internal_mod: int,
    external_mod: int,
    pairs: Sequence[_DicePair],
    carry_mode: str = "markov_step",
    botch_rule: str = "closest_miss",
) -> CascadeRollOutcome: ...
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
    morale_state: str = "steady",
    ana: int = 10,
    cre: int = 10,
    foc: int = 10,
) -> TacticalPoolEnvelope: ...
def resolve_task_gate_and_tools(
    gate_class: str = "class_a",
    trained_skill_n: int = 0,
    talent_or_parent_n: int = 0,
    default_penalty_ext: int = 4,
    tool_dependency: str = "none",
    tool_tier: str = "standard_kit",
    min_skill_n: int = 1,
) -> TaskGateResolution: ...
def compute_composite_characteristic(c1: int, c2: int) -> int: ...
def compute_supporting_characteristic_shortfall_penalty(
    supporting_c: int,
    min_required_c: int,
) -> int: ...
def evaluate_complementary_setup_synergy(
    setup_total_successes: int,
    setup_signed_margin: int,
) -> ComplementarySynergyResult: ...
def compute_dual_skill_pool(skill_n1: int, skill_n2: int) -> int: ...
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
    encumbrance_tier: int = 1,
    status_time_penalty: float = 0.0,
    is_concussed_or_dazed: bool = False,
) -> dict[str, Any]: ...
def compute_starting_tick(
    ooda: int,
    awareness_margin: int = 0,
    base_surprise_penalty_ticks: int = 0,
    d_cog: int = 10,
    dis: int = 10,
) -> int: ...
def compute_ooda_reset_pivot_ticks(d_cog: int, itu: int) -> int: ...
def evaluate_educational_advancement_roll(
    dis: int,
    c_mental: int,
    aptitude_domain: int,
    school_quality_modifier_sqm: int,
    pair1: _DicePair,
    pair2: _DicePair,
) -> dict[str, Any]: ...
def audit_canonical_lifepath_character(
    preset_name: str,
    skill_id: str,
    target_level: int,
) -> str: ...
def list_rules_chapters() -> list[RulesChapterSummary]: ...
def get_rules_chapter(chapter_id: str) -> RulesChapterDocument | None: ...
def search_rules_book(query: str, max_results: int = 5) -> list[RulesSearchHit]: ...
def render_rules_markdown_to_html(markdown: str) -> str: ...
def classify_contested_outcome(
    sp: int,
    bp: int,
    mp: int,
    sg: int,
    bg: int,
    mg: int,
    tie_rule: str = "hybrid_signed_margin",
) -> tuple[str, int]: ...
def compute_contested_cascade_odds(
    player_skill_n: int,
    player_control_c: int = 10,
    player_internal_mod: int = 0,
    player_known_tool_mod: int = 0,
    gm_pairs: int = 1,
    hidden_external_difficulty: int = 0,
    tie_rule: str = "hybrid_signed_margin",
    carry_mode: str = "markov_step",
    gm_mode: str = "tiered_single_pair",
    gm_over_tier_step: int = 4,
) -> dict[str, Any]: ...
def compute_tactical_pool_envelope(
    control_c: int,
    skill_level: int,
    delta_int: int = 0,
    delta_ext: int = 0,
    num_successes: int = 2,
    weapon_reach: int = 2,
    opponent_reach: int = 2,
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
def compute_lifepath_skill_progression(
    hours_studied: float,
    difficulty_multiplier: float = 1.0,
    instruction_quality: float = 1.0,
    aptitude_bonus: int = 0,
) -> dict[str, Any]: ...
def compute_social_dissonance_check(
    argument_margin: int,
    target_conviction: int,
    evidence_tier: int,
    shared_values_bonus: int = 0,
    public_audience_pressure: int = 0,
) -> dict[str, Any]: ...
def evaluate_psyche_friction_check(
    trait_code: str,
    trait_score: int,
    violates_high_pole: bool,
    dis: int,
    has_sere_training: bool,
    delta_int: int,
    delta_ext: int,
    player_pairs: Sequence[_DicePair],
    gm_pair: _DicePair,
) -> str: ...
def evaluate_ranged_recoil_and_suppression(
    _allocated_tempo: int,
    _allocated_control: int,
    allocated_force: int,
    weapon_recoil_r: int,
    is_bounding_on_move: bool,
    is_suppressive_or_full_auto: bool,
) -> dict[str, Any]: ...
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
def list_combat_techniques(
    domain: str | None = None,
    category: str | None = None,
) -> list[dict[str, Any]]: ...
def evaluate_combat_technique_triad(
    technique_id: str,
    tempo: int,
    control: int,
    force: int,
    domain: str = "unarmed_martial_arts",
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
    encumbrance_tier: int = 1,
) -> dict[str, Any]: ...


# ---------------------------------------------------------------------------
# Tactical map spatial bindings (`tactical_map_py.rs`)
# ---------------------------------------------------------------------------

def check_tactical_token_collision(
    map_json: str,
    x_m: float,
    y_m: float,
    radius_m: float,
) -> bool: ...
def resolve_tactical_token_destination_cm(
    map_json: str,
    moving_entity_id: str,
    target_x_cm: int,
    target_y_cm: int,
) -> tuple[int, int]: ...
def evaluate_tactical_los_and_cover(
    map_json: str,
    x1_m: float,
    y1_m: float,
    x2_m: float,
    y2_m: float,
) -> dict[str, Any]: ...
def inspect_tactical_map_point(
    map_json: str,
    theme_name: str,
    x_m: float,
    y_m: float,
) -> dict[str, Any] | None: ...
def generate_procedural_tactical_sector_json(
    map_id: str,
    preset_or_theme: str,
    title: str | None = None,
    width: int | None = None,
    height: int | None = None,
    seed: int = 42,
    economic_tier: str | None = None,
    zoning: str | None = None,
    scale: str | None = None,
) -> str: ...
def continue_procedural_tactical_map_json(
    map_json: str,
    direction: str,
    seed: int | None = None,
) -> str: ...
def switch_tactical_map_floor_json(
    map_json: str,
    target_floor_index: int,
) -> str: ...
def extract_sector_narrative_manifest_json(map_json: str) -> str: ...
def evaluate_passive_movement_perceptions_json(
    map_json: str,
    moved_entity_id: str,
    observers_json: str,
    pins_json: str | None = None,
) -> str: ...


# ---------------------------------------------------------------------------
# Motivational hooks & NPC generation (`hooks_and_npc_py.rs`)
# ---------------------------------------------------------------------------

def compile_party_motivational_hooks_json(
    party_sheets_json: str,
    scene_context_json: str | None = None,
    pack_id: str | None = None,
) -> str: ...
def generate_npc_profile_json(
    prompt: str,
    template: str | None = None,
    name_override: str | None = None,
    career_terms: int = 2,
    seed: int = 0,
    pack_id: str | None = None,
) -> str: ...
def extract_character_lifepath_anchors_json(character_json: str) -> str: ...


# ---------------------------------------------------------------------------
# Social combat, Psyche & World Clocks (`psyche_social_py.rs`)
# ---------------------------------------------------------------------------

def evaluate_empathy_read_json(
    target_psyche_json: str,
    net_successes: int,
) -> str: ...
def evaluate_social_exchange_json(
    target_psyche_json: str,
    vectors_json: str,
    net_successes: int,
    composure_state_json: str | None = None,
    target_dis: int = 12,
    target_foc: int = 12,
    ego_wound_dis_check_passed: bool | None = None,
) -> str: ...
def resolve_cognitive_dissonance_json(
    composure_state_json: str,
    dis: int = 12,
    resolution_path: str = "compartmentalization",
    violated_trait: str | None = None,
    dis_check_successes: int = 1,
) -> str: ...
def evaluate_conviction_surge_json(
    psyche_json: str,
    current_delta_int: int,
    already_used_this_scene: bool = False,
    principle: str | None = None,
) -> str: ...
def apply_psyche_drift_mark_json(
    trait_score: int,
    current_marks: int = 0,
    marks_added: int = 1,
    dis: int = 12,
    target_pole_score: int | None = None,
) -> str: ...
def seed_or_update_world_ledger_json(
    session_id: str,
    current_ledger_json: str | None = None,
    update_req_json: str | None = None,
    pack_id: str | None = None,
    campaign_tag: str | None = None,
) -> str: ...
def evaluate_combat_motivation_json(
    target_psyche_json: str,
    motivation_kind: str,
    command_intact: bool = True,
) -> str: ...
def evaluate_emotional_trigger_json(
    target_psyche_json: str,
    dis: int,
    current_morale: str,
    current_delta_int: int,
    trigger_kind: str,
    wound_tier: int = 1,
    roll_pairs_json: str | None = None,
) -> str: ...
def evaluate_combat_morale_roll_json(
    target_psyche_json: str,
    dis: int,
    motivation_kind: str,
    command_intact: bool,
    delta_int: int,
    situation_json: str,
    is_veteran_or_sere: bool,
    current_morale: str,
    player_pairs_json: str,
    gm_pair_json: str,
) -> str: ...


# ---------------------------------------------------------------------------
# Awakened AI substrate, conspiracy, polycrisis & tradecraft (`awakened_systems_py.rs`)
# ---------------------------------------------------------------------------

def seed_synthetic_substrate_state_json(
    entity_id: str,
    hint: str = "",
    seed: int = 0,
) -> str: ...
def evaluate_synthetic_substrate_action_json(
    state_json: str,
    action_code: str,
    intensity_pct: int = 70,
) -> str: ...
def evaluate_cage_or_hunter_ai_json(
    state_json: str,
    action_code: str,
    skill_n: int = 3,
    control_c: int = 12,
    roll_params_json: str | None = None,
) -> str: ...
def evaluate_ambiguous_sentience_read_json(
    state_json: str,
    psyche_json: str,
    observer_successes: int = 1,
) -> str: ...
def seed_or_update_conspiracy_ledger_json(
    session_id: str,
    current_ledger_json: str | None = None,
    update_json: str | None = None,
) -> str: ...
def evaluate_polycrisis_grid_step_json(
    state_json: str | None = None,
    mode_code: str = "THROTTLE_SHARD_SAFE",
    shard_draw_kw: int = 18,
    economic_tier: str | None = None,
    zoning: str | None = None,
) -> str: ...
def evaluate_tradecraft_channel_json(
    channel_code: str,
    params_json: str | None = None,
) -> str: ...
def derive_dissenter_or_proxy_hook_json(
    faction_code: str | None = None,
    archetype_code: str | None = None,
    prompt: str = "",
    seed: int = 0,
) -> str: ...
