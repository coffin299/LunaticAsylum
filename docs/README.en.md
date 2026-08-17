# LunaticAsylum

> **Currently in development.** Only a minimum set of features is implemented. Performance is not optimized.

<p align="center">
  <a href="https://count.getloli.com/@LunaticAsylumServerManager">
    <img src="https://count.getloli.com/@LunaticAsylumServerManager?name=LunaticAsylumServerManager&theme=green&padding=7&offset=0&align=top&scale=1&pixelated=1&darkmode=0" alt="LunaticAsylumServerManager moecounter">
  </a>
</p>

<p align="center">
  <strong>Desktop game server manager for Windows</strong><br>
  <sub>Start, stop, backup, configure, and Discord — all in one place</sub>
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

[日本語](../README.md) · [Docs](./INDEX.md)

---

Launch `LunaticAsylum.exe` and drop server folders into the adjacent `Servers/` directory — they are **auto-detected**.  
Manage start/stop, backups, configuration, and Discord integration from a single GUI.

> **Status**  
> Currently in development. Only a minimum set of features is implemented, and performance is not optimized. Not intended for production use.  
> Release ZIPs are **unsigned portable** builds — Windows SmartScreen may warn. Code signing (SignPath) is pending.

---

## Features

### Core

- SteamCMD install, start / stop / restart (buttons disabled by server state; stop sends REST `/stop` then terminates the process tree)
- Scheduled backups (REST save before backup), **two-step restore**, crash auto-restart
- Update detection: AppInfo API polling / manual / optional auto-apply
- Discord integration (tokens stored in **OS Credential Store**)
- UI: single dark theme, Overview (host resources & banners)
- Unsupported / empty folders appear in the list but hide action controls
- **Exit confirmation when servers are running** (must stop all servers before quitting)

### Palworld

| | |
| :--- | :--- |
| Settings GUI | `PalWorldSettings.ini` — **ini ↔ REST two-way sync** (manual sync button) |
| Save parsing | Port Map compliant through Character / Group / BaseCamp RawData (PlM fetches Oodle on first use) |
| Map | Base transforms (Level.sav) / player LastTransform (Players/*.sav) |
| Other | Steam banner, lazy-loaded save parsing |

### Minecraft

| | |
| :--- | :--- |
| Launch | `java -jar` (user picks type: Vanilla / Paper / etc.) |
| Settings GUI | `server.properties` editor |
| RCON | **Java Source RCON** — properties ↔ RCON two-way sync (manual sync button) |

### Other

- SteamCMD: fixed at `{appRoot}/tools/steamcmd`; fetch from banner when missing
- Product site scaffold: `apps/webpage` (Pages on `webpage` branch)
- Distribution: unsigned portable ZIP (no Cargo intermediate files in the archive)

---

## Quick start

```text
LunaticAsylum/
├── LunaticAsylum.exe
├── Servers/
│   ├── my-palworld/       ← put server folders here
│   └── my-minecraft/
└── tools/steamcmd/
```

1. Download the ZIP from [Releases](https://github.com/coffin299/LunaticAsylum/releases)
2. Extract and run `LunaticAsylum.exe`
3. Place server folders under `Servers/` → auto-detected

---

## Development

Requirements: **Node 20+**, **pnpm 9**, **Rust** (Tauri 2)

```bat
corepack enable
corepack prepare pnpm@9.15.0 --activate
pnpm install
pnpm dev
```

| Command | Description |
| :--- | :--- |
| `pnpm dev` | Tauri dev server |
| `pnpm build` | Production build |

---

## Documentation

| Document | Description |
| :--- | :--- |
| [INDEX.md](./INDEX.md) | Documentation index |
| [architecture.md](./architecture.md) | Layers, Discovery, Backup, Update check |
| [provider-guide.md](./provider-guide.md) | Adding a Provider |
| [discord.md](./discord.md) | Discord integration (Palworld-specific) |
| [palworld-save-migration.md](./palworld-save-migration.md) | Save migration (JA / EN) |
| [security-adoption.md](./security-adoption.md) | Adopted security measures |
| [code-signing.md](./code-signing.md) | SignPath / signing (future) |
| [../.github/SECURITY.md](../.github/SECURITY.md) | Vulnerability reporting policy |

---

## License

MIT
