"""Toroidal de-lighting, cross-seam blending, and DP quilting for seamless SVG tiles."""

from __future__ import annotations

import numpy as np
from PIL import Image

from satya_ml.texture_gen.catalog import TextureSpec


def delight_albedo(
    img_rgb: np.ndarray,
    *,
    sigma_frac: float = 0.18,
    strength: float = 0.78,
    exposure_scale: float = 1.0,
) -> np.ndarray:
    """Remove low-frequency lighting gradients while preserving local painterly contrast."""
    if strength <= 0.0:
        out = (img_rgb.astype(np.float32) / 255.0) * float(exposure_scale)
        return np.rint(np.clip(out, 0.0, 1.0) * 255.0).astype(np.uint8)
    rgb = img_rgb.astype(np.float32) / 255.0
    h, w, _ = rgb.shape
    lum = 0.2126 * rgb[..., 0] + 0.7152 * rgb[..., 1] + 0.0722 * rgb[..., 2]

    sigma_px = max(4.0, sigma_frac * float(min(h, w)))
    pad = min(max(16, int(round(2.5 * sigma_px))), max(h, w))
    lum_pad = np.pad(lum, ((pad, pad), (pad, pad)), mode="reflect")
    hp, wp = lum_pad.shape
    fy = np.fft.fftfreq(hp)[:, None]
    fx = np.fft.rfftfreq(wp)[None, :]
    gauss_fft = np.exp(-2.0 * (np.pi * sigma_px) ** 2 * (fx * fx + fy * fy))
    low_pad = np.fft.irfft2(np.fft.rfft2(lum_pad) * gauss_fft, s=(hp, wp)).astype(np.float32)
    low_lum = low_pad[pad : pad + h, pad : pad + w]

    target_lum = max(float(np.mean(lum)), 0.06)
    gain = (target_lum / np.maximum(low_lum, 0.06)) ** float(strength)
    gain = np.clip(gain, 0.45, 2.2)[..., None]
    out = np.clip(rgb * gain * float(exposure_scale), 0.0, 1.0)
    return np.rint(out * 255.0).astype(np.uint8)


