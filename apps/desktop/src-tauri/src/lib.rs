mod backup;
mod config;
mod discord;
mod game_art;
mod host_resources;
mod minecraft_rcon_ops;
mod palworld_rest;
mod palworld_save;
mod palworld_settings;
mod paths;
mod process;
mod rest_ops;
mod save_parser;
mod scheduler;
mod secret_util;
mod secrets;
mod server_properties;
mod source_rcon;
mod state;
mod steamcmd;
mod update_check;
mod validate;

use config::{
    apply_dto_updates, load_hydrated_config, save_instance_config, to_dto, InstanceConfigDto,
};
use discord::DiscordRuntime;
use state::{spawn_crash_monitor, AppState};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, State};
#[derive(Debug, serde::Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ServerInstanceDto {
    pub id: String,
    pub display_name: String,
    pub path: String,
    pub provider_id: String,
    pub status: String,
    pub update_available: bool,
    pub pid: Option<u32>,
    pub discord_running: bool,
    pub launchable: bool,
    pub minecraft_server_type: Option<String>,
}
#[tauri::command]
fn ensure_servers_layout(state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, String> {
    let root = paths::app_root()?;
    let servers = paths::servers_dir(&root);
    std::fs::create_dir_all(&servers).map_err(|e| e.to_string())?;
    let guide = servers.join("HOW_TO_ADD_SERVERS.txt");
    if !guide.exists() {
        std::fs::write(&guide, paths::HOW_TO_ADD_SERVERS).map_err(|e| e.to_string())?;
    }
    let mut g = state.lock().map_err(|e| e.to_string())?;
    g.root = Some(root.clone());
    Ok(servers.to_string_lossy().into_owned())
}
#[tauri::command]
fn get_app_root() -> Result<String, String> {
    Ok(paths::app_root()?.to_string_lossy().into_owned())
}
#[tauri::command]
fn list_server_instances(
    state: State<'_, Arc<Mutex<AppState>>>,
    discord: State<'_, Arc<DiscordRuntime>>,
) -> Result<Vec<ServerInstanceDto>, String> {
    let root = paths::app_root()?;
    let servers = paths::servers_dir(&root);
    if !servers.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    let mut guard = state.lock().map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(&servers).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        let provider_id = paths::detect_provider(&path).to_string();
        let launchable = paths::is_launchable(&path);
        let cfg = config::load_instance_config(&path);
        let minecraft_server_type = if provider_id == "minecraft" {
            Some(cfg.minecraft.server_type.clone())
        } else {
            None
        };
        let running = guard.is_running(&name);
        let status = if running {
            "running"
        } else if path.join(".asylum").join("installing").exists() {
            "installing"
        } else {
            "stopped"
        };
        out.push(ServerInstanceDto {
            id: name.clone(),
            display_name: name.clone(),
            path: path.to_string_lossy().into_owned(),
            provider_id,
            status: status.into(),
            update_available: guard.update_flags.get(&name).copied().unwrap_or(false),
            pid: guard.pid_of(&name),
            discord_running: discord.status(&name),
            launchable,
            minecraft_server_type,
        });
    }
    out.sort_by(|a, b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()));
    Ok(out)
}
#[tauri::command]
fn start_server(id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let provider = paths::detect_provider(&instance);
    if provider == "unknown" {
        return Err("unsupported or empty server folder".into());
    }
    let mut g = state.lock().map_err(|e| e.to_string())?;
    g.root = Some(root.clone());
    if provider == "palworld" {
        let exe = paths::find_palserver_exe(&instance)
            .ok_or_else(|| "PalServer.exe not found".to_string())?;
        return g.start(&id, &exe, &instance);
    }
    if provider == "minecraft" {
        let cfg = config::load_instance_config(&instance);
        let jar = resolve_minecraft_jar(&instance, &cfg.minecraft)?;
        let mut args = split_shell_args(&cfg.minecraft.jvm_args);
        args.push("-jar".into());
        args.push(jar.to_string_lossy().into_owned());
        args.extend(split_shell_args(&cfg.minecraft.server_args));
        return g.start_java(&id, &args, &instance);
    }
    Err("unsupported provider".into())
}

