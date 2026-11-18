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
    TextureSpec(
        id="marble_floor",
        filename="marble_floor.webp",
        label="Polished Carrara Marble Floor",
        svg_span_px=128,
        fallback_hex="#eceae6",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Large square polished Carrara white and warm alabaster marble floor tiles in a 2x2 grid "
            "seen straight down; soft dove-grey and subtle champagne mineral veining flowing across "
            "tiles separated by ultra-fine light-grey stone grout seams."
        ),
    ),
    TextureSpec(
        id="terracotta_tile",
        filename="terracotta_tile.webp",
        label="Warm Terracotta Quarry Tile",
        svg_span_px=112,
        fallback_hex="#c67856",
        delight_strength=0.76,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Hand-glazed warm terracotta and kiln-baked sienna ceramic kitchen and sunroom tiles "
            "in an even 4x4 grid seen straight down; subtle earthy clay tonal shifts and crisp "
            "warm-taupe mortar grout joints."
        ),
    ),
    TextureSpec(
        id="carpet_plush",
        filename="carpet_plush.webp",
        label="Residential Woven Plush Carpet",
        svg_span_px=96,
        fallback_hex="#c5bdb1",
        delight_strength=0.82,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Soft residential tufted plush wall-to-wall carpet in warm oatmeal-taupe seen straight "
            "down; subtle micro-woven loop pile texture and gentle uniform yarn heathering with "
            "zero seams or hard grid lines."
        ),
    ),
    TextureSpec(
        id="pool_water",
        filename="pool_water.webp",
        label="Turquoise Swimming Pool Water",
        svg_span_px=160,
        fallback_hex="#38b6d6",
        delight_strength=0.72,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Clear sparkling turquoise-azure swimming pool water seen straight down; sunlit white-cyan "
            "caustic ripple networks shimmering over faint sky-blue mosaic pool basin floor."
        ),
    ),
    TextureSpec(
        id="wood_deck",
        filename="wood_deck.webp",
        label="Weathered Cedar Deck Planks",
        svg_span_px=128,
        fallback_hex="#9c6a47",
        delight_strength=0.75,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Outdoor redwood and weathered cedar timber deck planks running horizontally left-to-right "
            "with distinct narrow dark drainage shadow gaps between boards, painterly wood grain fibers, "
            "and subtle sunlit matte patina."
        ),
    ),
    TextureSpec(
        id="hardwood_walnut",
        filename="hardwood_walnut.webp",
        label="Dark Walnut Hardwood Flooring",
        svg_span_px=128,
        fallback_hex="#5d3f2c",
        delight_strength=0.74,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Rich dark espresso-walnut hardwood strip planks running horizontally left-to-right; "
            "deep chocolate and warm umber wood grain with fine staggered dark-brown plank seams."
        ),
    ),
    TextureSpec(
        id="hardwood_ash",
        filename="hardwood_ash.webp",
        label="Blonde Ash Hardwood Flooring",
        svg_span_px=128,
        fallback_hex="#d9c6a3",
        delight_strength=0.76,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Pale Scandinavian blonde-ash and white-oak hardwood planks running horizontally "
            "left-to-right; light cream-honey timber grain with delicate warm-tan plank seams."
        ),
    ),
    TextureSpec(
        id="gravel_pea",
        filename="gravel_pea.webp",
        label="Warm Golden Pea Gravel",
        svg_span_px=128,
        fallback_hex="#c8b38d",
        delight_strength=0.75,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Rounded warm tan, cream, ochre, and sandstone pea gravel pebbles closely packed "
            "in fine sandy bed seen straight down with crisp micro-shadow crevices."
        ),
    ),
    TextureSpec(
        id="gravel_slate",
        filename="gravel_slate.webp",
        label="Crushed Charcoal Slate Gravel",
        svg_span_px=128,
        fallback_hex="#5b6169",
        delight_strength=0.76,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Angular crushed dark blue-slate, basalt charcoal, and cool pewter driveway gravel "
            "chips seen straight down with crisp mineral edges and uniform aggregate scale."
        ),
    ),
    TextureSpec(
        id="grass_lush",
        filename="grass_lush.webp",
        label="Manicured Emerald Estate Lawn",
        svg_span_px=192,
        fallback_hex="#4e8442",
        delight_strength=0.86,
        delight_sigma_frac=0.12,
        exposure_scale=0.92,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Manicured deep emerald and vibrant clover-green estate lawn turf seen straight down; "
            "fine combed short grass blades with crisp painterly foliage grain and zero bare dirt."
        ),
    ),
    TextureSpec(
        id="grass_dry",
        filename="grass_dry.webp",
        label="Dry Sunlit Pasture Grass",
        svg_span_px=192,
        fallback_hex="#9e9555",
        delight_strength=0.85,
        delight_sigma_frac=0.12,
        exposure_scale=0.94,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Sun-cured golden-olive and sage prairie pasture grass seen straight down; swept dry "
            "straw-tan and muted sage grass blades with fine warm earth undertones."
        ),
    ),
    TextureSpec(
        id="wood_veneer",
        filename="wood_veneer.webp",
        label="Satin Continuous Wood Veneer",
        svg_span_px=128,
        fallback_hex="#d4cfc7",
        delight_strength=0.80,
        is_neutral_tint_mask=True,
        prompt=_prompt(
            "Continuous fine satin hardwood cabinet and tabletop veneer grain running smoothly "
            "left-to-right in neutral warm putty-oatmeal tones suitable for multiply tinting; "
            "delicate organic cathedral grain fibers over one unbroken solid timber sheet with "
            "ZERO plank seams, ZERO floorboard lines, ZERO butt joints, ZERO grooves, ZERO knots, "
            "and ZERO gaps."
        ),
    ),
    TextureSpec(
        id="clinical_vinyl",
        filename="clinical_vinyl.webp",
        label="Antiseptic Clinical Sheet Vinyl",
        svg_span_px=128,
        fallback_hex="#dce6e5",
        delight_strength=0.82,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Antiseptic pale seafoam-white speckled medical and laboratory sheet vinyl floor "
            "seen straight down; fine matte mineral micro-chips embedded in uniform hygienic "
            "off-white hospital flooring with subtle thin heat-welded seam lines and crisp "
            "clean matte finish."
        ),
    ),
    TextureSpec(
        id="linoleum_check",
        filename="linoleum_check.webp",
        label="Checkerboard Linoleum Corridor Tile",
        svg_span_px=96,
        fallback_hex="#d6d2c8",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Institutional clinic and urban apartment corridor square checkerboard linoleum "
            "tiles seen straight down; alternating muted warm-cream and soft slate-sage "
            "vintage composite vinyl tiles with fine mottled flecks, thin dark seams, and "
            "light pedestrian scuff patina."
        ),
    ),
    TextureSpec(
        id="terrazzo_stone",
        filename="terrazzo_stone.webp",
        label="Polished Terrazzo Stone Floor",
        svg_span_px=128,
        fallback_hex="#dedbd3",
        delight_strength=0.82,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Polished hospital lobby and civic atrium warm-grey terrazzo floor seen straight "
            "down; smooth pale stone cement matrix speckled with fine crushed marble, quartz, "
            "and charcoal granite chips divided into square panels by thin muted brass strips."
        ),
    ),
    TextureSpec(
        id="herringbone_brick",
        filename="herringbone_brick.webp",
        label="Herringbone Clay Brick Pavers",
        svg_span_px=96,
        fallback_hex="#a85c48",
        delight_strength=0.76,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Weathered Greenwich Village and Chinatown terracotta, russet, and burnt-sienna "
            "clay sidewalk bricks laid in a crisp 45-degree herringbone bond pattern seen "
            "straight down; hand-painted tumbled brick texture with dark sandy mortar joints."
        ),
    ),
    TextureSpec(
        id="server_raised_tile",
        filename="server_raised_tile.webp",
        label="Brushed-Steel Raised Access Floor",
        svg_span_px=96,
        fallback_hex="#7d8690",
        delight_strength=0.80,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Data-center and cleanroom brushed cool-grey steel raised access floor panels "
            "seen straight down; square modular metal floor tiles with crisp dark neoprene "
            "gasket seams, corner screw recesses, and subtle fine perforated airflow grids."
        ),
    ),
    TextureSpec(
        id="spa_jacuzzi_water",
        filename="spa_jacuzzi_water.webp",
        label="Aerated Jacuzzi Spa Water",
        svg_span_px=128,
        fallback_hex="#3ec6dc",
        delight_strength=0.72,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Aerated turquoise-aquamarine whirlpool jacuzzi spa water seen straight down; "
            "swirling frothy white micro-bubble eddies, gentle concentric hydro-jet ripples, "
            "and sunlit cyan caustics over pale ice-blue mosaic basin floor with NO rim or edges."
        ),
    ),
    TextureSpec(
        id="travertine_stone",
        filename="travertine_stone.webp",
        label="Honed Travertine Coping & Counter Stone",
        svg_span_px=128,
        fallback_hex="#ddd5c7",
        delight_strength=0.78,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Continuous honed warm cream-ivory travertine and quartz-speckled limestone slab "
            "seen straight down; subtle natural warm mineral veins, tiny soft sedimentary "
            "pores, and matte architectural stone finish with ZERO tile grid lines and ZERO grout seams."
        ),
    ),
    TextureSpec(
        id="glazed_porcelain",
        filename="glazed_porcelain.webp",
        label="Satin Glazed Porcelain Enamel",
        svg_span_px=96,
        fallback_hex="#eef2f5",
        delight_strength=0.82,
        is_neutral_tint_mask=True,
        prompt=_prompt(
            "Continuous satin-glazed pearl-white architectural porcelain and molded ceramic "
            "enamel surface seen straight down; ultra-subtle hand-painted gouache mineral "
            "tonality and soft satin ceramic luster with ZERO grout lines, ZERO grid tiles, and ZERO cracks."
        ),
    ),
    TextureSpec(
        id="persian_rug",
        filename="persian_rug.webp",
        label="Neutral Hand-Knotted Kilim Rug Weave",
        svg_span_px=128,
        fallback_hex="#d4cdc3",
        delight_strength=0.80,
        is_neutral_tint_mask=True,
        prompt=_prompt(
            "Neutral pale oatmeal-putty hand-knotted kilim and woven wool rug pile texture "
            "suitable for multiply tinting, seen straight down; repeating small hand-painted "
            "geometric diamond and medallion yarn threads filling the entire frame edge-to-edge "
            "with ZERO outer rug border or fringe."
        ),
    ),
    TextureSpec(
        id="tufted_leather",
        filename="tufted_leather.webp",
        label="Supple Pebbled Upholstery Leather",
        svg_span_px=96,
        fallback_hex="#d2ccc4",
        delight_strength=0.80,
        is_neutral_tint_mask=True,
        prompt=_prompt(
            "Neutral pale stone-putty supple pebbled full-grain upholstery leather surface "
            "suitable for multiply tinting, seen straight down; fine organic grain pores and "
            "soft hand-brushed tonal variation with ZERO buttons, ZERO pleats, ZERO stitching seams."
        ),
    ),
    TextureSpec(
        id="cedar_shingle",
        filename="cedar_shingle.webp",
        label="Hand-Split Cedar Roof Shingles",
        svg_span_px=96,
        fallback_hex="#825e42",
        delight_strength=0.76,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Weathered hand-split red-cedar and warm sienna wood roof shakes overlapping in "
            "neat horizontal courses seen straight down; rich timber grain, subtle silvered "
            "patina, and crisp dark shadow butt lines under each shingle course."
        ),
    ),
    TextureSpec(
        id="tree_bark",
        filename="tree_bark.webp",
        label="Furrowed Oak & Walnut Tree Bark",
        svg_span_px=96,
        fallback_hex="#6b4c33",
        delight_strength=0.74,
        is_neutral_tint_mask=False,
        prompt=_prompt(
            "Rough furrowed oak and walnut tree trunk bark ridges running vertically seen "
            "straight down; hand-painted umber, espresso, and muted grey-brown bark plates "
            "with deep dark vertical fissures and tiny specks of muted sage lichen."
        ),
    ),
    TextureSpec(
        id="metallic_coat",
        filename="metallic_coat.webp",
        label="Satin Powder-Coated Painted Metal",
        svg_span_px=128,
        fallback_hex="#cfd4da",
        delight_strength=0.82,
        is_neutral_tint_mask=True,
        prompt=_prompt(
            "Neutral pale silver-putty satin painted automotive and powder-coated sheet metal "
            "surface suitable for multiply tinting, seen straight down; ultra-fine hand-painted "
            "metallic gouache stipple and subtle uniform satin patina with ZERO rivets, ZERO "
            "panel lines, and ZERO harsh specular streaks."
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
