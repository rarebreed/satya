"""CLI for generating and alpha-matting top-down 2D battlemap prop sprites."""

from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np
from PIL import Image

from satya_ml.texture_gen.cli import GeminiImageClient, _load_rgb, _resolve_api_key
from satya_ml.texture_gen.prop_catalog import PropSpriteSpec, select_prop_sprite_specs
from satya_ml.texture_gen.prop_matting import (
    matte_prop_sprite,
    render_prop_contact_sheet,
    verify_sprite_alpha_quality,
)


def _load_rgba(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.uint8)


def fetch_raw_prop_sprites(
    specs: list[PropSpriteSpec],
    *,
    raw_dir: Path,
    model: str,
    skip_existing: bool = False,
) -> None:
    api_key = _resolve_api_key()
    if not api_key:
        raise SystemExit("Missing GEMINI_API_KEY or GOOGLE_API_KEY environment variable.")
    raw_dir.mkdir(parents=True, exist_ok=True)
    client: GeminiImageClient | None = None
    for spec in specs:
        raw_path = raw_dir / f"{spec.id}_raw.png"
        if skip_existing and raw_path.exists():
            print(f"[prop_gen] skipping cached {spec.id}")
            continue
        if client is None:
            client = GeminiImageClient(api_key=api_key, preferred_model=model)
        print(f"[prop_gen] fetching {spec.id} ({spec.aspect_ratio}) -> {raw_path}")
        raw_bytes = client.generate(spec.prompt, aspect_ratio=spec.aspect_ratio)
        with Image.open(__import__("io").BytesIO(raw_bytes)) as im:
            rgb = np.asarray(im.convert("RGB"), dtype=np.uint8)
        Image.fromarray(rgb).save(raw_path, format="PNG")


def process_raw_prop_sprites(
    specs: list[PropSpriteSpec],
    *,
    raw_dir: Path,
    out_dir: Path,
    contact_sheet: Path | None,
    quality: int,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    baked: list[tuple[PropSpriteSpec, np.ndarray]] = []
    for spec in specs:
        raw_path = raw_dir / f"{spec.id}_raw.png"
        if not raw_path.exists():
            raise SystemExit(f"Missing raw prop sprite file: {raw_path}")
        raw_rgb = _load_rgb(raw_path)
        rgba = matte_prop_sprite(raw_rgb, spec)
        out_path = out_dir / spec.filename
        Image.fromarray(rgba, mode="RGBA").save(
            out_path, format="WEBP", quality=quality, method=6
        )
        metrics = verify_sprite_alpha_quality(_load_rgba(out_path), key_color=spec.key_color)
        print(
            f"[prop_gen] baked {out_path} ({spec.target_w_px}x{spec.target_h_px}, "
            f"fg={metrics['fg_coverage']:.2f}, corner_a={metrics['corner_alpha_max']:.0f}, "
            f"spill={metrics['edge_spill_frac']:.3f})"
        )
        baked.append((spec, rgba))

    if contact_sheet is not None:
        contact_sheet.parent.mkdir(parents=True, exist_ok=True)
        sheet_rgb = render_prop_contact_sheet(baked, cols=4)
        Image.fromarray(sheet_rgb).save(contact_sheet, format="PNG")
        print(f"[prop_gen] wrote contact sheet -> {contact_sheet}")


def verify_output_prop_sprites(specs: list[PropSpriteSpec], *, out_dir: Path) -> None:
    missing: list[str] = []
    for spec in specs:
        p = out_dir / spec.filename
        if not p.exists():
            missing.append(str(p))
            continue
        rgba = _load_rgba(p)
        expected = (spec.target_h_px, spec.target_w_px, 4)
        if rgba.shape != expected:
            raise SystemExit(f"Unexpected shape for {p}: {rgba.shape} != {expected}")
        metrics = verify_sprite_alpha_quality(rgba, key_color=spec.key_color)
        if metrics["corner_alpha_max"] >= 16.0:
            raise SystemExit(f"Opaque corner detected in {p}: {metrics}")
        if not (0.12 <= metrics["fg_coverage"] <= 0.96):
            raise SystemExit(f"Unexpected foreground coverage in {p}: {metrics}")
        if metrics["edge_spill_frac"] > 0.08:
            raise SystemExit(f"Residual chroma spill too high in {p}: {metrics}")
    if missing:
        raise SystemExit(f"Missing output prop sprites: {', '.join(missing)}")
    print(f"[prop_gen] verified {len(specs)} alpha-matted WebP prop sprites in {out_dir}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate and alpha-matte top-down 2D battlemap prop sprites."
    )
    parser.add_argument("--model", default="gemini-nano-banana-2.1")
    parser.add_argument("--pilot", action="store_true", help="Process the 12 clinic pilot props.")
    parser.add_argument("--only", default=None, help="Comma-separated prop sprite IDs.")
    parser.add_argument("--quality", type=int, default=92)
    parser.add_argument("--raw-dir", type=Path, default=Path("data/prop_sprites_raw"))
    parser.add_argument("--process-raw-dir", type=Path, default=None)
    parser.add_argument("--out-dir", type=Path, default=Path("assets/sprites/props"))
    parser.add_argument("--contact-sheet", type=Path, default=None)
    parser.add_argument("--fetch-raw-only", action="store_true")
    parser.add_argument("--skip-existing", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args(argv)

    specs = select_prop_sprite_specs(pilot=args.pilot, only=args.only)
    if args.verify_only:
        verify_output_prop_sprites(specs, out_dir=args.out_dir)
        return 0
    if args.fetch_raw_only:
        fetch_raw_prop_sprites(
            specs,
            raw_dir=args.raw_dir,
            model=args.model,
            skip_existing=args.skip_existing,
        )
        return 0
    if args.process_raw_dir is not None:
        process_raw_prop_sprites(
            specs,
            raw_dir=args.process_raw_dir,
            out_dir=args.out_dir,
            contact_sheet=args.contact_sheet,
            quality=args.quality,
        )
        return 0

    fetch_raw_prop_sprites(
        specs,
        raw_dir=args.raw_dir,
        model=args.model,
        skip_existing=args.skip_existing,
    )
    process_raw_prop_sprites(
        specs,
        raw_dir=args.raw_dir,
        out_dir=args.out_dir,
        contact_sheet=args.contact_sheet,
        quality=args.quality,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
