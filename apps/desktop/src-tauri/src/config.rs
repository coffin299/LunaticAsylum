//! インスタンス設定（`.asylum/config.json`）

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupConfig {
    pub enabled: bool,
    pub interval_value: u64,
    pub interval_unit: String,
    pub keep_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheckConfig {
    pub polling_enabled: bool,
    pub interval_minutes: u64,
    pub auto_apply: bool,
    pub auto_apply_only_when_empty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordNotifyConfig {
    pub join_leave: bool,
    pub rest_status: bool,
    pub topic: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordConfig {
    pub enabled: bool,
    pub token: String,
    pub guild_id: String,
    pub channel_id: String,
    /// カンマ区切り Discord ユーザー ID
    pub admin_ids: String,
    pub poll_interval_seconds: u64,
    pub topic_template: String,
    pub notify: DiscordNotifyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArtConfig {
    pub banner_path: String,
}

impl Default for ArtConfig {
    fn default() -> Self {
        Self {
            banner_path: String::new(),
        }
    }
}

fn default_rest_api_enabled() -> bool {
    true
}

fn default_rest_api_port() -> u16 {
    crate::palworld_settings::DEFAULT_REST_API_PORT
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MinecraftConfig {
    pub server_type: String,
    pub jar_file: String,
    pub jvm_args: String,
    pub server_args: String,
}

impl Default for MinecraftConfig {
    fn default() -> Self {
        Self {
            server_type: "unknown".into(),
            jar_file: String::new(),
            jvm_args: "-Xms2G -Xmx4G".into(),
            server_args: "nogui".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfig {
    pub rest_base_url: String,
    pub rest_username: String,
    pub rest_password: String,
    #[serde(default = "default_rest_api_enabled")]
    pub rest_api_enabled: bool,
    #[serde(default = "default_rest_api_port")]
    pub rest_api_port: u16,
    pub backup: BackupConfig,
    pub crash_restart_enabled: bool,
    pub update_check: UpdateCheckConfig,
    pub discord: DiscordConfig,
    #[serde(default)]
    pub art: ArtConfig,
    #[serde(default)]
    pub minecraft: MinecraftConfig,
}

impl Default for InstanceConfig {
    fn default() -> Self {
        Self {
            rest_base_url: crate::palworld_settings::rest_base_url_from_port(
                crate::palworld_settings::DEFAULT_REST_API_PORT,
            ),
            rest_username: "admin".into(),
            rest_password: String::new(),
            rest_api_enabled: true,
            rest_api_port: crate::palworld_settings::DEFAULT_REST_API_PORT,
            backup: BackupConfig {
                enabled: false,
                interval_value: 6,
                interval_unit: "hours".into(),
                keep_count: 5,
            },
            crash_restart_enabled: false,
            update_check: UpdateCheckConfig {
                polling_enabled: true,
                interval_minutes: 180,
                auto_apply: false,
                auto_apply_only_when_empty: true,
            },
            discord: DiscordConfig {
                enabled: false,
                token: String::new(),
                guild_id: String::new(),
                channel_id: String::new(),
                admin_ids: String::new(),
                poll_interval_seconds: 15,
                topic_template: "Online: {current}/{max}".into(),
                notify: DiscordNotifyConfig {
                    join_leave: true,
                    rest_status: true,
                    topic: true,
                },
            },
            art: ArtConfig::default(),
            minecraft: MinecraftConfig::default(),
        }
    }
}

impl InstanceConfig {
    pub fn from_value(v: &Value) -> Self {
        let mut cfg = Self::default();
        if let Some(s) = v.get("restBaseUrl").and_then(|x| x.as_str()) {
            cfg.rest_base_url = s.to_string();
        }
        if let Some(s) = v.get("restUsername").and_then(|x| x.as_str()) {
            cfg.rest_username = s.to_string();
        }
        if let Some(s) = v.get("restPassword").and_then(|x| x.as_str()) {
            cfg.rest_password = s.to_string();
        }
        if let Some(x) = v.get("restApiEnabled").and_then(|x| x.as_bool()) {
            cfg.rest_api_enabled = x;
        }
        if let Some(x) = v.get("restApiPort").and_then(|x| x.as_u64()) {
            if x > 0 && x <= u16::MAX as u64 {
                cfg.rest_api_port = x as u16;
            }
        }
        if let Some(b) = v.get("backup") {
            if let Some(x) = b.get("enabled").and_then(|x| x.as_bool()) {
                cfg.backup.enabled = x;
            }
            if let Some(x) = b.get("intervalValue").and_then(|x| x.as_u64()) {
                cfg.backup.interval_value = x.max(1);
            }
            if let Some(x) = b.get("intervalUnit").and_then(|x| x.as_str()) {
                cfg.backup.interval_unit = x.to_string();
            }
            if let Some(x) = b.get("keepCount").and_then(|x| x.as_u64()) {
                cfg.backup.keep_count = x as usize;
            }
        }
        if let Some(x) = v.get("crashRestartEnabled").and_then(|x| x.as_bool()) {
            cfg.crash_restart_enabled = x;
        }
        // 旧フラットキー互換
        if let Some(x) = v.get("updateCheckEnabled").and_then(|x| x.as_bool()) {
            cfg.update_check.polling_enabled = x;
        }
        if let Some(x) = v.get("updateAutoApply").and_then(|x| x.as_bool()) {
            cfg.update_check.auto_apply = x;
        }
        if let Some(u) = v.get("updateCheck") {
            if let Some(x) = u.get("pollingEnabled").and_then(|x| x.as_bool()) {
                cfg.update_check.polling_enabled = x;
            }
            if let Some(x) = u.get("intervalMinutes").and_then(|x| x.as_u64()) {
                cfg.update_check.interval_minutes = x.max(15);
            }
            if let Some(x) = u.get("autoApply").and_then(|x| x.as_bool()) {
                cfg.update_check.auto_apply = x;
            }
            if let Some(x) = u.get("autoApplyOnlyWhenEmpty").and_then(|x| x.as_bool()) {
                cfg.update_check.auto_apply_only_when_empty = x;
            }
        }
        if let Some(x) = v.get("discordEnabled").and_then(|x| x.as_bool()) {
            cfg.discord.enabled = x;
        }
        if let Some(d) = v.get("discord") {
            if let Some(x) = d.get("enabled").and_then(|x| x.as_bool()) {
                cfg.discord.enabled = x;
            }
            if let Some(x) = d.get("token").and_then(|x| x.as_str()) {
                cfg.discord.token = x.to_string();
            }
            if let Some(x) = d.get("guildId").and_then(|x| x.as_str()) {
                cfg.discord.guild_id = x.to_string();
            }
            if let Some(x) = d.get("channelId").and_then(|x| x.as_str()) {
                cfg.discord.channel_id = x.to_string();
            }
            if let Some(x) = d.get("adminIds").and_then(|x| x.as_str()) {
                cfg.discord.admin_ids = x.to_string();
            } else if let Some(arr) = d.get("adminIds").and_then(|x| x.as_array()) {
                cfg.discord.admin_ids = arr
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
                        v.as_u64().map(|n| n.to_string())
                    }))
                    .collect::<Vec<_>>()
                    .join(",");
            }
            if let Some(x) = d.get("pollIntervalSeconds").and_then(|x| x.as_u64()) {
                cfg.discord.poll_interval_seconds = x.max(5);
            }
            if let Some(x) = d.get("topicTemplate").and_then(|x| x.as_str()) {
                cfg.discord.topic_template = x.to_string();
            }
            if let Some(n) = d.get("notify") {
                if let Some(x) = n.get("joinLeave").and_then(|x| x.as_bool()) {
                    cfg.discord.notify.join_leave = x;
                }
                if let Some(x) = n.get("restStatus").and_then(|x| x.as_bool()) {
                    cfg.discord.notify.rest_status = x;
                }
                if let Some(x) = n.get("topic").and_then(|x| x.as_bool()) {
                    cfg.discord.notify.topic = x;
                }
            }
        }
        if let Some(a) = v.get("art") {
            if let Some(x) = a.get("bannerPath").and_then(|x| x.as_str()) {
                cfg.art.banner_path = x.to_string();
            }
        }
        if let Some(m) = v.get("minecraft") {
            if let Some(x) = m.get("serverType").and_then(|x| x.as_str()) {
                cfg.minecraft.server_type = x.to_string();
            }
            if let Some(x) = m.get("jarFile").and_then(|x| x.as_str()) {
                cfg.minecraft.jar_file = x.to_string();
            }
            if let Some(x) = m.get("jvmArgs").and_then(|x| x.as_str()) {
                cfg.minecraft.jvm_args = x.to_string();
            }
            if let Some(x) = m.get("serverArgs").and_then(|x| x.as_str()) {
                cfg.minecraft.server_args = x.to_string();
            }
        }
        cfg
    }

    pub fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }

    pub fn backup_interval_secs(&self) -> u64 {
        let n = self.backup.interval_value.max(1);
        match self.backup.interval_unit.as_str() {
            "minutes" => n * 60,
            "days" => n * 86_400,
            _ => n * 3_600,
        }
    }

    pub fn parse_admin_ids(&self) -> Vec<u64> {
        self.discord
            .admin_ids
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect()
    }

    /// ディスク保存用（秘密フィールドは空）
    pub fn to_disk_value(&self) -> Value {
        let mut v = self.to_value();
        if let Some(obj) = v.as_object_mut() {
            obj.insert("restPassword".into(), Value::String(String::new()));
            if let Some(d) = obj.get_mut("discord").and_then(|x| x.as_object_mut()) {
                d.insert("token".into(), Value::String(String::new()));
            }
        }
        v
    }
}

/// UI 向け（秘密の中身は返さない）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceConfigDto {
    pub rest_base_url: String,
    pub rest_username: String,
    /// 常に空。変更時のみ write で送る
    pub rest_password: String,
    pub rest_password_set: bool,
    pub rest_api_enabled: bool,
    pub rest_api_port: u16,
    pub backup: BackupConfig,
    pub crash_restart_enabled: bool,
    pub update_check: UpdateCheckConfig,
    pub discord: DiscordConfigDto,
    pub art: ArtConfig,
    pub minecraft: MinecraftConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordConfigDto {
    pub enabled: bool,
    pub token: String,
    pub token_set: bool,
    pub guild_id: String,
    pub channel_id: String,
    pub admin_ids: String,
    pub poll_interval_seconds: u64,
    pub topic_template: String,
    pub notify: DiscordNotifyConfig,
}

pub fn load_instance_config(instance: &std::path::Path) -> InstanceConfig {
    let path = instance.join(".asylum").join("config.json");
    if !path.exists() {
        return InstanceConfig::default();
    }
    let Ok(text) = std::fs::read_to_string(&path) else {
        return InstanceConfig::default();
    };
    let Ok(v) = serde_json::from_str::<Value>(&text) else {
        return InstanceConfig::default();
    };
    InstanceConfig::from_value(&v)
}

/// 平文 config からの移行 + keyring からのハイドレート
pub fn load_hydrated_config(
    instance: &std::path::Path,
    instance_id: &str,
) -> Result<InstanceConfig, String> {
    let mut cfg = load_instance_config(instance);
    let mut migrated = false;

    if !cfg.discord.token.is_empty() {
        crate::secrets::set_discord_token(instance_id, &cfg.discord.token)?;
        cfg.discord.token.clear();
        migrated = true;
    }
    if !cfg.rest_password.is_empty() {
        crate::secrets::set_rest_password(instance_id, &cfg.rest_password)?;
        cfg.rest_password.clear();
        migrated = true;
    }
    if migrated {
        save_instance_config(instance, &cfg)?;
    }

    if let Some(t) = crate::secrets::get_discord_token(instance_id)? {
        cfg.discord.token = t;
    }
    if let Some(p) = crate::secrets::get_rest_password(instance_id)? {
        cfg.rest_password = p;
    }

    if crate::paths::detect_provider(instance) == "palworld" {
        let ini_changed = crate::palworld_settings::sync_config_from_ini(instance, instance_id, &mut cfg)?;
        if ini_changed {
            save_instance_config(instance, &cfg)?;
        }
    }

    Ok(cfg)
}

pub fn to_dto(cfg: &InstanceConfig) -> InstanceConfigDto {
    InstanceConfigDto {
        rest_base_url: cfg.rest_base_url.clone(),
        rest_username: cfg.rest_username.clone(),
        rest_password: String::new(),
        rest_password_set: !cfg.rest_password.is_empty(),
        rest_api_enabled: cfg.rest_api_enabled,
        rest_api_port: cfg.rest_api_port,
        backup: cfg.backup.clone(),
        crash_restart_enabled: cfg.crash_restart_enabled,
        update_check: cfg.update_check.clone(),
        discord: DiscordConfigDto {
            enabled: cfg.discord.enabled,
            token: String::new(),
            token_set: !cfg.discord.token.is_empty(),
            guild_id: cfg.discord.guild_id.clone(),
            channel_id: cfg.discord.channel_id.clone(),
            admin_ids: cfg.discord.admin_ids.clone(),
            poll_interval_seconds: cfg.discord.poll_interval_seconds,
            topic_template: cfg.discord.topic_template.clone(),
            notify: cfg.discord.notify.clone(),
        },
        art: cfg.art.clone(),
        minecraft: cfg.minecraft.clone(),
    }
}

/// DTO をマージ。空の token/password は「変更なし」。
pub fn apply_dto_updates(
    instance_id: &str,
    base: &mut InstanceConfig,
    dto: &InstanceConfigDto,
) -> Result<(), String> {
    if dto.rest_api_port == 0 {
        return Err("REST API port must be greater than 0".into());
    }
    base.rest_api_enabled = dto.rest_api_enabled;
    base.rest_api_port = dto.rest_api_port;
    base.rest_base_url =
        crate::palworld_settings::rest_base_url_from_port(dto.rest_api_port);
    crate::validate::validate_rest_base_url(&base.rest_base_url)?;
    base.rest_username = dto.rest_username.clone();
    base.backup = dto.backup.clone();
    base.crash_restart_enabled = dto.crash_restart_enabled;
    base.update_check = dto.update_check.clone();
    base.discord.enabled = dto.discord.enabled;
    base.discord.guild_id = dto.discord.guild_id.clone();
    base.discord.channel_id = dto.discord.channel_id.clone();
    base.discord.admin_ids = dto.discord.admin_ids.clone();
    base.discord.poll_interval_seconds = dto.discord.poll_interval_seconds.max(5);
    base.discord.topic_template = dto.discord.topic_template.clone();
    base.discord.notify = dto.discord.notify.clone();
    base.art = dto.art.clone();
    base.minecraft = dto.minecraft.clone();

    if !dto.rest_password.is_empty() {
        crate::secrets::set_rest_password(instance_id, &dto.rest_password)?;
        base.rest_password = dto.rest_password.clone();
    }
    if !dto.discord.token.is_empty() {
        crate::secrets::set_discord_token(instance_id, &dto.discord.token)?;
        base.discord.token = dto.discord.token.clone();
    }
    Ok(())
}

pub fn save_instance_config(instance: &std::path::Path, cfg: &InstanceConfig) -> Result<(), String> {
    let asylum = instance.join(".asylum");
    std::fs::create_dir_all(&asylum).map_err(|e| e.to_string())?;
    let text = serde_json::to_string_pretty(&cfg.to_disk_value()).map_err(|e| e.to_string())?;
    std::fs::write(asylum.join("config.json"), text).map_err(|e| e.to_string())
}
