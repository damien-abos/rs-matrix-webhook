use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::Settings;

pub struct MatrixClient {
    client: Client,
    base_url: String,
    access_token: String,
    /// Rooms we have already successfully joined (cached to avoid redundant calls)
    joined_rooms: Arc<RwLock<HashSet<String>>>,
}

impl MatrixClient {
    pub async fn new(settings: &Settings) -> Result<Self> {
        let client = Client::new();

        let access_token = if let Some(token) = &settings.matrix_token {
            token.clone()
        } else if let Some(pw) = &settings.matrix_pw {
            login(&client, &settings.matrix_url, &settings.matrix_id, pw).await?
        } else {
            anyhow::bail!("Either MATRIX_TOKEN or MATRIX_PW must be provided");
        };

        Ok(Self {
            client,
            base_url: settings.matrix_url.trim_end_matches('/').to_string(),
            access_token,
            joined_rooms: Arc::new(RwLock::new(HashSet::new())),
        })
    }

    /// Sends a message to `room_id`, joining the room first if necessary.
    pub async fn send_message(
        &self,
        room_id: &str,
        body: &str,
        html_body: &str,
        msgtype: &str,
    ) -> Result<()> {
        self.ensure_joined(room_id).await;

        let txn_id = Uuid::new_v4();
        let url = format!(
            "{}/_matrix/client/v3/rooms/{}/send/m.room.message/{}",
            self.base_url,
            urlencoding::encode(room_id),
            txn_id,
        );

        let payload = json!({
            "msgtype": msgtype,
            "body": body,
            "format": "org.matrix.custom.html",
            "formatted_body": html_body,
        });

        let resp = self
            .client
            .put(&url)
            .bearer_auth(&self.access_token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| anyhow::anyhow!("HTTP request to Matrix homeserver failed: {:?}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Matrix send error {}: {}", status, text);
        }

        Ok(())
    }

    /// Attempts to join `room_id`, caching successes so we only join once.
    async fn ensure_joined(&self, room_id: &str) {
        {
            let guard = self.joined_rooms.read().await;
            if guard.contains(room_id) {
                return;
            }
        }

        let url = format!(
            "{}/_matrix/client/v3/join/{}",
            self.base_url,
            urlencoding::encode(room_id),
        );

        match self
            .client
            .post(&url)
            .bearer_auth(&self.access_token)
            .json(&json!({}))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                let mut guard = self.joined_rooms.write().await;
                guard.insert(room_id.to_string());
            }
            Ok(resp) => {
                tracing::warn!("Could not join room {}: HTTP {}", room_id, resp.status());
            }
            Err(e) => {
                tracing::warn!("Could not join room {}: {:?}", room_id, e);
            }
        }
    }
}

async fn login(client: &Client, matrix_url: &str, user_id: &str, password: &str) -> Result<String> {
    let url = format!(
        "{}/_matrix/client/v3/login",
        matrix_url.trim_end_matches('/')
    );

    let resp = client
        .post(&url)
        .json(&json!({
            "type": "m.login.password",
            "identifier": { "type": "m.id.user", "user": user_id },
            "password": password,
        }))
        .send()
        .await
        .context("Failed to reach Matrix homeserver for login")?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("Matrix login failed {}: {}", status, text);
    }

    let body: Value = resp
        .json()
        .await
        .context("Failed to parse Matrix login response")?;

    body["access_token"]
        .as_str()
        .map(String::from)
        .context("Matrix login response missing access_token")
}
