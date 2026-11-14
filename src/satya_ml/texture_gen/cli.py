"""CLI and Gemini Nano Banana image fetcher for 2D battlemap WebP textures."""

from __future__ import annotations

import argparse
import base64
import io
import os
from pathlib import Path
from typing import Any

import numpy as np
from PIL import Image

from satya_ml.texture_gen.catalog import TextureSpec, select_texture_specs
from satya_ml.texture_gen.seamless import (
    delight_albedo,
    finalize_tile,
    render_contact_sheet_3x3,
    roll_half,
    wrap_seam_ratio,
)

_HEAL_PROMPT = (
    "You are editing a toroidally shifted square surface texture where the vertical "
    "and horizontal center lines show a seam cross. Seamlessly paint over ONLY the "
    "central cross intersection and seam lines so the material fibers, wind-swept grass "
    "blades, brick courses, tile grout, pebbles, or wood grain flow naturally and "
    "continuously in the exact same direction across the middle of the image. Keep the "
    "outer edges and overall color, lighting, scale, and painterly gouache style identical."
)


_RECITATION_RETRY_SUFFIXES: tuple[str, ...] = (
    "",
    " Paint as a wholly original stylized gouache game-board illustration with organic brushstroke variation and irregular hand-inked details.",
    " Stylized painterly watercolor-gouache fantasy tabletop art study, asymmetric organic surface details, non-photographic.",
)


class GeminiImageClient:
    """Thin Nano Banana client supporting Developer API and Vertex AI Express keys."""

    def __init__(self, api_key: str, preferred_model: str = "gemini-nano-banana-2.1") -> None:
        self._api_key = api_key
        self._preferred_model = preferred_model
        self._cached_strategy: tuple[bool, str, str] | None = None

    def generate(self, prompt: str, *, ref_png_bytes: bytes | None = None) -> bytes:
        from google import genai
        from google.genai import types

        if self._cached_strategy is not None:
            vertexai, api_kind, model_id = self._cached_strategy
            return self._invoke_with_recitation_retry(
                genai, types, vertexai, api_kind, model_id, prompt, ref_png_bytes
            )

        models = list(
            dict.fromkeys(
                [
                    self._preferred_model,
                    "gemini-nano-banana-2.1",
                    "gemini-3.1-flash-image",
                    "gemini-3.1-flash-image-preview",
                ]
            )
        )
        attempts: list[tuple[bool, str, str]] = []
        for vertexai in (False, True):
            for model_id in models:
                attempts.append((vertexai, "interactions", model_id))
                attempts.append((vertexai, "generate_content", model_id))

        errors: list[str] = []
        for vertexai, api_kind, model_id in attempts:
            try:
                data = self._invoke_with_recitation_retry(
                    genai, types, vertexai, api_kind, model_id, prompt, ref_png_bytes
                )
                self._cached_strategy = (vertexai, api_kind, model_id)
                mode = "vertex-express" if vertexai else "gemini-dev"
                print(f"[texture_gen] connected via {mode} / {api_kind} ({model_id})")
                return data
            except Exception as exc:  # noqa: BLE001
                msg = str(exc).replace(self._api_key, "***")
                errors.append(f"vertexai={vertexai}/{api_kind}/{model_id}: {msg[:180]}")

        summary = "\n  - ".join(errors)
        raise RuntimeError(f"All Gemini image generation strategies failed:\n  - {summary}")

    def _invoke_with_recitation_retry(
        self,
        genai: Any,
        types: Any,
        vertexai: bool,
        api_kind: str,
        model_id: str,
        prompt: str,
        ref_png_bytes: bytes | None,
    ) -> bytes:
        """Retries with stronger artistic stylization suffixes if Gemini triggers image_recitation."""
        for idx, suffix in enumerate(_RECITATION_RETRY_SUFFIXES):
            try:
                return self._invoke(
                    genai,
                    types,
                    vertexai,
                    api_kind,
                    model_id,
                    f"{prompt}{suffix}" if suffix else prompt,
                    ref_png_bytes,
                )
            except Exception as exc:
                msg = str(exc).lower()
                is_recitation = "recitation" in msg or "copyright" in msg
                if is_recitation and idx + 1 < len(_RECITATION_RETRY_SUFFIXES):
                    print(
                        f"[texture_gen] recitation filter hit ({model_id}); "
                        f"retrying with artistic variation {idx + 1}"
                    )
                    continue
                raise
        raise RuntimeError("unreachable recitation retry exit")

    def _invoke(
        self,
        genai: Any,
        types: Any,
        vertexai: bool,
        api_kind: str,
        model_id: str,
        prompt: str,
        ref_png_bytes: bytes | None,
    ) -> bytes:
        client = (
            genai.Client(vertexai=True, api_key=self._api_key)
            if vertexai
            else genai.Client(api_key=self._api_key)
        )
        if api_kind == "interactions":
            inp: Any = prompt
            if ref_png_bytes is not None:
                inp = [
                    {"type": "text", "text": prompt},
                    {
                        "type": "image",
                        "data": base64.b64encode(ref_png_bytes).decode("ascii"),
                        "mime_type": "image/png",
                    },
                ]
            interaction = client.interactions.create(
                model=model_id,
                input=inp,
                response_format={
                    "type": "image",
                    "aspect_ratio": "1:1",
                    "image_size": "1K",
                },
            )
            out_img = getattr(interaction, "output_image", None)
            if out_img is not None and getattr(out_img, "data", None):
                return base64.b64decode(out_img.data)
            raise ValueError("interaction returned no output_image data")

        contents: list[Any] = [prompt]
        if ref_png_bytes is not None:
            contents.append(Image.open(io.BytesIO(ref_png_bytes)).convert("RGB"))
        resp = client.models.generate_content(
            model=model_id,
            contents=contents,
            config=types.GenerateContentConfig(
                response_modalities=["IMAGE"],
                image_config=types.ImageConfig(aspect_ratio="1:1"),
            ),
        )
        for cand in getattr(resp, "candidates", None) or []:
            content = getattr(cand, "content", None)
            for part in getattr(content, "parts", None) or []:
                inline = getattr(part, "inline_data", None)
                if inline is not None and getattr(inline, "data", None):
                    raw = inline.data
                    return base64.b64decode(raw) if isinstance(raw, str) else bytes(raw)
        raise ValueError("generate_content returned no inline image bytes")


