# Provider guide

新しいゲームを足すときは Provider を追加し、Capabilities で UI を駆動する。

## 必須

- `detect(path)` — フォルダ内マーカーで判定
- lifecycle hooks（start/stop）
- Capabilities フラグ

## Palworld

- REST: Rust `palworld_rest` + `rest_ops`（desktop）
- **REST 接続設定**: `PalWorldSettings.ini` の `RESTAPIEnabled` / `RESTAPIPort` / `AdminPassword` と双方向同期。移行時は ini を先に読み keyring / config を bootstrap
- Backup paths: `Pal/Saved/`
- RCON: なし（ini に RCONPort あっても LunaticAsylum では未使用）
- AppID: `2394010`（Steam バナー CDN にも使用）
- ゲーム設定: `Pal/Saved/Config/WindowsServer/PalWorldSettings.ini`（GUI Raw 編集 + REST 設定フォーム）
- 更新検知: desktop の `UpdateChecker`（AppInfo API）。Provider は SteamCMD / AppInfo を扱わない
- 適用のみ SteamCMD `app_update`（検知は AppInfo API。ローカル詳細は `docs/dev/`）
- SteamCMD: 必要。配置は `{appRoot}/tools/steamcmd` 固定

## Minecraft

- 検出: `paper.jar` / `spigot.jar` / `server.jar` / `bedrock_server.exe` / 直下 `*.jar`
- 起動: `java` + JVM 引数 + `-jar` + jar（Rust `Command`、シェル経由禁止）
- **RCON（Java のみ）**: Source RCON。`server.properties` の `enable-rcon` / `rcon.port` / `rcon.password` と双方向同期。Bedrock は v1 非対応
- 種別: ユーザーが `serverType` を選択（Vanilla/Paper/Purpur/Fabric/NeoForge/Forge/Spigot/Other/Unknown）。UI 表示は `Minecraft-{Type}`
- ゲーム設定: インスタンス直下 `server.properties`（GUI Raw 編集 + RCON 設定フォーム）
- バナー: 自動取得なし（透過）。ユーザー画像オーバーライド可
- SteamCMD: 不要

## unknown / 空フォルダ

- 一覧には表示する
- `launchable: false` — 起動・停止等の操作 UI は非表示
- `start_server` は Rust で拒否
