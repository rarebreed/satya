//! Collapsible Workspace Drawers, Floating Overlay Panes, Column Splitter Resize,
//! and Notebook WebGPU Viewport Refresh (`#workspace-panel`).
//!
//! Synchronizes all combinations of `(Docked, Floating, Collapsed)` across Session Chat
//! (`#chat-card`), Task Resolution (`#ttrpg-drawer`), and Web Cams (`#webrtc-card`),
//! ensuring `#notebook-column` expands to fill all freed horizontal space when panes
//! collapse or float even after a custom `#workspace-col-splitter` percentage has been persisted.

use crate::wasm_client::dom::{
    document, get_element, local_storage_get, local_storage_set, set_class, set_text, window,
};
use crate::wasm_client::state::{with_state, with_state_mut};
use js_sys::Reflect;
use satya_schema::SessionMode;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement, MouseEvent};

/// Storage key persisting the user's custom `#notebook-column` width percentage (`22.0..=72.0`).
const STORAGE_KEY_NOTEBOOK_WIDTH_PCT: &str = "satya.workspace.notebook_width_pct";

/// Workspace panes that support collapsing to `#right-drawer-rails` or floating over the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatingPaneKind {
    Chat,
    Ttrpg,
    Webcam,
}

impl FloatingPaneKind {
    /// Parses a JS callback pane slug (`"chat"`, `"ttrpg"`, or `"webcam"`).
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug.trim().to_ascii_lowercase().as_str() {
            "chat" => Some(Self::Chat),
            "ttrpg" => Some(Self::Ttrpg),
            "webcam" => Some(Self::Webcam),
            _ => None,
        }
    }
}

struct PaneDomSpec {
    card_id: &'static str,
    base_class: &'static str,
    drag_header_id: &'static str,
    drag_header_base_class: &'static str,
    float_btn_id: &'static str,
    toggle_btn_id: &'static str,
    expand_btn_id: &'static str,
    menu_btn_id: &'static str,
    rail_id: &'static str,
    aria_title: &'static str,
    rail_title: &'static str,
}

const PANE_DOM_SPECS: [PaneDomSpec; 3] = [
    PaneDomSpec {
        card_id: "chat-card",
        base_class: "card chat-card",
        drag_header_id: "chat-drag-header",
        drag_header_base_class: "card-header-inline pane-drag-header",
        float_btn_id: "btn-toggle-chat-float",
        toggle_btn_id: "btn-toggle-chat-drawer",
        expand_btn_id: "btn-expand-chat-drawer",
        menu_btn_id: "btn-menu-toggle-chat-pane",
        rail_id: "chat-drawer-rail",
        aria_title: "Session Chat",
        rail_title: "Session Chat",
    },
    PaneDomSpec {
        card_id: "ttrpg-drawer",
        base_class: "card ttrpg-card",
        drag_header_id: "ttrpg-drag-header",
        drag_header_base_class: "card-header-inline pane-drag-header",
        float_btn_id: "btn-toggle-ttrpg-float",
        toggle_btn_id: "btn-toggle-ttrpg-drawer",
        expand_btn_id: "btn-expand-ttrpg-drawer",
        menu_btn_id: "btn-menu-toggle-ttrpg-pane",
        rail_id: "ttrpg-drawer-rail",
        aria_title: "Task Resolution",
        rail_title: "Task Roller",
    },
    PaneDomSpec {
        card_id: "webrtc-card",
        base_class: "card webrtc-card",
        drag_header_id: "webrtc-drag-header",
        drag_header_base_class: "webrtc-header pane-drag-header",
        float_btn_id: "btn-toggle-webcam-float",
        toggle_btn_id: "btn-toggle-webcam-drawer",
        expand_btn_id: "btn-expand-webcam-drawer",
        menu_btn_id: "btn-menu-toggle-webcam-pane",
        rail_id: "webcam-drawer-rail",
        aria_title: "Web Cams",
        rail_title: "Web Cams",
    },
];

fn spec_for_pane(kind: FloatingPaneKind) -> &'static PaneDomSpec {
    match kind {
        FloatingPaneKind::Chat => &PANE_DOM_SPECS[0],
        FloatingPaneKind::Ttrpg => &PANE_DOM_SPECS[1],
        FloatingPaneKind::Webcam => &PANE_DOM_SPECS[2],
    }
}

/// Computed CSS classes and container `display` modes for the active workspace grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceLayoutSpec {
    pub workspace_classes: String,
    pub center_col_class: &'static str,
    pub center_col_display: &'static str,
    pub webrtc_col_class: &'static str,
    pub webrtc_col_display: &'static str,
    pub rails_display: &'static str,
}

