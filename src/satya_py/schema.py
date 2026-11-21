"""Apache Avro schema registry, canonical `satya-schema` DTO normalization, validation, and typed contracts."""

from __future__ import annotations

from typing import TypedDict

from satya_py._native import (
    SchemaRegistry,
    list_schema_dto_names,
    list_wire_enum_codes,
    normalize_character_sheet_record,
    normalize_tactical_map,
    schema_default_record,
    validate_schema_dto,
)


class CharacterCharacteristicsRecordDict(TypedDict):
    """Canonical Python representation matching `satya_schema::CharacterCharacteristicsRecord`."""

    spd: int
    for_stat: int
    dex: int
    kin: int
    hlt: int
    fit: int
    hgt_m: float
    mas_kg: float
    mem: int
    ana: int
    ins: int
    cre: int
    dis: int
    foc: int
    com: int
    pre: int
    elo: int
    sig: int
    hea: int
    tac: int
    olf: int
    tas: int
    itu: int
    ego: int
    slf: int
    opn: int
    blg: int
    ext: int
    cnf: int
    agg: int
    trs: int
    cps: int


class CharacterAptitudesRecordDict(TypedDict):
    """Canonical Python representation matching `satya_schema::CharacterAptitudesRecord`."""

    apt_qnt: int
    apt_lng: int
    apt_bio: int
    apt_mec: int
    apt_cmp: int
    apt_kin: int
    apt_fin: int
    apt_art: int
    apt_soc: int


class AnatomicalZoneRecordDict(TypedDict):
    """Canonical Python representation matching `satya_schema::AnatomicalZoneRecord`."""

    location_code: str
    wound_tier: int
    bone_status: str
    bleed_grade: int


class CharacterAssetRefDict(TypedDict):
    """Canonical Python representation matching `satya_schema::CharacterAssetRef`."""

    asset_id: str
    asset_kind: str
    filename: str
    mime_type: str
    sha256_hex: str
    size_bytes: int
    created_at_us: int


class CharacterSheetRecordDict(TypedDict):
    """Canonical Python representation matching `satya_schema::CharacterSheetRecord`."""

    character_id: str
    owner_user_id: str
    version: int
    character_name: str
    callsign_or_role: str
    age_years: int
    origin_label: str
    campaign_sqm: int
    operational_heat: int
    granted_access_tags: list[str]
    stage_history: list[str]
    characteristics: CharacterCharacteristicsRecordDict
    aptitudes: CharacterAptitudesRecordDict
    skills: dict[str, int]
    equipped_ranged_ids: list[str]
    equipped_melee_codes: list[str]
    equipped_armor_codes: list[str]
    field_gear_ids: list[str]
    hit_zones: list[AnatomicalZoneRecordDict]
    fatigue_level: int
    exertion_marks: int
    wound_shock_index: int
    blood_units_lost: float
    active_status_conditions: list[str]
    morale_state: str
    mental_trauma: str | None
    social_composure_lost: int
    social_wound: str | None
    psyche_drift_marks: int
    notes: str | None
    lifepath_config_json: str | None
    assets: list[CharacterAssetRefDict]
    verification_status: str
    sp_budget_valid: bool
    dag_prereqs_valid: bool
    audit_violations: list[str]
    content_sha256: str
    server_signature_hex: str | None
    updated_at_us: int
    faction_id: str | None
    faction_archetype: str | None


__all__ = [
    "AnatomicalZoneRecordDict",
    "CharacterAptitudesRecordDict",
    "CharacterAssetRefDict",
    "CharacterCharacteristicsRecordDict",
    "CharacterSheetRecordDict",
    "SchemaRegistry",
    "list_schema_dto_names",
    "list_wire_enum_codes",
    "normalize_character_sheet_record",
    "normalize_tactical_map",
    "schema_default_record",
    "validate_schema_dto",
]
