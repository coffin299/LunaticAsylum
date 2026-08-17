//! Players/*.sav — SaveData.LastTransform（プレイヤー最終位置）

use super::container::decompress_sav;
use super::error::ParseError;
use super::gvas::read_gvas;
use super::location::find_transform_xyz;
use super::properties::{encode_guid, PropertyValue};
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct PlayerLocation {
    pub player_uid: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

pub fn parse_player_sav_bytes(data: &[u8]) -> Result<PlayerLocation, ParseError> {
    let dec = decompress_sav(data)?;
    let gvas = read_gvas(&dec.gvas)?;
    let save_data = gvas
        .save_data
        .as_ref()
        .ok_or_else(|| ParseError::Unsupported("player.sav: missing SaveData".into()))?;
    let fields = struct_fields(save_data).ok_or_else(|| {
        ParseError::Unsupported("player.sav: SaveData is not a struct".into())
    })?;
    let (x, y, z) = find_transform_xyz(fields).ok_or_else(|| {
        ParseError::Unsupported("player.sav: SaveData.LastTransform not found".into())
    })?;
    let player_uid = gvas
        .player_uid
        .as_ref()
        .and_then(guid_from_prop)
        .or_else(|| find_guid_in_save_data(fields))
        .unwrap_or_default();
    Ok(PlayerLocation {
        player_uid,
        x,
        y,
        z,
    })
}

fn struct_fields(value: &PropertyValue) -> Option<&BTreeMap<String, PropertyValue>> {
    match value {
        PropertyValue::Struct { fields, .. } => Some(fields),
        _ => None,
    }
}

fn guid_from_prop(value: &PropertyValue) -> Option<String> {
    match value {
        PropertyValue::Guid(g) => Some(encode_guid(g)),
        PropertyValue::Struct { fields, .. } => fields.get("_guid").and_then(|v| match v {
            PropertyValue::Guid(g) => Some(encode_guid(g)),
            PropertyValue::Str(s) => Some(s.clone()),
            _ => None,
        }),
        PropertyValue::Str(s) => Some(s.clone()),
        _ => None,
    }
}

fn find_guid_in_save_data(fields: &BTreeMap<String, PropertyValue>) -> Option<String> {
    fields
        .get("PlayerUId")
        .or_else(|| fields.get("PlayerUid"))
        .and_then(guid_from_prop)
}

#[cfg(test)]
mod tests {
    use super::parse_player_sav_bytes;

    #[test]
    fn e2e_player_sav_from_env() {
        let Ok(path) = std::env::var("LUNATIC_ASYLUM_PLAYER_SAV") else {
            return;
        };
        let bytes = std::fs::read(&path).expect("read player.sav");
        let loc = parse_player_sav_bytes(&bytes).expect("parse player.sav");
        eprintln!("uid={} x={} y={} z={}", loc.player_uid, loc.x, loc.y, loc.z);
        assert!(loc.x != 0.0 || loc.y != 0.0, "expected LastTransform");
    }
}
