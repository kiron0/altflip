<picture>
  <source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/banner-dark.png" />
  <img src="assets/screenshots/banner-light.png" alt="AltFlip: every Claude Code, Codex and Antigravity account in your menu bar, with the menu panel and dashboard" />
</picture>

<p align="center">
  <a href="https://github.com/kiron0/altflip/releases/latest">
    <img src="https://img.shields.io/github/v/release/kiron0/altflip?label=Download%20AltFlip&style=for-the-badge&color=5A63FF&labelColor=1a1a1a" alt="Download AltFlip" height="38" />
  </a>
</p>

<p align="center">
  <a href="https://github.com/kiron0/altflip/actions/workflows/ci.yml"><img src="https://github.com/kiron0/altflip/actions/workflows/ci.yml/badge.svg" alt="CI Status" /></a>
  <img src="https://img.shields.io/badge/macOS%20%C2%B7%20Windows%20%C2%B7%20Linux-black" alt="macOS, Windows, Linux" />
  <img src="https://img.shields.io/badge/Tauri-2-24C8DB?logo=tauri" alt="Tauri 2" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License" /></a>
</p>

<p align="center">
  <b>Switch Claude Code and Codex accounts in one click · See every usage limit live · Antigravity usage included</b>
  <br />
  <sub>Free and open source. Everything stays on your computer.</sub>
</p>

---

## A look inside

<p align="center">
  <picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/menu-dark.png" /><img src="assets/screenshots/menu-light.png" alt="AltFlip menu bar panel with accounts, usage pills and reset timers" width="340" /></picture>
  <br />
  <sub>The menu bar panel: every account, its limits and when they reset.</sub>
</p>

<table>
  <tr>
    <td width="50%"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/dashboard-dark.png" /><img src="assets/screenshots/dashboard-light.png" alt="AltFlip overview with Claude Code, Codex and Antigravity cards" /></picture></td>
    <td width="50%"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/accounts-dark.png" /><img src="assets/screenshots/accounts-light.png" alt="Accounts page listing saved accounts by tool with Switch buttons" /></picture></td>
  </tr>
  <tr>
    <td><b>Overview.</b> Every tool, the account it uses now, and recent switches.</td>
    <td><b>Accounts.</b> Every saved account, grouped by tool. Switch in one click.</td>
  </tr>
  <tr>
    <td width="50%"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/usage-dark.png" /><img src="assets/screenshots/usage-light.png" alt="Usage page with 5-hour and weekly limit bars for each account" /></picture></td>
    <td width="50%"><picture><source media="(prefers-color-scheme: dark)" srcset="assets/screenshots/activity-dark.png" /><img src="assets/screenshots/activity-light.png" alt="Activity page with switch history, timings and filters" /></picture></td>
  </tr>
  <tr>
    <td><b>Usage.</b> 5-hour and weekly limits for every account, with reset times.</td>
    <td><b>Activity.</b> Every switch and account change, with how long it took and undo.</td>
  </tr>
</table>

<sub>Screenshots use made-up demo accounts.</sub>

## Features

- Save each account once through the tool's own sign-in. AltFlip never asks for passwords.
- Switch Claude Code and Codex in one click from the menu bar. One active account per tool.
- Every switch is verified; if anything fails, the previous sign-in is restored.
- Live 5-hour and weekly usage for every saved account, with reset countdowns and limit alerts.
- Antigravity: live per-model usage for every saved account (usage only, see the in-app **Usage only** button for why).
- Activity log of every switch and account event, with undo.
- Encrypted, passphrase-protected backups.
- Local only: no AltFlip account, no server, no telemetry.

## How it works

Architecture, switching, providers, security and release details live in [HOW-IT-WORKS.md](HOW-IT-WORKS.md).

## Install

Download the installer for your platform from [Releases](https://github.com/kiron0/altflip/releases/latest): `.dmg` for macOS, `.exe` or `.msi` for Windows, `.AppImage` or `.deb` for Linux.

## License

MIT © [Toufiq Hasan Kiron](https://github.com/kiron0)

The app and menu bar icons are based on the [AccountDeck](https://github.com/nadim-sheikh/AccountDeck) mascot, MIT © Nadim Sheikh ([license](assets/accountdeck/LICENSE)).
