//! Full-stack Rust + Leptos + WebAssembly frontend for the Satya WebRTC, TTRPG,
//! and Apache Iggy Event Streaming Platform.
//!
//! ## Module Map
//! - [`chat_cmd`]: Chat slash-command parser (`/msg`, `/roll`, `/r`) and dice pipeline evaluator.
//! - [`views_gate`]: Top navigation bar (`HeaderBar`) and pre-session auth/session gate (`AuthAndSessionGate`).
//! - [`views_notebook`]: Left-column Tabbed Tabletop Notebook (`NotebookColumn`) with 9 interactive panes.
//! - [`views_workspace`]: Active 3-column session workspace (`ActiveSessionWorkspace`) combining Notebook, Chat/Task Roller, and WebRTC Web Cams.
//! - [`wasm_client`]: Browser `wasm-bindgen` / `web-sys` / `wgpu` client runtime and `window.satya*` bindings.

pub mod chat_cmd;
pub mod views_gate;
pub mod views_notebook;
pub mod views_notebook_docs;
pub mod views_workspace;
pub mod wasm_client;

#[cfg(test)]
mod tests;

pub use views_gate::*;
pub use views_notebook::*;
pub use views_workspace::*;

use leptos::prelude::*;

/// Minimal ES module loader that restores saved appearance attributes from `localStorage`,
/// provides instant pre-hydration `window.satya*Appearance*` bindings, and boots `/pkg/satya_web.js`.
const WASM_BOOTSTRAP_LOADER: &str = r#"
const SATYA_APPEARANCE_SPEC = [
    ["theme", "data-theme", "menu-theme-select", "obsidian"],
    ["density", "data-density", "menu-density-select", "default"],
    ["corners", "data-corners", "menu-corners-select", "default"],
    ["font", "data-font", "menu-font-select", "sans"],
];
function satyaSyncAppearance(save) {
    const r = document.documentElement;
    for (const [k, attr, selId, defVal] of SATYA_APPEARANCE_SPEC) {
        const el = document.getElementById(selId);
        let val = save && el ? el.value : null;
        if (!val) {
            try { val = localStorage.getItem("satya.appearance." + k); } catch (_) {}
        }
        val = val || defVal;
        r.setAttribute(attr, val);
        if (el) el.value = val;
        if (save) {
            try { localStorage.setItem("satya.appearance." + k, val); } catch (_) {}
        }
    }
    const activeTheme = r.getAttribute("data-theme") || "obsidian";
    for (const btn of document.querySelectorAll(".theme-swatch-btn")) {
        const isAct = btn.id === "btn-theme-swatch-" + activeTheme;
        btn.classList.toggle("active", isAct);
        btn.setAttribute("aria-pressed", isAct ? "true" : "false");
    }
}
window.satyaToggleAppearanceSettings = () => {
    const p = document.getElementById("menu-appearance-settings-panel");
    if (p) p.style.display = p.style.display === "none" ? "flex" : "none";
};
window.satyaApplyAppearanceSettings = () => satyaSyncAppearance(true);
window.satyaSelectThemePreset = (t) => {
    const el = document.getElementById("menu-theme-select");
    if (el) el.value = t;
    satyaSyncAppearance(true);
};
window.satyaResetAppearanceSettings = () => {
    for (const [, , selId, defVal] of SATYA_APPEARANCE_SPEC) {
        const el = document.getElementById(selId);
        if (el) el.value = defVal;
    }
    satyaSyncAppearance(true);
};
satyaSyncAppearance(false);
import init from "/pkg/satya_web.js";
init().then(() => satyaSyncAppearance(false)).catch((err) => console.warn("Satya Wasm module init:", err));
"#;

/// Renders the full-stack Leptos application shell for Satya WebRTC, Chat & Avro Telemetry.
pub fn render_satya_app_html() -> String {
    SatyaApp().to_html()
}

/// Root Leptos component rendering the Satya WebRTC, Chat, TTRPG Mechanics, and Avro Inspector UI.
#[component]
pub fn SatyaApp() -> impl IntoView {
    let app_styles = concat!(
        include_str!("styles.css"),
        "\n",
        include_str!("styles_notebook.css")
    );

    view! {
        <!DOCTYPE html>
        <html
            lang="en"
            data-theme="obsidian"
            data-density="default"
            data-corners="default"
            data-font="sans"
        >
            <head>
                <meta charset="utf-8"/>
                <meta name="viewport" content="width=device-width, initial-scale=1"/>
                <title>"SATYA // WebRTC, TTRPG & Meeting Event Streaming Platform"</title>
                <style inner_html=app_styles></style>
            </head>
            <body>
                <div class="app-shell">
                    <HeaderBar/>
                    <main class="workspace-grid">
                        <AuthAndSessionGate/>
                        <ActiveSessionWorkspace/>
                    </main>
                </div>
                <script type="module" inner_html=WASM_BOOTSTRAP_LOADER></script>
            </body>
        </html>
    }
}
