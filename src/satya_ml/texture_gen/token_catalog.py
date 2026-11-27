"""Top-down 2D battlemap character, drone, and android Style C sprite catalog."""

from __future__ import annotations

from dataclasses import dataclass

TOKEN_STYLE_C_PREAMBLE = (
    "Strictly 90-degree orthographic top-down bird's-eye view looking straight down from "
    "directly overhead, Style C painted cel-shaded 3D illustration (Disney+ What If...? "
    "influence) with crisp sculpted volumes, bold angular black ink contour outlines, hard "
    "stepped cel-shadow terminators, directional hand-painted brushstroke texture, and sharp "
    "upper-left rim highlights, grounded near-future 2035 Earth threshold realism (NO glowing "
    "cyberpunk neon eyes, NO space-marine sci-fi armor), single centered isolated figure "
    "FACING STRAIGHT UP TOWARD THE TOP EDGE OF THE FRAME (12 O'CLOCK = FORWARD), NO perspective "
    "tilt, NO side view, NO isometric angle, NO floor tiles, NO circular base ring, NO cast "
    "drop shadow on the background, NO border, NO watermark, NO text labels, clean margin "
    "on a COMPLETELY FLAT UNIFORM SOLID NEON MAGENTA (#FF00FF) CHROMA-KEY BACKGROUND:"
)


def token_prompt(body: str) -> str:
    """Wrap a token body prompt with the strict 90-degree Style C 12-o'clock preamble."""
    return f"{TOKEN_STYLE_C_PREAMBLE} {body}"


@dataclass(frozen=True)
class TokenSpriteSpec:
    """One top-down Style C character/drone/android sprite baked to transparent WebP."""

    id: str
    preset: str
    stance: str
    batch: str
    filename: str
    label: str
    target_w_px: int
    target_h_px: int
    aspect_ratio: str
    prompt: str
    variant_directives: tuple[str, str, str] = ("", "", "")
    key_color: str = "magenta"
    pad_frac: float = 0.05

    def variant_filename(self, idx: int) -> str:
        return self.filename if idx == 0 else f"{self.id}_v{idx}.webp"

    def variant_raw_stem(self, idx: int) -> str:
        return self.id if idx == 0 else f"{self.id}_v{idx}"

    def prompt_for_variant(self, idx: int) -> str:
        if idx <= 0 or not self.variant_directives[idx].strip():
            return self.prompt
        return f"{self.prompt} Variant detail: {self.variant_directives[idx]}"


PILOT_TOKEN_IDS: tuple[str, ...] = (
    "civilian_handgun_standing",
    "soldier_rifle_prone",
    "drone_quad_standing",
)

BATCH_CIVILIANS_IDS: tuple[str, ...] = (
    "civilian_unarmed_standing",
    "civilian_unarmed_crouched",
    "civilian_unarmed_prone",
    "civilian_handgun_standing",
    "civilian_handgun_crouched",
    "civilian_handgun_prone",
    "civilian_rifle_standing",
    "civilian_rifle_crouched",
    "civilian_rifle_prone",
    "civilian_knife_standing",
    "civilian_knife_crouched",
)

BATCH_SECURITY_IDS: tuple[str, ...] = (
    "security_handgun_standing",
    "security_handgun_crouched",
    "security_handgun_prone",
    "soldier_rifle_standing",
    "soldier_rifle_crouched",
    "soldier_rifle_prone",
    "sniper_rifle_standing",
    "sniper_rifle_crouched",
    "sniper_rifle_prone",
)

BATCH_DRONES_IDS: tuple[str, ...] = (
    "drone_quad_standing",
    "drone_aero_standing",
)

BATCH_ANDROIDS_IDS: tuple[str, ...] = (
    "android_domestic_standing",
    "android_domestic_crouched",
    "android_industrial_standing",
    "android_industrial_crouched",
    "android_security_standing",
    "android_security_crouched",
    "android_security_prone",
)

BATCH_MOVE_CIVILIANS_IDS: tuple[str, ...] = (
    "civilian_unarmed_walking_holstered",
    "civilian_unarmed_tactical_advance",
    "civilian_unarmed_sprinting",
    "civilian_handgun_walking_holstered",
    "civilian_handgun_tactical_advance",
    "civilian_handgun_sprinting",
    "civilian_rifle_walking_holstered",
    "civilian_rifle_tactical_advance",
    "civilian_rifle_sprinting",
    "civilian_knife_walking_holstered",
    "civilian_knife_tactical_advance",
    "civilian_knife_sprinting",
)

BATCH_MOVE_SECURITY_IDS: tuple[str, ...] = (
    "security_handgun_walking_holstered",
    "security_handgun_tactical_advance",
    "security_handgun_sprinting",
    "soldier_rifle_walking_holstered",
    "soldier_rifle_tactical_advance",
    "soldier_rifle_sprinting",
    "sniper_rifle_walking_holstered",
    "sniper_rifle_tactical_advance",
    "sniper_rifle_sprinting",
)

