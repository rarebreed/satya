"""Offline unit tests for Style C top-down 2D battlemap token sprite catalog and alpha matting."""

from __future__ import annotations

import numpy as np
import pytest
from PIL import Image, ImageDraw

from satya_ml.texture_gen import (
    BATCH_ANDROIDS_IDS,
    BATCH_CIVILIANS_IDS,
    BATCH_DRONES_IDS,
    BATCH_MOVE_ANDROIDS_IDS,
    BATCH_MOVE_CIVILIANS_IDS,
    BATCH_MOVE_SECURITY_IDS,
    BATCH_MOVEMENT_IDS,
    BATCH_SECURITY_IDS,
    PILOT_TOKEN_IDS,
    TOKEN_SPRITE_CATALOG,
    get_token_sprite_spec,
    matte_prop_sprite,
    render_prop_contact_sheet,
    select_token_sprite_specs,
    verify_sprite_alpha_quality,
)
from satya_ml.texture_gen.token_cli import _rotate_raw_cw


def _synthetic_topdown_operator_on_magenta(w: int = 512, h: int = 512) -> np.ndarray:
    """Create a synthetic Style C top-down operator silhouette on #FA05F2 magenta."""
    hi = Image.new("RGB", (w * 2, h * 2), (250, 5, 242))
    draw = ImageDraw.Draw(hi)
    # Arms/rifle extending to 12 o'clock
    draw.rounded_rectangle([w - 38, 160, w + 38, h - 80], radius=16, fill=(48, 56, 62))
    # Shoulders
    draw.ellipse([w - 220, h - 140, w + 220, h + 150], fill=(96, 114, 102), outline=(18, 22, 26), width=12)
    # Helmet / head
    draw.ellipse([w - 110, h - 120, w + 110, h + 100], fill=(74, 86, 78), outline=(18, 22, 26), width=12)
    lo = hi.resize((w, h), Image.Resampling.LANCZOS)
    return np.asarray(lo, dtype=np.uint8)


def test_token_catalog_integrity_and_batch_selection() -> None:
    assert len(PILOT_TOKEN_IDS) == 3
    assert len(BATCH_CIVILIANS_IDS) == 11
    assert len(BATCH_SECURITY_IDS) == 9
    assert len(BATCH_DRONES_IDS) == 2
    assert len(BATCH_ANDROIDS_IDS) == 7
    assert len(BATCH_MOVE_CIVILIANS_IDS) == 12
    assert len(BATCH_MOVE_SECURITY_IDS) == 9
    assert len(BATCH_MOVE_ANDROIDS_IDS) == 9
    assert len(BATCH_MOVEMENT_IDS) == 30
    assert len(TOKEN_SPRITE_CATALOG) == 59

    ids = [s.id for s in TOKEN_SPRITE_CATALOG]
    files = [s.variant_filename(v) for s in TOKEN_SPRITE_CATALOG for v in range(3)]
    assert len(set(ids)) == 59
    assert len(files) == 177
    assert len(set(files)) == 177

    # Drones have zero movement poses.
    assert not any(
        s.preset in {"drone_quad", "drone_aero"}
        and s.stance in {"walking_holstered", "tactical_advance", "sprinting"}
        for s in TOKEN_SPRITE_CATALOG
    )

    for spec in TOKEN_SPRITE_CATALOG:
        assert spec.filename.endswith(".webp")
        assert "MAGENTA" in spec.prompt.upper()
        assert "STYLE C" in spec.prompt.upper()
        assert "12 O'CLOCK" in spec.prompt.upper()
        assert len(spec.variant_directives) == 3
        assert all(d.strip() for d in spec.variant_directives)
        assert spec.prompt_for_variant(0) == spec.prompt
        assert spec.variant_directives[1] in spec.prompt_for_variant(1)
        assert spec.variant_directives[2] in spec.prompt_for_variant(2)
        if spec.stance == "prone":
            assert spec.aspect_ratio == "2:3"
            assert (spec.target_w_px, spec.target_h_px) == (384, 576)
        else:
            assert spec.aspect_ratio == "1:1"
            assert (spec.target_w_px, spec.target_h_px) == (512, 512)

    assert len(select_token_sprite_specs(batch="pilot")) == 3
    assert len(select_token_sprite_specs(batch="civilians")) == 11
    assert len(select_token_sprite_specs(batch="security")) == 9
    assert len(select_token_sprite_specs(batch="drones")) == 2
    assert len(select_token_sprite_specs(batch="androids")) == 7
    assert len(select_token_sprite_specs(batch="move_civilians")) == 12
    assert len(select_token_sprite_specs(batch="move_security")) == 9
    assert len(select_token_sprite_specs(batch="move_androids")) == 9
    assert len(select_token_sprite_specs(batch="movement")) == 30
    assert len(select_token_sprite_specs(batch="all")) == 59
    assert [s.id for s in select_token_sprite_specs(only="civilian_knife_standing,drone_aero_standing")] == [
        "civilian_knife_standing",
        "drone_aero_standing",
    ]
    with pytest.raises(ValueError, match="Unknown token sprite ID"):
        select_token_sprite_specs(only="nonexistent_token")
    with pytest.raises(ValueError, match="Unknown token sprite batch"):
        select_token_sprite_specs(batch="batch99")


def test_token_matting_contact_sheet_and_rotation() -> None:
    spec = get_token_sprite_spec("soldier_rifle_standing")
    raw = _synthetic_topdown_operator_on_magenta()
    rgba = matte_prop_sprite(raw, spec)
    assert rgba.shape == (512, 512, 4)
    metrics = verify_sprite_alpha_quality(rgba, key_color=spec.key_color)
    assert metrics["corner_alpha_max"] < 16.0
    assert 0.06 <= metrics["fg_coverage"] <= 0.94
    assert metrics["edge_spill_frac"] <= 0.08

    sheet = render_prop_contact_sheet(
        [(spec, rgba)],
        cols=2,
        label_fn=lambda s: f"{s.preset} [{s.stance}]",
    )
    assert sheet.ndim == 3 and sheet.shape[2] == 3

    rot = _rotate_raw_cw(np.arange(12, dtype=np.uint8).reshape(3, 4, 1), 90)
    assert rot.shape == (4, 3, 1)
    assert rot[0, 2, 0] == 0
