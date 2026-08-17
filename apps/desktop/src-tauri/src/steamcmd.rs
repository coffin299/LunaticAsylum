//! SteamCMD: サーバーのインストール・アップデート専用。
//! Build ID の取得には使わない（`+app_info_print` の stdout は環境によって期待どおり出ない）。

use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::process::Command;

use zip::ZipArchive;

/// Palworld Dedicated Server
pub const PALWORLD_APP_ID: u32 = 2394010;

pub fn ensure_steamcmd(tools_dir: &Path) -> Result<PathBuf, String> {
    let exe = if cfg!(windows) {
        tools_dir.join("steamcmd.exe")
    } else {
        tools_dir.join("steamcmd.sh")
    };
    if exe.exists() {
        return Ok(exe);
    }
    fs::create_dir_all(tools_dir).map_err(|e| e.to_string())?;
    download_steamcmd(tools_dir)?;
    if !exe.exists() {
        let found = list_dir_names(tools_dir);
        return Err(format!(
            "steamcmd executable missing after install (expected {}, found: {found:?})",
            exe.display()
        ));
    }
    Ok(exe)
}

fn list_dir_names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn download_steamcmd(tools_dir: &Path) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = tools_dir;
        return Err("steamcmd bootstrap is Windows-only in this build".into());
    }

    let url = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
    let zip_path = tools_dir.join("steamcmd.zip");
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("steamcmd download failed: {e}"))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(&zip_path).map_err(|e| e.to_string())?;
    copy(&mut reader, &mut file).map_err(|e| e.to_string())?;

    let size = fs::metadata(&zip_path)
        .map_err(|e| e.to_string())?
        .len();
    if size < 100_000 {
        let _ = fs::remove_file(&zip_path);
        return Err(format!("steamcmd download too small ({size} bytes)"));
    }

    extract_steamcmd_zip(&zip_path, tools_dir)?;
    let _ = fs::remove_file(zip_path);
    Ok(())
}

fn extract_steamcmd_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("invalid steamcmd.zip: {e}"))?;

    const MAX_FILES: usize = 100;
    const MAX_UNCOMPRESSED: u64 = 50 * 1024 * 1024;

    if archive.len() > MAX_FILES {
        return Err("steamcmd.zip has too many entries".into());
    }

    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        total_uncompressed = total_uncompressed.saturating_add(entry.size());
        if total_uncompressed > MAX_UNCOMPRESSED {
            return Err("steamcmd.zip uncompressed size too large".into());
        }
        let rel = entry
            .enclosed_name()
            .ok_or_else(|| "steamcmd.zip entry path rejected".to_string())?;
        for component in rel.components() {
            if matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::RootDir
            ) {
                return Err("steamcmd.zip entry path rejected".into());
            }
        }
        let outpath = dest.join(rel);
        if entry.is_dir() {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

pub fn app_update(
    steamcmd: &Path,
    install_dir: &Path,
    app_id: u32,
    validate: bool,
) -> Result<(), String> {
    let script = install_dir.join(".asylum").join("steamcmd_script.txt");
    if let Some(parent) = script.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let validate_flag = if validate { " validate" } else { "" };
    let body = format!(
        "@ShutdownOnFailedCommand 1\n@NoPromptForPassword 1\nforce_install_dir \"{}\"\nlogin anonymous\napp_update {app_id}{validate_flag}\nquit\n",
        install_dir.display()
    );
    fs::write(&script, body).map_err(|e| e.to_string())?;

    let output = Command::new(steamcmd)
        .arg("+runscript")
        .arg(&script)
        .current_dir(steamcmd.parent().unwrap_or(Path::new(".")))
        .output()
        .map_err(|e| e.to_string())?;

    let log = install_dir.join(".asylum").join("steamcmd.log");
    let mut combined = Vec::new();
    combined.extend_from_slice(&output.stdout);
    combined.extend_from_slice(b"\n");
    combined.extend_from_slice(&output.stderr);
    let _ = fs::write(log, &combined);

    if !output.status.success() {
        return Err(format!(
            "steamcmd failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
