# AltFlip
## Complete Product Requirements, Features, Architecture & Implementation Plan

**Application Type:** Cross-Platform Desktop Utility  
**Framework:** Tauri 2  
**Backend:** Rust  
**Frontend:** React, TypeScript, Tailwind CSS, shadcn/ui  
**Database:** SQLite  
**Platforms:** macOS, Windows, Linux  
**Distribution:** Free and Open Source  
**License:** MIT (proposed)  
**Architecture:** Local-First, Offline-Capable, Single Application

---

# 1. Product Overview

AltFlip is a lightweight, open-source desktop application designed to simplify the management and switching of multiple accounts across AI coding assistants and development environments.

The application operates primarily from the macOS menu bar or the Windows/Linux system tray, allowing users to access, manage, monitor, and switch between their saved AI accounts without repeatedly signing in and out.

It acts as a centralized account management layer for supported AI development tools, including:

- Claude Code
- OpenAI Codex CLI
- Google Antigravity App and IDE
- Google Antigravity CLI
- Gemini services supported through Antigravity
- Additional AI providers through future integrations

Users can register multiple accounts for each provider, authenticate them once using the provider's supported authentication mechanisms, and select which account should be active.

The application handles supported account switching operations in the background, manages account-specific configuration and authentication state, monitors usage limits where reliable data is available, and coordinates application restarts or reloads only when necessary.

The primary goal is to eliminate repetitive authentication and manual account management while providing one consistent experience across different AI development tools.

## 1.1 Core Concept

**One application. Multiple AI providers. Multiple saved accounts. One active account per provider.**

The user should be able to:

1. Install the application.
2. Add multiple accounts for each supported AI provider.
3. Complete authentication for each account once.
4. Keep those accounts saved securely.
5. Open the menu bar or system tray interface.
6. Select another account.
7. Allow the application to activate that account.
8. Continue using the relevant AI tool with minimal interruption.

No additional companion application, custom command-line wrapper, or IDE extension should be required as part of the standard user experience.

However, the product must acknowledge that some provider applications expose no supported mechanism for changing an active account while running. These integrations may require a controlled application restart, reauthentication, or user intervention.

The product must never falsely report that switching succeeded.

---

# 2. Product Vision

Create a universal desktop account management utility that makes working with multiple AI coding assistants as simple as switching between profiles in a browser.

The application should feel native, lightweight, secure, and unobtrusive.

Users should never need to remember authentication file locations, manually modify configuration directories, copy tokens, edit environment variables, or run terminal commands just to switch between previously authenticated accounts.

The account manager must abstract provider-specific complexity behind a consistent interface.

### Primary Principles

- **Universal:** Multiple AI providers within one application.
- **Simple:** Minimal steps to switch accounts.
- **Fast:** Minimize switching delays and unnecessary restarts.
- **Native:** Integrate with the operating system's menu bar or system tray.
- **Secure:** Protect stored account credentials and authentication state.
- **Local-first:** Avoid mandatory remote servers.
- **Reliable:** Preserve existing sessions and configuration.
- **Non-destructive:** Never unexpectedly delete or overwrite user data.
- **Extensible:** New providers can be integrated through isolated Rust adapters.
- **Transparent:** Show the true result and limitations of every switch.
- **Open source:** Make the codebase auditable and community-maintainable.

---

# 3. Supported Platforms

## 3.1 macOS

The macOS application should work primarily as a native-feeling menu bar utility.

### Requirements

- macOS 14 or later as the initial target.
- Support Apple Silicon and Intel architectures.
- Persistent menu bar icon.
- Compact popover-style account selector.
- Optional full management window.
- Launch automatically at login.
- Run without a Dock icon in menu bar-only mode.
- Support native macOS appearance.
- Use macOS Keychain for appropriate credential storage.
- Detect supported AI applications.
- Monitor relevant application processes.
- Support controlled switching and restart operations.
- Show notifications when required.

The application should remain accessible even when its main window is closed.

## 3.2 Windows

### Requirements

- Windows 10 and Windows 11.
- Support x64, with ARM64 where dependencies permit.
- System tray icon.
- Compact quick-switch interface.
- Full settings window.
- Launch at system startup.
- Minimize to tray.
- Store protected secrets through Windows credential-protection facilities.
- Detect supported AI tools.
- Manage account profiles and application processes.
- Provide native notifications.

Closing the application window should hide it instead of terminating the background process, unless the user explicitly selects Quit.

## 3.3 Linux

### Requirements

- Support mainstream Linux desktop distributions.
- Prioritize Ubuntu, Debian, Fedora, and compatible distributions.
- Support x64 and ARM64 where feasible.
- Use system tray or StatusNotifier/AppIndicator integration where available.
- Support desktop environments such as GNOME, KDE Plasma, and Cinnamon.
- Provide a regular application window as a fallback where tray functionality is unavailable.
- Support automatic startup through desktop-compatible mechanisms.
- Use Secret Service-compatible secure storage where available.

Linux desktop environments differ in tray behavior. The application must provide a reliable fallback instead of assuming identical behavior on every distribution.

---

# 4. Supported AI Providers

The application should use a modular provider architecture.

Each provider operates through an independent integration module responsible for authentication discovery, saved profiles, switching, monitoring, and recovery.

## 4.1 Claude Code

Claude Code must be a first-class integration.

### Features

- Detect Claude Code installation.
- Detect installed version.
- Discover existing account configuration.
- Add multiple Claude accounts.
- Support personal, Pro, Max, Team, and authorized organizational accounts.
- Authenticate accounts through Claude Code's supported sign-in process.
- Keep account-specific configuration isolated.
- Select one default active account.
- Switch between previously authenticated accounts.
- Detect invalid or expired authentication.
- Preserve session histories.
- Monitor usage when supported.
- Display account-specific status.
- Manage supported MCP servers, skills, hooks, and settings.
- Restore the previous account after a failed switch.

### Technical Strategy

Claude Code supports the `CLAUDE_CONFIG_DIR` environment variable, allowing multiple configuration directories.

Example managed profile structure:

```text
profiles/
└── claude/
    ├── personal/
    ├── secondary/
    └── work/
```

Each profile should maintain its own supported configuration and authentication state.

The application must implement two layers of compatibility:

**Managed profile isolation**

Maintain separate account directories and preserve their settings, credentials, and session information according to Claude Code's documented behavior.

**Default account activation**

Investigate and implement a safe method for the ordinary `claude` command to use the selected account without requiring a companion wrapper.

This may require carefully coordinating Claude Code's default configuration and OS credential-store behavior.

Because a process's environment is established when it launches, changing the selected profile cannot automatically change the environment of an already-running Claude Code process.

The default activation mechanism must be verified against supported Claude Code versions before being marked reliable.

The application should never assume that copying a configuration directory automatically transfers every authentication credential, especially when keychain storage is involved.

