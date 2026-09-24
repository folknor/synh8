# synh8

A Synaptic-inspired TUI for managing APT packages on Debian/Ubuntu systems, built with Rust and ratatui.

**Linux only.** Requires `libapt-pkg-dev` and root privileges.

Built with LLMs. See [LLM.md](https://github.com/folknor/synh8/blob/master/LLM.md).

## Features

- Browse, search, install, upgrade, remove and hold packages from the terminal
- Full-text search (FTS5) across package names and descriptions
- `j`/`k` navigation and vim-style visual mode for batch-marking packages
- Automatic dependency resolution with preview before committing
- Live progress display for downloads and installs
- Run `apt update` with real-time download progress
- Configurable columns and sort order
- Changelog viewer

## APT locking

Unlike apt, aptitude, and synaptic, synh8 does not hold the dpkg/APT lock
while running. It only takes the lock while `apt update` runs and while
changes are applied, and reports it if another tool holds it then. It does
not prevent other tools from modifying package state while the UI is open:
if you run `apt install` in another terminal while synh8 is open, synh8
won't notice until the next update or apply. You're root. You know what
you're doing.

## Usage

Must be run as root:

```bash
sudo synh8
```

## Keybindings

Arrow keys (or `j`/`k`), PgUp/PgDn, Home/End act on the focused pane.
`Tab`/`Shift+Tab` cycles focus between Filters, Packages, and Details.
In the Packages pane, `Space` toggles a mark, `-` marks for removal, `=`
holds, `v` starts a visual selection and `a` reviews and applies. The help
bar at the bottom shows the keys for the current screen.

The full reference is [docs/keybindings.md](docs/keybindings.md).

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