/// Computes deterministic CSS Grid classes and container display values for all `(Docked, Floating, Collapsed)` combinations.
#[must_use]
pub fn compute_workspace_layout_classes(
    chat_floating: bool,
    chat_collapsed: bool,
    ttrpg_floating: bool,
    ttrpg_collapsed: bool,
    webcam_floating: bool,
    webcam_collapsed: bool,
    ttrpg_hidden_by_mode: bool,
) -> WorkspaceLayoutSpec {
    let ttrpg_eff_collapsed = ttrpg_collapsed || ttrpg_hidden_by_mode;
    let ttrpg_eff_floating = !ttrpg_hidden_by_mode && ttrpg_floating;

    let chat_docked = !chat_collapsed && !chat_floating;
    let chat_float_vis = !chat_collapsed && chat_floating;
    let ttrpg_docked = !ttrpg_eff_collapsed && !ttrpg_eff_floating;
    let ttrpg_float_vis = !ttrpg_eff_collapsed && ttrpg_eff_floating;
    let webcam_docked = !webcam_collapsed && !webcam_floating;

    let center_docked = chat_docked || ttrpg_docked;
    let center_has_floating = chat_float_vis || ttrpg_float_vis;
    let any_rail_collapsed =
        webcam_collapsed || chat_collapsed || (!ttrpg_hidden_by_mode && ttrpg_collapsed);

    let mut workspace_classes = String::from("workspace-columns");
    if !webcam_docked && any_rail_collapsed {
        workspace_classes.push_str(" webcam-collapsed");
    } else if !webcam_docked {
        workspace_classes.push_str(" webcam-undocked");
    }

    if !center_docked {
        workspace_classes.push_str(" center-collapsed");
    } else {
        if ttrpg_eff_collapsed {
            workspace_classes.push_str(" ttrpg-collapsed");
        } else if !ttrpg_docked {
            workspace_classes.push_str(" ttrpg-undocked");
        }
        if chat_collapsed {
            workspace_classes.push_str(" chat-collapsed");
        } else if !chat_docked {
            workspace_classes.push_str(" chat-undocked");
        }
    }

    let (center_col_class, center_col_display) = if center_docked {
        ("col-center", "flex")
    } else if center_has_floating {
        ("col-center is-undocked", "contents")
    } else {
        ("col-center is-collapsed", "none")
    };

    let (webrtc_col_class, webrtc_col_display) = if webcam_docked {
        ("col-webcams", "flex")
    } else if any_rail_collapsed {
        ("col-webcams is-collapsed", "flex")
    } else {
        ("col-webcams is-undocked", "contents")
    };

    let rails_display = if any_rail_collapsed { "flex" } else { "none" };

    WorkspaceLayoutSpec {
        workspace_classes,
        center_col_class,
        center_col_display,
        webrtc_col_class,
        webrtc_col_display,
        rails_display,
    }
}

/// Clamps floating pane `(left_px, top_px)` coordinates so the drag header stays reachable within the viewport.
#[must_use]
pub fn clamp_floating_pane_position(
    left_px: f64,
    top_px: f64,
    card_w_px: f64,
    _card_h_px: f64,
    viewport_w_px: f64,
    viewport_h_px: f64,
) -> (f64, f64) {
    let min_visible_w = card_w_px.clamp(140.0, 260.0);
    let max_left = (viewport_w_px - min_visible_w).max(8.0);
    let max_top = (viewport_h_px - 56.0).max(8.0);
    (left_px.clamp(8.0, max_left), top_px.clamp(8.0, max_top))
}

/// Moves `kind` to the top slot (`index 2`, CSS `z-index: 97`) while preserving relative order of the other panes.
pub(crate) fn bring_floating_pane_to_front(
    order: &mut [FloatingPaneKind; 3],
    kind: FloatingPaneKind,
) {
    if order[2] == kind {
        return;
    }
    if order[1] == kind {
        order.swap(1, 2);
    } else if order[0] == kind {
        order.rotate_left(1);
    }
}

fn z_index_for_pane(order: &[FloatingPaneKind; 3], kind: FloatingPaneKind) -> u32 {
    let pos = order.iter().position(|&k| k == kind).unwrap_or(0);
    95 + (pos as u32)
}

fn strip_header_drag_attrs(header: &Element, base_class: &str) {
    header.set_class_name(base_class);
    let _ = header.remove_attribute("tabindex");
    let _ = header.remove_attribute("role");
    let _ = header.remove_attribute("aria-label");
}

