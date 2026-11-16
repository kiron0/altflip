# Provider compatibility

## Claude Code

- Detected through the `claude` command and `~/.claude.json`.
- Sign-in state:
  - `~/.claude.json` → `oauthAccount` (only this key is changed; projects and settings are kept)
  - macOS: Keychain item `Claude Code-credentials` for the current user
  - Windows and Linux: `~/.claude/.credentials.json`
- Identity: `accountUuid` + `organizationUuid`, so a personal account and a team seat for the same person are separate accounts.
- Verification: stored data is read back, then `claude auth status --json` must report the expected email.
- Add accounts: `claude auth login` with a temporary `CLAUDE_CONFIG_DIR`; the temporary Keychain item and folder are removed afterwards.
- Usage (local, always on): Claude Desktop's `plan-usage-history.json` (5-hour and weekly percentages per organization; reset times estimated the way AccountDeck does), matched to saved accounts by organization, and Claude Code status-line files in `~/.claude/.altflip-status/` (written by `~/.claude/altflip-status.sh`, installed from Settings › Usage › Instant usage from Claude Code; any previous status line command keeps running) or the legacy `~/.claude/.accountdeck-status/`, attributed to the account that was active when they were written.
- Live usage (on by default): AltFlip sends each saved Claude login to `api.anthropic.com` (`/api/oauth/profile` to confirm the account, `/api/oauth/usage` for the 5-hour and weekly limits), the same method AccountDeck uses. Expired logins are renewed with Claude Code's own refresh flow; for the active account the renewed login is written back to the `Claude Code-credentials` item (only if Claude Code hasn't changed it meanwhile), for other accounts only AltFlip's encrypted copy is updated.
- Mode: new sessions. Running sessions keep their account in memory and may write their own tokens back when they refresh; AltFlip then shows the sign-in as changed.
- AltFlip manages the default profile only. If `CLAUDE_CONFIG_DIR` is set for AltFlip itself, Claude Code is marked untested.

## Codex CLI

- Detected through the `codex` command and `~/.codex` (or `$CODEX_HOME`).
- Sign-in state: `auth.json`. ChatGPT and API-key sign-ins are supported.
- Identity: ChatGPT user and workspace (`chatgpt_account_id`) from the ID token, or a hash of the API key.
- Add accounts: `codex login` with a temporary `CODEX_HOME` set to file storage.
- Live usage (on by default): `GET https://chatgpt.com/backend-api/wham/usage` with the account's ChatGPT login and `ChatGPT-Account-Id`, the same source as Codex's usage panel. Saved accounts whose login has expired show the error until they are used once.
- Local usage (fallback): the newest `rate_limits` event in each recent `sessions/**/rollout-*.jsonl` (last 600 KB read). Each session's `session_meta` names its `creator_user_id` and `creator_account_id`, so every saved Codex account gets its own reading. Windows whose reset time has passed show 0%.
- Not supported: `cli_auth_credentials_store = "keyring"` (and `auto` without an `auth.json`). Switching is turned off with an explanation.

## Antigravity (experimental)

Off until enabled in Settings › Switching.

- Detected through Antigravity.app / Antigravity IDE.app (macOS), `%LOCALAPPDATA%\Programs` (Windows), `/usr/share/antigravity` or `/opt/Antigravity` (Linux), the `agy` command, and `~/.gemini/antigravity`.
- Sign-in state:
  - Antigravity IDE / App `User/globalStorage/state.vscdb` keys `antigravityUnifiedStateSync.oauthToken`, `antigravityUnifiedStateSync.userStatus`, `antigravityUnifiedStateSync.modelCredits` and the encrypted `antigravity.authSync` secret (IDE 2.7 keeps its Google sign-in here)
  - credential store item `gemini` / `antigravity` (used by `agy` and older App builds; kept only when it is for the same account)
  - `~/.gemini/antigravity-cli/antigravity-oauth-token` if present
  - `~/.gemini/jetski-standalone-oauth-token` if present and for the same account
- Identity: the Google email in the IDE's `userStatus`, or in the credential store token when no IDE sign-in exists. The plan (for example "Google AI Pro") comes from `userStatus`.
- Mode: restart required while the App or IDE is open. AltFlip asks first, quits them gracefully (never forced), switches, verifies, and reopens them. If an app will not quit, nothing is changed.
- Add accounts: Antigravity has no sign-in command, so AltFlip saves the current account, opens Antigravity, and waits for the sign-in to change after you sign in to the other Google account there. The new account is saved and becomes active.
- Usage: per-model quota (remaining share and reset time) from the IDE's locally saved `userStatus`. It is attributed by email, and shows as reset once its reset time passes.
- Live quota for every saved account (5-hour and weekly pools per model group, with reset times) and the plan tier come from Google's Cloud Code API (`v1internal:retrieveUserQuotaSummary`, `v1internal:loadCodeAssist` on `daily-cloudcode-pa.googleapis.com`, then `cloudcode-pa.googleapis.com`), the same calls Antigravity makes. AltFlip renews each saved sign-in in memory with Antigravity's own OAuth client, read from the installed Antigravity app or `agy` at run time, and never writes the renewed token back. Polled every 2 minutes; the account signed in to a running Antigravity is read from its language server instead. A missing remaining share is shown as unknown, never as 0% or 100%.
- Not supported: independent IDE accounts on IDE versions before 2.6.
- Open agent conversations do not move to the new account.

This integration was informed by reading [agy-switch](https://github.com/anglee0323/agy-switch). No code was copied; that project uses a non-commercial license.
