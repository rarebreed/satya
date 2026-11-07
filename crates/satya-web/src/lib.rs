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

/// Minimal 4-line ES module loader that boots the compiled `satya-web` Rust/Wasm module
/// (`/pkg/satya_web.js` + `/pkg/satya_web_bg.wasm`), invoking `#[wasm_bindgen(start)]`.
const WASM_BOOTSTRAP_LOADER: &str = r#"
import init from "/pkg/satya_web.js";
init().catch((err) => console.warn("Satya Wasm module init:", err));
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
        <html lang="en">
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