fn apply_floating_card_xy(card: &HtmlElement, left_px: f64, top_px: f64) {
    let _ = card.style().remove_property("right");
    let _ = card.style().set_property("left", &format!("{left_px:.0}px"));
    let _ = card.style().set_property("top", &format!("{top_px:.0}px"));
}

fn sync_drawer_rail_and_aria(collapsed: bool, spec: &PaneDomSpec) {
    if let Some(rail) = get_element(spec.rail_id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        let _ = rail
            .style()
            .set_property("display", if collapsed { "flex" } else { "none" });
    }
    let expanded_str = if collapsed { "false" } else { "true" };
    for btn_id in [spec.toggle_btn_id, spec.expand_btn_id, spec.menu_btn_id] {
        if let Some(btn) = get_element(btn_id) {
            let _ = btn.set_attribute("aria-expanded", expanded_str);
        }
    }
    let toggle_label = if collapsed {
        format!("◂ {}", spec.rail_title)
    } else {
        "Collapse ▸".to_string()
    };
    set_text(spec.toggle_btn_id, &toggle_label);
    let menu_label = if collapsed {
        format!("Show {}", spec.rail_title)
    } else {
        format!("Hide {}", spec.rail_title)
    };
    set_text(spec.menu_btn_id, &menu_label);
}

fn sync_float_button(spec: &PaneDomSpec, floating: bool) {
    let label = if floating { "Dock" } else { "Float" };
    set_text(spec.float_btn_id, label);
    if let Some(btn) = get_element(spec.float_btn_id) {
        let _ = btn.set_attribute("aria-label", &format!("{label} {}", spec.aria_title));
        let _ = btn.set_attribute("aria-pressed", if floating { "true" } else { "false" });
    }
}

fn sync_pane_card_and_header(
    spec: &PaneDomSpec,
    collapsed: bool,
    floating: bool,
    saved_pos_px: Option<(f64, f64)>,
    z_index: u32,
) {
    let Some(card) = get_element(spec.card_id).and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let header_el = get_element(spec.drag_header_id);

    if collapsed {
        card.set_class_name(&format!("{} is-collapsed", spec.base_class));
        let _ = card.set_attribute("inert", "");
        let _ = card.style().set_property("display", "none");
        if let Some(header) = header_el {
            strip_header_drag_attrs(&header, spec.drag_header_base_class);
        }
    } else if floating {
        card.set_class_name(&format!("{} is-floating", spec.base_class));
        let _ = card.remove_attribute("inert");
        let _ = card.style().set_property("display", "flex");
        let _ = card.style().set_property("z-index", &z_index.to_string());
        if let Some((left_px, top_px)) = saved_pos_px {
            apply_floating_card_xy(&card, left_px, top_px);
        } else {
            for prop in ["left", "top", "right"] {
                let _ = card.style().remove_property(prop);
            }
        }
        if let Some(header) = header_el {
            let _ = header.set_attribute("tabindex", "0");
            let _ = header.set_attribute("role", "group");
            let _ = header.set_attribute(
                "aria-label",
                &format!("Move floating {} pane (use arrow keys)", spec.aria_title),
            );
        }
    } else {
        card.set_class_name(spec.base_class);
        let _ = card.remove_attribute("inert");
        let _ = card.style().set_property("display", "flex");
        for prop in ["left", "top", "right", "width", "height", "z-index"] {
            let _ = card.style().remove_property(prop);
        }
        if let Some(header) = header_el {
            strip_header_drag_attrs(&header, spec.drag_header_base_class);
        }
    }
}

fn sync_floating_z_indices() {
    let (chat_vis, ttrpg_vis, cam_vis, order) = with_state(|st| {
        (
            st.chat_floating && !st.chat_collapsed,
            st.session_mode != SessionMode::Meeting && st.ttrpg_floating && !st.ttrpg_collapsed,
            st.webcam_floating && !st.webcam_collapsed,
            st.floating_z_order,
        )
    });
    for (kind, active) in [
        (FloatingPaneKind::Chat, chat_vis),
        (FloatingPaneKind::Ttrpg, ttrpg_vis),
        (FloatingPaneKind::Webcam, cam_vis),
    ] {
        if active
            && let Some(card) =
                get_element(spec_for_pane(kind).card_id).and_then(|e| e.dyn_into::<HtmlElement>().ok())
        {
            let _ = card
                .style()
                .set_property("z-index", &z_index_for_pane(&order, kind).to_string());
        }
    }
}

