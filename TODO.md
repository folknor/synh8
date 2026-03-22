# synh8 TODO

## Bugs / Limitations

- [ ] Virtual package dependency resolution - cascade unmark doesn't work for packages
  with virtual package dependencies (e.g., nvidia packages). The dependency check only
  matches direct package names, not virtual package providers.

  Example: `libnvidia-extra-590` depends on `nvidia-kernel-common-590-590.48.01` (virtual),
  which is provided by `nvidia-kernel-common-590`. When trying to unmark
  `nvidia-kernel-common-590`, the cascade fails because the names don't match.

  Workaround: Users can unmark the original user-marked package, which correctly
  clears all associated dependencies.

## Performance

- [ ] Partial list updates - only update changed entries instead of full rebuild
- [ ] Changelog fetched synchronously - `apt changelog` is run as a blocking subprocess.
  The UI freezes for several seconds on slow connections. The "Loading changelog..."
  message is set but never rendered because the draw loop blocks. (`core.rs:563-583`,
  `app.rs:526`)

## UI/UX

- [ ] Scrollbar position indicator in modals - show "line X of Y" or visual marker
- [ ] Theming - load colors from config file
- [ ] Navigation keys ignore focused pane (DEFERRED) - PageUp/PageDown/Home/End/g/G
  always move the package list even when the filter or details pane is focused. Up/Down
  correctly dispatch by pane, but bulk navigation keys don't. (`main.rs:84-107`)
- [ ] Left/Right/d/l always change details tab regardless of focus (DEFERRED) - pressing
  `l` in the package list or `h` in the filter pane changes the details tab instead of
  doing something contextual to the focused pane. (`main.rs:116-119`)

## Features

- [ ] Package removal - `-` key marks for removal, shows red `-` in status column
- [ ] Package pinning - `=` key holds package at current version, prevents upgrades
- [ ] Repository filter - filter by origin (main, universe, PPAs)
- [ ] Help screen - `?` or `F1` shows keybindings grouped by context
- [ ] Confirm mark-all - prompt before `x` marks hundreds of packages
- [ ] Persist settings - save column visibility and sort order to ~/.config/synh8/config.toml
- [ ] Package history - show install/upgrade dates from /var/log/apt/history.log
- [ ] Custom filters - user-defined filters (e.g., "packages > 100MB")
- [ ] Fix broken packages - `B` attempts to resolve broken dependencies
- [ ] Version selection - picker when multiple candidates exist (different repos/pins)
- [ ] Debconf integration - currently `DEBIAN_FRONTEND=noninteractive` suppresses all
  debconf prompts (e.g., "really remove running kernel?"). A proper integration would
  write a custom debconf frontend that forwards the debconf protocol to our process
  over a unix socket/named pipe, parses the ~12 protocol commands, and presents
  questions as TUI modals. The debconf protocol is simple text-based (`INPUT`,
  `GO`, `GET`, `SET`, etc.) — the hard part is the plumbing between a subprocess
  spawned deep inside dpkg and our main event loop.
- [ ] Conffile prompt handling - currently `--force-confdef --force-confold` keeps
  existing config files without prompting. rust-apt's `DynInstallProgress` doesn't
  expose dpkg's `conffile` status-fd message. If rust-apt adds a `conffile()` callback,
  we could present a TUI modal asking keep/replace/diff.

## Error Messages

- [ ] Interrupted operation - detect and show "Resuming interrupted dpkg operation"

## Polish

- [ ] Remember scroll position - preserve position when switching filter categories
- [ ] Prominent search indicator - highlight active search query in status bar

## Documentation

- [ ] CLI arguments - `--help`, `--version`, `--dry-run`
- [ ] README with screenshots and feature list
