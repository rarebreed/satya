"""Declarative catalog of all top-down 2D battlemap prop sprites for Nano Banana 2."""

from __future__ import annotations

from satya_ml.texture_gen.prop_catalog_interior import (
    BATCH_3_LIVING_BEDROOM_IDS,
    BATCH_4_KITCHEN_BATH_BAR_IDS,
    BATCH_5_OFFICE_CLINICAL_PORTALS_IDS,
    INTERIOR_PROP_SPECS,
)
from satya_ml.texture_gen.prop_catalog_types import (
    PROP_STYLE_PREAMBLE,
    PropSpriteSpec,
    prop_prompt,
)
from satya_ml.texture_gen.prop_catalog_packs import (
    BATCH_8_CAFE_RETAIL_IDS,
    BATCH_9_INDUSTRIAL_FARM_IDS,
    BATCH_10_SCIFI_OFFICE_STREET_IDS,
    PACK_PROP_SPECS,
)
from satya_ml.texture_gen.prop_catalog_world import (
    BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS,
    BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS,
    WORLD_PROP_SPECS,
)

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

BATCH_2_CLINIC_PROP_IDS: tuple[str, ...] = (
    "cot",
    "lab_bench",
    "fume_hood",
    "air_gapped_terminal",
    "biometric_turnstile",
    "water_purifier_kiosk",
    "battery_bank_wall",
    "recliner_chair",
    "desk",
    "steel_shelving",
    "dumpster",
    "ac_unit",
)

