//! Palworld Save の検出・メタ Snapshot + palworld_save パーサ接続。
//! decode-only。参照: docs/dev/palworld-save-parser-reference-map.md

use crate::palworld_save;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveParseDto {
    pub status: String,
    pub message: String,
    pub snapshot: Option<WorldSnapshotDto>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WorldSnapshotDto {
    pub timestamp: u64,
    pub world_dir: String,
    pub level_sav: Option<FileMetaDto>,
    pub level_meta_sav: Option<FileMetaDto>,
    pub world_option_sav: Option<FileMetaDto>,
    /// Players/*.sav ファイル一覧
    pub players: Vec<PlayerFileDto>,
    /// CharacterSaveParameterMap から IsPlayer=true
    pub parsed_players: Vec<serde_json::Value>,
    pub pals: Vec<serde_json::Value>,
    pub guilds: Vec<serde_json::Value>,
    pub bases: Vec<serde_json::Value>,
    pub map_hints: MapHintsDto,
    pub full_parse: String,
    pub gvas: Option<GvasSummaryDto>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GvasSummaryDto {
    pub container: String,
    pub save_type: u8,
    pub class_name: String,
    pub root_keys: Vec<String>,
    pub skipped_properties: u64,
    pub unsupported_types: u64,
    pub subsection_failures: u64,
    pub diags: Vec<DiagDto>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiagDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FileMetaDto {
    pub path: String,
    pub size: u64,
    pub magic: String,
    pub modified_unix: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PlayerFileDto {
    pub file_name: String,
    pub size: u64,
    pub magic: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MapHintsDto {
    pub note: String,
    pub player_markers: Vec<MapMarkerDto>,
    pub base_markers: Vec<MapMarkerDto>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MapMarkerDto {
    pub id: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn peek_magic(path: &Path) -> String {
    let Ok(bytes) = fs::read(path) else {
        return "unreadable".into();
    };
    if bytes.len() >= 11 && &bytes[8..11] == b"PlZ" {
        return "PlZ".into();
    }
    if bytes.len() >= 11 && &bytes[8..11] == b"PlM" {
        return "PlM".into();
    }
    if bytes.len() >= 23 && &bytes[20..23] == b"PlZ" {
        return "PlZ+CNK".into();
    }
    if bytes.len() >= 23 && &bytes[20..23] == b"PlM" {
        return "PlM+CNK".into();
    }
    if bytes.len() >= 4 && &bytes[0..4] == b"GVAS" {
        return "GVAS".into();
    }
    "unknown".into()
}

fn file_meta(path: &Path) -> Option<FileMetaDto> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    let modified_unix = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok().map(|d| d.as_secs()));
    Some(FileMetaDto {
        path: path.to_string_lossy().into_owned(),
        size: meta.len(),
        magic: peek_magic(path),
        modified_unix,
    })
}

fn find_world_dirs(saved: &Path) -> Vec<PathBuf> {
    let root = saved.join("SaveGames").join("0");
    let mut out = Vec::new();
    let Ok(rd) = fs::read_dir(&root) else {
        return out;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() && p.join("Level.sav").is_file() {
            out.push(p);
        }
    }
    out.sort();
    out
}

struct GvasParseOut {
    gvas: Option<GvasSummaryDto>,
    parsed_players: Vec<serde_json::Value>,
    pals: Vec<serde_json::Value>,
    guilds: Vec<serde_json::Value>,
    bases: Vec<serde_json::Value>,
    full_parse: String,
    message: String,
}

fn try_gvas_parse(level_path: &Path) -> GvasParseOut {
    let empty = GvasParseOut {
        gvas: None,
        parsed_players: vec![],
        pals: vec![],
        guilds: vec![],
        bases: vec![],
        full_parse: "deferred".into(),
        message: String::new(),
    };
    let Ok(bytes) = fs::read(level_path) else {
        return empty;
    };
    match palworld_save::parse_level_sav_bytes(&bytes) {
        Ok(r) => {
            let summary = GvasSummaryDto {
                container: r.container.clone(),
                save_type: r.save_type,
                class_name: r.class_name.clone(),
                root_keys: r.root_keys.clone(),
                skipped_properties: r.stats.skipped_properties,
                unsupported_types: r.stats.unsupported_types,
                subsection_failures: r.stats.subsection_failures,
                diags: r
                    .stats
                    .diags
                    .into_iter()
                    .map(|d| DiagDto {
                        code: d.code,
                        message: d.message,
                    })
                    .collect(),
            };
            let full = if r.players.is_empty()
                && r.pals.is_empty()
                && r.guilds.is_empty()
                && r.bases.is_empty()
            {
                "partial".into()
            } else {
                "ok".into()
            };
            GvasParseOut {
                gvas: Some(summary),
                parsed_players: r.players,
                pals: r.pals,
                guilds: r.guilds,
                bases: r.bases,
                full_parse: full,
                message: r.message,
            }
        }
        Err(e) => {
            let msg = e.to_string();
            let full = if msg.contains("PlM") || msg.contains("Oodle") || msg.contains("libooz") {
                "needs_oodle".into()
            } else {
                "error".into()
            };
            GvasParseOut {
                gvas: Some(GvasSummaryDto {
                    container: "unknown".into(),
                    save_type: 0,
                    class_name: String::new(),
                    root_keys: vec![],
                    skipped_properties: 0,
                    unsupported_types: 0,
                    subsection_failures: 1,
                    diags: vec![DiagDto {
                        code: "parse_level".into(),
                        message: msg.clone(),
                    }],
                }),
                parsed_players: vec![],
                pals: vec![],
                guilds: vec![],
                bases: vec![],
                full_parse: full,
                message: msg,
            }
        }
    }
}

fn player_markers_from(players: &[serde_json::Value]) -> Vec<MapMarkerDto> {
    let mut out = Vec::new();
    for p in players {
        let x = p.get("x").and_then(|v| v.as_f64());
        let y = p.get("y").and_then(|v| v.as_f64());
        let (Some(x), Some(y)) = (x, y) else {
            continue;
        };
        let id = p
            .get("key")
            .and_then(|v| v.as_str())
            .unwrap_or("player")
            .to_string();
        let label = p
            .get("nickname")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or(&id)
            .to_string();
        out.push(MapMarkerDto { id, label, x, y });
    }
    out
}

fn base_markers_from(bases: &[serde_json::Value]) -> Vec<MapMarkerDto> {
    let mut out = Vec::new();
    for b in bases {
        let id = b
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("base")
            .to_string();
        let label = b
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        let (x, y) = b
            .pointer("/transform/translation")
            .map(|t| {
                (
                    t.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    t.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0),
                )
            })
            .unwrap_or((0.0, 0.0));
        if x != 0.0 || y != 0.0 {
            out.push(MapMarkerDto { id, label, x, y });
        }
    }
    out
}

pub fn parse_instance_save(instance: &Path) -> SaveParseDto {
    let saved = instance.join("Pal").join("Saved");
    if !saved.is_dir() {
        return SaveParseDto {
            status: "missing".into(),
            message: "Pal/Saved が見つかりません".into(),
            snapshot: None,
        };
    }

    let worlds = find_world_dirs(&saved);
    if worlds.is_empty() {
        return SaveParseDto {
            status: "missing".into(),
            message: "SaveGames/0/*/Level.sav が見つかりません（未起動ワールドの可能性）".into(),
            snapshot: None,
        };
    }

    let mut best = worlds[0].clone();
    let mut best_mtime = 0u64;
    for w in &worlds {
        if let Some(m) = file_meta(&w.join("Level.sav")) {
            let t = m.modified_unix.unwrap_or(0);
            if t >= best_mtime {
                best_mtime = t;
                best = w.clone();
            }
        }
    }

    let level_path = best.join("Level.sav");
    let level = file_meta(&level_path);
    let level_meta = file_meta(&best.join("LevelMeta.sav"));
    let world_option = file_meta(&best.join("WorldOption.sav"));

    let mut players = Vec::new();
    let players_dir = best.join("Players");
    if let Ok(rd) = fs::read_dir(&players_dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("sav") {
                continue;
            }
            let meta = fs::metadata(&p).ok();
            players.push(PlayerFileDto {
                file_name: e.file_name().to_string_lossy().into_owned(),
                size: meta.map(|m| m.len()).unwrap_or(0),
                magic: peek_magic(&p),
            });
        }
    }
    players.sort_by(|a, b| a.file_name.cmp(&b.file_name));

    let parsed = try_gvas_parse(&level_path);
    let player_markers = player_markers_from(&parsed.parsed_players);
    let base_markers = base_markers_from(&parsed.bases);

    let magic_ok = level
        .as_ref()
        .map(|m| {
            m.magic.starts_with("PlZ") || m.magic.starts_with("PlM") || m.magic == "GVAS"
        })
        .unwrap_or(false);

    let status = match parsed.full_parse.as_str() {
        "ok" => "ok",
        "needs_oodle" => "partial",
        "error" if magic_ok => "partial",
        "partial" => "partial",
        _ if magic_ok => "partial",
        _ => "unsupported",
    };

    let message = if !parsed.message.is_empty() {
        format!(
            "ワールド: {} / Players .sav {} 件。{}",
            best.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default(),
            players.len(),
            parsed.message
        )
    } else if magic_ok {
        format!(
            "ワールド検出: {} / プレイヤー .sav {} 件。",
            best.file_name()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default(),
            players.len()
        )
    } else {
        "Level.sav の形式を認識できませんでした。".into()
    };

    SaveParseDto {
        status: status.into(),
        message,
        snapshot: Some(WorldSnapshotDto {
            timestamp: now_unix(),
            world_dir: best.to_string_lossy().into_owned(),
            level_sav: level,
            level_meta_sav: level_meta,
            world_option_sav: world_option,
            players,
            parsed_players: parsed.parsed_players,
            pals: parsed.pals,
            guilds: parsed.guilds,
            bases: parsed.bases,
            map_hints: MapHintsDto {
                note: match (player_markers.is_empty(), base_markers.is_empty()) {
                    (false, false) => "プレイヤー Location / 拠点 transform からマーカー生成。".into(),
                    (false, true) => "プレイヤー Location からマーカー生成。".into(),
                    (true, false) => "拠点 transform からマーカー生成。プレイヤー座標は未検出。".into(),
                    (true, true) => "座標フィールド未検出（PlM/Oodle または Location 欠落）。".into(),
                },
                player_markers,
                base_markers,
            },
            full_parse: parsed.full_parse,
            gvas: parsed.gvas,
        }),
    }
}
