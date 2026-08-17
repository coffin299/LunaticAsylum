//! 座標抽出（FTransform / Vector）。フィールドが無ければ None（捏造しない）。

use super::properties::PropertyValue;
use std::collections::BTreeMap;

/// Struct フィールドから (x, y, z) を取り出す（x/X など UE 表記ゆれ対応）。
pub fn xyz_from_fields(fields: &BTreeMap<String, PropertyValue>) -> Option<(f64, f64, f64)> {
    Some((
        as_f64(fields.get("x").or_else(|| fields.get("X"))?)?,
        as_f64(fields.get("y").or_else(|| fields.get("Y"))?)?,
        as_f64(fields.get("z").or_else(|| fields.get("Z"))?)?,
    ))
}

pub fn xyz_from_value(value: &PropertyValue) -> Option<(f64, f64, f64)> {
    match value {
        PropertyValue::Struct {
            struct_type,
            fields,
        } => {
            if struct_type == "Vector" || struct_type == "Vector3d" {
                return xyz_from_fields(fields);
            }
            if let Some(xyz) = xyz_from_fields(fields) {
                return Some(xyz);
            }
            for key in ["translation", "Translation", "Location"] {
                if let Some(v) = fields.get(key) {
                    if let Some(xyz) = xyz_from_value(v) {
                        return Some(xyz);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

/// SaveParameter 等から LastTransform / Transform / Location を探す。
pub fn find_transform_xyz(fields: &BTreeMap<String, PropertyValue>) -> Option<(f64, f64, f64)> {
    for name in ["LastTransform", "Transform", "Location"] {
        if let Some(PropertyValue::Struct { fields: tf, .. }) = fields.get(name) {
            if let Some(xyz) = xyz_from_transform_fields(tf) {
                return Some(xyz);
            }
        }
    }
    None
}

fn xyz_from_transform_fields(tf: &BTreeMap<String, PropertyValue>) -> Option<(f64, f64, f64)> {
    for key in ["translation", "Translation", "Location"] {
        if let Some(v) = tf.get(key) {
            if let Some(xyz) = xyz_from_value(v) {
                return Some(xyz);
            }
        }
    }
    xyz_from_fields(tf)
}

fn as_f64(value: &PropertyValue) -> Option<f64> {
    match value {
        PropertyValue::Double(v) => Some(*v),
        PropertyValue::Float(v) => Some(*v as f64),
        PropertyValue::Int(v) => Some(*v as f64),
        PropertyValue::Int64(v) => Some(*v as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn ftransform_translation_xyz() {
        let mut translation = BTreeMap::new();
        translation.insert("X".into(), PropertyValue::Double(100.0));
        translation.insert("Y".into(), PropertyValue::Double(200.0));
        translation.insert("Z".into(), PropertyValue::Double(50.0));
        let mut tf = BTreeMap::new();
        tf.insert(
            "Translation".into(),
            PropertyValue::Struct {
                struct_type: "Vector".into(),
                fields: translation,
            },
        );
        let (x, y, z) = xyz_from_transform_fields(&tf).unwrap();
        assert!((x - 100.0).abs() < f64::EPSILON);
        assert!((y - 200.0).abs() < f64::EPSILON);
        assert!((z - 50.0).abs() < f64::EPSILON);
    }
}