_CLINIC_PROP_SPECS: tuple[PropSpriteSpec, ...] = (
    # --- Batch 1: Pilot Clinic & Vault Pack (12 props) ---
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
        prompt=prop_prompt(
            "Horizontal landscape modern hospital ward bed (twice as wide as tall); "
            "brushed aluminum headboard and crisp white cotton pillow on the LEFT end, "
            "neatly folded pale sky-blue quilted hospital blanket covering the mattress "
            "extending RIGHT, brushed steel side guard rails along top and bottom edges, "
            "compact wheeled steel corner posts."
        ),
        variant_directives=(
            "Standard pale sky-blue blanket with neatly made corners.",
            "Sage-green hospital blanket folded slightly back at the pillow with a dark clipboard resting on the foot.",
            "Warm cream thermal blanket with a folded navy fleece throw across the foot end and slight natural fabric creases.",
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
        prompt=prop_prompt(
            "Vertical portrait wheeled emergency trauma stretcher gurney (twice as tall "
            "as wide); white padded foam mattress with small headrest pillow at the TOP "
            "end, dark navy safety strap across the middle, brushed stainless steel "
            "folding side rails along left and right edges, steel push handles at top "
            "and bottom ends."
        ),
        variant_directives=(
            "Clean white trauma mattress with navy restraint strap.",
            "Charcoal vinyl EMS stretcher pad with two bright orange safety harness straps and a compact oxygen cylinder bracket at the head.",
            "Pale blue waterproof pad with folded yellow trauma blanket across the lower half and stainless IV pole socket.",
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
        prompt=prop_prompt(
            "Horizontal landscape clinical surgical operating table (twice as wide as "
            "tall); contoured headrest cushion on the LEFT end, stainless steel chassis "
            "with sterile seafoam-teal surgical drape pad across the center, compact "
            "stainless instrument tray attached on one side, articulated circular OR "
            "surgical light arm tucked slightly over the center-right."
        ),
        variant_directives=(
            "Seafoam-teal surgical drape with circular OR lamp overhead.",
            "Sterile cobalt-blue waterproof surgical drape with twin padded arm-rest wings extended and stainless kidney basin on side tray.",
            "Clean matte-black articulated carbon surgical pad with dual compact LED lamp pods and neat stainless scalpel tray.",
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
        prompt=prop_prompt(
            "Square footprint white medical MRI / CT diagnostic scanner viewed straight "
            "down from above; wide matte-white molded composite donut gantry housing "
            "positioned in the TOP half with a subtle glowing soft cyan circular inner "
            "bore ring and fine control indicators, and a narrow motorized sliding "
            "patient cradle bed extending straight DOWNWARD toward the bottom edge."
        ),
        variant_directives=(
            "Matte-white gantry with soft cyan bore glow.",
            "Clinical cool-grey and white dual-ring CT gantry with amber diagnostic laser line indicator and navy padded cradle.",
            "2035 slate-and-white superconducting MRI housing with copper shielding trim vents and pale-teal cradle pad.",
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
        prompt=prop_prompt(
            "Wide modern hospital nursing & intake reception station desk (3:2 landscape); "
            "curved light warm-oak and matte-white composite counter wrapping along the "
            "TOP edge and left/right wings with an open staff bay at the bottom, three "
            "slim dark flat-screen monitors and keyboards along the top counter, neat "
            "medical clipboards and tablet, two dark charcoal rolling swivel task chairs "
            "tucked inside the bottom staff opening."
        ),
        variant_directives=(
            "Light oak counter with white lip, 3 monitors, and 2 charcoal chairs.",
            "Brushed steel and cool-slate clinic triage counter with blue acrylic divider lip, dual wide monitors, and two teal swivel stools.",
            "Warm walnut and matte-cream reception desk with red first-aid binder, coffee mug, scattered intake charts, and two dark mesh chairs.",
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
        prompt=prop_prompt(
            "Compact pair of bedside medical devices seen straight down on a square "
            "canvas: on the LEFT, a dark charcoal wheeled rolling medical telemetry "
            "cart topped with a compact patient vitals monitor screen showing a crisp "
            "green ECG waveform; on the RIGHT, a brushed chrome five-star caster IV "
            "drip pole base with a translucent hanging saline pouch seen from above."
        ),
        variant_directives=(
            "Rolling charcoal ECG cart on left and chrome IV drip stand on right.",
            "White/teal clinical infusion pump cart with amber waveform screen on left and twin-bag chrome IV stand on right.",
            "Compact ruggedized field-trauma defibrillator/monitor stand on left with coiled leads and matte-steel IV pole on right.",
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
        prompt=prop_prompt(
            "Horizontal landscape hospital clean linen and medical supply crash cart "
            "(twice as wide as tall); brushed stainless steel rectangular frame with "
            "tubular push handles on left and right ends, neatly stacked folded white "
            "towels and pale-blue hospital sheets on the left half, and a crimson-red "
            "emergency medical tray with labeled sterile gauze packs on the right half."
        ),
        variant_directives=(
            "Stainless cart with folded white/blue linens left and red emergency tray right.",
            "Powder-coated light-grey crash cart with bright red top drawer, defibrillator paddles, oxygen bottle slot, and stacked sterile bandages.",
            "All-stainless linen trolley stacked with sage-green surgical gowns, white pillowcases, and a blue glove dispenser box.",
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
        prompt=prop_prompt(
            "Pair of rectangular clinical waste bins side-by-side viewed straight down "
            "from above: on the LEFT, a crimson-red medical biohazard sharps container "
            "with a recessed dark lid and small yellow biohazard emblem; on the RIGHT, "
            "a cobalt-blue hinged soiled-linen hamper bin with pale blue lid."
        ),
        variant_directives=(
            "Crimson biohazard bin left and cobalt soiled-linen bin right.",
            "Bright yellow sharps disposal bin left with dark chute slot, paired with a brushed steel pedal waste bin lined with white bag on right.",
            "Red step-on infectious waste bin left paired with a canvas-bagged rolling hospital laundry hamper frame on right.",
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
        prompt=prop_prompt(
            "Horizontal landscape 2035 near-future cryogenic quantum compute rack cabinet "
            "(twice as wide as tall); matte dark gunmetal-slate steel top enclosure with "
            "precision slatted cooling fans, frosted stainless cryogenic coolant pipes, "
            "a crisp glowing cyan diagnostic LED strip near the top edge, and heavy "
            "insulated copper busbar manifold along the bottom edge."
        ),
        variant_directives=(
            "Gunmetal cryo cabinet with 3 slatted fans and cyan LED strip.",
            "Matte-obsidian Faraday-shielded QPU rack with twin frosted liquid-nitrogen canisters, amber status indicators, and braided copper grounding strap.",
            "Industrial silver-slate cryo compute blade housing with radial turbine vents, frost condensation lines, and glowing ice-blue telemetry bar.",
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
        prompt=prop_prompt(
            "Circular top-down 2035 cryogenic biological seed-vault and bio-compute "
            "containment vat seen straight down from above; heavy bolted gunmetal steel "
            "outer ring collar, thick circular viewport glass filled with luminous "
            "pale-cyan cryogenic fluid and subtle suspended seed vials, light frost "
            "condensation on the rim, and a thin brass/copper sensor crossbar."
        ),
        variant_directives=(
            "Gunmetal collar with luminous pale-cyan fluid and copper crossbar.",
            "Brushed steel cryo-dewar tank with emerald-teal nutrient solution, concentric sample tray carousel visible inside, and pressure dial.",
            "Heavy weathered bronze-and-steel Faraday bio-vat with deep sapphire cryogenic mist, frosted valve wheel, and braided sensor cable.",
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
        prompt=prop_prompt(
            "Horizontal landscape weathered industrial backup diesel microgrid generator "
            "enclosure (twice as wide as tall); heavy industrial ochre-amber painted "
            "steel canopy with dark louvered radiator exhaust grilles, steel lifting "
            "lugs, dark cylindrical muffler exhaust pipe on top, and a recessed black "
            "breaker control panel on the right end."
        ),
        variant_directives=(
            "Ochre-amber steel enclosure with dark muffler pipe.",
            "Olive-drab military surplus diesel genset canopy with weathered rust streaks, yellow warning stencil block, and twin exhaust louvers.",
            "Industrial slate-grey and safety-red microgrid generator housing with heavy ribbed radiator roof, fuel cap, and thick conduit collar.",
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
        prompt=prop_prompt(
            "Vertical portrait top-down 2035 mutual-aid clinic utility van / ambulance "
            "support vehicle seen strictly from 90 degrees directly overhead (much taller "
            "than wide); rear cargo doors at the TOP edge, clean matte off-white and "
            "cool-grey painted roof with black roof-rack rails, compact rooftop HVAC pod, "
            "a crisp red medical cross painted cleanly on the center roof, side mirrors, "
            "dark tinted front windshield and sloped hood pointing straight DOWNWARD "
            "toward the bottom edge."
        ),
        variant_directives=(
            "Matte off-white clinic roof with red medical cross and roof rack.",
            "Weathered slate-blue municipal response van with matte-black solar roof panel, amber lightbar across front cab, and rear ladder rack.",
            "Rugged olive-grey 2035 mobile-clinic box van with rooftop AC condenser, spare tire mount on roof rack, and red cross decal.",
        ),
    ),
    # --- Batch 2: Clinic Lobby, Wards, Labs, Vault & Exterior (12 props) ---
    PropSpriteSpec(
        id="cot",
        prop_kind_code="COT",
        filename="cot.webp",
        label="Folding Triage / Camp Cot",
        base_w_cells=1,
        base_h_cells=2,
        target_w_px=256,
        target_h_px=512,
        aspect_ratio="9:16",
        prompt=prop_prompt(
            "Vertical portrait folding emergency triage / camp cot seen strictly straight "
            "down from above (twice as tall as wide); tubular aluminum folding X-frame "
            "corners, taut olive-drab heavy canvas sleeping deck, small white pillow "
            "at the TOP end, and neatly folded grey wool blanket across the lower half."
        ),
        variant_directives=(
            "Olive canvas deck with white pillow at top and grey wool blanket.",
            "Navy-blue nylon field cot with charcoal foam sleeping pad, small inflatable pillow at top, and folded pale-blue sheet.",
            "Tan/khaki heavy duck-canvas triage cot with stitched seam pockets and dark olive thermal emergency blanket.",
        ),
    ),
    PropSpriteSpec(
        id="lab_bench",
        prop_kind_code="LAB_BENCH",
        filename="lab_bench.webp",
        label="Laboratory Workbench",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape chemical/medical laboratory workbench seen straight "
            "down from above (twice as wide as tall); matte dark-charcoal epoxy resin "
            "countertop with steel edge trim, compact binocular microscope on the left, "
            "rack of blue/amber glass reagent vials in the center, and a dark tablet "
            "with stainless specimen tray on the right."
        ),
        variant_directives=(
            "Dark epoxy bench with microscope, vial rack, and tablet.",
            "Dark slate lab table holding a round benchtop centrifuge, stack of clear petri dishes, micropipette stand, and notebook.",
            "Brushed-steel and charcoal diagnostic bench with portable PCR analyzer box, insulated sample cooler, and clipboard.",
        ),
    ),
    PropSpriteSpec(
        id="fume_hood",
        prop_kind_code="FUME_HOOD",
        filename="fume_hood.webp",
        label="Chemical Fume Hood & Biosafety Cabinet",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape laboratory chemical fume hood / biosafety cabinet "
            "seen straight down from above (twice as wide as tall); clean off-white "
            "and brushed stainless steel top housing with circular exhaust duct collar "
            "near the TOP rear edge, glowing cyan status bar along front lip, and "
            "recessed dark interior work bay partially visible through clear sash edge."
        ),
        variant_directives=(
            "Off-white & steel hood with exhaust collar and cyan status bar.",
            "Clinical light-grey Class-II biosafety cabinet with HEPA filter housing panel, teal status strip, and visible stainless work tray.",
            "Industrial matte-slate fume extraction hood with twin galvanized duct ports on top and amber chemical safety label strip.",
        ),
    ),
    PropSpriteSpec(
        id="air_gapped_terminal",
        prop_kind_code="AIR_GAPPED_TERMINAL",
        filename="air_gapped_terminal.webp",
        label="Optical-Diode Air-Gapped Terminal",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square footprint 2035 Faraday-shielded air-gapped computer terminal desk "
            "seen straight down from above; dark brushed steel Faraday desk chassis "
            "with copper grounding bar along the TOP edge, twin angled matte-black "
            "monitors showing subtle green terminal text, compact mechanical keyboard, "
            "and sealed optical-diode drive bay."
        ),
        variant_directives=(
            "Steel Faraday desk with twin green-glow monitors and copper bar.",
            "Ruggedized gunmetal cyber-diagnostic station with wide curved monitor showing amber waveforms, trackball, and orange fiber patch cable.",
            "Compact white-and-slate vault workstation with single crisp cyan readout display, smartcard reader, and stacked log drives.",
        ),
    ),
    PropSpriteSpec(
        id="biometric_turnstile",
        prop_kind_code="BIOMETRIC_TURNSTILE",
        filename="biometric_turnstile.webp",
        label="Biometric Gait & Iris Checkpoint Gate",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape security biometric turnstile gate pair seen straight "
            "down from above (twice as wide as tall); two sleek brushed stainless-steel "
            "rectangular lane pedestals along TOP and BOTTOM edges with glowing green "
            "status indicator dots and translucent sky-blue tempered glass swing-arms "
            "meeting across the center walkway."
        ),
        variant_directives=(
            "Brushed steel twin pedestals with translucent blue glass arms.",
            "Matte dark-gunmetal 2035 security checkpoint pedestals with amber iris-scanner lenses and smoked glass barrier paddles.",
            "Clean white-and-chrome hospital intake gate with glowing cyan LED lane strips and slim clear acrylic paddles.",
        ),
    ),
    PropSpriteSpec(
        id="water_purifier_kiosk",
        prop_kind_code="WATER_PURIFIER_KIOSK",
        filename="water_purifier_kiosk.webp",
        label="Municipal Ration Water & Misting Kiosk",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down 2035 municipal ration water purifier and cooling mist "
            "kiosk seen straight down from above; compact octagonal/rounded stainless-steel "
            "chassis with sky-blue armored top cap, two cylindrical filter canister "
            "caps, analog pressure dial, and recessed stainless drip-grate tray on "
            "the front edge."
        ),
        variant_directives=(
            "Stainless chassis with sky-blue cap, twin filter canisters, and drip grate.",
            "Weathered teal-grey street ration-water dispenser with matte-black solar roof cell, brass shutoff valve, and grated basin.",
            "Clean white medical hydration station with cobalt-blue UV indicator ring, steel nozzle bar, and stainless catch grate.",
        ),
    ),
    PropSpriteSpec(
        id="battery_bank_wall",
        prop_kind_code="BATTERY_BANK_WALL",
        filename="battery_bank_wall.webp",
        label="Modular LFP Microgrid Battery Cabinet",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape modular 2035 LFP microgrid battery bank cabinet seen "
            "straight down from above (twice as wide as tall); matte dark-charcoal "
            "steel rack housing four modular battery packs, heavy copper busbar "
            "interconnects along the top edge, finned aluminum cooling vents, and "
            "crisp green LED charge level indicators."
        ),
        variant_directives=(
            "Dark charcoal battery modules with copper busbars and green LEDs.",
            "Industrial slate-grey UPS battery wall with high-voltage yellow/black hazard trim, ribbed heat sinks, and amber charge bar.",
            "Clean matte-white and dark-graphite clinic power-wall modules with cyan telemetry LED strip and insulated conduit glands.",
        ),
    ),
    PropSpriteSpec(
        id="recliner_chair",
        prop_kind_code="RECLINER_CHAIR",
        filename="recliner_chair.webp",
        label="Visitor Sleeper Recliner",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square top-down upholstered clinical visitor sleeper recliner chair seen "
            "straight down from above; plump padded slate-blue vinyl seat cushion, "
            "thick curved backrest along the TOP edge, wide padded armrests left and "
            "right, and subtle dark seam tufting."
        ),
        variant_directives=(
            "Slate-blue padded vinyl recliner chair.",
            "Muted sage-green medical waiting recliner with light oak armrest caps and a folded beige throw pillow in the corner.",
            "Warm taupe/tan quilted leatherette recliner with dark charcoal piping and slightly extended footrest lip.",
        ),
    ),
    PropSpriteSpec(
        id="desk",
        prop_kind_code="DESK",
        filename="desk.webp",
        label="Desk & Rolling Chair",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape office desk and rolling task chair seen straight "
            "down from above (twice as wide as tall); warm light-oak wooden tabletop "
            "along the TOP half with open matte-black laptop, white paper folder, and "
            "ceramic mug, paired with a dark charcoal swivel office chair tucked "
            "neatly at the BOTTOM center."
        ),
        variant_directives=(
            "Light oak desk with laptop, folder, mug, and charcoal swivel chair.",
            "Matte-white modern clinic intake desk with slim desktop monitor, keyboard, stethoscope, notepad, and slate-blue swivel chair.",
            "Warm walnut doctor desk with brass-rimmed desk lamp, twin tablets, stacked binders, and black leatherette task chair.",
        ),
    ),
    PropSpriteSpec(
        id="steel_shelving",
        prop_kind_code="STEEL_SHELVING",
        filename="steel_shelving.webp",
        label="Steel Supply Shelving",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape industrial steel supply shelving unit seen straight "
            "down from above (twice as wide as tall); brushed galvanized steel frame "
            "posts at corners, top shelf packed neatly with white and pale-blue "
            "medical supply cartons, grey plastic storage bins, and sealed sterile "
            "packs."
        ),
        variant_directives=(
            "Steel shelf packed with white/blue medical cartons and grey bins.",
            "Heavy-duty charcoal steel rack holding assorted brown cardboard supply boxes, red first-aid kits, and dark blue storage totes.",
            "Brushed chrome clinic pharmacy rack stocked with stainless organizer trays, amber glass bottle crates, and folded green gown packs.",
        ),
    ),
    PropSpriteSpec(
        id="dumpster",
        prop_kind_code="DUMPSTER",
        filename="dumpster.webp",
        label="Commercial Dumpster",
        base_w_cells=2,
        base_h_cells=1,
        target_w_px=512,
        target_h_px=256,
        aspect_ratio="16:9",
        prompt=prop_prompt(
            "Horizontal landscape commercial steel alley dumpster seen strictly straight "
            "down from above (twice as wide as tall); weathered dark forest-green "
            "ribbed steel body with corner forklift pockets and twin closed ribbed "
            "black heavy-duty plastic lids across the top."
        ),
        variant_directives=(
            "Weathered dark forest-green steel body with closed black plastic lids.",
            "Faded slate-blue commercial bin with one black lid closed and the right lid hinged open revealing dark tied trash bags inside.",
            "Rusted olive-grey industrial waste dumpster with weathered yellow hazard corner stripes and scratched black lids.",
        ),
    ),
    PropSpriteSpec(
        id="ac_unit",
        prop_kind_code="AC_UNIT",
        filename="ac_unit.webp",
        label="Exterior HVAC / AC Condenser",
        base_w_cells=1,
        base_h_cells=1,
        target_w_px=256,
        target_h_px=256,
        aspect_ratio="1:1",
        prompt=prop_prompt(
            "Square footprint exterior HVAC / AC condenser unit seen straight down "
            "from above; clean light-grey painted galvanized sheet-steel square housing "
            "with recessed circular black protective fan grille showing dark curved "
            "fan blades inside, corner bolts, and insulated copper pipe stub."
        ),
        variant_directives=(
            "Light-grey steel housing with circular black fan grille.",
            "Sun-weathered beige-grey commercial condenser unit with light surface grime, twin copper coolant stubs, and heavy wire fan cage.",
            "Dark gunmetal-slate industrial rooftop chiller unit with recessed turbine shroud, aluminum radiator fin border, and breaker box.",
        ),
    ),
)

PROP_SPRITE_CATALOG: tuple[PropSpriteSpec, ...] = (
    _CLINIC_PROP_SPECS + INTERIOR_PROP_SPECS + WORLD_PROP_SPECS + PACK_PROP_SPECS
)

_BATCH_IDS: dict[str, tuple[str, ...]] = {
    "pilot": PILOT_CLINIC_PROP_IDS,
    "batch1": PILOT_CLINIC_PROP_IDS,
    "batch2": BATCH_2_CLINIC_PROP_IDS,
    "batch3": BATCH_3_LIVING_BEDROOM_IDS,
    "batch4": BATCH_4_KITCHEN_BATH_BAR_IDS,
    "batch5": BATCH_5_OFFICE_CLINICAL_PORTALS_IDS,
    "batch6": BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS,
    "batch7": BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS,
    "batch8": BATCH_8_CAFE_RETAIL_IDS,
    "batch9": BATCH_9_INDUSTRIAL_FARM_IDS,
    "batch10": BATCH_10_SCIFI_OFFICE_STREET_IDS,
    "packs": (
        BATCH_8_CAFE_RETAIL_IDS
        + BATCH_9_INDUSTRIAL_FARM_IDS
        + BATCH_10_SCIFI_OFFICE_STREET_IDS
    ),
    "remaining": (
        BATCH_3_LIVING_BEDROOM_IDS
        + BATCH_4_KITCHEN_BATH_BAR_IDS
        + BATCH_5_OFFICE_CLINICAL_PORTALS_IDS
        + BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS
        + BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS
    ),
}


def get_prop_sprite_spec(sprite_id: str) -> PropSpriteSpec:
    """Lookup one [`PropSpriteSpec`] by ID."""
    for spec in PROP_SPRITE_CATALOG:
        if spec.id == sprite_id:
            return spec
    raise ValueError(f"Unknown prop sprite ID: {sprite_id!r}")


def select_prop_sprite_specs(
    *, pilot: bool = False, batch: str | None = None, only: str | None = None
) -> list[PropSpriteSpec]:
    """Select all, batch (`pilot`, `batch1`..`batch7`, `remaining`, `all`), or comma-separated specs."""
    if only:
        wanted = [item.strip() for item in only.split(",") if item.strip()]
        return [get_prop_sprite_spec(item) for item in wanted]
    if batch is not None:
        key = batch.strip().lower()
        if key == "all":
            return list(PROP_SPRITE_CATALOG)
        ids = _BATCH_IDS.get(key)
        if ids is None:
            raise ValueError(f"Unknown prop sprite batch: {batch!r}")
        return [get_prop_sprite_spec(item) for item in ids]
    if pilot:
        return [get_prop_sprite_spec(item) for item in PILOT_CLINIC_PROP_IDS]
    return list(PROP_SPRITE_CATALOG)


__all__ = [
    "BATCH_2_CLINIC_PROP_IDS",
    "BATCH_3_LIVING_BEDROOM_IDS",
    "BATCH_4_KITCHEN_BATH_BAR_IDS",
    "BATCH_5_OFFICE_CLINICAL_PORTALS_IDS",
    "BATCH_6_LAB_RETAIL_INDUSTRIAL_IDS",
    "BATCH_7_OUTDOOR_VEHICLES_INFRA_IDS",
    "BATCH_8_CAFE_RETAIL_IDS",
    "BATCH_9_INDUSTRIAL_FARM_IDS",
    "BATCH_10_SCIFI_OFFICE_STREET_IDS",
    "PACK_PROP_SPECS",
    "PILOT_CLINIC_PROP_IDS",
    "PROP_SPRITE_CATALOG",
    "PROP_STYLE_PREAMBLE",
    "PropSpriteSpec",
    "get_prop_sprite_spec",
    "prop_prompt",
    "select_prop_sprite_specs",
]
