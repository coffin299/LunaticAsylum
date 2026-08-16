//! Palworld Discord Integration（インスタンスごと独立）

mod bot;
mod notifier;
mod poller;

use crate::config::DiscordConfig;
use crate::palworld_rest::PalworldRestClient;
use crate::state::AppState;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;

pub struct DiscordRuntime {
    /// instance_id -> stop sender
    stops: Mutex<HashMap<String, oneshot::Sender<()>>>,
}

impl Default for DiscordRuntime {
    fn default() -> Self {
        Self {
            stops: Mutex::new(HashMap::new()),
        }
    }
}

impl DiscordRuntime {
    pub fn apply(
        &self,
        instance_id: &str,
        discord: &DiscordConfig,
        rest_base: &str,
        rest_user: &str,
        rest_pass: &str,
        app_state: Arc<Mutex<AppState>>,
    ) -> Result<(), String> {
        self.stop(instance_id);
        if !discord.enabled {
            return Ok(());
        }
        if discord.token.trim().is_empty() {
            return Err("Discord Token が空です".into());
        }
        if discord.guild_id.trim().is_empty() || discord.channel_id.trim().is_empty() {
            return Err("Guild ID / Channel ID が必要です".into());
        }

        let (tx, rx) = oneshot::channel();
        {
            let mut g = self.stops.lock().map_err(|e| e.to_string())?;
            g.insert(instance_id.to_string(), tx);
        }

        let cfg = discord.clone();
        let id = instance_id.to_string();
        let rest = PalworldRestClient::new(rest_base, rest_user, rest_pass);

        std::thread::Builder::new()
            .name(format!("discord-{id}"))
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()
                {
                    Ok(r) => r,
                    Err(e) => {
                        eprintln!("discord runtime failed: {e}");
                        return;
                    }
                };
                rt.block_on(bot::run_bot(id, cfg, rest, app_state, rx));
            })
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn stop(&self, instance_id: &str) {
        if let Ok(mut g) = self.stops.lock() {
            if let Some(tx) = g.remove(instance_id) {
                let _ = tx.send(());
            }
        }
    }

    pub fn status(&self, instance_id: &str) -> bool {
        self.stops
            .lock()
            .map(|g| g.contains_key(instance_id))
            .unwrap_or(false)
    }
}
