//! Palworld PalWorldSettings.ini 読み書き + REST 設定同期

use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA: &str = include_str!("../resources/palworld_settings_schema.json");

pub const DEFAULT_REST_API_PORT: u16 = 8212;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PalworldSettingsDto {
    pub exists: bool,
    pub raw: String,
    pub fields: BTreeMap<String, String>,
    pub schema: serde_json::Value,
    pub running_warning: bool,
}

#[derive(Debug, Serialize, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PalworldRestSettingsDto {
    pub rest_api_enabled: bool,
    pub rest_api_port: u16,
    pub admin_password_configured: bool,
    pub rest_base_url: String,
    pub rest_username: String,
    pub running_warning: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PalworldRestSettingsWriteDto {
    pub rest_api_enabled: bool,
    pub rest_api_port: u16,
    /// 空なら変更なし
    pub admin_password: String,
}

pub fn settings_path(instance: &Path) -> PathBuf {
    instance
        .join("Pal")
        .join("Saved")
        .join("Config")
        .join("WindowsServer")
        .join("PalWorldSettings.ini")
}

fn default_template_path(instance: &Path) -> PathBuf {
    instance.join("DefaultPalWorldSettings.ini")
}

pub fn rest_base_url_from_port(port: u16) -> String {
    format!("http://127.0.0.1:{port}/v1/api")
}

pub fn parse_bool_ini(s: &str) -> bool {
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "true" | "1" | "yes"
    )
}

pub fn format_bool_ini(b: bool) -> String {
    if b { "True".into() } else { "False".into() }
}

pub fn ensure_settings_file(instance: &Path) -> Result<PathBuf, String> {
    let target = settings_path(instance);
    if target.is_file() {
        return Ok(target);
    }
    let default = default_template_path(instance);
    if !default.is_file() {
        return Err(
            "PalWorldSettings.ini not found. Start the server once or copy DefaultPalWorldSettings.ini"
                .into(),
        );
    }
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::copy(&default, &target).map_err(|e| e.to_string())?;
    Ok(target)
}

pub fn read_settings(instance: &Path, server_running: bool) -> Result<PalworldSettingsDto, String> {
    let path = settings_path(instance);
    let root = paths::app_root().map_err(|e| e.to_string())?;
    let servers = paths::servers_dir(&root);
    let schema: serde_json::Value =
        serde_json::from_str(SCHEMA).unwrap_or(serde_json::json!([]));

    if !path.is_file() {
        return Ok(PalworldSettingsDto {
            exists: false,
            raw: String::new(),
            fields: BTreeMap::new(),
            schema,
            running_warning: server_running,
        });
    }
    crate::validate::ensure_within(&servers, &path)?;
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    Ok(PalworldSettingsDto {
        exists: true,
        raw: raw.clone(),
        fields: parse_option_settings(&raw),
        schema,
        running_warning: server_running,
    })
}

pub fn read_rest_settings(
    instance: &Path,
    server_running: bool,
    rest_username: &str,
) -> Result<PalworldRestSettingsDto, String> {
    let dto = read_settings(instance, server_running)?;
    let port = dto
        .fields
        .get("RESTAPIPort")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_REST_API_PORT);
    let enabled = dto
        .fields
        .get("RESTAPIEnabled")
        .map(|s| parse_bool_ini(s))
        .unwrap_or(true);
    let admin_password_configured = dto
        .fields
        .get("AdminPassword")
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    Ok(PalworldRestSettingsDto {
        rest_api_enabled: enabled,
        rest_api_port: port,
        admin_password_configured,
        rest_base_url: rest_base_url_from_port(port),
        rest_username: rest_username.to_string(),
        running_warning: server_running,
    })
}

pub fn write_settings(instance: &Path, raw: &str) -> Result<(), String> {
    let path = ensure_settings_file(instance)?;
    let root = paths::app_root().map_err(|e| e.to_string())?;
    let servers = paths::servers_dir(&root);
    crate::validate::ensure_within(&servers, &path)?;
    if raw.len() > 1_024_000 {
        return Err("PalWorldSettings.ini too large".into());
    }
    let bak = instance.join(".asylum").join("PalWorldSettings.ini.bak");
    fs::create_dir_all(bak.parent().unwrap()).map_err(|e| e.to_string())?;
    if path.is_file() {
        fs::copy(&path, &bak).map_err(|e| e.to_string())?;
    }
    fs::write(&path, raw).map_err(|e| e.to_string())
}

