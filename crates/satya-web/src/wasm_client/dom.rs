//! Typed `web-sys` DOM query/mutation helpers, JSON HTTP `fetch` wrapper, and inline SVG constants.

use serde::Serialize;
use serde::de::DeserializeOwned;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    Document, Element, Headers, HtmlButtonElement, HtmlElement, HtmlInputElement,
    HtmlSelectElement, HtmlTextAreaElement, Request, RequestInit, Response, Window,
};

/// SVG icon markup for the "show password" (open eye) toggle state.
pub const EYE_OPEN_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>"#;

/// SVG icon markup for the "hide password" (slashed eye) toggle state.
pub const EYE_OFF_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>"#;

/// SVG icon markup for the top-bar hamburger menu button.
pub const HAMBURGER_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/></svg>"#;

/// SVG icon markup for the `IN_CHARACTER` chat input toggle button (character speech bubble).
pub const CTX_IC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15a2 2 0 0 1-2 2H7l-4 4V5a2 2 0 0 1 2-2h14a2 2 0 0 1 2 2z"/><path d="M8 9h8"/><path d="M8 13h5"/></svg>"#;

/// SVG icon markup for the `OUT_OF_CHARACTER` chat input toggle button (brackets / OOC talk).
pub const CTX_OOC_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>"#;

/// SVG icon markup for the `GM_NARRATION` chat input toggle button (open book / tome).
pub const CTX_GM_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z"/><path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z"/></svg>"#;

pub(crate) fn window() -> Option<Window> {
    web_sys::window()
}

pub(crate) fn document() -> Option<Document> {
    window()?.document()
}

pub(crate) fn get_element(id: &str) -> Option<Element> {
    document()?.get_element_by_id(id)
}

pub(crate) fn input_value(id: &str) -> String {
    let Some(el) = get_element(id) else {
        return String::new();
    };
    if let Ok(inp) = el.clone().dyn_into::<HtmlInputElement>() {
        return inp.value();
    }
    if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
        return ta.value();
    }
    js_sys::Reflect::get(&el, &JsValue::from_str("value"))
        .ok()
        .and_then(|v| v.as_string())
        .unwrap_or_default()
}

pub(crate) fn set_input_value(id: &str, val: &str) {
    let Some(el) = get_element(id) else {
        return;
    };
    if let Ok(inp) = el.clone().dyn_into::<HtmlInputElement>() {
        inp.set_value(val);
    } else if let Ok(ta) = el.clone().dyn_into::<HtmlTextAreaElement>() {
        ta.set_value(val);
    } else {
        let _ = js_sys::Reflect::set(&el, &JsValue::from_str("value"), &JsValue::from_str(val));
    }
}

pub(crate) fn set_input_checked(id: &str, checked: bool) {
    if let Some(inp) = get_element(id).and_then(|el| el.dyn_into::<HtmlInputElement>().ok()) {
        inp.set_checked(checked);
    }
}

pub(crate) fn input_checked(id: &str) -> bool {
    get_element(id)
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|inp| inp.checked())
        .unwrap_or(false)
}

pub(crate) fn select_value(id: &str) -> String {
    get_element(id)
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|sel| sel.value())
        .unwrap_or_default()
}

pub(crate) fn set_select_value(id: &str, val: &str) {
    if let Some(sel) = get_element(id).and_then(|el| el.dyn_into::<HtmlSelectElement>().ok()) {
        sel.set_value(val);
    }
}

pub(crate) fn set_text(id: &str, text: &str) {
    if let Some(el) = get_element(id) {
        el.set_text_content(Some(text));
    }
}

pub(crate) fn set_html(id: &str, html: &str) {
    if let Some(el) = get_element(id) {
        el.set_inner_html(html);
    }
}

pub(crate) fn set_class(id: &str, class_name: &str) {
    if let Some(el) = get_element(id) {
        el.set_class_name(class_name);
    }
}

