# Security

## Reporting a vulnerability

Please open a private security advisory on GitHub instead of a public issue. Include steps to reproduce and the AltFlip version.

## How AltFlip handles credentials

- AltFlip never asks for or stores passwords. Accounts are added through each tool's own sign-in.
- Saved sign-ins are encrypted with AES-256-GCM. The key is generated on first launch and kept in the system credential store (macOS Keychain, Windows Credential Manager, Secret Service), not next to the encrypted files.
- Each encrypted file is bound to its name, so files cannot be swapped between accounts.
- Credentials are never written to SQLite, logs, the activity history, notifications or the UI.
- On macOS, Keychain items owned by other tools are written through `/usr/bin/security` with the secret passed on standard input, keeping those tools' access settings intact.
- Files are replaced atomically and read back; symlinked targets are refused.
- Backups are encrypted with a passphrase (PBKDF2-HMAC-SHA256, 600,000 rounds, AES-256-GCM). Anyone with the file and passphrase can use the accounts in it.
- If the credential store is unavailable, switching is turned off.

## Network

AltFlip makes no network requests of its own unless you turn on *Claude usage from Anthropic*, which sends saved Claude logins only to `api.anthropic.com` to read usage. `claude auth login`, `claude auth status` and `codex login` contact their providers when you use them.
