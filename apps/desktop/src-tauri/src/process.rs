//! 子プロセスツリー終了（PalServer.exe → Shipping.exe 対策）

use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// ルート PID とその子孫を終了する。
pub fn kill_process_tree(pid: u32) -> Result<(), String> {
    if pid == 0 {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("taskkill failed: {e}"))?;
        // 128 = プロセスが既に存在しない
        let code = status.code().unwrap_or(-1);
        if !status.success() && code != 128 {
            return Err(format!("taskkill exited with code {code}"));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("pkill")
            .args(["-TERM", "-P", &pid.to_string()])
            .status();
        let _ = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status();
        thread::sleep(Duration::from_millis(400));
        let _ = Command::new("pkill")
            .args(["-KILL", "-P", &pid.to_string()])
            .status();
        let _ = Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .status();
        Ok(())
    }
}

/// 管理対象外に残った Palworld プロセス（exe パスが instance 配下）を終了する。
pub fn kill_palworld_under_instance(instance: &Path) -> Result<(), String> {
    let canonical = instance
        .canonicalize()
        .unwrap_or_else(|_| instance.to_path_buf());
    let prefix = canonical.to_string_lossy().replace('\'', "''");
    #[cfg(windows)]
    {
        let script = format!(
            "Get-CimInstance Win32_Process | Where-Object {{ $_.ExecutablePath -and $_.ExecutablePath.StartsWith('{prefix}', [System.StringComparison]::OrdinalIgnoreCase) }} | ForEach-Object {{ Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }}"
        );
        let status = Command::new("powershell")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &script,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map_err(|e| format!("orphan cleanup failed: {e}"))?;
        if !status.success() {
            return Err(format!(
                "orphan cleanup exited with code {}",
                status.code().unwrap_or(-1)
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = prefix;
        Ok(())
    }
}
