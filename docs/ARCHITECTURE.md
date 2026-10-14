# Architecture

Folio is a suite of free office and creative apps, written in pure Rust, plus a launcher that
installs and updates them. One repository per app (like `folio-write`), and this repository for
the launcher and shared pieces.

```text
crates/
  core       folio-core     pure, no I/O: the suite catalog, version ordering, release parsing,
                            choosing the right download per platform, checksum files, app status
  native     folio-native   desktop only: GitHub lookups, verified downloads, the installed record,
                            handing an installer to the OS. Work runs on background threads and
                            reports back as events
  ui-egui    folio-ui       the launcher window in egui (model, theme, views)
  write-doc  folio-write-doc Folio Write's document model: blocks, inline bold/italic spans,
                            Markdown in and out, undo history, find and replace. No windows, no files
  docx       folio-docx     reads and writes Word .docx (size-capped; foreign files are hostile)
  sheet-engine folio-sheet-engine  cell addresses, formula parser, lazy evaluator, CSV
  slide-doc  folio-slide-doc the deck model: slides, items, themes, safe JSON loading
apps/
  folio-launcher            the desktop shell (eframe) around folio-ui
  folio-write               the word processor window (eframe) around folio-write-doc
  folio-sheet               the spreadsheet window around folio-sheet-engine
  folio-slide               the presentation editor around folio-slide-doc
xtask/                      project automation (placeholder)
prototypes/web/             the first Write / Sheet / Slide, as small browser apps, until each gets
                            its own Rust repository
```

Layering: `core` depends on nothing else here. `native` uses `core`. `ui-egui` uses both.
Nothing below the apps opens windows.

## Rules

- **Never crash.** Non-test code has no `unwrap`, `expect`, `panic!`, `todo!`; clippy denies them.
  No `unsafe` (the workspace forbids it). A bad response, a half-written file or a corrupt
  settings file becomes an error the user can read.
- **Network data is hostile.** Sizes are capped, nothing is indexed, unknown fields are ignored.
- **Install safely.** Download only from `github.com/folio-suite/<app>` and GitHub's file CDN
  (checked again after redirects). Verify against the release's `SHA256SUMS.txt`. Never replace a
  running app, never delete files the launcher did not create, never elevate silently.
- **No Electron, no webview shells.**

## Where releases live

Until each app has its own repository, every app publishes to this repository's releases, with files named
`<app id>-<version>-windows-x64.exe`. The launcher finds an app's file by that prefix
(`folio_core::platform::pick_asset`). To split an app out later, give it its own repository and change
`App::repo()` in `crates/core/src/catalog.rs`.

## Checks

`scripts/check.ps1` runs what CI runs: rustfmt, clippy with warnings denied, and all tests.

## Releases

Tag `vX.Y.Z`; GitHub Actions tests, builds `folio-launcher.exe`, zips it with `SHA256SUMS.txt` and
publishes the release.
