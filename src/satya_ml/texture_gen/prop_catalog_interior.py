"""Batches 3–5 prop sprite specs: living/bedroom, kitchen/bath/bar, and office/portals."""

from __future__ import annotations

from satya_ml.texture_gen.prop_catalog_types import PropSpriteSpec, prop_prompt

BATCH_3_LIVING_BEDROOM_IDS: tuple[str, ...] = (
    "sofa",
    "loveseat",
    "armchair",
    "coffee_table",
    "rug",
    "runner_rug",
    "round_rug",
    "dining_table",
    "round_table",
    "bed",
    "single_bed",
    "bookshelf",
    "wardrobe",
    "nightstand",
    "dresser",
    "tv_stand",
)

BATCH_4_KITCHEN_BATH_BAR_IDS: tuple[str, ...] = (
    "kitchen_counter",
    "stove",
    "fridge",
    "bar_counter",
    "back_bar_shelf",
    "bar_stool",
    "pool_table",
    "jukebox",
    "booth",
    "bathtub",
    "toilet",
    "bathroom_sink",
    "shower_stall",
    "spa_bath",
    "linen_closet",
    "washer_dryer",
    "stereo",
)

BATCH_5_OFFICE_CLINICAL_PORTALS_IDS: tuple[str, ...] = (
    "desk_l",
    "desk_t",
    "desk_lamp",
    "standing_lamp",
    "desk_clutter",
    "filing_cabinet",
    "water_cooler",
    "fire_extinguisher",
    "trash_can",
    "floor_clothes",
    "floor_toys",
    "medical_counter",
    "privacy_curtain",
    "stairwell",
    "elevator_car",
    "service_elevator",
)

