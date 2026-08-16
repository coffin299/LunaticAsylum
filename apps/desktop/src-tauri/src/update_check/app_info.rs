//! 最新 Build ID のデータソース抽象。
//! 実装詳細（HTTP / Binary VDF）はここより下に閉じ込める。

/// 外部（またはローカルキャッシュ）から最新 buildid を取る。
pub trait LatestBuildIdSource {
    fn name(&self) -> &str;
    fn fetch_latest_build_id(&self, app_id: u32, branch: &str) -> Result<String, String>;
}

/// 将来用: SteamCMD の Binary `appinfo.vdf`（v41）を読む fallback。
/// **UTF-8 テキストとしては解析しない。** 現状は無効スタブ。
pub struct BinaryAppInfoVdfSource {
    enabled: bool,
    #[allow(dead_code)]
    cache_path: Option<std::path::PathBuf>,
}

impl BinaryAppInfoVdfSource {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            cache_path: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_cache_path(path: std::path::PathBuf) -> Self {
        Self {
            enabled: true,
            cache_path: Some(path),
        }
    }
}

impl LatestBuildIdSource for BinaryAppInfoVdfSource {
    fn name(&self) -> &str {
        "binary-appinfo-vdf"
    }

    fn fetch_latest_build_id(&self, _app_id: u32, _branch: &str) -> Result<String, String> {
        if !self.enabled {
            return Err("Binary AppInfo VDF fallback is not enabled yet".into());
        }
        Err("Binary AppInfo VDF (v41) parser is not implemented yet".into())
    }
}
