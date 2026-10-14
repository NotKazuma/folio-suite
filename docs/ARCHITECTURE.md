# Architecture

Wren is a suite of free office and creative apps in pure Rust. **Each app has its own repository**; this
repository is only the launcher that lists them and installs them.

```text
crates/
  core       wren-core     pure, no I/O: the app catalog (21 apps in 4 sections), version ordering,
                           release parsing, choosing the right download per platform, checksum files,
                           app status
  native     wren-native   desktop only: GitHub lookups (with ETag caching), verified downloads, the
                           installed-apps record, handing an installer to the OS; runs on background
                           threads and reports back as events
  ui-egui    wren-ui       the launcher window in egui: model, theme, views
apps/
  wren-launcher            the desktop shell (eframe) around wren-ui
xtask/                     project automation (placeholder)
scripts/check.ps1          what CI runs: rustfmt, clippy -D warnings, tests
```

Layering: `core` depends on nothing else here; `native` uses `core`; `ui-egui` uses both. Nothing below the app
opens windows.

## Rules

- **Never crash.** Non-test code has no `unwrap`, `expect`, `panic!` or `todo!` (clippy denies them) and no
  `unsafe` (the workspace forbids it). A bad response, a half-written file or a corrupt settings file becomes
  an error the user can read.
- **Network data is hostile.** Sizes are capped, nothing is indexed blindly, unknown fields are ignored.
- **Install safely.** Download only from `github.com/NotKazuma/<app id>` and GitHub's file CDN (checked again
  after redirects). Verify against the release's `SHA256SUMS.txt`. Never delete files the launcher did not
  create; never elevate silently.
- **No Electron, no webview shells.**

## Where an app's releases live

Every app is its own repository (`App::repo()` in `crates/core/src/catalog.rs` is `NotKazuma/<id>`) and publishes
`<id>-<version>-windows-x64.exe` plus `SHA256SUMS.txt` with each tagged release. `pick_asset(assets, platform,
stem)` in `crates/core/src/platform.rs` selects an app's file by that id prefix (so `wren-pinion` never sees
`wren-pinionx`'s files). To move the suite to a GitHub organization, transfer the repos and change `ORG`.

## Adding an app

Add one `app(...)` line to `SUITE` in the catalog (id, name, tagline, what it replaces, colour, stage, category),
create the app's repository with a release workflow that publishes the file name above, and it appears in its
section. The catalog tests check ids are unique, prefixed `wren-`, match the display name, and sit in category
order.

## Releases

Tag `vX.Y.Z` to build and publish `wren-launcher-<version>-windows-x64.exe` plus its checksum file.
