"""Declarative material catalog for 2D SVG battlemap Nano Banana textures."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class TextureSpec:
    """One tileable 2D battlemap surface or object material texture specification."""

    id: str
    filename: str
    label: str
    svg_span_px: int
    fallback_hex: str
    delight_strength: float
    is_neutral_tint_mask: bool
    prompt: str
    delight_sigma_frac: float = 0.18
    exposure_scale: float = 1.0


PILOT_4_IDS: tuple[str, ...] = (
    "grass_meadow",
    "gravel_shoal",
    "asphalt_road",
    "wood_oak",
)

_PAINTERLY_STYLE_PREAMBLE = (
    "Strictly 90-degree orthographic top-down bird's-eye view, flat 2D tabletop RPG "
    "battlemap surface texture, hand-painted gouache and ink illustration aesthetic "
    "(Dungeondraft / Czepeku battlemap style), rich brushwork and tactile detail, "
    "uniform flat zenithal ambient lighting, NO cast directional shadows, NO perspective "
    "distortion, NO vignette, NO border, NO text, uniform feature scale edge-to-edge, "
    "seamless tileable surface filling the entire square frame:"
)


def _prompt(body: str) -> str:
    return f"{_PAINTERLY_STYLE_PREAMBLE} {body}"


BATTLEMAP_2D_TEXTURE_CATALOG: tuple[TextureSpec, ...] = (
    TextureSpec(
        id="grass_meadow",
        filename="grass_meadow.webp",
        label="Meadow Turf Grass",
        svg_span_px=256,
        fallback_hex="#638045",
        delight_strength=0.88,
        delight_sigma_frac=0.11,
        exposure_scale=0.89,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Continuous dense wind-swept meadow grass turf seen straight down from above; "
            "long, straight, fine hand-painted grass blades gently combed and leaning "
            "uniformly in the same diagonal wind direction (swept toward upper-right at ~30 "
            "degrees) across the entire frame, in rich deeper forest-emerald, olive-hunter, "
            "and shaded moss-green tones; uniform darker mid-tone green shade with NO curly "
            "rosettes, NO clover flowers, NO bare brown dirt patches, NO dark holes, NO rocks, "
            "and NO isolated bright blotches."
        ),
    ),
    TextureSpec(
        id="gravel_shoal",
        filename="gravel_shoal.webp",
        label="Compacted River Gravel",
        svg_span_px=128,
        fallback_hex="#b7afa2",
        delight_strength=0.75,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Compacted river gravel and crushed stone path seen straight down; interlocking "
            "small and medium painted pebbles in warm grey, taupe, slate, ochre, and pale "
            "limestone nestled in fine sandy grit, with crisp painterly dark ambient-occlusion "
            "crevices between individual stones; even density with no big isolated boulders."
        ),
    ),
    TextureSpec(
        id="asphalt_road",
        filename="asphalt_road.webp",
        label="Weathered Asphalt Tarmac",
        svg_span_px=256,
        fallback_hex="#55585c",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Weathered dark charcoal asphalt road tarmac surface seen straight down; fine "
            "crushed mineral aggregate speckles, subtle painted tarmac roller grain, faint "
            "hairline bitumen tar micro-cracks, and muted slate-grey surface wear; strictly "
            "continuous raw asphalt ONLY with NO painted road lines, NO yellow stripes, NO "
            "white markings, NO crosswalks, NO curbs, and NO potholes."
        ),
    ),
    TextureSpec(
        id="wood_oak",
        filename="wood_oak.webp",
        label="Oiled Oak Wood Planks",
        svg_span_px=128,
        fallback_hex="#a47b55",
        delight_strength=0.72,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Warm honey-oak timber planks running strictly horizontally left-to-right; "
            "hand-painted wood grain fibers, subtle cathedral grain rings, occasional tiny "
            "tight knots, crisp warm-umber horizontal plank seams, and satin oiled matte "
            "finish with zero glossy specular highlights."
        ),
    ),
    TextureSpec(
        id="packed_dirt",
        filename="packed_dirt.webp",
        label="Packed Earth Trail",
        svg_span_px=192,
        fallback_hex="#9e8260",
        delight_strength=0.78,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Compacted warm earth and dusty trail soil seen straight down; painterly "
            "blended umber, sandy ochre, and dry clay patches, fine grit speckles, faint "
            "surface scuffs, and tiny scattered pebbles; uniform mid-brown trail earth."
        ),
    ),
    TextureSpec(
        id="bark_mulch",
        filename="bark_mulch.webp",
        label="Shredded Bark Mulch",
        svg_span_px=128,
        fallback_hex="#5c402c",
        delight_strength=0.75,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Garden bed shredded dark cedar and pine bark mulch seen straight down; "
            "overlapping painted wood chips and fibrous bark slivers in chestnut, mahogany, "
            "and dark espresso brown over rich dark compost soil crevices."
        ),
    ),
    TextureSpec(
        id="creek_water",
        filename="creek_water.webp",
        label="Clear Shallow Creek",
        svg_span_px=192,
        fallback_hex="#4689a3",
        delight_strength=0.75,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Clear shallow woodland stream water seen straight down; painted teal-blue water "
            "ripples and gentle bright caustic light filaments drifting over submerged smooth "
            "river pebbles and silty creek bed."
        ),
    ),
    TextureSpec(
        id="drainage_ditch",
        filename="drainage_ditch.webp",
        label="Damp Muddy Ditch Bed",
        svg_span_px=128,
        fallback_hex="#546854",
        delight_strength=0.75,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Damp mossy drainage channel bed seen straight down; olive moss cushions, dark "
            "wet silt mud, scattered wet twigs, and shallow murky water sheen with subtle "
            "painterly ripple highlights."
        ),
    ),
    TextureSpec(
        id="flagstone_paver",
        filename="flagstone_paver.webp",
        label="Garden Flagstone Pavers",
        svg_span_px=128,
        fallback_hex="#cfc7b8",
        delight_strength=0.74,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Hand-laid sandstone and limestone garden flagstone pavers seen straight down; "
            "irregular fitted polygonal and rectangular stone slabs in warm cream, buff, "
            "and weathered grey separated by dark earthy mortar and tiny moss joints."
        ),
    ),
    TextureSpec(
        id="concrete_slab",
        filename="concrete_slab.webp",
        label="Poured Concrete Floor",
        svg_span_px=256,
        fallback_hex="#bdb9b0",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Smooth poured warehouse concrete floor seen straight down; warm neutral grey "
            "troweled cement mottling, faint expansion joint lines, subtle mineral dust "
            "specks, and light scuff patina without puddles or debris."
        ),
    ),
    TextureSpec(
        id="stained_concrete",
        filename="stained_concrete.webp",
        label="Weathered Industrial Concrete",
        svg_span_px=256,
        fallback_hex="#a8a397",
        delight_strength=0.78,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Aged derelict industrial concrete slab seen straight down; cracked pale olive-grey "
            "cement, hairline fractures, rust-ochre mineral weathering, and gritty dust "
            "accumulation with balanced painterly texture."
        ),
    ),
    TextureSpec(
        id="hardwood_plank",
        filename="hardwood_plank.webp",
        label="Interior Hardwood Flooring",
        svg_span_px=128,
        fallback_hex="#c8a57e",
        delight_strength=0.74,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Finished interior hardwood floor boards running horizontally left-to-right; "
            "even narrow tongue-and-groove planks in warm golden-walnut and honey-maple "
            "tones with delicate hand-painted wood grain and crisp fine plank seams."
        ),
    ),
    TextureSpec(
        id="vinyl_tile",
        filename="vinyl_tile.webp",
        label="Commercial Ceramic / Vinyl Tile",
        svg_span_px=128,
        fallback_hex="#d4d8dc",
        delight_strength=0.78,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Clean modern square architectural floor tiles in an even 4x4 grid seen straight "
            "down; light cool-grey matte porcelain-vinyl surface with subtle terrazzo stone "
            "flecks and crisp thin charcoal grout lines aligned horizontally and vertically."
        ),
    ),
    TextureSpec(
        id="foliage_canopy",
        filename="foliage_canopy.webp",
        label="Botanical Leaf Canopy",
        svg_span_px=128,
        fallback_hex="#4b7b37",
        delight_strength=0.72,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Dense tree and shrub foliage canopy seen straight down from above; overlapping "
            "hand-painted botanical leaves in deep forest green, mid emerald, and sunlit "
            "chartreuse leaf tips with crisp dark shadowed gaps between leaf clusters."
        ),
    ),
    TextureSpec(
        id="granite_boulder",
        filename="granite_boulder.webp",
        label="Weathered Granite Rock",
        svg_span_px=128,
        fallback_hex="#7b7972",
        delight_strength=0.72,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Rough natural granite boulder rock surface seen straight down; hand-painted "
            "fractured stone facets, quartz and mica flecks, subtle hairline cracks, and "
            "tiny patches of muted sage-green lichen in warm charcoal and slate grey."
        ),
    ),
    TextureSpec(
        id="woven_fabric",
        filename="woven_fabric.webp",
        label="Neutral Woven Upholstery Cloth",
        svg_span_px=96,
        fallback_hex="#d6d3cd",
        delight_strength=0.80,
        is_neutral_tint_mask=True,
        prompt=_prompt(
            "Original hand-painted gouache and ink tabletop RPG battlemap illustration of "
            "coarse oatmeal-grey woven sofa upholstery and heavy burlap cloth seen straight "
            "down; irregular hand-brushed slubbed yarn threads, organic crosshatched fiber "
            "tufts, and soft pale stone-putty tonal mottling with zero folds."
        ),
    ),
    TextureSpec(
        id="brushed_steel",
        filename="brushed_steel.webp",
        label="Brushed Industrial Steel",
        svg_span_px=128,
        fallback_hex="#9aa2a9",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Brushed satin stainless and industrial sheet steel surface seen straight down; "
            "fine horizontal linear metal grain, muted cool pewter and silver-grey gouache "
            "tonality, subtle micro-scratches, and matte workshop sheen with zero harsh mirror "
            "glare, zero diamond plate bumps, and zero bolt holes."
        ),
    ),
    TextureSpec(
        id="rooftop_gravel",
        filename="rooftop_gravel.webp",
        label="Ballasted Rooftop Gravel",
        svg_span_px=128,
        fallback_hex="#736f68",
        delight_strength=0.78,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Commercial flat rooftop tar-and-gravel ballast aggregate seen straight down; "
            "densely packed small angular crushed slate, charcoal basalt, and weathered warm "
            "grey stone chips pressed into dark bitumen tar membrane with crisp dark crevices "
            "and uniform edge-to-edge grain."
        ),
    ),
    TextureSpec(
        id="ceramic_tile",
        filename="ceramic_tile.webp",
        label="Porcelain Mosaic Ceramic Tile",
        svg_span_px=96,
        fallback_hex="#dfe5e8",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Clean small square porcelain mosaic ceramic tiles arranged in an even 6x6 grid "
            "aligned strictly horizontally and vertically seen straight down; pale cool-ivory "
            "and clinical ice-grey matte glaze with subtle painted porcelain variation and crisp "
            "thin slate-grey mortar grout lines."
        ),
    ),
    TextureSpec(
        id="brick_masonry",
        filename="brick_masonry.webp",
        label="Running-Bond Clay Brick",
        svg_span_px=96,
        fallback_hex="#9c5a49",
        delight_strength=0.76,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Weathered clay brick masonry paving laid in horizontal running-bond courses seen "
            "straight down; hand-painted terracotta, burnt sienna, muted crimson, and warm "
            "ochre kiln-fired bricks separated by crisp sandy-grey mortar joints, with faint "
            "masonry dust and micro-chips."
        ),
    ),
    TextureSpec(
        id="hazard_spill",
        filename="hazard_spill.webp",
        label="Iridescent Chemical / Oil Spill",
        svg_span_px=128,
        fallback_hex="#9db82c",
        delight_strength=0.72,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Swirled industrial chemical and oil-sheen slick filling the entire square frame "
            "edge-to-edge with NO outer puddle shoreline; painterly marbled ribbons of translucent "
            "olive-chartreuse, amber-ochre, bruised teal, and dark petroleum swirl ripples."
        ),
    ),
)

_BY_ID: dict[str, TextureSpec] = {spec.id: spec for spec in BATTLEMAP_2D_TEXTURE_CATALOG}


def get_texture_spec(texture_id: str) -> TextureSpec:
    """Return a [`TextureSpec`] by ID or raise ``KeyError``."""
    return _BY_ID[texture_id]


def select_texture_specs(
    *, pilot: bool = False, only: str | None = None
) -> list[TextureSpec]:
    """Resolve the ordered list of [`TextureSpec`] targets from CLI selection flags."""
    if only:
        ids = [item.strip() for item in only.split(",") if item.strip()]
        unknown = [tid for tid in ids if tid not in _BY_ID]
        if unknown:
            valid = ", ".join(_BY_ID.keys())
            raise ValueError(f"Unknown texture ID(s): {', '.join(unknown)}. Valid: {valid}")
        return [_BY_ID[tid] for tid in ids]
    if pilot:
        return [_BY_ID[tid] for tid in PILOT_4_IDS]
    return list(BATTLEMAP_2D_TEXTURE_CATALOG)