/// ini の REST 関連フィールドを runtime config / keyring に反映（ini 優先）
pub fn sync_config_from_ini(
    instance: &Path,
    instance_id: &str,
    cfg: &mut crate::config::InstanceConfig,
) -> Result<bool, String> {
    let path = settings_path(instance);
    if !path.is_file() {
        return Ok(false);
    }
    let raw = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let fields = parse_option_settings(&raw);
    apply_ini_fields_to_config(instance_id, cfg, &fields)
}

pub fn apply_ini_fields_to_config(
    instance_id: &str,
    cfg: &mut crate::config::InstanceConfig,
    fields: &BTreeMap<String, String>,
) -> Result<bool, String> {
    let mut changed = false;

    if let Some(port_str) = fields.get("RESTAPIPort") {
        if let Ok(port) = port_str.parse::<u16>() {
            if port > 0 {
                let url = rest_base_url_from_port(port);
                if cfg.rest_api_port != port {
                    cfg.rest_api_port = port;
                    changed = true;
                }
                if cfg.rest_base_url != url {
                    cfg.rest_base_url = url;
                    changed = true;
                }
            }
        }
    }

    if let Some(enabled_str) = fields.get("RESTAPIEnabled") {
        let enabled = parse_bool_ini(enabled_str);
        if cfg.rest_api_enabled != enabled {
            cfg.rest_api_enabled = enabled;
            changed = true;
        }
    }

    if let Some(admin_pw) = fields.get("AdminPassword") {
        if !admin_pw.is_empty() {
            let stored = crate::secrets::get_rest_password(instance_id)?;
            if stored.as_deref() != Some(admin_pw.as_str()) {
                crate::secrets::set_rest_password(instance_id, admin_pw)?;
                cfg.rest_password = admin_pw.clone();
                changed = true;
            } else if cfg.rest_password.is_empty() {
                cfg.rest_password = admin_pw.clone();
            }
        }
    }

    Ok(changed)
}

/// config / パスワード変更を ini の OptionSettings に書き戻す
pub fn sync_ini_from_config(
    instance: &Path,
    cfg: &crate::config::InstanceConfig,
    new_admin_password: Option<&str>,
) -> Result<(), String> {
    let mut patches = BTreeMap::new();
    patches.insert(
        "RESTAPIEnabled".into(),
        format_bool_ini(cfg.rest_api_enabled),
    );
    patches.insert("RESTAPIPort".into(), cfg.rest_api_port.to_string());

    if let Some(pw) = new_admin_password.filter(|s| !s.is_empty()) {
        patches.insert("AdminPassword".into(), pw.to_string());
    } else if !cfg.rest_password.is_empty() {
        patches.insert("AdminPassword".into(), cfg.rest_password.clone());
    }

    let path = settings_path(instance);
    let raw = if path.is_file() {
        fs::read_to_string(&path).map_err(|e| e.to_string())?
    } else {
        String::new()
    };
    let updated = update_option_settings(&raw, &patches);
    write_settings(instance, &updated)
}

pub fn write_rest_settings(
    instance: &Path,
    instance_id: &str,
    cfg: &mut crate::config::InstanceConfig,
    dto: &PalworldRestSettingsWriteDto,
) -> Result<(), String> {
    if dto.rest_api_port == 0 {
        return Err("REST API port must be greater than 0".into());
    }
    cfg.rest_api_enabled = dto.rest_api_enabled;
    cfg.rest_api_port = dto.rest_api_port;
    cfg.rest_base_url = rest_base_url_from_port(dto.rest_api_port);
    crate::validate::validate_rest_base_url(&cfg.rest_base_url)?;

    let new_pw = if dto.admin_password.is_empty() {
        None
    } else {
        crate::secrets::set_rest_password(instance_id, &dto.admin_password)?;
        cfg.rest_password = dto.admin_password.clone();
        Some(dto.admin_password.as_str())
    };

    sync_ini_from_config(instance, cfg, new_pw)
}

pub fn update_option_settings(raw: &str, patches: &BTreeMap<String, String>) -> String {
    if raw.is_empty() {
        return build_minimal_ini(patches);
    }

    let Some(start) = raw.find("OptionSettings=(") else {
        return append_option_settings_line(raw, patches);
    };

    let prefix = &raw[..start];
    let after_marker = &raw[start + "OptionSettings=(".len()..];
    let Some(end_rel) = after_marker.rfind(')') else {
        return append_option_settings_line(raw, patches);
    };

    let body = &after_marker[..end_rel];
    let suffix = &after_marker[end_rel + 1..];

    let mut fields = BTreeMap::new();
    for part in split_option_body(body) {
        if let Some((k, v)) = part.split_once('=') {
            fields.insert(k.trim().to_string(), strip_quotes(v.trim()));
        }
    }
    for (k, v) in patches {
        fields.insert(k.clone(), v.clone());
    }

    let new_body = format_option_body(&fields);
    format!("{prefix}OptionSettings=({new_body}){suffix}")
}

