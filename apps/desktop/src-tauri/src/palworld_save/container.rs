//! SAV コンテナ（PlZ zlib / PlM Oodle）

use super::error::ParseError;
use super::oodle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    PlZ,
    PlM,
    CnkThenPlZ,
    CnkThenPlM,
}

#[derive(Debug, Clone)]
pub struct DecompressedSav {
    pub kind: ContainerKind,
    pub save_type: u8,
    pub gvas: Vec<u8>,
}

pub fn decompress_sav(data: &[u8]) -> Result<DecompressedSav, ParseError> {
    if data.len() < 12 {
        return Err(ParseError::Format("SAV too small".into()));
    }

    let mut uncompressed_len = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let mut compressed_len = u32::from_le_bytes(data[4..8].try_into().unwrap()) as usize;
    let mut magic = &data[8..11];
    let mut save_type = data[11];
    let mut offset = 12usize;
    let mut kind = ContainerKind::PlZ;

    if magic == b"CNK" {
        if data.len() < 24 {
            return Err(ParseError::Format("CNK header truncated".into()));
        }
        uncompressed_len = u32::from_le_bytes(data[12..16].try_into().unwrap()) as usize;
        compressed_len = u32::from_le_bytes(data[16..20].try_into().unwrap()) as usize;
        magic = &data[20..23];
        save_type = data[23];
        offset = 24;
        kind = ContainerKind::CnkThenPlZ;
    }

    match magic {
        b"PlZ" => {
            let payload = data
                .get(offset..)
                .ok_or_else(|| ParseError::Eof("compressed payload".into()))?;
            let gvas = decompress_plz(payload, uncompressed_len, compressed_len, save_type)?;
            ensure_gvas_magic(&gvas)?;
            Ok(DecompressedSav {
                kind,
                save_type,
                gvas,
            })
        }
        b"PlM" => {
            if matches!(kind, ContainerKind::CnkThenPlZ) {
                kind = ContainerKind::CnkThenPlM;
            } else {
                kind = ContainerKind::PlM;
            }
            // Palhelm: PlM save_type 0x31 = Oodle Mermaid
            if save_type != 0x31 {
                return Err(ParseError::Unsupported(format!(
                    "unhandled PlM save_type 0x{save_type:02x}"
                )));
            }
            let payload = data
                .get(offset..)
                .ok_or_else(|| ParseError::Eof("oodle payload".into()))?;
            let gvas = oodle::decompress_mermaid(payload, uncompressed_len, compressed_len)?;
            ensure_gvas_magic(&gvas)?;
            Ok(DecompressedSav {
                kind,
                save_type,
                gvas,
            })
        }
        other => Err(ParseError::Format(format!(
            "unknown magic {other:?} (expected PlZ or PlM)"
        ))),
    }
}

fn decompress_plz(
    payload: &[u8],
    uncompressed_len: usize,
    compressed_len: usize,
    save_type: u8,
) -> Result<Vec<u8>, ParseError> {
    // 0x31 single zlib, 0x32 double zlib（従来 PlZ）
    match save_type {
        0x31 => {
            if compressed_len != payload.len() {
                return Err(ParseError::Format(format!(
                    "PlZ compressed_len mismatch: header={compressed_len} actual={}",
                    payload.len()
                )));
            }
            let out = inflate_zlib(payload)?;
            if out.len() != uncompressed_len {
                return Err(ParseError::Format(format!(
                    "PlZ uncompressed_len mismatch: header={uncompressed_len} actual={}",
                    out.len()
                )));
            }
            Ok(out)
        }
        0x32 => {
            let mid = inflate_zlib(payload)?;
            if mid.len() != compressed_len {
                return Err(ParseError::Format(format!(
                    "PlZ double-zlib mid length mismatch: header={compressed_len} actual={}",
                    mid.len()
                )));
            }
            let out = inflate_zlib(&mid)?;
            if out.len() != uncompressed_len {
                return Err(ParseError::Format(format!(
                    "PlZ uncompressed_len mismatch: header={uncompressed_len} actual={}",
                    out.len()
                )));
            }
            Ok(out)
        }
        other => Err(ParseError::Unsupported(format!(
            "unhandled PlZ save_type 0x{other:02x}"
        ))),
    }
}

fn inflate_zlib(data: &[u8]) -> Result<Vec<u8>, ParseError> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut dec = ZlibDecoder::new(data);
    let mut out = Vec::new();
    dec.read_to_end(&mut out)
        .map_err(|e| ParseError::Format(format!("zlib decompress failed: {e}")))?;
    Ok(out)
}

fn ensure_gvas_magic(gvas: &[u8]) -> Result<(), ParseError> {
    if gvas.len() < 4 {
        return Err(ParseError::Format("decompressed data too small for GVAS".into()));
    }
    // little-endian 0x53415647 == b"GVAS"
    if &gvas[0..4] != b"GVAS" {
        return Err(ParseError::Format(format!(
            "decompressed data must start with GVAS, got {:?}",
            &gvas[0..4.min(gvas.len())]
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_too_small() {
        assert!(decompress_sav(&[0u8; 8]).is_err());
    }
}