/// Triggers a canvas redraw on whichever WebGPU stage tab is active after workspace column resizing/collapsing.
pub(crate) fn refresh_active_notebook_stage_on_layout_change() {
    let tab = with_state(|st| st.active_notebook_tab.clone());
    match tab.as_str() {
        "maps" => crate::wasm_client::map_3d::refresh_map_wgpu_if_active(),
        "dice" => crate::wasm_client::dice_3d::sync_dice_camera_from_dom_and_render(),
        "model" => crate::wasm_client::model_3d_wgpu::sync_3d_model_viewer_from_dom(),
        _ => {}
    }
}

/// Reconciles DOM classes, ARIA attributes, rail tabs, floating overlays, and grid layout across all workspace panes.
pub(crate) fn sync_workspace_drawers_dom() {
    let snap = with_state(|st| {
        (
            st.chat_floating,
            st.chat_collapsed,
            st.chat_float_pos_px,
            st.ttrpg_floating,
            st.ttrpg_collapsed,
            st.ttrpg_float_pos_px,
            st.webcam_floating,
            st.webcam_collapsed,
            st.webcam_float_pos_px,
            st.session_mode == SessionMode::Meeting,
            st.floating_z_order,
        )
    });
    let (
        chat_floating,
        chat_collapsed,
        chat_pos,
        ttrpg_floating,
        ttrpg_collapsed,
        ttrpg_pos,
        webcam_floating,
        webcam_collapsed,
        webcam_pos,
        ttrpg_hidden_by_mode,
        z_order,
    ) = snap;

    let layout = compute_workspace_layout_classes(
        chat_floating,
        chat_collapsed,
        ttrpg_floating,
        ttrpg_collapsed,
        webcam_floating,
        webcam_collapsed,
        ttrpg_hidden_by_mode,
    );

    sync_pane_card_and_header(
        spec_for_pane(FloatingPaneKind::Chat),
        chat_collapsed,
        chat_floating,
        chat_pos,
        z_index_for_pane(&z_order, FloatingPaneKind::Chat),
    );
    sync_pane_card_and_header(
        spec_for_pane(FloatingPaneKind::Ttrpg),
        ttrpg_collapsed || ttrpg_hidden_by_mode,
        ttrpg_floating && !ttrpg_hidden_by_mode,
        ttrpg_pos,
        z_index_for_pane(&z_order, FloatingPaneKind::Ttrpg),
    );
    sync_pane_card_and_header(
        spec_for_pane(FloatingPaneKind::Webcam),
        webcam_collapsed,
        webcam_floating,
        webcam_pos,
        z_index_for_pane(&z_order, FloatingPaneKind::Webcam),
    );

    if let Some(col) =
        get_element("webrtc-dock-column").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        col.set_class_name(layout.webrtc_col_class);
        let _ = col
            .style()
            .set_property("display", layout.webrtc_col_display);
    }
    if let Some(rails_dock) =
        get_element("right-drawer-rails").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = rails_dock
            .style()
            .set_property("display", layout.rails_display);
    }
    if let Some(center_col) =
        get_element("center-column").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        center_col.set_class_name(layout.center_col_class);
        let _ = center_col
            .style()
            .set_property("display", layout.center_col_display);
    }
    if let Some(ws_panel) = get_element("workspace-panel") {
        ws_panel.set_class_name(&layout.workspace_classes);
    }

    for (kind, rail_collapsed, floating) in [
        (FloatingPaneKind::Webcam, webcam_collapsed, webcam_floating),
        (FloatingPaneKind::Chat, chat_collapsed, chat_floating),
        (
            FloatingPaneKind::Ttrpg,
            ttrpg_collapsed && !ttrpg_hidden_by_mode,
            ttrpg_floating && !ttrpg_hidden_by_mode,
        ),
    ] {
        let spec = spec_for_pane(kind);
        sync_drawer_rail_and_aria(rail_collapsed, spec);
        sync_float_button(spec, floating);
    }
    if ttrpg_hidden_by_mode
        && let Some(rail) =
            get_element("ttrpg-drawer-rail").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = rail.style().set_property("display", "none");
    }

    refresh_active_notebook_stage_on_layout_change();
}

