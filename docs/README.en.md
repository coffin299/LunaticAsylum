# LunaticAsylum

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
  <a href="../README.md"><img src="https://img.shields.io/badge/日本語-README-6366F1?style=flat-square" alt="Japanese README"></a>
  <a href="./INDEX.md"><img src="https://img.shields.io/badge/Docs-Index-6366F1?style=flat-square" alt="Docs Index"></a>
</p>

<p align="center">
  <a href="https://github.com/coffin299/LunaticAsylum"><img src="https://img.shields.io/github/v/release/coffin299/LunaticAsylum?label=Release&sort=semver&logo=github" alt="Release"></a>
  <a href="https://github.com/coffin299/LunaticAsylum/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/coffin299/LunaticAsylum/release.yml?branch=main&label=Release%20CI&logo=githubactions" alt="Release CI"></a>
  <a href="https://github.com/coffin299/LunaticAsylum/blob/main/LICENSE"><img src="https://img.shields.io/github/license/coffin299/LunaticAsylum?logo=opensourceinitiative&logoColor=white" alt="License MIT"></a>
  <a href="https://github.com/coffin299/LunaticAsylum/security"><img src="https://img.shields.io/badge/Security-Policy-blue?logo=github&logoColor=white" alt="Security Policy"></a>
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
  <img src="https://img.shields.io/badge/Palworld-REST%20%2B%20Save%20Parse-5865F2?style=flat-square&logo=steam&logoColor=white" alt="Palworld">
  <img src="https://img.shields.io/badge/Minecraft-Java%20RCON-62B47A?style=flat-square&logo=minecraft&logoColor=white" alt="Minecraft Java">
  <img src="https://img.shields.io/badge/Discord-Integration-5865F2?style=flat-square&logo=discord&logoColor=white" alt="Discord">
  <img src="https://img.shields.io/badge/SteamCMD-Auto%20Update-1B2838?style=flat-square&logo=steam&logoColor=white" alt="SteamCMD">
  <img src="https://img.shields.io/badge/Backup-Scheduled%20%2B%20Restore-22C55E?style=flat-square" alt="Backup">
  <img src="https://img.shields.io/badge/Portable-ZIP%20Distribution-F59E0B?style=flat-square" alt="Portable ZIP">
</p>

<p align="center">
  <img src="https://img.shields.io/badge/Secrets-OS%20Credential%20Store-22C55E?style=flat-square&logo=windows&logoColor=white" alt="OS Credential Store">
  <img src="https://img.shields.io/badge/ZIP%20Restore-Zip%20Slip%20Protected-22C55E?style=flat-square" alt="Zip Slip Protected">
  <img src="https://img.shields.io/badge/Code%20Signing-Pending%20(SignPath)-F97316?style=flat-square" alt="Code Signing Pending">
</p>

---

Launch `LunaticAsylum.exe` and drop server folders into the adjacent `Servers/` directory — they are **auto-detected**.  
Manage start/stop, backups, configuration, and Discord integration from a single GUI.

> **Production status**  
> The app is ready for production use with the current feature set.  
> Release ZIPs are **unsigned portable** builds — Windows SmartScreen may warn. Code signing (SignPath) is pending.

---

## Features

<p align="center">
  <img src="https://img.shields.io/badge/Discovery-Auto%20Detect-22C55E?style=for-the-badge" alt="Auto Detect">
  <img src="https://img.shields.io/badge/Lifecycle-Start%20%2F%20Stop%20%2F%20Restart-22C55E?style=for-the-badge" alt="Lifecycle">
  <img src="https://img.shields.io/badge/Crash%20Recovery-Auto%20Restart-22C55E?style=for-the-badge" alt="Crash Recovery">
  <img src="https://img.shields.io/badge/Update%20Check-AppInfo%20API-22C55E?style=for-the-badge" alt="Update Check">
</p>

### Core

- SteamCMD install, start / stop / restart (stop sends REST `/stop` then terminates the process tree)
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

<p align="center">
  <img src="https://img.shields.io/badge/Prerequisites-Node%2020%2B%20%7C%20pnpm%209%20%7C%20Rust-333?style=flat-square" alt="Prerequisites">
</p>

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

<p align="center">
  <a href="https://github.com/coffin299/LunaticAsylum/blob/main/LICENSE"><img src="https://img.shields.io/badge/License-MIT-6366F1?style=for-the-badge" alt="MIT License"></a>
</p>

MIT
