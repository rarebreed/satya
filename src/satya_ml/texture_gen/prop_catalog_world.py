"""Batches 6–7 prop sprite specs: labs/retail/industrial and outdoor/vehicles/infra."""

from __future__ import annotations

from satya_ml.texture_gen.prop_catalog_types import PropSpriteSpec, prop_prompt

BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS: tuple[str, ...] = (
    "soldering_station",
    "test_bench",
    "printer3d_station",
    "air_compressor",
    "cnc_mill",
    "server_rack",
    "display_rack",
    "checkout_counter",
    "mall_kiosk",
    "conveyor_belt",
    "industrial_press",
    "pallet_rack",
    "forklift",
    "shipping_container",
    "wood_crate",
    "drum_cluster",
    "structural_pillar",
    "elevated_catwalk",
)

BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS: tuple[str, ...] = (
    "car",
    "jersey_barrier",
    "park_bench",
    "picnic_table",
    "planter",
    "planter_table",
    "crop_row",
    "hedge",
    "bush",
    "boulder",
    "fallen_log",
    "small_fountain",
    "medium_fountain",
    "large_fountain",
    "bbq_grill",
    "garden_shed",
    "hot_tub",
    "garbage_pile",
    "street_debris",
    "brownstone_stoop",
    "security_camera_mast",
    "drone_dock_pad",
    "hv_transformer",
)