fn resolve_minecraft_jar(
    instance: &std::path::Path,
    mc: &config::MinecraftConfig,
) -> Result<std::path::PathBuf, String> {
    let root = paths::app_root()?;
    let servers = paths::servers_dir(&root);
    if !mc.jar_file.trim().is_empty() {
        let p = instance.join(mc.jar_file.trim());
        crate::validate::ensure_within(&servers, &p)?;
        if p.is_file() {
            return Ok(p);
        }
        return Err("configured jar file not found".into());
    }
    paths::find_minecraft_jar(instance).ok_or_else(|| "minecraft jar not found".into())
}

fn split_shell_args(s: &str) -> Vec<String> {
    s.split_whitespace()
        .map(|x| x.to_string())
        .filter(|x| !x.is_empty())
        .collect()
}
#[tauri::command]
fn stop_server(id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let root = paths::app_root()?;
    let had_child = {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.root = Some(root);
        g.pid_of(&id).is_some()
    };
    if had_child {
        let _ = rest_ops::stop(&id);
        for _ in 0..20 {
            std::thread::sleep(std::time::Duration::from_millis(500));
            let mut g = state.lock().map_err(|e| e.to_string())?;
            if !g.is_running(&id) {
                break;
            }
        }
    }
    let mut g = state.lock().map_err(|e| e.to_string())?;
    g.stop_intentional(&id)
}
#[tauri::command]
fn restart_server(id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.stop_intentional(&id)?;
    }
    std::thread::sleep(std::time::Duration::from_millis(800));
    start_server(id, state)
}
#[tauri::command]
fn install_palworld(id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<String, String> {
    let root = paths::app_root()?;
    validate::validate_instance_id(&id)?;
    let instance = paths::servers_dir(&root).join(&id);
    if instance.exists() {
        return Err("instance folder already exists".into());
    }
    std::fs::create_dir_all(&instance).map_err(|e| e.to_string())?;
    let asylum = instance.join(".asylum");
    std::fs::create_dir_all(&asylum).map_err(|e| e.to_string())?;
    let flag = asylum.join("installing");
    std::fs::write(&flag, "1").map_err(|e| e.to_string())?;
    let tools = root.join("tools").join("steamcmd");
    let result = steamcmd::ensure_steamcmd(&tools).and_then(|steamcmd_exe| {
        steamcmd::app_update(&steamcmd_exe, &instance, steamcmd::PALWORLD_APP_ID, true)
    });
    let _ = std::fs::remove_file(&flag);
    result?;
    let mut g = state.lock().map_err(|e| e.to_string())?;
    g.update_flags.insert(id.clone(), false);
    Ok(instance.to_string_lossy().into_owned())
}
#[tauri::command]
fn update_palworld(id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        if g.is_running(&id) {
            g.stop_intentional(&id)?;
        }
    }
    let tools = root.join("tools").join("steamcmd");
    let steamcmd_exe = steamcmd::ensure_steamcmd(&tools)?;
    steamcmd::app_update(&steamcmd_exe, &instance, steamcmd::PALWORLD_APP_ID, true)?;
    let mut g = state.lock().map_err(|e| e.to_string())?;
    g.update_flags.insert(id, false);
    Ok(())
}
#[tauri::command]
fn check_palworld_update(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<bool, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let (primary, _) = update_check::default_palworld_checker();
    let checker = update_check::UpdateChecker::new(vec![&primary]);
    let result = checker.check(steamcmd::PALWORLD_APP_ID, &instance, "public")?;
    let mut g = state.lock().map_err(|e| e.to_string())?;
    g.update_flags.insert(id, result.update_available);
    Ok(result.update_available)
}
#[tauri::command]
fn create_backup(id: String) -> Result<backup::BackupEntryDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let cfg = load_hydrated_config(&instance, &id)?;
    backup::create_backup(&instance, cfg.backup.keep_count)
}
#[tauri::command]
fn list_backups(id: String) -> Result<Vec<backup::BackupEntryDto>, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    backup::list_backups(&instance)
}
#[tauri::command]
fn restore_backup(id: String, backup_name: String) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    backup::restore_backup(&instance, &backup_name)
}
#[tauri::command]
fn read_instance_config(id: String) -> Result<InstanceConfigDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let cfg = load_hydrated_config(&instance, &id)?;
    Ok(to_dto(&cfg))
}
#[tauri::command]
fn write_instance_config(
    id: String,
    config: InstanceConfigDto,
    state: State<'_, Arc<Mutex<AppState>>>,
    discord: State<'_, Arc<DiscordRuntime>>,
) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let mut cfg = load_hydrated_config(&instance, &id)?;
    apply_dto_updates(&id, &mut cfg, &config)?;
    save_instance_config(&instance, &cfg)?;
    if paths::detect_provider(&instance) == "palworld" {
        let new_pw = if config.rest_password.is_empty() {
            None
        } else {
            Some(config.rest_password.as_str())
        };
        palworld_settings::sync_ini_from_config(&instance, &cfg, new_pw)?;
    }
    {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        if cfg.crash_restart_enabled {
            g.crash_restart.insert(id.clone(), true);
        } else {
            g.crash_restart.remove(&id);
        }
    }
    discord.apply(
        &id,
        &cfg.discord,
        &cfg.rest_base_url,
        &cfg.rest_username,
        &cfg.rest_password,
        Arc::clone(&state),
    )?;
    Ok(())
}
#[tauri::command]
fn apply_discord_integration(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
    discord: State<'_, Arc<DiscordRuntime>>,
) -> Result<bool, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let cfg = load_hydrated_config(&instance, &id)?;
    discord.apply(
        &id,
        &cfg.discord,
        &cfg.rest_base_url,
        &cfg.rest_username,
        &cfg.rest_password,
        Arc::clone(&state),
    )?;
    Ok(discord.status(&id))
}
#[tauri::command]
fn stop_discord_integration(
    id: String,
    discord: State<'_, Arc<DiscordRuntime>>,
) -> Result<(), String> {
    discord.stop(&id);
    Ok(())
}
#[tauri::command]
fn discord_integration_status(
    id: String,
    discord: State<'_, Arc<DiscordRuntime>>,
) -> Result<bool, String> {
    Ok(discord.status(&id))
}
#[tauri::command]
fn set_crash_restart(
    id: String,
    enabled: bool,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let mut g = state.lock().map_err(|e| e.to_string())?;
    if enabled {
        g.crash_restart.insert(id, true);
    } else {
        g.crash_restart.remove(&id);
    }
    Ok(())
}
#[tauri::command]
fn read_log_tail(id: String, max_bytes: usize) -> Result<String, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let log = instance.join(".asylum").join("process.log");
    if !log.exists() {
        return Ok(String::new());
    }
    let data = std::fs::read(&log).map_err(|e| e.to_string())?;
    let start = data.len().saturating_sub(max_bytes.max(1024));
    Ok(String::from_utf8_lossy(&data[start..]).into_owned())
}
#[tauri::command]
fn rest_get_players(id: String) -> Result<Vec<serde_json::Value>, String> {
    rest_ops::get_players(&id)
}
#[tauri::command]
fn rest_get_info(id: String) -> Result<serde_json::Value, String> {
    rest_ops::get_info(&id)
}
#[tauri::command]
fn rest_get_metrics(id: String) -> Result<serde_json::Value, String> {
    rest_ops::get_metrics(&id)
}
#[tauri::command]
fn rest_announce(id: String, message: String) -> Result<(), String> {
    rest_ops::announce(&id, &message)
}
#[tauri::command]
fn rest_save(id: String) -> Result<(), String> {
    rest_ops::save(&id)
}
#[tauri::command]
fn rest_kick(id: String, userid: String, message: String) -> Result<(), String> {
    rest_ops::kick(&id, &userid, &message)
}
#[tauri::command]
fn rest_ban(id: String, userid: String, message: String) -> Result<(), String> {
    rest_ops::ban(&id, &userid, &message)
}
#[tauri::command]
fn rest_unban(id: String, userid: String) -> Result<(), String> {
    rest_ops::unban(&id, &userid)
}
#[tauri::command]
fn rest_get_settings(id: String) -> Result<serde_json::Value, String> {
    rest_ops::get_settings(&id)
}
#[tauri::command]
fn rest_shutdown(
    id: String,
    waittime: i64,
    message: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        let _ = g.stop_intentional(&id);
    }
    rest_ops::shutdown(&id, waittime, &message)
}
#[tauri::command]
fn rest_stop_api(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        let _ = g.stop_intentional(&id);
    }
    rest_ops::stop(&id)
}
#[tauri::command]
fn save_parser_status(id: String) -> Result<save_parser::SaveParseDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    Ok(save_parser::parse_instance_save(&instance))
}
#[tauri::command]
fn dashboard_metrics(id: String) -> Result<serde_json::Value, String> {
    let metrics = rest_ops::get_metrics(&id).unwrap_or(serde_json::json!({}));
    let info = rest_ops::get_info(&id).unwrap_or(serde_json::json!({}));
    Ok(serde_json::json!({
        "metrics": metrics,
        "info": info,
    }))
}

