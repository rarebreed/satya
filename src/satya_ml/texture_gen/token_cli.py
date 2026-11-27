"""CLI for generating and alpha-matting top-down 2D Style C battlemap token sprites."""

from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import io
from pathlib import Path
import time

import numpy as np
from PIL import Image

from satya_ml.texture_gen.cli import GeminiImageClient, _load_rgb, _resolve_api_key
from satya_ml.texture_gen.prop_matting import (
    matte_prop_sprite,
    render_prop_contact_sheet,
    render_prop_variants_contact_sheet,
    verify_sprite_alpha_quality,
)
from satya_ml.texture_gen.token_catalog import TokenSpriteSpec, select_token_sprite_specs


def _load_rgba(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGBA"), dtype=np.uint8)


def _parse_rotations(raw_items: list[str]) -> dict[str, int]:
    rotations: dict[str, int] = {}
    for item in raw_items:
        if ":" not in item:
            raise SystemExit(f"Invalid --rotate-raw value {item!r}; expected ID:DEG (90/180/270).")
        sprite_id, deg_s = item.split(":", 1)
        deg = int(deg_s.strip()) % 360
        if deg not in (0, 90, 180, 270):
            raise SystemExit(f"Unsupported rotation {deg} for {sprite_id!r}; use 90, 180, or 270.")
        rotations[sprite_id.strip()] = deg
    return rotations


def _rotate_raw_cw(rgb: np.ndarray, deg_cw: int) -> np.ndarray:
    steps = (deg_cw // 90) % 4
    if steps == 0:
        return rgb
    return np.ascontiguousarray(np.rot90(rgb, k=-steps))


def _fetch_one_variant(
    client: GeminiImageClient,
    spec: TokenSpriteSpec,
    v_idx: int,
    raw_dir: Path,
    ref_png_bytes: bytes | None,
) -> Path:
    stem = spec.variant_raw_stem(v_idx)
    raw_path = raw_dir / f"{stem}_raw.png"
    prompt = spec.prompt_for_variant(v_idx)
    last_exc: Exception | None = None
    for attempt in range(3):
        try:
            print(f"[token_gen] fetching {stem} ({spec.aspect_ratio}) -> {raw_path}", flush=True)
            raw_bytes = client.generate(
                prompt,
                ref_png_bytes=ref_png_bytes,
                aspect_ratio=spec.aspect_ratio,
            )
            with Image.open(io.BytesIO(raw_bytes)) as im:
                rgb = np.asarray(im.convert("RGB"), dtype=np.uint8)
            Image.fromarray(rgb).save(raw_path, format="PNG")
            return raw_path
        except Exception as exc:  # noqa: BLE001
            last_exc = exc
            if attempt < 2:
                delay = 2.0 * (attempt + 1)
                print(
                    f"[token_gen] retry {attempt + 1}/2 for {stem} after error: {str(exc)[:120]}",
                    flush=True,
                )
                time.sleep(delay)
    assert last_exc is not None
    raise last_exc


def fetch_raw_token_sprites(
    specs: list[TokenSpriteSpec],
    *,
    raw_dir: Path,
    model: str,
    variants: int = 1,
    workers: int = 6,
    skip_existing: bool = False,
    style_ref: Path | None = None,
) -> None:
    api_key = _resolve_api_key()
    if not api_key:
        raise SystemExit("Missing GEMINI_API_KEY or GOOGLE_API_KEY environment variable.")
    ref_png_bytes = style_ref.read_bytes() if style_ref is not None else None
    raw_dir.mkdir(parents=True, exist_ok=True)
    tasks: list[tuple[TokenSpriteSpec, int]] = []
    for spec in specs:
        for v_idx in range(variants):
            raw_path = raw_dir / f"{spec.variant_raw_stem(v_idx)}_raw.png"
            if skip_existing and raw_path.exists():
                print(f"[token_gen] skipping cached {spec.variant_raw_stem(v_idx)}", flush=True)
                continue
            tasks.append((spec, v_idx))
    if not tasks:
        return

    client = GeminiImageClient(api_key=api_key, preferred_model=model)
    first_spec, first_v = tasks[0]
    _fetch_one_variant(client, first_spec, first_v, raw_dir, ref_png_bytes)
    rest = tasks[1:]
    if not rest:
        return

    max_workers = max(1, min(workers, len(rest)))
    if max_workers == 1:
        for spec, v_idx in rest:
            _fetch_one_variant(client, spec, v_idx, raw_dir, ref_png_bytes)
        return

    with ThreadPoolExecutor(max_workers=max_workers) as pool:
        futs = [
            pool.submit(_fetch_one_variant, client, spec, v_idx, raw_dir, ref_png_bytes)
            for spec, v_idx in rest
        ]
        for fut in as_completed(futs):
            fut.result()


def _bake_one_variant(
    spec: TokenSpriteSpec,
    v_idx: int,
    raw_dir: Path,
    out_dir: Path,
    quality: int,
    skip_existing: bool,
    rot_cw: int,
) -> tuple[str, np.ndarray, dict[str, float]]:
    stem = spec.variant_raw_stem(v_idx)
    out_path = out_dir / spec.variant_filename(v_idx)
    if skip_existing and out_path.exists() and rot_cw == 0:
        rgba = _load_rgba(out_path)
        metrics = verify_sprite_alpha_quality(rgba, key_color=spec.key_color)
        return (f"cached {out_path}", rgba, metrics)

    raw_path = raw_dir / f"{stem}_raw.png"
    if not raw_path.exists():
        raise SystemExit(f"Missing raw token sprite file: {raw_path}")
    raw_rgb = _load_rgb(raw_path)
    if rot_cw:
        raw_rgb = _rotate_raw_cw(raw_rgb, rot_cw)
        Image.fromarray(raw_rgb).save(raw_path, format="PNG")
    rgba = matte_prop_sprite(raw_rgb, spec)
    Image.fromarray(rgba, mode="RGBA").save(out_path, format="WEBP", quality=quality, method=6)
    baked_rgba = _load_rgba(out_path)
    metrics = verify_sprite_alpha_quality(baked_rgba, key_color=spec.key_color)
    return (f"baked {out_path}", rgba, metrics)


def process_raw_token_sprites(
    specs: list[TokenSpriteSpec],
    *,
    raw_dir: Path,
    out_dir: Path,
    variants: int = 1,
    workers: int = 6,
    skip_existing: bool = False,
    contact_sheet: Path | None,
    quality: int,
    rotations: dict[str, int] | None = None,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    rot_map = rotations or {}
    results: dict[tuple[str, int], np.ndarray] = {}
    job_list = [(spec, v_idx) for spec in specs for v_idx in range(variants)]
    max_workers = max(1, min(workers, len(job_list)))

    with ThreadPoolExecutor(max_workers=max_workers) as pool:
        fut_map = {
            pool.submit(
                _bake_one_variant,
                spec,
                v_idx,
                raw_dir,
                out_dir,
                quality,
                skip_existing,
                rot_map.get(spec.variant_raw_stem(v_idx), rot_map.get(spec.id, 0)),
            ): (spec, v_idx)
            for spec, v_idx in job_list
        }
        for fut in as_completed(fut_map):
            spec, v_idx = fut_map[fut]
            label, rgba, metrics = fut.result()
            results[(spec.id, v_idx)] = rgba
            print(
                f"[token_gen] {label} ({spec.target_w_px}x{spec.target_h_px}, "
                f"fg={metrics['fg_coverage']:.2f}, corner_a={metrics['corner_alpha_max']:.0f}, "
                f"spill={metrics['edge_spill_frac']:.3f})",
                flush=True,
            )

    if contact_sheet is not None:
        variant_rows = [
            (spec, [results[(spec.id, v_idx)] for v_idx in range(variants)])
            for spec in specs
        ]
        contact_sheet.parent.mkdir(parents=True, exist_ok=True)
        if variants == 1:
            sheet_rgb = render_prop_contact_sheet(
                [(spec, vars_list[0]) for spec, vars_list in variant_rows],
                cols=4,
                label_fn=lambda s: f"{s.preset} [{s.stance}]",
            )
        else:
            sheet_rgb = render_prop_variants_contact_sheet(variant_rows, props_per_row=1)
        Image.fromarray(sheet_rgb).save(contact_sheet, format="PNG")
        print(f"[token_gen] wrote contact sheet -> {contact_sheet}", flush=True)


def verify_output_token_sprites(
    specs: list[TokenSpriteSpec], *, out_dir: Path, variants: int = 1
) -> None:
    missing: list[str] = []
    checked = 0
    for spec in specs:
        for v_idx in range(variants):
            p = out_dir / spec.variant_filename(v_idx)
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
            if not (0.06 <= metrics["fg_coverage"] <= 0.94):
                raise SystemExit(f"Unexpected foreground coverage in {p}: {metrics}")
            if metrics["edge_spill_frac"] > 0.08:
                raise SystemExit(f"Residual chroma spill too high in {p}: {metrics}")
            checked += 1
    if missing:
        raise SystemExit(f"Missing output token sprites: {', '.join(missing)}")
    print(f"[token_gen] verified {checked} alpha-matted Style C WebP token sprites in {out_dir}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Generate and alpha-matte top-down 2D Style C battlemap token sprites."
    )
    parser.add_argument("--model", default="gemini-nano-banana-2.1")
    parser.add_argument(
        "--batch",
        choices=(
            "pilot",
            "civilians",
            "security",
            "drones",
            "androids",
            "move_civilians",
            "move_security",
            "move_androids",
            "movement",
            "all",
        ),
        default=None,
        help="Select token sprite batch.",
    )
    parser.add_argument("--only", default=None, help="Comma-separated token sprite IDs.")
    parser.add_argument("--variants", type=int, default=1, choices=(1, 2, 3))
    parser.add_argument("--workers", type=int, default=6)
    parser.add_argument("--quality", type=int, default=92)
    parser.add_argument("--raw-dir", type=Path, default=Path("data/token_sprites_raw"))
    parser.add_argument("--process-raw-dir", type=Path, default=None)
    parser.add_argument("--out-dir", type=Path, default=Path("assets/sprites/tokens"))
    parser.add_argument("--contact-sheet", type=Path, default=None)
    parser.add_argument("--style-ref", type=Path, default=None)
    parser.add_argument(
        "--rotate-raw",
        action="append",
        default=[],
        help="Rotate raw PNG clockwise before matting: ID:DEG (e.g. soldier_rifle_prone:90).",
    )
    parser.add_argument("--fetch-raw-only", action="store_true")
    parser.add_argument("--skip-existing", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args(argv)

    specs = select_token_sprite_specs(batch=args.batch, only=args.only)
    if args.verify_only:
        verify_output_token_sprites(specs, out_dir=args.out_dir, variants=args.variants)
        return 0
    if args.fetch_raw_only:
        fetch_raw_token_sprites(
            specs,
            raw_dir=args.raw_dir,
            model=args.model,
            variants=args.variants,
            workers=args.workers,
            skip_existing=args.skip_existing,
            style_ref=args.style_ref,
        )
        return 0
    rotations = _parse_rotations(args.rotate_raw)
    if args.process_raw_dir is not None:
        process_raw_token_sprites(
            specs,
            raw_dir=args.process_raw_dir,
            out_dir=args.out_dir,
            variants=args.variants,
            workers=args.workers,
            skip_existing=args.skip_existing,
            contact_sheet=args.contact_sheet,
            quality=args.quality,
            rotations=rotations,
        )
        return 0

    fetch_raw_token_sprites(
        specs,
        raw_dir=args.raw_dir,
        model=args.model,
        variants=args.variants,
        workers=args.workers,
        skip_existing=args.skip_existing,
        style_ref=args.style_ref,
    )
    process_raw_token_sprites(
        specs,
        raw_dir=args.raw_dir,
        out_dir=args.out_dir,
        variants=args.variants,
        workers=args.workers,
        skip_existing=args.skip_existing,
        contact_sheet=args.contact_sheet,
        quality=args.quality,
        rotations=rotations,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
