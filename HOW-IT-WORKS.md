# How AltFlip works

AltFlip is one Tauri 2 app. The Rust side owns every credential operation; the React UI only sees account names, emails and usage. There is no AltFlip account, server or telemetry.

## Architecture

| Crate | What it does |
|---|---|
| `crates/core` | Errors, shared models, atomic file writes (temp file, fsync, rename, read-back, refuses symlinks), the OS credential store, the encrypted vault, process discovery and graceful quit |
| `crates/database` | SQLite schema and queries: providers, accounts, active accounts, usage, switch history, recovery snapshots, activity, settings |
| `crates/providers` | The `Provider` trait and the Claude Code, Codex and Antigravity adapters |
| `crates/engine` | Account management, the switch engine, live-state checks, usage, backups and settings |
| `apps/desktop` | Tauri shell (tray, windows, commands, background refresh) and the React UI |

TypeScript types for the UI are generated from Rust with `ts-rs` into `apps/desktop/src/lib/bindings.ts` by the `export_bindings` test.

## Saved sign-ins

Each adapter lists the places that hold its sign-in: a whole file, one key inside a JSON file (the rest of the file is kept), or an OS credential store item. A saved account is the bytes at each place plus the identity they belong to, sealed with AES-256-GCM in `vault/accounts/<id>.bin`. The key lives in the system credential store (macOS Keychain, Windows Credential Manager, Secret Service), never next to the files, and each file is bound to its name so files can't be swapped between accounts.

Tokens never enter SQLite, logs, the activity history, notifications or the UI. AltFlip never asks for passwords and never saves a sign-in it can't read.

## Switching

| Step | What happens |
|---|---|
| Gate | The provider must be installed, supported, able to switch, and the credential store must be available |
| Check | The saved sign-in is loaded and validated. If the live sign-in already matches, AltFlip records it and stops |
| Confirm | If apps must restart, AltFlip asks first |
| Keep tokens fresh | The outgoing account is re-saved from the live data, so refreshed tokens aren't lost |
| Snapshot | A raw recovery snapshot is sealed and a `started` journal row is written |
| Apply | Apps that must restart are asked to quit (never forced). The target is written and verified through the adapter |
| Finish | On success the selection is committed and closed apps reopen. On failure the snapshot is restored byte for byte (`rolled-back`), or the switch is marked `needs-recovery` |

Each provider has its own lock, so different tools can switch at the same time. On launch, `started` rows are closed as abandoned, and `applying` rows are shown with **Restore previous** or **Keep current**.

## Live checks

Running sessions can rewrite their own tokens. When AltFlip reads the live sign-in:

- it matches the selected account: tokens are re-saved;
- it matches another saved account: the selection follows it and the change is logged;
- it matches nothing saved: the tool is marked *changed outside AltFlip*;
- nothing is signed in: the tool is marked *signed out*.

Live state is checked every 5 minutes and when a window gains focus (at most every 15 seconds).

## Providers

### Claude Code

| | |
|---|---|
| Sign-in | `~/.claude.json` `oauthAccount` (only this key changes), plus the `Claude Code-credentials` Keychain item on macOS or `~/.claude/.credentials.json` elsewhere |
| Identity | `accountUuid` + `organizationUuid`, so a personal account and a team seat are separate |
| Verify | Stored data is read back, then `claude auth status --json` must report the expected email |
| Add account | `claude auth login` with a temporary `CLAUDE_CONFIG_DIR`, removed afterwards |
| Switch mode | New sessions use the selected account; running sessions keep theirs |
| Usage | Live 5-hour and weekly limits from `api.anthropic.com` (`/api/oauth/usage`). Expired logins are renewed with Claude Code's own refresh flow. Local fallback: Claude Desktop's `plan-usage-history.json` and the optional status-line script (Settings › Usage) |

### Codex CLI

| | |
|---|---|
| Sign-in | `~/.codex/auth.json` (or `$CODEX_HOME/auth.json`). ChatGPT and API-key sign-ins |
| Identity | ChatGPT user and workspace from the ID token, or a hash of the API key |
| Add account | `codex login` with a temporary `CODEX_HOME` |
| Switch mode | New sessions use the selected account |
| Usage | Live limits from `chatgpt.com/backend-api/wham/usage`. Local fallback: the newest `rate_limits` event in recent session logs, matched to each account |
| Not supported | `cli_auth_credentials_store = "keyring"`; switching is turned off with an explanation |

### Antigravity (usage only)

| | |
|---|---|
| Why no switching | Antigravity only reads its sign-in when it starts, so every switch would have to close it, and open chats would keep the old account. The app shows this in plain words from the **Usage only** button |
| Add account | AltFlip saves the current account, opens Antigravity, and saves the new one after you sign in there |
| Usage | Live per-model quota and plan from Google's Cloud Code API, using Antigravity's own OAuth client read from the installed app at run time. Renewed tokens stay in memory. The running app's account is read from its local language server instead. Checked every 2 minutes |

## Usage refresh

Usage refreshes every minute while a window is open, and on the interval set in Settings otherwise. Each account's refresh button on the Usage page refreshes only that provider; the toolbar button (⌘R) refreshes everything. Pages keep their data when you move between them and only reload when something changed.

## Network

With **Live usage** on (the default), AltFlip asks each provider for each saved account's limits at most once a minute: Claude logins go only to `api.anthropic.com` (and `console.anthropic.com` to renew), Codex logins only to `chatgpt.com`, Antigravity logins only to Google's Cloud Code endpoints. Turn it off in Settings › Usage to make no network requests. `claude auth login`, `claude auth status` and `codex login` contact their providers when you use them.

## Backups

Backups are encrypted with a passphrase (PBKDF2-HMAC-SHA256, 600,000 rounds, AES-256-GCM). Anyone with the file and the passphrase can use the accounts in it.

## Releasing

1. Add a `## X.Y.Z` section to `CHANGELOG.md`.
2. Push a `vX.Y.Z` tag, or run **Actions › Release**.

`.github/workflows/release.yml` sets the version, builds installers for macOS (universal), Linux and Windows, copies the changelog section into the release notes, and publishes the GitHub Release with SHA-256 checksums.

## Reporting a vulnerability

Open a private security advisory on GitHub instead of a public issue, with steps to reproduce and the AltFlip version.