#[tauri::command]
fn list_running_server_ids(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<Vec<String>, String> {
    let mut g = state.lock().map_err(|e| e.to_string())?;
    Ok(g.running_server_ids())
}

/// 管理中の稼働サーバーをすべて停止してからアプリ終了できるようにする
#[tauri::command]
fn shutdown_all_running_servers(
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<(), String> {
    let root = paths::app_root()?;
    let ids = {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.root = Some(root.clone());
        g.running_server_ids()
    };
    for id in ids {
        let instance = paths::instance_dir(&root, &id)?;
        let provider = paths::detect_provider(&instance);
        let had_child = {
            let g = state.lock().map_err(|e| e.to_string())?;
            g.pid_of(&id).is_some()
        };
        if had_child && provider == "palworld" {
            let _ = rest_ops::stop(&id);
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(500));
                let mut g = state.lock().map_err(|e| e.to_string())?;
                if !g.is_running(&id) {
                    break;
                }
            }
        }
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.stop_intentional(&id)?;
    }
    Ok(())
}

#[tauri::command]
fn get_host_resources() -> Result<host_resources::HostResourcesDto, String> {
    Ok(host_resources::snapshot())
}

#[tauri::command]
fn get_server_banner(id: String) -> Result<game_art::BannerDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let provider = paths::detect_provider(&instance);
    game_art::resolve_banner(&instance, provider)
}