WORLD_PROP_SPECS: tuple[PropSpriteSpec, ...] = (
    # --- Batch 6: Lab, Retail & Industrial (18) ---
    PropSpriteSpec(
        id="soldering_station",
        prop_kind_code="SOLDERING_STATION",
        filename="soldering_station.webp",
        label="Electronics Soldering & Rework Bench",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal electronics soldering and PCB rework bench seen straight down "
            "(twice as wide as tall); blue anti-static ESD soldering mat centered on "
            "dark steel bench, green circuit boards, digital soldering station unit, "
            "copper wire spools, and fume extractor arm."
        ),
        variant_directives=(
            "Blue ESD mat with green PCBs, soldering iron stand, and solder spools.",
            "Dark charcoal workbench with hot-air rework station, brass magnifying ring light, and component bins.",
            "Teal repair mat with multimeter, oscilloscope probes, ribbon cables, and precision screwdriver tray.",
        ),
    ),
    PropSpriteSpec(
        id="test_bench",
        prop_kind_code="TEST_BENCH",
        filename="test_bench.webp",
        label="Oscilloscope & Test Rig Bench",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal electronic diagnostics & oscilloscope test bench seen straight "
            "down (twice as wide as tall); matte slate steel tabletop holding a crisp "
            "oscilloscope chassis, benchtop power supply, coiled BNC coax leads, and "
            "ruggedized test laptop."
        ),
        variant_directives=(
            "Slate bench with oscilloscope, benchtop PSU, and coiled test leads.",
            "Grey engineering rig with spectrum analyzer, logic probe pod, and open schematic binder.",
            "Dark 2035 avionics bench with dual diagnostic screens, copper RF shield box, and waveform meter.",
        ),
    ),
    PropSpriteSpec(
        id="printer3d_station",
        prop_kind_code="PRINTER3D_STATION",
        filename="printer3d_station.webp",
        label="3D Printer & Curing Station",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal 3D printing & fabrication bench seen straight down (twice as "
            "wide as tall); dark steel workbench with enclosed CoreXY 3D printer on "
            "left showing top gantry & filament spool, paired with UV resin curing "
            "chamber and tool tray on right."
        ),
        variant_directives=(
            "CoreXY filament 3D printer with orange/blue spools left and UV wash/cure box right.",
            "Dual-extruder industrial printer enclosure with white resin prints and scraper tray.",
            "Sleek matte-black carbon-fiber printer station with glowing cyan build-chamber window.",
        ),
    ),
    PropSpriteSpec(
        id="air_compressor",
        prop_kind_code="AIR_COMPRESSOR",
        filename="air_compressor.webp",
        label="Lab / Shop Air Compressor",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down workshop / clinic pneumatic air compressor seen straight "
            "down; glossy painted horizontal cylindrical steel pressure tank, electric "
            "motor head with cooling fins, dual pressure gauges, and coiled orange "
            "air hose."
        ),
        variant_directives=(
            "Industrial crimson-red horizontal tank compressor with coiled orange hose.",
            "Safety-yellow twin-tank portable shop compressor with black belt guard.",
            "Matte slate-blue quiet medical scroll air compressor on rubber caster frame.",
        ),
    ),
    PropSpriteSpec(
        id="cnc_mill",
        prop_kind_code="CNC_MILL",
        filename="cnc_mill.webp",
        label="Enclosed CNC Milling Machine",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Wide 3:2 enclosed industrial CNC milling machine seen strictly straight "
            "down from above; heavy painted sheet-steel machine housing with top "
            "skyline viewing window showing aluminum T-slot bed and spindle head, "
            "coolant tank, and swinging side operator control console (NO outer glow)."
        ),
        variant_directives=(
            "Light industrial grey-and-blue Haas-style CNC enclosure with top window.",
            "Weathered dark-slate and safety-orange 5-axis precision mill with chip conveyor.",
            "Clean off-white medical-prototyping micro-CNC unit with crisp small unlit status cap (NO outer bloom).",
        ),
    ),
    PropSpriteSpec(
        id="server_rack",
        prop_kind_code="SERVER_RACK",
        filename="server_rack.webp",
        label="42U Compute & Network Server Rack",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down 42U data center server rack cabinet roof seen straight "
            "down; matte-black perforated steel top panel with circular exhaust fan "
            "grilles, overhead fiber cable tray trough, and subtle green/blue status LEDs."
        ),
        variant_directives=(
            "Matte-black perforated 42U rack roof with twin top exhaust fans and blue fiber conduit.",
            "Dark gunmetal network rack with yellow fiber raceway trough and copper ground lug.",
            "Industrial slate enterprise server cabinet with brushed steel cable chimney.",
        ),
    ),
    PropSpriteSpec(
        id="display_rack",
        prop_kind_code="DISPLAY_RACK",
        filename="display_rack.webp",
        label="Retail Display Gondola",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal retail / pharmacy store display gondola shelf seen straight "
            "down (twice as wide as tall); painted steel double-sided rack top packed "
            "neatly with colorful packaged goods, boxed supplies, and central spine divider."
        ),
        variant_directives=(
            "Light grey pharmacy gondola stocked with colorful medicine and snack boxes.",
            "Warm oak-and-steel market display rack filled with bottled drinks and canned goods.",
            "Matte-black electronics retail display shelf with boxed hardware and cable packs.",
        ),
    ),
    PropSpriteSpec(
        id="checkout_counter",
        prop_kind_code="CHECKOUT_COUNTER",
        filename="checkout_counter.webp",
        label="Checkout Counter",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal store checkout counter & POS register seen straight down "
            "(twice as wide as tall); laminate/wood counter with black conveyor belt "
            "section on left, touchscreen POS register, barcode scanner, and bagging "
            "rack on right."
        ),
        variant_directives=(
            "Light oak counter with black checkout belt, touchscreen POS, and paper bags.",
            "Stainless steel & slate convenience checkout counter with card reader and acrylic guard.",
            "Warm painted teal-and-cream pharmacy counter with cash drawer, monitor, and basket.",
        ),
    ),
    PropSpriteSpec(
        id="mall_kiosk",
        prop_kind_code="MALL_KIOSK",
        filename="mall_kiosk.webp",
        label="Mall Kiosk Island",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Wide 3:2 enclosed retail mall kiosk island counter seen straight down "
            "from above; rectangular four-sided display counter island with open inner "
            "clerk pit, glass display cases, POS tablet, and compact stool inside."
        ),
        variant_directives=(
            "Modern light-birch and glass tech/phone kiosk island with display cases.",
            "Brushed steel and bright red coffee/snack kiosk counter with espresso machine.",
            "Dark walnut and brass watch/pharmacy kiosk island with glass jewelry trays.",
        ),
    ),
    PropSpriteSpec(
        id="conveyor_belt",
        prop_kind_code="CONVEYOR_BELT",
        filename="conveyor_belt.webp",
        label="Assembly Conveyor Belt",
        base_w_cells=3,
        base_h_cells=1,
        target_w_px=576,
        target_h_px=192,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal industrial factory assembly conveyor belt section (3x wider "
            "than tall) seen strictly straight down; dark ribbed rubber/roller belt "
            "framed by painted steel side rails and carrying two neat cardboard/plastic crates."
        ),
        variant_directives=(
            "Slate-grey steel conveyor frame with dark rubber belt and two grey plastic totes.",
            "Safety-yellow galvanized roller conveyor carrying sealed brown shipping cartons.",
            "Clean stainless medical-packaging conveyor belt with sealed white supply packs.",
        ),
    ),
    PropSpriteSpec(
        id="industrial_press",
        prop_kind_code="INDUSTRIAL_PRESS",
        filename="industrial_press.webp",
        label="Hydraulic Industrial Press",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square heavy hydraulic stamping / forging press seen straight down from "
            "above; massive painted industrial cast-steel crown housing, hydraulic "
            "cylinder top cap, steel pressure pipes, and hazard-striped bolster deck."
        ),
        variant_directives=(
            "Weathered safety-orange hydraulic press crown with dark steel bolster.",
            "Heavy olive-grey forging press with brass pressure manifold and oil stain patina.",
            "Industrial cobalt-blue stamping press with twin hydraulic cylinders and control pendant.",
        ),
    ),
    PropSpriteSpec(
        id="pallet_rack",
        prop_kind_code="PALLET_RACK",
        filename="pallet_rack.webp",
        label="High-Bay Pallet Rack",
        base_w_cells=3,
        base_h_cells=1,
        target_w_px=576,
        target_h_px=192,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal heavy warehouse high-bay pallet rack (3x wider than tall) "
            "seen strictly straight down; safety-orange steel beam uprights holding "
            "three wooden pallets loaded with shrink-wrapped cartons and crates."
        ),
        variant_directives=(
            "Orange beam rack holding three wood pallets of shrink-wrapped brown cartons.",
            "Galvanized steel rack loaded with blue plastic drums, grey totes, and strapped crates.",
            "Weathered charcoal warehouse rack holding stacked white medical supply pallets.",
        ),
    ),
    PropSpriteSpec(
        id="forklift",
        prop_kind_code="FORKLIFT",
        filename="forklift.webp",
        label="Warehouse Forklift",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal compact warehouse counterbalance forklift truck seen strictly "
            "straight down from above (twice as wide as tall); heavy counterweight "
            "body on the LEFT, dark protective overhead ROPS cage roof in middle, "
            "and twin steel lifting forks extending straight RIGHT."
        ),
        variant_directives=(
            "Classic safety-yellow warehouse forklift with black ROPS cage and steel forks.",
            "Weathered safety-orange industrial forklift carrying a compact wooden pallet on forks.",
            "Slate-blue electric clinic/depot forklift with amber strobe beacon on cage roof.",
        ),
    ),
    PropSpriteSpec(
        id="shipping_container",
        prop_kind_code="SHIPPING_CONTAINER",
        filename="shipping_container.webp",
        label="ISO Shipping Container",
        base_w_cells=6,
        base_h_cells=2,
        target_w_px=640,
        target_h_px=214,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Long horizontal 20-foot ISO corrugated steel cargo shipping container "
            "roof (3x wider than tall) seen strictly straight down from above; crisp "
            "vertical corrugated steel roof ribs, heavy corner castings, and subtle "
            "weathering."
        ),
        variant_directives=(
            "Weathered maritime ocean-blue corrugated steel container roof with light rust.",
            "Rust-red cargo container roof with faded white stencil markings on corner.",
            "Dark forest-green military-surplus ISO container roof with matte protective coat.",
        ),
    ),
    PropSpriteSpec(
        id="wood_crate",
        prop_kind_code="WOOD_CRATE",
        filename="wood_crate.webp",
        label="Wooden Crate",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square wooden cargo shipping crate seen straight down from above; "
            "slatted pine timber frame with diagonal bracing plank, iron corner "
            "brackets, and nail heads."
        ),
        variant_directives=(
            "Natural light-pine slatted shipping crate with diagonal brace plank.",
            "Dark weathered oak military supply crate with steel corner straps.",
            "Compact double-crate pair (one strapped wooden box beside smaller ammo/tool box).",
        ),
    ),
    PropSpriteSpec(
        id="drum_cluster",
        prop_kind_code="DRUM_CLUSTER",
        filename="drum_cluster.webp",
        label="Steel Drums",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square cluster of three 55-gallon industrial steel drums / barrels touching "
            "shoulder-to-shoulder on a small dark-steel spill pallet pad seen strictly "
            "straight down from above; circular stamped steel drum lids with twin bung plugs."
        ),
        variant_directives=(
            "Two cobalt-blue and one rusty steel 55-gallon drums touching tightly with NO center magenta gap.",
            "Two safety-yellow hazmat drums and one matte-black oil drum.",
            "Three weathered olive-green and galvanized steel surplus fuel barrels.",
        ),
    ),
    PropSpriteSpec(
        id="structural_pillar",
        prop_kind_code="STRUCTURAL_PILLAR",
        filename="structural_pillar.webp",
        label="Structural Bulkhead / Pillar",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square architectural concrete & steel structural load-bearing column / "
            "pillar cap seen straight down; chamfered reinforced concrete block with "
            "dark steel H-beam flange trim and subtle corner bolts."
        ),
        variant_directives=(
            "Reinforced architectural concrete square column with chamfered steel corners.",
            "Painted industrial grey concrete pillar with yellow/black hazard bumper collar.",
            "Heavy dark riveted cast-steel structural warehouse bulkhead column.",
        ),
    ),
    PropSpriteSpec(
        id="elevated_catwalk",
        prop_kind_code="ELEVATED_CATWALK",
        filename="elevated_catwalk.webp",
        label="Elevated Gantry / Catwalk",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square industrial steel catwalk / gantry walkway deck section seen "
            "straight down; dark diamond-mesh steel grating deck framed by brushed "
            "steel side channels and tubular safety handrails along TOP and BOTTOM edges."
        ),
        variant_directives=(
            "Galvanized steel mesh gantry deck with brushed steel safety rails.",
            "Dark charcoal industrial catwalk grating with safety-yellow handrail bars.",
            "Weathered steel perforated deck plate with bolted seam flanges.",
        ),
    ),
    # --- Batch 7: Outdoor, Vehicles & Infrastructure (23) ---
    PropSpriteSpec(
        id="car",
        prop_kind_code="CAR",
        filename="car.webp",
        label="Sedan",
        base_w_cells=2,
        base_h_cells=5,
        target_w_px=256,
        target_h_px=640,
        aspect_ratio="9:16",
        prompt=prop_prompt(
            "Vertical portrait top-down modern 4-door sedan car seen strictly from 90 "
            "degrees directly overhead (much taller than wide); rear trunk & tail-lights "
            "at the TOP edge, dark tinted rear & front windshields, smooth painted "
            "roof, side mirrors, and sleek hood pointing straight DOWNWARD toward the "
            "bottom edge."
        ),
        variant_directives=(
            "Metallic slate-blue modern sedan with dark glass sunroof.",
            "Clean matte pearl-silver compact hatchback sedan.",
            "Deep burgundy-crimson 4-door sedan with subtle road dust.",
        ),
    ),
    PropSpriteSpec(
        id="jersey_barrier",
        prop_kind_code="JERSEY_BARRIER",
        filename="jersey_barrier.webp",
        label="Concrete Jersey Barrier",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal precast concrete highway Jersey road barrier seen strictly "
            "straight down from above (twice as wide as tall); weathered pale grey "
            "concrete tapered profile with flat top ridge, lifting slots, and orange "
            "or yellow reflective top marker."
        ),
        variant_directives=(
            "Weathered grey precast concrete Jersey barrier with orange top reflector.",
            "Chipped urban concrete security barrier with faded yellow/black chevron stripe.",
            "Clean industrial concrete block barrier with steel interlocking pins at ends.",
        ),
    ),
    PropSpriteSpec(
        id="park_bench",
        prop_kind_code="PARK_BENCH",
        filename="park_bench.webp",
        label="Park Bench",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal slatted wooden park bench seen straight down from above "
            "(twice as wide as tall); backrest slats along TOP edge, three wide "
            "bench seat planks, and black cast-iron armrests left and right."
        ),
        variant_directives=(
            "Warm oiled cedar wood slats with matte-black cast-iron arms.",
            "Weathered silver-grey teak park bench with ornate bronze armrests.",
            "Painted forest-green municipal wood-and-steel bench.",
        ),
    ),
    PropSpriteSpec(
        id="picnic_table",
        prop_kind_code="PICNIC_TABLE",
        filename="picnic_table.webp",
        label="Picnic Table",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square outdoor wooden picnic table with attached parallel benches along "
            "TOP and BOTTOM sides seen strictly straight down from above; slatted "
            "timber planks with natural grain and knots."
        ),
        variant_directives=(
            "Warm honey-cedar slatted picnic table with natural grain.",
            "Weathered silver-pine park picnic table with small thermos and paper bag.",
            "Coated dark-green perforated steel mesh municipal picnic table.",
        ),
    ),
    PropSpriteSpec(
        id="planter",
        prop_kind_code="PLANTER",
        filename="planter.webp",
        label="Planter",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square architectural concrete/terracotta street planter box filled with "
            "lush leafy green ornamental shrubs and small blossoms seen straight down."
        ),
        variant_directives=(
            "Square precast grey concrete planter with lush emerald boxwood foliage.",
            "Round terracotta pot planter holding broad tropical fern leaves.",
            "Warm cedar timber planter box with blooming lavender and green herbs.",
        ),
    ),
    PropSpriteSpec(
        id="planter_table",
        prop_kind_code="PLANTER_TABLE",
        filename="planter_table.webp",
        label="Conservatory Planter Bench",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal botanical conservatory / greenhouse potting bench (twice as "
            "wide as tall) seen straight down; wooden slatted bench holding terracotta "
            "seedling pots, dark soil trays, lush green sprouts, and small trowel."
        ),
        variant_directives=(
            "Cedar potting table holding rows of terracotta herb pots and trowel.",
            "Galvanized steel hydroponic seedling tray bench with bright green microgreens.",
            "Weathered wood greenhouse table with clay pots, watering can, and fern seedlings.",
        ),
    ),
    PropSpriteSpec(
        id="crop_row",
        prop_kind_code="CROP_ROW",
        filename="crop_row.webp",
        label="Field Crop Row",
        base_w_cells=3,
        base_h_cells=1,
        target_w_px=576,
        target_h_px=192,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal narrow garden / farm crop row strip (3x wider than tall) "
            "seen straight down; neat raised dark soil ridge densely planted with "
            "leafy green vegetable crops."
        ),
        variant_directives=(
            "Lush emerald cabbage and leafy kale heads along a dark soil ridge.",
            "Tall golden-green corn/grain stalks clustered in an organic row.",
            "Trellised tomato and pepper vines with small red ripe fruit dots.",
        ),
    ),
    PropSpriteSpec(
        id="hedge",
        prop_kind_code="HEDGE",
        filename="hedge.webp",
        label="Trimmed Hedge",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square manicured garden boxwood hedge shrub block seen strictly straight "
            "down from above; dense overlapping tiny dark-and-bright emerald leaves "
            "forming a neat crisp rectangular hedge top."
        ),
        variant_directives=(
            "Crisply trimmed vibrant emerald boxwood hedge block.",
            "Deep olive-forest privet hedge with slightly organic ruffled leaf edges.",
            "Flowering garden hedge dotted with sparse tiny white jasmine blossoms.",
        ),
    ),
    PropSpriteSpec(
        id="bush",
        prop_kind_code="BUSH",
        filename="bush.webp",
        label="Shrub / Bush",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Organic wild garden shrub / bush seen strictly straight down from above "
            "on a square canvas; irregular layered clusters of green foliage leaves "
            "with darker shaded inner twigs."
        ),
        variant_directives=(
            "Dense natural forest-green deciduous bush with sunlit upper leaves.",
            "Wild sage-and-olive thicket shrub with a few tiny red berries.",
            "Broad-leafed tropical garden shrub in rich deep emerald greens.",
        ),
    ),
    PropSpriteSpec(
        id="boulder",
        prop_kind_code="BOULDER",
        filename="boulder.webp",
        label="Granite Boulder",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal natural rugged granite rock boulder outcrop (twice as wide "
            "as tall) seen strictly straight down from above; chiseled stone facets, "
            "natural mineral cracks, and flecks of olive-green moss."
        ),
        variant_directives=(
            "Cool grey fractured granite boulder with subtle green lichen patches.",
            "Warm ochre-sandstone weathered rock cluster with layered crevices.",
            "Dark basalt rock outcrop with crisp faceted fractures.",
        ),
    ),
    PropSpriteSpec(
        id="fallen_log",
        prop_kind_code="FALLEN_LOG",
        filename="fallen_log.webp",
        label="Fallen Log",
        base_w_cells=3,
        base_h_cells=1,
        target_w_px=576,
        target_h_px=192,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal weathered fallen forest tree trunk log (3x wider than tall) "
            "seen strictly straight down from above; rugged furrowed brown oak bark, "
            "broken branch stub, and patches of vibrant green forest moss."
        ),
        variant_directives=(
            "Mossy brown oak trunk with short broken branch fork on right.",
            "Silvery weathered driftwood log with split heartwood crack.",
            "Dark pine log with shelf mushrooms and green moss blanket along top ridge.",
        ),
    ),
    PropSpriteSpec(
        id="small_fountain",
        prop_kind_code="SMALL_FOUNTAIN",
        filename="small_fountain.webp",
        label="Small Courtyard Fountain",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Circular small carved-stone courtyard water fountain centered on a square "
            "canvas seen straight down; carved limestone basin rim, clear turquoise "
            "water with gentle white foam ripples around central bronze spout."
        ),
        variant_directives=(
            "Carved warm limestone circular basin with turquoise water and bronze spout.",
            "Octagonal dark granite plaza fountain with concentric water ripples.",
            "Weathered terracotta-and-moss garden birdbath fountain with clear water.",
        ),
    ),
    PropSpriteSpec(
        id="medium_fountain",
        prop_kind_code="MEDIUM_FOUNTAIN",
        filename="medium_fountain.webp",
        label="Tiered Plaza Fountain",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square 2x2m tiered circular stone plaza fountain seen straight down; "
            "wide carved stone coping wall, sparkling aqua-blue pool filled with "
            "concentric ripple rings, and smaller raised stone centerpiece bowl."
        ),
        variant_directives=(
            "Warm travertine tiered fountain with sparkling aqua water.",
            "Cool pale-grey granite civic plaza fountain with bronze tiered dish.",
            "Classical white marble fountain with soft teal water and mossy edge.",
        ),
    ),
    PropSpriteSpec(
        id="large_fountain",
        prop_kind_code="LARGE_FOUNTAIN",
        filename="large_fountain.webp",
        label="Grand Monumental Fountain",
        base_w_cells=3,
        base_h_cells=3,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Grand circular monumental civic stone fountain seen straight down on a "
            "square canvas; wide stepped marble/limestone outer basin rim, deep "
            "cerulean-aqua pool with radial water jet plumes, and multi-tiered "
            "carved stone center pedestal."
        ),
        variant_directives=(
            "Grand white marble multi-tier plaza fountain with radial spray plumes.",
            "Ornate carved limestone and oxidized verdigris-bronze civic fountain.",
            "Sleek dark basalt architectural reflecting fountain with illuminated cyan water jets.",
        ),
    ),
    PropSpriteSpec(
        id="bbq_grill",
        prop_kind_code="BBQ_GRILL",
        filename="bbq_grill.webp",
        label="Outdoor Barbeque Grill",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down outdoor stainless-steel barbeque gas grill with side "
            "prep shelves seen straight down; brushed steel hood cover in center "
            "with handle along bottom edge and stainless trays left and right."
        ),
        variant_directives=(
            "Brushed stainless steel gas grill with closed hood and side trays.",
            "Matte-black enameled barrel smoker grill with wooden side shelf and tongs.",
            "Classic glossy crimson-red kettle charcoal grill beside small wood prep cart.",
        ),
    ),
    PropSpriteSpec(
        id="garden_shed",
        prop_kind_code="GARDEN_SHED",
        filename="garden_shed.webp",
        label="Backyard Utility Shed",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Wide 3:2 backyard garden utility shed roof seen strictly straight down "
            "from 90 degrees directly overhead; symmetrical pitched gable roof with "
            "center ridge beam running horizontally left-to-right and textured roof "
            "shingles or corrugated metal panels."
        ),
        variant_directives=(
            "Weathered cedar wood shingle gable shed roof with dark timber ridge cap.",
            "Painted forest-green & white trim asphalt-shingle garden shed roof.",
            "Corrugated galvanized steel utility shed roof with faint rust streaks.",
        ),
    ),
    PropSpriteSpec(
        id="hot_tub",
        prop_kind_code="HOT_TUB",
        filename="hot_tub.webp",
        label="Outdoor Spa & Hot Tub",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square 2x2m outdoor cedar-clad hot tub / jacuzzi spa seen straight down; "
            "warm timber deck surround framing a contoured acrylic basin filled with "
            "bubbling translucent turquoise water and four padded corner headrests."
        ),
        variant_directives=(
            "Warm redwood cedar surround with bubbling bright turquoise spa water.",
            "Sleek slate-grey composite hot tub with glowing cyan water jets.",
            "Warm travertine stone spa deck with calm crystal-blue water and folded towels.",
        ),
    ),
    PropSpriteSpec(
        id="garbage_pile",
        prop_kind_code="GARBAGE_PILE",
        filename="garbage_pile.webp",
        label="Garbage & Refuse Pile",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal irregular pile of tied black plastic garbage bags, crushed "
            "cardboard shipping cartons, and urban refuse seen straight down."
        ),
        variant_directives=(
            "Cluster of glossy black tied trash bags and damp crushed brown cardboard boxes.",
            "Assorted blue/black refuse sacks beside broken wooden pallet slats.",
            "Alley pile of tied white medical-linen sacks, flattened cartons, and scrap plastic.",
        ),
    ),
    PropSpriteSpec(
        id="street_debris",
        prop_kind_code="STREET_DEBRIS",
        filename="street_debris.webp",
        label="Scattered Street Litter & Debris",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal low-profile scatter of urban street debris seen straight "
            "down: crushed newspaper pages, flattened aluminum cans, broken concrete "
            "chunks, and dry autumn leaves."
        ),
        variant_directives=(
            "Newspaper sheets, crushed soda cans, gravel bits, and dry leaves.",
            "Shattered brick rubble chunks, torn tarp corner, and weathered plastic crate lid.",
            "Storm-blown street litter with plastic bottles, cardboard scrap, and orange cone tip.",
        ),
    ),
    PropSpriteSpec(
        id="brownstone_stoop",
        prop_kind_code="BROWNSTONE_STOOP",
        filename="brownstone_stoop.webp",
        label="Carved Front Entry Stoop & Rails",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square carved brownstone / masonry front entry porch stoop and steps "
            "seen strictly straight down; top landing slab along TOP edge, descending "
            "stone stair treads toward BOTTOM, and wrought-iron railings along left "
            "and right cheek walls."
        ),
        variant_directives=(
            "Warm carved terracotta-brownstone steps with wrought-iron handrails.",
            "Cool weathered grey granite entry stoop steps with matte-black iron rails.",
            "Red brick and limestone-tread front porch steps with brass ball finials.",
        ),
    ),
    PropSpriteSpec(
        id="security_camera_mast",
        prop_kind_code="SECURITY_CAMERA_MAST",
        filename="security_camera_mast.webp",
        label="ALPR & Acoustic Sensor Mast",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down 2035 municipal surveillance & acoustic sensor pole mast "
            "seen straight down; bolted octagonal concrete/steel base plate with "
            "vertical galvanized mast cap, dual directional white CCTV bullet camera "
            "housings, and compact solar sensor disc."
        ),
        variant_directives=(
            "Galvanized steel mast cap with dual white bullet cameras and glowing green LED.",
            "Matte-black tactical ALPR sensor pylon with triple acoustic dome array.",
            "Weathered urban street-pole mount with solar micro-panel and amber strobe cap.",
        ),
    ),
    PropSpriteSpec(
        id="drone_dock_pad",
        prop_kind_code="DRONE_DOCK_PAD",
        filename="drone_dock_pad.webp",
        label="Inductive UAV Launch & Recharge Pad",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square 2x2m 2035 autonomous UAV quadrotor drone landing and inductive "
            "recharge pad seen strictly straight down; matte dark-slate composite "
            "hex/octagon landing deck with yellow/cyan corner alignment markings, "
            "copper inductive coil ring in center, and corner status beacons."
        ),
        variant_directives=(
            "Dark composite UAV pad with crisp yellow corner chevrons and cyan coil ring.",
            "Medical-courier drone landing deck with red-and-white cross markings and steel edge.",
            "Ruggedized matte-olive tactical UAV charging pad with amber perimeter strobe LEDs.",
        ),
    ),
    PropSpriteSpec(
        id="hv_transformer",
        prop_kind_code="HV_TRANSFORMER",
        filename="hv_transformer.webp",
        label="500 kV High-Voltage Transformer",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square 2x2m heavy electrical substation high-voltage transformer unit "
            "seen strictly straight down; industrial olive-grey painted steel oil tank "
            "roof with vertical finned radiator banks along left/right sides, three "
            "ceramic ribbed HV insulator bushings protruding cleanly upward, and "
            "expansion conservator cylinder."
        ),
        variant_directives=(
            "Industrial battleship-grey HV transformer with three brown ceramic bushings and radiator fins.",
            "Weathered olive-slate substation transformer with pale-blue porcelain insulators and warning placard.",
            "Heavy dark-charcoal 500 kV step-down transformer with copper busbar risers.",
        ),
    ),
)