BATCH_MOVE_ANDROIDS_IDS: tuple[str, ...] = (
    "android_domestic_walking_holstered",
    "android_domestic_tactical_advance",
    "android_domestic_sprinting",
    "android_industrial_walking_holstered",
    "android_industrial_tactical_advance",
    "android_industrial_sprinting",
    "android_security_walking_holstered",
    "android_security_tactical_advance",
    "android_security_sprinting",
)

BATCH_MOVEMENT_IDS: tuple[str, ...] = (
    *BATCH_MOVE_CIVILIANS_IDS,
    *BATCH_MOVE_SECURITY_IDS,
    *BATCH_MOVE_ANDROIDS_IDS,
)


_ARCHETYPE_VARIANTS: dict[str, tuple[str, str, str]] = {
    "civilian_unarmed": (
        "Worn olive-canvas utility jacket, short dark hair, dark jeans.",
        "Faded burgundy-rust windbreaker, dark tied-back hair, small charcoal daypack.",
        "Charcoal denim work shirt, sandy-brown short hair, weathered tan shoulder bag.",
    ),
    "civilian_handgun": (
        "Faded slate-blue hoodie, canvas messenger strap, dark trousers.",
        "Worn dark-green bomber jacket, dark hair in a tight ponytail, grey utility pants.",
        "Muted ochre-tan work jacket over charcoal tee, close-cropped hair, dark jeans.",
    ),
    "civilian_rifle": (
        "Dusty earth-tan utility vest over sage shirt and field cap.",
        "Weathered dark denim overshirt, olive rifle sling, dark braided hair.",
        "Matte charcoal rain shell, dark knit beanie, khaki field trousers.",
    ),
    "civilian_knife": (
        "Rumpled tan-brown field coat over dark collared shirt, dark hair greying at temples.",
        "Dark olive streetwear hoodie, compact utility belt, short black hair.",
        "Worn charcoal leather field jacket, shoulder-length dark hair, dark jeans.",
    ),
    "security_handgun": (
        "Dark navy tactical windbreaker over charcoal vest, peaked cap.",
        "Matte charcoal corporate security plate vest, comms earpiece, dark tied hair.",
        "Dark slate municipal patrol uniform with subtle muted hi-vis trim, close-cropped hair.",
    ),
    "soldier_rifle": (
        "Greenish-grey optical camo uniform, matte olive-grey plate carrier and ballistic helmet.",
        "Darker slate-sage urban optical camo uniform, comms helmet and compact chest rig.",
        "Dusty arid olive-grey optical camo uniform, matte helmet and compact assault pack.",
    ),
    "sniper_rifle": (
        "Muted greenish-grey optical camo hooded scrim mantle.",
        "Urban slate-charcoal netting veil over hood and matte-wrapped suppressor.",
        "Dry brush tan-olive leafy scrim cloak and matte desert-drab barrel wrap.",
    ),
    "drone_quad": (
        "Olive-grey and matte-charcoal armored quadruped chassis.",
        "Matte charcoal-slate urban security quadruped with subtle muted amber dorsal trim.",
        "Weathered sand-drab field quadruped with compact matte-black dorsal lidar pod.",
    ),
    "drone_aero": (
        "Matte dark-carbon tactical quadrotor fuselage and black rotor discs.",
        "Muted slate-grey municipal ISR quadrotor with subtle amber rotor-tip stripes.",
        "Olive-drab military tactical quadrotor with twin-lens sensor gimbal nose.",
    ),
    "android_domestic": (
        "Matte off-white and warm pale-grey polymer shell with fine charcoal joint seams.",
        "Soft sage-tinted matte ceramic polymer shell with muted slate-teal waist band.",
        "Warm sandstone-beige matte polymer shell with charcoal caregiver utility sash.",
    ),
    "android_industrial": (
        "Brushed aluminum and dark charcoal torso with weathered safety-amber shoulder stripes.",
        "Weathered industrial steel-blue casing with matte black-and-white hazard chevrons.",
        "Matte burnt-orange heavy logistics chassis with dark titanium actuator joints.",
    ),
    "android_security": (
        "Matte gunmetal-slate and muted olive-grey angular composite armor.",
        "Dark charcoal-navy urban enforcement plating with subtle light-grey shoulder stenciling.",
        "Muted sage-camo composite ballistic armor with compact rear battery pack.",
    ),
}


def _square_stance_spec(
    *,
    preset: str,
    stance: str,
    stance_label: str,
    batch: str,
    label: str,
    body: str,
) -> TokenSpriteSpec:
    slug = f"{preset}_{stance}"
    return TokenSpriteSpec(
        id=slug,
        preset=preset,
        stance=stance,
        batch=batch,
        filename=f"{slug}.webp",
        label=f"{label} ({stance_label})",
        target_w_px=512,
        target_h_px=512,
        aspect_ratio="1:1",
        prompt=token_prompt(body),
        variant_directives=_ARCHETYPE_VARIANTS[preset],
        pad_frac=0.05,
    )


def _standing_spec(*, preset: str, batch: str, label: str, body: str) -> TokenSpriteSpec:
    return _square_stance_spec(
        preset=preset, stance="standing", stance_label="Standing", batch=batch, label=label, body=body
    )