#[tauri::command]
fn steamcmd_status() -> Result<serde_json::Value, String> {
    let root = paths::app_root()?;
    Ok(serde_json::json!({
        "installed": paths::steamcmd_installed(&root),
        "path": root.join("tools").join("steamcmd").to_string_lossy(),
    }))
}

#[tauri::command]
fn ensure_steamcmd_cmd() -> Result<String, String> {
    let root = paths::app_root()?;
    let tools = root.join("tools").join("steamcmd");
    steamcmd::ensure_steamcmd(&tools).map(|p| p.to_string_lossy().into_owned())
}

#[tauri::command]
fn suggest_minecraft_type(id: String) -> Result<String, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    Ok(paths::suggest_minecraft_server_type(&instance).into())
}

#[tauri::command]
fn read_palworld_settings(id: String, state: State<'_, Arc<Mutex<AppState>>>) -> Result<palworld_settings::PalworldSettingsDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let running = {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.is_running(&id)
    };
    palworld_settings::read_settings(&instance, running)
}

#[tauri::command]
fn write_palworld_settings(id: String, raw: String) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    palworld_settings::write_settings(&instance, &raw)?;
    let mut cfg = load_hydrated_config(&instance, &id)?;
    let fields = palworld_settings::parse_option_settings(&raw);
    if palworld_settings::apply_ini_fields_to_config(&id, &mut cfg, &fields)? {
        save_instance_config(&instance, &cfg)?;
    }
    Ok(())
}

