//! GroupSaveDataMap（oMaN-Rod group.py / Palworld 1.0 — cheahjs 0.24 禁止）

use super::error::{ParseError, ParseStats};
use super::properties::{encode_guid, FArchiveReader, PropertyValue};
use serde_json::json;

pub fn decode_map_entries(
    reader: &mut FArchiveReader<'_>,
    count: i32,
    key_type: &str,
    value_type: &str,
    path: &str,
    stats: &mut ParseStats,
) -> Result<PropertyValue, ParseError> {
    let key_path = format!("{path}.Key");
    let value_path = format!("{path}.Value");
    let key_struct = if key_type == "StructProperty" { "Guid" } else { "" };
    let value_struct = if value_type == "StructProperty" {
        "StructProperty"
    } else {
        ""
    };

    let mut entries = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let key = reader.prop_value(key_type, key_struct, &key_path, stats)?;
        let value = reader.prop_value(value_type, value_struct, &value_path, stats)?;
        entries.push((key, value));
    }

    Ok(PropertyValue::Map {
        key_type: key_type.into(),
        value_type: value_type.into(),
        count,
        opaque: false,
        entries,
    })
}

pub fn decode_bytes(
    group_bytes: &[u8],
    group_type: &str,
    stats: &mut ParseStats,
) -> Result<serde_json::Value, ParseError> {
    let mut reader = FArchiveReader::new(group_bytes);
    let group_id = reader.guid()?;
    let group_name = reader.fstring()?;
    let handles = reader.tarray_instance_ids()?;

    let mut obj = json!({
        "groupType": group_type,
        "groupId": encode_guid(&group_id),
        "groupName": group_name,
        "individualCharacterHandleIds": handles.iter().map(|(a,b)| json!({
            "guid": encode_guid(a),
            "instanceId": encode_guid(b),
        })).collect::<Vec<_>>(),
    });

    let is_org_family = matches!(
        group_type,
        "EPalGroupType::Guild"
            | "EPalGroupType::IndependentGuild"
            | "EPalGroupType::Organization"
    );

    if is_org_family {
        obj["orgType"] = json!(reader.u8()?);
    }

    if group_type == "EPalGroupType::Organization" {
        obj["trailingBytes"] = json!(reader.byte_list(12)?);
    }

    if group_type == "EPalGroupType::Guild" {
        let leading = reader.byte_list(4).unwrap_or_default();
        let base_ids = reader.tarray_guid()?;
        let unknown_1 = reader.i32()?;
        let base_camp_level = reader.i32()?;
        let map_object_ids = reader.tarray_guid()?;
        let guild_name = reader.fstring()?;
        let last_mod = reader.guid()?;
        let marker_count = reader.u32()? as i32;
        if marker_count < 0 || marker_count > 100_000 {
            return Err(ParseError::Format(format!(
                "guild_markers count {marker_count}"
            )));
        }
        let mut markers = Vec::new();
        for _ in 0..marker_count {
            let marker_id = reader.guid()?;
            let x = reader.f64()?;
            let y = reader.f64()?;
            let z = reader.f64()?;
            let icon_type = reader.i32()?;
            let owner = reader.guid()?;
            markers.push(json!({
                "markerId": encode_guid(&marker_id),
                "x": x, "y": y, "z": z,
                "iconType": icon_type,
                "ownerPlayerUid": encode_guid(&owner),
            }));
        }
        obj["leadingBytes"] = json!(leading);
        obj["baseIds"] = json!(base_ids.iter().map(encode_guid).collect::<Vec<_>>());
        obj["unknown1"] = json!(unknown_1);
        obj["baseCampLevel"] = json!(base_camp_level);
        obj["mapObjectInstanceIdsBaseCampPoints"] =
            json!(map_object_ids.iter().map(encode_guid).collect::<Vec<_>>());
        obj["guildName"] = json!(guild_name);
        obj["lastGuildNameModifierPlayerUid"] = json!(encode_guid(&last_mod));
        obj["guildMarkers"] = json!(markers);

        match read_guild_tail(&mut reader) {
            Ok(tail) => {
                if let Some(map) = obj.as_object_mut() {
                    for (k, v) in tail.as_object().cloned().into_iter().flatten() {
                        map.insert(k, v);
                    }
                }
            }
            Err(_) => {
                let rest = reader.read_to_end();
                obj["opaqueTail"] = json!(rest);
                stats.push(
                    "GroupSaveDataMap.Value.RawData.guild-tail",
                    "(tolerated)",
                );
            }
        }
    }

    if group_type == "EPalGroupType::IndependentGuild" {
        let base_camp_level = reader.i32()?;
        let map_object_ids = reader.tarray_guid()?;
        let guild_name = reader.fstring()?;
        let player_uid = reader.guid()?;
        let guild_name_2 = reader.fstring()?;
        let last_online = reader.i64()?;
        let player_name = reader.fstring()?;
        obj["baseCampLevel"] = json!(base_camp_level);
        obj["mapObjectInstanceIdsBaseCampPoints"] =
            json!(map_object_ids.iter().map(encode_guid).collect::<Vec<_>>());
        obj["guildName"] = json!(guild_name);
        obj["playerUid"] = json!(encode_guid(&player_uid));
        obj["guildName2"] = json!(guild_name_2);
        obj["playerInfo"] = json!({
            "lastOnlineRealTime": last_online,
            "playerName": player_name,
        });
    }

    if !reader.eof() && group_type != "EPalGroupType::Guild" {
        let rest = reader.read_to_end();
        if !rest.is_empty() {
            stats.push(
                "group_eof",
                format!("{group_type}: {} trailing bytes", rest.len()),
            );
            obj["trailingUnknown"] = json!(rest);
        }
    }

    Ok(obj)
}