---

## 4.2 OpenAI Codex

Codex should support multiple saved accounts using managed profile isolation.

### Features

- Detect Codex CLI installation.
- Detect version and configuration.
- Discover current authentication.
- Add multiple ChatGPT or supported API-authenticated profiles.
- Keep account state isolated.
- Select one active profile.
- Switch between accounts.
- Preserve Codex sessions and settings.
- Monitor usage when reliable interfaces are available.
- Manage supported MCP configuration, skills, and preferences.
- Detect expired authentication.
- Support recovery after unsuccessful switching.

### Technical Strategy

Codex supports the `CODEX_HOME` environment variable.

Its state directory contains account-related configuration, authentication, sessions, and other runtime data.

Example:

```text
profiles/
└── codex/
    ├── personal/
    ├── work/
    └── secondary/
```

Each saved Codex account should maintain an independent profile.

The account manager should select which profile is active for new sessions.

For the requirement that an unmodified `codex` command works everywhere, the backend must evaluate and validate safe activation of the default Codex state.

Existing Codex processes or IDE integrations may retain their previous state until restarted.

The application must detect such situations and avoid reporting complete activation prematurely.

### Authentication Types

Support relevant sign-in methods offered by the installed Codex version, such as:

- ChatGPT-based authentication.
- Supported API-key authentication.
- Organizational authentication where officially supported.

Store account secrets only through approved, protected mechanisms.

---

## 4.3 Google Antigravity

Google Antigravity is a major integration and requires a dedicated provider module.

It should be designed to support Antigravity's available application surfaces:

- Antigravity desktop application.
- Antigravity IDE.
- Antigravity CLI.
- Supported Antigravity integrations in other IDEs.

### Features

- Detect Antigravity installation.
- Detect version and supported authentication mechanism.
- Discover the currently authenticated Google account.
- Add multiple authorized Google accounts.
- Save account metadata and supported authentication state securely.
- Select one active account.
- Switch between saved accounts.
- Detect relevant Antigravity processes.
- Identify whether an IDE restart or language-server reload is necessary.
- Preserve project and workspace state.
- Support controlled application restart when required.
- Detect authentication failures.
- Monitor available usage and model quota information where supported.
- Restore the previous account when switching fails.

### Authentication Strategy

Google Antigravity uses a unified authentication system across its application, CLI, and supported IDE integrations.

This means authentication changes can affect multiple Antigravity surfaces.

The application must prioritize documented account and sign-in mechanisms.

If a reliable public account-switching API is unavailable, the adapter may experimentally investigate local authentication state or supported profile isolation.

However:

- No hardcoded token substitution should be assumed safe.
- No undocumented credential mutation should be enabled by default.
- No credential copying should occur without validation.
- No operation should corrupt the user's existing Antigravity profile.
- All experimental integrations must be version-gated.
- The user must be informed when switching requires an IDE restart.
- The application must not interfere with active agent tasks without permission.

The adapter must verify that the selected account is actually active before showing success.

### Antigravity-Specific Limitations

The application should not promise that an active Antigravity AI conversation can continue seamlessly under another Google account.

Switching may restart background services, invalidate current conversation context, or require the user to begin a new session.

Where provider internals are undocumented, this integration should be clearly labeled Experimental until tested and verified.

---

## 4.4 Gemini Integration

Gemini support should initially focus on Google accounts used by Antigravity.

### Initial Scope

- Identify the Google account connected to Antigravity.
- Display supported subscription or plan information.
- Display available model quota information where legitimately accessible.
- Switch the selected Google account through the Antigravity adapter.
- Distinguish subscription-based model access from API-key-based access.

### Future Scope

- Standalone Gemini CLI profile management where independently supported.
- Gemini API key profiles.
- Google Cloud-based authentication.
- Project-specific configurations.

Gemini API keys and consumer Google accounts must be treated as different authentication types.

The application must never assume that switching an Antigravity Google account also switches unrelated Gemini integrations that do not use the same authentication system.

---

# 5. Universal Account Management

The most important feature is a centralized interface for managing every saved account.

## 5.1 Add Account

Users can add multiple accounts for a provider.

The account creation flow should include:

1. Select provider.
2. Select supported authentication method.
3. Start the provider's official authentication flow.
4. Authenticate through the provider.
5. Verify account identity.
6. Capture permitted account metadata.
7. Save protected authentication state where supported.
8. Assign a friendly account name.
9. Add the account to the provider's account list.

### Account Information

Store appropriate non-secret metadata:

- Unique internal account ID.
- Provider ID.
- Account label.
- Account email or identifier, if available.
- Subscription type, if available.
- Account status.
- Active/inactive status.
- Authentication method.
- Date added.
- Last successfully authenticated.
- Last successfully used.
- Last successful quota refresh.
- Associated provider configuration path.
- Last switching error.

The user should be able to rename accounts without changing their original authentication identity.

## 5.2 Account Categories

Support custom account labels, such as:

- Personal
- Work
- University
- Secondary
- Testing
- Custom

These labels are organizational only.

## 5.3 Account Operations

Every account should support relevant operations:

- Activate.
- Rename.
- View details.
- Refresh status.
- Reauthenticate.
- View usage.
- Open related application.
- Export permitted configuration.
- Create backup.
- Restore backup.
- Remove from manager.
- Remove managed local authentication data, with confirmation.

Removing an account from the manager should not automatically delete an account from the AI provider.

## 5.4 One Active Account Per Provider

Each provider maintains one selected default account at a time.

Example:

```text
Claude Code
  Personal      ACTIVE
  Work
  Secondary

Codex
  Personal
  Work          ACTIVE

Antigravity
  Google Main   ACTIVE
  Google Alt
```

Users may simultaneously have one active profile for each different provider.

The manager must distinguish the selected default profile from already-running sessions that still use another account.

---

# 6. Universal Account Switching Engine

The switching engine is the application's most important backend component.

It should coordinate provider-specific authentication and profile changes through a common Rust interface.

## 6.1 Switching Goals

- One-click switching.
- No repeated authentication when saved credentials remain valid.
- No manual file operations.
- No manual environment-variable configuration.
- No external launcher required by the user.
- Minimal disruption to running applications.
- Reliable success verification.
- Automatic rollback after failure.

## 6.2 Switching Modes

The application must support several switch modes.

### Mode A — Immediate

A provider offers a safe, validated mechanism to activate a saved account without restarting an application.

The account manager activates the account and verifies success.

### Mode B — New Sessions Only

The selected account is activated for subsequent supported sessions, but existing processes remain connected to their previous accounts.

The UI must explicitly show this distinction.

### Mode C — Restart Required

The provider requires an application or background service restart.

The manager should:

1. Identify running processes.
2. Detect known active tasks where possible.
3. Warn the user.
4. Wait for permission.
5. Save or back up necessary state.
6. Activate the account.
7. Restart or reload the relevant application.
8. Verify the new account.