fn toggle_pane_drawer(kind: FloatingPaneKind) {
    with_state_mut(|st| {
        st.active_pane_drag = None;
        let (collapsed, floating) = match kind {
            FloatingPaneKind::Chat => {
                st.chat_collapsed = !st.chat_collapsed;
                (st.chat_collapsed, st.chat_floating)
            }
            FloatingPaneKind::Ttrpg => {
                st.ttrpg_collapsed = !st.ttrpg_collapsed;
                (st.ttrpg_collapsed, st.ttrpg_floating)
            }
            FloatingPaneKind::Webcam => {
                st.webcam_collapsed = !st.webcam_collapsed;
                (st.webcam_collapsed, st.webcam_floating)
            }
        };
        if !collapsed && floating {
            bring_floating_pane_to_front(&mut st.floating_z_order, kind);
        }
    });
    sync_workspace_drawers_dom();
}

fn toggle_pane_float(kind: FloatingPaneKind) {
    with_state_mut(|st| {
        st.active_pane_drag = None;
        let now_floating = match kind {
            FloatingPaneKind::Chat => {
                st.chat_floating = !st.chat_floating;
                st.chat_collapsed = false;
                if !st.chat_floating {
                    st.chat_float_pos_px = None;
                }
                st.chat_floating
            }
            FloatingPaneKind::Ttrpg => {
                st.ttrpg_floating = !st.ttrpg_floating;
                st.ttrpg_collapsed = false;
                if !st.ttrpg_floating {
                    st.ttrpg_float_pos_px = None;
                }
                st.ttrpg_floating
            }
            FloatingPaneKind::Webcam => {
                st.webcam_floating = !st.webcam_floating;
                st.webcam_collapsed = false;
                if !st.webcam_floating {
                    st.webcam_float_pos_px = None;
                }
                st.webcam_floating
            }
        };
        if now_floating {
            bring_floating_pane_to_front(&mut st.floating_z_order, kind);
        }
    });
    sync_workspace_drawers_dom();
}

/// Toggles Web Cams (`#webrtc-dock-column`) between expanded and right-edge rail (`#webcam-drawer-rail`).
#[wasm_bindgen]
pub fn satya_toggle_webcam_drawer() {
    toggle_pane_drawer(FloatingPaneKind::Webcam);
}

/// Toggles Session Chat (`#chat-card`) between expanded (docked or floating) and `#chat-drawer-rail`.
#[wasm_bindgen]
pub fn satya_toggle_chat_drawer() {
    toggle_pane_drawer(FloatingPaneKind::Chat);
}

/// Toggles Task Resolution & Action Pool (`#ttrpg-drawer`) between expanded (docked or floating) and `#ttrpg-drawer-rail`.
#[wasm_bindgen]
pub fn satya_toggle_ttrpg_drawer() {
    toggle_pane_drawer(FloatingPaneKind::Ttrpg);
}

/// Toggles Web Cams (`#webrtc-card`) between docked column panel and floating draggable overlay.
#[wasm_bindgen]
pub fn satya_toggle_webcam_float() {
    toggle_pane_float(FloatingPaneKind::Webcam);
}

/// Toggles Session Chat (`#chat-card`) between docked column panel and floating draggable overlay.
#[wasm_bindgen]
pub fn satya_toggle_chat_float() {
    toggle_pane_float(FloatingPaneKind::Chat);
}

/// Toggles Task Resolution (`#ttrpg-drawer`) between docked column panel and floating draggable overlay.
#[wasm_bindgen]
pub fn satya_toggle_ttrpg_float() {
    toggle_pane_float(FloatingPaneKind::Ttrpg);
}

/// Toggles `#video-grid` between 1 column (`cols-1`) and 2 columns (`cols-2`), strictly capped at 2 columns.
#[wasm_bindgen]
pub fn satya_toggle_webcam_columns() {
    let cols = with_state_mut(|st| {
        st.webcam_columns = if st.webcam_columns == 1 { 2 } else { 1 };
        st.webcam_columns
    });

    set_class(
        "video-grid",
        if cols == 2 {
            "video-grid cols-2"
        } else {
            "video-grid cols-1"
        },
    );
    set_text(
        "btn-toggle-webcam-cols",
        if cols == 2 { "1 Col" } else { "2 Cols" },
    );
}

fn is_pane_floating_and_visible(kind: FloatingPaneKind) -> bool {
    with_state(|st| match kind {
        FloatingPaneKind::Chat => st.chat_floating && !st.chat_collapsed,
        FloatingPaneKind::Ttrpg => {
            st.session_mode != SessionMode::Meeting && st.ttrpg_floating && !st.ttrpg_collapsed
        }
        FloatingPaneKind::Webcam => st.webcam_floating && !st.webcam_collapsed,
    })
}

