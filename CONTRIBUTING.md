# Contributing

Thanks for helping. Provider adapters, platform support, tests, accessibility and translations are all welcome.

## Setup

```sh
pnpm install
pnpm dev
```

Before opening a pull request:

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm --filter @altflip/desktop build
```

## Ground rules

- Never test switching against your primary accounts. Use `ALTFLIP_DATA_DIR` and throwaway accounts.
- Never log, print or display tokens. `Resource`'s `Debug` output hides data; keep it that way.
- An adapter must not report success until it has verified the active identity.
- New capabilities must reflect tested behavior. Mark untested providers or versions as experimental or untested.

## Adding a provider

1. Add `crates/providers/src/<name>.rs` implementing `Provider`.
2. List every place that holds the sign-in in `locators()`. Mark the ones required to switch as `Role::Primary`.
3. Implement `identify` so different accounts get different keys and the same account always gets the same key.
4. Override `verify` if the tool has a status command.
5. Add the provider to `ProviderId` and `registry()`, and to the UI's `ProviderIcon`.
6. Add tests using `testutil::ctx()`, which uses an in-memory credential store and a temporary home folder.
7. Document it in `HOW-IT-WORKS.md`.