### Mode D — Manual Action Required

The provider cannot safely complete switching automatically.

Display the required official sign-in or account-selection action.

### Mode E — Unsupported

The provider version is incompatible with the available switching mechanism.

Disable switching rather than attempting risky modifications.

## 6.3 Switching Workflow

```text
User selects account
        |
        v
Validate account and provider
        |
        v
Check authentication state
        |
        v
Detect running applications
        |
        v
Determine switching mode
        |
        v
Check for active tasks
        |
        v
Acquire provider switch lock
        |
        v
Create recovery snapshot
        |
        v
Activate selected profile
        |
        v
Reload application if necessary
        |
        v
Verify active account identity
        |
   +----+----+
   |         |
 Success   Failure
   |         |
   v         v
 Commit    Rollback
   |         |
   v         v
Update UI  Show error
```

## 6.4 Atomic Switching

Switching should behave like a transaction.

A provider must never be left in an unknown or partially modified state without recovery information.

The engine should:

- Acquire a provider-specific lock.
- Identify affected files and credential references.
- Validate source and destination profiles.
- Create recovery snapshots.
- Use atomic filesystem operations where appropriate.
- Ensure required data is flushed safely.
- Verify the resulting authentication state.
- Record the operation.
- Roll back on failure.

Atomic filesystem operations alone do not provide transactional behavior across files, processes, and OS keychain entries. The recovery system must account for each resource separately.

## 6.5 Crash Recovery

If the application crashes or the operating system shuts down during switching:

- Detect the incomplete operation on next launch.
- Read the switch journal.
- Check which steps completed.
- Avoid starting another switch automatically.
- Offer safe recovery or rollback.
- Preserve original account data.
- Record the recovery result.

## 6.6 Concurrent Operation Protection

Only one switching operation may modify a particular provider's active state at a time.

Switches for unrelated providers can run independently if their resources do not overlap.

For example, Codex and Claude Code could switch independently, while two Antigravity switches must be serialized.

## 6.7 Switching Performance

Performance targets should prioritize correctness over arbitrary speed.

- Tray opening should feel immediate.
- Account selection should update the UI promptly.
- File-only switching should avoid unnecessary work.
- Restart-based operations should show progress.
- Long authentication checks must not block the UI.
- The manager should never freeze while waiting for another process.

No fixed switching duration can be guaranteed for external provider applications.

---

# 7. Menu Bar and System Tray Interface

The interface should be intentionally minimal.

The app should not resemble a complicated developer dashboard during normal use.

## 7.1 Compact Interface

Example:

```text
┌────────────────────────────────┐
│ AI Account Manager         ⚙  │
├────────────────────────────────┤
│ CLAUDE CODE                    │
│ ● Personal             Active  │
│ ○ Secondary            Switch  │
│ ○ Work                 Switch  │
│                                │
│ CODEX                          │
│ ● ChatGPT Plus         Active  │
│ ○ Work                 Switch  │
│                                │
│ ANTIGRAVITY                    │
│ ● Google Main          Active  │
│ ○ Google Secondary     Switch  │
│                                │
├────────────────────────────────┤
│ + Add Account                  │
│ Manage Accounts                │
│ Settings                       │
│ Quit                           │
└────────────────────────────────┘
```

## 7.2 Visual Requirements

- Clean, compact layout.
- Native-feeling typography.
- Rounded containers.
- Neutral color palette.
- Subtle borders.
- Appropriate provider icons.
- Minimal animation.
- Clear active-account indicators.
- Accessible status colors.
- Light and dark mode.
- Keyboard navigation.
- Small-screen compatibility.
- No unnecessary decorative graphics.
- No oversized cards or excessive spacing.

## 7.3 Account Row

Every account row may display:

- Account label.
- Email or short identifier.
- Subscription tier.
- Active status.
- Last known usage percentage.
- Authentication health.
- Switch button or action.

The row should visually distinguish:

- Active and verified.
- Selected for new sessions.
- Switching.
- Restart required.
- Authentication expired.
- Error.
- Incompatible provider version.

## 7.4 Tray Icon

Provide a simple monochrome icon that works with native system themes.

Possible states:

- Normal.
- Switching.
- Attention required.
- Error.
- Disconnected.

Avoid continuously flashing or animating the icon.

## 7.5 Interaction

### macOS

- Left click opens the compact interface.
- Right click can open a native context menu.
- Clicking outside dismisses the interface.
- Optional keyboard shortcut opens the account selector.

### Windows

- Left click opens the quick-switch interface.
- Right click opens the native tray menu.
- Double click may open the full dashboard.

### Linux

Use the supported tray interaction model.

If the desktop environment does not support a custom left-click popover, provide equivalent functionality through the tray menu or main window.

---

# 8. Full Management Dashboard

The application should also include a dedicated management window.

This window is intended for account setup, monitoring, configuration, and troubleshooting.

## 8.1 Dashboard

Display:

- Installed supported providers.
- Total saved accounts.
- Active account per provider.
- Authentication health.
- Available usage data.
- Recent switches.
- Pending restart requirements.
- Detected integration problems.

## 8.2 Accounts Page

Provide:

- Provider filters.
- Account search.
- Account labels.
- Account status.
- Subscription information.
- Usage information.
- Activation controls.
- Edit and remove actions.

## 8.3 Account Details Page

Show available metadata and supported capabilities for the selected account.

Sections may include:

- Identity.
- Authentication.
- Usage.
- Profile configuration.
- Sessions.
- MCP servers.
- Skills.
- Hooks.
- Backup history.
- Diagnostics.

Only display sections supported by that provider.

## 8.4 Activity Page

Maintain a local activity history.

Example:

```text
10:35  Claude Code → Personal
10:41  Codex → Work
10:52  Antigravity → Google Secondary
10:53  Antigravity restart required
10:54  Antigravity switch verified
```

The activity log should never contain passwords, access tokens, refresh tokens, or full session credentials.

---

# 9. Usage and Quota Monitoring

The application should offer centralized usage monitoring wherever reliable provider data is available.

## 9.1 Supported Information

Depending on the provider:

- Current usage percentage.
- Remaining quota.
- Rolling usage window.
- Weekly usage.
- Model-specific quota.
- Reset time.
- Subscription tier.
- Rate-limit state.
- Authentication status.
- Last refreshed time.

## 9.2 Usage Dashboard

Example:

```text
CLAUDE CODE

Personal
5-hour usage     42%
Weekly usage     63%
Reset            3h 15m

Work
5-hour usage     81%
Weekly usage     27%
Reset            1h 40m


CODEX

Personal
Usage            68%
Reset            Provider reported time


ANTIGRAVITY

Google Main
Gemini Model A   34% remaining
Gemini Model B   76% remaining
```

The numbers above are illustrative only.

