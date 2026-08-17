//! Oodle Mermaid（PlM）。
//!
//! Palhelm `sav/oodle.go` と同じ方針:
//! 明示パス → data dir のキャッシュ → ピン留め成果物を download + SHA-256。
//! ライブラリは同梱しない。フロントからパスを受け取らない。実行もしない。

use super::error::ParseError;
use libloading::{Library, Symbol};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Duration;

/// 展開後 GVAS の上限（セーブ制御の巨大確保を防ぐ）
const MAX_UNCOMPRESSED: usize = 512 * 1024 * 1024;
const MAX_DOWNLOAD_BYTES: u64 = 8 * 1024 * 1024;
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

#[cfg(windows)]
const LIB_NAME: &str = "oo2core_9_win64.dll";
#[cfg(windows)]
const LIB_URL: &str =
    "https://github.com/new-world-tools/go-oodle/releases/download/v0.2.3-files/oo2core_9_win64.dll";
/// go-oodle v0.2.3-files `oo2core_9_win64.dll`（実測）
#[cfg(windows)]
const LIB_SHA256: &str = "6f5d41a7892ea6b2db420f2458dad2f84a63901c9a93ce9497337b16c195f457";

#[cfg(all(unix, not(target_os = "macos")))]
const LIB_NAME: &str = "liboo2corelinux64.so.9";
#[cfg(all(unix, not(target_os = "macos")))]
const LIB_URL: &str =
    "https://github.com/new-world-tools/go-oodle/releases/download/v0.2.3-files/liboo2corelinux64.so.9";
/// Palhelm がピン留めしている go-oodle v0.2.3-files Linux 成果物
#[cfg(all(unix, not(target_os = "macos")))]
const LIB_SHA256: &str = "7354655eb25b587dc34cbf98696b91e30e6d7a3f0eefad3872e6c1b76ef86a6e";

type OodleLzDecompressFn = unsafe extern "C" fn(
    comp_buf: *const u8,
    comp_buf_size: isize,
    raw_buf: *mut u8,
    raw_len: isize,
    fuzz_safe: i32,
    check_crc: i32,
    verbosity: i32,
    dec_buf_base: *mut u8,
    dec_buf_size: isize,
    fp_callback: usize,
    callback_user_data: usize,
    decoder_memory: *mut u8,
    decoder_memory_size: isize,
    thread_phase: i32,
) -> isize;

static LIBRARY: OnceLock<Result<Library, String>> = OnceLock::new();

/// PlM を GVAS バイト列に展開する。
pub fn decompress_mermaid(
    payload: &[u8],
    uncompressed_len: usize,
    compressed_len: usize,
) -> Result<Vec<u8>, ParseError> {
    if uncompressed_len == 0 || uncompressed_len > MAX_UNCOMPRESSED {
        return Err(ParseError::Format(format!(
            "PlM uncompressed_len out of range: {uncompressed_len}"
        )));
    }
    if compressed_len == 0 || compressed_len > payload.len() {
        return Err(ParseError::Format(format!(
            "PlM compressed_len mismatch: header={compressed_len} actual={}",
            payload.len()
        )));
    }
    let src = &payload[..compressed_len];
    let lib = library()?;
    let mut out = vec![0u8; uncompressed_len];
    let written = unsafe {
        let decompress: Symbol<OodleLzDecompressFn> = lib
            .get(b"OodleLZ_Decompress\0")
            .map_err(|e| ParseError::Unsupported(format!("OodleLZ_Decompress missing: {e}")))?;
        decompress(
            src.as_ptr(),
            src.len() as isize,
            out.as_mut_ptr(),
            out.len() as isize,
            0, // FuzzSafe_No（go-oodle / PalSav-Flex と同じ）
            0, // CheckCRC_No
            0, // Verbosity_None
            std::ptr::null_mut(),
            0,
            0,
            0,
            std::ptr::null_mut(),
            0,
            3, // DecodeThreadPhase_All
        )
    };
    if written <= 0 {
        return Err(ParseError::Format(
            "OodleLZ_Decompress failed (empty or error)".into(),
        ));
    }
    let written = written as usize;
    if written != uncompressed_len {
        return Err(ParseError::Format(format!(
            "Oodle uncompressed_len mismatch: header={uncompressed_len} actual={written}"
        )));
    }
    Ok(out)
}

fn library() -> Result<&'static Library, ParseError> {
    match LIBRARY.get_or_init(|| load_library().map_err(|e| e.to_string())) {
        Ok(lib) => Ok(lib),
        Err(e) => Err(ParseError::Unsupported(e.clone())),
    }
}