fn is_interactive_drag_target(ev: &web_sys::Event) -> bool {
    ev.target()
        .and_then(|t| t.dyn_into::<Element>().ok())
        .is_some_and(|el| {
            el.closest("button, input, select, textarea, a, label")
                .ok()
                .flatten()
                .is_some()
        })
}

fn extract_pointer_xy(ev: &web_sys::Event) -> Option<(f64, f64)> {
    let cx = Reflect::get(ev, &JsValue::from_str("clientX"))
        .ok()
        .and_then(|v| v.as_f64())?;
    let cy = Reflect::get(ev, &JsValue::from_str("clientY"))
        .ok()
        .and_then(|v| v.as_f64())?;
    Some((cx, cy))
}

fn viewport_size_px() -> (f64, f64) {
    if let Some(win) = window()
        && let (Ok(w_val), Ok(h_val)) = (win.inner_width(), win.inner_height())
        && let (Some(w), Some(h)) = (w_val.as_f64(), h_val.as_f64())
    {
        return (w, h);
    }
    if let Some(root) = document().and_then(|d| d.document_element()) {
        return (
            f64::from(root.client_width().max(320)),
            f64::from(root.client_height().max(240)),
        );
    }
    (1920.0, 1080.0)
}

fn move_floating_pane_clamped(
    kind: FloatingPaneKind,
    card: &HtmlElement,
    raw_left_px: f64,
    raw_top_px: f64,
    bring_to_front: bool,
) {
    let rect = card.get_bounding_client_rect();
    let (vw, vh) = viewport_size_px();
    let (new_left, new_top) = clamp_floating_pane_position(
        raw_left_px,
        raw_top_px,
        rect.width(),
        rect.height(),
        vw,
        vh,
    );
    apply_floating_card_xy(card, new_left, new_top);
    with_state_mut(|st| {
        if bring_to_front {
            bring_floating_pane_to_front(&mut st.floating_z_order, kind);
        }
        match kind {
            FloatingPaneKind::Chat => st.chat_float_pos_px = Some((new_left, new_top)),
            FloatingPaneKind::Ttrpg => st.ttrpg_float_pos_px = Some((new_left, new_top)),
            FloatingPaneKind::Webcam => st.webcam_float_pos_px = Some((new_left, new_top)),
        }
    });
}

fn start_floating_pane_drag(kind: FloatingPaneKind, ev: web_sys::Event) {
    if !is_pane_floating_and_visible(kind) || is_interactive_drag_target(&ev) {
        return;
    }
    if let Some(me) = ev.dyn_ref::<MouseEvent>()
        && me.button() != 0
    {
        return;
    }
    let Some((client_x, client_y)) = extract_pointer_xy(&ev) else {
        return;
    };
    let spec = spec_for_pane(kind);
    let Some(card) = get_element(spec.card_id).and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };

    ev.prevent_default();
    let rect = card.get_bounding_client_rect();
    if let Some(header) = get_element(spec.drag_header_id) {
        header.set_class_name(&format!("{} is-dragging", spec.drag_header_base_class));
    }
    with_state_mut(|st| {
        bring_floating_pane_to_front(&mut st.floating_z_order, kind);
        st.active_pane_drag = Some((kind, client_x - rect.left(), client_y - rect.top()));
    });
    sync_floating_z_indices();
}

/// Begins dragging the floating Web Cams frame (`#webrtc-card.is-floating`).
#[wasm_bindgen]
pub fn satya_start_webcam_drag(ev: web_sys::Event) {
    start_floating_pane_drag(FloatingPaneKind::Webcam, ev);
}

/// Begins dragging the floating Session Chat frame (`#chat-card.is-floating`).
#[wasm_bindgen]
pub fn satya_start_chat_drag(ev: web_sys::Event) {
    start_floating_pane_drag(FloatingPaneKind::Chat, ev);
}

/// Begins dragging the floating Task Resolution frame (`#ttrpg-drawer.is-floating`).
#[wasm_bindgen]
pub fn satya_start_ttrpg_drag(ev: web_sys::Event) {
    start_floating_pane_drag(FloatingPaneKind::Ttrpg, ev);
}

