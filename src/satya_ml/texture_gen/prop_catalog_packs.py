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

BATCH_11_RESTROOM_DINER_FARM_IDS: tuple[str, ...] = (
    "urinal",
    "changing_table",
    "snack_vending_machine",
    "fast_food_counter",
    "soda_fountain",
    "drive_thru_menu_board",
    "host_stand",
    "deep_fryer_station",
    "pickup_truck",
    "horse",
    "cow",
    "pig",
    "chicken",
    "goat",
    "dog",
    "silo",
    "wheelbarrow",
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
    # --- Batch 11: Restroom Suites, Restaurants, Pickup Truck & Livestock (17) ---
    PropSpriteSpec(
        id="urinal",
        prop_kind_code="URINAL",
        filename="urinal.webp",
        label="Wall-Mounted Ceramic Urinal",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square 1x1 public restroom wall-mounted white glazed ceramic urinal fixture flush against the TOP edge, "
            "seen strictly 90-degree straight down; bright glossy porcelain basin crescent projecting forward with brushed "
            "chrome flushometer valve pipe at top and small stainless drain grate."
        ),
        variant_directives=(
            "Crisp white glazed porcelain commercial wall urinal with chrome flush valve.",
            "Sleek matte-cream modern waterless ceramic urinal basin with brushed nickel trim.",
            "Commercial stainless-steel brushed urinal bowl with chrome top pipe and drain strainer.",
        ),
    ),
    PropSpriteSpec(
        id="changing_table",
        prop_kind_code="CHANGING_TABLE",
        filename="changing_table.webp",
        label="Restroom Baby Changing Table",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square 1x1 restroom family room fold-down wall-mounted baby changing table flush along TOP edge, "
            "seen strictly straight down; moulded warm cream-white antibacterial plastic tray with curved raised side lips, "
            "safety nylon buckle strap across center, and small paper liner dispenser."
        ),
        variant_directives=(
            "Cream-white horizontal fold-down changing station with grey nylon safety strap.",
            "Brushed stainless steel & light-grey horizontal family restroom changing table.",
            "Warm almond molded polymer changing station with blue liner pocket along top hinge.",
        ),
    ),
    PropSpriteSpec(
        id="snack_vending_machine",
        prop_kind_code="SNACK_VENDING_MACHINE",
        filename="snack_vending_machine.webp",
        label="Glass-Front Snack Vending Machine",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square commercial coil-spiral snack vending machine cabinet top seen strictly 90-degree straight down (1:1); "
            "matte charcoal-black steel cabinet roof with warm amber glowing glass showcase lip and coin-slot bezel along "
            "the BOTTOM front edge and small ventilation louver along the rear top edge."
        ),
        variant_directives=(
            "Matte-black glass-front snack vending cabinet with warm amber illuminated front lip.",
            "Hunter-green corporate breakroom snack machine top with silver keypad rim and vent grille.",
            "Burgundy-red retro coil-snack vending machine cabinet with chrome front edge trim.",
        ),
    ),
    PropSpriteSpec(
        id="fast_food_counter",
        prop_kind_code="FAST_FOOD_COUNTER",
        filename="fast_food_counter.webp",
        label="Quick-Service Order & POS Counter",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal continuous straight quick-service fast food order counter (2x wider than tall, open flat "
            "counter ends left & right so sections tile cleanly), seen straight down; crimson-red laminate counter edge, "
            "brushed steel pickup trays, dual touchscreen POS cash registers facing front, receipt printer, and condiment bins."
        ),
        variant_directives=(
            "Crimson-red and brushed-steel fast-food order counter with dual POS registers and serving trays.",
            "Bright retro teal & cream diner service counter with metal napkin holders and twin POS screens.",
            "Modern amber-wood & stainless burger counter with digital order pads and heat-lamp pass bar.",
        ),
    ),
    PropSpriteSpec(
        id="soda_fountain",
        prop_kind_code="SODA_FOUNTAIN",
        filename="soda_fountain.webp",
        label="Self-Serve Soda Fountain & Ice Island",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal self-serve restaurant beverage & soda fountain island counter seen straight down (twice as wide as tall); "
            "brushed stainless-steel multi-nozzle carbonated drink dispenser turret with ice chute, slotted chrome drip grate, "
            "colorful beverage lever tabs, cup lid sleeves, and straw organizer caddy."
        ),
        variant_directives=(
            "Stainless steel multi-flavor soda dispenser tower with chrome drip tray and red/blue logo tabs.",
            "Black & silver touchscreen freestyle beverage counter with cup lid turret and ice dispenser.",
            "Compact diner fountain bar with chrome soda heads, napkin caddy, and waxed drink cups.",
        ),
    ),
    PropSpriteSpec(
        id="drive_thru_menu_board",
        prop_kind_code="DRIVE_THRU_MENU_BOARD",
        filename="drive_thru_menu_board.webp",
        label="Drive-Thru Outdoor Illuminated Menu Board",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal drive-thru outdoor double-panel illuminated menu board canopy & intercom post seen straight down (2:1); "
            "weatherproof dark charcoal powder-coated steel hood overhang with slim warm glowing lightbox glow along front bottom edge, "
            "bolted heavy steel base plate, and speaker bollard."
        ),
        variant_directives=(
            "Dark charcoal weatherproof drive-thru menu board hood with warm glowing yellow bottom edge and intercom speaker.",
            "Crimson-trimmed dual digital drive-thru menu sign canopy with rain hood and call button.",
            "Sleek silver aluminum QSR order confirmation canopy with solar top plate and bollard mount.",
        ),
    ),
    PropSpriteSpec(
        id="host_stand",
        prop_kind_code="HOST_STAND",
        filename="host_stand.webp",
        label="Restaurant Host Podium & Menu Lectern",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square restaurant maître d' host podium stand seen strictly 90-degree straight down (1:1); "
            "dark polished walnut lectern top with leather-bound floorplan reservation book lying open, stacked linen menus, "
            "small brass bell, and cup of pens."
        ),
        variant_directives=(
            "Dark walnut host podium with open leather reservation binder and stacked menus.",
            "Sleek matte-black steel restaurant check-in stand with tablet mount and menu sheaf.",
            "Warm golden oak diner podium stand with brass menu clip and seating chart.",
        ),
    ),
    PropSpriteSpec(
        id="deep_fryer_station",
        prop_kind_code="DEEP_FRYER_STATION",
        filename="deep_fryer_station.webp",
        label="Commercial Dual-Vat Deep Fryer Station",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal commercial restaurant stainless-steel dual-vat deep fryer battery seen straight down (2:1); "
            "heavy brushed chrome frame holding two rectangular glowing golden cooking-oil vats side-by-side with four "
            "wire mesh fry baskets hooked cleanly in the oil and flue riser along the TOP edge."
        ),
        variant_directives=(
            "Stainless dual-vat commercial deep fryer with amber oil and four wire fry baskets.",
            "Heavy triple-basket restaurant fryer bank with stainless dump station tray of golden fries.",
            "Brushed steel commercial kitchen fryer unit with digital timer bar along top backsplash.",
        ),
    ),
    PropSpriteSpec(
        id="pickup_truck",
        prop_kind_code="PICKUP_TRUCK",
        filename="pickup_truck.webp",
        label="Rural & Work Crew Pickup Truck",
        base_w_cells=2,
        base_h_cells=5,
        target_w_px=256,
        target_h_px=640,
        aspect_ratio="9:16",
        prompt=prop_prompt(
            "Vertical portrait full-size four-wheel-drive farm & work pickup truck seen strictly from 90 degrees directly "
            "overhead (much taller than wide); open rectangular pickup cargo bed with ribbed liner, tailgate at the TOP edge "
            "holding a toolbox and tarp strap, double cab roof in the middle, dark tinted front windshield, side mirrors, "
            "and broad flat engine hood pointing straight DOWNWARD toward the bottom edge."
        ),
        variant_directives=(
            "Weathered barn-red single-cab pickup truck with black bedliner and silver toolbox in bed.",
            "Forest-green 4x4 ranch crew-cab pickup truck with dark tonneau trim, roof rack, and spare tire in cargo bed.",
            "Dusty slate-blue farm utility pickup truck with wood side-stakes in bed and muddy fender flares.",
        ),
    ),
    PropSpriteSpec(
        id="horse",
        prop_kind_code="HORSE",
        filename="horse.webp",
        label="Farmstead Horse",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal standing horse facing strictly RIGHT seen directly from 90-degree straight down overhead bird's-eye view "
            "(clean compact 2:1 footprint); rich glossy chestnut-brown barrel coat, flowing dark mane along neck extending right, "
            "pointed ears forward at right, and dark swishing tail extending left (NO ground shadow)."
        ),
        variant_directives=(
            "Chestnut bay horse with black mane and tail facing right.",
            "Dapple-grey farm draft horse with creamy flaxen mane facing right.",
            "Warm pinto paint horse with chestnut and cream patches facing right.",
        ),
    ),
    PropSpriteSpec(
        id="cow",
        prop_kind_code="COW",
        filename="cow.webp",
        label="Pasture Dairy / Beef Cow",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal sturdy dairy/beef cow facing strictly RIGHT seen directly from 90-degree straight down overhead bird's-eye view "
            "(2:1 footprint); rounded broad back with organic Holstein black-and-white hide patches, out-turned ears and pale horns "
            "at right head, and slender tail at left (NO ground shadow)."
        ),
        variant_directives=(
            "Black-and-white spotted Holstein dairy cow seen strictly from above facing right.",
            "Warm reddish-brown Hereford beef cattle cow with white head stripe facing right.",
            "Gentle cream-jersey pasture cow with dark muzzle and ears facing right.",
        ),
    ),
    PropSpriteSpec(
        id="pig",
        prop_kind_code="PIG",
        filename="pig.webp",
        label="Farmstead Hog / Pig",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        key_color="green",
        prompt=prop_prompt(
            "Plump round farmstead pig facing RIGHT seen strictly from 90-degree straight down overhead bird's-eye view "
            "centered neatly inside a square footprint; warm rosy-peach pink pig body with rounded snout at right, flopped ears, "
            "light mud dappling along flanks, and tiny curly tail at left (NO ground shadow).",
            key_color="green",
        ),
        variant_directives=(
            "Rosy pink farmstead hog with pale mud speckles along back facing right.",
            "Spotted Berkshire dark-slate and cream-pink piglet facing right.",
            "Reddish-ginger Tamworth farm pig with tufted ears facing right.",
        ),
    ),
    PropSpriteSpec(
        id="chicken",
        prop_kind_code="CHICKEN",
        filename="chicken.webp",
        label="Free-Range Farm Chicken",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Small round plump barnyard hen/chicken facing RIGHT seen strictly 90-degree straight down from above centered in a square "
            "(plump feathered teardrop back filling ~55% of center); warm russet-red and buff plumage feathers, small red comb and beak at right, "
            "fanned tail feathers at left (NO ground shadow)."
        ),
        variant_directives=(
            "Warm Rhode Island russet-red hen with fanned dark tail feathers facing right.",
            "Speckled cream-white Plymouth barnyard chicken with red comb facing right.",
            "Golden-buff & black barred heritage hen facing right.",
        ),
    ),
    PropSpriteSpec(
        id="goat",
        prop_kind_code="GOAT",
        filename="goat.webp",
        label="Farmyard Goat",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Compact shaggy farm goat facing RIGHT seen strictly 90-degree straight down from above centered in a square; "
            "slender off-white and taupe woolly back, curved dark swept-back horns behind head at right, floppy ears, and short tail at left "
            "(NO ground shadow)."
        ),
        variant_directives=(
            "Cream-and-taupe dairy goat with curved dark horns facing right.",
            "Chocolate-brown Nubian farm goat with long droopy ears facing right.",
            "Black-and-white Alpine piebald goat facing right.",
        ),
    ),
    PropSpriteSpec(
        id="dog",
        prop_kind_code="DOG",
        filename="dog.webp",
        label="Farmstead / Watch Dog",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Medium shepherd/farmstead dog facing RIGHT seen strictly 90-degree straight down from above centered in a square; "
            "sleek golden-tan and sable fur along spine, alert pointed ears and dark muzzle pointing right, and bushy tail extending left "
            "(NO ground shadow)."
        ),
        variant_directives=(
            "Golden-retriever & red collar farm dog facing right.",
            "Black-and-white Australian shepherd border collie with fluffy tail facing right.",
            "Tan-and-black German shepherd ranch watchdog facing right.",
        ),
    ),
    PropSpriteSpec(
        id="silo",
        prop_kind_code="SILO",
        filename="silo.webp",
        label="Corrugated Galvanized Grain Silo",
        base_w_cells=2,
        base_h_cells=2,
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Circular corrugated galvanized steel agricultural grain silo conical roof seen strictly 90-degree straight down overhead (1:1); "
            "silver segmented radial metal roof ribs converging cleanly to a raised center auger vent cap, steel ladder cage stub at bottom rim, "
            "and subtle weathered zinc patina."
        ),
        variant_directives=(
            "Clean silver galvanized radial-rib steel grain silo dome with central vent cap.",
            "Weathered zinc-and-rust farm feed bin roof with grain auger pipe flange.",
            "Painted classic dark-green enamel Harvestore silo dome with white roof cap hatch.",
        ),
    ),
    PropSpriteSpec(
        id="wheelbarrow",
        prop_kind_code="WHEELBARROW",
        filename="wheelbarrow.webp",
        label="Steel Farm Wheelbarrow",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Single-wheel heavy-duty garden & farm wheelbarrow pointing RIGHT seen strictly 90-degree straight down overhead (1:1); "
            "twin wooden grip handles extending left, painted deep-green pressed steel hopper tray filled lightly with fresh golden straw/soil, "
            "and dark rubber front tire peeking out on right."
        ),
        variant_directives=(
            "Forest-green steel farm wheelbarrow with oak handles and straw wisp.",
            "Weathered royal-blue contractor wheelbarrow with galvanized rim and dark mulch.",
            "Rusty crimson barn wheelbarrow with timber handles pointing right.",
        ),
    ),
)
