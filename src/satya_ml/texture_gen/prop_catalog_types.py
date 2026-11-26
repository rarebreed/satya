"""Shared types and prompt preamble for top-down 2D battlemap prop sprites."""

from __future__ import annotations

from dataclasses import dataclass

PROP_STYLE_PREAMBLE = (
    "Strictly 90-degree orthographic top-down bird's-eye view looking straight down at "
    "the roof/top surfaces, realistic hand-painted gouache and ink tabletop RPG battlemap "
    "prop illustration (Czepeku / Dungeondraft style), crisp tactile materials, subtle "
    "self-occlusion and fine surface details, uniform zenithal flat lighting, NO perspective "
    "tilt, NO isometric angle, NO side view, NO floor tiles, NO cast drop shadow on the "
    "background, NO border, NO watermark, NO text labels, single centered isolated object "
    "with a clean small margin on a COMPLETELY FLAT UNIFORM SOLID NEON MAGENTA (#FF00FF) "
    "CHROMA-KEY BACKGROUND:"
)


def prop_prompt(body: str) -> str:
    """Wrap a prop body prompt with the standard 90-degree top-down chroma-key preamble."""
    return f"{PROP_STYLE_PREAMBLE} {body}"


@dataclass(frozen=True)
class PropSpriteSpec:
    """One top-down architectural/prop sprite baked to transparent WebP (with 3 visual variants)."""

    id: str
    prop_kind_code: str
    filename: str
    label: str
    base_w_cells: int
    base_h_cells: int
    target_w_px: int
    target_h_px: int
    aspect_ratio: str
    prompt: str
    variant_directives: tuple[str, str, str]
    key_color: str = "magenta"
    pad_frac: float = 0.03

    def variant_filename(self, idx: int) -> str:
        return self.filename if idx == 0 else f"{self.id}_v{idx}.webp"

    def variant_raw_stem(self, idx: int) -> str:
        return self.id if idx == 0 else f"{self.id}_v{idx}"

    def prompt_for_variant(self, idx: int) -> str:
        if idx <= 0:
            return self.prompt
        return f"{self.prompt} Variant detail: {self.variant_directives[idx]}"