/// Moves a focused floating pane header cleanly within viewport bounds via Arrow keys (`Shift` = larger step).
#[wasm_bindgen]
pub fn satya_floating_header_keydown(pane_slug: String, ev: web_sys::KeyboardEvent) {
    let Some(kind) = FloatingPaneKind::from_slug(&pane_slug) else {
        return;
    };
    if !is_pane_floating_and_visible(kind) || is_interactive_drag_target(ev.as_ref()) {
        return;
    }
    let step_px = if ev.shift_key() { 48.0 } else { 20.0 };
    let (dx, dy) = match ev.key().as_str() {
        "ArrowLeft" => (-step_px, 0.0),
        "ArrowRight" => (step_px, 0.0),
        "ArrowUp" => (0.0, -step_px),
        "ArrowDown" => (0.0, step_px),
        _ => return,
    };

    ev.prevent_default();
    let Some(card) =
        get_element(spec_for_pane(kind).card_id).and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let rect = card.get_bounding_client_rect();
    move_floating_pane_clamped(kind, &card, rect.left() + dx, rect.top() + dy, true);
    sync_floating_z_indices();
}

/// Restores a previously saved `#notebook-column` width percentage (`22.0..=72.0`) from `localStorage`.
pub fn restore_saved_column_split() {
    if let Some(raw) = local_storage_get(STORAGE_KEY_NOTEBOOK_WIDTH_PCT)
        && let Ok(pct) = raw.trim().parse::<f64>()
    {
        apply_column_split_pct(pct.clamp(22.0, 72.0));
    }
}

fn apply_column_split_pct(pct: f64) {
    if let Some(ws) = get_element("workspace-panel").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = ws
            .style()
            .set_property("--col-left-track", &format!("minmax(320px, {pct:.1}%)"));
        let _ = ws
            .style()
            .set_property("--col-center-track", "minmax(300px, 1fr)");
    }
}

/// Begins resizing the division between `#notebook-column` and `#center-column` via `#workspace-col-splitter`.
#[wasm_bindgen]
pub fn satya_start_column_resize(ev: web_sys::Event) {
    ev.prevent_default();
    with_state_mut(|st| {
        st.column_resizing = true;
    });
    if let Some(splitter) = get_element("workspace-col-splitter") {
        splitter.set_class_name("workspace-col-splitter is-dragging");
    }
}

/// Resets `#notebook-column` vs `#center-column` split back to the responsive CSS defaults.
#[wasm_bindgen]
pub fn satya_reset_column_resize() {
    with_state_mut(|st| {
        st.column_resizing = false;
    });
    local_storage_set(STORAGE_KEY_NOTEBOOK_WIDTH_PCT, "");
    if let Some(ws) = get_element("workspace-panel").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        let _ = ws.style().remove_property("--col-left-track");
        let _ = ws.style().remove_property("--col-center-track");
    }
    if let Some(splitter) = get_element("workspace-col-splitter") {
        splitter.set_class_name("workspace-col-splitter");
    }
    refresh_active_notebook_stage_on_layout_change();
}

pub(crate) fn handle_column_resize_move(ev: &web_sys::Event) {
    if !with_state(|st| st.column_resizing) {
        return;
    }
    let Some((client_x, _)) = extract_pointer_xy(ev) else {
        return;
    };
    let Some(ws) = get_element("workspace-panel").and_then(|e| e.dyn_into::<HtmlElement>().ok())
    else {
        return;
    };
    let total_w = ws.offset_width() as f64;
    if total_w < 640.0 {
        return;
    }
    let rel_x = (client_x - (ws.offset_left() as f64)).max(320.0);
    let pct = ((rel_x / total_w) * 100.0).clamp(22.0, 72.0);
    apply_column_split_pct(pct);
    local_storage_set(STORAGE_KEY_NOTEBOOK_WIDTH_PCT, &format!("{pct:.1}"));
    refresh_active_notebook_stage_on_layout_change();
}

pub(crate) fn handle_floating_pane_drag_move(ev: &web_sys::Event) {
    let Some((kind, off_x, off_y)) = with_state(|st| st.active_pane_drag) else {
        return;
    };
    if !is_pane_floating_and_visible(kind) {
        return;
    }
    let Some((client_x, client_y)) = extract_pointer_xy(ev) else {
        return;
    };
    if let Some(card) =
        get_element(spec_for_pane(kind).card_id).and_then(|e| e.dyn_into::<HtmlElement>().ok())
    {
        move_floating_pane_clamped(kind, &card, client_x - off_x, client_y - off_y, false);
    }
}