#[tauri::command]
fn read_palworld_rest_settings(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<palworld_settings::PalworldRestSettingsDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let cfg = load_hydrated_config(&instance, &id)?;
    let running = {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.is_running(&id)
    };
    palworld_settings::read_rest_settings(&instance, running, &cfg.rest_username)
}

#[tauri::command]
fn write_palworld_rest_settings(
    id: String,
    settings: palworld_settings::PalworldRestSettingsWriteDto,
) -> Result<InstanceConfigDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let mut cfg = load_hydrated_config(&instance, &id)?;
    palworld_settings::write_rest_settings(&instance, &id, &mut cfg, &settings)?;
    save_instance_config(&instance, &cfg)?;
    Ok(to_dto(&cfg))
}

#[tauri::command]
fn sync_palworld_from_ini(id: String) -> Result<InstanceConfigDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    if paths::detect_provider(&instance) != "palworld" {
        return Err("not a palworld instance".into());
    }
    let mut cfg = config::load_instance_config(&instance);
    if let Some(p) = secrets::get_rest_password(&id)? {
        cfg.rest_password = p;
    }
    if let Some(t) = secrets::get_discord_token(&id)? {
        cfg.discord.token = t;
    }
    if palworld_settings::sync_config_from_ini(&instance, &id, &mut cfg)? {
        save_instance_config(&instance, &cfg)?;
    }
    let cfg = load_hydrated_config(&instance, &id)?;
    Ok(to_dto(&cfg))
}

#[tauri::command]
fn read_server_properties(id: String) -> Result<server_properties::ServerPropertiesDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    server_properties::read_properties(&instance)
}

#[tauri::command]
fn write_server_properties(id: String, raw: String) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    server_properties::write_properties(&instance, &raw)
}

#[tauri::command]
fn read_minecraft_rcon_settings(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<minecraft_rcon_ops::MinecraftRconSettingsDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    let running = {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.is_running(&id)
    };
    minecraft_rcon_ops::read_rcon_settings(&instance, running)
}

#[tauri::command]
fn write_minecraft_rcon_settings(
    id: String,
    settings: minecraft_rcon_ops::MinecraftRconSettingsWriteDto,
) -> Result<(), String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    minecraft_rcon_ops::write_rcon_settings(&instance, &settings)
}

#[tauri::command]
fn sync_minecraft_rcon_from_properties(
    id: String,
    state: State<'_, Arc<Mutex<AppState>>>,
) -> Result<minecraft_rcon_ops::MinecraftRconSettingsDto, String> {
    let root = paths::app_root()?;
    let instance = paths::instance_dir(&root, &id)?;
    if !paths::is_minecraft_java(&instance) {
        return Err("RCON sync requires a Minecraft Java server folder".into());
    }
    let running = {
        let mut g = state.lock().map_err(|e| e.to_string())?;
        g.is_running(&id)
    };
    minecraft_rcon_ops::read_rcon_settings(&instance, running)
}

#[tauri::command]
fn minecraft_rcon_get_players(id: String) -> Result<Vec<serde_json::Value>, String> {
    minecraft_rcon_ops::get_players(&id)
}

#[tauri::command]
fn minecraft_rcon_get_info(id: String) -> Result<serde_json::Value, String> {
    minecraft_rcon_ops::get_info(&id)
}

#[tauri::command]
fn minecraft_rcon_get_metrics(id: String) -> Result<serde_json::Value, String> {
    minecraft_rcon_ops::get_metrics(&id)
}

#[tauri::command]
fn minecraft_rcon_announce(id: String, message: String) -> Result<(), String> {
    minecraft_rcon_ops::announce(&id, &message)
}

#[tauri::command]
fn minecraft_rcon_save(id: String) -> Result<(), String> {
    minecraft_rcon_ops::save_world(&id)
}

#[tauri::command]
fn minecraft_rcon_kick(id: String, player: String, message: String) -> Result<(), String> {
    minecraft_rcon_ops::kick(&id, &player, &message)
}

#[tauri::command]
fn minecraft_rcon_ban(id: String, player: String, message: String) -> Result<(), String> {
    minecraft_rcon_ops::ban(&id, &player, &message)
}

