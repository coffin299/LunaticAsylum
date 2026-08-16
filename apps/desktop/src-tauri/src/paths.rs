use std::path::{Path, PathBuf};

pub const HOW_TO_ADD_SERVERS: &str = include_str!("../resources/HOW_TO_ADD_SERVERS.txt");

pub fn app_root() -> Result<PathBuf, String> {
    if cfg!(debug_assertions) {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        return Ok(manifest
            .join("../../..")
            .canonicalize()
            .map_err(|e| e.to_string())?);
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    exe.parent()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| "failed to resolve exe directory".into())
}

pub fn servers_dir(root: &Path) -> PathBuf {
    root.join("Servers")
}

pub fn instance_dir(root: &Path, id: &str) -> Result<PathBuf, String> {
    crate::validate::validate_instance_id(id)?;
    let servers = servers_dir(root);
    let path = servers.join(id);
    if !path.is_dir() {
        return Err("instance not found".into());
    }
    crate::validate::ensure_within(&servers, &path)
}

pub fn detect_provider(instance_path: &Path) -> &'static str {
    if find_palserver_exe(instance_path).is_some() {
        return "palworld";
    }
    let markers = [
        "bedrock_server.exe",
        "paper.jar",
        "spigot.jar",
        "server.jar",
    ];
    for m in markers {
        if instance_path.join(m).exists() {
            return "minecraft";
        }
    }
    "unknown"
}

pub fn find_palserver_exe(instance_path: &Path) -> Option<PathBuf> {
    let candidates = [
        instance_path.join("PalServer.exe"),
        instance_path.join("Pal/Binaries/Win64/PalServer-Win64-Shipping.exe"),
        instance_path.join("PalServer.sh"),
    ];
    candidates.into_iter().find(|p| p.exists())
}
