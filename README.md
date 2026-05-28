# Atrus — TUI File Manager

A terminal-based file manager written in Rust, built with [Ratatui](https://github.com/ratatui-org/ratatui) and [Crossterm](https://github.com/crossterm-rs/crossterm). Atrus lets you browse, organize, rename, and clean up files entirely from the keyboard, with a dry-run preview before any destructive operation touches disk.

---

## Features

- **Directory browsing** with scrollable file list and live metadata panel
- **Regex file filtering** — search files by name using regular expressions, with automatic fallback to plain text matching on invalid regex
- **Auto-sort** — move files into categorized subdirectories based on extension (Images, Documents, Code, etc.)
- **Batch rename** — apply a prefix and transform filenames to lowercase, uppercase, or camelCase
- **Cleanup** — detect and delete empty directories
- **Duplicate finder** — SHA-256 hash scan to identify duplicate files across the directory tree
- **Large file scanner** — surface the top 20 heaviest files recursively
- **Dry-run preview** — every bulk operation shows a full scrollable change manifest before committing
- **Undo** — a per-session undo stack lets you reverse the last applied operation
- **Configurable categories** — edit sort targets, file extensions, and category names through an in-app settings screen; config persists to `~/.atrus-config.json`

---

## Installation

**Prerequisites:** Rust toolchain (1.70+). Install via [rustup](https://rustup.rs).

```bash
git clone https://github.com/Ecuacion-Seth/Atrus.git
cd Atrus
cargo build --release
./target/release/atrus
```

The binary can be copied anywhere on your `$PATH`:

```bash
cp target/release/atrus ~/.local/bin/
```

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `ratatui` | 0.24 | TUI rendering |
| `crossterm` | 0.27 | Terminal backend / input |
| `anyhow` | 1.0 | Error handling |
| `serde` / `serde_json` | 1.0 | Config serialization |
| `chrono` | 0.4 | File modification timestamps |
| `walkdir` | 2.4 | Recursive directory traversal |
| `sha2` | 0.10 | SHA-256 hashing for duplicate detection |
| `regex` | 1.10 | File name filtering |

---

## Configuration

On first run, Atrus writes a default config to `~/.atrus-config.json` (Linux/macOS) or `%USERPROFILE%\.atrus-config.json` (Windows).

```json
{
  "extension_map": {
    "jpg": "Images",
    "rs": "Code",
    "pdf": "Documents"
  },
  "sort_targets": {
    "Images":    "Images",
    "Documents": "Documents",
    "Code":      "Code"
  }
}
```

Sort targets are relative paths by default, resolved against the current working directory when a sort operation runs. You can set them to absolute paths for a fixed destination (e.g. `/home/user/Organized/Images`).

All config fields are editable from within the app via the Settings screen — no manual JSON editing required.

---

## Keybindings

### Explorer (default mode)

| Key | Action |
|---|---|
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `l` / `→` / `Enter` | Enter directory |
| `h` / `←` / `Backspace` | Go to parent directory |
| `f` | Open filter input |
| `u` | Open utility menu |
| `s` | Open settings |
| `z` | Undo last operation |
| `q` / `Esc` | Quit |

### Utility Menu (`u`)

| Key | Tool |
|---|---|
| `1` | Sort files by extension |
| `2` | Batch rename |
| `3` | Clean empty directories |
| `4` | Find duplicate files |
| `5` | Find large files |
| `Esc` / `q` | Close menu |

### Dry-Run Preview

| Key | Action |
|---|---|
| `y` | Confirm and apply all changes |
| `n` / `Esc` | Cancel — nothing is written |
| `j` / `k` | Scroll the change list |

### Settings (`s`)

| Key | Action |
|---|---|
| `j` / `k` | Navigate categories |
| `e` / `Enter` | Edit sort target path |
| `x` | Edit file extensions for category |
| `n` | Add new category |
| `d` / `Delete` | Delete selected category |
| `s` | Save config to disk |
| `Esc` / `q` | Return to explorer |

### Filter / Input Modal

| Key | Action |
|---|---|
| `Enter` | Confirm input |
| `Esc` | Cancel |
| `Tab` | (Rename mode only) Cycle case: lowercase → UPPERCASE → camelCase |

---

## Architecture

```
main.rs       Entry point — terminal setup/teardown, main render loop
app.rs        App state struct and all state-mutation methods
events.rs     Keyboard event dispatch by mode
ui.rs         Ratatui rendering (header, file list, panels, modals)
engine.rs     Pure filesystem logic (read, sort, rename, hash, apply)
models.rs     Shared data types (FileItem, AppMode, PendingChange, etc.)
config.rs     AppConfig with serde load/save and default extension map
```

Filesystem side-effects are isolated in `engine.rs`. The `App` struct in `app.rs` owns all mutable state; `events.rs` calls methods on it, and `ui.rs` reads from it immutably (except for `adjust_viewport`, which requires `&mut` for scroll correction).

---

## Known Issues / Roadmap

- Mouse support is enabled at the terminal level but no mouse event handling is implemented yet.
- The `scroll_offset` heuristic on parent navigation (`saturating_sub(5)`) is corrected on the next render frame by `adjust_viewport`; visible only as a brief flicker on very long directory listings.

---

## License

MIT