#[tauri::command]
fn minecraft_rcon_unban(id: String, player: String) -> Result<(), String> {
    minecraft_rcon_ops::unban(&id, &player)
}

#[tauri::command]
fn minecraft_rcon_command(id: String, command: String) -> Result<String, String> {
    minecraft_rcon_ops::command(&id, &command)
}
fn spawn_servers_watch(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_sig = String::new();
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3));
            let Ok(root) = paths::app_root() else {
                continue;
            };
            let servers = paths::servers_dir(&root);
            let mut names = Vec::new();
            if let Ok(rd) = std::fs::read_dir(&servers) {
                for e in rd.flatten() {
                    if e.path().is_dir() {
                        names.push(e.file_name().to_string_lossy().into_owned());
                    }
                }
            }
            names.sort();
            let sig = names.join("|");
            if sig != last_sig {
                last_sig = sig;
                let _ = app.emit("servers-changed", ());
            }
        }
    });
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = Arc::new(Mutex::new(AppState::default()));
    let discord = Arc::new(DiscordRuntime::default());
    spawn_crash_monitor(Arc::clone(&state));
    scheduler::spawn_scheduler(Arc::clone(&state));
    let state_setup = Arc::clone(&state);
    let discord_setup = Arc::clone(&discord);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Arc::clone(&state))
        .manage(Arc::clone(&discord))
        .setup(move |_app| {
            spawn_servers_watch(_app.handle().clone());
            // 起動時に既存設定の Discord を復元
            if let Ok(root) = paths::app_root() {
                let servers = paths::servers_dir(&root);
                if let Ok(rd) = std::fs::read_dir(servers) {
                    for e in rd.flatten() {
                        let path = e.path();
                        if !path.is_dir() {
                            continue;
                        }
                        let id = e.file_name().to_string_lossy().into_owned();
                        let cfg = match load_hydrated_config(&path, &id) {
                            Ok(c) => c,
                            Err(e) => {
                                eprintln!("{}", crate::secret_util::redact_text(&e));
                                continue;
                            }
                        };
                        if cfg.crash_restart_enabled {
                            if let Ok(mut g) = state_setup.lock() {
                                g.crash_restart.insert(id.clone(), true);
                            }
                        }
                        if cfg.discord.enabled {
                            let _ = discord_setup.apply(
                                &id,
                                &cfg.discord,
                                &cfg.rest_base_url,
                                &cfg.rest_username,
                                &cfg.rest_password,
                                Arc::clone(&state_setup),
                            );
                        }
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ensure_servers_layout,
            get_app_root,
            list_server_instances,
            start_server,
            stop_server,
            restart_server,
            install_palworld,
            update_palworld,
            check_palworld_update,
            create_backup,
            list_backups,
            restore_backup,
            read_instance_config,
            write_instance_config,
            apply_discord_integration,
            stop_discord_integration,
            discord_integration_status,
            set_crash_restart,
            read_log_tail,
            rest_get_players,
            rest_get_info,
            rest_get_metrics,
            rest_announce,
            rest_save,
            rest_kick,
            rest_ban,
            rest_unban,
            rest_get_settings,
            rest_shutdown,
            rest_stop_api,
            dashboard_metrics,
            save_parser_status,
            get_host_resources,
            get_server_banner,
            steamcmd_status,
            ensure_steamcmd_cmd,
            suggest_minecraft_type,
            read_palworld_settings,
            write_palworld_settings,
            read_palworld_rest_settings,
            write_palworld_rest_settings,
            sync_palworld_from_ini,
            read_server_properties,
            write_server_properties,
            read_minecraft_rcon_settings,
            write_minecraft_rcon_settings,
            sync_minecraft_rcon_from_properties,
            minecraft_rcon_get_players,
            minecraft_rcon_get_info,
            minecraft_rcon_get_metrics,
            minecraft_rcon_announce,
            minecraft_rcon_save,
            minecraft_rcon_kick,
            minecraft_rcon_ban,
            minecraft_rcon_unban,
            minecraft_rcon_command,
            list_running_server_ids,
            shutdown_all_running_servers,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
