//! PALWORLD_TYPE_HINTS（oMaN-Rod paltypes.py の必要パスのみ）

/// パス → 期待する struct / 型名（空 Map/Array や Guid キー用）
pub fn hint_for(path: &str) -> Option<&'static str> {
    HINTS.iter().find(|(p, _)| *p == path).map(|(_, t)| *t)
}

/// Map/Array 展開が必要なパス（巨大な無関係 Map は opaque のまま）
pub fn should_expand_map(path: &str) -> bool {
    path.ends_with("CharacterSaveParameterMap")
        || path.ends_with("GroupSaveDataMap")
        || path.ends_with("BaseCampSaveData")
}

pub fn is_character_rawdata(path: &str) -> bool {
    path.ends_with("CharacterSaveParameterMap.Value.RawData")
}

pub fn is_basecamp_rawdata(path: &str) -> bool {
    path.ends_with("BaseCampSaveData.Value.RawData")
}

const HINTS: &[(&str, &str)] = &[
    (
        ".worldSaveData.CharacterSaveParameterMap.Key",
        "StructProperty",
    ),
    (
        ".worldSaveData.CharacterSaveParameterMap.Value",
        "StructProperty",
    ),
    (".worldSaveData.GroupSaveDataMap.Key", "Guid"),
    (
        ".worldSaveData.GroupSaveDataMap.Value",
        "StructProperty",
    ),
    (".worldSaveData.BaseCampSaveData.Key", "Guid"),
    (
        ".worldSaveData.BaseCampSaveData.Value",
        "StructProperty",
    ),
    (
        ".worldSaveData.BaseCampSaveData.Value.ModuleMap.Value",
        "StructProperty",
    ),
];
