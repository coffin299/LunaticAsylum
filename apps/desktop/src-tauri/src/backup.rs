use chrono::Local;
use serde::Serialize;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupEntryDto {
    pub name: String,
    pub path: String,
    pub created_at: String,
}

fn backups_dir(instance: &Path) -> PathBuf {
    instance.join(".asylum").join("backups")
}

pub fn create_backup(instance: &Path, keep_count: usize) -> Result<BackupEntryDto, String> {
    // 可能なら事前に REST save（instance id は親フォルダ名）
    if let Some(id) = instance.file_name().and_then(|s| s.to_str()) {
        let _ = crate::rest_ops::try_save(id);
        let _ = crate::minecraft_rcon_ops::try_save(id);
    }
    let saved = instance.join("Pal").join("Saved");
    if !saved.is_dir() {
        return Err("Pal/Saved not found".into());
    }
    let dir = backups_dir(instance);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let stamp = Local::now().format("%Y%m%d-%H%M%S").to_string();
    let name = format!("backup-{stamp}.zip");
    let zip_path = dir.join(&name);

    let file = File::create(&zip_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for entry in WalkDir::new(&saved) {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            continue;
        }
        let rel = path
            .strip_prefix(instance)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .replace('\\', "/");
        zip.start_file(rel, options).map_err(|e| e.to_string())?;
        let mut f = File::open(path).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        zip.write_all(&buf).map_err(|e| e.to_string())?;
    }
    zip.finish().map_err(|e| e.to_string())?;

    trim_backups(instance, keep_count.max(1))?;

    Ok(BackupEntryDto {
        name: name.clone(),
        path: zip_path.to_string_lossy().into_owned(),
        created_at: stamp,
    })
}

pub fn list_backups(instance: &Path) -> Result<Vec<BackupEntryDto>, String> {
    let dir = backups_dir(instance);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("zip") {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let created_at = name
            .trim_start_matches("backup-")
            .trim_end_matches(".zip")
            .to_string();
        out.push(BackupEntryDto {
            name,
            path: path.to_string_lossy().into_owned(),
            created_at,
        });
    }
    out.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(out)
}

pub fn restore_backup(instance: &Path, backup_name: &str) -> Result<(), String> {
    crate::validate::validate_backup_name(backup_name)?;
    let zip_path = backups_dir(instance).join(backup_name);
    if !zip_path.is_file() {
        return Err("backup not found".into());
    }
    let _ = crate::validate::ensure_within(&backups_dir(instance), &zip_path)?;

    let saved = instance.join("Pal").join("Saved");
    if saved.exists() {
        let bak = instance.join(".asylum").join(format!(
            "saved-before-restore-{}",
            Local::now().format("%Y%m%d-%H%M%S")
        ));
        fs::rename(&saved, &bak).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&saved).map_err(|e| e.to_string())?;

    let file = File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    const MAX_FILES: usize = 50_000;
    const MAX_UNCOMPRESSED: u64 = 8 * 1024 * 1024 * 1024; // 8 GiB
    if archive.len() > MAX_FILES {
        return Err("backup archive has too many files".into());
    }
    let mut total_uncompressed: u64 = 0;
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        total_uncompressed = total_uncompressed.saturating_add(file.size());
        if total_uncompressed > MAX_UNCOMPRESSED {
            return Err("backup archive uncompressed size too large".into());
        }
        let rel = file
            .enclosed_name()
            .ok_or_else(|| "zip entry path rejected".to_string())?;
        for c in rel.components() {
            if matches!(
                c,
                std::path::Component::ParentDir | std::path::Component::RootDir
            ) {
                return Err("zip entry path rejected".into());
            }
        }
        let outpath = instance.join(rel);
        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn trim_backups(instance: &Path, keep: usize) -> Result<(), String> {
    let mut list = list_backups(instance)?;
    if list.len() <= keep {
        return Ok(());
    }
    for entry in list.drain(keep..) {
        let _ = fs::remove_file(entry.path);
    }
    Ok(())
}