def roll_half(img_rgb: np.ndarray) -> np.ndarray:
    """Toroidally shift an image by ``(H // 2, W // 2)`` so outer edges meet at the center."""
    h, w, _ = img_rgb.shape
    return np.roll(np.roll(img_rgb, h // 2, axis=0), w // 2, axis=1)


def _match_color_stats(src: np.ndarray, ref: np.ndarray) -> np.ndarray:
    src_f = src.astype(np.float32)
    ref_f = ref.astype(np.float32)
    src_mean = src_f.mean(axis=(0, 1), keepdims=True)
    ref_mean = ref_f.mean(axis=(0, 1), keepdims=True)
    src_std = np.maximum(src_f.std(axis=(0, 1), keepdims=True), 1.0)
    ref_std = np.maximum(ref_f.std(axis=(0, 1), keepdims=True), 1.0)
    matched = (src_f - src_mean) * (ref_std / src_std) + ref_mean
    return np.clip(matched, 0.0, 255.0)


def blend_center_cross_heal(
    rolled_rgb: np.ndarray, healed_rgb: np.ndarray, *, band_frac: float = 0.24
) -> np.ndarray:
    """Composite a center-healed image strictly inside the interior ``+`` cross, then un-roll."""
    h, w, _ = rolled_rgb.shape
    if healed_rgb.shape[:2] != (h, w):
        healed_img = Image.fromarray(healed_rgb).resize((w, h), Image.Resampling.LANCZOS)
        healed_rgb = np.asarray(healed_img, dtype=np.uint8)
    healed_matched = _match_color_stats(healed_rgb, rolled_rgb)

    y = np.arange(h, dtype=np.float32)
    x = np.arange(w, dtype=np.float32)
    half_y = max(8.0, band_frac * h * 0.5)
    half_x = max(8.0, band_frac * w * 0.5)

    wy_cross = 0.5 * (1.0 + np.cos(np.pi * np.clip(np.abs(y - (h * 0.5)) / half_y, 0.0, 1.0)))
    wx_cross = 0.5 * (1.0 + np.cos(np.pi * np.clip(np.abs(x - (w * 0.5)) / half_x, 0.0, 1.0)))

    # Keep outer border locked to rolled_rgb (0.0 at perimeter) so un-rolling adds zero seam.
    edge_y = np.maximum(0.0, np.sin(np.pi * y / max(h - 1, 1))) ** 0.75
    edge_x = np.maximum(0.0, np.sin(np.pi * x / max(w - 1, 1))) ** 0.75
    cross_mask = np.maximum(wy_cross[:, None], wx_cross[None, :])
    mask = (cross_mask * edge_y[:, None] * edge_x[None, :])[..., None]

    blended = rolled_rgb.astype(np.float32) * (1.0 - mask) + healed_matched * mask
    unrolled = np.roll(
        np.roll(np.clip(np.rint(blended), 0, 255).astype(np.uint8), -(h // 2), axis=0),
        -(w // 2),
        axis=1,
    )
    return unrolled


def _min_seam_cut(err: np.ndarray, *, min_idx: int, max_idx: int, periodic: bool) -> np.ndarray:
    """Compute a 1-pixel-step minimum-energy seam across ``err`` (shape ``[length, ov]``)."""
    length, ov = err.shape
    dp = np.full((length, ov), np.inf, dtype=np.float64)
    back = np.zeros((length, ov), dtype=np.int32)

    if periodic:
        anchor = min_idx + int(np.argmin(err[0, min_idx : max_idx + 1] + err[-1, min_idx : max_idx + 1]))
        dp[0, anchor] = float(err[0, anchor])
    else:
        dp[0, min_idx : max_idx + 1] = err[0, min_idx : max_idx + 1]

    for r in range(1, length):
        for c in range(min_idx, max_idx + 1):
            c_lo = max(min_idx, c - 1)
            c_hi = min(max_idx, c + 1)
            prev_slice = dp[r - 1, c_lo : c_hi + 1]
            rel = int(np.argmin(prev_slice))
            best_prev = c_lo + rel
            dp[r, c] = err[r, c] + dp[r - 1, best_prev]
            back[r, c] = best_prev

    cut = np.empty(length, dtype=np.int32)
    if periodic:
        c_lo = max(min_idx, anchor - 1)
        c_hi = min(max_idx, anchor + 1)
        cut[-1] = c_lo + int(np.argmin(dp[-1, c_lo : c_hi + 1]))
    else:
        cut[-1] = min_idx + int(np.argmin(dp[-1, min_idx : max_idx + 1]))
    for r in range(length - 1, 0, -1):
        cut[r - 1] = back[r, cut[r]]
    return cut


def _quilt_horizontal(img_rgb: np.ndarray, *, overlap_frac: float, feather_px: int) -> np.ndarray:
    _, w, _ = img_rgb.shape
    ov = int(np.clip(round(w * overlap_frac), 2 * feather_px + 8, w // 3))
    w_out = w - ov
    left_a = img_rgb[:, :ov].astype(np.float32)
    right_b = img_rgb[:, w - ov :].astype(np.float32)
    err = np.sum((right_b - left_a) ** 2, axis=2)

    min_j = feather_px + 2
    max_j = ov - feather_px - 3
    cut = _min_seam_cut(err, min_idx=min_j, max_idx=max_j, periodic=False)

    cols = np.arange(ov, dtype=np.float32)[None, :]
    t = np.clip((cols - cut[:, None]) / float(2 * feather_px + 1) + 0.5, 0.0, 1.0)
    alpha = (0.5 - 0.5 * np.cos(np.pi * t))[..., None]

    out = img_rgb[:, :w_out].astype(np.float32).copy()
    out[:, :ov] = right_b * (1.0 - alpha) + left_a * alpha
    return np.clip(np.rint(out), 0, 255).astype(np.uint8)


def _quilt_vertical_periodic(
    img_rgb: np.ndarray, *, overlap_frac: float, feather_px: int
) -> np.ndarray:
    h, _, _ = img_rgb.shape
    ov = int(np.clip(round(h * overlap_frac), 2 * feather_px + 8, h // 3))
    h_out = h - ov
    top_a = img_rgb[:ov, :].astype(np.float32)
    bot_b = img_rgb[h - ov :, :].astype(np.float32)
    err = np.sum((bot_b - top_a) ** 2, axis=2).T  # shape [W, ov]

    min_i = feather_px + 2
    max_i = ov - feather_px - 3
    cut = _min_seam_cut(err, min_idx=min_i, max_idx=max_i, periodic=True)

    rows = np.arange(ov, dtype=np.float32)[:, None]
    t = np.clip((rows - cut[None, :]) / float(2 * feather_px + 1) + 0.5, 0.0, 1.0)
    alpha = (0.5 - 0.5 * np.cos(np.pi * t))[..., None]

    out = img_rgb[:h_out, :].astype(np.float32).copy()
    out[:ov, :] = bot_b * (1.0 - alpha) + top_a * alpha
    return np.clip(np.rint(out), 0, 255).astype(np.uint8)


def toroidal_dp_quilt(
    img_rgb: np.ndarray, *, overlap_frac: float = 0.14, feather_px: int = 2
) -> np.ndarray:
    """Splice opposite edges along minimum-error crevices so X and Y wrap without blurring."""
    horiz = _quilt_horizontal(img_rgb, overlap_frac=overlap_frac, feather_px=feather_px)
    return _quilt_vertical_periodic(horiz, overlap_frac=overlap_frac, feather_px=feather_px)


def periodic_lanczos_resize(tile_rgb: np.ndarray, out_px: int) -> np.ndarray:
    """Resize a periodic tile with exact integer phase preservation via 3x3 tiling."""
    tiled = np.tile(tile_rgb, (3, 3, 1))
    resized = Image.fromarray(tiled).resize(
        (3 * out_px, 3 * out_px), Image.Resampling.LANCZOS
    )
    arr = np.asarray(resized, dtype=np.uint8)
    return arr[out_px : 2 * out_px, out_px : 2 * out_px].copy()


def finalize_tile(
    raw_rgb: np.ndarray,
    spec: TextureSpec,
    *,
    healed_rolled_rgb: np.ndarray | None = None,
    out_px: int = 512,
) -> np.ndarray:
    """Run full de-lighting, optional cross healing, DP quilting, and periodic resize."""
    delit = delight_albedo(
        raw_rgb,
        sigma_frac=spec.delight_sigma_frac,
        strength=spec.delight_strength,
        exposure_scale=spec.exposure_scale,
    )
    if healed_rolled_rgb is not None:
        delit = blend_center_cross_heal(roll_half(delit), healed_rolled_rgb)
    quilted = toroidal_dp_quilt(delit, overlap_frac=0.14, feather_px=2)
    return periodic_lanczos_resize(quilted, out_px=out_px)


def render_contact_sheet_3x3(tile_rgb: np.ndarray, *, out_px: int = 768) -> np.ndarray:
    """Render a 3x3 tiled grid preview to verify invisible seams across repetitions."""
    tiled = np.tile(tile_rgb, (3, 3, 1))
    resized = Image.fromarray(tiled).resize((out_px, out_px), Image.Resampling.LANCZOS)
    return np.asarray(resized, dtype=np.uint8)


def wrap_seam_ratio(tile_rgb: np.ndarray) -> tuple[float, float]:
    """Return ``(horiz_ratio, vert_ratio)`` comparing wrap-edge diff against natural neighbor diff."""
    f = tile_rgb.astype(np.float32)
    nat_x = float(np.mean(np.abs(f[:, 1:, :] - f[:, :-1, :]))) + 1e-4
    wrap_x = float(np.mean(np.abs(f[:, 0, :] - f[:, -1, :])))
    nat_y = float(np.mean(np.abs(f[1:, :, :] - f[:-1, :, :]))) + 1e-4
    wrap_y = float(np.mean(np.abs(f[0, :, :] - f[-1, :, :])))
    return (wrap_x / nat_x, wrap_y / nat_y)