## 9.3 Data Retrieval

Use the following priority:

1. Official provider APIs or documented integrations.
2. Documented local usage records.
3. Verified local session logs.
4. Explicitly enabled experimental integrations.

Never treat estimates as exact usage.

Do not reverse-engineer or repeatedly call undocumented quota endpoints by default.

## 9.4 Freshness

Every usage reading should record:

- Source.
- Retrieval time.
- Account identity.
- Data age.
- Reliability.
- Estimated or exact status.

Possible states:

- Fresh.
- Stale.
- Estimated.
- Unsupported.
- Authentication required.
- Temporarily unavailable.

## 9.5 Refresh Controls

- Manual refresh.
- Configurable automatic refresh.
- Adaptive polling.
- Refresh when opening the dashboard.
- Pause polling when unnecessary.
- Respect rate limits.
- Back off after errors.

## 9.6 Usage Alerts

Optional notifications may include:

- Usage near a user-defined threshold.
- Quota exhausted.
- Quota reset confirmed.
- Credentials expired.
- Account requiring reauthentication.

The application may recommend another saved account, but should not automatically switch during an active task.

Account selection should remain under user control and comply with provider usage policies.

---

# 10. Session and Conversation Management

The application should preserve existing account-specific development context whenever possible.

This feature is secondary to account switching but important for a complete multi-account experience.

## 10.1 Session Discovery

Where supported, discover:

- Local Claude Code sessions.
- Local Codex sessions.
- Provider-supported conversation metadata.
- Associated projects.
- Session timestamps.
- Last-used profiles.

## 10.2 Session Browser

Provide a read-only browser for supported local sessions.

Possible filters:

- Provider.
- Account.
- Project.
- Date.
- Session status.

## 10.3 Session Preservation

Switching accounts should not automatically:

- Delete conversation history.
- Delete local sessions.
- Modify project files.
- Remove workspaces.
- Clear agent settings.
- Overwrite provider-managed session databases.

## 10.4 Session Transfer

Future versions may support exporting or copying permitted local session data.

However, the application must distinguish between:

- Copying local conversation records.
- Resuming a provider-native session.
- Transferring context to a different provider.

These are different operations.

Copying a session file does not guarantee that another account or provider can resume that conversation.

## 10.5 Chat Synchronization

AccountDeck-inspired chat synchronization may be explored as an optional advanced feature.

Requirements:

- Provider-specific support only.
- Explicit opt-in.
- Backup before synchronization.
- Conflict detection.
- Clear handling of deleted conversations.
- No copying credentials.
- No claim of universal cross-provider compatibility.

This feature should not be included in the initial switching MVP.

---

# 11. Provider Configuration Management

Users may want their preferred development configuration across multiple accounts.

The app should support safe management of provider-specific settings.

## 11.1 Claude Code Configuration

Potential supported items:

- MCP server configuration.
- Skills.
- Hooks.
- Preferences.
- Allowed permissions.
- Compatible plugin metadata.
- User settings.

## 11.2 Codex Configuration

Potential supported items:

- MCP configuration.
- Skills.
- Model preferences.
- Approval settings.
- Sandbox configuration.
- Profile preferences.

## 11.3 Antigravity Configuration

Only copy settings that are documented, portable, and independent of authentication state.

Examples may include:

- Supported user preferences.
- Non-secret configuration.
- Provider-approved profile settings.

Do not blindly copy Antigravity's complete application data directory.

## 11.4 Copy Configuration

Workflow:

1. Select source account.
2. Select destination account.
3. Select configuration categories.
4. Preview changes.
5. Detect conflicts.
6. Create backup.
7. Apply safe changes.
8. Verify configuration.

Never overwrite authentication state as part of configuration sharing.

---

# 12. Security and Privacy Architecture

Security is a foundational product requirement.

The app will interact with authentication-sensitive data belonging to external services.

## 12.1 Security Principles

- Local-first operation.
- No mandatory cloud backend.
- No proprietary authentication proxy.
- No telemetry by default.
- No password collection.
- No logging of bearer tokens.
- No plaintext credential database.
- No credentials exposed to the React frontend.
- No unauthorized session modifications.
- Use official authentication methods wherever available.
- Minimize filesystem permissions.
- Ask before destructive or disruptive operations.

## 12.2 Secret Storage

### macOS

Prefer OS Keychain services with appropriate access controls.

### Windows

Use Windows-protected credential storage.

### Linux

Prefer a Secret Service-compatible credential store.

If protected storage is unavailable, notify the user and disable operations requiring persistent secrets until a secure alternative is configured.

## 12.3 SQLite Storage

SQLite should primarily hold non-secret metadata:

- Provider registry.
- Account labels.
- Account identifiers.
- Profile references.
- Active account selections.
- Usage snapshots.
- Switch history.
- Settings.
- Backup records.

Credentials should be stored separately in protected storage.

Where provider-owned credentials already reside in provider-managed stores, the application should avoid unnecessary duplication.

## 12.4 Encryption

Any manager-owned sensitive backup containing authentication state must be encrypted.

The encryption key should not be stored beside the encrypted data.

Avoid creating readable temporary files containing credentials.

## 12.5 OAuth and Authentication

Use provider-supported authentication flows.

The application must not:

- Request provider passwords directly.
- Scrape browser passwords.
- Bypass normal account authorization.
- Reuse OAuth tokens across unrelated client applications without authorization.
- Assume existing provider credentials can legally or technically be imported into a new OAuth client.

Where an external provider does not support third-party authentication integration, use the provider's own login process.

## 12.6 Process Safety

Before modifying a provider's active profile:

- Identify relevant running processes.
- Check whether their state may be affected.
- Avoid interrupting running agent tasks.
- Require confirmation for disruptive actions.
- Preserve a recovery snapshot.

## 12.7 Privacy

The application should operate without requiring users to create an application-specific account.

No account metadata or session contents should be uploaded to a central service.

The only required network activity should be communication with supported AI providers for authentication, usage retrieval, updates if enabled, and other explicitly authorized functions.

---

# 13. Application Architecture

## 13.1 Technology Stack

| Component | Technology |
|---|---|
| Desktop framework | Tauri 2 |
| Core language | Rust |
| Frontend | React |
| Frontend language | TypeScript |
| Build tool | Vite |
| Styling | Tailwind CSS |
| UI components | shadcn/ui |
| Icons | Lucide |
| Database | SQLite |
| Database access | rusqlite or SQLx |
| State management | Zustand or React state |
| Form validation | Zod |
| Credential storage | Native OS credential stores |
| Process management | Rust standard library and OS APIs |
| Background monitoring | Tokio and filesystem watchers |
| Application messaging | Tauri commands and events |
| Packaging | Tauri bundler |

The implementation should favor minimal dependencies.

## 13.2 High-Level Architecture

