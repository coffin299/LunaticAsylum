//! サーバーバナー画像の解決とキャッシュ

use crate::config::{load_instance_config, ArtConfig};
use crate::paths;
use crate::steamcmd::PALWORLD_APP_ID;
use serde::Serialize;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

const CACHE_NAME: &str = "banner.jpg";
const CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 3600);
const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BannerDto {
    pub path: Option<String>,
    pub source: String,
    pub transparent: bool,
}

pub fn resolve_banner(instance: &Path, provider_id: &str) -> Result<BannerDto, String> {
    let cfg = load_instance_config(instance);
    if let Some(user) = resolve_user_banner(instance, &cfg.art)? {
        return Ok(user);
    }

    if provider_id == "minecraft" || provider_id == "unknown" {
        return Ok(BannerDto {
            path: None,
            source: "transparent".into(),
            transparent: true,
        });
    }

    if provider_id == "palworld" {
        if let Some(steam) = resolve_steam_banner(instance)? {
            return Ok(steam);
        }
    }

    Ok(BannerDto {
        path: None,
        source: "transparent".into(),
        transparent: true,
    })
}

fn resolve_user_banner(instance: &Path, art: &ArtConfig) -> Result<Option<BannerDto>, String> {
    let rel = art.banner_path.trim();
    if rel.is_empty() {
        return Ok(None);
    }
    let root = paths::app_root().map_err(|e| e.to_string())?;
    let servers = paths::servers_dir(&root);
    let target = instance.join(rel);
    crate::validate::ensure_within(&servers, &target)?;
    if !target.is_file() {
        return Ok(None);
    }
    Ok(Some(BannerDto {
        path: Some(target.to_string_lossy().into_owned()),
        source: "user".into(),
        transparent: false,
    }))
}

fn cache_path(instance: &Path) -> PathBuf {
    instance.join(".asylum").join("cache").join(CACHE_NAME)
}

fn cache_fresh(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    let Ok(modified) = meta.modified() else {
        return false;
    };
    SystemTime::now()
        .duration_since(modified)
        .map(|d| d < CACHE_TTL)
        .unwrap_or(false)
}

fn resolve_steam_banner(instance: &Path) -> Result<Option<BannerDto>, String> {
    let cache = cache_path(instance);
    if cache_fresh(&cache) {
        return Ok(Some(BannerDto {
            path: Some(cache.to_string_lossy().into_owned()),
            source: "steam".into(),
            transparent: false,
        }));
    }

    let app_id = PALWORLD_APP_ID;
    let url = format!(
        "https://cdn.cloudflare.steamstatic.com/steam/apps/{app_id}/library_hero.jpg"
    );
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(15))
        .build();
    let resp = match agent.get(&url).call() {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };
    if resp.status() != 200 {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    resp.into_reader()
        .take((MAX_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_BYTES || !looks_like_image(&bytes) {
        return Ok(None);
    }

    let dir = cache.parent().unwrap();
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    fs::write(&cache, &bytes).map_err(|e| e.to_string())?;

    Ok(Some(BannerDto {
        path: Some(cache.to_string_lossy().into_owned()),
        source: "steam".into(),
        transparent: false,
    }))
}

fn looks_like_image(data: &[u8]) -> bool {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return true;
    }
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        return true;
    }
    data.len() >= 12 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP"
}