fn append_option_settings_line(raw: &str, patches: &BTreeMap<String, String>) -> String {
    let mut trimmed = raw.trim_end().to_string();
    if !trimmed.is_empty() && !trimmed.ends_with('\n') {
        trimmed.push('\n');
    }
    if !trimmed.contains("[/Script/Pal.PalGameWorldSettings]") {
        trimmed.push_str("[/Script/Pal.PalGameWorldSettings]\n");
    }
    let body = format_option_body(patches);
    trimmed.push_str(&format!("OptionSettings=({body})\n"));
    trimmed
}

fn build_minimal_ini(patches: &BTreeMap<String, String>) -> String {
    let body = format_option_body(patches);
    format!("[/Script/Pal.PalGameWorldSettings]\nOptionSettings=({body})\n")
}

fn format_option_body(fields: &BTreeMap<String, String>) -> String {
    fields
        .iter()
        .map(|(k, v)| format!("{}={}", k, format_option_value(k, v)))
        .collect::<Vec<_>>()
        .join(",")
}

fn format_option_value(key: &str, value: &str) -> String {
    if key == "RESTAPIEnabled"
        || key.ends_with("Enabled")
        || key.starts_with('b')
            && key.len() > 1
            && key.as_bytes()[1].is_ascii_uppercase()
    {
        if parse_bool_ini(value) {
            "True".into()
        } else {
            "False".into()
        }
    } else if key == "RESTAPIPort"
        || key.ends_with("Num")
        || key.ends_with("Rate")
        || key.ends_with("Port")
    {
        value.to_string()
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

pub fn parse_option_settings(raw: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Some(start) = raw.find("OptionSettings=(") else {
        for line in raw.lines() {
            let t = line.trim();
            if t.is_empty() || t.starts_with(';') || t.starts_with('[') {
                continue;
            }
            if let Some((k, v)) = t.split_once('=') {
                map.insert(k.trim().to_string(), strip_quotes(v.trim()));
            }
        }
        return map;
    };
    let rest = &raw[start + "OptionSettings=(".len()..];
    let Some(end) = rest.rfind(')') else {
        return map;
    };
    let body = &rest[..end];
    for part in split_option_body(body) {
        if let Some((k, v)) = part.split_once('=') {
            map.insert(k.trim().to_string(), strip_quotes(v.trim()));
        }
    }
    map
}

fn split_option_body(body: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut cur = String::new();
    let mut depth = 0usize;
    let mut in_quotes = false;
    let mut escape = false;
    for ch in body.chars() {
        if escape {
            cur.push(ch);
            escape = false;
            continue;
        }
        if ch == '\\' && in_quotes {
            cur.push(ch);
            escape = true;
            continue;
        }
        if ch == '"' {
            in_quotes = !in_quotes;
            cur.push(ch);
            continue;
        }
        match ch {
            '(' if !in_quotes => {
                depth += 1;
                cur.push(ch);
            }
            ')' if !in_quotes => {
                depth = depth.saturating_sub(1);
                cur.push(ch);
            }
            ',' if depth == 0 && !in_quotes => {
                parts.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(ch),
        }
    }
    if !cur.trim().is_empty() {
        parts.push(cur.trim().to_string());
    }
    parts
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patch_preserves_other_keys() {
        let raw = r#"[/Script/Pal.PalGameWorldSettings]
OptionSettings=(ServerName="Test",AdminPassword="old",RESTAPIPort=8212,RESTAPIEnabled=True)
"#;
        let mut patches = BTreeMap::new();
        patches.insert("AdminPassword".into(), "newpass".into());
        patches.insert("RESTAPIPort".into(), "9000".into());
        let out = update_option_settings(raw, &patches);
        assert!(out.contains("ServerName=\"Test\""));
        assert!(out.contains("AdminPassword=\"newpass\""));
        assert!(out.contains("RESTAPIPort=9000"));
        assert!(out.contains("RESTAPIEnabled=True"));
    }

    #[test]
    fn parse_roundtrip_bool_and_port() {
        let raw = "OptionSettings=(RESTAPIEnabled=False,RESTAPIPort=7777,AdminPassword=\"pw\")";
        let fields = parse_option_settings(raw);
        assert_eq!(fields.get("RESTAPIPort").map(String::as_str), Some("7777"));
        assert!(!parse_bool_ini(fields.get("RESTAPIEnabled").unwrap()));
        assert_eq!(fields.get("AdminPassword").map(String::as_str), Some("pw"));
    }
}
