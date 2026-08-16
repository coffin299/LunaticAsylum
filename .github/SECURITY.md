# Security Policy

LunaticAsylum is an **under-development** desktop game-server manager (Windows-first).  
This document is the public security policy shown on GitHub.

Internal implementation rules for contributors/agents live in [`.cursor/rules/security.mdc`](../.cursor/rules/security.mdc).  
What we have actually shipped so far is summarized in [`docs/security-adoption.md`](../docs/security-adoption.md).

## Supported versions

| Version | Supported |
| --- | --- |
| `main` (development builds) | Best-effort while the project is under active development |
| Pre-release / unsigned ZIP | Use at your own risk — not a signed production channel yet |

We do not yet maintain long-term support (LTS) channels.

## Reporting a vulnerability

Please **do not** open a public GitHub Issue for security-sensitive reports.

Prefer one of:

1. [GitHub Private Vulnerability Reporting](https://github.com/coffin299/LunaticAsylum/security/advisories/new) (if enabled for this repository)
2. Contact the repository owner via GitHub (@coffin299)

Include, where possible:

- affected version / commit
- OS and build type (dev vs portable ZIP)
- steps to reproduce
- impact (secret exposure, path traversal, remote code execution, etc.)
- a minimal PoC if you can share one safely

We will acknowledge reports as soon as practical and coordinate disclosure.

## Scope (current product)

In scope for reports against current builds:

- secret handling (Discord bot token, Palworld REST admin password, and similar)
- unsafe path / archive handling (including Zip Slip on restore)
- Tauri command surfaces that could lead to unintended privileged actions
- XSS or injection via untrusted game/Discord/server output rendered in the UI

Out of scope for now (not implemented or explicitly deferred):

- multi-user / remote management APIs
- Integration API keys and password-hashing schemes for local accounts
- supply-chain review of every transitive dependency (reports still welcome, triage may be slower)
- issues that require ignoring OS trust boundaries on a machine you already fully control, unless they escalate beyond that model

## Security posture (high level)

- Privileged work stays in the **Rust / Tauri** backend; the WebView is untrusted input.
- Recoverable secrets are stored in the **OS credential store**, not in `config.json` or frontend storage.
- Management networking is intended to stay **local/loopback** unless remote management is explicitly designed later.
- Release signing is **not** enabled yet (unsigned portable ZIP).

## Preference

When in doubt we fail closed: deny the operation, preserve state, and return a safe error without leaking secrets.
