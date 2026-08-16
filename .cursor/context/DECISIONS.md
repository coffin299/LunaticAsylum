# DECISIONS

## Architecture
- **Layers**: UI → Core → Provider。Discord は Provider ではなく **ゲーム専用 Integration**。
- **Palworld**: REST のみ（RCON なし）。
- **Discovery**: `{appRoot}/Servers/{name}/` + exe マーカー。
- **Update check**: AppInfo API で latest、ACF で installed。SteamCMD は install/`app_update` のみ。

## Save parser
- **配置**: `apps/desktop/src-tauri/src/palworld_save/`（Core 外）。
- **方針**: Palhelm decode-only。Phase 1 は Character / Group /（必要なら）BaseCamp RawData のみ。
- **参照順**: Palhelm sav-parser → `internal/sav` → oMaN-Rod/palsav → PalSav-Flex(Oodle) → JS toolkit。
- **禁止**: cheahjs 0.24 Guild 直移植。write-back は Phase 1 外。

## Secrets / security
- 正本: `.cursor/rules/security.mdc`（Always）。公開ポリシー: `.github/SECURITY.md`。採用要約: `docs/security-adoption.md`。
- Discord Token / REST password → OS Credential Store（`keyring`）。`config.json` に平文秘密を置かない。
- 当面除外: リモート管理 API、多ユーザー、Argon2 ローカルユーザー、署名付き release。

## Distribution
- 当面 unsigned ポータブル ZIP。SignPath は後続。