pub(crate) fn end_workspace_pointer_interactions() {
    let (drag_kind, was_resizing) = with_state_mut(|st| {
        let dragged = st.active_pane_drag.take().map(|(k, _, _)| k);
        let resizing = st.column_resizing;
        st.column_resizing = false;
        (dragged, resizing)
    });
    if let Some(kind) = drag_kind {
        let spec = spec_for_pane(kind);
        if let Some(header) = get_element(spec.drag_header_id) {
            header.set_class_name(spec.drag_header_base_class);
        }
    }
    if was_resizing && let Some(splitter) = get_element("workspace-col-splitter") {
        splitter.set_class_name("workspace-col-splitter");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Mode {
        Docked,
        Floating,
        Collapsed,
    }

    impl Mode {
        fn flags(self) -> (bool, bool) {
            match self {
                Self::Docked => (false, false),
                Self::Floating => (true, false),
                Self::Collapsed => (false, true),
            }
        }
    }

    #[test]
    fn test_all_27_workspace_drawer_and_float_layout_states() {
        let modes = [Mode::Docked, Mode::Floating, Mode::Collapsed];
        let mut count = 0;
        for chat in modes {
            for ttrpg in modes {
                for webcam in modes {
                    count += 1;
                    let (chat_float, chat_coll) = chat.flags();
                    let (ttrpg_float, ttrpg_coll) = ttrpg.flags();
                    let (cam_float, cam_coll) = webcam.flags();
                    let spec = compute_workspace_layout_classes(
                        chat_float, chat_coll, ttrpg_float, ttrpg_coll, cam_float, cam_coll, false,
                    );
                    let center_docked = chat == Mode::Docked || ttrpg == Mode::Docked;
                    let any_collapsed =
                        chat == Mode::Collapsed || ttrpg == Mode::Collapsed || webcam == Mode::Collapsed;

                    assert_eq!(spec.rails_display, if any_collapsed { "flex" } else { "none" });
                    if !center_docked {
                        assert!(spec.workspace_classes.contains("center-collapsed"));
                        if chat == Mode::Floating || ttrpg == Mode::Floating {
                            assert_eq!(spec.center_col_display, "contents");
                            assert_eq!(spec.center_col_class, "col-center is-undocked");
                        } else {
                            assert_eq!(spec.center_col_display, "none");
                            assert_eq!(spec.center_col_class, "col-center is-collapsed");
                        }
                    } else {
                        assert_eq!(spec.center_col_display, "flex");
                        assert!(!spec.workspace_classes.contains("center-collapsed"));
                    }
                    assert_eq!(
                        spec.webrtc_col_display,
                        if webcam == Mode::Floating && !any_collapsed { "contents" } else { "flex" }
                    );
                }
            }
        }
        assert_eq!(count, 27);

        let all_coll = compute_workspace_layout_classes(false, true, false, true, false, true, false);
        assert_eq!(all_coll.workspace_classes, "workspace-columns webcam-collapsed center-collapsed");
        assert_eq!(all_coll.center_col_display, "none");

        let all_float = compute_workspace_layout_classes(true, false, true, false, true, false, false);
        assert_eq!(all_float.workspace_classes, "workspace-columns webcam-undocked center-collapsed");
        assert_eq!((all_float.center_col_display, all_float.webrtc_col_display), ("contents", "contents"));

        let meeting_coll = compute_workspace_layout_classes(false, true, false, false, false, true, true);
        assert_eq!(meeting_coll.workspace_classes, "workspace-columns webcam-collapsed center-collapsed");
        let meeting_dock = compute_workspace_layout_classes(false, false, false, false, false, false, true);
        assert_eq!(meeting_dock.workspace_classes, "workspace-columns ttrpg-collapsed");
        assert_eq!(meeting_dock.rails_display, "none");
    }

    #[test]
    fn test_clamp_floating_pane_position_and_z_order() {
        assert_eq!(clamp_floating_pane_position(-50.0, -20.0, 420.0, 500.0, 1440.0, 900.0), (8.0, 8.0));
        assert_eq!(clamp_floating_pane_position(2000.0, 1200.0, 420.0, 500.0, 1440.0, 900.0), (1180.0, 844.0));
        assert_eq!(clamp_floating_pane_position(300.0, 200.0, 420.0, 500.0, 120.0, 40.0), (8.0, 8.0));

        let mut order = [FloatingPaneKind::Chat, FloatingPaneKind::Ttrpg, FloatingPaneKind::Webcam];
        bring_floating_pane_to_front(&mut order, FloatingPaneKind::Chat);
        assert_eq!(order, [FloatingPaneKind::Ttrpg, FloatingPaneKind::Webcam, FloatingPaneKind::Chat]);
        assert_eq!(z_index_for_pane(&order, FloatingPaneKind::Chat), 97);
        assert_eq!(z_index_for_pane(&order, FloatingPaneKind::Ttrpg), 95);
    }
}