def _png_bytes(rgb: np.ndarray) -> bytes:
    buf = io.BytesIO()
    Image.fromarray(rgb).save(buf, format="PNG")
    return buf.getvalue()


def _load_rgb(path: Path) -> np.ndarray:
    return np.asarray(Image.open(path).convert("RGB"), dtype=np.uint8)


def fetch_raw_textures(
    specs: list[TextureSpec],
    *,
    raw_dir: Path,
    model: str,
    heal_pass: bool,
    skip_existing: bool = False,
) -> None:
    api_key = os.environ.get("GEMINI_API_KEY") or os.environ.get("GOOGLE_API_KEY")
    if not api_key:
        raise SystemExit("Missing GEMINI_API_KEY or GOOGLE_API_KEY environment variable.")
    raw_dir.mkdir(parents=True, exist_ok=True)
    client: GeminiImageClient | None = None
    for spec in specs:
        raw_path = raw_dir / f"{spec.id}_raw.png"
        heal_path = raw_dir / f"{spec.id}_healed_rolled.png"
        already_cached = raw_path.exists() and (not heal_pass or heal_path.exists())
        if skip_existing and already_cached:
            print(f"[texture_gen] skipping cached {spec.id}")
            continue
        if client is None:
            client = GeminiImageClient(api_key=api_key, preferred_model=model)
        print(f"[texture_gen] fetching {spec.id} -> {raw_path}")
        raw_bytes = client.generate(spec.prompt)
        raw_img = np.asarray(Image.open(io.BytesIO(raw_bytes)).convert("RGB"), dtype=np.uint8)
        Image.fromarray(raw_img).save(raw_path, format="PNG")
        if heal_pass:
            rolled = roll_half(
                delight_albedo(
                    raw_img,
                    sigma_frac=spec.delight_sigma_frac,
                    strength=spec.delight_strength,
                    exposure_scale=spec.exposure_scale,
                )
            )
            healed_bytes = client.generate(_HEAL_PROMPT, ref_png_bytes=_png_bytes(rolled))
            healed_img = np.asarray(
                Image.open(io.BytesIO(healed_bytes)).convert("RGB"), dtype=np.uint8
            )
            Image.fromarray(healed_img).save(heal_path, format="PNG")


