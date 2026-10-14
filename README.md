# Folio

Free office and creative tools, written in pure Rust, and the launcher that installs and updates them.

| App | An alternative to | Stage |
|---|---|---|
| Folio Write | Word | early alpha, native (`apps/folio-write`, 3 MB) |
| Folio Sheet | Excel | early alpha, native (`apps/folio-sheet`) |
| Folio Slide | PowerPoint | early alpha, native (`apps/folio-slide`) |
| Folio Photo, Vector, PDF, Video, Audio | Photoshop, Illustrator, Acrobat, Premiere Pro, Audition | planned |

## Folio Launcher

A native window (about 4 MB, egui) listing every app, showing installed and latest versions, and
installing the right build for your computer after checking its SHA-256.

```sh
cargo run --release -p folio-launcher
cargo test --workspace
```

Needs Rust 1.90+ (on Windows, the Visual Studio C++ Build Tools). See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Download

Grab the latest build from [Releases](https://github.com/NotKazuma/folio-suite/releases): each app is a single
`.exe` (Windows, x64) named `folio-<app>-<version>-windows-x64.exe`, with a `SHA256SUMS.txt` to check them
against. Folio Launcher lists and installs the rest for you.

## Folio Write

A native word processor: headings, paragraphs, quotes, bulleted and numbered lists, **bold** and *italic*
(Ctrl+B / Ctrl+I), Enter to split and Backspace to join blocks, undo and redo, find and replace, and open and
save of Markdown or Word (`.docx`) files.

```sh
cargo run --release -p folio-write
```

## Folio Sheet

A native spreadsheet with its own formula engine: references and ranges, 20 functions (`SUM`, `AVERAGE`, `IF`,
`ROUND`, `CONCAT` and more), clear error values (`#DIV/0!`, `#CYCLE!`), undo and redo, copy and paste, and CSV
open and save. Only the visible cells are drawn, so large sheets stay quick.

## Folio Slide

A native presentation editor: title, content and blank layouts, four themes, text, rectangles and ellipses you
can drag and resize, speaker notes, full-screen presenting (F5), outline export, and open and save as JSON.

## Prototypes

`prototypes/web/` holds the first Write, Sheet and Slide as dependency-free browser apps
(`npm test` there runs their tests). They are the first sketches of the three apps above, kept for reference.

All code here is original. License: MIT OR Apache-2.0.
