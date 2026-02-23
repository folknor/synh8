# synh8

A Synaptic-inspired TUI for managing APT packages on Debian/Ubuntu systems, built with Rust and ratatui.

## Features

- Browse, search, install, upgrade, and remove packages from the terminal
- Full-text search (FTS5) across package names and descriptions
- Vim-style navigation and visual mode for batch-marking packages
- Automatic dependency resolution with preview before committing
- Live progress display for downloads and installs
- Run `apt update` with real-time download progress
- Configurable columns and sort order
- Changelog viewer

## Layout

```
┌──────────────────────────────────────────────────────────────┐
│ Title Bar               │ N changes │ X.X MB download        │
├──────────┬──────────────────────────┬────────────────────────┤
│ Filters  │      Package Table       │     Details Pane       │
│          │                          │                        │
│ Upgr (N) │ S  Package    Candidate  │  [Info] [Deps] [RDeps]│
│ Marked(N)│ ↑  libfoo     1.2.3     │                        │
│ Inst (N) │ ·  libbar     4.5.6     │  Package: libfoo       │
│ Not  (N) │                          │  Status: ↑ Upgradable  │
│ All  (N) │                          │  Section: libs         │
├──────────┴──────────────────────────┴────────────────────────┤
│ Status bar                                                   │
├──────────────────────────────────────────────────────────────┤
│ /:Search  Space:Mark  v:Visual  u:Review  x:Upgrade all     │
└──────────────────────────────────────────────────────────────┘
```

## Keybindings

> **Note:** Keybindings are being reviewed and will change soon.

| Key | Action |
|-----|--------|
| `j`/`k`, arrows | Navigate |
| `g`/`G` | Jump to first/last |
| `PageUp`/`PageDown` | Jump 10 items |
| `Tab`/`Shift+Tab` | Cycle focus between panes |
| `Space` | Toggle mark on package |
| `v` | Visual mode (multi-select) |
| `/` | Search |
| `d` | Cycle details tab (Info/Deps/RDeps) |
| `c` | View changelog |
| `s` | Settings (columns, sort) |
| `u` | Review and apply changes |
| `x` | Mark all upgradable |
| `N` | Unmark all |
| `U` | Run apt update |
| `r` | Refresh cache |
| `q` | Quit |

## Building

Requires `libapt-pkg-dev`:

```bash
sudo apt install libapt-pkg-dev
cargo build --release
```

## Usage

Must be run as root:

```bash
sudo ./target/release/synh8
```

## License

MIT
