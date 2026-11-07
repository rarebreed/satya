//! Apache Iggy Topic & Avro Schema Inspector (`#admin-inspector-card` inside the `iggy` Notebook tab).

use crate::wasm_client::chat_and_ttrpg::append_system_notice;
use crate::wasm_client::dom::{escape_html, fetch_json, get_element};
use crate::wasm_client::state::with_state;
use satya_schema::{AccountRole, StorageMetricsViewResponse, TopicMetricsView};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

fn current_admin_token() -> Option<String> {
    with_state(|st| {
        if st.user.as_ref().map(|u| u.account_role) == Some(AccountRole::Admin) {
            st.auth_token.clone()
        } else {
            None
        }
    })
}

/// Fetches live Apache Iggy topic metrics from `GET /api/storage/metrics` and updates `#topic-metrics-table`
/// (only executed when the currently authenticated user has [`AccountRole::Admin`]).
#[wasm_bindgen]
pub fn satya_refresh_metrics() {
    spawn_local(async move {
        let Some(tok) = current_admin_token() else {
            return;
        };
        if let Ok(resp) = fetch_json::<(), StorageMetricsViewResponse>(
            "GET",
            "/api/storage/metrics",
            Some(&tok),
            None,
        )
        .await
        {
            render_metrics_table(&resp.topics);
        }
    });
}

/// Triggers an immediate flush of all buffered topic events into `.avro` container batches via `POST /api/storage/flush`
/// (requires an authenticated [`AccountRole::Admin`] session).
#[wasm_bindgen]
pub fn satya_flush_batches() {
    spawn_local(async move {
        let Some(tok) = current_admin_token() else {
            return;
        };
        if let Ok(val) =
            fetch_json::<(), serde_json::Value>("POST", "/api/storage/flush", Some(&tok), None)
                .await
        {
            let count = val
                .get("flushed_files")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .unwrap_or(0);
            append_system_notice(&format!(
                "Flushed {count} offset-tracked Avro container batch(es) for PyLance ingestion."
            ));
            satya_refresh_metrics();
        }
    });
}

/// Renders an HTML `<table>` string summarizing per-topic Apache Iggy and Avro schema metrics.
pub fn format_metrics_table_html(topics: &[TopicMetricsView]) -> String {
    let rows: String = topics
        .iter()
        .map(|t| {
            let fp_short: String = t.active_schema_fingerprint_hex.chars().take(10).collect();
            let last_off = t
                .last_flushed_offset
                .map(|o| o.to_string())
                .unwrap_or_else(|| "-".to_string());
            format!(
                r#"<tr>
                    <td><strong>{}</strong></td>
                    <td><span class="mono">v{} ({fp_short}...)</span></td>
                    <td>{} (<span class="mono">{}</span>)</td>
                    <td>{}</td>
                    <td>{}</td>
                    <td>{} (last offset: {last_off})</td>
                </tr>"#,
                escape_html(&t.topic),
                t.active_schema_version,
                escape_html(&t.storage_backend.to_uppercase()),
                escape_html(&t.destination_uri),
                t.total_messages_ingested,
                t.buffered_messages,
                t.flushed_batches_count,
            )
        })
        .collect();

    format!(
        r#"<table class="metrics-table">
            <thead>
                <tr>
                    <th>Apache Iggy Topic</th>
                    <th>Schema Ver &amp; CRC-64</th>
                    <th>Configurable Storage Sink</th>
                    <th>Ingested Events</th>
                    <th>Buffered</th>
                    <th>Flushed .avro Batches</th>
                </tr>
            </thead>
            <tbody>{rows}</tbody>
        </table>"#
    )
}

pub(crate) fn render_metrics_table(topics: &[TopicMetricsView]) {
    if let Some(wrap) = get_element("topic-metrics-table") {
        wrap.set_inner_html(&format_metrics_table_html(topics));
    }
}
