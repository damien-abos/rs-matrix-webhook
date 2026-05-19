mod auth;
mod config;
mod formatters;
mod markdown;
mod matrix;

use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Settings;
use formatters::FormatterRegistry;
use matrix::MatrixClient;

// ---------------------------------------------------------------------------
// Shared application state (cheap to clone — lives behind Arc)
// ---------------------------------------------------------------------------

struct AppState {
    settings: Settings,
    matrix: MatrixClient,
    formatters: FormatterRegistry,
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::load()?;

    init_tracing(settings.verbosity);

    let matrix = MatrixClient::new(&settings).await?;
    let formatters = FormatterRegistry::new(&settings)?;

    tracing::info!("Formatters available: {}", formatters.names().join(", "));

    let state = Arc::new(AppState {
        settings,
        matrix,
        formatters,
    });

    let app = Router::new()
        .route("/health", get(health))
        .route("/", post(webhook_no_path))
        .route("/{room_id}", post(webhook_with_path))
        .with_state(state.clone());

    // ── Unix socket (Linux/macOS) ──────────────────────────────────────────
    #[cfg(unix)]
    if let Some(ref socket_path) = state.settings.server_path {
        use tokio::net::UnixListener;
        let _ = std::fs::remove_file(socket_path); // clean up stale socket
        let listener = UnixListener::bind(socket_path)?;
        tracing::info!("Listening on Unix socket {}", socket_path);
        axum::serve(listener, app).await?;
        return Ok(());
    }

    // ── TCP ───────────────────────────────────────────────────────────────
    let host = if state.settings.host.is_empty() {
        "0.0.0.0"
    } else {
        &state.settings.host
    };
    let addr = format!("{}:{}", host, state.settings.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Listening on {}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing(verbosity: u8) {
    let level = match verbosity {
        0 => "error",
        1 => "warn",
        2 => "info",
        3 => "debug",
        _ => "trace",
    };
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| level.to_string());

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(filter))
        .with(tracing_subscriber::fmt::layer())
        .init();
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

async fn health() -> impl IntoResponse {
    StatusCode::OK
}

async fn webhook_no_path(
    State(state): State<Arc<AppState>>,
    Query(query): Query<WebhookQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    handle_webhook(state, None, query, headers, body).await
}

async fn webhook_with_path(
    State(state): State<Arc<AppState>>,
    Path(room_id): Path<String>,
    Query(query): Query<WebhookQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Response {
    handle_webhook(state, Some(room_id), query, headers, body).await
}

#[derive(Deserialize)]
struct WebhookQuery {
    room_id: Option<String>,
    key: Option<String>,
    formatter: Option<String>,
}

// ---------------------------------------------------------------------------
// Core webhook logic
// ---------------------------------------------------------------------------

async fn handle_webhook(
    state: Arc<AppState>,
    path_room_id: Option<String>,
    query: WebhookQuery,
    http_headers: HeaderMap,
    raw_body: axum::body::Bytes,
) -> Response {
    // ── 1. Parse JSON ──────────────────────────────────────────────────────
    let mut data: Value = match serde_json::from_slice(&raw_body) {
        Ok(v) => v,
        Err(_) => return err(StatusCode::BAD_REQUEST, "Invalid JSON body"),
    };

    // ── 2. Authenticate ────────────────────────────────────────────────────
    let key_in_body = data["key"].as_str().map(String::from);
    let digest_in_body = data["digest"].as_str().map(String::from);

    let authenticated = if let Some(k) = key_in_body.as_deref().or(query.key.as_deref()) {
        k == state.settings.api_key
    } else if let Some(ref digest) = digest_in_body {
        auth::verify_hmac(&raw_body, &state.settings.api_key, digest)
    } else {
        false
    };

    if !authenticated {
        return err(StatusCode::UNAUTHORIZED, "Unauthorized");
    }

    // Strip auth fields so formatters never see them.
    if let Some(obj) = data.as_object_mut() {
        obj.remove("key");
        obj.remove("digest");
    }

    // ── 3. Resolve room_id (path > query > body) ───────────────────────────
    let room_id = path_room_id
        .or_else(|| query.room_id.clone())
        .or_else(|| data["room_id"].as_str().map(String::from));

    let room_id = match room_id {
        Some(r) => r,
        None => return err(StatusCode::BAD_REQUEST, "Missing room_id"),
    };

    // ── 4. Legacy 'text' field alias ───────────────────────────────────────
    if let Some(obj) = data.as_object_mut() {
        if let Some(text) = obj.remove("text") {
            obj.entry("body").or_insert(text);
        }
    }

    // ── 5. Apply formatter (query > body) ─────────────────────────────────
    let formatter_name = query
        .formatter
        .or_else(|| data["formatter"].as_str().map(String::from));

    let header_map: HashMap<String, String> = http_headers
        .iter()
        .filter_map(|(k, v)| {
            v.to_str()
                .ok()
                .map(|s| (k.as_str().to_string(), s.to_string()))
        })
        .collect();

    let formatted = if let Some(ref name) = formatter_name {
        match state.formatters.apply(name, data, &header_map) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("Formatter '{}' error: {}", name, e);
                return err(StatusCode::INTERNAL_SERVER_ERROR, "Formatter error");
            }
        }
    } else {
        data
    };

    // ── 6. Extract body text ───────────────────────────────────────────────
    let body_text = match formatted["body"].as_str() {
        Some(b) => b.to_string(),
        None => return err(StatusCode::BAD_REQUEST, "Missing 'body' field in message"),
    };

    let msgtype = formatted["msgtype"]
        .as_str()
        .unwrap_or("m.text")
        .to_string();

    // ── 7. Markdown → HTML ─────────────────────────────────────────────────
    let html_body = markdown::to_html(&body_text);

    // ── 8. Send to Matrix ──────────────────────────────────────────────────
    match state
        .matrix
        .send_message(&room_id, &body_text, &html_body, &msgtype)
        .await
    {
        Ok(_) => {
            tracing::info!("Message delivered to {}", room_id);
            (StatusCode::OK, Json(json!({"status": "ok"}))).into_response()
        }
        Err(e) => {
            tracing::error!("Matrix send error: {:#}", e);
            err(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to send message to Matrix",
            )
        }
    }
}

fn err(status: StatusCode, message: &'static str) -> Response {
    (status, Json(json!({"error": message}))).into_response()
}
