# CURRENT

## Phase
Palworld desktop manager — Save parser Phase 1（decode-only）

## Objective
Palhelm 方針の Rust Save パーサを実セーブ（主に PlM）まで通す

## Current task
次エージェント: **PlM / Oodle 配線**（最大のブロッカー）

## Completed
- 運用一式: SteamCMD / start-stop / backup / Discord / REST GUI / update check / secrets(keyring)
- Save: container(PlZ) / GVAS / type_hints / Character·Group·BaseCamp RawData
- UI: `save_parser_status` Snapshot（parsedPlayers / pals / guilds / bases / baseMarkers）

## In progress / gaps
- Oodle Mermaid（`oodle.rs` は Unsupported stub）
- プレイヤー座標マーカー
- fixture / ベンチ

## Blockers
- 現行 1.0 セーブはほぼ PlM → Oodle なしでは実機検証不可

## Immediate next
1. PalSav-Flex / PalworldSaveTools を参考に Oodle 抽象を配線（DLL 同梱しない・hash 検証）
2. 実 Level.sav で players/guilds 確認
3. Location → playerMarkers
