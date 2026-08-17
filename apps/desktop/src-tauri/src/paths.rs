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
    if find_minecraft_jar(instance_path).is_some() || instance_path.join("bedrock_server.exe").exists()
    {
        return "minecraft";
    }
    "unknown"
}

pub fn is_launchable(instance_path: &Path) -> bool {
    detect_provider(instance_path) != "unknown"
}

pub fn is_minecraft_java(instance_path: &Path) -> bool {
    find_minecraft_jar(instance_path).is_some()
}

pub fn find_minecraft_jar(instance_path: &Path) -> Option<PathBuf> {
    let markers = [
        "paper.jar",
        "spigot.jar",
        "purpur.jar",
        "server.jar",
        "fabric-server-launch.jar",
    ];
    for m in markers {
        let p = instance_path.join(m);
        if p.is_file() {
            return Some(p);
        }
    }
    let Ok(rd) = std::fs::read_dir(instance_path) else {
        return None;
    };
    let mut jars: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("jar"))
        .collect();
    jars.sort();
    jars.into_iter().next()
}

/// jar 名から UI 推奨種別（自動保存しない）
pub fn suggest_minecraft_server_type(instance_path: &Path) -> &'static str {
    if instance_path.join("paper.jar").exists() {
        return "paper";
    }
    if instance_path.join("spigot.jar").exists() {
        return "spigot";
    }
    if instance_path.join("purpur.jar").exists() {
        return "purpur";
    }
    if instance_path.join("fabric-server-launch.jar").exists() {
        return "fabric";
    }
    if instance_path.join("server.jar").exists() {
        return "vanilla";
    }
    "unknown"
}

pub fn steamcmd_installed(root: &Path) -> bool {
    root.join("tools")
        .join("steamcmd")
        .join(if cfg!(windows) {
            "steamcmd.exe"
        } else {
            "steamcmd.sh"
        })
        .is_file()
}

pub fn find_palserver_exe(instance_path: &Path) -> Option<PathBuf> {
    let candidates = [
        instance_path.join("PalServer.exe"),
        instance_path.join("Pal/Binaries/Win64/PalServer-Win64-Shipping.exe"),
        instance_path.join("PalServer.sh"),
    ];
    candidates.into_iter().find(|p| p.exists())
}