def _kneeling_spec(*, preset: str, batch: str, label: str, body: str) -> TokenSpriteSpec:
    return _square_stance_spec(
        preset=preset, stance="crouched", stance_label="Kneeling", batch=batch, label=label, body=body
    )


def _walking_holstered_spec(*, preset: str, batch: str, label: str, body: str) -> TokenSpriteSpec:
    return _square_stance_spec(
        preset=preset,
        stance="walking_holstered",
        stance_label="Walking (Holstered)",
        batch=batch,
        label=label,
        body=body,
    )


def _tactical_advance_spec(*, preset: str, batch: str, label: str, body: str) -> TokenSpriteSpec:
    return _square_stance_spec(
        preset=preset,
        stance="tactical_advance",
        stance_label="Tactical / Bounding",
        batch=batch,
        label=label,
        body=body,
    )


def _sprinting_spec(*, preset: str, batch: str, label: str, body: str) -> TokenSpriteSpec:
    return _square_stance_spec(
        preset=preset,
        stance="sprinting",
        stance_label="Sprinting",
        batch=batch,
        label=label,
        body=body,
    )


def _prone_spec(
    *,
    preset: str,
    batch: str,
    label: str,
    body: str,
) -> TokenSpriteSpec:
    slug = f"{preset}_prone"
    return TokenSpriteSpec(
        id=slug,
        preset=preset,
        stance="prone",
        batch=batch,
        filename=f"{slug}.webp",
        label=f"{label} (Prone)",
        target_w_px=384,
        target_h_px=576,
        aspect_ratio="2:3",
        prompt=token_prompt(body),
        variant_directives=_ARCHETYPE_VARIANTS[preset],
        pad_frac=0.04,
    )


