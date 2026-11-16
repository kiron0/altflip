# Architecture

AltFlip is one Tauri 2 process. The Rust side owns every credential operation; the React UI only sees account metadata.

```text
React (quick switcher, dashboard)  ──Tauri IPC──▶  src-tauri commands
                                                    │
                                                    ▼
                                           altflip-engine  (Engine)
                                ┌───────────────┼────────────────┐
                                ▼               ▼                ▼
                      altflip-database   altflip-providers   altflip-core
                         (SQLite)        (adapters)          (vault, secrets, files, processes)
```

## Crates

- **altflip-core**: `Error`, shared models, `fsutil::atomic_write` (temp file, fsync, rename, read-back check, refuses symlinks), `SecretStore` (macOS `security`, Windows Credential Manager, Secret Service), `Vault` (AES-256-GCM, key in the credential store), `resource` (locators and snapshots), process discovery and graceful quit.
- **altflip-database**: migrations and queries for `providers`, `accounts`, `active_accounts`, `usage_snapshots`, `switch_history`, `recovery_snapshots`, `activity`, `app_settings`. Foreign keys are on; `active_accounts.provider_id` is the primary key, so only one selected account per provider can exist.
- **altflip-providers**: the `Provider` trait and the three adapters.
- **altflip-engine**: account management, the switch engine, live-state reconciliation, usage attribution, backups and settings.

## Snapshots

Each adapter lists the places that hold its sign-in as `Locator`s:

- `File { path }`
- `JsonKey { path, key }`: one key inside a JSON document; the rest of the file is preserved
- `Credential { service, account }`: an OS credential store item

A `Snapshot` is the bytes at each locator plus the identity they belong to. Saved accounts are snapshots sealed in `vault/accounts/<id>.bin`. Secrets never enter SQLite, logs or the frontend.

## Switching

`Engine::switch` runs as a transaction:

1. Acquire the provider lock. Different providers can switch at the same time.
2. Check the switching gate: installed, compatible, credential store available, experimental opt-in.
3. Load and validate the target snapshot.
4. Capture the live state. If it already matches the target, record it and stop.
5. If apps must restart and the user has not confirmed, stop with `confirmation_required`.
6. Re-seal the outgoing account with the live data, so rotated refresh tokens are kept.
7. Seal a raw recovery snapshot and write a `started` journal row.
8. Ask apps that must restart to quit, and wait. If they do not close, cancel with nothing changed.
9. Mark the journal `applying`, write the target, verify through the adapter.
10. On success, commit the active selection. On failure, restore the recovery snapshot byte for byte and report `rolled-back`, or `needs-recovery` if the restore itself failed.
11. Reopen closed apps.

On launch, `started` rows are closed as abandoned (nothing changed) and `applying` rows are shown to the user with *Restore previous* or *Keep current*.

## Live reconciliation

Running sessions can rewrite credentials when they refresh tokens. On refresh, AltFlip reads the live sign-in for each tool:

- matches the selected account: tokens are re-sealed;
- matches another saved account: the selection follows it and the change is logged;
- matches nothing saved: the provider is marked *changed outside AltFlip* and the user can save it;
- nothing signed in: marked *signed out*.

## Background work

One thread refreshes live state every 5 minutes (and on window focus, throttled) and usage on the configured interval. No server or daemon is started.
