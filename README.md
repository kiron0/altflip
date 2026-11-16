# AltFlip

Switch between multiple Claude Code, Codex and Google Antigravity accounts from your menu bar or system tray.

Add each account once through the tool's own sign-in. AltFlip keeps an encrypted copy of each sign-in on your computer and swaps the active one when you pick another account. Every switch is verified, and the previous account is restored if anything goes wrong.

- **One app, three tools.** Claude Code, OpenAI Codex CLI, and Google Antigravity (App, `agy` CLI, IDE 2.6+).
- **One active account per tool.** Each tool keeps its own selection.
- **Safe switching.** Recovery snapshot, provider lock, verification, automatic rollback, crash recovery.
- **Honest status.** Sessions that are already running keep their account; AltFlip says so instead of claiming an instant switch.
- **Local only.** No AltFlip account, no server, no telemetry.

## Supported tools

| Tool | How switching works | Status |
| --- | --- | --- |
| Claude Code | Swaps the default sign-in (`~/.claude.json` `oauthAccount` and the `Claude Code-credentials` Keychain item on macOS, or `~/.claude/.credentials.json` elsewhere), then checks `claude auth status`. New sessions use the selected account. | Supported |
| Codex CLI | Swaps `~/.codex/auth.json` (or `$CODEX_HOME/auth.json`). New sessions use the selected account. Keyring-backed Codex credentials are not supported yet. | Supported |
| Antigravity | Swaps the shared Google sign-in used by the App, the `agy` CLI and IDE 2.6+. Running Antigravity apps are closed and reopened after you confirm. | Experimental, off by default |

Details and limitations: [docs/PROVIDERS.md](docs/PROVIDERS.md).

## Adding accounts

- **Sign in with another account**: for Claude Code and Codex, AltFlip runs `claude auth login` or `codex login` in a private, temporary profile, so your current sign-in is not touched. For Antigravity, which has no sign-in command, AltFlip saves the current account, opens Antigravity, and saves the new account once you sign in to it there.
- **Save the account that's signed in now** (all tools): sign in with the tool as usual, then save it in AltFlip.

AltFlip never asks for passwords.

## Development

Requirements: Rust (stable), Node 22+, pnpm 10, and the [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/) for your OS.

```sh
pnpm install
pnpm dev          # run the desktop app
pnpm test         # Rust and frontend tests
pnpm build        # create installers
```

`pnpm --filter @altflip/desktop dev` runs the UI alone in a browser with mock data.

In debug builds, set `ALTFLIP_DATA_DIR` to keep AltFlip's database and encrypted vault in a separate folder.

## Repository layout

```text
apps/desktop/          Tauri 2 app (React + TypeScript + Tailwind)
  src/                 Quick switcher, dashboard, onboarding, settings
  src-tauri/           Tray, windows, commands, background refresh
crates/core/           Errors, models, atomic files, credential store, vault, processes
crates/database/       SQLite schema and queries
crates/providers/      Claude Code, Codex and Antigravity adapters
crates/engine/         Account manager, switch engine, recovery, backups
docs/                  Architecture and provider notes
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

MIT. See [LICENSE](LICENSE). The app and menu bar icons are based on the [AccountDeck](https://github.com/nadim-sheikh/AccountDeck) mascot (MIT); see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
