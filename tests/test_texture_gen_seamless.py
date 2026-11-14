"""Offline unit tests for seamless 2D battlemap texture baking and catalog specs."""

from __future__ import annotations

import numpy as np
import pytest

from satya_ml.texture_gen import (
    BATTLEMAP_2D_TEXTURE_CATALOG,
    PILOT_4_IDS,
    blend_center_cross_heal,
    delight_albedo,
    finalize_tile,
    get_texture_spec,
    render_contact_sheet_3x3,
    roll_half,
    select_texture_specs,
    toroidal_dp_quilt,
    wrap_seam_ratio,
)


def _synthetic_pebble_patch(h: int = 256, w: int = 256, seed: int = 7) -> np.ndarray:
    """Non-periodic synthetic multi-frequency pebble texture with strong edge contrast."""
    rng = np.random.default_rng(seed)
    yy, xx = np.mgrid[0:h, 0:w].astype(np.float32)
    base = (
        0.50
        + 0.14 * np.sin(xx * 0.21 + 0.3) * np.cos(yy * 0.19 - 0.5)
        + 0.10 * np.sin((xx + yy) * 0.37)
        + 0.06 * rng.normal(0.0, 1.0, size=(h, w)).astype(np.float32)
    )
    rgb = np.stack(
        [
            np.clip(base * 1.04, 0.05, 0.95),
            np.clip(base * 0.96, 0.05, 0.95),
            np.clip(base * 0.86, 0.05, 0.95),
        ],
        axis=-1,
    )
    return np.rint(rgb * 255.0).astype(np.uint8)


def test_catalog_and_pilot_selection() -> None:
    assert len(BATTLEMAP_2D_TEXTURE_CATALOG) == 21
    assert PILOT_4_IDS == ("grass_meadow", "gravel_shoal", "asphalt_road", "wood_oak")

    ids = [s.id for s in BATTLEMAP_2D_TEXTURE_CATALOG]
    for extra_id in ("brushed_steel", "rooftop_gravel", "ceramic_tile", "brick_masonry", "hazard_spill"):
        assert extra_id in ids
    filenames = [s.filename for s in BATTLEMAP_2D_TEXTURE_CATALOG]
    assert len(set(ids)) == len(ids)
    assert len(set(filenames)) == len(filenames)
    for spec in BATTLEMAP_2D_TEXTURE_CATALOG:
        assert spec.filename.endswith(".webp")
        assert spec.svg_span_px >= 64
        assert spec.fallback_hex.startswith("#")
        assert len(spec.prompt) > 40

    pilot_specs = select_texture_specs(pilot=True)
    assert [s.id for s in pilot_specs] == list(PILOT_4_IDS)
    assert [s.id for s in select_texture_specs(only="wood_oak,grass_meadow")] == [
        "wood_oak",
        "grass_meadow",
    ]
    with pytest.raises(ValueError, match="Unknown texture ID"):
        select_texture_specs(only="nonexistent_material")


def test_delight_albedo_flattens_diagonal_ramp_and_keeps_contrast() -> None:
    pebbles = _synthetic_pebble_patch(256, 256, seed=11).astype(np.float32) / 255.0
    yy, xx = np.mgrid[0:256, 0:256].astype(np.float32)
    ramp = 0.50 + 0.50 * (xx / 255.0 + yy / 255.0)
    shaded = np.clip(pebbles * ramp[..., None], 0.0, 1.0)
    shaded_u8 = np.rint(shaded * 255.0).astype(np.uint8)

    delit = delight_albedo(shaded_u8, sigma_frac=0.18, strength=0.95).astype(np.float32) / 255.0
    lum = 0.2126 * delit[..., 0] + 0.7152 * delit[..., 1] + 0.0722 * delit[..., 2]
    q_means = [
        float(np.mean(lum[:128, :128])),
        float(np.mean(lum[:128, 128:])),
        float(np.mean(lum[128:, :128])),
        float(np.mean(lum[128:, 128:])),
    ]
    assert np.var(q_means) < 0.03
    assert max(q_means) - min(q_means) < 0.12
    # High-frequency contrast remains strong after flattening the low-frequency gradient.
    assert float(np.std(lum)) > 0.07
    darker = delight_albedo(shaded_u8, sigma_frac=0.18, strength=0.95, exposure_scale=0.80)
    assert float(np.mean(darker)) < float(np.mean(delit * 255.0)) * 0.90


def test_toroidal_dp_quilt_and_finalize_tile_erase_wrap_seams() -> None:
    raw = _synthetic_pebble_patch(640, 640, seed=42)
    # Inject a harsh bright/dark step along opposite boundaries so wrap error starts very high.
    raw[:, :24] = np.clip(raw[:, :24].astype(np.int16) + 55, 0, 255).astype(np.uint8)
    raw[:, -24:] = np.clip(raw[:, -24:].astype(np.int16) - 55, 0, 255).astype(np.uint8)
    raw[:24, :] = np.clip(raw[:24, :].astype(np.int16) - 45, 0, 255).astype(np.uint8)
    raw[-24:, :] = np.clip(raw[-24:, :].astype(np.int16) + 45, 0, 255).astype(np.uint8)
    raw_rx, raw_ry = wrap_seam_ratio(raw)
    assert raw_rx > 3.0 and raw_ry > 3.0

    quilted = toroidal_dp_quilt(raw, overlap_frac=0.16, feather_px=2)
    qx, qy = wrap_seam_ratio(quilted)
    assert qx <= 1.25
    assert qy <= 1.25

    spec = get_texture_spec("gravel_shoal")
    healed_dummy = roll_half(raw)
    tile = finalize_tile(raw, spec, healed_rolled_rgb=healed_dummy, out_px=512)
    assert tile.shape == (512, 512, 3)
    assert tile.dtype == np.uint8
    tx, ty = wrap_seam_ratio(tile)
    assert tx <= 1.25
    assert ty <= 1.25

    sheet = render_contact_sheet_3x3(tile, out_px=768)
    assert sheet.shape == (768, 768, 3)


def test_blend_center_cross_heal_keeps_outer_perimeter_seamless() -> None:
    base = _synthetic_pebble_patch(256, 256, seed=3)
    rolled = roll_half(base)
    healed = np.clip(rolled.astype(np.int16) + 12, 0, 255).astype(np.uint8)
    restored = blend_center_cross_heal(rolled, healed)
    assert restored.shape == base.shape
    # Outer perimeter of rolled frame (mask == 0) is preserved byte-for-byte on un-roll.
    assert np.array_equal(restored[128, :], base[128, :])
    assert np.array_equal(restored[:, 128], base[:, 128])
    rx, ry = wrap_seam_ratio(toroidal_dp_quilt(restored))
    assert rx <= 1.25 and ry <= 1.25
