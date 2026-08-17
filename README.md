# LunaticAsylum

<p align="center">
  <a href="https://count.getloli.com/@LunaticAsylumServerManager">
    <img src="https://count.getloli.com/@LunaticAsylumServerManager?name=LunaticAsylumServerManager&theme=green&padding=7&offset=0&align=top&scale=1&pixelated=1&darkmode=0" alt="LunaticAsylumServerManager moecounter">
  </a>
</p>

<p align="center">
  <strong>Windows 向けゲームサーバー管理デスクトップアプリ</strong><br>
  <sub>起動・停止・バックアップ・設定編集・Discord 連携をひとつに</sub>
</p>

<p align="center">
  <a href="https://github.com/coffin299/LunaticAsylum"><img src="https://img.shields.io/github/v/release/coffin299/LunaticAsylum?label=Release&sort=semver&logo=github" alt="Release"></a>
  <a href="https://github.com/coffin299/LunaticAsylum/blob/main/LICENSE"><img src="https://img.shields.io/github/license/coffin299/LunaticAsylum?logo=opensourceinitiative&logoColor=white" alt="License MIT"></a>
  <a href="https://github.com/coffin299/LunaticAsylum/releases"><img src="https://img.shields.io/badge/Download-Portable%20ZIP-22C55E?logo=github&logoColor=white" alt="Download"></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Platform-Windows%20x64-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows x64">
  <img src="https://img.shields.io/badge/Tauri-2-FFC131?style=flat-square&logo=tauri&logoColor=black" alt="Tauri 2">
  <img src="https://img.shields.io/badge/Rust-stable-orange?style=flat-square&logo=rust&logoColor=white" alt="Rust stable">
  <img src="https://img.shields.io/badge/React-18-61DAFB?style=flat-square&logo=react&logoColor=black" alt="React">
  <img src="https://img.shields.io/badge/Node.js-%3E%3D20-339933?style=flat-square&logo=nodedotjs&logoColor=white" alt="Node >=20">
  <img src="https://img.shields.io/badge/pnpm-9.15-F69220?style=flat-square&logo=pnpm&logoColor=white" alt="pnpm 9">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Discord-Integration-5865F2?style=flat-square&logo=discord&logoColor=white" alt="Discord">
  <img src="https://img.shields.io/badge/Code%20Signing-Pending%20(SignPath)-F97316?style=flat-square" alt="Code Signing Pending">
</p>

[English](docs/README.en.md) · [Docs](docs/INDEX.md)

---

`LunaticAsylum.exe` を起動し、隣の `Servers/` に好きな名前のフォルダでサーバーを置くと **自動検出** されます。  
起動・停止・バックアップ・設定編集・Discord 連携まで GUI から一括管理できます。

> **運用ステータス**  
> 本番運用可能な機能セットが揃っています。  
> 配布 ZIP は **未署名ポータブル** のため SmartScreen 警告が出る場合があります。コード署名（SignPath）は申請待ちです。

---

## 機能

### コア

- SteamCMD インストール、起動 / 停止 / 再起動（停止時は REST `/stop` → プロセスツリー終了）
- 定期バックアップ（事前 REST save）、**二段階レストア**、クラッシュ再起動
- 更新検知: AppInfo API ポーリング / 手動 / 任意自動適用
- Discord 連携（Token は **OS Credential Store** に保存）
- UI: 単一ダークテーマ、Overview（PC リソース・バナー）
- 非対応 / 空フォルダは一覧表示・操作 UI 非表示
- **稼働中サーバーがあるとき終了確認**（全サーバー停止してから終了）

### Palworld

| | |
| :--- | :--- |
| 設定 GUI | `PalWorldSettings.ini` — **ini ↔ REST 双方向同期**（手動同期ボタン付き） |
| セーブ解析 | Port Map 準拠で Character / Group / BaseCamp RawData まで接続（PlM は Oodle を初回取得） |
| マップ | 拠点 transform（Level.sav）/ プレイヤー LastTransform（Players/*.sav） |
| その他 | Steam バナー、セーブ解析は後追いローディング |

### Minecraft

| | |
| :--- | :--- |
| 起動 | `java -jar`（種別: Vanilla / Paper 等をユーザー選択） |
| 設定 GUI | `server.properties` 編集 |
| RCON | **Java 版 Source RCON** — properties ↔ RCON 双方向同期（手動同期ボタン付き） |

### その他

- SteamCMD: `{appRoot}/tools/steamcmd` 固定。未取得時バナーから取得
- 製品サイト骨格: `apps/webpage`（Pages は `webpage` ブランチ）
- 配布: 未署名ポータブル ZIP（Cargo 中間ファイルは ZIP に含めない）

---

## クイックスタート

```text
LunaticAsylum/
├── LunaticAsylum.exe
├── Servers/
│   ├── my-palworld/       ← ここにサーバーフォルダを置く
│   └── my-minecraft/
└── tools/steamcmd/
```

1. [Releases](https://github.com/coffin299/LunaticAsylum/releases) から ZIP をダウンロード
2. 展開して `LunaticAsylum.exe` を起動
3. `Servers/` にサーバーフォルダを配置 → 自動検出

---

## 開発

要件: **Node 20+**、**pnpm 9**、**Rust**（Tauri 2）

```bat
corepack enable
corepack prepare pnpm@9.15.0 --activate
pnpm install
pnpm dev
```

| コマンド | 説明 |
| :--- | :--- |
| `pnpm dev` | Tauri 開発サーバー |
| `pnpm build` | 本番ビルド |

---

## ドキュメント

| ドキュメント | 内容 |
| :--- | :--- |
| [docs/INDEX.md](docs/INDEX.md) | ドキュメント索引 |
| [docs/README.en.md](docs/README.en.md) | プロジェクト概要（English） |
| [docs/architecture.md](docs/architecture.md) | レイヤ・Discovery・Backup・Update check |
| [docs/provider-guide.md](docs/provider-guide.md) | Provider 追加手順 |
| [docs/discord.md](docs/discord.md) | Discord 連携（Palworld 専用） |
| [docs/palworld-save-migration.md](docs/palworld-save-migration.md) | セーブ移行（日英） |
| [docs/security-adoption.md](docs/security-adoption.md) | 採用済みセキュリティ要約 |
| [docs/code-signing.md](docs/code-signing.md) | SignPath / 署名（将来） |
| [.github/SECURITY.md](.github/SECURITY.md) | 脆弱性報告ポリシー |

---

## ライセンス

MIT
