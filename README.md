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

Arrow keys, PgUp/PgDn, Home/End act on the focused pane.
`Tab`/`Shift+Tab` cycles focus between Filters, Packages, and Details.

| Key | Context | Action |
|-----|---------|--------|
| `s` | Global | Search |
| `u` | Global | Run apt update |
| `F2` | Global | Settings (columns, sort) |
| `Esc` | Global | Clear search / cancel visual mode |
| `Space` | Packages | Toggle mark |
| `v` | Packages | Visual mode (multi-select) |
| `c` | Packages | View changelog |
| `a` | Packages | Apply pending changes |
| `x` | Packages | Mark all upgradable |
| `z` | Packages | Unmark all |
| `,`/`.` | Details | Switch tab (Info/Deps/RDeps) |
| `q` | Any | Quit |

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

## APT locking

Unlike apt, aptitude, and synaptic, synh8 does not hold the dpkg/APT lock
while running. It checks the lock on startup and before committing changes,
but does not prevent other tools from modifying package state while the UI
is open. If you run `apt install` in another terminal while synh8 is open,
synh8 won't notice — and that's fine. You're root. You know what you're doing.

## License

MIT
