//! Palworld REST を Rust 側で実行（秘密をフロントに返さない）

use crate::config::load_hydrated_config;
use crate::palworld_rest::PalworldRestClient;
use crate::paths;
use serde_json::Value;

fn client_for(id: &str) -> Result<PalworldRestClient, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, id)?;
    let cfg = load_hydrated_config(&instance, id)?;
    if !cfg.rest_api_enabled {
        return Err("REST API is disabled in PalWorldSettings.ini".into());
    }
    if cfg.rest_password.is_empty() {
        return Err("REST password is not set".into());
    }
    Ok(PalworldRestClient::new(
        &cfg.rest_base_url,
        &cfg.rest_username,
        &cfg.rest_password,
    ))
}

pub fn get_players(id: &str) -> Result<Vec<Value>, String> {
    client_for(id)?
        .get_players()
        .map_err(|e| e.message)
}

pub fn get_info(id: &str) -> Result<Value, String> {
    client_for(id)?.get_info().map_err(|e| e.message)
}

pub fn get_metrics(id: &str) -> Result<Value, String> {
    client_for(id)?.get_metrics().map_err(|e| e.message)
}

pub fn announce(id: &str, message: &str) -> Result<(), String> {
    client_for(id)?
        .announce(message)
        .map_err(|e| e.message)
}

pub fn save(id: &str) -> Result<(), String> {
    client_for(id)?.save().map_err(|e| e.message)
}

pub fn kick(id: &str, userid: &str, message: &str) -> Result<(), String> {
    client_for(id)?
        .kick(userid, message)
        .map_err(|e| e.message)
}

pub fn ban(id: &str, userid: &str, message: &str) -> Result<(), String> {
    client_for(id)?
        .ban(userid, message)
        .map_err(|e| e.message)
}

pub fn get_settings(id: &str) -> Result<Value, String> {
    client_for(id)?.get_settings().map_err(|e| e.message)
}

pub fn unban(id: &str, userid: &str) -> Result<(), String> {
    client_for(id)?.unban(userid).map_err(|e| e.message)
}

pub fn shutdown(id: &str, waittime: i64, message: &str) -> Result<(), String> {
    client_for(id)?
        .shutdown(waittime, message)
        .map_err(|e| e.message)
}

pub fn stop(id: &str) -> Result<(), String> {
    client_for(id)?.stop().map_err(|e| e.message)
}

/// バックアップ前の任意 save（失敗しても呼び出し側で握りつぶし可）
pub fn try_save(id: &str) -> Result<(), String> {
    save(id)
}