```text
┌──────────────────────────────────────┐
│            Tauri 2 App               │
│                                      │
│   Menu Bar / Tray / Dashboard        │
│                                      │
│   React + TypeScript + shadcn/ui     │
└─────────────────┬────────────────────┘
                  │
            Tauri IPC
                  │
                  ▼
┌──────────────────────────────────────┐
│           Rust Backend               │
│                                      │
│  Account Manager                     │
│  Profile Manager                     │
│  Authentication Coordinator          │
│  Switch Engine                       │
│  Process Manager                     │
│  Usage Monitor                       │
│  Session Manager                     │
│  Backup & Recovery                   │
│  Notification Manager                │
│                                      │
└─────────────────┬────────────────────┘
                  │
                  ▼
┌──────────────────────────────────────┐
│          Provider Adapters           │
│                                      │
│  Claude Code Adapter                 │
│  Codex Adapter                       │
│  Antigravity Adapter                 │
│  Future Provider Adapters            │
└─────────────────┬────────────────────┘
                  │
                  ▼
┌──────────────────────────────────────┐
│         Operating System             │
│                                      │
│  Filesystem                          │
│  Credential Store                    │
│  Application Processes               │
│  Provider Configurations             │
│  Native Notifications                │
└──────────────────────────────────────┘
```

## 13.3 Backend Responsibility

All sensitive operations must be performed by Rust.

The frontend should never directly manipulate authentication files or credentials.

The backend should provide narrowly scoped commands for:

- Listing providers.
- Listing accounts.
- Starting supported authentication.
- Activating accounts.
- Reading usage metadata.
- Managing settings.
- Discovering processes.
- Creating backups.
- Restoring profiles.
- Viewing activity history.

## 13.4 Background Operation

The same Tauri application process should manage all required background tasks.

No separate daemon, localhost web server, or CLI companion should be required for standard operation.

Background services may include internal asynchronous Rust tasks for:

- File change monitoring.
- Usage refresh scheduling.
- Account health checks.
- Switch coordination.
- Application process detection.

The application should not keep expensive polling loops running unnecessarily.

---

# 14. Provider Adapter Architecture

Every provider should implement a consistent Rust interface.

The purpose is to avoid hardcoding Claude, Codex, and Antigravity behavior into the UI or shared switching engine.

## 14.1 Adapter Responsibilities

Each adapter should define:

- Provider identity.
- Supported operating systems.
- Supported application versions.
- Installation detection.
- Account discovery.
- Supported sign-in methods.
- Authentication validation.
- Profile activation.
- Running-process detection.
- Reload requirements.
- Quota support.
- Session support.
- Configuration support.
- Backup and rollback support.

## 14.2 Conceptual Rust Interface

```rust
trait ProviderAdapter {
    fn id(&self) -> &'static str;

    async fn detect(
        &self
    ) -> Result<ProviderInfo>;

    async fn list_accounts(
        &self
    ) -> Result<Vec<Account>>;

    async fn validate_account(
        &self,
        account_id: &str
    ) -> Result<AuthStatus>;

    async fn switch_account(
        &self,
        account_id: &str
    ) -> Result<SwitchResult>;

    async fn get_usage(
        &self,
        account_id: &str
    ) -> Result<UsageStatus>;

    async fn restore_previous(
        &self
    ) -> Result<()>;
}
```

This is conceptual architecture, not production-ready compilable code.

Real implementations should include cancellation, structured error types, authorization handling, version compatibility, transaction context, and recovery operations.

## 14.3 Capabilities

Every provider should expose capability metadata.

Example:

```json
{
  "provider": "claude-code",
  "capabilities": {
    "multiple_accounts": true,
    "profile_isolation": true,
    "account_switching": true,
    "live_switching": false,
    "session_management": true,
    "usage_monitoring": "conditional",
    "restart_required": "conditional"
  }
}
```

Capability values must reflect tested integration behavior.

The interface should automatically hide unsupported controls rather than presenting features that cannot work.

---

# 15. Local Data Architecture

## 15.1 Application Data

Store application metadata under standard OS-specific application data locations.

Illustrative structure:

```text
app-data/
├── database/
│   └── manager.db
│
├── profiles/
│   ├── claude/
│   ├── codex/
│   └── antigravity/
│
├── backups/
│
├── logs/
│
├── recovery/
│
└── settings/
```

Actual storage locations must follow operating-system conventions.

For example:

- macOS: Application Support directory.
- Windows: LocalAppData.
- Linux: XDG data and configuration directories.

Sensitive provider files must not be copied into these directories unless there is a justified, protected, provider-compatible mechanism.

## 15.2 SQLite Tables

### providers

```text
id
name
enabled
installed_version
integration_version
compatibility_status
last_detected_at
```

### accounts

```text
id
provider_id
label
email
account_identifier
subscription_type
auth_method
profile_reference
status
created_at
last_authenticated_at
last_used_at
```

### active_accounts

```text
provider_id
account_id
activation_state
verified_at
```

### usage_snapshots

```text
id
provider_id
account_id
usage_type
used_value
limit_value
reset_at
data_source
confidence
retrieved_at
```

### switch_history

```text
id
provider_id
from_account_id
to_account_id
status
switch_mode
error_code
started_at
completed_at
```

### recovery_snapshots

```text
id
provider_id
operation_id
snapshot_reference
created_at
status
```

### app_settings

```text
key
value
updated_at
```

Foreign keys and proper constraints must be included.

Only one selected default account may exist per provider.

---

# 16. First Launch and Onboarding

The onboarding process should be simple and optional after initial setup.

## Step 1 — Welcome

Introduce the product:

"Manage all your AI coding accounts from one place."

## Step 2 — Detect Installed Tools

Scan for supported installed applications.

Example:

```text
Installed Tools

✓ Claude Code
✓ Codex CLI
✓ Google Antigravity
```

Detection must be read-only.

## Step 3 — Import Existing Account

Offer to register existing locally authenticated accounts.

Before importing:

- Detect supported account identity.
- Explain what metadata will be accessed.
- Request user permission.
- Avoid exposing or copying credentials unnecessarily.
- Preserve original configurations.

## Step 4 — Add Additional Accounts

Allow the user to authenticate additional accounts through official provider flows.

## Step 5 — Choose Defaults

Select the default account for each provider.

## Step 6 — Configure Background Behavior

Optional settings:

- Launch at login.
- Start minimized.
- Enable notifications.
- Enable usage monitoring.
- Enable automatic backups.

## Step 7 — Complete

Close onboarding and show the menu bar or tray interface.

The user can add more accounts later.

---

# 17. Settings and Preferences

## 17.1 General

- Launch at startup.
- Start minimized.
- Close window to tray.
- Show/hide Dock icon on macOS where supported.
- Theme: System, Light, Dark.
- Language.
- Notification preferences.
- Application update preferences.

## 17.2 Account Switching

