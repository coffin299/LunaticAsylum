//! FArchive 風リーダ（oMaN-Rod archive.py / Palhelm 方針）

use super::error::{ParseError, ParseStats};
use super::type_hints;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum PropertyValue {
    None,
    Bool(bool),
    Int(i32),
    Int64(i64),
    Float(f32),
    Double(f64),
    Str(String),
    Name(String),
    Guid([u8; 16]),
    Byte { enum_name: String, value: String },
    Bytes(Vec<u8>),
    Struct {
        struct_type: String,
        fields: BTreeMap<String, PropertyValue>,
    },
    Array {
        inner_type: String,
        values: Vec<PropertyValue>,
    },
    Map {
        key_type: String,
        value_type: String,
        count: i32,
        opaque: bool,
        entries: Vec<(PropertyValue, PropertyValue)>,
    },
    /// Character RawData 展開結果など
    CharacterRaw(Box<CharacterRawData>),
    Opaque {
        type_name: String,
        size: u64,
    },
}

#[derive(Debug, Clone)]
pub struct CharacterRawData {
    pub fields: BTreeMap<String, PropertyValue>,
    pub group_id: [u8; 16],
    pub unknown_bytes: Vec<u8>,
    pub trailing_bytes: Vec<u8>,
}

pub fn encode_guid(g: &[u8; 16]) -> String {
    g.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join("")
}

pub struct FArchiveReader<'a> {
    data: &'a [u8],
    pub(crate) pos: usize,
}

