# Folio

Free office and creative tools, written in pure Rust, and the launcher that installs and updates them.

| App | An alternative to | Stage |
|---|---|---|
| Folio Write | Word | early alpha, native (`apps/folio-write`, 3 MB) |
| Folio Sheet | Excel | early alpha (browser prototype) |
| Folio Slide | PowerPoint | early alpha (browser prototype) |
| Folio Photo, Vector, PDF, Video, Audio | Photoshop, Illustrator, Acrobat, Premiere Pro, Audition | planned |

## Folio Launcher

A native window (about 4 MB, egui) listing every app, showing installed and latest versions, and
installing the right build for your computer after checking its SHA-256.

```sh
cargo run --release -p folio-launcher
cargo test --workspace
```

Needs Rust 1.90+ (on Windows, the Visual Studio C++ Build Tools). See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Folio Write

A native word processor: headings, paragraphs, quotes, bulleted and numbered lists, **bold** and *italic*
(Ctrl+B / Ctrl+I), Enter to split and Backspace to join blocks, open and save Markdown.

```sh
cargo run --release -p folio-write
```

## Prototypes

`prototypes/web/` holds the first Write, Sheet and Slide as dependency-free browser apps
(`npm test` there runs their tests). Sheet and Slide will be rebuilt in Rust like Write.

All code here is original. License: MIT OR Apache-2.0.
