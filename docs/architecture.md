# Architecture

## Layers

```text
Desktop GUI (Tauri + React)
        │
   Core (game-agnostic)
        │
   Providers (palworld, …)
        │
Integrations (discord)  ← outside Core
```

- UI → Core → Provider（一方通行）
- Discord は `packages/integrations/palworld-discord`（**ゲーム専用**。共通は Core のみ）
- Palworld は REST のみ（RCON 非サポート）

## Discovery

`{appRoot}/Servers/{任意名}/` をインスタンスとし、中の exe マーカーで Provider を判定する。

## Backup

Core がスケジュール・保持数・レストア二段階を担当。Provider が `backupPaths`（Palworld は `Pal/Saved/`）を返す。

## Update check（Steam / buildid）

```text
UpdateChecker → LatestBuildIdSource（AppInfo API）
             → Installed buildid（ACF）
             → 比較のみ
SteamCMD     → install / app_update のみ（Build ID 取得には使わない）
```

- `+app_info_print` の stdout は使わない（環境によって排出されない）
- Binary `appinfo.vdf` は将来 fallback（UTF-8 として読まない）
- Discord Integration: インスタンス GUI で Token / Guild / Channel / 通知を設定（ゲームごと独立）。ランタイムは desktop Rust

## Save parse（Palworld）

decode-only。実装は `apps/desktop/src-tauri/src/palworld_save/`（Palhelm 方針・Core 外）。  
PlZ→GVAS→Character/Group/BaseCamp RawData まで接続。**PlM/Oodle 未配線**。詳細は `docs/dev/`。
