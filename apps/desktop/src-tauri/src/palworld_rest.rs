//! Palworld REST（同期・Bot / スケジューラ用）

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::Value;

#[derive(Debug)]
pub struct PalworldApiError {
    pub message: String,
    pub status_code: Option<u16>,
}

impl std::fmt::Display for PalworldApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

pub struct PalworldRestClient {
    base_url: String,
    auth_header: String,
    timeout_ms: u64,
}

impl PalworldRestClient {
    pub fn new(base_url: &str, username: &str, password: &str) -> Self {
        let token = STANDARD.encode(format!("{username}:{password}"));
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            auth_header: format!("Basic {token}"),
            timeout_ms: 10_000,
        }
    }

    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<Value>,
    ) -> Result<Option<Value>, PalworldApiError> {
        let url = format!(
            "{}{}",
            self.base_url,
            if path.starts_with('/') {
                path.to_string()
            } else {
                format!("/{path}")
            }
        );
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_millis(self.timeout_ms))
            .build();
        let mut req = match method {
            "GET" => agent.get(&url),
            "POST" => agent.post(&url),
            other => {
                return Err(PalworldApiError {
                    message: format!("unsupported method {other}"),
                    status_code: None,
                })
            }
        };
        req = req
            .set("Accept", "application/json")
            .set("Authorization", &self.auth_header);
        let resp = if let Some(json) = body {
            req.set("Content-Type", "application/json")
                .send_string(&json.to_string())
        } else {
            req.call()
        };
        let resp = resp.map_err(|e| PalworldApiError {
            message: format!("REST API 接続に失敗しました: {e}"),
            status_code: None,
        })?;
        let status = resp.status();
        if status == 401 {
            return Err(PalworldApiError {
                message: "REST API 認証に失敗しました（ユーザー名/パスワードを確認）。".into(),
                status_code: Some(401),
            });
        }
        if status >= 400 {
            let detail = resp.into_string().unwrap_or_default();
            return Err(PalworldApiError {
                message: format!("REST API エラー ({status}): {}", detail.trim()),
                status_code: Some(status),
            });
        }
        if status == 204 {
            return Ok(None);
        }
        let text = resp.into_string().map_err(|e| PalworldApiError {
            message: format!("read body: {e}"),
            status_code: Some(status),
        })?;
        if text.trim().is_empty() {
            return Ok(None);
        }
        let v: Value = serde_json::from_str(&text).map_err(|_| PalworldApiError {
            message: "REST API の応答が JSON ではありません。".into(),
            status_code: Some(status),
        })?;
        Ok(Some(v))
    }

    pub fn get_players(&self) -> Result<Vec<Value>, PalworldApiError> {
        let data = self.request("GET", "/players", None)?;
        let Some(data) = data else {
            return Ok(vec![]);
        };
        if let Some(players) = data.get("players").and_then(|p| p.as_array()) {
            return Ok(players.clone());
        }
        if let Some(arr) = data.as_array() {
            return Ok(arr.clone());
        }
        Err(PalworldApiError {
            message: "players 応答の形式が不正です。".into(),
            status_code: None,
        })
    }

    pub fn get_info(&self) -> Result<Value, PalworldApiError> {
        let data = self
            .request("GET", "/info", None)?
            .ok_or_else(|| PalworldApiError {
                message: "info 応答が空です。".into(),
                status_code: None,
            })?;
        if !data.is_object() {
            return Err(PalworldApiError {
                message: "info 応答の形式が不正です。".into(),
                status_code: None,
            });
        }
        Ok(data)
    }

    pub fn get_metrics(&self) -> Result<Value, PalworldApiError> {
        let data = self
            .request("GET", "/metrics", None)?
            .ok_or_else(|| PalworldApiError {
                message: "metrics 応答が空です。".into(),
                status_code: None,
            })?;
        if !data.is_object() {
            return Err(PalworldApiError {
                message: "metrics 応答の形式が不正です。".into(),
                status_code: None,
            });
        }
        Ok(data)
    }

    pub fn get_settings(&self) -> Result<Value, PalworldApiError> {
        let data = self
            .request("GET", "/settings", None)?
            .ok_or_else(|| PalworldApiError {
                message: "settings 応答が空です。".into(),
                status_code: None,
            })?;
        if !data.is_object() {
            return Err(PalworldApiError {
                message: "settings 応答の形式が不正です。".into(),
                status_code: None,
            });
        }
        Ok(data)
    }

    pub fn kick(&self, userid: &str, message: &str) -> Result<(), PalworldApiError> {
        self.request(
            "POST",
            "/kick",
            Some(serde_json::json!({ "userid": userid, "message": message })),
        )?;
        Ok(())
    }

    pub fn ban(&self, userid: &str, message: &str) -> Result<(), PalworldApiError> {
        self.request(
            "POST",
            "/ban",
            Some(serde_json::json!({ "userid": userid, "message": message })),
        )?;
        Ok(())
    }

    pub fn unban(&self, userid: &str) -> Result<(), PalworldApiError> {
        self.request(
            "POST",
            "/unban",
            Some(serde_json::json!({ "userid": userid })),
        )?;
        Ok(())
    }

    pub fn announce(&self, message: &str) -> Result<(), PalworldApiError> {
        self.request(
            "POST",
            "/announce",
            Some(serde_json::json!({ "message": message })),
        )?;
        Ok(())
    }

    pub fn save(&self) -> Result<(), PalworldApiError> {
        self.request("POST", "/save", None)?;
        Ok(())
    }

    pub fn shutdown(&self, waittime: i64, message: &str) -> Result<(), PalworldApiError> {
        self.request(
            "POST",
            "/shutdown",
            Some(serde_json::json!({ "waittime": waittime, "message": message })),
        )?;
        Ok(())
    }

    pub fn stop(&self) -> Result<(), PalworldApiError> {
        self.request("POST", "/stop", None)?;
        Ok(())
    }

    pub fn current_player_count(&self) -> Result<u64, PalworldApiError> {
        match self.get_metrics() {
            Ok(m) => Ok(m
                .get("currentplayernum")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)),
            Err(_) => Ok(self.get_players()?.len() as u64),
        }
    }
}
