"""Offline Gemini Nano Banana seamless 2D battlemap texture generator."""

from satya_ml.texture_gen.catalog import (
    BATTLEMAP_2D_TEXTURE_CATALOG,
    PILOT_4_IDS,
    TextureSpec,
    get_texture_spec,
    select_texture_specs,
)
from satya_ml.texture_gen.prop_catalog import (
    PILOT_CLINIC_PROP_IDS,
    PROP_SPRITE_CATALOG,
    PropSpriteSpec,
    get_prop_sprite_spec,
    select_prop_sprite_specs,
)
from satya_ml.texture_gen.prop_matting import (
    crop_and_fit_rgba,
    despill_edges,
    estimate_border_key_rgb,
    extract_chroma_alpha,
    matte_prop_sprite,
    render_prop_contact_sheet,
    verify_sprite_alpha_quality,
)
from satya_ml.texture_gen.seamless import (
    blend_center_cross_heal,
    delight_albedo,
    finalize_tile,
    render_contact_sheet_3x3,
    roll_half,
    toroidal_dp_quilt,
    wrap_seam_ratio,
)

__all__ = [
    "BATTLEMAP_2D_TEXTURE_CATALOG",
    "PILOT_4_IDS",
    "PILOT_CLINIC_PROP_IDS",
    "PROP_SPRITE_CATALOG",
    "PropSpriteSpec",
    "TextureSpec",
    "blend_center_cross_heal",
    "crop_and_fit_rgba",
    "delight_albedo",
    "despill_edges",
    "estimate_border_key_rgb",
    "extract_chroma_alpha",
    "finalize_tile",
    "get_prop_sprite_spec",
    "get_texture_spec",
    "matte_prop_sprite",
    "render_contact_sheet_3x3",
    "render_prop_contact_sheet",
    "roll_half",
    "select_prop_sprite_specs",
    "select_texture_specs",
    "toroidal_dp_quilt",
    "verify_sprite_alpha_quality",
    "wrap_seam_ratio",
]