def process_raw_textures(
    specs: list[TextureSpec],
    *,
    raw_dir: Path,
    out_dir: Path,
    preview_dir: Path | None,
    size_px: int,
    quality: int,
) -> None:
    out_dir.mkdir(parents=True, exist_ok=True)
    if preview_dir is not None:
        preview_dir.mkdir(parents=True, exist_ok=True)

    for spec in specs:
        raw_path = raw_dir / f"{spec.id}_raw.png"
        if not raw_path.exists():
            raise SystemExit(f"Missing raw texture file: {raw_path}")
        raw_rgb = _load_rgb(raw_path)
        heal_path = raw_dir / f"{spec.id}_healed_rolled.png"
        healed_rgb = _load_rgb(heal_path) if heal_path.exists() else None

        tile = finalize_tile(raw_rgb, spec, healed_rolled_rgb=healed_rgb, out_px=size_px)
        out_path = out_dir / spec.filename
        Image.fromarray(tile).save(out_path, format="WEBP", quality=quality, method=6)
        rx, ry = wrap_seam_ratio(tile)
        print(f"[texture_gen] baked {out_path} ({size_px}x{size_px}, wrap_ratio=({rx:.2f}, {ry:.2f}))")

        if preview_dir is not None:
            tile_png = preview_dir / f"pilot_{spec.id}_tile.png"
            grid_png = preview_dir / f"pilot_{spec.id}_tiled_3x3.png"
            Image.fromarray(tile).save(tile_png, format="PNG")
            Image.fromarray(render_contact_sheet_3x3(tile, out_px=768)).save(grid_png, format="PNG")


def verify_output_textures(specs: list[TextureSpec], *, out_dir: Path, size_px: int) -> None:
    missing: list[str] = []
    for spec in specs:
        p = out_dir / spec.filename
        if not p.exists():
            missing.append(str(p))
            continue
        rgb = _load_rgb(p)
        if rgb.shape[:2] != (size_px, size_px):
            raise SystemExit(f"Unexpected size for {p}: {rgb.shape[:2]} != ({size_px}, {size_px})")
        rx, ry = wrap_seam_ratio(rgb)
        if rx > 1.42 or ry > 1.42:
            raise SystemExit(f"Wrap seam check failed for {p}: ratios=({rx:.2f}, {ry:.2f})")
    if missing:
        raise SystemExit(f"Missing output textures: {', '.join(missing)}")
    print(f"[texture_gen] verified {len(specs)} seamless WebP textures in {out_dir}")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Generate seamless 2D battlemap WebP textures.")
    parser.add_argument("--model", default="gemini-nano-banana-2.1")
    parser.add_argument("--pilot", action="store_true", help="Process only the 4 pilot textures.")
    parser.add_argument("--only", default=None, help="Comma-separated texture IDs.")
    parser.add_argument("--heal-pass", dest="heal_pass", action="store_true", default=True)
    parser.add_argument("--no-heal-pass", dest="heal_pass", action="store_false")
    parser.add_argument("--size", type=int, default=512)
    parser.add_argument("--quality", type=int, default=92)
    parser.add_argument("--raw-dir", type=Path, default=Path("data/textures_raw"))
    parser.add_argument("--process-raw-dir", type=Path, default=None)
    parser.add_argument("--out-dir", type=Path, default=Path("assets/textures/map2d"))
    parser.add_argument("--preview-dir", type=Path, default=None)
    parser.add_argument("--fetch-raw-only", action="store_true")
    parser.add_argument("--skip-existing", action="store_true")
    parser.add_argument("--verify-only", action="store_true")
    args = parser.parse_args(argv)

    specs = select_texture_specs(pilot=args.pilot, only=args.only)
    if args.verify_only:
        verify_output_textures(specs, out_dir=args.out_dir, size_px=args.size)
        return 0
    if args.fetch_raw_only:
        fetch_raw_textures(
            specs,
            raw_dir=args.raw_dir,
            model=args.model,
            heal_pass=args.heal_pass,
            skip_existing=args.skip_existing,
        )
        return 0
    if args.process_raw_dir is not None:
        process_raw_textures(
            specs,
            raw_dir=args.process_raw_dir,
            out_dir=args.out_dir,
            preview_dir=args.preview_dir,
            size_px=args.size,
            quality=args.quality,
        )
        return 0

    fetch_raw_textures(
        specs,
        raw_dir=args.raw_dir,
        model=args.model,
        heal_pass=args.heal_pass,
        skip_existing=args.skip_existing,
    )
    process_raw_textures(
        specs,
        raw_dir=args.raw_dir,
        out_dir=args.out_dir,
        preview_dir=args.preview_dir,
        size_px=args.size,
        quality=args.quality,
    )
    return 0