pub(crate) fn set_button_disabled(id: &str, disabled: bool) {
    if let Some(btn) = get_element(id).and_then(|el| el.dyn_into::<HtmlButtonElement>().ok()) {
        btn.set_disabled(disabled);
    }
}

pub(crate) fn set_display(id: &str, display: &str) {
    if let Some(el) = get_element(id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        let _ = el.style().set_property("display", display);
    }
}

/// Toggles `#id`'s inline `style.display` between `open_display` (when currently `"none"`) and `"none"`,
/// returning `Some(true)` when opened, `Some(false)` when closed, or `None` if the element is absent.
pub(crate) fn toggle_element_display(id: &str, open_display: &str) -> Option<bool> {
    let el = get_element(id).and_then(|e| e.dyn_into::<HtmlElement>().ok())?;
    let is_opening = el.style().get_property_value("display").unwrap_or_default() == "none";
    let _ = el
        .style()
        .set_property("display", if is_opening { open_display } else { "none" });
    Some(is_opening)
}

/// Rolls a uniform `1..=sides` die using browser `Math.random()`.
pub(crate) fn roll_browser_die_i32(sides: i32) -> i32 {
    let sides = sides.max(1);
    ((js_sys::Math::random() * (sides as f64)).floor() as i32 + 1).clamp(1, sides)
}

/// Rolls a uniform `1..=sides` die as `i64` using browser `Math.random()`.
pub(crate) fn roll_browser_die_i64(sides: i64) -> i64 {
    i64::from(roll_browser_die_i32(sides as i32))
}

/// Updates a status feedback element (`#auth-feedback` or `#session-feedback`) with styled text.
pub fn set_feedback(id: &str, text: &str, is_error: bool) {
    if let Some(el) = get_element(id).and_then(|e| e.dyn_into::<HtmlElement>().ok()) {
        el.set_text_content(Some(text));
        let color = if is_error { "#f43f5e" } else { "#38bdf8" };
        let _ = el.style().set_property("color", color);
    }
}

/// Escapes `&`, `<`, and `>` characters in `raw` for safe HTML text interpolation.
pub fn escape_html(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

pub(crate) async fn fetch_json<Req: Serialize, Resp: DeserializeOwned>(
    method: &str,
    url: &str,
    auth_token: Option<&str>,
    body: Option<&Req>,
) -> Result<Resp, String> {
    let win = window().ok_or_else(|| "No browser Window available".to_string())?;
    let opts = RequestInit::new();
    opts.set_method(method);

    let headers = Headers::new().map_err(|_| "Failed to construct Headers".to_string())?;
    let _ = headers.set("Content-Type", "application/json");
    if let Some(tok) = auth_token {
        let _ = headers.set("Authorization", &format!("Bearer {tok}"));
    }
    opts.set_headers(&headers);

    if let Some(payload) = body {
        let json_str = serde_json::to_string(payload).map_err(|e| e.to_string())?;
        opts.set_body(&JsValue::from_str(&json_str));
    }

    let request =
        Request::new_with_str_and_init(url, &opts).map_err(|_| "Invalid Request".to_string())?;
    let resp_val = JsFuture::from(win.fetch_with_request(&request))
        .await
        .map_err(|_| "Network fetch failed".to_string())?;
    let resp: Response = resp_val
        .dyn_into()
        .map_err(|_| "Invalid Response cast".to_string())?;

    let text_promise = resp
        .text()
        .map_err(|_| "Failed to read response text".to_string())?;
    let text_val = JsFuture::from(text_promise)
        .await
        .map_err(|_| "Failed awaiting response text".to_string())?;
    let text = text_val.as_string().unwrap_or_default();

    if !resp.ok() {
        if let Ok(err_val) = serde_json::from_str::<serde_json::Value>(&text)
            && let Some(msg) = err_val.get("error").and_then(|v| v.as_str())
        {
            return Err(msg.to_string());
        }
        return Err(format!("HTTP {}: {}", resp.status(), text));
    }

    serde_json::from_str::<Resp>(&text).map_err(|e| format!("JSON decode error: {e}"))
}
