# Security（採用サマリ）

詳細ルールの正本: [`.cursor/rules/security.mdc`](../.cursor/rules/security.mdc)（Always）。

## いま採用して実装したもの

- Discord Token / REST パスワード → **OS Credential Store**（`keyring`）
- `config.json` は非秘密のみ
- インスタンス ID / バックアップ名 / REST URL（当面 loopback のみ）の検証
- ZIP レストアの Zip Slip 拒否 + ファイル数・展開サイズ上限
- フロントに秘密の中身を返さない

## いま除外（将来）

- Integration API キー / Argon2 ユーザーパスワード
- リモート管理 API / 多ユーザー
- 署名付き Release（当面 unsigned ZIP）
- 高度な監査 UI / 外部シークレットマネージャ
