# AGENTS.md

Notes for AI coding assistants. Everything below was checked against the repository files.

## Overview

- Rust bindings to the Steamworks SDK: crate `steamworks` 0.12.2 plus the raw FFI crate `steamworks-sys` 0.12.0 (README: SDK 1.62, MSRV 1.80.0). License MIT / Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`), except the SDK files under `steamworks-sys/lib/steam/`.
- This repo, `ducky7go/steamworks-rs`, is a fork of upstream `Noxime/steamworks-rs` (the `repository` field in both `Cargo.toml` files; no `upstream` remote is configured in this checkout). The fork's visible change is `UpdateHandle::language` in `src/ugc.rs`, wrapping `SteamAPI_ISteamUGC_SetItemUpdateLanguage` (commit "SteamAPI_ISteamUGC_SetItemUpdateLanguage"). Keep upstream structure so changes can be rebased/contributed back.
- Main consumer: `ducky7go/steamworks.js` vendors this repo as its submodule `vendor/steamworks-rs` (features `serde`, `raw-bindings`). Its gitlink points at this repo's HEAD, so any change here has to be pushed before the submodule pointer is bumped there.

## Layout

- `src/` - safe API, one module per Steam interface (`app.rs`, `friends.rs`, `ugc.rs`, `user.rs`, `user_stats/`, `matchmaking.rs`, `networking_*.rs`, `remote_storage.rs`, `screenshots.rs`, `server.rs`, `utils.rs`, ...). Entry/`Client`/`Server` in `src/lib.rs`.
- `steamworks-sys/` - raw bindings: `src/{linux,macos,windows}_bindings.rs` (generated, committed), `build.rs`, `Makefile`, and `lib/steam/` (SDK headers in `public/steam/` and prebuilt `redistributable_bin/` binaries, both tracked in git).
- `examples/` - workspace members `chat-example`, `workshop`, `lobby`, `networking-messages`, `game-server` (plus `achievements` which only has a Readme).
- `steam_appid.txt` (contains `480`, Spacewar) for running examples; `rustfmt.toml` ignores `steamworks-sys`.
- `updating-steamworks-sdk-version.md` - maintainer notes for SDK upgrades.

## Build, test, run

- Workspace is defined in the root `Cargo.toml` (`steamworks-sys` plus the examples). Cargo features of `steamworks`: `default = []`, `raw-bindings` (re-exports `steamworks_sys` as `steamworks::sys`), `image`, `serde`. `steamworks-sys` has `rebuild-bindings` (needs `bindgen`).
- What CI runs (`.github/workflows/ci.yml`, on Windows/Ubuntu/macOS with stable, nightly and 1.80.0): `cargo fmt --all -- --check` (Linux), `cargo build --lib`, `cargo build --examples`. On Ubuntu it installs libudev/X11/GL/ALSA dev packages first. There is no test suite beyond `serial_test` in dev-dependencies; running examples needs a running Steam client.
- Regenerating bindings: workflow `.github/workflows/rebuild-bindings.yml` (`cargo build --features rebuild-bindings` in `steamworks-sys` with `STEAM_SDK_LOCATION=./lib/steam`), or the `Makefile` targets. Full SDK-upgrade procedure: `updating-steamworks-sdk-version.md`.
- `build.rs` links the dynamic Steam library from `$STEAM_SDK_LOCATION/redistributable_bin/...` (default `steamworks-sys/lib/steam`). At runtime the redistributable (`steam_api64.dll` / `libsteam_api.so` / `libsteam_api.dylib`) must be findable next to the executable (README, "Help, I can't run my game!").

## Conventions

- Format with `cargo fmt` (CI enforces `--check`); `steamworks-sys` is excluded from rustfmt.
- Safe wrappers follow the existing pattern: `unsafe` call into `sys::SteamAPI_*`, builder-style `#[must_use]` setters on handles such as `UpdateHandle`. Mirror the style of neighbouring functions when adding bindings.
- Generated `*_bindings.rs` files and SDK headers/binaries come from the SDK upgrade/`rebuild-bindings` flow; do not hand-edit the generated bindings.
- Keep `rust-version = "1.80.0"` compatible (CI builds on 1.80.0).
- No contribution or commit-message guidelines file exists in this repo. The checkout is shallow (one commit), so commit habits could not be verified.

## Gotchas

- `.gitignore` ignores `Cargo.lock` and `steamworks-sys/lib/steam/*` except `public/` and `redistributable_bin/`; a repository-wide `git add` will not pick up other SDK files.
- Third-party SDK binaries/headers are committed; check licensing (README says the SDK files are excluded from the dual license) before redistributing.
- The README's "Features" section lists only `serde` and `image`; `raw-bindings` exists in `Cargo.toml` but is undocumented there, and the README install snippet still shows `steamworks = "0.12.0"` (crates.io release, not this fork).
- `updating-steamworks-sdk-version.md` links to the upstream `Noxime/steamworks-rs` Actions page, not this fork's.
