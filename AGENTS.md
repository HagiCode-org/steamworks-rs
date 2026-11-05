# steamworks-rs

## Overview

- Rust bindings to the Steamworks SDK. Crate `steamworks` 0.12.2 wraps the raw FFI crate `steamworks-sys` 0.12.0. README: SDK 1.62, MSRV 1.80.0.
- License: MIT / Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`). The SDK files under `steamworks-sys/lib/steam/` are excluded from that license.
- This is the `ducky7go` fork of `Noxime/steamworks-rs` (the `repository` field in both `Cargo.toml` files points upstream; no `upstream` remote is configured here). The fork's visible change is `UpdateHandle::language` in `src/ugc.rs`, which wraps `SteamAPI_ISteamUGC_SetItemUpdateLanguage`. Keep upstream structure so changes can be contributed back.
- Main consumer: `ducky7go/steamworks.js`, which vendors this repo as `vendor/steamworks-rs` with features `serde` and `raw-bindings`. Push changes here before bumping that submodule pointer there.

## Layout

- `src/`: safe API, one module per Steam interface (`app.rs`, `friends.rs`, `ugc.rs`, `user.rs`, `user_stats/`, `matchmaking.rs`, `networking_*.rs`, `remote_storage.rs`, `screenshots.rs`, `server.rs`, `utils.rs`, ...). `Client` and `Server` entry points are in `src/lib.rs`.
- `steamworks-sys/`: raw bindings. `src/{linux,macos,windows}_bindings.rs` are generated and committed. Also `build.rs`, `Makefile`, and `lib/steam/` (SDK headers in `public/steam/`, prebuilt `redistributable_bin/`; both tracked in git).
- `examples/`: `chat-example`, `workshop`, `lobby`, `networking-messages`, `game-server` are workspace members. `achievements` contains only a Readme.
- `steam_appid.txt` contains `480` (Spacewar), used when running examples. `rustfmt.toml` excludes `steamworks-sys`.
- `updating-steamworks-sdk-version.md`: maintainer steps for SDK upgrades.

## Build, test, run

- Root `Cargo.toml` defines the workspace (`steamworks-sys` plus the examples).
- Features of `steamworks`: `raw-bindings` (re-exports `steamworks_sys` as `steamworks::sys`), `image`, `serde`. `steamworks-sys` has `rebuild-bindings`, which needs `bindgen`.
- CI (`.github/workflows/ci.yml`, Windows, Ubuntu and macOS, stable, nightly and 1.80.0): `cargo fmt --all -- --check` (Linux), `cargo build --lib`, then `cargo build --examples`. The examples step's `if:` is malformed, so do not rely on it as a gate. Ubuntu installs libudev, X11, GL and ALSA dev packages first.
- CI does not run `cargo test`. Unit tests with `#[test]` exist in several `src/` modules (for example `error.rs`, `networking_types.rs`, `matchmaking.rs`); run them locally. Examples need a running Steam client.
- Regenerate bindings with `.github/workflows/rebuild-bindings.yml` (`cargo build --features rebuild-bindings` in `steamworks-sys` with `STEAM_SDK_LOCATION=./lib/steam`) or the `Makefile` targets (`windows`, `linux`, `macos`, `local`). The full SDK upgrade is in `updating-steamworks-sdk-version.md`.
- `steamworks-sys/build.rs` links the Steam library from `$STEAM_SDK_LOCATION/redistributable_bin/<win64|linux64|osx>` (default `steamworks-sys/lib/steam`). At runtime the redistributable (`steam_api64.dll`, `libsteam_api.so`, `libsteam_api.dylib`) must be next to the executable.

## Conventions

- Format with `cargo fmt`. CI enforces `--check`.
- Safe wrappers call `sys::SteamAPI_*` inside `unsafe` and expose builder-style `#[must_use]` setters on handles such as `UpdateHandle`. Copy the style of neighbouring functions.
- Do not hand-edit `*_bindings.rs`, SDK headers or SDK binaries. Regenerate them through the SDK upgrade or `rebuild-bindings` flow.
- Keep `rust-version = "1.80.0"` compatible; CI builds on 1.80.0.
- The repo has no contribution or commit-message guide.

## Gotchas

- `.gitignore` ignores `Cargo.lock` and `steamworks-sys/lib/steam/*`, except `public/` and `redistributable_bin/`. A repository-wide `git add` will not pick up other SDK files.
- The SDK headers and binaries are committed. Check their license terms before redistributing.
- The README's feature list names only `serde` and `image`. `raw-bindings` is in `Cargo.toml` but not documented there. The README install snippet shows `steamworks = "0.12.0"`, the crates.io release rather than this fork.
- `updating-steamworks-sdk-version.md` links to the upstream `Noxime/steamworks-rs` Actions page, not this fork's.