INTERIOR_PROP_SPECS: tuple[PropSpriteSpec, ...] = (
    # --- Batch 3: Living & Bedroom (16) ---
    PropSpriteSpec(
        id="sofa",
        prop_kind_code="SOFA",
        filename="sofa.webp",
        label="Sofa (3-seat)",
        base_w_cells=3,
        base_h_cells=1,
        target_w_px=576,
        target_h_px=192,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal 3-seat upholstered living-room sofa (3x wider than tall); "
            "thick padded backrest along the TOP edge, wide armrests on left and right, "
            "three plump seat cushions facing down."
        ),
        variant_directives=(
            "Warm slate-blue woven linen upholstery with two subtle cream throw pillows.",
            "Distressed cognac-brown tufted leather three-seat couch with warm saddle stitching.",
            "Muted olive-moss velvet sofa with a folded ochre knit throw blanket draped over one seat.",
        ),
    ),
    PropSpriteSpec(
        id="loveseat",
        prop_kind_code="LOVESEAT",
        filename="loveseat.webp",
        label="Loveseat",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal 2-seat upholstered loveseat sofa (twice as wide as tall); "
            "padded backrest along the TOP edge, rounded armrests left and right, "
            "two deep seat cushions facing down."
        ),
        variant_directives=(
            "Warm terracotta-rust woven fabric with neat piped seams.",
            "Charcoal-grey modern tweed loveseat with a small mustard accent cushion.",
            "Soft oatmeal cream boucle two-seater sofa with exposed oak corner trim.",
        ),
    ),
    PropSpriteSpec(
        id="armchair",
        prop_kind_code="ARMCHAIR",
        filename="armchair.webp",
        label="Armchair",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square upholstered reading armchair; padded backrest along the TOP edge, "
            "sturdy armrests left and right, square seat cushion facing down."
        ),
        variant_directives=(
            "Tufted camel-tan leather club chair.",
            "Deep forest-green velvet wingback armchair with brass tack trim.",
            "Cool heather-grey mid-century fabric armchair with warm walnut wood arm caps.",
        ),
    ),
    PropSpriteSpec(
        id="coffee_table",
        prop_kind_code="COFFEE_TABLE",
        filename="coffee_table.webp",
        label="Coffee Table",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal rectangular living-room coffee table (twice as wide as tall) "
            "seen straight down from above; clean wooden or stone tabletop with subtle "
            "coasters, art book, or small tray."
        ),
        variant_directives=(
            "Warm honey-oak timber coffee table with two magazines and ceramic mug.",
            "Smoked glass and matte-black steel rectangular coffee table with small succulent dish.",
            "Dark walnut slab coffee table with cream stone tray and brass coaster.",
        ),
    ),
    PropSpriteSpec(
        id="rug",
        prop_kind_code="RUG",
        filename="rug.webp",
        label="Area Rug",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Rectangular woven living-room area rug (3:2 landscape) lying completely "
            "flat seen straight down from above; rich woven textile border and fine "
            "short fringed edges left and right."
        ),
        variant_directives=(
            "Faded burgundy and navy Persian medallion woven rug.",
            "Warm cream and charcoal geometric Berber shag area rug.",
            "Earthy sage-green and ochre Southwestern flatweave dhurrie carpet.",
        ),
    ),
    PropSpriteSpec(
        id="runner_rug",
        prop_kind_code="RUNNER_RUG",
        filename="runner_rug.webp",
        label="Hallway / Bedside Runner Rug",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal rectangular hallway runner rug (twice as wide as tall) lying "
            "flat seen straight down; patterned border and finely stitched ends."
        ),
        variant_directives=(
            "Deep crimson and indigo Anatolian runner carpet with diamond motifs.",
            "Natural woven jute and slate striped modern hallway runner.",
            "Muted teal and sand floral-border woven bedside runner.",
        ),
    ),
    PropSpriteSpec(
        id="round_rug",
        prop_kind_code="ROUND_RUG",
        filename="round_rug.webp",
        label="Circular Accent Rug",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Circular woven floor rug viewed strictly straight down on a square canvas; "
            "concentric decorative weave pattern and clean circular perimeter."
        ),
        variant_directives=(
            "Braided warm wheat and terracotta concentric farmhouse rug.",
            "Deep navy and gold radial mandala woven wool carpet.",
            "Soft dove-grey and ivory textured circular bouclé rug.",
        ),
    ),
    PropSpriteSpec(
        id="dining_table",
        prop_kind_code="DINING_TABLE",
        filename="dining_table.webp",
        label="Dining Table & Chairs",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square/rectangular wooden dining table with four wooden dining chairs "
            "tucked cleanly around all four sides (top, bottom, left, right), seen "
            "straight down from above; subtle table runner or centerpiece."
        ),
        variant_directives=(
            "Warm oak table with linen runner, ceramic bowl, and four dark upholstered chairs.",
            "Dark walnut farmhouse table set with four white placemats and four spindle-back chairs.",
            "Sleek matte-white and birch modern dining set with teapot and four slate chairs.",
        ),
    ),
    PropSpriteSpec(
        id="round_table",
        prop_kind_code="ROUND_TABLE",
        filename="round_table.webp",
        label="Bistro Table & Chairs",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Small circular bistro / cafe table centered on a square canvas with two "
            "or three compact curved chairs tucked neatly around it, seen straight down."
        ),
        variant_directives=(
            "Round marble-top bistro table with brass rim, coffee cup, and two dark bentwood chairs.",
            "Warm oak pub table with two amber pint glasses and two black steel bar-style chairs.",
            "Painted sage-green café table with small vase and two woven wicker chairs.",
        ),
    ),
    PropSpriteSpec(
        id="bed",
        prop_kind_code="BED",
        filename="bed.webp",
        label="Double Bed",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square double / queen residential bed seen straight down; wooden headboard "
            "and two fluffy sleeping pillows along the TOP edge, neatly layered duvet "
            "and folded throw blanket extending toward the bottom."
        ),
        variant_directives=(
            "Walnut headboard with crisp white sheets, slate-blue duvet, and knitted cream throw.",
            "Light oak frame with warm terracotta linen comforter and four layered pillows.",
            "Upholstered charcoal headboard with sage-green quilted coverlet slightly rumpled on one side.",
        ),
    ),
    PropSpriteSpec(
        id="single_bed",
        prop_kind_code="SINGLE_BED",
        filename="single_bed.webp",
        label="Single / Twin Bed",
        base_w_cells=1,
        base_h_cells=2,
        target_w_px=256,
        target_h_px=512,
        aspect_ratio="9:16",
        prompt=prop_prompt(
            "Vertical single / twin bed (twice as tall as wide) seen straight down; "
            "wooden headboard and single pillow at the TOP edge, tucked comforter "
            "extending down to the foot."
        ),
        variant_directives=(
            "Natural pine headboard with crisp white pillow and navy plaid blanket.",
            "Matte-black iron bed frame with heather-grey duvet and mustard cushion.",
            "Painted white cottage twin bed with soft sage floral quilt.",
        ),
    ),
    PropSpriteSpec(
        id="bookshelf",
        prop_kind_code="BOOKSHELF",
        filename="bookshelf.webp",
        label="Bookshelf",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal wooden bookcase top surface (twice as wide as tall) seen "
            "strictly straight down from above; solid wood top shelf rim with neat "
            "stacks of hardcover books, brass bookends, and a small potted trailing plant."
        ),
        variant_directives=(
            "Warm walnut bookcase top with stacked vintage books and small brass lamp.",
            "Light oak shelving top with neat binders, storage baskets, and framed photo.",
            "Matte-white painted bookcase top with colorful paperbacks and terracotta succulent.",
        ),
    ),
    PropSpriteSpec(
        id="wardrobe",
        prop_kind_code="WARDROBE",
        filename="wardrobe.webp",
        label="Wardrobe",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal tall wooden bedroom wardrobe / armoire seen strictly from 90 "
            "degrees directly overhead (twice as wide as tall); molded cornice top "
            "panel, subtle wood grain, and a neatly stored woven basket on top."
        ),
        variant_directives=(
            "Rich cherry-mahogany armoire crown with brass corner trim.",
            "Warm light-oak Scandinavian wardrobe top with two folded linen storage boxes.",
            "Antique painted charcoal-grey wardrobe cornice with carved edge molding.",
        ),
    ),
    PropSpriteSpec(
        id="nightstand",
        prop_kind_code="NIGHTSTAND",
        filename="nightstand.webp",
        label="Bedside Nightstand",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square wooden bedside nightstand seen straight down from above; small "
            "warm-lit circular lamp shade, paperback book, and ceramic water glass "
            "on top."
        ),
        variant_directives=(
            "Walnut nightstand with cream fabric shade lamp, book, and glass.",
            "Light birch nightstand with matte-black reading lamp, alarm clock, and eyeglasses.",
            "Painted sage-blue end table with brass dome lamp and coaster.",
        ),
    ),
    PropSpriteSpec(
        id="dresser",
        prop_kind_code="DRESSER",
        filename="dresser.webp",
        label="Bedroom Dresser",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal wooden bedroom chest of drawers / dresser seen straight down "
            "(twice as wide as tall); smooth timber top surface with bevelled trim, "
            "small jewelry tray, folded scarf, and mirror-frame edge along the TOP back."
        ),
        variant_directives=(
            "Warm oak dresser with leather valet tray, comb, and linen runner.",
            "Dark espresso wood chest of drawers with small ceramic dish and folded sweater.",
            "Vintage painted ivory dresser with brass mirror rail along top edge and perfume bottle.",
        ),
    ),
    PropSpriteSpec(
        id="tv_stand",
        prop_kind_code="TV_STAND",
        filename="tv_stand.webp",
        label="Entertainment Stand & TV",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal living-room media console and flat-screen TV seen straight "
            "down from above (twice as wide as tall); low timber credenza cabinet "
            "with ultra-thin black flatscreen TV panel centered along the TOP back, "
            "compact soundbar and black game console in front."
        ),
        variant_directives=(
            "Walnut credenza with sleek black 4K TV, soundbar, and two remotes.",
            "Matte-black steel-and-glass media stand with curved display, speaker dock, and controller.",
            "Light Scandinavian ash console with thin white-bezel screen and stacked vinyl sleeves.",
        ),
    ),
    # --- Batch 4: Kitchen, Bath & Bar (17) ---
    PropSpriteSpec(
        id="kitchen_counter",
        prop_kind_code="KITCHEN_COUNTER",
        filename="kitchen_counter.webp",
        label="Kitchen Counter",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square modular kitchen countertop section seen straight down; polished "
            "stone counter with brushed stainless steel recessed sink & faucet along "
            "the TOP edge, wooden cutting board and knife block beside it."
        ),
        variant_directives=(
            "White Carrara marble counter with stainless sink and oak cutting board.",
            "Dark charcoal slate granite counter with espresso maker, mugs, and utensil crock.",
            "Warm butcher-block oak countertop with ceramic prep bowls and folded dish towel.",
        ),
    ),
    PropSpriteSpec(
        id="stove",
        prop_kind_code="STOVE",
        filename="stove.webp",
        label="Stove / Range",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square kitchen stove / range cooktop seen straight down; brushed "
            "stainless steel chassis with four cast-iron burner grates, control strip "
            "along TOP edge, and a stainless skillet or copper stockpot on one burner."
        ),
        variant_directives=(
            "Stainless 4-burner gas range with black grates and cast-iron skillet.",
            "Black ceramic induction glass cooktop with sleek glowing red ring and stainless saucepan.",
            "Heavy commercial 4-burner steel range with copper soup pot and wooden spoon.",
        ),
    ),
    PropSpriteSpec(
        id="fridge",
        prop_kind_code="FRIDGE",
        filename="fridge.webp",
        label="Refrigerator",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down kitchen refrigerator roof surface seen straight down; "
            "brushed steel or enamel top panel with subtle rear hinge plates, cooling "
            "vent slots along TOP edge, and recessed front door lip at bottom."
        ),
        variant_directives=(
            "Brushed stainless steel refrigerator top with clean metallic bevel.",
            "Matte-white retro curved-edge fridge roof with small woven breadbasket stored on top.",
            "Dark graphite-black commercial cooler top with digital thermostat strip along top edge.",
        ),
    ),
    PropSpriteSpec(
        id="bar_counter",
        prop_kind_code="BAR_COUNTER",
        filename="bar_counter.webp",
        label="Bar Counter",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Single solid 1x1 straight modular tavern / lounge bar counter slab filling "
            "the square footprint (tileable horizontally, NOT a U-shaped box), seen "
            "strictly straight down; polished dark timber or stone bar top with brass "
            "foot/drip rail along BOTTOM edge, stainless service mat, and pint glasses."
        ),
        variant_directives=(
            "Polished dark mahogany straight bar slab with brass draft tap tower and coasters.",
            "Weathered reclaimed oak straight tavern bar slab with black rubber bar mat, cocktail shaker, and amber pint glass.",
            "Sleek black marble and brushed-steel straight cocktail bar slab with chrome speed-rail along top and martini glasses.",
        ),
    ),
    PropSpriteSpec(
        id="back_bar_shelf",
        prop_kind_code="BACK_BAR_SHELF",
        filename="back_bar_shelf.webp",
        label="Back-Bar Bottle Shelf",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square wooden back-bar liquor cabinet & bottle counter seen straight down; "
            "dark timber top lined neatly with multi-colored glass liquor bottles "
            "(amber, emerald, cobalt, clear) and stacked clean glassware."
        ),
        variant_directives=(
            "Mahogany back-bar shelf packed with amber whiskey and green gin bottles.",
            "Dark steel-and-oak bar credenza holding rows of craft beer cans, wine bottles, and ice bucket.",
            "Warm walnut apothecary/cocktail shelf with tinted glass decanters and copper mugs.",
        ),
    ),
    PropSpriteSpec(
        id="bar_stool",
        prop_kind_code="BAR_STOOL",
        filename="bar_stool.webp",
        label="Bar Stool",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Circular top-down bar stool centered on a square canvas; padded round "
            "seat cushion with subtle ring stitching and steel footrest ring peeking "
            "around base."
        ),
        variant_directives=(
            "Oxblood-red vinyl padded chrome bar stool.",
            "Turned dark walnut wood saddle-seat tavern stool.",
            "Matte-black steel industrial swivel stool with cognac leather seat pad.",
        ),
    ),
    PropSpriteSpec(
        id="pool_table",
        prop_kind_code="POOL_TABLE",
        filename="pool_table.webp",
        label="Pool Table",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Horizontal 3:2 billiard / pool table seen strictly straight down; rich "
            "wooden rails with six corner/side leather pockets, crisp felt playing "
            "surface, racked triangle of colorful billiard balls, cue ball, and two "
            "wooden cues resting along one rail."
        ),
        variant_directives=(
            "Classic emerald-green felt with mahogany rails and neat triangle rack.",
            "Deep burgundy tournament felt with dark walnut rails and scattered mid-game balls.",
            "Royal navy-blue felt with brushed steel corner plates and chalk cubes on rail.",
        ),
    ),
    PropSpriteSpec(
        id="jukebox",
        prop_kind_code="JUKEBOX",
        filename="jukebox.webp",
        label="Jukebox",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down vintage neon jukebox / music console roof dome seen "
            "strictly from 90 degrees directly overhead (NO upright front face); "
            "curved walnut and brushed-chrome arched top cap with glowing amber-and-crimson "
            "neon light rim along the bottom front edge and speaker vent slots."
        ),
        variant_directives=(
            "Classic walnut-and-chrome jukebox roof cap with warm amber and ruby neon arch.",
            "1980s matte-black and electric-cyan digital wall-box jukebox top.",
            "Retro cream-and-turquoise chrome diner jukebox curved roof dome ONLY (strictly overhead 90-degree view of top dome, NO upright front face).",
        ),
    ),
    PropSpriteSpec(
        id="booth",
        prop_kind_code="BOOTH",
        filename="booth.webp",
        label="Booth Seating",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square restaurant / diner booth dining set seen straight down; two "
            "parallel upholstered bench seats along TOP and BOTTOM edges facing a "
            "central laminate/wood dining table with menus and condiment caddy."
        ),
        variant_directives=(
            "Crimson-red ribbed vinyl diner benches with cream speckled table.",
            "Tufted dark forest-green leather pub booth with warm oak table and pint glasses.",
            "Muted mustard-tan modern café booth seating with pale birch tabletop.",
        ),
    ),
    PropSpriteSpec(
        id="bathtub",
        prop_kind_code="BATHTUB",
        filename="bathtub.webp",
        label="Bathtub",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal porcelain bathroom bathtub seen straight down (twice as wide "
            "as tall); glossy white glazed porcelain oval basin, chrome drain and "
            "faucet fixtures on the LEFT end, folded bath mat or sponge on rim."
        ),
        variant_directives=(
            "Crisp white porcelain built-in alcove tub with chrome fixtures.",
            "White tub filled with translucent pale-aqua bathwater and soap tray on rim.",
            "Vintage clawfoot-rim porcelain tub with brass faucet and folded sage towel across corner.",
        ),
    ),
    PropSpriteSpec(
        id="toilet",
        prop_kind_code="TOILET",
        filename="toilet.webp",
        label="Toilet",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down ceramic bathroom toilet seen straight down; rectangular "
            "porcelain tank along the TOP edge with chrome flush lever/button, smooth "
            "oval closed lid extending downward."
        ),
        variant_directives=(
            "Glossy white glazed porcelain toilet with closed lid and chrome flush button.",
            "Off-white commercial tankless flush-valve toilet with matte-black seat.",
            "Clean modern square-tank ceramic toilet with pale-grey bath mat lip.",
        ),
    ),
    PropSpriteSpec(
        id="bathroom_sink",
        prop_kind_code="BATHROOM_SINK",
        filename="bathroom_sink.webp",
        label="Bathroom Vanity & Sink",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square bathroom vanity cabinet & sink seen straight down; stone/ceramic "
            "countertop with recessed white porcelain oval basin, chrome faucet at "
            "TOP edge, liquid soap dispenser and folded hand towel."
        ),
        variant_directives=(
            "White marble vanity with oval porcelain basin and chrome faucet.",
            "Warm oak vanity top with round ceramic vessel sink and amber soap bottle.",
            "Commercial stainless-and-white clinic restroom sink with sensor faucet and paper towel pack.",
        ),
    ),
    PropSpriteSpec(
        id="shower_stall",
        prop_kind_code="SHOWER_STALL",
        filename="shower_stall.webp",
        label="Walk-in Glass Shower Stall",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square walk-in tiled shower stall seen straight down; light ceramic/mosaic "
            "shower pan with centered chrome floor drain, corner shampoo caddy, and "
            "solid brushed aluminum & pale ice-blue opaque curb rim around the outer "
            "edges (strictly NO transparent magenta/pink reflection on the rim)."
        ),
        variant_directives=(
            "Clean white hex-tile shower pan with chrome drain and solid brushed-silver aluminum outer rim.",
            "Warm slate-grey pebble mosaic shower floor with brass drain, corner teak mat, and matte-black aluminum outer frame.",
            "Seafoam ceramic tile shower stall with subtle water droplets and stainless drain.",
        ),
    ),
    PropSpriteSpec(
        id="spa_bath",
        prop_kind_code="SPA_BATH",
        filename="spa_bath.webp",
        label="Jetted Whirlpool Spa Bath",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square built-in jetted whirlpool spa bath seen straight down; broad "
            "stone or acrylic deck surround, deep sculpted soaking basin filled with "
            "rippling pale turquoise water, chrome hydro-jets and padded headrests."
        ),
        variant_directives=(
            "White acrylic corner whirlpool tub with pale-aqua water and chrome jets.",
            "Warm travertine stone surround spa bath with clear turquoise water and folded towels.",
            "Dark slate-stone luxury soaking tub with soft cyan LED underwater glow.",
        ),
    ),
    PropSpriteSpec(
        id="linen_closet",
        prop_kind_code="LINEN_CLOSET",
        filename="linen_closet.webp",
        label="Built-in Towel & Linen Cabinet",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square wooden bathroom linen & towel storage cabinet seen straight down "
            "from above; clean cabinet top panel with neatly folded stack of plush "
            "towels and a small wicker toiletry basket."
        ),
        variant_directives=(
            "Painted white wood cabinet top with folded seafoam towels and wicker basket.",
            "Warm honey-oak linen cabinet with folded cream bath sheets and glass jar.",
            "Light grey clinical supply cupboard top with stacked white towels and sanitizer bottle.",
        ),
    ),
    PropSpriteSpec(
        id="washer_dryer",
        prop_kind_code="WASHER_DRYER",
        filename="washer_dryer.webp",
        label="Laundry Washer & Dryer Pair",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal side-by-side washer and dryer laundry pair seen straight down "
            "(twice as wide as tall); two square appliance tops side-by-side with "
            "control dial strips along the TOP back, detergent bottle and plastic "
            "laundry basket with folded clothes on top."
        ),
        variant_directives=(
            "Clean white washer & dryer pair with blue detergent jug and white basket.",
            "Brushed metallic-graphite front-loader pair with red-and-white laundry basket.",
            "Commercial stainless-steel clinic laundry pair with stacked folded blue scrubs.",
        ),
    ),
    PropSpriteSpec(
        id="stereo",
        prop_kind_code="STEREO",
        filename="stereo.webp",
        label="Stereo System & Speakers",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down home hi-fi stereo console flanked by two compact "
            "wooden bookshelf speakers; brushed silver turntable / amplifier receiver "
            "in the center with a spinning black vinyl record on top."
        ),
        variant_directives=(
            "Walnut hi-fi cabinet with black turntable vinyl platter and matching wood speakers.",
            "Matte-black 1990s rack stereo deck with dual speaker towers and CD jewel cases.",
            "Modern brushed-aluminum receiver with glowing blue dial strip and white studio monitors.",
        ),
    ),
    # --- Batch 5: Office, Clutter, Clinical & Vertical Portals (16) ---
    PropSpriteSpec(
        id="desk_l",
        prop_kind_code="DESK_L",
        filename="desk_l.webp",
        label="L-Shaped Corner Desk",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square L-shaped executive corner office desk seen straight down; wooden "
            "desktop wrapping along TOP and LEFT edges with open inner bay at bottom-right, "
            "dual monitors, keyboard, folders, desk lamp, and a dark rolling swivel "
            "chair tucked inside the L."
        ),
        variant_directives=(
            "Warm oak L-desk with dual dark monitors, neat papers, and charcoal chair.",
            "Matte-white and steel modern workstation L-desk with ultrawide monitor and slate chair.",
            "Rich walnut executive L-desk with leather blotter, brass desk lamp, laptop, and tan chair.",
        ),
    ),
    PropSpriteSpec(
        id="desk_t",
        prop_kind_code="DESK_T",
        filename="desk_t.webp",
        label="T-Shaped Workstation Desk",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Wide T-shaped / U-shaped dual-wing workstation desk (3:2 landscape) seen "
            "straight down; broad wooden/laminate desk span along TOP with a central "
            "peninsula wing extending down, dual monitors, keyboard, binders, and a "
            "swivel office chair tucked alongside."
        ),
        variant_directives=(
            "Light birch T-workstation with dual monitors, blueprints, and black task chair.",
            "Dark slate-grey tactical dispatch console desk with three monitors and radio handset.",
            "Warm walnut conference-peninsula desk with laptop, coffee mug, and two dark chairs.",
        ),
    ),
    PropSpriteSpec(
        id="desk_lamp",
        prop_kind_code="DESK_LAMP",
        filename="desk_lamp.webp",
        label="Desk Lamp",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Compact bedside or tabletop task lamp seen straight down from above on "
            "a square canvas; small weighted circular base and solid opaque domed or "
            "conical lamp shade with ZERO outer glow halo or light bloom onto the background."
        ),
        variant_directives=(
            "Solid opaque brushed-brass banker's lamp with deep emerald-green glass shade (NO outer pink/magenta glow halo).",
            "Sleek matte-black architect swing-arm desk lamp with warm cream bulb glow.",
            "Small cream linen drum-shade table lamp on pale ceramic base.",
        ),
    ),
    PropSpriteSpec(
        id="standing_lamp",
        prop_kind_code="STANDING_LAMP",
        filename="standing_lamp.webp",
        label="Standing Floor Lamp",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Circular top-down standing floor lamp seen strictly straight down; wide "
            "fabric or frosted-glass drum shade glowing softly warm amber-white in "
            "the center with subtle brass/steel finial cap."
        ),
        variant_directives=(
            "Warm ivory linen cylinder floor-lamp shade with brushed brass finial.",
            "Modern frosted-glass torchiere bowl floor lamp with dark bronze outer rim.",
            "Mid-century pleated cream shade floor lamp with walnut tripod edge peeking out.",
        ),
    ),
    PropSpriteSpec(
        id="desk_clutter",
        prop_kind_code="DESK_CLUTTER",
        filename="desk_clutter.webp",
        label="Scattered Desk Papers & Clutter",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Small compact cluster of tabletop office/clinic clutter seen straight "
            "down: slightly skewed manila file folder, white printed report sheets, "
            "ceramic coffee mug, ballpoint pen, and small digital tablet."
        ),
        variant_directives=(
            "Manila chart folder, scattered forms, black mug, and blue pen.",
            "Medical clipboard with lab printouts, silver stethoscope coiled beside a tablet, and paper cup.",
            "Open hardcover notebook, brass compass/ruler, sticky notes, and ceramic teacup.",
        ),
    ),
    PropSpriteSpec(
        id="filing_cabinet",
        prop_kind_code="FILING_CABINET",
        filename="filing_cabinet.webp",
        label="Steel Office Filing Cabinet",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down vertical steel office filing cabinet seen straight down; "
            "painted sheet-steel rectangular top surface with faint recessed seam "
            "lines and a neat stack of manila folders or wire inbox tray on top."
        ),
        variant_directives=(
            "Cool industrial grey steel cabinet with wire letter tray and manila files.",
            "Weathered putty-beige 4-drawer steel file cabinet with red ledger binder on top.",
            "Matte dark-charcoal security filing cabinet with combination dial lock bar and blue binder.",
        ),
    ),
    PropSpriteSpec(
        id="water_cooler",
        prop_kind_code="WATER_COOLER",
        filename="water_cooler.webp",
        label="Corridor / Office Water Cooler",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down office water cooler dispenser seen straight down; white "
            "square tower base with circular translucent sky-blue 5-gallon water jug "
            "centered on top and recessed front cup-drip grate at bottom."
        ),
        variant_directives=(
            "White water cooler with translucent pale-blue 5-gallon water bottle.",
            "Brushed-steel and black bottleless office cooler with drip tray and stacked paper cone cups.",
            "Clean light-grey clinic hydration tower with clear aqua water jug and blue spigot lip.",
        ),
    ),
    PropSpriteSpec(
        id="fire_extinguisher",
        prop_kind_code="FIRE_EXTINGUISHER",
        filename="fire_extinguisher.webp",
        label="Fire Extinguisher",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Top-down wall/floor-stand crimson-red fire extinguisher cylinder centered "
            "on a small square stand plate seen straight down; bright red glossy steel "
            "tank top, brass valve assembly, black squeeze handle, and coiled black "
            "discharge horn."
        ),
        variant_directives=(
            "Bright crimson ABC dry-chem fire extinguisher with black rubber horn.",
            "Red CO2 extinguisher with wide flared black funnel nozzle on steel bracket plate.",
            "Wall-mounted red-and-silver pressurized water/foam extinguisher on dark base.",
        ),
    ),
    PropSpriteSpec(
        id="trash_can",
        prop_kind_code="TRASH_CAN",
        filename="trash_can.webp",
        label="Trash Bin",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Top-down indoor trash/waste bin centered on a square canvas seen straight "
            "down; compact cylindrical or rectangular bin with rimmed plastic liner "
            "or swing-top lid."
        ),
        variant_directives=(
            "Brushed stainless steel round step-on trash can with closed domed lid.",
            "Matte dark-grey rectangular office wastebasket lined with folded black trash bag.",
            "Cobalt-blue recycling bin with slotted lid beside small paper scraps.",
        ),
    ),
    PropSpriteSpec(
        id="floor_clothes",
        prop_kind_code="FLOOR_CLOTHES",
        filename="floor_clothes.webp",
        label="Scattered Clothes on Floor",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Compact natural cluster of discarded/folded clothes on the floor seen "
            "straight down: rumpled denim jacket or jeans, folded grey t-shirt, and "
            "pair of canvas sneakers."
        ),
        variant_directives=(
            "Crumpled blue denim jeans, olive t-shirt, and worn sneakers.",
            "Folded dark navy hoodie, khaki cargo pants, and wool socks.",
            "Discarded pale-blue medical scrubs top, white lab coat, and dark slip-on clogs.",
        ),
    ),
    PropSpriteSpec(
        id="floor_toys",
        prop_kind_code="FLOOR_TOYS",
        filename="floor_toys.webp",
        label="Scattered Toys on Floor",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Compact cluster of children's toys scattered on the floor seen straight "
            "down: primary red, yellow, sky-blue, and forest-green wooden building "
            "blocks, small wooden toy train car, and bright orange/white rubber ball "
            "(strictly NO pink or purple pieces)."
        ),
        variant_directives=(
            "Red, blue, yellow, and green wooden blocks with toy wooden train.",
            "Small yellow dump truck, primary-color stacking rings, and striped beach ball.",
            "Opened storybook, green toy dinosaur, and scattered wooden puzzle pieces.",
        ),
    ),
    PropSpriteSpec(
        id="medical_counter",
        prop_kind_code="MEDICAL_COUNTER",
        filename="medical_counter.webp",
        label="Medical Counter & Scrub Sink",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square clinical medical cabinetry countertop & scrub sink seen straight "
            "down; seamless off-white/pale-grey solid-surface lab counter with "
            "recessed stainless steel scrub basin and gooseneck faucet along TOP "
            "right, glove dispenser box and sanitizer pump on left."
        ),
        variant_directives=(
            "Off-white solid-surface counter with stainless sink and blue glove box.",
            "Light cool-grey stainless medical counter with double scrub sink and gauze tray.",
            "Clean cream-quartz clinic prep station with autoclave tray, alcohol bottles, and sink.",
        ),
    ),
    PropSpriteSpec(
        id="privacy_curtain",
        prop_kind_code="PRIVACY_CURTAIN",
        filename="privacy_curtain.webp",
        label="Medical Privacy Curtain",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Strictly 90-degree overhead architectural top-down cut view of a thin "
            "horizontal hospital cubicle privacy curtain track (NO side drape view); "
            "thin brushed aluminum ceiling rail running straight left-to-right with "
            "a narrow wavy S-folded fabric ribbon seen edge-on from directly above."
        ),
        variant_directives=(
            "Thin overhead wavy S-folded pale seafoam-aqua fabric curtain ribbon along brushed aluminum ceiling track (seen strictly from directly above).",
            "Soft sky-blue antimicrobial cubicle curtain with subtle woven stripe pattern.",
            "Warm cream-beige pleated medical screen curtain partially bunched toward the left end.",
        ),
    ),
    PropSpriteSpec(
        id="stairwell",
        prop_kind_code="STAIRWELL",
        filename="stairwell.webp",
        label="Stairwell",
        base_w_cells=2,
        base_h_cells=3,
        target_w_px=384,
        target_h_px=576,
        aspect_ratio="2:3",
        prompt=prop_prompt(
            "Vertical 2:3 architectural concrete and steel stairwell flight seen "
            "strictly straight down from above; brushed concrete stair treads with "
            "dark rubberized anti-slip nosing strips ascending upward, brushed steel "
            "handrails along left and right edges, and flat landing at top."
        ),
        variant_directives=(
            "Clean brushed architectural concrete treads with steel handrails.",
            "Industrial painted grey-and-yellow hazard-nosing concrete clinic fire stairs.",
            "Warm terrazzo-and-oak interior staircase with matte-black iron railings.",
        ),
    ),
    PropSpriteSpec(
        id="elevator_car",
        prop_kind_code="ELEVATOR_CAR",
        filename="elevator_car.webp",
        label="Passenger Elevator",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square architectural cut-away passenger elevator cab seen straight down "
            "from above; brushed steel frame walls along TOP, LEFT, and RIGHT, ribbed "
            "charcoal safety floor tile, stainless handrails, and sliding stainless "
            "cab doors along BOTTOM edge."
        ),
        variant_directives=(
            "Brushed steel interior cab with dark ribbed rubber tile floor.",
            "Warm oak-paneled and brass passenger lift with polished stone diamond-tile floor.",
            "Clean hospital-grade matte stainless elevator cab with pale-grey vinyl floor and LED rim.",
        ),
    ),
    PropSpriteSpec(
        id="service_elevator",
        prop_kind_code="SERVICE_ELEVATOR",
        filename="service_elevator.webp",
        label="Hospital Gurney / Freight Elevator",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Wide 3:2 architectural cut-away hospital gurney / freight elevator cab "
            "seen straight down; heavy steel bumper walls along TOP, LEFT, and RIGHT, "
            "diamond-plate grey anti-slip floor, yellow safety threshold bar along "
            "BOTTOM bi-parting steel doors."
        ),
        variant_directives=(
            "Heavy brushed steel gurney lift with grey diamond-plate deck and yellow threshold.",
            "Clinical cleanroom service elevator with white-enamel walls, blue bumper rails, and vinyl floor.",
            "Industrial freight elevator with olive-steel cage walls and caution-striped door sill.",
        ),
    ),
)
