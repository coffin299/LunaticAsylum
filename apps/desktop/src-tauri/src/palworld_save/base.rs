//! BaseCampSaveData.Value.RawData（必要時のみ・oMaN-Rod base_camp.py）

use super::error::{ParseError, ParseStats};
use super::properties::{encode_guid, FArchiveReader, PropertyValue};
use serde_json::json;

pub fn decode_rawdata_property(bytes: &[u8], stats: &mut ParseStats) -> PropertyValue {
    match decode_bytes(bytes) {
        Ok(v) => PropertyValue::Str(v.to_string()),
        Err(e) => {
            stats.subsection_failures += 1;
            stats.push("base_rawdata", e.to_string());
            PropertyValue::Bytes(bytes.to_vec())
        }
    }
}

pub fn decode_bytes(b_bytes: &[u8]) -> Result<serde_json::Value, ParseError> {
    let mut reader = FArchiveReader::new(b_bytes);
    let id = reader.guid()?;
    let name = reader.fstring()?;
    let state = reader.u8()?;
    let transform = read_ftransform(&mut reader)?;
    let area_range = reader.f32()?;
    let group_id = reader.guid()?;
    let ft = read_ftransform(&mut reader)?;
    let owner = reader.guid()?;
    let trailing = reader.byte_list(4).unwrap_or_default();
    let mut obj = json!({
        "id": encode_guid(&id),
        "name": name,
        "state": state,
        "transform": transform,
        "areaRange": area_range,
        "groupIdBelongTo": encode_guid(&group_id),
        "fastTravelLocalTransform": ft,
        "ownerMapObjectInstanceId": encode_guid(&owner),
        "trailingBytes": trailing,
    });
    if !reader.eof() {
        obj["unknownBytes"] = json!(reader.read_to_end());
    }
    Ok(obj)
}

fn read_ftransform(reader: &mut FArchiveReader<'_>) -> Result<serde_json::Value, ParseError> {
    Ok(json!({
        "rotation": {
            "x": reader.f64()?, "y": reader.f64()?, "z": reader.f64()?, "w": reader.f64()?,
        },
        "translation": {
            "x": reader.f64()?, "y": reader.f64()?, "z": reader.f64()?,
        },
        "scale3d": {
            "x": reader.f64()?, "y": reader.f64()?, "z": reader.f64()?,
        },
    }))
}

pub fn extract_bases(
    world: Option<&PropertyValue>,
    stats: &mut ParseStats,
) -> Vec<serde_json::Value> {
    let mut bases = Vec::new();
    let Some(PropertyValue::Struct { fields, .. }) = world else {
        return bases;
    };
    let Some(PropertyValue::Map { entries, count, opaque, .. }) = fields.get("BaseCampSaveData")
    else {
        stats.push("base_map", "missing BaseCampSaveData");
        return bases;
    };
    if *opaque {
        stats.push("base_map", format!("BaseCampSaveData opaque count={count}"));
        return bases;
    }
    for (_key, value) in entries {
        let PropertyValue::Struct { fields, .. } = value else {
            continue;
        };
        match fields.get("RawData") {
            Some(PropertyValue::Str(s)) => {
                if let Ok(v) = serde_json::from_str(s) {
                    bases.push(v);
                }
            }
            Some(PropertyValue::Bytes(b)) => {
                if let Ok(v) = decode_bytes(b) {
                    bases.push(v);
                }
            }
            _ => {}
        }
    }
    stats.push("base_map", format!("decoded bases={}", bases.len()));
    bases
}
