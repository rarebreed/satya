"""Batches 8–10 prop sprite specs: Cafe/Retail, Industrial/Farm/Conservatory, Sci-Fi/Office/Street."""

from __future__ import annotations

from satya_ml.texture_gen.prop_catalog_types import PropSpriteSpec, prop_prompt

BATCH_8_CAFE_RETAIL_IDS: tuple[str, ...] = (
    "espresso_counter",
    "commercial_prep_table",
    "commercial_grill_hood",
    "patio_umbrella_table",
    "buffet_counter",
    "refrigerated_display_case",
    "clothing_rack",
    "pharmacy_counter",
    "vending_machine",
    "atm_kiosk",
)

BATCH_9_INDUSTRIAL_FARM_IDS: tuple[str, ...] = (
    "robotic_assembly_arm",
    "tool_chest_workbench",
    "cargo_pallet",
    "electrical_switchgear",
    "tractor",
    "hay_bale_stack",
    "livestock_trough",
    "conservatory_specimen_island",
)

BATCH_10_SCIFI_OFFICE_STREET_IDS: tuple[str, ...] = (
    "command_console",
    "eva_suit_locker",
    "scifi_cargo_pod",
    "hull_debris",
    "conference_table",
    "copy_machine",
    "street_lamp",
    "bus_shelter",
)

