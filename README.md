# Folio

Free, original office and creative tools, plus **Folio Hub**, the launcher that installs and updates them.

## Run it

```sh
npm start        # http://127.0.0.1:4873
npm test
```

The Hub lists the suite and opens the browser apps directly:

| App | Alternative to | Status |
|---|---|---|
| Folio Write | Word | rich text, styles, lists, tables, links, find/replace; open/save HTML, Markdown, text; print to PDF |
| Folio Sheet | Excel | grid, formulas (own parser/engine, 20+ functions), sort, bold; open/save CSV |
| Folio Slide | PowerPoint | layouts, themes, text and shapes, drag/resize, reorder, present full screen; save JSON, export HTML |
| Photo, Vector, PDF, Video, Audio | Photoshop, Illustrator, Acrobat, Premiere Pro, Audition | planned |

Everything works offline and autosaves in the browser. Ctrl+K opens a command palette in each app; every
toolbar action is a named command in `apps/shared/cmd.js`.

## Layout

- `server.js`, `src/`, `public/`: Folio Hub (release lookup, asset picking, SHA-256 verified downloads)
- `apps/write|sheet|slide`: the browser apps, zero dependencies
- `catalog.json`: the suite list (`org` is the GitHub org that hosts native releases)

All code here was written from scratch; no third-party code or assets are included.
