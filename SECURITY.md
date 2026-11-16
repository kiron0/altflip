# Security

## Reporting a vulnerability

Please open a private security advisory on GitHub instead of a public issue. Include steps to reproduce and the AltFlip version.

## How AltFlip handles credentials

- AltFlip never asks for or stores passwords. Accounts are added through each tool's own sign-in.
- Saved sign-ins are encrypted with AES-256-GCM. The key is generated on first launch and kept in the system credential store (macOS Keychain, Windows Credential Manager, Secret Service), not next to the encrypted files.
- Each encrypted file is bound to its name, so files cannot be swapped between accounts.
- Credentials are never written to SQLite, logs, the activity history, notifications or the UI.
- On macOS, Keychain items owned by other tools are written through `/usr/bin/security`, keeping those tools' access settings intact. Values up to about 1.8 KB go through standard input; longer ones (which `security -i` would truncate) are passed as a hex argument for the moment the command runs. Every write is read back, and if it doesn't match, the previous value is put back.
- AltFlip never saves a sign-in it can't read: an unreadable or incomplete login is not copied over a saved account.
- Files are replaced atomically and read back; symlinked targets are refused.
- Backups are encrypted with a passphrase (PBKDF2-HMAC-SHA256, 600,000 rounds, AES-256-GCM). Anyone with the file and passphrase can use the accounts in it.
- If the credential store is unavailable, switching is turned off.

## Network

With *Live usage* on (the default), AltFlip asks each provider for the limits of each saved account at most once a minute: Claude logins go only to `api.anthropic.com` (and `console.anthropic.com` to renew an expired login), Codex logins only to `chatgpt.com`. Turn it off in Settings › Usage monitoring to make no network requests. While Antigravity or `agy` is running, AltFlip asks its local language server on `127.0.0.1` for quota; that request never leaves the machine and uses the server's own session token, so its self-signed certificate is not checked. `claude auth login`, `claude auth status` and `codex login` contact their providers when you use them.