- Confirm application restarts.
- Warn about running tasks.
- Enable safe automatic switching where supported.
- Verify account after switching.
- Enable automatic rollback.
- Configure restart behavior.
- Set per-provider switching preferences.

## 17.3 Usage Monitoring

- Enable usage monitoring.
- Refresh interval.
- Notification thresholds.
- Show reset countdowns.
- Reduce background polling.
- Display only verified usage data.

## 17.4 Privacy and Security

- Credential storage status.
- Lock sensitive account management actions.
- Diagnostic logging level.
- Clear non-sensitive cache.
- Export application settings.
- Backup and restore configuration.
- Reset application settings.
- Remove managed application data.

## 17.5 Advanced

- Provider compatibility diagnostics.
- Installed application paths.
- Experimental features.
- Provider adapter version.
- Recovery snapshots.
- Debug information.
- Export sanitized diagnostics.

---

# 18. Notifications

Use native operating-system notifications.

### Supported Notifications

**Account Switched**

"Claude Code is now using Personal."

**Restart Required**

"Antigravity needs to restart to complete the account switch."

**Authentication Expired**

"Your Codex Work account needs authentication."

**Usage Warning**

"Claude Code Personal has reached your configured usage threshold."

**Switch Failed**

"Unable to activate Google Secondary. Your previous account has been restored."

Notifications must be optional, concise, and actionable.

Do not repeatedly notify users about unchanged states.

---

# 19. Advanced Features

These features can be introduced after the core app becomes reliable.

## 19.1 Account Health Monitoring

Check whether saved accounts remain usable.

Possible indicators:

- Authenticated.
- Token expired.
- Reauthentication needed.
- Provider unavailable.
- Profile corrupted.
- Unknown.

## 19.2 Smart Account Suggestions

Offer recommendations based on available information.

Example:

"Your current Claude account is close to its usage limit. Work has recently reported available capacity."

Suggestions must not automatically switch accounts.

The user makes the final decision.

## 19.3 Favorites

Allow users to mark frequently used accounts as favorites.

Favorites can appear first within a provider.

## 19.4 Recent Accounts

Show recently activated accounts in the quick-switch interface.

## 19.5 Keyboard Shortcuts

Optional system-wide shortcut to open the account selector.

Support accessible keyboard navigation and account selection.

## 19.6 Usage History

Store permitted usage snapshots locally.

Provide:

- Usage trends.
- Previous reset observations.
- Provider comparisons.
- Account-level usage charts.

Do not invent missing historical data.

## 19.7 Configuration Sharing

Copy supported non-secret configuration across accounts.

## 19.8 Backup and Restore

Allow the user to create and restore supported application state backups.

Authentication backups must remain encrypted and should not be represented as universally portable between devices.

## 19.9 Provider Plugin Architecture

Future versions may introduce additional provider adapters.

Initially, adapters should be compiled into the Rust application.

An external plugin system should be considered only after a safe, permission-based plugin contract is established.

## 19.10 Additional Providers

Potential integrations:

- Standalone Gemini CLI.
- Other officially supported AI coding CLIs.
- Additional desktop AI assistants.
- Other AI development environments.

Each future integration must meet the same security and reliability requirements.

---

# 20. Performance Requirements

The application should be optimized for low resource consumption.

## 20.1 Resource Goals

- Lightweight resident process.
- Minimal CPU usage while idle.
- Low memory overhead.
- No constantly running embedded server.
- No heavyweight browser automation.
- No unnecessary background polling.
- Lazy loading of management views.
- Asynchronous filesystem and network operations.
- Minimal frontend bundle.

## 20.2 Responsiveness

- Tray interaction must remain responsive.
- Switching must not block rendering.
- Long-running operations must support cancellation where safe.
- Background refreshes must not interrupt other actions.
- UI components should render only necessary data.

## 20.3 Application Lifetime

Closing the main window must not terminate account management.

The application should continue running in the background until the user explicitly quits or the operating system ends the process.

## 20.4 Resource Targets

Performance targets should be established through testing on macOS, Windows, and Linux rather than assuming that a small Tauri bundle guarantees low runtime memory usage.

---

# 21. Reliability and Error Handling

The application must treat account state as valuable user data.

## 21.1 Failure Scenarios

Handle at least:

- Missing provider installation.
- Unsupported provider version.
- Expired authentication.
- Locked credential store.
- Filesystem permission failure.
- Network outage.
- Interrupted authentication.
- Provider process refusing to stop.
- Failed application restart.
- Invalid profile state.
- Conflicting provider configuration.
- App crash during switching.
- User cancellation.
- Changes to provider authentication internals.

## 21.2 Recovery Rules

- Never delete the original profile during onboarding.
- Always preserve a restorable baseline before risky changes.
- Never replace an active profile without verifying the destination.
- Never claim success solely because a file operation completed.
- Restore the previous state whenever restoration is safe.
- Report cases requiring manual recovery.
- Keep a record of recovery actions.

## 21.3 Provider Updates

Provider updates may change authentication structures or behavior.

Every adapter should maintain a compatibility policy.

If a provider version has not been validated, disable potentially destructive switching behavior until compatibility is confirmed.

---

# 22. Proposed Repository Structure

