//! SteamCMD.net の公開 AppInfo API（第三者がキャッシュした AppInfo JSON）。
//! SteamCMD 本体とは別経路。インストール用途には使わない。

use super::app_info::LatestBuildIdSource;

const DEFAULT_BASE: &str = "https://api.steamcmd.net/v1/info";

pub struct SteamCmdNetAppInfoSource {
    base_url: String,
}

impl Default for SteamCmdNetAppInfoSource {
    fn default() -> Self {
        Self {
            base_url: DEFAULT_BASE.to_string(),
        }
    }
}

impl LatestBuildIdSource for SteamCmdNetAppInfoSource {
    fn name(&self) -> &str {
        "steamcmd-net"
    }

    fn fetch_latest_build_id(&self, app_id: u32, branch: &str) -> Result<String, String> {
        let url = format!("{}/{app_id}", self.base_url.trim_end_matches('/'));
        let resp = ureq::get(&url)
            .set("Accept", "application/json")
            .call()
            .map_err(|e| format!("HTTP failed: {e}"))?;

        let status = resp.status();
        if !(200..300).contains(&status) {
            return Err(format!("HTTP status {status}"));
        }

        let body = resp
            .into_string()
            .map_err(|e| format!("read body failed: {e}"))?;
        let json: serde_json::Value =
            serde_json::from_str(&body).map_err(|e| format!("JSON parse failed: {e}"))?;

        // data."<appid>".depots.branches."<branch>".buildid
        let app_key = app_id.to_string();
        let path = format!("/data/{app_key}/depots/branches/{branch}/buildid");
        let value = json.pointer(&path).ok_or_else(|| {
            format!("buildid not found for app {app_id} branch '{branch}' in steamcmd.net response")
        })?;

        let build_id = value
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| value.as_u64().map(|n| n.to_string()))
            .or_else(|| value.as_i64().map(|n| n.to_string()))
            .ok_or_else(|| "buildid has unexpected type".to_string())?;

        if build_id.is_empty() {
            return Err("empty buildid".into());
        }
        Ok(build_id)
    }
}