TOKEN_SPRITE_CATALOG: tuple[TokenSpriteSpec, ...] = (
    # --- Civilians (11) ---
    _standing_spec(
        preset="civilian_unarmed",
        batch="civilians",
        label="Civilian Empty-Handed",
        body=(
            "Standing ordinary 2035 civilian seen strictly straight down from overhead, facing 12 o'clock: "
            "short dark hair, worn olive-canvas utility jacket over charcoal tee, dark jeans, empty relaxed "
            "hands slightly forward at sides, compact standing shoulder-to-hip silhouette with toes of dark "
            "sneakers barely peeking forward."
        ),
    ),
    _kneeling_spec(
        preset="civilian_unarmed",
        batch="civilians",
        label="Civilian Empty-Handed",
        body=(
            "Kneeling ordinary 2035 civilian seen strictly straight down from overhead, facing 12 o'clock: "
            "short dark hair, worn olive-canvas utility jacket, crouched low on one knee with shoulders "
            "hunched forward compactly, one hand resting near forward knee, rear boot sole tucked behind at 6 o'clock."
        ),
    ),
    _prone_spec(
        preset="civilian_unarmed",
        batch="civilians",
        label="Civilian Empty-Handed",
        body=(
            "Prone ordinary 2035 civilian lying flat on stomach seen strictly straight down from overhead, "
            "head and folded forearms tucked near 12 o'clock top, worn olive-canvas utility jacket back, "
            "dark jeans and sneaker heels stretched straight down toward 6 o'clock bottom."
        ),
    ),
    _standing_spec(
        preset="civilian_handgun",
        batch="civilians",
        label="Civilian Handgun",
        body=(
            "Standing 2035 civilian with handgun seen strictly straight down from overhead, facing 12 o'clock: "
            "faded slate-blue utility hoodie (hood down) and canvas messenger strap, both arms extended forward "
            "toward 12 o'clock in an isosceles grip aiming a compact matte-black 9mm semiautomatic pistol straight up."
        ),
    ),
    _kneeling_spec(
        preset="civilian_handgun",
        batch="civilians",
        label="Civilian Handgun",
        body=(
            "Kneeling 2035 civilian with handgun seen strictly straight down from overhead, facing 12 o'clock: "
            "faded slate-blue hoodie, dropped low onto right knee in compact tactical crouch, both hands gripping "
            "a compact matte-black 9mm pistol extended forward toward 12 o'clock, rear boot tucked at 6 o'clock."
        ),
    ),
    _prone_spec(
        preset="civilian_handgun",
        batch="civilians",
        label="Civilian Handgun",
        body=(
            "Prone 2035 civilian lying flat on stomach with handgun seen strictly straight down from overhead: "
            "head near 12 o'clock, elbows braced on ground aiming a compact matte-black 9mm pistol straight up "
            "toward 12 o'clock, faded slate-blue jacket, dark trousers and boots stretched straight back to 6 o'clock."
        ),
    ),
    _standing_spec(
        preset="civilian_rifle",
        batch="civilians",
        label="Civilian Rifle",
        body=(
            "Standing 2035 civilian with rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "dusty earth-tan utility vest over muted sage long-sleeve shirt, brimmed field cap or dark hair, "
            "two-handed grip shouldering a matte dark-polymer hunting/carbine rifle pointed straight up at 12 o'clock."
        ),
    ),
    _kneeling_spec(
        preset="civilian_rifle",
        batch="civilians",
        label="Civilian Rifle",
        body=(
            "Kneeling 2035 civilian with rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "dusty earth-tan utility vest over sage shirt, dropped onto one knee with left elbow braced on forward "
            "knee, aiming a matte dark-polymer carbine rifle straight up toward 12 o'clock."
        ),
    ),
    _prone_spec(
        preset="civilian_rifle",
        batch="civilians",
        label="Civilian Rifle",
        body=(
            "Prone 2035 civilian lying flat on stomach with rifle seen strictly straight down from overhead: "
            "aiming a matte dark-polymer carbine rifle straight up toward 12 o'clock, dusty earth-tan utility vest, "
            "elbows splayed slightly, legs and rugged boots stretched straight back toward 6 o'clock."
        ),
    ),
    _standing_spec(
        preset="civilian_knife",
        batch="civilians",
        label="Civilian Knife",
        body=(
            "Standing 2035 civilian investigator with concealed kerambit knife seen strictly straight down from overhead, "
            "facing 12 o'clock: dark hair greying slightly at temples, rumpled tan-brown field coat over dark collared shirt, "
            "alert close-quarters posture with right arm angled forward gripping a matte-dark curved kerambit knife "
            "pointing toward 12 o'clock."
        ),
    ),
    _kneeling_spec(
        preset="civilian_knife",
        batch="civilians",
        label="Civilian Knife",
        body=(
            "Kneeling 2035 civilian investigator with kerambit knife seen strictly straight down from overhead, "
            "facing 12 o'clock: rumpled tan-brown coat over dark shirt, crouched low in a silat low-guard stance "
            "on one knee, left hand guarding centerline and right hand holding a dark curved kerambit blade ready toward 12 o'clock."
        ),
    ),
    # --- Police, Security & Military (9) ---
    _standing_spec(
        preset="security_handgun",
        batch="security",
        label="Police / Security Handgun",
        body=(
            "Standing 2035 municipal police / security officer seen strictly straight down from overhead, facing 12 o'clock: "
            "dark navy tactical windbreaker over matte-charcoal kevlar vest, duty belt with radio pouch, peaked cap or "
            "close-cropped hair, aiming a matte-black service pistol two-handed straight up toward 12 o'clock."
        ),
    ),
    _kneeling_spec(
        preset="security_handgun",
        batch="security",
        label="Police / Security Handgun",
        body=(
            "Kneeling 2035 municipal police / security officer seen strictly straight down from overhead, facing 12 o'clock: "
            "dark navy tactical jacket over charcoal kevlar vest, crouched on one knee in braced cover posture aiming a "
            "matte-black service pistol straight up toward 12 o'clock, rear tactical boot tucked at 6 o'clock."
        ),
    ),
    _prone_spec(
        preset="security_handgun",
        batch="security",
        label="Police / Security Handgun",
        body=(
            "Prone 2035 municipal police / security officer lying flat on stomach seen strictly straight down from overhead: "
            "dark navy jacket and charcoal vest, arms extended straight up toward 12 o'clock aiming matte-black service pistol, "
            "duty belt, dark cargo trousers and boots stretched straight back to 6 o'clock."
        ),
    ),
    _standing_spec(
        preset="soldier_rifle",
        batch="security",
        label="Security / Soldier Rifle",
        body=(
            "Standing 2035 DAISE tactical soldier seen strictly straight down from overhead, facing 12 o'clock: "
            "greenish-grey optical military camo uniform, matte olive-grey plate carrier vest and ballistic helmet with "
            "dark visor rim, shouldering a modern short-stroke piston tactical carbine rifle pointed straight up at 12 o'clock."
        ),
    ),
    _kneeling_spec(
        preset="soldier_rifle",
        batch="security",
        label="Security / Soldier Rifle",
        body=(
            "Kneeling 2035 DAISE tactical soldier seen strictly straight down from overhead, facing 12 o'clock: "
            "greenish-grey optical military camo uniform, matte olive-grey plate carrier and ballistic helmet, crouched "
            "tightly on one knee bracing a tactical carbine rifle aimed straight up toward 12 o'clock."
        ),
    ),
    _prone_spec(
        preset="soldier_rifle",
        batch="security",
        label="Security / Soldier Rifle",
        body=(
            "Prone 2035 DAISE tactical soldier lying flat on stomach seen strictly straight down from overhead: "
            "greenish-grey optical military camo uniform, matte ballistic helmet, bracing a modern tactical carbine rifle "
            "pointed straight up toward 12 o'clock, tactical pack and combat boots stretched straight back to 6 o'clock."
        ),
    ),
    _standing_spec(
        preset="sniper_rifle",
        batch="security",
        label="Military / Security Sniper",
        body=(
            "Standing 2035 military designated marksman seen strictly straight down from overhead, facing 12 o'clock: "
            "muted greenish-grey optical camo hooded scrim cloak over tactical chest rig, gripping a long-barrel suppressed "
            "precision sniper rifle with telescopic scope pointed straight up toward 12 o'clock."
        ),
    ),
    _kneeling_spec(
        preset="sniper_rifle",
        batch="security",
        label="Military / Security Sniper",
        body=(
            "Kneeling 2035 military designated marksman seen strictly straight down from overhead, facing 12 o'clock: "
            "muted greenish-grey camo hood and scrim mantle, crouched low on one knee bracing a long-barrel suppressed "
            "precision sniper rifle with optics scope pointed straight up toward 12 o'clock."
        ),
    ),
    _prone_spec(
        preset="sniper_rifle",
        batch="security",
        label="Military / Security Sniper",
        body=(
            "Prone 2035 military sniper lying flat on stomach seen strictly straight down from overhead: "
            "muted greenish-grey optical camo scrim cloak draped over shoulders and back, bipod-braced long suppressed "
            "precision sniper rifle with optics scope extending straight up to 12 o'clock, boots trailing to 6 o'clock."
        ),
    ),
    # --- Drones (2) ---
    _standing_spec(
        preset="drone_quad",
        batch="drones",
        label="Quadruped Drone (Echo-9)",
        body=(
            "2035 agile four-legged tactical quadruped ground robot (Echo-9 / Spot-style chassis) seen strictly straight down "
            "from overhead, facing 12 o'clock: compact rectangular olive-grey and matte-charcoal armored body with subtle "
            "dorsal battery latch, four articulated mechanical legs splayed symmetrically at the corners, and a non-glowing "
            "dark matte optical camera sensor bar at the 12 o'clock front edge."
        ),
    ),
    _standing_spec(
        preset="drone_aero",
        batch="drones",
        label="Aero Drone (Quadrotor UAV)",
        body=(
            "2035 matte dark-carbon tactical quadrotor reconnaissance UAV drone seen strictly straight down from overhead, "
            "facing 12 o'clock: sleek angular central sensor fuselage pointing toward 12 o'clock with tiny non-glowing dark "
            "gimbal camera lens, four slender diagonal X-frame carbon arms, and four circular twin-blade rotor rings at the tips."
        ),
    ),
    # --- Biped Androids (7) ---
    _standing_spec(
        preset="android_domestic",
        batch="androids",
        label="Android Domestic",
        body=(
            "Standing slender 2035 domestic caregiver biped android (Unitree G1 agile human proportions) seen strictly straight down "
            "from overhead, facing 12 o'clock: clean matte off-white and warm pale-grey sculpted polymer shell with fine charcoal "
            "shoulder/elbow joint seams, smooth matte rounded head facing 12 o'clock with zero glowing eyes, empty open hands."
        ),
    ),
    _kneeling_spec(
        preset="android_domestic",
        batch="androids",
        label="Android Domestic",
        body=(
            "Kneeling slender 2035 domestic caregiver biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "matte off-white and warm pale-grey sculpted polymer shell with charcoal joint seams, crouched compactly on one knee "
            "with arms steadied forward, smooth matte head facing 12 o'clock with zero glowing eyes."
        ),
    ),
    _standing_spec(
        preset="android_industrial",
        batch="androids",
        label="Android Industrial",
        body=(
            "Standing athletic 2035 industrial utility biped android (Boston Dynamics Atlas agile scale, NOT bulky Warhammer armor) "
            "seen strictly straight down from overhead, facing 12 o'clock: brushed aluminum and dark charcoal composite torso with "
            "weathered safety-amber shoulder hazard stripes, exposed mechanical actuator linkages, compact sensor head facing 12 o'clock "
            "with non-glowing dark optics, heavy work grippers empty at sides."
        ),
    ),
    _kneeling_spec(
        preset="android_industrial",
        batch="androids",
        label="Android Industrial",
        body=(
            "Kneeling athletic 2035 industrial utility biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "brushed aluminum and dark charcoal chassis with weathered safety-amber shoulder hazard stripes, crouched low on one knee "
            "with compact articulated sensor head facing 12 o'clock and zero glowing lights."
        ),
    ),
    _standing_spec(
        preset="android_security",
        batch="androids",
        label="Android Military / Security",
        body=(
            "Standing agile 2035 military/security biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "matte gunmetal-slate and muted olive-grey angular composite armor plating, athletic human-scale shoulders, sculpted "
            "robotic head with non-glowing dark matte optical visor strip facing 12 o'clock, holding a matte tactical carbine "
            "pointed straight up toward 12 o'clock."
        ),
    ),
    _kneeling_spec(
        preset="android_security",
        batch="androids",
        label="Android Military / Security",
        body=(
            "Kneeling agile 2035 military/security biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "matte gunmetal-slate and muted olive-grey angular composite armor, crouched tightly on one knee aiming a matte tactical "
            "carbine rifle straight up toward 12 o'clock, zero glowing eyes."
        ),
    ),
    _prone_spec(
        preset="android_security",
        batch="androids",
        label="Android Military / Security",
        body=(
            "Prone agile 2035 military/security biped android lying flat on stomach seen strictly straight down from overhead: "
            "matte gunmetal-slate and olive-grey composite armor plates, non-glowing dark optical head near 12 o'clock aiming a "
            "matte tactical carbine rifle straight up toward 12 o'clock, articulated metallic legs stretched back to 6 o'clock."
        ),
    ),
    # --- Movement Poses: Civilians (12) ---
    _walking_holstered_spec(
        preset="civilian_unarmed",
        batch="move_civilians",
        label="Civilian Empty-Handed",
        body=(
            "Walking 2035 civilian seen strictly straight down from overhead, facing 12 o'clock: natural mid-stride walk with "
            "one sneaker stepping slightly forward and opposite boot trailing behind, relaxed arms swinging gently at sides "
            "with empty open hands, calm upright posture."
        ),
    ),
    _tactical_advance_spec(
        preset="civilian_unarmed",
        batch="move_civilians",
        label="Civilian Empty-Handed",
        body=(
            "Tactical cautious-pace 2035 civilian advancing in guarded overwatch stride seen strictly straight down from overhead, "
            "facing 12 o'clock: staggered athletic bent-knee stride (left foot forward, right heel trailing), shoulders hunched "
            "cautiously forward, empty hands raised slightly in alert defensive guard."
        ),
    ),
    _sprinting_spec(
        preset="civilian_unarmed",
        batch="move_civilians",
        label="Civilian Empty-Handed",
        body=(
            "All-out sprinting 2035 civilian seen strictly straight down from overhead, facing 12 o'clock: dramatic wide running "
            "stride with one leg driving far forward toward 12 o'clock and opposite boot kicked back toward 6 o'clock, torso "
            "pitched forward, bent elbows pumping hard at sides."
        ),
    ),
    _walking_holstered_spec(
        preset="civilian_handgun",
        batch="move_civilians",
        label="Civilian Handgun",
        body=(
            "Walking 2035 civilian with holstered handgun seen strictly straight down from overhead, facing 12 o'clock: "
            "relaxed mid-stride walk with empty hands swinging gently at sides, compact matte-black pistol tucked securely "
            "inside a right-hip belt holster (NO gun in hands), one boot stepping forward and opposite boot trailing."
        ),
    ),
    _tactical_advance_spec(
        preset="civilian_handgun",
        batch="move_civilians",
        label="Civilian Handgun",
        body=(
            "Tactical bounding-overwatch 2035 civilian with handgun seen strictly straight down from overhead, facing 12 o'clock: "
            "cautious staggered combat-glide stride with knees bent and rear boot trailing, shoulders angled forward, both hands "
            "gripping a compact matte-black 9mm pistol at controlled low-ready pointed toward 12 o'clock."
        ),
    ),
    _sprinting_spec(
        preset="civilian_handgun",
        batch="move_civilians",
        label="Civilian Handgun",
        body=(
            "All-out sprinting 2035 civilian with handgun seen strictly straight down from overhead, facing 12 o'clock: "
            "dramatic long running stride with lead knee driving forward and rear boot kicked back toward 6 o'clock, torso "
            "pitched forward, right arm pumping forward gripping compact 9mm pistol and left arm pumping back."
        ),
    ),
    _walking_holstered_spec(
        preset="civilian_rifle",
        batch="move_civilians",
        label="Civilian Rifle",
        body=(
            "Walking 2035 civilian with slung rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "relaxed mid-stride walk with empty hands swinging gently at sides, carbine rifle slung diagonally across back "
            "on a shoulder sling (NO rifle in hands), one boot stepping forward and opposite boot trailing."
        ),
    ),
    _tactical_advance_spec(
        preset="civilian_rifle",
        batch="move_civilians",
        label="Civilian Rifle",
        body=(
            "Tactical bounding-overwatch 2035 civilian with rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "cautious staggered combat-glide stride with bent knees, both hands holding a matte polymer carbine rifle at "
            "controlled low-ready scanning toward 12 o'clock."
        ),
    ),
    _sprinting_spec(
        preset="civilian_rifle",
        batch="move_civilians",
        label="Civilian Rifle",
        body=(
            "All-out sprinting 2035 civilian with rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "wide explosive running stride with one boot driving forward and other kicked back toward 6 o'clock, torso pitched "
            "forward, gripping carbine rifle diagonally across lower chest while running hard."
        ),
    ),
    _walking_holstered_spec(
        preset="civilian_knife",
        batch="move_civilians",
        label="Civilian Knife",
        body=(
            "Walking 2035 civilian investigator with sheathed knife seen strictly straight down from overhead, facing 12 o'clock: "
            "relaxed mid-stride walk in rumpled field coat with empty open hands swinging naturally at sides and a matte dark "
            "sheathed kerambit knife clipped unobtrusively at the right waist belt (NO blade in hand)."
        ),
    ),
    _tactical_advance_spec(
        preset="civilian_knife",
        batch="move_civilians",
        label="Civilian Knife",
        body=(
            "Tactical cautious-pace 2035 civilian investigator with kerambit knife seen strictly straight down from overhead, "
            "facing 12 o'clock: staggered low silat-guard stepping stride, left forearm guarding centerline and right hand gripping "
            "a matte dark curved kerambit knife held low and ready toward 12 o'clock."
        ),
    ),
    _sprinting_spec(
        preset="civilian_knife",
        batch="move_civilians",
        label="Civilian Knife",
        body=(
            "All-out sprinting 2035 civilian investigator with kerambit knife seen strictly straight down from overhead, "
            "facing 12 o'clock: dramatic wide running stride with coat tails flaring slightly, one boot driving forward and rear "
            "boot kicked back to 6 o'clock, arms pumping hard with curved kerambit gripped in right hand."
        ),
    ),
    # --- Movement Poses: Police, Security & Military (9) ---
    _walking_holstered_spec(
        preset="security_handgun",
        batch="move_security",
        label="Police / Security Handgun",
        body=(
            "Patrol-walking 2035 police / security officer with holstered sidearm seen strictly straight down from overhead, "
            "facing 12 o'clock: steady mid-stride walk with empty relaxed hands at sides, matte-black service pistol secured "
            "in right-hip duty holster (NO gun in hands), one boot stepping forward and rear boot trailing."
        ),
    ),
    _tactical_advance_spec(
        preset="security_handgun",
        batch="move_security",
        label="Police / Security Handgun",
        body=(
            "Tactical bounding-overwatch 2035 police / security officer seen strictly straight down from overhead, facing 12 o'clock: "
            "cautious staggered combat-glide stride, knees bent, both hands gripping service pistol in controlled compressed "
            "low-ready pointed toward 12 o'clock."
        ),
    ),
    _sprinting_spec(
        preset="security_handgun",
        batch="move_security",
        label="Police / Security Handgun",
        body=(
            "All-out sprinting 2035 police / security officer seen strictly straight down from overhead, facing 12 o'clock: "
            "wide explosive running stride with one boot driving forward and opposite boot kicked far back toward 6 o'clock, "
            "torso pitched forward, arms pumping hard while gripping service pistol in right hand."
        ),
    ),
    _walking_holstered_spec(
        preset="soldier_rifle",
        batch="move_security",
        label="Security / Soldier Rifle",
        body=(
            "Walking 2035 DAISE soldier with slung rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "steady patrol stride with empty hands swinging relaxed at sides, tactical carbine rifle slung diagonally across "
            "plate-carrier back (NO rifle held in hands), one boot forward and opposite boot trailing."
        ),
    ),
    _tactical_advance_spec(
        preset="soldier_rifle",
        batch="move_security",
        label="Security / Soldier Rifle",
        body=(
            "Tactical bounding-overwatch 2035 DAISE soldier seen strictly straight down from overhead, facing 12 o'clock: "
            "cautious staggered combat-glide stride with bent knees, both hands holding short-stroke piston tactical carbine "
            "at controlled high/low-ready scanning toward 12 o'clock."
        ),
    ),
    _sprinting_spec(
        preset="soldier_rifle",
        batch="move_security",
        label="Security / Soldier Rifle",
        body=(
            "All-out sprinting 2035 DAISE soldier seen strictly straight down from overhead, facing 12 o'clock: "
            "dramatic long running stride with lead knee driving forward and rear boot kicked back toward 6 o'clock, torso "
            "pitched forward carrying tactical carbine diagonally across chest while sprinting at full speed."
        ),
    ),
    _walking_holstered_spec(
        preset="sniper_rifle",
        batch="move_security",
        label="Military / Security Sniper",
        body=(
            "Walking 2035 military marksman with slung sniper rifle seen strictly straight down from overhead, facing 12 o'clock: "
            "steady mid-stride walk with empty relaxed hands at sides, long suppressed scoped sniper rifle slung diagonally "
            "across back over camo scrim mantle (NO rifle in hands), one boot forward and one trailing."
        ),
    ),
    _tactical_advance_spec(
        preset="sniper_rifle",
        batch="move_security",
        label="Military / Security Sniper",
        body=(
            "Tactical cautious-pace 2035 military marksman seen strictly straight down from overhead, facing 12 o'clock: "
            "low staggered stealth-creep stride with knees bent under camo scrim cloak, holding long suppressed scoped sniper "
            "rifle at controlled low-ready angled toward 12 o'clock."
        ),
    ),
    _sprinting_spec(
        preset="sniper_rifle",
        batch="move_security",
        label="Military / Security Sniper",
        body=(
            "All-out sprinting 2035 military marksman seen strictly straight down from overhead, facing 12 o'clock: "
            "wide running stride with one boot forward and opposite boot kicked back toward 6 o'clock, torso pitched forward "
            "carrying long suppressed sniper rifle diagonally across body while sprinting."
        ),
    ),
    # --- Movement Poses: Biped Androids (9) ---
    _walking_holstered_spec(
        preset="android_domestic",
        batch="move_androids",
        label="Android Domestic",
        body=(
            "Walking 2035 domestic caregiver biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "smooth upright mid-stride walk with one metallic-polymer foot forward and one trailing, relaxed arms swinging "
            "gently at sides with empty open hands, smooth matte head facing 12 o'clock with zero glowing eyes."
        ),
    ),
    _tactical_advance_spec(
        preset="android_domestic",
        batch="move_androids",
        label="Android Domestic",
        body=(
            "Cautious-pace advancing 2035 domestic biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "staggered athletic crouched stride with knees bent and rear foot trailing, shoulders angled forward, empty hands "
            "held cautiously forward in protective readiness, zero glowing eyes."
        ),
    ),
    _sprinting_spec(
        preset="android_domestic",
        batch="move_androids",
        label="Android Domestic",
        body=(
            "All-out sprinting 2035 domestic biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "wide dynamic running stride with one leg driving far forward and opposite foot kicked back toward 6 o'clock, "
            "torso pitched forward, articulated arms pumping hard at sides, zero glowing eyes."
        ),
    ),
    _walking_holstered_spec(
        preset="android_industrial",
        batch="move_androids",
        label="Android Industrial",
        body=(
            "Walking 2035 industrial utility biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "heavy balanced mid-stride walk with one foot stepping forward and rear foot trailing, empty mechanical grippers "
            "swinging naturally at sides, compact sensor head facing 12 o'clock with zero glowing lights."
        ),
    ),
    _tactical_advance_spec(
        preset="android_industrial",
        batch="move_androids",
        label="Android Industrial",
        body=(
            "Cautious-pace advancing 2035 industrial utility biped android seen strictly straight down from overhead, "
            "facing 12 o'clock: staggered bent-knee low-center-of-gravity stride, heavy shoulders pitched forward, mechanical "
            "grippers held slightly forward in alert brace, zero glowing lights."
        ),
    ),
    _sprinting_spec(
        preset="android_industrial",
        batch="move_androids",
        label="Android Industrial",
        body=(
            "All-out sprinting 2035 industrial utility biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "powerful wide running stride with lead leg driving forward and rear foot kicked back toward 6 o'clock, torso pitched "
            "forward, hydraulic arms pumping forcefully, zero glowing lights."
        ),
    ),
    _walking_holstered_spec(
        preset="android_security",
        batch="move_androids",
        label="Android Military / Security",
        body=(
            "Patrol-walking 2035 military/security biped android with slung carbine seen strictly straight down from overhead, "
            "facing 12 o'clock: steady mid-stride walk with empty articulated hands swinging at sides and matte tactical carbine "
            "magnetically clamped/slung diagonally across rear armor back (NO rifle in hands), zero glowing eyes."
        ),
    ),
    _tactical_advance_spec(
        preset="android_security",
        batch="move_androids",
        label="Android Military / Security",
        body=(
            "Tactical bounding-overwatch 2035 military/security biped android seen strictly straight down from overhead, "
            "facing 12 o'clock: cautious staggered combat-glide stride with knees bent, holding matte tactical carbine at "
            "controlled low-ready scanning straight toward 12 o'clock, zero glowing eyes."
        ),
    ),
    _sprinting_spec(
        preset="android_security",
        batch="move_androids",
        label="Android Military / Security",
        body=(
            "All-out sprinting 2035 military/security biped android seen strictly straight down from overhead, facing 12 o'clock: "
            "explosive wide running stride with one metallic leg driving forward and opposite boot kicked back toward 6 o'clock, "
            "torso pitched forward carrying tactical carbine diagonally across chest while sprinting, zero glowing eyes."
        ),
    ),
)

