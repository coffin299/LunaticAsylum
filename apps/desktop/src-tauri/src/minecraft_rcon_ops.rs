//! Minecraft RCON 操作（server.properties 同期）

use crate::paths;
use crate::server_properties;
use crate::source_rcon::SourceRconClient;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;

pub const DEFAULT_RCON_PORT: u16 = 25575;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftRconSettingsDto {
    pub enabled: bool,
    pub port: u16,
    pub password_configured: bool,
    pub running_warning: bool,
}

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftRconSettingsWriteDto {
    pub enabled: bool,
    pub port: u16,
    pub password: String,
}

pub fn read_rcon_settings(instance: &Path, server_running: bool) -> Result<MinecraftRconSettingsDto, String> {
    let props = server_properties::read_properties(instance)?;
    let enabled = props
        .fields
        .get("enable-rcon")
        .map(|s| parse_bool_prop(s))
        .unwrap_or(false);
    let port = props
        .fields
        .get("rcon.port")
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_RCON_PORT);
    let password_configured = props
        .fields
        .get("rcon.password")
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    Ok(MinecraftRconSettingsDto {
        enabled,
        port,
        password_configured,
        running_warning: server_running,
    })
}

pub fn write_rcon_settings(instance: &Path, dto: &MinecraftRconSettingsWriteDto) -> Result<(), String> {
    if dto.port == 0 {
        return Err("RCON port must be greater than 0".into());
    }
    let props = server_properties::read_properties(instance)?;
    let mut fields = props.fields;
    fields.insert("enable-rcon".into(), format_bool_prop(dto.enabled));
    fields.insert("rcon.port".into(), dto.port.to_string());
    if !dto.password.is_empty() {
        fields.insert("rcon.password".into(), dto.password.clone());
    }
    let raw = render_properties(&props.raw, &fields);
    server_properties::write_properties(instance, &raw)
}

fn client_for(id: &str) -> Result<SourceRconClient, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, id)?;
    if !paths::is_minecraft_java(&instance) {
        return Err("RCON is supported for Minecraft Java servers only".into());
    }
    let settings = read_rcon_settings(&instance, true)?;
    if !settings.enabled {
        return Err("RCON is disabled in server.properties".into());
    }
    let props = server_properties::read_properties(&instance)?;
    let password = props
        .fields
        .get("rcon.password")
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "RCON password is not set in server.properties".to_string())?
        .clone();
    SourceRconClient::connect("127.0.0.1", settings.port, &password, 10_000).map_err(|e| e.message)
}

fn with_client<F>(id: &str, f: F) -> Result<String, String>
where
    F: FnOnce(&mut SourceRconClient) -> Result<String, String>,
{
    let mut client = client_for(id)?;
    f(&mut client)
}

fn rcon_cmd(client: &mut SourceRconClient, cmd: &str) -> Result<String, String> {
    client.command(cmd).map_err(|e| e.message)
}

pub fn get_players(id: &str) -> Result<Vec<Value>, String> {
    let output = with_client(id, |c| rcon_cmd(c, "list"))?;
    Ok(parse_list_output(&output))
}

pub fn get_info(id: &str) -> Result<Value, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, id)?;
    let props = server_properties::read_properties(&instance)?;
    let list_out = with_client(id, |c| rcon_cmd(c, "list")).unwrap_or_default();
    let parsed = parse_list_output(&list_out);
    Ok(json!({
        "maxPlayers": props.fields.get("max-players").cloned().unwrap_or_default(),
        "gamemode": props.fields.get("gamemode").cloned().unwrap_or_default(),
        "difficulty": props.fields.get("difficulty").cloned().unwrap_or_default(),
        "onlineCount": parsed.len(),
        "motd": props.fields.get("motd").cloned().unwrap_or_default(),
    }))
}

pub fn get_metrics(id: &str) -> Result<Value, String> {
    let players = get_players(id)?;
    let tps = with_client(id, |c| rcon_cmd(c, "tps")).ok();
    Ok(json!({
        "currentPlayers": players.len(),
        "tps": tps,
    }))
}

pub fn announce(id: &str, message: &str) -> Result<(), String> {
    validate_message(message)?;
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    with_client(id, |c| {
        rcon_cmd(c, &format!("say {escaped}"))?;
        Ok(String::new())
    })?;
    Ok(())
}

pub fn save_world(id: &str) -> Result<(), String> {
    with_client(id, |c| {
        rcon_cmd(c, "save-all flush")?;
        Ok(String::new())
    })?;
    Ok(())
}

pub fn kick(id: &str, player: &str, reason: &str) -> Result<(), String> {
    validate_player_name(player)?;
    let cmd = if reason.trim().is_empty() {
        format!("kick {player}")
    } else {
        format!("kick {player} {reason}")
    };
    with_client(id, |c| {
        rcon_cmd(c, &cmd)?;
        Ok(String::new())
    })?;
    Ok(())
}

pub fn ban(id: &str, player: &str, reason: &str) -> Result<(), String> {
    validate_player_name(player)?;
    let cmd = if reason.trim().is_empty() {
        format!("ban {player}")
    } else {
        format!("ban {player} {reason}")
    };
    with_client(id, |c| {
        rcon_cmd(c, &cmd)?;
        Ok(String::new())
    })?;
    Ok(())
}

pub fn unban(id: &str, player: &str) -> Result<(), String> {
    validate_player_name(player)?;
    with_client(id, |c| {
        rcon_cmd(c, &format!("pardon {player}"))?;
        Ok(String::new())
    })?;
    Ok(())
}

pub fn stop(id: &str) -> Result<(), String> {
    with_client(id, |c| {
        rcon_cmd(c, "stop")?;
        Ok(String::new())
    })?;
    Ok(())
}

pub fn command(id: &str, cmd: &str) -> Result<String, String> {
    if cmd.len() > 4096 {
        return Err("command too long".into());
    }
    with_client(id, |c| rcon_cmd(c, cmd))
}

pub fn try_save(id: &str) -> Result<(), String> {
    save_world(id)
}

fn parse_list_output(output: &str) -> Vec<Value> {
    let lower = output.to_lowercase();
    let names_part = if let Some(idx) = lower.find("online:") {
        output[idx + "online:".len()..].trim()
    } else {
        return vec![];
    };
    if names_part.is_empty() {
        return vec![];
    }
    names_part
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|name| {
            json!({
                "name": name,
                "userId": name,
                "playerName": name,
            })
        })
        .collect()
}

fn parse_bool_prop(s: &str) -> bool {
    matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "yes")
}

fn format_bool_prop(b: bool) -> String {
    if b { "true".into() } else { "false".into() }
}

fn render_properties(original: &str, fields: &BTreeMap<String, String>) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut seen = BTreeMap::<String, bool>::new();
    for line in original.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            lines.push(line.to_string());
            continue;
        }
        if let Some((k, _)) = trimmed.split_once('=') {
            let key = k.trim().to_string();
            if let Some(v) = fields.get(&key) {
                lines.push(format!("{key}={v}"));
                seen.insert(key, true);
            } else {
                lines.push(line.to_string());
            }
        } else {
            lines.push(line.to_string());
        }
    }
    for (k, v) in fields {
        if !seen.contains_key(k) {
            lines.push(format!("{k}={v}"));
        }
    }
    lines.join("\n") + "\n"
}

fn validate_message(message: &str) -> Result<(), String> {
    if message.is_empty() || message.len() > 512 {
        return Err("message length invalid".into());
    }
    Ok(())
}

fn validate_player_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("player name invalid".into());
    }
    if name.chars().any(|c| c.is_control()) {
        return Err("player name invalid".into());
    }
    Ok(())
}
