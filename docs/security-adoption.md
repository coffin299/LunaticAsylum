# Security adoption summary

このファイルは **採用済み対策の社内/開発向け要約** です。  
GitHub の Security タブ（脆弱性報告ポリシー）は [`.github/SECURITY.md`](../.github/SECURITY.md) を見てください。

詳細ルールの正本: [`.cursor/rules/security.mdc`](../.cursor/rules/security.mdc)

## Implemented now

- Discord Token / REST admin password → OS Credential Store (`keyring`)
- `config.json` holds non-secrets only
- Validation for instance id / backup name / REST URL (loopback-oriented for now)
- ZIP restore: Zip Slip rejection + file-count / uncompressed-size limits
- Frontend never receives secret plaintext back from the backend

## Deferred

- Integration API keys / Argon2 local user passwords
- Remote management API / multi-user
- Signed releases (unsigned ZIP for now)
- Advanced audit UI / external secret managers
