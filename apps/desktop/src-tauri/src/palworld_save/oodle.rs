//! Oodle Mermaid（PlM）。外部 libooz 接続は後続。

use super::error::ParseError;

/// PlM を GVAS バイト列に展開する。
/// 現状は DLL 未同梱のため明示的に Unsupported を返す（推測展開しない）。
pub fn decompress_mermaid(
    _payload: &[u8],
    _uncompressed_len: usize,
    _compressed_len: usize,
) -> Result<Vec<u8>, ParseError> {
    Err(ParseError::Unsupported(
        "PlM (Oodle Mermaid) requires libooz / Oodle binding — not wired yet".into(),
    ))
}
