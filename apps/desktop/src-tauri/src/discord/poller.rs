use super::notifier::{player_key, Notifier};
use crate::palworld_rest::{PalworldApiError, PalworldRestClient};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PlayerPoller {
    client: Arc<PalworldRestClient>,
    notifier: Arc<Mutex<Notifier>>,
    interval_secs: u64,
    known: HashMap<String, Value>,
    unreachable: bool,
    initialized: bool,
}

impl PlayerPoller {
    pub fn new(
        client: Arc<PalworldRestClient>,
        notifier: Arc<Mutex<Notifier>>,
        interval_secs: u64,
    ) -> Self {
        Self {
            client,
            notifier,
            interval_secs: interval_secs.max(5),
            known: HashMap::new(),
            unreachable: false,
            initialized: false,
        }
    }

    fn fetch_snapshot(&self) -> Result<(Vec<Value>, i64, i64, Option<Value>), PalworldApiError> {
        let players = self.client.get_players()?;
        let mut current = players.len() as i64;
        let mut maximum = current.max(1);
        if let Ok(metrics) = self.client.get_metrics() {
            if let Some(v) = metrics.get("currentplayernum").and_then(|v| v.as_i64()) {
                current = v;
            }
            if let Some(v) = metrics.get("maxplayernum").and_then(|v| v.as_i64()) {
                maximum = v;
            }
        }
        let info = self.client.get_info().ok();
        Ok((players, current, maximum, info))
    }

    pub async fn bootstrap(&mut self) {
        match self.fetch_snapshot() {
            Ok((players, current, maximum, info)) => {
                self.known = players
                    .iter()
                    .map(|p| (player_key(p), p.clone()))
                    .collect();
                self.initialized = true;
                self.unreachable = false;
                {
                    let mut n = self.notifier.lock().await;
                    n.notify_startup(info.as_ref(), &players, true).await;
                    n.update_topic(current, maximum, true).await;
                }
            }
            Err(e) => {
                self.unreachable = true;
                let n = self.notifier.lock().await;
                n.notify_startup(None, &[], false).await;
                n.notify_unreachable(&e.message).await;
            }
        }
    }

    async fn tick(&mut self) {
        let snapshot = self.fetch_snapshot();
        match snapshot {
            Err(e) => {
                if !self.unreachable {
                    self.unreachable = true;
                    self.notifier.lock().await.notify_unreachable(&e.message).await;
                }
            }
            Ok((players, current, maximum, _)) => {
                if self.unreachable {
                    self.unreachable = false;
                    self.notifier
                        .lock()
                        .await
                        .notify_recovered(players.len())
                        .await;
                }
                let current_map: HashMap<String, Value> = players
                    .iter()
                    .map(|p| (player_key(p), p.clone()))
                    .collect();
                if !self.initialized {
                    self.known = current_map;
                    self.initialized = true;
                    self.notifier
                        .lock()
                        .await
                        .update_topic(current, maximum, true)
                        .await;
                    return;
                }
                let joined: Vec<_> = current_map
                    .keys()
                    .filter(|k| !self.known.contains_key(*k))
                    .cloned()
                    .collect();
                let left: Vec<_> = self
                    .known
                    .keys()
                    .filter(|k| !current_map.contains_key(*k))
                    .cloned()
                    .collect();
                {
                    let mut n = self.notifier.lock().await;
                    for key in joined {
                        if let Some(p) = current_map.get(&key) {
                            n.notify_join(p, current, maximum).await;
                        }
                    }
                    for key in left {
                        if let Some(p) = self.known.get(&key) {
                            n.notify_leave(p, current, maximum).await;
                        }
                    }
                    n.update_topic(current, maximum, false).await;
                }
                self.known = current_map;
            }
        }
    }

    pub async fn run(mut self, mut stop: tokio::sync::oneshot::Receiver<()>) {
        self.bootstrap().await;
        loop {
            tokio::select! {
                _ = &mut stop => break,
                _ = tokio::time::sleep(std::time::Duration::from_secs(self.interval_secs)) => {
                    self.tick().await;
                }
            }
        }
    }
}
