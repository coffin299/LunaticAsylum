//! 更新検知のオーケストレーション。
//! SteamCMD の stdout / Binary VDF を Provider に露出しない。

mod app_info;
mod installed;
mod steamcmd_net;

pub use app_info::{BinaryAppInfoVdfSource, LatestBuildIdSource};
pub use installed::read_installed_build_id;
pub use steamcmd_net::SteamCmdNetAppInfoSource;

/// 更新チェック結果（UI / 呼び出し側向けの薄い DTO）
#[derive(Debug, Clone)]
#[allow(dead_code)] // installed/latest/source は今後 UI・ログで利用
pub struct UpdateCheckResult {
    pub update_available: bool,
    pub installed_build_id: Option<String>,
    pub latest_build_id: Option<String>,
    /// 最新 buildid を返したソース名（失敗時は None）
    pub latest_source: Option<String>,
}

/// 複数の AppInfo ソースを順に試し、インストール済み buildid と比較する。
pub struct UpdateChecker<'a> {
    sources: Vec<&'a dyn LatestBuildIdSource>,
}

impl<'a> UpdateChecker<'a> {
    pub fn new(sources: Vec<&'a dyn LatestBuildIdSource>) -> Self {
        Self { sources }
    }

    /// `app_id` / `branch`（通常 `public`）で最新とローカルを比較する。
    pub fn check(
        &self,
        app_id: u32,
        install_dir: &std::path::Path,
        branch: &str,
    ) -> Result<UpdateCheckResult, String> {
        let installed = read_installed_build_id(install_dir, app_id);

        let mut last_err = String::from("no AppInfo sources configured");
        let mut latest: Option<(String, String)> = None;

        for source in &self.sources {
            match source.fetch_latest_build_id(app_id, branch) {
                Ok(build_id) => {
                    latest = Some((source.name().to_string(), build_id));
                    break;
                }
                Err(e) => {
                    last_err = format!("{}: {e}", source.name());
                }
            }
        }

        let Some((source_name, latest_build_id)) = latest else {
            return Err(last_err);
        };

        let update_available = match &installed {
            Some(local) => local != &latest_build_id,
            None => false,
        };

        Ok(UpdateCheckResult {
            update_available,
            installed_build_id: installed,
            latest_build_id: Some(latest_build_id),
            latest_source: Some(source_name),
        })
    }
}

/// Palworld Dedicated Server 向けの既定チェッカー（SteamCMD.net 本線）。
/// Binary VDF は将来 fallback としてチェーンに足せる。
pub fn default_palworld_checker() -> (SteamCmdNetAppInfoSource, BinaryAppInfoVdfSource) {
    (
        SteamCmdNetAppInfoSource::default(),
        BinaryAppInfoVdfSource::disabled(),
    )
}
