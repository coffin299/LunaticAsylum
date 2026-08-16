//! バックアップ・更新検知の定期実行

use crate::backup;
use crate::config::{load_hydrated_config, InstanceConfig};
use crate::palworld_rest::PalworldRestClient;
use crate::paths;
use crate::state::AppState;
use crate::steamcmd;
use crate::update_check;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchedulerState {
    last_backup_unix: HashMap<String, u64>,
    last_update_check_unix: HashMap<String, u64>,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn state_path(root: &PathBuf) -> PathBuf {
    root.join(".asylum").join("scheduler.json")
}

fn load_state(root: &PathBuf) -> SchedulerState {
    let path = state_path(root);
    let Ok(text) = std::fs::read_to_string(path) else {
        return SchedulerState::default();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn save_state(root: &PathBuf, state: &SchedulerState) {
    let asylum = root.join(".asylum");
    let _ = std::fs::create_dir_all(&asylum);
    if let Ok(text) = serde_json::to_string_pretty(state) {
        let _ = std::fs::write(state_path(root), text);
    }
}

pub fn spawn_scheduler(app_state: Arc<Mutex<AppState>>) {
    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(60));
        let Ok(root) = paths::app_root() else {
            continue;
        };
        let servers = paths::servers_dir(&root);
        if !servers.exists() {
            continue;
        }
        let mut sched = load_state(&root);
        let now = now_unix();

        let Ok(entries) = std::fs::read_dir(&servers) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let id = entry.file_name().to_string_lossy().into_owned();
            if id.starts_with('.') {
                continue;
            }
            if paths::detect_provider(&path) != "palworld" {
                continue;
            }
            let cfg = match load_hydrated_config(&path, &id) {
                Ok(c) => c,
                Err(_) => continue,
            };
            tick_backup(&id, &path, &cfg, &mut sched, now);
            tick_update(&id, &path, &cfg, &mut sched, now, &app_state, &root);
        }
        save_state(&root, &sched);
    });
}

fn tick_backup(
    id: &str,
    instance: &PathBuf,
    cfg: &InstanceConfig,
    sched: &mut SchedulerState,
    now: u64,
) {
    if !cfg.backup.enabled {
        return;
    }
    let interval = cfg.backup_interval_secs().max(60);
    let last = sched.last_backup_unix.get(id).copied().unwrap_or(0);
    if now.saturating_sub(last) < interval {
        return;
    }
    if backup::create_backup(instance, cfg.backup.keep_count).is_ok() {
        sched.last_backup_unix.insert(id.to_string(), now);
    }
}

fn tick_update(
    id: &str,
    instance: &PathBuf,
    cfg: &InstanceConfig,
    sched: &mut SchedulerState,
    now: u64,
    app_state: &Arc<Mutex<AppState>>,
    root: &PathBuf,
) {
    if !cfg.update_check.polling_enabled {
        return;
    }
    let interval = cfg.update_check.interval_minutes.max(15) * 60;
    let last = sched.last_update_check_unix.get(id).copied().unwrap_or(0);
    if now.saturating_sub(last) < interval {
        return;
    }
    sched.last_update_check_unix.insert(id.to_string(), now);

    let (primary, _) = update_check::default_palworld_checker();
    let checker = update_check::UpdateChecker::new(vec![&primary]);
    let Ok(result) = checker.check(steamcmd::PALWORLD_APP_ID, instance, "public") else {
        return;
    };
    if let Ok(mut g) = app_state.lock() {
        g.update_flags
            .insert(id.to_string(), result.update_available);
    }
    if !result.update_available || !cfg.update_check.auto_apply {
        return;
    }
    if cfg.update_check.auto_apply_only_when_empty {
        let client = PalworldRestClient::new(
            &cfg.rest_base_url,
            &cfg.rest_username,
            &cfg.rest_password,
        );
        if client.current_player_count().unwrap_or(1) > 0 {
            return;
        }
    }
    {
        if let Ok(mut g) = app_state.lock() {
            if g.is_running(id) {
                let _ = g.stop_intentional(id);
            }
        }
    }
    let tools = root.join("tools").join("steamcmd");
    if let Ok(exe) = steamcmd::ensure_steamcmd(&tools) {
        if steamcmd::app_update(&exe, instance, steamcmd::PALWORLD_APP_ID, true).is_ok() {
            if let Ok(mut g) = app_state.lock() {
                g.update_flags.insert(id.to_string(), false);
            }
        }
    }
}
