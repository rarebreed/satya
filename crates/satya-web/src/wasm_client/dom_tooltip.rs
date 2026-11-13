//! Reusable Floating Tooltip Positioning, Sticky/Hold Modifier Toggle & Typing-Target Guard (`dom_tooltip`).
//!
//! Keeps pointer-clamping math and sticky/hold toggle transitions host-testable while centralizing
//! `web_sys` DOM style updates (`sync_floating_tooltip_dom`) and keyboard shortcut input guards.

use crate::wasm_client::dom::{get_element, set_display, set_html};
use wasm_bindgen::JsCast;

/// Layout constants for clamping a floating tooltip card inside its host container rect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TooltipClampSpec {
    pub offset_x_px: f64,
    pub offset_y_px: f64,
    pub est_width_px: f64,
    pub est_height_px: f64,
    pub margin_px: f64,
}

impl Default for TooltipClampSpec {
    fn default() -> Self {
        Self {
            offset_x_px: 16.0,
            offset_y_px: 16.0,
            est_width_px: 290.0,
            est_height_px: 148.0,
            margin_px: 8.0,
        }
    }
}

/// Computes container-relative `(left_px, top_px)` clamped so an estimated tooltip stays inside `[0..w] x [0..h]`.
#[must_use]
pub fn clamp_tooltip_within_rect(
    cursor_x_px: f64,
    cursor_y_px: f64,
    container_width_px: f64,
    container_height_px: f64,
    spec: TooltipClampSpec,
) -> (f64, f64) {
    let min_x = spec.margin_px;
    let min_y = spec.margin_px;
    let max_x = (container_width_px - spec.est_width_px).max(min_x);
    let max_y = (container_height_px - spec.est_height_px).max(min_y);
    let left = (cursor_x_px + spec.offset_x_px).clamp(min_x, max_x);
    let top = (cursor_y_px + spec.offset_y_px).clamp(min_y, max_y);
    (left, top)
}

/// Pure host-testable toggle state pairing a persistent click/hotkey pin with a momentary hold key (e.g. `Alt`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StickyHoldToggle {
    pub pinned: bool,
    pub hold_active: bool,
}

impl StickyHoldToggle {
    #[must_use]
    pub const fn is_visible(self) -> bool {
        self.pinned || self.hold_active
    }

    pub fn toggle_pinned(&mut self) -> bool {
        self.pinned = !self.pinned;
        self.pinned
    }

    pub fn set_hold(&mut self, active: bool) -> bool {
        let changed = self.hold_active != active;
        self.hold_active = active;
        changed
    }
}

/// Updates visibility, HTML body, and clamped container-relative `(left, top)` of a floating tooltip element.
///
/// When `client_x_px <= 0.0` or `client_y_px <= 0.0` (e.g., toggled via hotkey), preserves previous coordinates.
pub fn sync_floating_tooltip_dom(
    tooltip_id: &str,
    container_id: &str,
    visible: bool,
    client_x_px: f64,
    client_y_px: f64,
    inner_html: &str,
    spec: TooltipClampSpec,
) {
    if !visible {
        set_display(tooltip_id, "none");
        return;
    }
    set_html(tooltip_id, inner_html);
    set_display(tooltip_id, "block");
    if client_x_px > 0.0
        && client_y_px > 0.0
        && let Some(container) = get_element(container_id)
        && let Some(tip) = get_element(tooltip_id)
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let rect = container.get_bounding_client_rect();
        let (rx, ry) = clamp_tooltip_within_rect(
            client_x_px - rect.left(),
            client_y_px - rect.top(),
            rect.width(),
            rect.height(),
            spec,
        );
        let _ = tip.style().set_property("left", &format!("{rx:.0}px"));
        let _ = tip.style().set_property("top", &format!("{ry:.0}px"));
    }
}

/// Returns `true` when `ev`'s target is an editable form control (`input`, `textarea`, `select`, `contenteditable`).
#[must_use]
pub fn is_keyboard_typing_target(ev: &web_sys::KeyboardEvent) -> bool {
    let Some(target) = ev.target() else {
        return false;
    };
    let Ok(el) = target.dyn_into::<web_sys::Element>() else {
        return false;
    };
    let tag = el.tag_name();
    tag.eq_ignore_ascii_case("INPUT")
        || tag.eq_ignore_ascii_case("TEXTAREA")
        || tag.eq_ignore_ascii_case("SELECT")
        || el
            .dyn_ref::<web_sys::HtmlElement>()
            .is_some_and(|h| h.is_content_editable())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tooltip_clamping_respects_margins_and_edges() {
        let spec = TooltipClampSpec::default();
        assert_eq!(clamp_tooltip_within_rect(40.0, 50.0, 800.0, 600.0, spec), (56.0, 66.0));
        // Near bottom-right corner clamps cleanly
        assert_eq!(clamp_tooltip_within_rect(790.0, 595.0, 800.0, 600.0, spec), (510.0, 452.0));
        // Tiny container never produces negative/sub-margin coordinates
        assert_eq!(clamp_tooltip_within_rect(200.0, 120.0, 120.0, 80.0, spec), (8.0, 8.0));
    }

    #[test]
    fn sticky_hold_toggle_transitions() {
        let mut t = StickyHoldToggle::default();
        assert!(!t.is_visible());
        assert!(t.set_hold(true));
        assert!(t.is_visible());
        assert!(!t.set_hold(true));
        assert!(t.set_hold(false));
        assert!(!t.is_visible());
        assert!(t.toggle_pinned());
        assert!(t.is_visible());
        assert!(!t.toggle_pinned());
        assert!(!t.is_visible());
    }
}