```text
altflip/
│
├── src/
│   ├── app/
│   ├── components/
│   │   ├── ui/
│   │   ├── accounts/
│   │   ├── providers/
│   │   ├── usage/
│   │   └── settings/
│   │
│   ├── features/
│   │   ├── dashboard/
│   │   ├── account-manager/
│   │   ├── account-switcher/
│   │   ├── usage-monitor/
│   │   ├── sessions/
│   │   ├── settings/
│   │   └── onboarding/
│   │
│   ├── hooks/
│   ├── lib/
│   ├── stores/
│   ├── types/
│   ├── styles/
│   └── main.tsx
│
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   │
│   │   ├── commands/
│   │   ├── core/
│   │   │   ├── accounts/
│   │   │   ├── authentication/
│   │   │   ├── profiles/
│   │   │   ├── switching/
│   │   │   ├── processes/
│   │   │   ├── monitoring/
│   │   │   ├── recovery/
│   │   │   └── security/
│   │   │
│   │   ├── providers/
│   │   │   ├── mod.rs
│   │   │   ├── claude/
│   │   │   ├── codex/
│   │   │   └── antigravity/
│   │   │
│   │   ├── database/
│   │   ├── platform/
│   │   │   ├── macos/
│   │   │   ├── windows/
│   │   │   └── linux/
│   │   │
│   │   └── utils/
│   │
│   ├── migrations/
│   ├── capabilities/
│   ├── icons/
│   ├── Cargo.toml
│   └── tauri.conf.json
│
├── assets/
├── docs/
├── tests/
│   ├── integration/
│   ├── providers/
│   └── recovery/
│
├── .github/
│   └── workflows/
│
├── package.json
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

---

# 23. Development Roadmap

## Phase 1 — Project Foundation

Build the basic desktop application.

### Tasks

- Initialize Tauri 2.
- Configure React and TypeScript.
- Install Tailwind CSS and shadcn/ui.
- Implement theme support.
- Create menu bar/tray behavior.
- Implement the main management window.
- Configure native OS integrations.
- Implement application startup preferences.
- Configure SQLite.
- Set up logging and error handling.

### Deliverable

A lightweight, functional cross-platform desktop shell.

---

## Phase 2 — Core Account Manager

### Tasks

- Create provider registry.
- Implement account data models.
- Implement provider detection.
- Implement account list.
- Implement account labels.
- Implement active account selection.
- Integrate protected credential storage.
- Create onboarding.
- Add account import workflow.
- Implement account health reporting.

### Deliverable

Users can register and organize accounts without switching them yet.

---

## Phase 3 — Claude Code Integration

### Tasks

- Detect Claude Code.
- Discover existing account profile.
- Add isolated profiles.
- Integrate supported sign-in.
- Implement account validation.
- Develop and test safe default-profile activation.
- Preserve history and configuration.
- Detect running processes.
- Implement rollback.
- Test switching across supported operating systems.

### Deliverable

Reliable switching between previously authenticated Claude Code profiles for validated scenarios.

---

## Phase 4 — Codex Integration

### Tasks

- Detect Codex.
- Discover `CODEX_HOME`.
- Add multiple profiles.
- Implement account validation.
- Implement default-profile activation where compatible.
- Preserve session history.
- Support required reauthentication.
- Implement rollback.
- Test ordinary Codex CLI usage.

### Deliverable

Reliable switching between saved Codex accounts for supported versions and workflows.

---

## Phase 5 — Antigravity Integration

### Tasks

- Research supported Antigravity authentication interfaces.
- Detect installed Antigravity applications.
- Identify account state and process dependencies.
- Implement account registration through supported authentication.
- Design safe profile activation.
- Evaluate whether switching can be completed entirely outside the IDE.
- Implement restart coordination when necessary.
- Verify account identity after switching.
- Implement encrypted backup and recovery.
- Validate behavior across supported operating systems.
- Gate experimental techniques behind explicit settings.

### Deliverable

The strongest safe Antigravity account-switching integration achievable without requiring a companion extension.

Do not mark the integration stable until it passes real-device switching and recovery tests.

---

## Phase 6 — Usage Monitoring

### Tasks

- Define common usage models.
- Implement provider-specific retrieval.
- Display quota freshness.
- Add reset countdowns.
- Add refresh controls.
- Implement notifications.
- Handle unavailable and stale data.
- Add local usage history.

### Deliverable

A unified usage dashboard for providers exposing reliable usage information.

---

## Phase 7 — Session and Configuration Features

### Tasks

- Add session discovery.
- Add read-only session browser.
- Implement supported configuration export.
- Add selected settings copy.
- Add backup management.
- Test conflicts.
- Preserve project data.

### Deliverable

A complete account management and configuration utility.

---

## Phase 8 — Cross-Platform Hardening

### Tasks

- Test macOS Apple Silicon and Intel.
- Test Windows x64.
- Test Windows ARM64 where supported.
- Test mainstream Linux environments.
- Validate tray behavior.
- Test credential-store compatibility.
- Test application startup.
- Test external tool upgrades.
- Test interrupted switching.
- Test crash recovery.
- Review security permissions.
- Optimize memory and CPU usage.

### Deliverable

Stable, reproducible releases with documented platform limitations.

---

# 24. Testing Requirements

## 24.1 Unit Tests

Cover:

- Account validation.
- Profile resolution.
- Switching decisions.
- Capability detection.
- Database constraints.
- Usage calculations.
- Recovery journal logic.
- Error handling.

## 24.2 Integration Tests

Cover:

- Provider installation detection.
- Profile creation.
- Authentication-state validation.
- Successful account switching.
- Expired account handling.
- Restart-required switching.
- Rollback after failure.
- Backup restoration.
- Application update compatibility.

## 24.3 End-to-End Tests

Simulate the complete user journey:

1. Launch the application.
2. Detect installed providers.
3. Register an existing account.
4. Add a second account.
5. Select the second account.
6. Verify the account in the actual target application.
7. Start a new session.
8. Switch back.
9. Verify the original account.
10. Confirm configuration and history remain intact.

## 24.4 Destructive Failure Tests

Before release, intentionally test:

- App termination during switching.
- Interrupted filesystem operations.
- Permission failures.
- Corrupted backup records.
- Provider process locks.
- Missing credentials.
- Unexpected provider version changes.

Use isolated test profiles.

Never perform destructive integration testing against users' primary credentials or workspaces.

---

# 25. Product Acceptance Criteria

The initial application should not be considered production-ready unless the following requirements are met.

## Core Application

- [ ] Runs on macOS, Windows, and supported Linux environments.
- [ ] Opens through a menu bar or system tray interface.
- [ ] Works without a companion CLI or IDE extension.
- [ ] Supports automatic startup.
- [ ] Stores application metadata locally.
- [ ] Keeps secrets outside the frontend.
- [ ] Provides a usable main management window.

## Account Management

- [ ] Detects supported installed providers.
- [ ] Supports multiple saved accounts per provider.
- [ ] Allows users to rename accounts.
- [ ] Tracks one selected default account per provider.
- [ ] Supports account removal.
- [ ] Detects expired authentication where possible.
- [ ] Preserves existing provider configurations during onboarding.

## Switching

- [ ] Validates destination accounts.
- [ ] Detects the switching capabilities of the provider.
- [ ] Handles supported profile activation.
- [ ] Warns before restarting applications.
- [ ] Does not silently interrupt running agent tasks.
- [ ] Verifies the active account after switching.
- [ ] Records switch results.
- [ ] Supports rollback or explicit recovery.
- [ ] Does not claim unsupported instant switching.

## Security

- [ ] Uses protected secret storage.
- [ ] Never logs credentials.
- [ ] Requires consent for risky operations.
- [ ] Maintains recovery snapshots.
- [ ] Protects locally stored account data.
- [ ] Does not require a proprietary cloud server.

## Quality

- [ ] Responsive interface.
- [ ] Low idle resource usage.
- [ ] Clear error messages.
- [ ] Clean light/dark themes.
- [ ] Cross-platform packaging.
- [ ] Automated testing.
- [ ] Documented compatibility limitations.

---

# 26. Non-Goals

To maintain a focused product, the initial implementation should not attempt to:

- Replace Claude Code, Codex, or Antigravity.
- Become a full AI IDE.
- Provide a separate CLI companion.
- Require users to install browser extensions.
- Require users to install IDE plugins.
- Host AI conversations on a central server.
- Proxy all provider traffic.
- Collect passwords directly.
- Automatically bypass subscription limits.
- Share paid subscriptions between unrelated accounts.
- Merge incompatible provider authentication systems.
- Promise universal conversation migration.
- Force-switch accounts during active agent operations.
- Modify unsupported credential formats without safeguards.
- Guarantee that every running AI process adopts a new account immediately.

The application is an account manager, not a replacement authentication provider.

---

# 27. Open-Source Strategy

The repository should be published under a permissive license, subject to compatibility with included dependencies and referenced code.

Recommended documentation:

- README.md
- CONTRIBUTING.md
- SECURITY.md
- CHANGELOG.md
- CODE_OF_CONDUCT.md
- Provider compatibility documentation
- Architecture documentation
- Developer setup instructions

## Contribution Areas

Contributors should be able to work independently on:

- Provider adapters.
- Platform integrations.
- UI improvements.
- Usage monitoring.
- Testing.
- Documentation.
- Accessibility.
- Localization.

The provider architecture should make integrations easy to maintain without affecting unrelated services.

---

# 28. Technical Risks and Constraints

## Risk 1 — Provider Authentication Changes

AI services may change credential storage, authentication flows, or configuration layouts.

**Mitigation:** Version-aware provider adapters, compatibility testing, and safe disabling of incompatible features.

## Risk 2 — Already-Running Processes

Existing terminals and AI agents may retain their previous environment and authentication.

**Mitigation:** Distinguish selected defaults from active process state. Use controlled restarts when necessary.

## Risk 3 — Antigravity Integration

Antigravity may lack a public API for fully automatic third-party account switching.

**Mitigation:** Research supported interfaces, isolate experimental code, and require verified activation.

## Risk 4 — Token and Keychain Handling

Some credentials are bound to application-specific keychain items or authentication mechanisms.

**Mitigation:** Prefer native provider authentication; do not assume raw credential portability.

## Risk 5 — Platform Differences

Process handling, credential storage, and system trays differ across operating systems.

**Mitigation:** Platform-specific Rust modules and comprehensive native testing.

## Risk 6 — Usage Monitoring Reliability

Usage data may be unavailable, delayed, or estimated.

**Mitigation:** Display source, freshness, and confidence; avoid fabricated or stale readings.

## Risk 7 — Data Loss During Switching

Unexpected failures may damage or overwrite existing account state.

**Mitigation:** Recovery snapshots, journals, verification, rollback, and non-destructive default behavior.

---

# 29. Minimum Viable Product

The first public release should focus on a small set of reliable features.

### MVP Features

1. Cross-platform Tauri 2 application.
2. macOS menu bar.
3. Windows/Linux system tray.
4. Automatic startup.
5. Modern compact account selector.
6. Installed-provider detection.
7. Multiple saved accounts.
8. One selected default account per provider.
9. Claude Code profile management.
10. Codex profile management.
11. Protected local storage.
12. Account activation and verification.
13. Restart warnings.
14. Switching history.
15. Backup and recovery.
16. Full settings window.

Antigravity account switching should launch as an experimental or separately validated feature unless a reliable switching mechanism has been established.

Usage monitoring and advanced conversation management can follow after core switching stability.

---

# 30. Final Product Experience

The completed application should provide a simple and predictable experience.

### First-Time Usage

The user installs the application and opens it.

The application detects Claude Code, Codex, and Antigravity.

The user registers existing accounts and authenticates additional accounts.

All registered account information is displayed in one interface.

The user selects an active account for each provider.

The manager saves and protects the supported profiles.

### Daily Usage

The user opens an IDE or terminal and works with an AI coding assistant.

Later, the user wants to use another account.

Instead of manually signing out and back in:

1. The user clicks the menu bar or system tray icon.
2. The user expands the relevant provider.
3. The user selects a previously authenticated account.
4. The manager validates and activates it.
5. If necessary, it requests permission to restart the affected application.
6. The manager verifies which account is active.
7. The user continues working with the selected account.

For supported CLI configurations, new processes should use the activated account.

Already-running sessions must remain untouched unless the user explicitly approves an appropriate restart or reauthentication operation.

### Intended Outcome

One lightweight, universal, open-source application handles the complexity of managing multiple AI development accounts.

Users spend less time managing authentication and more time working with their tools.

The application should deliver the fastest safe account-switching experience each provider permits while remaining transparent about limitations.

**Core Promise:**

> Add your AI accounts once. Manage them from one place. Switch between them with minimal interruption.

---

# 31. Implementation Priorities

Development decisions should follow this order:

**Priority 1:** Reliability and credential security.

**Priority 2:** Correct account identity and switching verification.

**Priority 3:** Smooth menu bar and system tray experience.

**Priority 4:** Claude Code and Codex integration.

**Priority 5:** Antigravity and Gemini integration.

**Priority 6:** Usage monitoring.

**Priority 7:** Session and configuration management.

**Priority 8:** Additional providers and advanced automation.

Do not sacrifice account safety for the appearance of instant switching.

---

# 32. Technical Reference Material

The following documentation and projects should be reviewed during implementation.

### Official Documentation

- [Tauri 2](https://v2.tauri.app/)
- [Tauri System Tray](https://v2.tauri.app/learn/system-tray/)
- [Tauri Plugins](https://v2.tauri.app/plugin/)
- [Claude Code Settings](https://code.claude.com/docs/en/settings)
- [Codex Environment Variables](https://learn.chatgpt.com/docs/config-file/environment-variables)
- [Google Antigravity Authentication](https://antigravity.google/docs/install/)
- [Google Antigravity Getting Started](https://antigravity.google/docs/getting-started)

### Related Open-Source Projects

- [AccountDeck](https://github.com/nadim-sheikh/AccountDeck)
- [Antigravity Account Manager](https://github.com/Ron-Niri/AntigravityAccManager)

These repositories should be treated as research references rather than assumed drop-in implementations.

Review their licenses, provider compatibility, security practices, and account-switching methods before reusing code.

---

# 33. Final Development Directive

Build a polished, lightweight, cross-platform AI account management desktop application using Tauri 2 and Rust.

The application must operate as a single background desktop utility with native menu bar or system tray access.

It must support multiple saved accounts per provider, maintain one selected active default per provider, and provide safe, verified account switching with minimal user interaction.

Start with reliable Claude Code and Codex profile management, then implement Google Antigravity and Gemini integration using validated provider-compatible mechanisms.

Use a modular Rust backend, SQLite for application metadata, OS-protected credential storage, and a minimal React/TypeScript interface built with Tailwind CSS and shadcn/ui.

Prioritize reliable switching, strong privacy, rollback safety, native system behavior, and a clean user experience.

Avoid extra companion processes, mandatory browser extensions, IDE plugins, remote servers, and unnecessary complexity.

**The final product should feel like a native universal account switcher for AI coding tools—not another complex development platform.**