_TOKEN_BY_ID: dict[str, TokenSpriteSpec] = {s.id: s for s in TOKEN_SPRITE_CATALOG}


def get_token_sprite_spec(sprite_id: str) -> TokenSpriteSpec:
    """Return a single token sprite spec by ID."""
    try:
        return _TOKEN_BY_ID[sprite_id]
    except KeyError as exc:
        raise ValueError(f"Unknown token sprite ID: {sprite_id!r}") from exc


def select_token_sprite_specs(
    *,
    batch: str | None = None,
    only: str | None = None,
) -> list[TokenSpriteSpec]:
    """Select token sprite specs by `--only` CSV or `--batch` name."""
    if only:
        ids = [item.strip() for item in only.split(",") if item.strip()]
        return [get_token_sprite_spec(i) for i in ids]
    batches: dict[str, tuple[str, ...]] = {
        "pilot": PILOT_TOKEN_IDS,
        "civilians": BATCH_CIVILIANS_IDS,
        "security": BATCH_SECURITY_IDS,
        "drones": BATCH_DRONES_IDS,
        "androids": BATCH_ANDROIDS_IDS,
        "move_civilians": BATCH_MOVE_CIVILIANS_IDS,
        "move_security": BATCH_MOVE_SECURITY_IDS,
        "move_androids": BATCH_MOVE_ANDROIDS_IDS,
        "movement": BATCH_MOVEMENT_IDS,
    }
    if batch is None or batch == "all":
        return list(TOKEN_SPRITE_CATALOG)
    if batch not in batches:
        raise ValueError(f"Unknown token sprite batch: {batch!r}")
    return [get_token_sprite_spec(i) for i in batches[batch]]
