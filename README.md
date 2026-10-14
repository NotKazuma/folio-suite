# Wren Launcher

One window for the whole Wren suite: lists every app, shows which you have and which version is out, and
installs the right build for your computer after checking its SHA-256. A native app (about 4 MB, pure Rust,
egui): no Electron, no web view.

Wren is a family of lightweight, original, free alternatives to the big office and creative tools. Each app has
its own name and its own repository, so each is tracked, versioned and released on its own.

| Group | App | An alternative to | Repository | Status |
|---|---|---|---|---|
| Office | **Pinion** (writing) | Word | [wren-pinion](https://github.com/NotKazuma/wren-pinion) | early alpha |
| Office | **Trellis** (spreadsheets) | Excel | [wren-trellis](https://github.com/NotKazuma/wren-trellis) | early alpha |
| Office | **Perch** (presentations) | PowerPoint | [wren-perch](https://github.com/NotKazuma/wren-perch) | early alpha |
| Office | Nest, Dove, Heron, Godwit, Jackdaw, Magpie | OneNote, Outlook, Visio, Project, Access, Acrobat | | planned |
| Creative | Osprey, Egret, Goldfinch, Lorikeet, Tanager | Photoshop, Illustrator, Fresco, Express, XD | | planned |
| Publishing and photo | Bower, Peregrine | InDesign, Lightroom | | planned |
| Video, audio, web, 3D | Starling, Warbler, Colibri, Weaver, Condor | Premiere, Audition, After Effects, Dreamweaver, Maya | | planned |

The list lives in [`crates/core/src/catalog.rs`](crates/core/src/catalog.rs). The names of apps that are not
built yet are provisional and get a trademark/conflict check before any code is written for them.

## How it finds an app

For each app the launcher asks GitHub for the latest release of `NotKazuma/<app id>` and looks for a file named
`<app id>-<version>-windows-x64.exe`, with a `SHA256SUMS.txt` next to it. It downloads only from that account
and GitHub's file servers, checks the hash, and then hands the file to Windows. An app with no release yet shows
"No release yet."

## Build and run

```sh
cargo run --release -p wren-launcher
powershell -File scripts/check.ps1     # rustfmt, clippy (warnings denied), all tests
```

Needs Rust 1.90+ (on Windows, the Visual Studio C++ Build Tools). Architecture notes:
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## License

MIT OR Apache-2.0, at your option. See `LICENSE-MIT` and `LICENSE-APACHE`.
