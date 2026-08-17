//! CharacterSaveParameterMap.Value.RawData（oMaN-Rod character.py）

use super::error::ParseStats;
use super::location::find_transform_xyz;
use super::properties::{encode_guid, CharacterRawData, FArchiveReader, PropertyValue};
use serde_json::json;
use std::collections::BTreeMap;

pub fn decode_rawdata_property(bytes: &[u8], stats: &mut ParseStats) -> PropertyValue {
    match decode_bytes(bytes, stats) {
        Ok(raw) => PropertyValue::CharacterRaw(Box::new(raw)),
        Err(e) => {
            stats.subsection_failures += 1;
            stats.push("character_rawdata", e.to_string());
            PropertyValue::Bytes(bytes.to_vec())
        }
    }
}

pub fn decode_bytes(bytes: &[u8], stats: &mut ParseStats) -> Result<CharacterRawData, super::error::ParseError> {
    let mut reader = FArchiveReader::new(bytes);
    let fields = reader.properties_until_end("", stats);
    let unknown_bytes = reader.byte_list(4).unwrap_or_default();
    let group_id = reader.guid().unwrap_or([0u8; 16]);
    let trailing_bytes = reader.byte_list(4).unwrap_or_default();
    if !reader.eof() {
        let rest = reader.read_to_end();
        if rest.len() > 256 {
            stats.push(
                "character_trailing",
                format!("extra {} bytes preserved as opaque", rest.len()),
            );
        }
    }
    Ok(CharacterRawData {
        fields,
        group_id,
        unknown_bytes,
        trailing_bytes,
    })
}

/// worldSaveData からプレイヤー / Pal を抽出（IsPlayer で判定）
pub fn extract_characters(
    world: Option<&PropertyValue>,
    stats: &mut ParseStats,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let mut players = Vec::new();
    let mut pals = Vec::new();
    let Some(PropertyValue::Struct { fields, .. }) = world else {
        return (players, pals);
    };
    let Some(PropertyValue::Map { entries, count, opaque, .. }) =
        fields.get("CharacterSaveParameterMap")
    else {
        stats.push("character_map", "missing CharacterSaveParameterMap");
        return (players, pals);
    };
    if *opaque {
        stats.push(
            "character_map",
            format!("CharacterSaveParameterMap opaque count={count}"),
        );
        return (players, pals);
    }

    for (key, value) in entries {
        let key_id = map_key_id(key);

        let raw = match value {
            PropertyValue::Struct { fields, .. } => fields.get("RawData"),
            _ => None,
        };

        let Some(raw) = raw else {
            stats.push("character_entry", format!("{key_id}: missing RawData"));
            continue;
        };

        let summary = match raw {
            PropertyValue::CharacterRaw(c) => summarize_character(&c.fields),
            PropertyValue::Bytes(b) => match decode_bytes(b, stats) {
                Ok(c) => summarize_character(&c.fields),
                Err(_) => continue,
            },
            _ => {
                stats.push("character_entry", format!("{key_id}: unexpected RawData shape"));
                continue;
            }
        };

        if summary.is_player {
            let mut obj = json!({
                "key": key_id,
                "nickname": summary.nickname,
                "level": summary.level,
                "isPlayer": true,
            });
            if let Some((x, y, z)) = summary.location {
                obj["x"] = json!(x);
                obj["y"] = json!(y);
                obj["z"] = json!(z);
            }
            players.push(obj);
        } else {
            pals.push(json!({
                "key": key_id,
                "characterId": summary.character_id,
                "nickname": summary.nickname,
                "level": summary.level,
                "ownerPlayerUid": summary.owner,
                "isPlayer": false,
            }));
        }
    }

    stats.push(
        "character_map",
        format!(
            "decoded players={} pals={} (IsPlayer discriminant)",
            players.len(),
            pals.len()
        ),
    );
    (players, pals)
}

struct CharacterSummary {
    is_player: bool,
    nickname: String,
    level: i32,
    character_id: String,
    owner: String,
    location: Option<(f64, f64, f64)>,
}

fn summarize_character(fields: &BTreeMap<String, PropertyValue>) -> CharacterSummary {
    let sp = nested_save_parameter(fields);
    CharacterSummary {
        is_player: find_bool(sp, "IsPlayer").unwrap_or(false),
        nickname: find_string(sp, "NickName")
            .or_else(|| find_string(sp, "Nickname"))
            .unwrap_or_default(),
        level: find_int(sp, "Level").unwrap_or(0),
        character_id: find_string(sp, "CharacterID")
            .or_else(|| find_string(sp, "CharacterId"))
            .unwrap_or_default(),
        owner: find_guid_str(sp, "OwnerPlayerUId")
            .or_else(|| find_guid_str(sp, "OwnerPlayerUid"))
            .unwrap_or_default(),
        location: find_transform_xyz(sp),
    }
}

fn map_key_id(key: &PropertyValue) -> String {
    match key {
        PropertyValue::Guid(g) => encode_guid(g),
        PropertyValue::Struct { fields, .. } => find_guid_str(fields, "PlayerUId")
            .or_else(|| find_guid_str(fields, "InstanceId"))
            .or_else(|| find_guid_str(fields, "_guid"))
            .unwrap_or_else(|| "unknown".into()),
        _ => "unknown".into(),
    }
}

fn nested_save_parameter(fields: &BTreeMap<String, PropertyValue>) -> &BTreeMap<String, PropertyValue> {
    if let Some(PropertyValue::Struct { fields: inner, .. }) = fields.get("SaveParameter") {
        return inner;
    }
    fields
}

fn find_bool(fields: &BTreeMap<String, PropertyValue>, name: &str) -> Option<bool> {
    match fields.get(name) {
        Some(PropertyValue::Bool(b)) => Some(*b),
        _ => None,
    }
}

fn find_int(fields: &BTreeMap<String, PropertyValue>, name: &str) -> Option<i32> {
    match fields.get(name) {
        Some(PropertyValue::Int(v)) => Some(*v),
        Some(PropertyValue::Int64(v)) => Some(*v as i32),
        Some(PropertyValue::Byte { value, .. }) => value.parse().ok(),
        _ => None,
    }
}

fn find_string(fields: &BTreeMap<String, PropertyValue>, name: &str) -> Option<String> {
    match fields.get(name) {
        Some(PropertyValue::Str(s) | PropertyValue::Name(s)) => Some(s.clone()),
        _ => None,
    }
}

fn find_guid_str(fields: &BTreeMap<String, PropertyValue>, name: &str) -> Option<String> {
    match fields.get(name) {
        Some(PropertyValue::Guid(g)) => Some(encode_guid(g)),
        Some(PropertyValue::Struct { fields, .. }) => fields.get("_guid").and_then(|v| match v {
            PropertyValue::Guid(g) => Some(encode_guid(g)),
            PropertyValue::Str(s) => Some(s.clone()),
            _ => None,
        }),
        Some(PropertyValue::Str(s)) => Some(s.clone()),
        _ => None,
    }
}
