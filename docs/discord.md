# Discord（Palworld 専用）

Discord は **ゲーム横断で再利用しない**。

- 共通のサーバー管理: `packages/core` / Providers
- Palworld の Discord: `packages/integrations/palworld-discord`（型）+ desktop Rust ランタイム
- 将来の Minecraft Discord: `packages/integrations/minecraft-discord`（別パッケージ）

## エンドユーザー設定（GUI）

インスタンス詳細の **Discord 連携** パネルから:

1. Bot Token（Discord Developer Portal でユーザー自身が取得）
2. Guild ID / 通知チャンネル ID
3. 管理者 Discord ユーザー ID（カンマ区切り）
4. 入退室・REST 障害/復旧・トピック更新のオンオフ
5. **設定を保存** → **連携を適用（Bot 起動）**

## 秘密の保存

- Discord Bot Token / REST AdminPassword は **OS 資格情報ストア**（Windows Credential Manager）
- `config.json` には **書かない**（旧平文があれば起動時に移行して消去）
- GUI には中身を返さない（保存済みフラグのみ）。変更時だけ再入力
- 開発者側 Token は使わない

詳細: [`docs/security.md`](./security.md) / `.cursor/rules/security.mdc`（API キー / リモート管理等の将来項目は未実装）

## コマンド

通知: 入退出、REST 障害/復旧、起動ステータス、トピック

スラッシュ: `/help` `/players` `/info` `/metrics` `/settings` `/kick` `/ban` `/unban` `/announce` `/save` `/shutdown` `/stop`

`/shutdown`・`/stop` は Confirm 必須。意図的停止フラグと連携しクラッシュ再起動に引っかからない。
