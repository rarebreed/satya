"""Chroma-key alpha matting, edge despill, and aspect-exact baking for prop sprites."""

from __future__ import annotations

from collections import deque

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

from satya_ml.texture_gen.prop_catalog import PropSpriteSpec


def estimate_border_key_rgb(rgb: np.ndarray, border_px: int = 8) -> np.ndarray:
    """Estimate the chroma-key background RGB from the outer border strips."""
    h, w, _ = rgb.shape
    b = max(1, min(border_px, h // 6, w // 6))
    strips = [
        rgb[:b, :, :].reshape(-1, 3),
        rgb[-b:, :, :].reshape(-1, 3),
        rgb[:, :b, :].reshape(-1, 3),
        rgb[:, -b:, :].reshape(-1, 3),
    ]
    samples = np.concatenate(strips, axis=0)
    return np.median(samples, axis=0).astype(np.float32)


def _key_dominance(rgb_f: np.ndarray, key_color: str) -> np.ndarray:
    if key_color == "green":
        return rgb_f[..., 1] - np.maximum(rgb_f[..., 0], rgb_f[..., 2])
    # Default magenta (#FF00FF): high red & blue, low green.
    return np.minimum(rgb_f[..., 0], rgb_f[..., 2]) - rgb_f[..., 1]


def _flood_fill_border(candidate: np.ndarray) -> np.ndarray:
    """Flood-fill True pixels in `candidate` reachable from any outer image edge."""
    h, w = candidate.shape
    visited = np.zeros((h, w), dtype=bool)
    queue: deque[tuple[int, int]] = deque()

    for x in range(w):
        if candidate[0, x] and not visited[0, x]:
            visited[0, x] = True
            queue.append((0, x))
        if candidate[h - 1, x] and not visited[h - 1, x]:
            visited[h - 1, x] = True
            queue.append((h - 1, x))
    for y in range(1, h - 1):
        if candidate[y, 0] and not visited[y, 0]:
            visited[y, 0] = True
            queue.append((y, 0))
        if candidate[y, w - 1] and not visited[y, w - 1]:
            visited[y, w - 1] = True
            queue.append((y, w - 1))

    while queue:
        cy, cx = queue.popleft()
        if cy > 0 and candidate[cy - 1, cx] and not visited[cy - 1, cx]:
            visited[cy - 1, cx] = True
            queue.append((cy - 1, cx))
        if cy + 1 < h and candidate[cy + 1, cx] and not visited[cy + 1, cx]:
            visited[cy + 1, cx] = True
            queue.append((cy + 1, cx))
        if cx > 0 and candidate[cy, cx - 1] and not visited[cy, cx - 1]:
            visited[cy, cx - 1] = True
            queue.append((cy, cx - 1))
        if cx + 1 < w and candidate[cy, cx + 1] and not visited[cy, cx + 1]:
            visited[cy, cx + 1] = True
            queue.append((cy, cx + 1))

    return visited


def _dilate_mask(mask: np.ndarray, radius: int) -> np.ndarray:
    if radius <= 0:
        return mask
    k = 2 * radius + 1
    img = Image.fromarray((mask.astype(np.uint8) * 255), mode="L").filter(
        ImageFilter.MaxFilter(k)
    )
    return np.asarray(img, dtype=np.uint8) > 0


def _erode_mask(mask: np.ndarray, radius: int) -> np.ndarray:
    if radius <= 0:
        return mask
    k = 2 * radius + 1
    img = Image.fromarray((mask.astype(np.uint8) * 255), mode="L").filter(
        ImageFilter.MinFilter(k)
    )
    return np.asarray(img, dtype=np.uint8) > 0


def extract_chroma_alpha(
    rgb: np.ndarray,
    *,
    key_color: str = "magenta",
    lo_dist: float = 52.0,
    hi_dist: float = 128.0,
) -> tuple[np.ndarray, np.ndarray]:
    """Extract smooth alpha matte (`[0, 1]`) and estimated border key RGB.

    Uses border-connected flood-fill so interior bright/white details stay solid while
    anti-aliased boundary pixels transition smoothly without jagged stair-stepping.
    """
    rgb_f = rgb.astype(np.float32)
    bg_rgb = estimate_border_key_rgb(rgb)
    dist = np.linalg.norm(rgb_f - bg_rgb[None, None, :], axis=2)
    dom = _key_dominance(rgb_f, key_color)

    # Candidate exterior background pixels (either close to border key or clearly chroma-dominant).
    candidate_bg = ((dist < hi_dist) & (dom > 18.0)) | (dist < 38.0)
    bg_connected = _flood_fill_border(candidate_bg)

    # Punch out unmistakable enclosed holes (e.g. bed-rail gaps or handle openings) if chroma-key.
    pure_holes = (dist < 76.0) & (dom > 48.0)
    definite_bg = bg_connected | pure_holes

    # Narrow 2px transition ribbon just inside the foreground boundary for soft AA.
    near_bg = _dilate_mask(definite_bg, 2)
    edge_band = near_bg & ~_erode_mask(definite_bg, 1)

    alpha = np.where(definite_bg, 0.0, 1.0).astype(np.float32)
    ramp = np.clip((dist - lo_dist) / max(hi_dist - lo_dist, 1.0), 0.0, 1.0)
    # Only soften pixels inside the boundary ribbon; keep solid core at 1.0.
    alpha = np.where(edge_band & ~definite_bg, np.maximum(ramp, 0.0), alpha)
    alpha = np.where(edge_band & definite_bg & (dom < 45.0), np.minimum(ramp, 0.65), alpha)

    # Gently smooth alpha strictly inside the narrow edge band.
    alpha_u8 = np.rint(np.clip(alpha, 0.0, 1.0) * 255.0).astype(np.uint8)
    blurred_u8 = np.asarray(
        Image.fromarray(alpha_u8, mode="L").filter(ImageFilter.GaussianBlur(0.6)),
        dtype=np.uint8,
    )
    blurred = blurred_u8.astype(np.float32) / 255.0
    alpha = np.where(edge_band, blurred, alpha)
    alpha = np.where(_erode_mask(definite_bg, 1), 0.0, alpha)
    return np.clip(alpha, 0.0, 1.0), bg_rgb


def despill_edges(
    rgb: np.ndarray,
    alpha: np.ndarray,
    bg_rgb: np.ndarray,
    *,
    key_color: str = "magenta",
) -> np.ndarray:
    """Remove chroma-key fringe from semi-transparent and 2px boundary pixels."""
    rgb_f = rgb.astype(np.float32).copy()
    edge_halo = _dilate_mask(alpha < 0.995, 3) & (alpha > 0.0)

    # 1. Color un-mixing on translucent edge pixels.
    translucent = (alpha > 0.02) & (alpha < 0.98)
    if np.any(translucent):
        a = alpha[translucent, None]
        unmixed = (rgb_f[translucent] - (1.0 - a) * bg_rgb[None, :]) / np.maximum(a, 0.22)
        unmixed = np.clip(unmixed, 0.0, 255.0)
        blend = np.clip((0.98 - a) * 1.4, 0.0, 1.0)
        rgb_f[translucent] = rgb_f[translucent] * (1.0 - blend) + unmixed * blend

    # 2. Suppress residual chroma-key cast along boundary halo.
    if np.any(edge_halo):
        r = rgb_f[..., 0]
        g = rgb_f[..., 1]
        b = rgb_f[..., 2]
        if key_color == "green":
            spill = edge_halo & (g > np.maximum(r, b) + 10.0)
            rgb_f[..., 1] = np.where(spill, np.maximum(r, b) + 4.0, g)
        else:
            spill = edge_halo & (r > g + 10.0) & (b > g + 10.0)
            cap = g + 6.0
            rgb_f[..., 0] = np.where(spill, np.minimum(r, cap), r)
            rgb_f[..., 2] = np.where(spill, np.minimum(b, cap), b)

    return np.rint(np.clip(rgb_f, 0.0, 255.0)).astype(np.uint8)


def _keep_main_foreground(alpha_u8: np.ndarray, *, min_alpha: int = 18) -> np.ndarray:
    """Zero out tiny isolated specks while preserving multi-part objects (e.g. twin bins)."""
    fg = alpha_u8 >= min_alpha
    if not np.any(fg):
        return alpha_u8
    h, w = fg.shape
    min_area = max(16, int(0.002 * h * w))
    visited = np.zeros((h, w), dtype=bool)
    keep = np.zeros((h, w), dtype=bool)

    ys, xs = np.nonzero(fg)
    for sy, sx in zip(ys.tolist(), xs.tolist(), strict=False):
        if visited[sy, sx]:
            continue
        comp: list[tuple[int, int]] = []
        queue: deque[tuple[int, int]] = deque([(sy, sx)])
        visited[sy, sx] = True
        while queue:
            cy, cx = queue.popleft()
            comp.append((cy, cx))
            for ny, nx in ((cy - 1, cx), (cy + 1, cx), (cy, cx - 1), (cy, cx + 1)):
                if 0 <= ny < h and 0 <= nx < w and fg[ny, nx] and not visited[ny, nx]:
                    visited[ny, nx] = True
                    queue.append((ny, nx))
        if len(comp) >= min_area:
            for cy, cx in comp:
                keep[cy, cx] = True

    if not np.any(keep):
        return alpha_u8
    return np.where(_dilate_mask(keep, 1), alpha_u8, 0).astype(np.uint8)


def crop_and_fit_rgba(
    rgba: np.ndarray,
    target_w: int,
    target_h: int,
    *,
    pad_frac: float = 0.03,
) -> np.ndarray:
    """Tight-crop foreground and premultiplied-LANCZOS fit into `(target_h, target_w, 4)`."""
    alpha_clean = _keep_main_foreground(rgba[..., 3], min_alpha=18)
    ys, xs = np.nonzero(alpha_clean >= 18)
    if len(ys) == 0:
        return np.zeros((target_h, target_w, 4), dtype=np.uint8)

    y0, y1 = int(ys.min()), int(ys.max()) + 1
    x0, x1 = int(xs.min()), int(xs.max()) + 1
    cropped = rgba[y0:y1, x0:x1].copy()
    cropped[..., 3] = alpha_clean[y0:y1, x0:x1]

    cw = x1 - x0
    ch = y1 - y0
    avail_w = max(1, int(round(target_w * (1.0 - 2.0 * pad_frac))))
    avail_h = max(1, int(round(target_h * (1.0 - 2.0 * pad_frac))))
    scale = min(avail_w / float(cw), avail_h / float(ch))
    new_w = max(1, min(avail_w, int(round(cw * scale))))
    new_h = max(1, min(avail_h, int(round(ch * scale))))

    # Premultiply before LANCZOS so transparent border pixels contribute zero color bleed.
    a_f = cropped[..., 3:4].astype(np.float32) / 255.0
    premul_rgb = np.rint(cropped[..., :3].astype(np.float32) * a_f).astype(np.uint8)
    premul_rgba = np.dstack([premul_rgb, cropped[..., 3]])

    resized = np.asarray(
        Image.fromarray(premul_rgba, mode="RGBA").resize((new_w, new_h), Image.Resampling.LANCZOS),
        dtype=np.uint8,
    )
    out_a = resized[..., 3:4].astype(np.float32) / 255.0
    straight_rgb = np.where(
        out_a > 0.01,
        np.clip(resized[..., :3].astype(np.float32) / np.maximum(out_a, 1e-4), 0.0, 255.0),
        0.0,
    )
    fitted = np.dstack([np.rint(straight_rgb).astype(np.uint8), resized[..., 3]])

    canvas = np.zeros((target_h, target_w, 4), dtype=np.uint8)
    ox = (target_w - new_w) // 2
    oy = (target_h - new_h) // 2
    canvas[oy : oy + new_h, ox : ox + new_w] = fitted
    return canvas


def matte_prop_sprite(raw_rgb: np.ndarray, spec: PropSpriteSpec) -> np.ndarray:
    """Run full chroma extraction, edge despill, crop, and fit for one prop sprite."""
    alpha, bg_rgb = extract_chroma_alpha(raw_rgb, key_color=spec.key_color)
    clean_rgb = despill_edges(raw_rgb, alpha, bg_rgb, key_color=spec.key_color)
    alpha_u8 = np.rint(np.clip(alpha, 0.0, 1.0) * 255.0).astype(np.uint8)
    rgba = np.dstack([clean_rgb, alpha_u8])
    return crop_and_fit_rgba(
        rgba,
        spec.target_w_px,
        spec.target_h_px,
        pad_frac=spec.pad_frac,
    )


def verify_sprite_alpha_quality(
    rgba: np.ndarray, *, key_color: str = "magenta"
) -> dict[str, float]:
    """Compute alpha cutout & spill metrics to catch bad masks or leftover fringes."""
    alpha = rgba[..., 3]
    corners = np.array([alpha[0, 0], alpha[0, -1], alpha[-1, 0], alpha[-1, -1]], dtype=np.float32)
    corner_alpha_max = float(corners.max())
    fg_mask = alpha >= 128
    fg_coverage = float(np.mean(fg_mask))

    visible_edge = (alpha > 16) & (alpha < 245)
    if np.any(visible_edge):
        dom = _key_dominance(rgba[..., :3].astype(np.float32), key_color)
        edge_spill_frac = float(np.mean(dom[visible_edge] > 30.0))
    else:
        edge_spill_frac = 0.0

    return {
        "corner_alpha_max": corner_alpha_max,
        "fg_coverage": fg_coverage,
        "edge_spill_frac": edge_spill_frac,
    }


def render_prop_contact_sheet(
    sprites: list[tuple[PropSpriteSpec, np.ndarray]], *, cols: int = 4
) -> np.ndarray:
    """Render a labeled 3-strip (vinyl / terrazzo / dark slate) inspection grid."""
    cols = max(1, cols)
    rows = max(1, (len(sprites) + cols - 1) // cols)
    tile_w, tile_h = 320, 300
    label_h = 34
    pad = 14
    sheet = Image.new("RGB", (cols * tile_w + pad, rows * tile_h + pad), (30, 34, 40))
    draw = ImageDraw.Draw(sheet)

    strip_colors = [(214, 224, 228), (216, 212, 203), (58, 65, 74)]
    for idx, (spec, rgba) in enumerate(sprites):
        r = idx // cols
        c = idx % cols
        x0 = pad + c * tile_w
        y0 = pad + r * tile_h
        box_w = tile_w - pad
        box_h = tile_h - pad - label_h

        for s_idx, col in enumerate(strip_colors):
            sx0 = x0 + (box_w * s_idx) // 3
            sx1 = x0 + (box_w * (s_idx + 1)) // 3
            draw.rectangle([sx0, y0, sx1, y0 + box_h], fill=col)

        sh, sw = rgba.shape[:2]
        scale = min((box_w - 20) / float(sw), (box_h - 20) / float(sh))
        dw = max(1, int(round(sw * scale)))
        dh = max(1, int(round(sh * scale)))
        sprite_img = Image.fromarray(rgba, mode="RGBA").resize((dw, dh), Image.Resampling.LANCZOS)
        px = x0 + (box_w - dw) // 2
        py = y0 + (box_h - dh) // 2
        sheet.paste(sprite_img, (px, py), sprite_img)
        draw.rectangle([x0, y0, x0 + box_w, y0 + box_h], outline=(90, 98, 110), width=1)
        draw.text(
            (x0 + 4, y0 + box_h + 6),
            f"{spec.id} ({spec.base_w_cells}x{spec.base_h_cells}m)",
            fill=(232, 236, 242),
        )

    return np.asarray(sheet, dtype=np.uint8)
