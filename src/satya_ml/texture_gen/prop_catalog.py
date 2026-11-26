"""Declarative catalog of top-down 2D battlemap prop sprites for Nano Banana 2."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class PropSpriteSpec:
    """One top-down architectural/prop sprite baked to transparent WebP."""

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
    key_color: str = "magenta"
    pad_frac: float = 0.03


PILOT_CLINIC_PROP_IDS: tuple[str, ...] = (
    "hospital_bed",
    "gurney",
    "surgical_table",
    "imaging_scanner",
    "nurse_station",
    "telemetry_monitor",
    "linen_cart",
    "biohazard_bin",
    "cryo_compute_cluster",
    "bio_server_vat",
    "diesel_generator",
    "utility_van",
)

_PROP_STYLE_PREAMBLE = (
    "Strictly 90-degree orthographic top-down bird's-eye view looking straight down at "
    "the roof/top surfaces, realistic hand-painted gouache and ink tabletop RPG battlemap "
    "prop illustration (Czepeku / Dungeondraft style), crisp tactile materials, subtle "
    "self-occlusion and fine surface details, uniform zenithal flat lighting, NO perspective "
    "tilt, NO isometric angle, NO side view, NO floor tiles, NO cast drop shadow on the "
    "background, NO border, NO watermark, NO text labels, single centered isolated object "
    "with a clean small margin on a COMPLETELY FLAT UNIFORM SOLID NEON MAGENTA (#FF00FF) "
    "CHROMA-KEY BACKGROUND:"
)


def _prop_prompt(body: str) -> str:
    return f"{_PROP_STYLE_PREAMBLE} {body}"


PROP_SPRITE_CATALOG: tuple[PropSpriteSpec, ...] = (
    PropSpriteSpec(
        id="hospital_bed",
        prop_kind_code="HOSPITAL_BED",
        filename="hospital_bed.webp",
        label="Hospital Ward Bed",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=_prop_prompt(
            "Horizontal landscape modern hospital ward bed (twice as wide as tall); "
            "brushed aluminum headboard and crisp white cotton pillow on the LEFT end, "
            "neatly folded pale sky-blue quilted hospital blanket covering the mattress "
            "extending RIGHT, brushed steel side guard rails along top and bottom edges, "
            "compact wheeled steel corner posts."
        ),
    ),
    PropSpriteSpec(
        id="gurney",
        prop_kind_code="GURNEY",
        filename="gurney.webp",
        label="Wheeled Trauma Stretcher",
        base_w_cells=1,
        base_h_cells=2,
        target_w_px=256,
        target_h_px=512,
        aspect_ratio="9:16",
        prompt=_prop_prompt(
            "Vertical portrait wheeled emergency trauma stretcher gurney (twice as tall "
            "as wide); white padded foam mattress with small headrest pillow at the TOP "
            "end, dark navy safety strap across the middle, brushed stainless steel "
            "folding side rails along left and right edges, steel push handles at top "
            "and bottom ends."
        ),
    ),
    PropSpriteSpec(
        id="surgical_table",
        prop_kind_code="SURGICAL_TABLE",
        filename="surgical_table.webp",
        label="Surgical Operating Table",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=_prop_prompt(
            "Horizontal landscape clinical surgical operating table (twice as wide as "
            "tall); contoured headrest cushion on the LEFT end, stainless steel chassis "
            "with sterile seafoam-teal surgical drape pad across the center, compact "
            "stainless instrument tray attached on one side, articulated circular OR "
            "surgical light arm tucked slightly over the center-right."
        ),
    ),
    PropSpriteSpec(
        id="imaging_scanner",
        prop_kind_code="IMAGING_SCANNER",
        filename="imaging_scanner.webp",
        label="MRI / CT Diagnostic Scanner",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=_prop_prompt(
            "Square footprint white medical MRI / CT diagnostic scanner viewed straight "
            "down from above; wide matte-white molded composite donut gantry housing "
            "positioned in the TOP half with a subtle glowing soft cyan circular inner "
            "bore ring and fine control indicators, and a narrow motorized sliding "
            "patient cradle bed extending straight DOWNWARD toward the bottom edge."
        ),
    ),
    PropSpriteSpec(
        id="nurse_station",
        prop_kind_code="NURSE_STATION",
        filename="nurse_station.webp",
        label="Corridor Nursing Station",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=_prop_prompt(
            "Wide modern hospital nursing & intake reception station desk (3:2 landscape); "
            "curved light warm-oak and matte-white composite counter wrapping along the "
            "TOP edge and left/right wings with an open staff bay at the bottom, three "
            "slim dark flat-screen monitors and keyboards along the top counter, neat "
            "medical clipboards and tablet, two dark charcoal rolling swivel task chairs "
            "tucked inside the bottom staff opening."
        ),
    ),
    PropSpriteSpec(
        id="telemetry_monitor",
        prop_kind_code="TELEMETRY_MONITOR",
        filename="telemetry_monitor.webp",
        label="Vitals Telemetry Monitor & IV Stand",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=_prop_prompt(
            "Compact pair of bedside medical devices seen straight down on a square "
            "canvas: on the LEFT, a dark charcoal wheeled rolling medical telemetry "
            "cart topped with a compact patient vitals monitor screen showing a crisp "
            "green ECG waveform; on the RIGHT, a brushed chrome five-star caster IV "
            "drip pole base with a translucent hanging saline pouch seen from above."
        ),
    ),
    PropSpriteSpec(
        id="linen_cart",
        prop_kind_code="LINEN_CART",
        filename="linen_cart.webp",
        label="Clean Linen & Supply Cart",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=_prop_prompt(
            "Horizontal landscape hospital clean linen and medical supply crash cart "
            "(twice as wide as tall); brushed stainless steel rectangular frame with "
            "tubular push handles on left and right ends, neatly stacked folded white "
            "towels and pale-blue hospital sheets on the left half, and a crimson-red "
            "emergency medical tray with labeled sterile gauze packs on the right half."
        ),
    ),
    PropSpriteSpec(
        id="biohazard_bin",
        prop_kind_code="BIOHAZARD_BIN",
        filename="biohazard_bin.webp",
        label="Biohazard & Soiled-Linen Bins",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=_prop_prompt(
            "Pair of rectangular clinical waste bins side-by-side viewed straight down "
            "from above: on the LEFT, a crimson-red medical biohazard sharps container "
            "with a recessed dark lid and small yellow biohazard emblem; on the RIGHT, "
            "a cobalt-blue hinged soiled-linen hamper bin with pale blue lid."
        ),
    ),
    PropSpriteSpec(
        id="cryo_compute_cluster",
        prop_kind_code="CRYO_COMPUTE_CLUSTER",
        filename="cryo_compute_cluster.webp",
        label="Cryogenic QPU Compute Cluster",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=_prop_prompt(
            "Horizontal landscape 2035 near-future cryogenic quantum compute rack cabinet "
            "(twice as wide as tall); matte dark gunmetal-slate steel top enclosure with "
            "precision slatted cooling fans, frosted stainless cryogenic coolant pipes, "
            "a crisp glowing cyan diagnostic LED strip near the top edge, and heavy "
            "insulated copper busbar manifold along the bottom edge."
        ),
    ),
    PropSpriteSpec(
        id="bio_server_vat",
        prop_kind_code="BIO_SERVER_VAT",
        filename="bio_server_vat.webp",
        label="Cryogenic Seed / Bio-Compute Vat",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=_prop_prompt(
            "Circular top-down 2035 cryogenic biological seed-vault and bio-compute "
            "containment vat seen straight down from above; heavy bolted gunmetal steel "
            "outer ring collar, thick circular viewport glass filled with luminous "
            "pale-cyan cryogenic fluid and subtle suspended seed vials, light frost "
            "condensation on the rim, and a thin brass/copper sensor crossbar."
        ),
    ),
    PropSpriteSpec(
        id="diesel_generator",
        prop_kind_code="DIESEL_GENERATOR",
        filename="diesel_generator.webp",
        label="Backup Diesel Microgrid Generator",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=_prop_prompt(
            "Horizontal landscape weathered industrial backup diesel microgrid generator "
            "enclosure (twice as wide as tall); heavy industrial ochre-amber painted "
            "steel canopy with dark louvered radiator exhaust grilles, steel lifting "
            "lugs, dark cylindrical muffler exhaust pipe on top, and a recessed black "
            "breaker control panel on the right end."
        ),
    ),
    PropSpriteSpec(
        id="utility_van",
        prop_kind_code="UTILITY_VAN",
        filename="utility_van.webp",
        label="Clinic Utility Van",
        base_w_cells=2,
        base_h_cells=5,
        target_w_px=256,
        target_h_px=640,
        aspect_ratio="9:16",
        prompt=_prop_prompt(
            "Vertical portrait top-down 2035 mutual-aid clinic utility van / ambulance "
            "support vehicle seen strictly from 90 degrees directly overhead (much taller "
            "than wide); rear cargo doors at the TOP edge, clean matte off-white and "
            "cool-grey painted roof with black roof-rack rails, compact rooftop HVAC pod, "
            "a crisp red medical cross painted cleanly on the center roof, side mirrors, "
            "dark tinted front windshield and sloped hood pointing straight DOWNWARD "
            "toward the bottom edge."
        ),
    ),
)


def get_prop_sprite_spec(sprite_id: str) -> PropSpriteSpec:
    """Lookup one [`PropSpriteSpec`] by ID."""
    for spec in PROP_SPRITE_CATALOG:
        if spec.id == sprite_id:
            return spec
    raise ValueError(f"Unknown prop sprite ID: {sprite_id!r}")


def select_prop_sprite_specs(
    *, pilot: bool = False, only: str | None = None
) -> list[PropSpriteSpec]:
    """Select all, pilot, or comma-separated prop sprite specs."""
    if only:
        wanted = [item.strip() for item in only.split(",") if item.strip()]
        return [get_prop_sprite_spec(item) for item in wanted]
    if pilot:
        return [get_prop_sprite_spec(item) for item in PILOT_CLINIC_PROP_IDS]
    return list(PROP_SPRITE_CATALOG)
