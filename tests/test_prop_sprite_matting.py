"""Offline unit tests for Nano Banana 2 prop sprite catalog and chroma-key alpha matting."""

from __future__ import annotations

import numpy as np
import pytest
from PIL import Image, ImageDraw

from satya_ml.texture_gen import (
    BATCH_2_CLINIC_PROP_IDS,
    BATCH_3_LIVING_BEDROOM_IDS,
    BATCH_4_KITCHEN_BATH_BAR_IDS,
    BATCH_5_OFFICE_CLINICAL_PORTALS_IDS,
    BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS,
    BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS,
    BATCH_8_CAFE_RETAIL_IDS,
    BATCH_9_INDUSTRIAL_FARM_IDS,
    BATCH_10_SCIFI_OFFICE_STREET_IDS,
    BATCH_11_RESTROOM_DINER_FARM_IDS,
    PILOT_CLINIC_PROP_IDS,
    PROP_SPRITE_CATALOG,
    crop_and_fit_rgba,
    despill_edges,
    extract_chroma_alpha,
    get_prop_sprite_spec,
    matte_prop_sprite,
    render_prop_contact_sheet,
    render_prop_variants_contact_sheet,
    select_prop_sprite_specs,
    verify_sprite_alpha_quality,
)


def _synthetic_hospital_bed_on_magenta(w: int = 768, h: int = 432) -> np.ndarray:
    """Create a synthetic AA white/blue hospital bed on an imperfect #F608EF chroma background."""
    hi = Image.new("RGB", (w * 2, h * 2), (246, 8, 239))
    draw = ImageDraw.Draw(hi)
    draw.rounded_rectangle([140, 120, w * 2 - 140, h * 2 - 120], radius=24, fill=(148, 160, 174))
    draw.rounded_rectangle([180, 156, w * 2 - 180, h * 2 - 156], radius=16, fill=(250, 252, 255))
    draw.rounded_rectangle([205, 190, 420, h * 2 - 190], radius=18, fill=(255, 255, 255))
    draw.rounded_rectangle([460, 168, w * 2 - 200, h * 2 - 168], radius=12, fill=(168, 212, 238))
    lo = hi.resize((w, h), Image.Resampling.LANCZOS)
    return np.asarray(lo, dtype=np.uint8)


