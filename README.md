<div align="center">

<img src="assets/icon.png" alt="LitePad" width="128" />

# LitePad

**A bloat-free, lightning-fast, zero-telemetry notepad for Windows.**

Native Rust (egui/eframe) — no Electron, no web view, no background services.
A single ~4 MB `.exe`.

<p>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows-0078D6" />
  <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust-DEA584?logo=rust&logoColor=white" />
  <img alt="Binary size" src="https://img.shields.io/badge/binary-~4%20MB-2ea44f" />
  <img alt="Telemetry" src="https://img.shields.io/badge/telemetry-zero-2ea44f" />
</p>

</div>

---

## Screenshots

| Light | Dark | Brown |
| :---: | :---: | :---: |
| ![LitePad — Light theme](website/preview-light.png) | ![LitePad — Dark theme](website/preview-dark.png) | ![LitePad — Brown theme](website/preview-brown.png) |

## Download

Grab the latest `litepad.exe` from the [**Releases**](https://github.com/yashpandey0031/litepad/releases/latest)
page — a single portable executable. No installer, no runtime, no dependencies, no admin rights.
Double-click and you're writing.

> The first launch of an unsigned app may trigger Windows SmartScreen —
> choose **More info → Run anyway**.

## Features

- **Autosave** — a live indicator shows a spinner + "Saving…" as you type and settles to
  "✓ Saved"; writes to disk 0.7 s after you stop, on note switch, and on close.
- **Sidebar** — all notes listed macOS-style (title, preview, relative time, file path), newest
  first. Its font is fixed and never changes when you resize the editor text.
- **Search** — filter notes instantly (`Ctrl+F`).
- **Save As…** — export the current note anywhere via a native Windows dialog (`Ctrl+Shift+S`).
- **Themes** — three built-in: **Light**, **Dark**, and a warm **Brown**. Solid colors,
  rounded corners, no gradients. The toolbar button cycles Light → Dark → Brown.
- **Fonts** — pick from 4 real Windows fonts (Segoe UI, Arial, Georgia, Consolas), adjustable
  size (`Ctrl +` / `Ctrl -`), with **Bold / Italic / Underline** toggles applied to the editor.
- **Clickable links** — `http(s)` URLs are highlighted; `Ctrl+Click` opens them in your browser.
- **Standard editing** — copy / cut / paste / select-all / undo / redo (native `Ctrl+C/X/V/A/Z`).
- **Shortcuts panel** — click **Shortcuts** in the toolbar for the full list.
- **Plain-text files** — reads & writes `.txt`, `.md`, `.markdown`, `.log`, `.csv`, `.conf`.
- **No networking** — there is no network code in the project *at all*. It never phones home.

## Where notes live

`%APPDATA%\LitePad\notes\` — one real text file per note, named after its first line.
Drop your own `.txt`/`.md` files in there and they show up in the sidebar. Preferences are
stored in `%APPDATA%\LitePad\config.txt`. The **Folder** button opens this in Explorer.
(Notes from an earlier `RustPad` install are migrated automatically on first launch.)

## Shortcuts

| Action                    | Keys                             |
| ------------------------- | -------------------------------- |
| New note                  | `Ctrl+N`                         |
| Save now                  | `Ctrl+S`                         |
| Save As… (export)         | `Ctrl+Shift+S`                   |
| Search                    | `Ctrl+F`                         |
| Delete note               | `Ctrl+D` (or right-click a card) |
| Bold / Italic / Underline | `Ctrl+B` / `Ctrl+I` / `Ctrl+U`   |
| Bigger / smaller text     | `Ctrl+=` / `Ctrl+-`              |
| Select all                | `Ctrl+A`                         |
| Cut / Copy / Paste        | `Ctrl+X` / `Ctrl+C` / `Ctrl+V`   |
| Undo / Redo               | `Ctrl+Z` / `Ctrl+Y`              |
| Open link                 | `Ctrl+Click`                     |

## Build from source

Requires a [Rust toolchain](https://rustup.rs/) (MSVC target on Windows).

```powershell
cargo run --release
```

The optimized binary lands at `target\release\litepad.exe` — copy it anywhere and double-click.

## Why LitePad?

Most "lightweight" notepads today ship an entire browser engine. LitePad is a native
immediate-mode GUI app in pure Rust: one small binary, instant startup, zero telemetry,
and your notes stay as plain-text files you fully own.

---

<div align="center">
Made with Rust 🦀 by <a href="https://github.com/yashpandey0031">Yash Pandey</a>
</div>
