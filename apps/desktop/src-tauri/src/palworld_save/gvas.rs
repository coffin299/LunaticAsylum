//! GVAS ヘッダ + トップレベルプロパティの走査（必要分）

use super::error::{ParseError, ParseStats};
use super::properties::{FArchiveReader, PropertyValue};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GvasHeader {
    pub save_game_version: i32,
    pub engine_major: u16,
    pub engine_minor: u16,
    pub engine_patch: u16,
    pub engine_changelist: u32,
    pub engine_branch: String,
    pub save_game_class_name: String,
}

#[derive(Debug)]
pub struct GvasFile {
    pub header: GvasHeader,
    pub root_keys: Vec<String>,
    pub world_save_data: Option<PropertyValue>,
    pub stats: ParseStats,
}

pub fn read_gvas(data: &[u8]) -> Result<GvasFile, ParseError> {
    let mut reader = FArchiveReader::new(data);
    let mut stats = ParseStats::default();

    let magic = reader.i32()?;
    if magic != 0x5341_5647 {
        return Err(ParseError::Format(format!(
            "invalid GVAS magic 0x{magic:08x}"
        )));
    }
    let save_game_version = reader.i32()?;
    let _pkg_ue4 = reader.i32()?;
    let _pkg_ue5 = reader.i32()?;
    let engine_major = reader.u16()?;
    let engine_minor = reader.u16()?;
    let engine_patch = reader.u16()?;
    let engine_changelist = reader.u32()?;
    let engine_branch = reader.fstring()?;
    let custom_version_format = reader.i32()?;
    if custom_version_format != 3 {
        stats.push(
            "custom_version_format",
            format!("expected 3, got {custom_version_format}"),
        );
    }
    let custom_count = reader.i32()?;
    if custom_count < 0 || custom_count > 10_000 {
        return Err(ParseError::Format(format!(
            "unreasonable custom_versions count {custom_count}"
        )));
    }
    for _ in 0..custom_count {
        let _ = reader.guid()?;
        let _ = reader.i32()?;
    }
    let save_game_class_name = reader.fstring()?;

    let header = GvasHeader {
        save_game_version,
        engine_major,
        engine_minor,
        engine_patch,
        engine_changelist,
        engine_branch,
        save_game_class_name,
    };

    let mut root_keys = Vec::new();
    let mut world_save_data = None;

    loop {
        let name = match reader.fstring() {
            Ok(n) => n,
            Err(e) => {
                stats.push("properties_eof", e.to_string());
                stats.subsection_failures += 1;
                break;
            }
        };
        if name == "None" || name.is_empty() {
            break;
        }
        root_keys.push(name.clone());
        let path = format!(".{name}");
        match reader.read_property_value(&path, &mut stats) {
            Ok(val) => {
                if name == "worldSaveData" {
                    world_save_data = Some(val);
                }
            }
            Err(e) => {
                stats.subsection_failures += 1;
                stats.push("property", format!("{path}: {e}"));
                break;
            }
        }
    }

    Ok(GvasFile {
        header,
        root_keys,
        world_save_data,
        stats,
    })
}
