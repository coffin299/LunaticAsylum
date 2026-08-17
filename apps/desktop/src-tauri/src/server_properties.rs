//! Minecraft server.properties 読み書き

use crate::paths;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerPropertiesDto {
    pub exists: bool,
    pub raw: String,
    pub fields: BTreeMap<String, String>,
}

pub fn properties_path(instance: &Path) -> std::path::PathBuf {
    instance.join("server.properties")
}

pub fn read_properties(instance: &Path) -> Result<ServerPropertiesDto, String> {
    let path = properties_path(instance);
    if !path.is_file() {
        return Ok(ServerPropertiesDto {
            exists: false,
            raw: String::new(),
            fields: BTreeMap::new(),
        });
    }
    let root = paths::app_root().map_err(|e| e.to_string())?;
    let servers = paths::servers_dir(&root);
    crate::validate::ensure_within(&servers, &path)?;
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(ServerPropertiesDto {
        exists: true,
        raw: raw.clone(),
        fields: parse_properties(&raw),
    })
}

pub fn write_properties(instance: &Path, raw: &str) -> Result<(), String> {
    let path = properties_path(instance);
    let root = paths::app_root().map_err(|e| e.to_string())?;
    let servers = paths::servers_dir(&root);
    if path.exists() {
        crate::validate::ensure_within(&servers, &path)?;
        let bak = instance.join(".asylum").join("server.properties.bak");
        fs::create_dir_all(bak.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::copy(&path, &bak).map_err(|e| e.to_string())?;
    }
    let parent = path.parent().ok_or("invalid path")?;
    crate::validate::ensure_within(&servers, parent)?;
    if raw.len() > 512_000 {
        return Err("server.properties too large".into());
    }
    fs::write(&path, raw).map_err(|e| e.to_string())
}

pub fn parse_properties(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}
