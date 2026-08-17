# LunaticAsylum

> ## 🚧 UNDER DEVELOPMENT
>
> **このリポジトリは現在開発中です。**  
> API・設定・セーブ解析・配布形式は予告なく変わる可能性があります。  
> 本番運用・本番セーブの取り扱いは自己責任でお願いします。

---

ゲームサーバー管理デスクトップアプリ（Windows 優先）。  
`LunaticAsylum.exe` を起動し、隣の `Servers/` に好きな名前のフォルダでサーバーを置くと自動検出します。

## 現状

- SteamCMD インストール、起動/停止/再起動（停止時は REST `/stop` → プロセスツリー終了）、定期バックアップ（事前 REST save）、二段階レストア、REST GUI、クラッシュ再起動
- 更新検知: AppInfo API ポーリング / 手動 / 任意自動適用
- Discord 連携（OS Credential Store に Token）
- Save: Port Map 準拠で Character/Group/BaseCamp RawData まで接続（PlM は Oodle ライブラリを初回取得）。巨大 MapObject は size スキップしてギルド/拠点まで到達する
- Map: 拠点 transform（Level.sav）/ プレイヤー LastTransform（Players/*.sav）
- Minecraft: detect stub（Coming later）
- 製品サイト骨格: `apps/webpage`（Pages は `webpage` ブランチ）
- 配布: 当面未署名ポータブル ZIP

## 開発

要件: Node 20+、pnpm 9、Rust（Tauri 2）

```bat
corepack enable
corepack prepare pnpm@9.15.0 --activate
pnpm install
pnpm dev
```

## ドキュメント

- [docs/architecture.md](docs/architecture.md)
- [docs/provider-guide.md](docs/provider-guide.md)
- [docs/discord.md](docs/discord.md)
- [docs/security-adoption.md](docs/security-adoption.md)
- [.github/SECURITY.md](.github/SECURITY.md)（脆弱性報告ポリシー）
- [docs/code-signing.md](docs/code-signing.md)
- [docs/palworld-save-migration.md](docs/palworld-save-migration.md)

## ライセンス

MIT