def test_prop_catalog_integrity_and_variant_selection() -> None:
    assert len(PILOT_CLINIC_PROP_IDS) == 12
    assert len(BATCH_2_CLINIC_PROP_IDS) == 12
    assert len(BATCH_3_LIVING_BEDROOM_IDS) == 16
    assert len(BATCH_4_KITCHEN_BATH_BAR_IDS) == 17
    assert len(BATCH_5_OFFICE_CLINICAL_PORTALS_IDS) == 16
    assert len(BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS) == 18
    assert len(BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS) == 23
    assert len(BATCH_8_CAFE_RETAIL_IDS) == 10
    assert len(BATCH_9_INDUSTRIAL_FARM_IDS) == 8
    assert len(BATCH_10_SCIFI_OFFICE_STREET_IDS) == 8
    assert len(BATCH_11_RESTROOM_DINER_FARM_IDS) == 17
    assert len(PROP_SPRITE_CATALOG) == 157

    ids = [s.id for s in PROP_SPRITE_CATALOG]
    codes = [s.prop_kind_code for s in PROP_SPRITE_CATALOG]
    all_variant_files = [s.variant_filename(v) for s in PROP_SPRITE_CATALOG for v in range(3)]
    assert len(set(ids)) == 157
    assert len(set(codes)) == 157
    assert len(all_variant_files) == 471
    assert len(set(all_variant_files)) == 471

    for spec in PROP_SPRITE_CATALOG:
        assert spec.filename.endswith(".webp")
        assert spec.target_w_px >= 256 and spec.target_h_px >= 192
        assert spec.aspect_ratio in {"1:1", "16:9", "9:16", "3:2", "2:3"}
        expected_key = "GREEN" if spec.key_color == "green" else "MAGENTA"
        assert expected_key in spec.prompt.upper()
        assert len(spec.variant_directives) == 3
        assert spec.prompt_for_variant(0) == spec.prompt
        assert spec.variant_directives[1] in spec.prompt_for_variant(1)
        assert spec.variant_directives[2] in spec.prompt_for_variant(2)

    assert get_prop_sprite_spec("pig").key_color == "green"

    assert [s.id for s in select_prop_sprite_specs(pilot=True)] == list(PILOT_CLINIC_PROP_IDS)
    assert [s.id for s in select_prop_sprite_specs(batch="batch2")] == list(BATCH_2_CLINIC_PROP_IDS)
    assert len(select_prop_sprite_specs(batch="batch3")) == 16
    assert len(select_prop_sprite_specs(batch="batch4")) == 17
    assert len(select_prop_sprite_specs(batch="batch5")) == 16
    assert len(select_prop_sprite_specs(batch="batch6")) == 18
    assert len(select_prop_sprite_specs(batch="batch7")) == 23
    assert len(select_prop_sprite_specs(batch="batch8")) == 10
    assert len(select_prop_sprite_specs(batch="batch9")) == 8
    assert len(select_prop_sprite_specs(batch="batch10")) == 8
    assert len(select_prop_sprite_specs(batch="batch11")) == 17
    assert len(select_prop_sprite_specs(batch="packs")) == 43
    assert len(select_prop_sprite_specs(batch="remaining")) == 90
    assert len(select_prop_sprite_specs(batch="all")) == 157
    assert [s.id for s in select_prop_sprite_specs(only="utility_van,cot,car")] == [
        "utility_van",
        "cot",
        "car",
    ]
    with pytest.raises(ValueError, match="Unknown prop sprite ID"):
        select_prop_sprite_specs(only="nonexistent_prop")
    with pytest.raises(ValueError, match="Unknown prop sprite batch"):
        select_prop_sprite_specs(batch="batch99")


def test_chroma_alpha_preserves_white_interior_and_despills_edges() -> None:
    raw = _synthetic_hospital_bed_on_magenta()
    spec = get_prop_sprite_spec("hospital_bed")

    alpha, bg_rgb = extract_chroma_alpha(raw, key_color="magenta")
    assert bg_rgb[0] > 235 and bg_rgb[1] < 20 and bg_rgb[2] > 225
    assert alpha[0, 0] == 0.0 and alpha[-1, -1] == 0.0
    assert alpha[raw.shape[0] // 2, raw.shape[1] // 4] == 1.0

    despilled = despill_edges(raw, alpha, bg_rgb, key_color="magenta")
    rgba = matte_prop_sprite(raw, spec)
    assert rgba.shape == (spec.target_h_px, spec.target_w_px, 4)
    assert rgba.dtype == np.uint8

    metrics = verify_sprite_alpha_quality(rgba, key_color="magenta")
    assert metrics["corner_alpha_max"] == 0.0
    assert 0.55 <= metrics["fg_coverage"] <= 0.95
    assert metrics["edge_spill_frac"] < 0.02
    assert despilled.shape == raw.shape


def test_crop_and_fit_and_contact_sheets() -> None:
    raw = _synthetic_hospital_bed_on_magenta(432, 768)
    gurney = get_prop_sprite_spec("gurney")
    baked_gurney = matte_prop_sprite(raw, gurney)
    assert baked_gurney.shape == (512, 256, 4)

    van = get_prop_sprite_spec("utility_van")
    baked_van = crop_and_fit_rgba(
        np.dstack([raw, np.where(raw[..., 1] > 50, 255, 0).astype(np.uint8)]),
        van.target_w_px,
        van.target_h_px,
    )
    assert baked_van.shape == (640, 256, 4)

    sheet = render_prop_contact_sheet([(gurney, baked_gurney), (van, baked_van)], cols=2)
    assert sheet.ndim == 3 and sheet.shape[2] == 3

    vs = render_prop_variants_contact_sheet(
        [(gurney, [baked_gurney, baked_gurney, baked_gurney])], props_per_row=1
    )
    assert vs.ndim == 3 and vs.shape[2] == 3
