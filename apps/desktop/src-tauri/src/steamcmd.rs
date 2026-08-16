//! SteamCMD: サーバーのインストール・アップデート専用。
//! Build ID の取得には使わない（`+app_info_print` の stdout は環境によって期待どおり出ない）。

use std::fs::{self, File};
use std::io::copy;
use std::path::{Path, PathBuf};
use std::process::Command;

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
        return Err("steamcmd executable missing after install".into());
    }
    Ok(exe)
}

fn download_steamcmd(tools_dir: &Path) -> Result<(), String> {
    let url = "https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip";
    let zip_path = tools_dir.join("steamcmd.zip");
    let resp = ureq::get(url)
        .call()
        .map_err(|e| format!("steamcmd download failed: {e}"))?;
    let mut reader = resp.into_reader();
    let mut file = File::create(&zip_path).map_err(|e| e.to_string())?;
    copy(&mut reader, &mut file).map_err(|e| e.to_string())?;

    // PowerShell Expand-Archive（Windows）
    #[cfg(windows)]
    {
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    zip_path.display(),
                    tools_dir.display()
                ),
            ])
            .status()
            .map_err(|e| e.to_string())?;
        if !status.success() {
            return Err("failed to extract steamcmd.zip".into());
        }
    }
    #[cfg(not(windows))]
    {
        return Err("steamcmd bootstrap is Windows-only in this build".into());
    }
    let _ = fs::remove_file(zip_path);
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