impl<'a> FArchiveReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    pub fn eof(&self) -> bool {
        self.pos >= self.data.len()
    }

    pub fn tell(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, pos: usize) -> Result<(), ParseError> {
        if pos > self.data.len() {
            return Err(ParseError::Eof(format!("seek {pos} past {}", self.data.len())));
        }
        self.pos = pos;
        Ok(())
    }

    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    fn need(&self, n: usize) -> Result<(), ParseError> {
        if self.remaining() < n {
            Err(ParseError::Eof(format!(
                "need {n} bytes at {}, have {}",
                self.pos,
                self.remaining()
            )))
        } else {
            Ok(())
        }
    }

    pub fn skip(&mut self, n: usize) -> Result<(), ParseError> {
        self.need(n)?;
        self.pos += n;
        Ok(())
    }

    pub fn byte_list(&mut self, n: usize) -> Result<Vec<u8>, ParseError> {
        self.need(n)?;
        let v = self.data[self.pos..self.pos + n].to_vec();
        self.pos += n;
        Ok(v)
    }

    pub fn read_to_end(&mut self) -> Vec<u8> {
        let v = self.data[self.pos..].to_vec();
        self.pos = self.data.len();
        v
    }

    pub fn u8(&mut self) -> Result<u8, ParseError> {
        self.need(1)?;
        let v = self.data[self.pos];
        self.pos += 1;
        Ok(v)
    }

    pub fn bool_byte(&mut self) -> Result<bool, ParseError> {
        Ok(self.u8()? != 0)
    }

    pub fn u16(&mut self) -> Result<u16, ParseError> {
        self.need(2)?;
        let v = u16::from_le_bytes(self.data[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        Ok(v)
    }

    pub fn i32(&mut self) -> Result<i32, ParseError> {
        self.need(4)?;
        let v = i32::from_le_bytes(self.data[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        Ok(v)
    }

    pub fn u32(&mut self) -> Result<u32, ParseError> {
        Ok(self.i32()? as u32)
    }

    pub fn i64(&mut self) -> Result<i64, ParseError> {
        self.need(8)?;
        let v = i64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(v)
    }

    pub fn u64(&mut self) -> Result<u64, ParseError> {
        self.need(8)?;
        let v = u64::from_le_bytes(self.data[self.pos..self.pos + 8].try_into().unwrap());
        self.pos += 8;
        Ok(v)
    }

    pub fn f32(&mut self) -> Result<f32, ParseError> {
        Ok(f32::from_bits(self.u32()?))
    }

    pub fn f64(&mut self) -> Result<f64, ParseError> {
        Ok(f64::from_bits(self.u64()?))
    }

    pub fn guid(&mut self) -> Result<[u8; 16], ParseError> {
        self.need(16)?;
        let mut g = [0u8; 16];
        g.copy_from_slice(&self.data[self.pos..self.pos + 16]);
        self.pos += 16;
        Ok(g)
    }

    pub fn optional_guid(&mut self) -> Result<Option<[u8; 16]>, ParseError> {
        if self.bool_byte()? {
            Ok(Some(self.guid()?))
        } else {
            Ok(None)
        }
    }

    pub fn fstring(&mut self) -> Result<String, ParseError> {
        let len = self.i32()?;
        if len == 0 {
            return Ok(String::new());
        }
        if len > 0 {
            let n = len as usize;
            if n > 16 * 1024 * 1024 {
                return Err(ParseError::Format(format!("fstring too large: {n}")));
            }
            self.need(n)?;
            let bytes = &self.data[self.pos..self.pos + n];
            self.pos += n;
            let s = if bytes.last() == Some(&0) {
                String::from_utf8_lossy(&bytes[..n.saturating_sub(1)]).into_owned()
            } else {
                String::from_utf8_lossy(bytes).into_owned()
            };
            Ok(s)
        } else {
            let n = ((-len) as usize).saturating_mul(2);
            if n > 16 * 1024 * 1024 {
                return Err(ParseError::Format(format!("fstring utf16 too large: {n}")));
            }
            self.need(n)?;
            let raw = &self.data[self.pos..self.pos + n];
            self.pos += n;
            let mut u16s = Vec::with_capacity(n / 2);
            for chunk in raw.chunks_exact(2) {
                u16s.push(u16::from_le_bytes([chunk[0], chunk[1]]));
            }
            if u16s.last() == Some(&0) {
                u16s.pop();
            }
            Ok(String::from_utf16_lossy(&u16s))
        }
    }

    pub fn tarray_guid(&mut self) -> Result<Vec<[u8; 16]>, ParseError> {
        let count = self.u32()? as i32;
        if count < 0 || count > 1_000_000 {
            return Err(ParseError::Format(format!("tarray guid count {count}")));
        }
        let mut out = Vec::with_capacity(count as usize);
        for _ in 0..count {
            out.push(self.guid()?);
        }
        Ok(out)
    }

    pub fn tarray_instance_ids(&mut self) -> Result<Vec<([u8; 16], [u8; 16])>, ParseError> {
        let count = self.u32()? as i32;
        if count < 0 || count > 1_000_000 {
            return Err(ParseError::Format(format!("tarray instance count {count}")));
        }
        let mut out = Vec::with_capacity(count as usize);
        for _ in 0..count {
            out.push((self.guid()?, self.guid()?));
        }
        Ok(out)
    }

    /// name 直後の type / size / value
    pub fn read_property_value(
        &mut self,
        path: &str,
        stats: &mut ParseStats,
    ) -> Result<PropertyValue, ParseError> {
        let type_name = self.fstring()?;
        let size = self.u64()?;
        let start = self.pos;
        let result = self.read_typed(&type_name, size, path, stats);
        let end_expected = start.saturating_add(size as usize).min(self.data.len());
        match &result {
            Ok(PropertyValue::Opaque { .. }) | Ok(PropertyValue::Map { opaque: true, .. }) => {
                if self.pos < end_expected {
                    let _ = self.skip(end_expected - self.pos);
                } else if self.pos > end_expected {
                    stats.push(
                        "over_read",
                        format!("{type_name}@{path}: pos {} > {end_expected}", self.pos),
                    );
                    self.pos = end_expected;
                }
            }
            Ok(_) => {
                // ArrayType / optional_guid が size 外のことがあるため巻き戻さない
                if self.pos < end_expected {
                    let _ = self.skip(end_expected - self.pos);
                } else if self.pos > end_expected {
                    stats.push(
                        "size_mismatch",
                        format!("{type_name}@{path}: read {} > size {size}", self.pos - start),
                    );
                }
            }
            Err(_) => {
                if self.pos < end_expected {
                    let _ = self.skip(end_expected - self.pos);
                }
            }
        }
        result
    }

    pub fn properties_until_end(
        &mut self,
        path: &str,
        stats: &mut ParseStats,
    ) -> BTreeMap<String, PropertyValue> {
        let mut fields = BTreeMap::new();
        let mut guard = 0u32;
        loop {
            guard += 1;
            if guard > 50_000 {
                stats.push("struct_fields", "field loop guard");
                break;
            }
            let name = match self.fstring() {
                Ok(n) => n,
                Err(e) => {
                    stats.subsection_failures += 1;
                    stats.push("struct_fields", e.to_string());
                    break;
                }
            };
            if name == "None" || name.is_empty() {
                break;
            }
            let child = if path.is_empty() {
                format!(".{name}")
            } else {
                format!("{path}.{name}")
            };
            match self.read_property_value(&child, stats) {
                Ok(v) => {
                    fields.insert(name, v);
                }
                Err(e) => {
                    stats.subsection_failures += 1;
                    stats.push("struct_field", format!("{child}: {e}"));
                    break;
                }
            }
        }
        fields
    }

    fn read_typed(
        &mut self,
        type_name: &str,
        size: u64,
        path: &str,
        stats: &mut ParseStats,
    ) -> Result<PropertyValue, ParseError> {
        match type_name {
            "BoolProperty" => {
                let v = self.bool_byte()?;
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Bool(v))
            }
            "IntProperty" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Int(self.i32()?))
            }
            "Int64Property" | "UInt64Property" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Int64(self.i64()?))
            }
            "UInt32Property" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Int(self.u32()? as i32))
            }
            "UInt16Property" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Int(self.u16()? as i32))
            }
            "FloatProperty" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Float(self.f32()?))
            }
            "DoubleProperty" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Double(self.f64()?))
            }
            "StrProperty" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Str(self.fstring()?))
            }
            "NameProperty" => {
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Name(self.fstring()?))
            }
            "ByteProperty" => {
                let enum_name = self.fstring()?;
                let _ = self.optional_guid()?;
                if enum_name == "None" {
                    let b = self.u8()?;
                    Ok(PropertyValue::Byte {
                        enum_name,
                        value: b.to_string(),
                    })
                } else {
                    let value = self.fstring()?;
                    Ok(PropertyValue::Byte { enum_name, value })
                }
            }
            "EnumProperty" => {
                let _enum_type = self.fstring()?;
                let _ = self.optional_guid()?;
                Ok(PropertyValue::Name(self.fstring()?))
            }
            "StructProperty" => {
                let struct_type = self.fstring()?;
                let _struct_id = self.guid()?;
                let _ = self.optional_guid()?;
                let fields = self.read_struct_value(&struct_type, path, stats)?;
                Ok(PropertyValue::Struct {
                    struct_type,
                    fields,
                })
            }
            "ArrayProperty" => self.read_array(size, path, stats),
            "MapProperty" => self.read_map(path, stats),
            "SetProperty" => {
                stats.unsupported_types += 1;
                stats.skipped_properties += 1;
                Ok(PropertyValue::Opaque {
                    type_name: type_name.into(),
                    size,
                })
            }
            other => {
                stats.unsupported_types += 1;
                stats.skipped_properties += 1;
                Ok(PropertyValue::Opaque {
                    type_name: other.into(),
                    size,
                })
            }
        }
    }

    fn read_struct_value(
        &mut self,
        struct_type: &str,
        path: &str,
        stats: &mut ParseStats,
    ) -> Result<BTreeMap<String, PropertyValue>, ParseError> {
        match struct_type {
            "Guid" => {
                let g = self.guid()?;
                let mut m = BTreeMap::new();
                m.insert("_guid".into(), PropertyValue::Guid(g));
                Ok(m)
            }
            "DateTime" => {
                let mut m = BTreeMap::new();
                m.insert("_datetime".into(), PropertyValue::Int64(self.u64()? as i64));
                Ok(m)
            }
            "Vector" | "Vector3d" => {
                let mut m = BTreeMap::new();
                m.insert("x".into(), PropertyValue::Double(self.f64()?));
                m.insert("y".into(), PropertyValue::Double(self.f64()?));
                m.insert("z".into(), PropertyValue::Double(self.f64()?));
                Ok(m)
            }
            "Quat" => {
                let mut m = BTreeMap::new();
                m.insert("x".into(), PropertyValue::Double(self.f64()?));
                m.insert("y".into(), PropertyValue::Double(self.f64()?));
                m.insert("z".into(), PropertyValue::Double(self.f64()?));
                m.insert("w".into(), PropertyValue::Double(self.f64()?));
                Ok(m)
            }
            "LinearColor" => {
                let mut m = BTreeMap::new();
                m.insert("r".into(), PropertyValue::Float(self.f32()?));
                m.insert("g".into(), PropertyValue::Float(self.f32()?));
                m.insert("b".into(), PropertyValue::Float(self.f32()?));
                m.insert("a".into(), PropertyValue::Float(self.f32()?));
                Ok(m)
            }
            _ => Ok(self.properties_until_end(path, stats)),
        }
    }

    fn read_array(
        &mut self,
        size: u64,
        path: &str,
        stats: &mut ParseStats,
    ) -> Result<PropertyValue, ParseError> {
        let inner = self.fstring()?;
        let _ = self.optional_guid()?;
        let count = self.u32()? as i32;
        if count < 0 || count > 50_000_000 {
            stats.unsupported_types += 1;
            return Ok(PropertyValue::Opaque {
                type_name: "ArrayProperty".into(),
                size,
            });
        }

        // RawData: Byte 配列 → 生バイト（Character は上位で decode）
        if inner == "ByteProperty" {
            let payload_size = size.saturating_sub(4); // count 分を除く概算
            if payload_size == count as u64 || count as u64 == size.saturating_sub(4) {
                let bytes = self.byte_list(count as usize)?;
                if type_hints::is_character_rawdata(path) {
                    return Ok(super::character::decode_rawdata_property(&bytes, stats));
                }
                if type_hints::is_basecamp_rawdata(path) {
                    return Ok(super::base::decode_rawdata_property(&bytes, stats));
                }
                return Ok(PropertyValue::Bytes(bytes));
            }
        }

        if inner == "StructProperty" {
            stats.skipped_properties += 1;
            stats.push("array_struct", format!("opaque StructProperty array @ {path}"));
            return Ok(PropertyValue::Array {
                inner_type: inner,
                values: vec![],
            });
        }

        // 巨大非 RawData 配列は opaque skip
        if count > 20_000 {
            stats.skipped_properties += 1;
            return Ok(PropertyValue::Array {
                inner_type: inner,
                values: vec![],
            });
        }

        let mut values = Vec::with_capacity(count as usize);
        for _ in 0..count {
            match inner.as_str() {
                "NameProperty" | "EnumProperty" | "StrProperty" => {
                    values.push(PropertyValue::Name(self.fstring()?));
                }
                "IntProperty" => values.push(PropertyValue::Int(self.i32()?)),
                "Int64Property" => values.push(PropertyValue::Int64(self.i64()?)),
                "ByteProperty" => values.push(PropertyValue::Int(self.u8()? as i32)),
                "FloatProperty" => values.push(PropertyValue::Float(self.f32()?)),
                "Guid" => values.push(PropertyValue::Guid(self.guid()?)),
                _ => {
                    stats.unsupported_types += 1;
                    break;
                }
            }
        }
        Ok(PropertyValue::Array {
            inner_type: inner,
            values,
        })
    }

    fn read_map(
        &mut self,
        path: &str,
        stats: &mut ParseStats,
    ) -> Result<PropertyValue, ParseError> {
        let key_type = self.fstring()?;
        let value_type = self.fstring()?;
        let _ = self.optional_guid()?;
        let _unknown = self.u32()?;
        let count = self.u32()? as i32;
        if count < 0 || count > 5_000_000 {
            return Err(ParseError::Format(format!("map count {count} @ {path}")));
        }

        let expand = type_hints::should_expand_map(path);
        if !expand {
            stats.skipped_properties += 1;
            return Ok(PropertyValue::Map {
                key_type,
                value_type,
                count,
                opaque: true,
                entries: vec![],
            });
        }

        // GroupSaveDataMap は専用デコード（Value 内 GroupType + RawData）
        if path.ends_with("GroupSaveDataMap") {
            return super::group::decode_map_entries(self, count, &key_type, &value_type, path, stats);
        }

        let key_path = format!("{path}.Key");
        let value_path = format!("{path}.Value");
        let key_struct = if key_type == "StructProperty" {
            type_hints::hint_for(&key_path).unwrap_or("Guid")
        } else {
            ""
        };
        let value_struct = if value_type == "StructProperty" {
            type_hints::hint_for(&value_path).unwrap_or("StructProperty")
        } else {
            ""
        };

        let mut entries = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let key = self.prop_value(&key_type, key_struct, &key_path, stats)?;
            let value = self.prop_value(&value_type, value_struct, &value_path, stats)?;
            entries.push((key, value));
        }
        Ok(PropertyValue::Map {
            key_type,
            value_type,
            count,
            opaque: false,
            entries,
        })
    }

    pub fn prop_value(
        &mut self,
        type_name: &str,
        struct_type_name: &str,
        path: &str,
        stats: &mut ParseStats,
    ) -> Result<PropertyValue, ParseError> {
        match type_name {
            "StructProperty" => {
                let hint = type_hints::hint_for(path);
                let st = if !struct_type_name.is_empty() {
                    struct_type_name
                } else {
                    hint.unwrap_or("StructProperty")
                };
                if st == "Guid" || hint == Some("Guid") {
                    return Ok(PropertyValue::Guid(self.guid()?));
                }
                let fields = self.read_struct_value(st, path, stats)?;
                Ok(PropertyValue::Struct {
                    struct_type: st.into(),
                    fields,
                })
            }
            "EnumProperty" | "NameProperty" | "StrProperty" => {
                Ok(PropertyValue::Name(self.fstring()?))
            }
            "IntProperty" => Ok(PropertyValue::Int(self.i32()?)),
            "BoolProperty" => Ok(PropertyValue::Bool(self.bool_byte()?)),
            "UInt32Property" => Ok(PropertyValue::Int(self.u32()? as i32)),
            "Int64Property" => Ok(PropertyValue::Int64(self.i64()?)),
            other => {
                stats.unsupported_types += 1;
                Err(ParseError::Unsupported(format!(
                    "prop_value type {other} @ {path}"
                )))
            }
        }
    }
}
