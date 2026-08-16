# Agent Handoff

## Status
In Progress

## What Changed
- `palworld_save/` 実装: PlZ→GVAS→type_hints→Character/Group/BaseCamp RawData
- `save_parser.rs` 接続（`parsedPlayers` / pals / guilds / bases / baseMarkers）
- Port Map・参照順を `docs/dev/` に定着
- セキュリティ詳細の二重管理を整理（正本は `.cursor/rules/security.mdc`）

## Files Changed
- `apps/desktop/src-tauri/src/palworld_save/**`
- `apps/desktop/src-tauri/src/save_parser.rs`
- `apps/desktop/src-tauri/src/lib.rs`（`mod palworld_save`）
- `apps/desktop/src-tauri/Cargo.toml`（`flate2`）
- `docs/dev/palworld-save-parser*.md`, `save-parser-status.md`
- `docs/security.md`, `docs/architecture.md`, `README.md`
- `.cursor/context/*`（本セッション）

## Current State
- PlZ パスはコンパイル・配線済み（実 PlZ fixture 未）
- PlM は `needs_oodle` で停止
- RawData デコーダはコード上あるが、本番セーブ検証は Oodle 待ち
- セキュリティ Always ルール: `.cursor/rules/security.mdc`
- 採用済みセキュリティ要約: `docs/security.md`

## Important Discoveries
- SteamCMD `app_info_print` は Build ID 取得に使わない（AppInfo API + ACF）
- Guild RawData は cheahjs 0.24 不可。oMaN-Rod 1.0 + guild tail v1/v2（EOF 判別）、不明尾は tolerated
- プレイヤー判定は `IsPlayer` のみ（表示名で推論しない）
- 参照順: Palhelm spec → `internal/sav` → oMaN-Rod/palsav → PalSav-Flex(Oodle) → JS toolkit

## Decisions
- Save パーサは desktop Rust（Core 外）、decode-only Phase 1
- Discord はゲーム専用 Integration（Core に入れない）
- 秘密は keyring；詳細ルールは security.mdc に一本化

## Known Issues
- `oodle.rs` stub
- プレイヤー Location 未抽出 → `playerMarkers` 空
- fixture / ベンチ未整備
- `architecture.mdc` / `react.mdc` / `rust.mdc` は workflow 記載あり・未作成

## Next Task
PlM + Oodle 配線（参照はまず PalSav-Flex、次に Rust PalworldSaveTools）。フロントから任意 DLL ロード不可。ピン留め hash・原子的配置。

## Do Not
- cheahjs 0.24 Guild を直移植しない
- 全 RawData を JSON 化しない / write-back しない
- 欠落フィールドを捏造しない / panic しない
- セキュリティ長文を context や docs/dev に再複製しない（`security.mdc` を読む）
- SteamCMD で Build ID を取ろうとしない