PACK_PROP_SPECS: tuple[PropSpriteSpec, ...] = (
    # --- Batch 8: Cafe, Restaurant & Retail (10) ---
    PropSpriteSpec(
        id="espresso_counter",
        prop_kind_code="ESPRESSO_COUNTER",
        filename="espresso_counter.webp",
        label="Barista Espresso & Pastry Counter",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal cafe barista espresso counter (twice as wide as tall) seen straight down; "
            "warm walnut wood and slate countertop with brushed chrome two-group espresso machine "
            "and coffee grinder on the left half, glass-topped croissant pastry display case and "
            "point-of-sale register tablet on the right half."
        ),
        variant_directives=(
            "Warm walnut counter, stainless chrome espresso machine, golden croissants in glass pastry tray.",
            "Dark matte-olive artisan coffee bar with copper-trimmed espresso machine and ceramic cup stack.",
            "Light oak cafe counter with brushed aluminum espresso unit, matcha tin jars, and POS terminal.",
        ),
    ),
    PropSpriteSpec(
        id="commercial_prep_table",
        prop_kind_code="COMMERCIAL_PREP_TABLE",
        filename="commercial_prep_table.webp",
        label="Stainless Commercial Prep Table",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal stainless steel commercial restaurant kitchen prep island table seen straight down "
            "(twice as wide as tall); brushed metal top with maple butcher-block cutting board, chef knife, "
            "recessed steel gastronorm ingredient pans, and mixing bowl."
        ),
        variant_directives=(
            "Brushed steel worktable with thick wooden cutting board, knife, and stainless hotel pans.",
            "Clean commercial kitchen pass counter with two plated ceramic dishes, heat lamp rail, and squeeze bottles.",
            "Prep station with green and white poly cutting boards, herb prep bowls, and stainless wire whisk.",
        ),
    ),
    PropSpriteSpec(
        id="commercial_grill_hood",
        prop_kind_code="COMMERCIAL_GRILL_HOOD",
        filename="commercial_grill_hood.webp",
        label="Commercial Grill Range & Exhaust Hood",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal commercial restaurant flat-top griddle and deep-fryer range seen straight down "
            "(twice as wide as tall); heavy stainless steel chassis with dark cast-iron flat griddle plate "
            "and twin fryer wire baskets on right, slim stainless exhaust baffle rim along top edge."
        ),
        variant_directives=(
            "Seasoned dark steel flat-top grill on the left and golden fryer oil vats with wire baskets on the right.",
            "Six-burner cast-iron commercial gas stove grates with copper stockpot and sauté skillet.",
            "Clean stainless charbroiler grate section next to recessed bain-marie sauce warmers.",
        ),
    ),
    PropSpriteSpec(
        id="patio_umbrella_table",
        prop_kind_code="PATIO_UMBRELLA_TABLE",
        filename="patio_umbrella_table.webp",
        label="Outdoor Patio Umbrella Bistro Table",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square outdoor cafe patio bistro table with octagonal canvas market parasol umbrella and "
            "four tucked-in bistro chairs peeking slightly around the edge, seen strictly 90-degree straight down "
            "showing umbrella ribs converging on a wooden finial cap."
        ),
        variant_directives=(
            "Terracotta-rust canvas parasol umbrella with four dark wrought-iron cafe chairs.",
            "Forest-green and cream striped bistro patio umbrella with woven rattan café seats peeking out.",
            "Classic navy-blue canvas market umbrella with bentwood bistro chairs around the perimeter.",
        ),
    ),
    PropSpriteSpec(
        id="buffet_counter",
        prop_kind_code="BUFFET_COUNTER",
        filename="buffet_counter.webp",
        label="Buffet & Self-Serve Salad Bar Counter",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal restaurant buffet & self-serve salad bar counter seen straight down (twice as wide as tall); "
            "warm oak perimeter trim framing a stainless chilled well holding four neat trays of fresh greens, "
            "carrots, olives, pasta, and stack of white ceramic plates at the left edge."
        ),
        variant_directives=(
            "Chilled salad bar well with crisp greens, cherry tomatoes, dressings, and plate stack.",
            "Hot buffet carving station with brushed brass chafing dishes, tongs, and soup tureen pots.",
            "Bakery & coffee self-serve credenza with glass pastry domes, silver airpot carafes, and napkin holder.",
        ),
    ),
    PropSpriteSpec(
        id="refrigerated_display_case",
        prop_kind_code="REFRIGERATED_DISPLAY_CASE",
        filename="refrigerated_display_case.webp",
        label="Refrigerated Beverage & Deli Cooler Case",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal supermarket & cafe refrigerated beverage cooler cabinet seen straight down (twice as wide as tall); "
            "brushed steel top roof with twin ventilation louvers and a slim front blue-white glass door trim bar "
            "along the bottom edge revealing rows of chilled bottles inside."
        ),
        variant_directives=(
            "Brushed steel supermarket cooler cabinet roof with illuminated icy-blue front glass bevel.",
            "Matte black deli & cold-drink display fridge housing with top condenser grille and chrome strip.",
            "Clean white commercial grocery freezer chest with sliding clear frost-rimmed top glass lids.",
        ),
    ),
    PropSpriteSpec(
        id="clothing_rack",
        prop_kind_code="CLOTHING_RACK",
        filename="clothing_rack.webp",
        label="Retail Apparel Garment Rack",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal retail clothing store rolling garment rack seen straight down (twice as wide as tall); "
            "brushed chrome center crossbar hung tightly with neat rows of folded shoulders in navy, olive, "
            "camel tan, cream, mustard ochre, and charcoal fabric hangers (NO pink or purple clothes)."
        ),
        variant_directives=(
            "Men's and women's blazers and knitwear in navy, olive, cream, and camel wool.",
            "Streetwear rack of denim jackets, charcoal hoodies, olive field coats, and ochre canvas shirts.",
            "Double-rail boutique wooden display rack with neatly spaced cream linen and sage garments.",
        ),
    ),
    PropSpriteSpec(
        id="pharmacy_counter",
        prop_kind_code="PHARMACY_COUNTER",
        filename="pharmacy_counter.webp",
        label="Clinical Pharmacy Dispensary Counter",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal modern clinical pharmacy dispensary counter seen straight down (twice as wide as tall); "
            "crisp white solid-surface top with teal accent strip, prescription label printer, amber pill vials "
            "in a blue sorting tray, barcode scanner, and dual monitor terminals."
        ),
        variant_directives=(
            "Crisp white and seafoam-teal pharmacy dispensary counter with prescription bottles and sorting tray.",
            "Warm maple and stainless apothecary counter with glass partition edge, mortar tray, and scale.",
            "Clinical hospital pharmacy window counter with automated dispensing chute and terminal keyboard.",
        ),
    ),
    PropSpriteSpec(
        id="vending_machine",
        prop_kind_code="VENDING_MACHINE",
        filename="vending_machine.webp",
        label="Automated Snack & Beverage Vending Machine",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square commercial snack & cold drink vending machine roof seen strictly 90-degree straight down "
            "(1:1 footprint); deep cobalt-blue painted steel top with recessed rear cooling vent louver and "
            "a narrow glowing cyan-lit glass shelf lip visible along the front bottom edge."
        ),
        variant_directives=(
            "Cobalt-blue beverage vending cabinet with chrome front trim lip.",
            "Matte dark-red soda machine top with silver rear compressor grille and illuminated front edge.",
            "Sleek charcoal-slate futuristic cyberpunk cyber-snack kiosk machine with amber LED front strip.",
        ),
    ),
    PropSpriteSpec(
        id="atm_kiosk",
        prop_kind_code="ATM_KIOSK",
        filename="atm_kiosk.webp",
        label="Walk-Up ATM & Teller Kiosk",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square free-standing bank ATM vestibule kiosk seen strictly 90-degree straight down (1:1); "
            "dark slate-blue armored steel topper hood with recessed keypad shelf protruding along the front bottom edge "
            "and compact security camera dome on the top hood."
        ),
        variant_directives=(
            "Armored slate-blue bank ATM terminal hood with brushed steel keypad shelf.",
            "Teal-and-chrome neighborhood credit union ATM kiosk with glowing cyan screen bezel rim.",
            "Heavy weathered dark-grey corporate cred-terminal stand with protective side privacy wings.",
        ),
    ),
    # --- Batch 9: Industrial, Farm & Conservatory (8) ---
    PropSpriteSpec(
        id="robotic_assembly_arm",
        prop_kind_code="ROBOTIC_ASSEMBLY_ARM",
        filename="robotic_assembly_arm.webp",
        label="Articulated Robotic Welding & Assembly Cell",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square industrial factory floor 6-axis robotic welding arm mounted on a heavy bolted charcoal steel "
            "pedestal plate with hazard yellow-black corner stripes, articulated safety-orange robotic arm boom "
            "reaching diagonally to a precision laser weld torch head, seen strictly 90-degree straight down."
        ),
        variant_directives=(
            "Safety-orange articulated robot arm on bolted charcoal square floor pedestal.",
            "Industrial canary-yellow assembly robot arm with coiled black pneumatic umbilical conduit.",
            "Crisp white & cobalt cleanroom wafer-handling robotic arm with dual suction end-effector.",
        ),
    ),
    PropSpriteSpec(
        id="tool_chest_workbench",
        prop_kind_code="TOOL_CHEST_WORKBENCH",
        filename="tool_chest_workbench.webp",
        label="Rolling Steel Tool Chest & Mechanic Bench",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal heavy garage rolling red tool chest workbench seen straight down (twice as wide as tall); "
            "oiled butcher-block oak worktop over crimson-red steel body, chrome cast-iron bench vise bolted on corner, "
            "laid-out wrench set, socket rail, oil can, and rugged shop rag."
        ),
        variant_directives=(
            "Classic crimson-red rolling tool chest with butcher-block wood top, vise, and chrome wrenches.",
            "Matte cobalt-blue diesel mechanic chest with diamond-plate aluminum worktop and cordless impact drill.",
            "Olive-drab military field-maintenance chest bench with calipers, grease gun, and parts tray.",
        ),
    ),
    PropSpriteSpec(
        id="cargo_pallet",
        prop_kind_code="CARGO_PALLET",
        filename="cargo_pallet.webp",
        label="Shrink-Wrapped Freight Cargo Pallet",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square warehouse wooden shipping pallet stacked neatly with strapped cardboard cartons and tan crate boxes "
            "wrapped in clear glossy stretch film, wooden pallet slat edges peeking out underneath, strictly 90-degree top-down."
        ),
        variant_directives=(
            "Neatly stacked kraft cardboard logistics cartons strapped on an oak wooden EUR pallet.",
            "Industrial blue polymer chemical jerrycans and white pails strapped on a wooden pallet.",
            "Olive military & relief supply cartons with amber cargo strapping on heavy timber skid.",
        ),
    ),
    PropSpriteSpec(
        id="electrical_switchgear",
        prop_kind_code="ELECTRICAL_SWITCHGEAR",
        filename="electrical_switchgear.webp",
        label="Industrial Electrical Breaker Switchgear",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal industrial high-voltage electrical switchgear cabinet bank seen straight down (twice as wide as tall); "
            "industrial grey steel top roof divided into three bolted bay modules with copper bus-bar roof conduit stubs, "
            "louvered heat vents, and small yellow high-voltage hazard decals."
        ),
        variant_directives=(
            "Industrial slate-grey 3-bay switchgear cabinet top with galvanized conduit elbows.",
            "Weathered sage-grey substation relay cabinet roof with twin circular cooling fans and yellow warning tape.",
            "Modern dark-charcoal smart microgrid distribution rack top with copper grounding bar and amber LED strip.",
        ),
    ),
    PropSpriteSpec(
        id="tractor",
        prop_kind_code="TRACTOR",
        filename="tractor.webp",
        label="Utility Agricultural Farm Tractor",
        base_w_cells=2,
        base_h_cells=3,
        target_w_px=340,
        target_h_px=512,
        aspect_ratio="2:3",
        prompt=prop_prompt(
            "Vertical portrait modern agricultural farm tractor facing UP (2:3 ratio) seen strictly 90-degree straight down "
            "on its roof and hood: deep forest-green sculpted engine hood at top with vertical black exhaust stack cap, "
            "glazed dark-tinted cab roof with amber beacon at middle, massive deep-tread black rubber rear tractor tires "
            "flanking left and right lower sides."
        ),
        variant_directives=(
            "Deep agricultural John-Deere green hood and roof with yellow wheel hubs.",
            "Classic crimson-red orchard tractor with open ROPS canopy frame, padded seat, and mud-dusted tires.",
            "Weathered azure-blue utility farm tractor with front loader bucket arms extended forward.",
        ),
    ),
    PropSpriteSpec(
        id="hay_bale_stack",
        prop_kind_code="HAY_BALE_STACK",
        filename="hay_bale_stack.webp",
        label="Stacked Golden Hay Bales",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal cluster of three tightly bound rectangular golden-straw hay bales stacked side by side "
            "seen straight down (twice as wide as tall); crisp woven dried wheat straw texture with warm amber shadows "
            "and dark twine bindings circling each bale."
        ),
        variant_directives=(
            "Three golden rectangular twine-bound straw bales in a neat farmyard stack.",
            "Two large round rolled hay bales resting side by side with golden textured spiral ends.",
            "Warm ochre alfalfa hay bales with a wooden pitchfork resting diagonally across the top.",
        ),
    ),
    PropSpriteSpec(
        id="livestock_trough",
        prop_kind_code="LIVESTOCK_TROUGH",
        filename="livestock_trough.webp",
        label="Galvanized Feed & Water Trough",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal galvanized steel farm livestock water & feed trough seen straight down (twice as wide as tall); "
            "rolled zinc-metal rim enclosing clear rippling fresh water inside the tub with brass float valve at one end."
        ),
        variant_directives=(
            "Galvanized zinc livestock water trough filled with clear teal-blue water.",
            "Weathered timber and iron farm feed trough filled with golden oat grain and crushed corn.",
            "Heavy molded slate-grey stock tank with crisp water surface and copper inlet pipe.",
        ),
    ),
    PropSpriteSpec(
        id="conservatory_specimen_island",
        prop_kind_code="CONSERVATORY_SPECIMEN_ISLAND",
        filename="conservatory_specimen_island.webp",
        label="Tiered Botanical Specimen Planter Bed",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square octagonal limestone-curbed conservatory botanical specimen island planter bed seen strictly 90-degree straight down; "
            "rich dark potting soil filled with lush layered tropical fern fronds, broad variegated monstera leaves, "
            "compact fan palm canopy in center, and warm terracotta stone rim."
        ),
        variant_directives=(
            "Limestone octagonal planter island packed with tropical ferns, monstera, and central palm fronds.",
            "Aged brick circular botanical bed with vibrant bird-of-paradise foliage, golden bromeliads, and moss.",
            "Dark slate modern conservatory bed with lush bamboo clumps, white calla lilies, and smooth river stones.",
        ),
    ),
    # --- Batch 10: Sci-Fi, Office & Streetscape (8) ---
    PropSpriteSpec(
        id="command_console",
        prop_kind_code="COMMAND_CONSOLE",
        filename="command_console.webp",
        label="Flight & Security Command Console",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal sci-fi orbital flight & security operations control console seen straight down (twice as wide as tall); "
            "chamfered dark gunmetal steel desk housing angled triple widescreen telemetry displays glowing crisp cyan & amber, "
            "tactile backlit switch panels, trackball, and two ergonomic swivel operator chairs tucked along bottom edge."
        ),
        variant_directives=(
            "Dark gunmetal flight-control console with cyan tactical radar screens and two black operator chairs.",
            "Brushed titanium orbital telemetry bridge station with green vector orbit readouts and dual joysticks.",
            "Weathered olive-slate industrial security monitor desk with amber warning indicators and comm handset.",
        ),
    ),
    PropSpriteSpec(
        id="eva_suit_locker",
        prop_kind_code="EVA_SUIT_LOCKER",
        filename="eva_suit_locker.webp",
        label="Pressurized EVA Suit Bay Locker",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal spacecraft airlock pressurized EVA spacesuit storage locker bay seen straight down (twice as wide as tall); "
            "ribbed titanium locker enclosure with twin oxygen manifold valves on top roof, recessed helmet rack shelf, "
            "and two white-and-orange EVA suit shoulders & gold-visor helmets peeking cleanly at the front bay."
        ),
        variant_directives=(
            "Titanium EVA locker bay with white pressurized suit shoulders, gold-tinted helmets, and O2 valves.",
            "Utility slate orbital airlock rack with bright safety-orange hazard suits and Umbilical air hoses.",
            "Closed reinforced gunmetal airlock suit cabinet top with yellow hazard striping and pressure gauges.",
        ),
    ),
    PropSpriteSpec(
        id="scifi_cargo_pod",
        prop_kind_code="SCI_FI_CARGO_POD",
        filename="scifi_cargo_pod.webp",
        label="Sealed Orbital Freight Cargo Pod",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal heavy octagonal polymer-alloy spacecraft cargo container pod seen straight down (twice as wide as tall); "
            "ribbed gunmetal-blue composite lid with central magnetic grapple latch ring, status strip light in ice cyan, "
            "and reinforced corner tie-down bumpers."
        ),
        variant_directives=(
            "Gunmetal-blue orbital cargo pod with glowing cyan latch indicator and yellow stencil stripes.",
            "Clean medical white cryo-freight capsule pod with seafoam-teal seal ring and digital temp readout.",
            "Scuffed amber-ochre mining corporation alloy crate pod with heavy bolted corner clamps.",
        ),
    ),
    PropSpriteSpec(
        id="hull_debris",
        prop_kind_code="HULL_DEBRIS",
        filename="hull_debris.webp",
        label="Twisted Bulkhead Hull Debris",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal pile of torn spacecraft hull plating & severed deck grating debris seen straight down (twice as wide as tall); "
            "jagged scorched charcoal steel bulkhead fragments, bent I-beam rib, tangled copper & yellow conduit cables, "
            "and loose rivets forming an irregular wreckage cluster."
        ),
        variant_directives=(
            "Scorched gunmetal hull armor plates torn open with copper conduit cables and bent steel rib.",
            "Rusted industrial deck grate shard tangled with severed coolant pipes and broken hazard tile.",
            "Blasted white thermal hull insulation tiles mixed with carbonized strut beams and wiring.",
        ),
    ),
    PropSpriteSpec(
        id="conference_table",
        prop_kind_code="CONFERENCE_TABLE",
        filename="conference_table.webp",
        label="Boardroom Conference Table & Chairs",
        base_w_cells=3,
        base_h_cells=2,
        target_w_px=576,
        target_h_px=384,
        aspect_ratio="3:2",
        prompt=prop_prompt(
            "Wide rectangular walnut corporate boardroom conference table surrounded neatly by eight tucked-in dark "
            "swivel office chairs (3 along top, 3 along bottom, 1 each left/right) seen straight down (3:2 ratio); "
            "center recessed brushed-aluminum power grommet strip, crisp architectural notebooks and water glasses."
        ),
        variant_directives=(
            "Rich walnut executive boardroom table with black leather swivel chairs and notepads.",
            "Light Nordic oak oval meeting table with charcoal fabric chairs, laptops, and speakerphone puck.",
            "Frosted tempered glass corporate briefing table over steel frame with sleek slate chairs.",
        ),
    ),
    PropSpriteSpec(
        id="copy_machine",
        prop_kind_code="COPY_MACHINE",
        filename="copy_machine.webp",
        label="Office Multifunction Copier Station",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square commercial office multifunction laser copier & printer station seen strictly 90-degree straight down (1:1); "
            "off-white & dark-charcoal casing with flat automatic document feeder lid, fanned stack of crisp white paper "
            "in output tray on left, and compact angled touchscreen control panel on front right."
        ),
        variant_directives=(
            "Off-white & charcoal enterprise copier with paper output tray and touchscreen panel.",
            "Sleek matte-black & silver wide-format plotter/copier unit with rolled blueprint paper tube beside it.",
            "Compact grey corporate laser print station with neat stack of crisp white reams on top.",
        ),
    ),
    PropSpriteSpec(
        id="street_lamp",
        prop_kind_code="STREET_LAMP",
        filename="street_lamp.webp",
        label="Sidewalk Streetlight Standard",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square sidewalk streetlamp standard seen strictly 90-degree straight down (1:1); circular dark charcoal "
            "cast-iron post base cap at center with slim horizontal bracket arm extending right holding a sleek oval "
            "LED luminaire cobra-head housing glowing warm cream glass on its bottom edge (clean solid isolated object)."
        ),
        variant_directives=(
            "Modern matte-charcoal cobra-head LED streetlight arm extending right from circular pole cap.",
            "Classic historic cast-iron sidewalk lamp post with octagonal lantern glass top.",
            "Industrial galvanized twin-arm floodlight mast cap with dual rectangular LED pods.",
        ),
    ),
    PropSpriteSpec(
        id="bus_shelter",
        prop_kind_code="BUS_SHELTER",
        filename="bus_shelter.webp",
        label="Transit Bus Stop Shelter & Bench",
        base_w_cells=3,
        base_h_cells=1,
        target_w_px=576,
        target_h_px=192,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Wide rectangular urban transit bus stop shelter roof seen strictly 90-degree straight down (3x wider than tall); "
            "slim anodized dark-charcoal aluminum roof frame holding a lightly tinted frosted blue-glass skylight canopy "
            "revealing a faint silhouette of a perforated metal bench and transit route map lightbox underneath."
        ),
        variant_directives=(
            "Anodized dark-charcoal aluminum bus shelter canopy with frosted sky-blue glass roof.",
            "Curved brushed-steel & teal glass urban BRT transit shelter roof with solar trickle panel.",
            "Classic dark bronze-painted municipal bus shelter with corrugated metal awning ribs and bench.",
        ),
    ),
)
