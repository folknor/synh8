# synh8

A Synaptic-inspired TUI for managing APT packages on Debian/Ubuntu systems, built with Rust and ratatui.

**Linux only.** Requires `libapt-pkg-dev` and root privileges.

Built with LLMs. See [LLM.md](https://github.com/folknor/synh8/blob/master/LLM.md).

## Features

- Browse, search, install, upgrade, and remove packages from the terminal
- Full-text search (FTS5) across package names and descriptions
- Vim-style navigation and visual mode for batch-marking packages
- Automatic dependency resolution with preview before committing
- Live progress display for downloads and installs
- Run `apt update` with real-time download progress
- Configurable columns and sort order
- Changelog viewer

## APT locking

Unlike apt, aptitude, and synaptic, synh8 does not hold the dpkg/APT lock
while running. It checks the lock on startup and before committing changes,
but does not prevent other tools from modifying package state while the UI
is open. If you run `apt install` in another terminal while synh8 is open,
synh8 won't notice. You're root. You know what you're doing.

## Usage

Must be run as root:

```bash
sudo synh8
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

## Installation

```bash
sudo apt install libapt-pkg-dev
cargo install synh8
```

Or build from source:

```bash
sudo apt install libapt-pkg-dev
git clone https://github.com/folknor/synh8.git
cd synh8
cargo build --release
```

## License

MIT