fn load_library() -> Result<Library, ParseError> {
    let path = resolve_oodle_library()?;
    // SAFETY: パスは絶対パスの通常ファイル。ダウンロード分は SHA-256 検証済み。実行はしない。
    let lib = unsafe { Library::new(&path) }.map_err(|e| {
        ParseError::Unsupported(format!("failed to load Oodle library {}: {e}", path.display()))
    })?;
    Ok(lib)
}

fn resolve_oodle_library() -> Result<PathBuf, ParseError> {
    if let Some(explicit) = std::env::var_os("LUNATIC_ASYLUM_OODLE_LIB") {
        let path = PathBuf::from(explicit);
        if !path.is_absolute() {
            return Err(ParseError::Unsupported(
                "LUNATIC_ASYLUM_OODLE_LIB must be an absolute path".into(),
            ));
        }
        validate_regular_file(&path)?;
        return Ok(path);
    }

    #[cfg(target_os = "macos")]
    {
        return Err(ParseError::Unsupported(
            "PlM/Oodle on macOS requires LUNATIC_ASYLUM_OODLE_LIB (no pinned download)".into(),
        ));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let dest = data_dir()?.join(LIB_NAME);
        if dest.is_file() {
            verify_sha256(&dest, LIB_SHA256)?;
            return Ok(dest);
        }
        download_oodle(&dest)?;
        Ok(dest)
    }
}

fn data_dir() -> Result<PathBuf, ParseError> {
    let root = crate::paths::app_root().map_err(ParseError::Io)?;
    Ok(root.join("data"))
}

fn validate_regular_file(path: &Path) -> Result<(), ParseError> {
    let meta = fs::metadata(path).map_err(|e| {
        ParseError::Unsupported(format!("Oodle library {}: {e}", path.display()))
    })?;
    if !meta.is_file() {
        return Err(ParseError::Unsupported(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    Ok(())
}

fn verify_sha256(path: &Path, expected: &str) -> Result<(), ParseError> {
    let bytes = fs::read(path).map_err(ParseError::from)?;
    let got = sha256_hex(&bytes);
    if got != expected {
        return Err(ParseError::Unsupported(format!(
            "Oodle library SHA-256 mismatch: got {got}, want {expected}"
        )));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn download_oodle(dest: &Path) -> Result<(), ParseError> {
    let dir = dest.parent().ok_or_else(|| {
        ParseError::Io("Oodle destination has no parent directory".into())
    })?;
    fs::create_dir_all(dir)?;

    let resp = ureq::get(LIB_URL)
        .timeout(DOWNLOAD_TIMEOUT)
        .call()
        .map_err(|e| ParseError::Unsupported(format!("Oodle download failed: {e}")))?;
    if !(200..300).contains(&resp.status()) {
        return Err(ParseError::Unsupported(format!(
            "Oodle download returned HTTP {}",
            resp.status()
        )));
    }

    let mut limited = resp.into_reader().take(MAX_DOWNLOAD_BYTES + 1);
    let mut bytes = Vec::new();
    limited.read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_DOWNLOAD_BYTES {
        return Err(ParseError::Unsupported(format!(
            "Oodle download exceeds {MAX_DOWNLOAD_BYTES} bytes"
        )));
    }

    let got = sha256_hex(&bytes);
    if got != LIB_SHA256 {
        return Err(ParseError::Unsupported(format!(
            "Oodle download SHA-256 mismatch: got {got}, want {LIB_SHA256}"
        )));
    }

    let tmp_path = dest.with_extension("tmp");
    {
        let mut tmp = File::create(&tmp_path)?;
        tmp.write_all(&bytes)?;
        tmp.sync_all()?;
    }
    fs::rename(&tmp_path, dest).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        ParseError::Io(format!("atomic install of Oodle library failed: {e}"))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_uncompressed() {
        let err = decompress_mermaid(&[1, 2, 3], 0, 3).unwrap_err();
        assert!(err.to_string().contains("uncompressed_len"));
    }

    #[test]
    fn rejects_huge_uncompressed() {
        let err = decompress_mermaid(&[1, 2, 3], MAX_UNCOMPRESSED + 1, 3).unwrap_err();
        assert!(err.to_string().contains("uncompressed_len"));
    }

    #[test]
    fn rejects_compressed_len_past_payload() {
        let err = decompress_mermaid(&[1, 2, 3], 16, 99).unwrap_err();
        assert!(err.to_string().contains("compressed_len"));
    }

    #[test]
    fn sha256_hex_known() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
