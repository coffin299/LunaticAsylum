# Palworld save migration

日英併記。

## 日本語

SteamCMD 版 Dedicated Server では、ワールド／セーブは主に次にあります。

- `Pal/Saved/`

### 移行手順（概要）

1. 移行元・移行先のサーバーを **停止** する
2. 移行先の `Pal/Saved/` を退避（任意だが推奨）
3. 移行元の `Pal/Saved/` をコピーする
4. LunaticAsylum の `Servers/{任意名}/` 配下に配置されていることを確認する
5. サーバーを起動し、ワールドを確認する

LunaticAsylum のバックアップ機能も同じ `Pal/Saved/` を対象にします（公式バックアップと併用する場合はアプリ側バックアップを disable できます）。

## English

On SteamCMD dedicated servers, world/save data typically lives under:

- `Pal/Saved/`

### Migration outline

1. **Stop** both source and destination servers
2. Optionally back up the destination `Pal/Saved/`
3. Copy source `Pal/Saved/` into place
4. Confirm the folder is under `Servers/{name}/` managed by LunaticAsylum
5. Start the server and verify the world

LunaticAsylum backups also target `Pal/Saved/` (you can disable app backups if you rely on the official server backup feature).
