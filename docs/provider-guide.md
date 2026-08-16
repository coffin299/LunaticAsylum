# Provider guide

新しいゲームを足すときは Provider を追加し、Capabilities で UI を駆動する。

## 必須

- `detect(path)` — フォルダ内マーカーで判定
- lifecycle hooks（start/stop）
- Capabilities フラグ

## Palworld

- REST: `@lunatic-asylum/provider-palworld` の `PalworldRestClient`
- Backup paths: `Pal/Saved/`
- RCON: なし
- AppID: `2394010`
- 更新検知: desktop の `UpdateChecker`（AppInfo API）。Provider は SteamCMD / AppInfo を扱わない
- 適用のみ SteamCMD `app_update`（検知は AppInfo API。ローカル詳細は `docs/dev/`）

## Minecraft

次期実装。detect stub（`server.jar` / Paper 等）と Capabilities のみ。Coming later。
