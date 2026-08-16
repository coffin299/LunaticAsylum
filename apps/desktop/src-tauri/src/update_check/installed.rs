//! インストール済み Build ID（appmanifest ACF・テキスト KeyValues）。

use std::fs;
use std::path::Path;

/// `appmanifest_{app_id}.acf` から `"buildid"` を読む。
/// 候補パスを順に探し、最初に見つかった値を返す。
pub fn read_installed_build_id(install_dir: &Path, app_id: u32) -> Option<String> {
    let name = format!("appmanifest_{app_id}.acf");
    let candidates = [
        install_dir.join("steamapps").join(&name),
        install_dir.join(&name),
    ];

    for path in candidates {
        if let Ok(text) = fs::read_to_string(&path) {
            if let Some(id) = extract_acf_build_id(&text) {
                return Some(id);
            }
        }
    }
    None
}

/// ACF テキストから `"buildid" "…"` を拾う（浅い KeyValues 想定）。
fn extract_acf_build_id(text: &str) -> Option<String> {
    for line in text.lines() {
        let t = line.trim();
        if !t.starts_with('"') {
            continue;
        }
        // "buildid" "12345"
        let mut parts = t.split('"').filter(|s| !s.trim().is_empty());
        let key = parts.next()?;
        if key != "buildid" {
            continue;
        }
        let value = parts.next()?.trim();
        if !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_buildid_line() {
        let sample = r#"
"AppState"
{
	"appid"		"2394010"
	"buildid"		"24575149"
}
"#;
        assert_eq!(
            extract_acf_build_id(sample).as_deref(),
            Some("24575149")
        );
    }
}