fn read_guild_tail(reader: &mut FArchiveReader<'_>) -> Result<serde_json::Value, ParseError> {
    let start = reader.tell();
    if let Ok(v2) = read_guild_tail_v2(reader) {
        if reader.eof() {
            return Ok(v2);
        }
    }
    reader.seek(start)?;
    read_guild_tail_v1(reader)
}

fn read_guild_tail_v2(reader: &mut FArchiveReader<'_>) -> Result<serde_json::Value, ParseError> {
    let role_count = reader.u32()? as i32;
    if !(0..=10_000).contains(&role_count) {
        return Err(ParseError::Format("chest roles".into()));
    }
    let mut chest_roles = Vec::new();
    for _ in 0..role_count {
        chest_roles.push(reader.u8()?);
    }
    let unknown_i32 = reader.i32()?;
    let admin = reader.guid()?;
    let players = read_guild_players(reader, true)?;
    let perm_count = reader.u32()? as i32;
    if !(0..=10_000).contains(&perm_count) {
        return Err(ParseError::Format("role_permissions".into()));
    }
    let mut role_permissions = Vec::new();
    for _ in 0..perm_count {
        let role = reader.u8()?;
        let n = reader.u32()? as i32;
        if !(0..=10_000).contains(&n) {
            return Err(ParseError::Format("permissions".into()));
        }
        let mut perms = Vec::new();
        for _ in 0..n {
            perms.push(reader.u8()?);
        }
        role_permissions.push(json!({ "role": role, "permissions": perms }));
    }
    let trailing = reader.byte_list(4)?;
    Ok(json!({
        "guildChestAllowedRoles": chest_roles,
        "unknownI32": unknown_i32,
        "adminPlayerUid": encode_guid(&admin),
        "players": players,
        "rolePermissions": role_permissions,
        "trailingBytes": trailing,
        "tailVersion": 2,
    }))
}

fn read_guild_tail_v1(reader: &mut FArchiveReader<'_>) -> Result<serde_json::Value, ParseError> {
    let admin = reader.guid()?;
    let players = read_guild_players(reader, false)?;
    let trailing = reader.byte_list(4)?;
    if !reader.eof() {
        return Err(ParseError::Format("guild tail v1 did not reach EOF".into()));
    }
    Ok(json!({
        "adminPlayerUid": encode_guid(&admin),
        "players": players,
        "trailingBytes": trailing,
        "tailVersion": 1,
    }))
}

fn read_guild_players(
    reader: &mut FArchiveReader<'_>,
    with_role: bool,
) -> Result<Vec<serde_json::Value>, ParseError> {
    let count = reader.u32()? as i32;
    if !(0..=100_000).contains(&count) {
        return Err(ParseError::Format(format!("players count {count}")));
    }
    let mut out = Vec::with_capacity(count as usize);
    for _ in 0..count {
        let uid = reader.guid()?;
        let last_online = reader.i64()?;
        let name = reader.fstring()?;
        let mut p = json!({
            "playerUid": encode_guid(&uid),
            "playerInfo": {
                "lastOnlineRealTime": last_online,
                "playerName": name,
            }
        });
        if with_role {
            p["role"] = json!(reader.u8()?);
        }
        out.push(p);
    }
    Ok(out)
}

pub fn extract_guilds(
    world: Option<&PropertyValue>,
    stats: &mut ParseStats,
) -> Vec<serde_json::Value> {
    let mut guilds = Vec::new();
    let Some(PropertyValue::Struct { fields, .. }) = world else {
        return guilds;
    };
    let Some(PropertyValue::Map { entries, count, opaque, .. }) = fields.get("GroupSaveDataMap")
    else {
        stats.push("group_map", "missing GroupSaveDataMap");
        return guilds;
    };
    if *opaque {
        stats.push("group_map", format!("GroupSaveDataMap opaque count={count}"));
        return guilds;
    }

    for (_key, value) in entries {
        let PropertyValue::Struct { fields, .. } = value else {
            continue;
        };
        let group_type = fields
            .get("GroupType")
            .and_then(|v| match v {
                PropertyValue::Name(s)
                | PropertyValue::Str(s)
                | PropertyValue::Byte { value: s, .. } => Some(s.as_str()),
                _ => None,
            })
            .unwrap_or("");

        // 型で判定（表示名では推論しない）
        if group_type != "EPalGroupType::Guild"
            && group_type != "EPalGroupType::IndependentGuild"
        {
            continue;
        }

        if let Some(PropertyValue::Bytes(raw)) = fields.get("RawData") {
            match decode_bytes(raw, group_type, stats) {
                Ok(v) => guilds.push(v),
                Err(e) => {
                    stats.subsection_failures += 1;
                    stats.push("group_rawdata", format!("{group_type}: {e}"));
                }
            }
        }
    }

    stats.push(
        "group_map",
        format!("decoded guilds={} (by EPalGroupType)", guilds.len()),
    );
    guilds
}
