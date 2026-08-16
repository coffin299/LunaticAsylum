//! 入力検証（パス・REST URL）

use std::path::{Component, Path, PathBuf};

/// インスタンス ID: フォルダ名として安全なものだけ。
pub fn validate_instance_id(id: &str) -> Result<(), String> {
    let id = id.trim();
    if id.is_empty() || id.len() > 128 {
        return Err("invalid instance id".into());
    }
    if id.starts_with('.') {
        return Err("invalid instance id".into());
    }
    if id.contains("..") || id.contains('/') || id.contains('\\') || id.contains('\0') {
        return Err("invalid instance id".into());
    }
    if Path::new(id).components().any(|c| !matches!(c, Component::Normal(_))) {
        return Err("invalid instance id".into());
    }
    Ok(())
}

/// `candidate` が `root` 配下に収まるか（canonicalize 後）。
pub fn ensure_within(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|e| format!("root resolve: {e}"))?;
    let cand = if candidate.exists() {
        candidate
            .canonicalize()
            .map_err(|e| format!("path resolve: {e}"))?
    } else {
        let parent = candidate
            .parent()
            .ok_or_else(|| "invalid path".to_string())?;
        let name = candidate
            .file_name()
            .ok_or_else(|| "invalid path".to_string())?;
        parent
            .canonicalize()
            .map_err(|e| format!("path resolve: {e}"))?
            .join(name)
    };
    if !cand.starts_with(&root) {
        return Err("path escapes allowed root".into());
    }
    Ok(cand)
}

/// Palworld REST Base URL。ローカル運用向けに http(s) + host 制限。
pub fn validate_rest_base_url(url: &str) -> Result<(), String> {
    let url = url.trim();
    if url.is_empty() || url.len() > 512 {
        return Err("invalid REST URL".into());
    }
    let lower = url.to_ascii_lowercase();
    if !(lower.starts_with("http://") || lower.starts_with("https://")) {
        return Err("REST URL must be http:// or https://".into());
    }
    if lower.starts_with("file:") || lower.contains('@') {
        return Err("unsupported REST URL".into());
    }
    // 簡易ホスト抽出
    let rest = url
        .split("://")
        .nth(1)
        .ok_or_else(|| "invalid REST URL".to_string())?;
    let hostport = rest.split('/').next().unwrap_or("");
    let host = hostport.split(':').next().unwrap_or("").to_ascii_lowercase();
    let allowed = host == "localhost"
        || host == "127.0.0.1"
        || host == "::1"
        || host == "[::1]";
    if !allowed {
        // 将来リモート管理するまでは loopback のみ
        return Err("REST URL host must be localhost / 127.0.0.1（当面）".into());
    }
    Ok(())
}

pub fn validate_backup_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 200 {
        return Err("invalid backup name".into());
    }
    if name.contains("..") || name.contains('/') || name.contains('\\') || name.contains('\0') {
        return Err("invalid backup name".into());
    }
    if !name.ends_with(".zip") {
        return Err("backup must be .zip".into());
    }
    Ok(())
}
