"""Offline unit tests for Nano Banana 2 prop sprite catalog and chroma-key alpha matting."""

from __future__ import annotations

import numpy as np
import pytest
from PIL import Image, ImageDraw

from satya_ml.texture_gen import (
    PILOT_CLINIC_PROP_IDS,
    PROP_SPRITE_CATALOG,
    crop_and_fit_rgba,
    despill_edges,
    extract_chroma_alpha,
    get_prop_sprite_spec,
    matte_prop_sprite,
    render_prop_contact_sheet,
    select_prop_sprite_specs,
    verify_sprite_alpha_quality,
)


def _synthetic_hospital_bed_on_magenta(w: int = 768, h: int = 432) -> np.ndarray:
    """Create a synthetic AA white/blue hospital bed on an imperfect #F608EF chroma background."""
    # Render at 2x and downsample so border pixels have realistic anti-aliased pink/magenta blend.
    hi = Image.new("RGB", (w * 2, h * 2), (246, 8, 239))
    draw = ImageDraw.Draw(hi)
    # Brushed steel outer frame
    draw.rounded_rectangle([140, 120, w * 2 - 140, h * 2 - 120], radius=24, fill=(148, 160, 174))
    # Pure white mattress & pillow inside (must stay 100% opaque!)
    draw.rounded_rectangle([180, 156, w * 2 - 180, h * 2 - 156], radius=16, fill=(250, 252, 255))
    draw.rounded_rectangle([205, 190, 420, h * 2 - 190], radius=18, fill=(255, 255, 255))
    # Pale blue medical blanket
    draw.rounded_rectangle([460, 168, w * 2 - 200, h * 2 - 168], radius=12, fill=(168, 212, 238))
    lo = hi.resize((w, h), Image.Resampling.LANCZOS)
    return np.asarray(lo, dtype=np.uint8)


def test_prop_catalog_integrity_and_selection() -> None:
    assert len(PILOT_CLINIC_PROP_IDS) == 12
    assert len(PROP_SPRITE_CATALOG) >= 12
    ids = [s.id for s in PROP_SPRITE_CATALOG]
    codes = [s.prop_kind_code for s in PROP_SPRITE_CATALOG]
    files = [s.filename for s in PROP_SPRITE_CATALOG]
    assert len(set(ids)) == len(ids)
    assert len(set(codes)) == len(codes)
    assert len(set(files)) == len(files)

    for spec in PROP_SPRITE_CATALOG:
        assert spec.filename.endswith(".webp")
        assert spec.target_w_px >= 256 and spec.target_h_px >= 256
        assert spec.aspect_ratio in {"1:1", "16:9", "9:16", "3:2", "2:3"}
        assert "MAGENTA" in spec.prompt.upper()

    pilot = select_prop_sprite_specs(pilot=True)
    assert [s.id for s in pilot] == list(PILOT_CLINIC_PROP_IDS)
    assert [s.id for s in select_prop_sprite_specs(only="utility_van,hospital_bed")] == [
        "utility_van",
        "hospital_bed",
    ]
    with pytest.raises(ValueError, match="Unknown prop sprite ID"):
        select_prop_sprite_specs(only="nonexistent_prop")


def test_chroma_alpha_preserves_white_interior_and_despills_edges() -> None:
    raw = _synthetic_hospital_bed_on_magenta()
    spec = get_prop_sprite_spec("hospital_bed")

    alpha, bg_rgb = extract_chroma_alpha(raw, key_color="magenta")
    assert bg_rgb[0] > 235 and bg_rgb[1] < 20 and bg_rgb[2] > 225
    # All corners must be zeroed out.
    assert alpha[0, 0] == 0.0 and alpha[-1, -1] == 0.0
    # Center-left pure white pillow pixels must remain completely opaque.
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


def test_crop_and_fit_and_contact_sheet_shapes() -> None:
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
