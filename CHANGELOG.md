# Changelog

## Unreleased

- Recreate `/var/cache/apt/archives/partial` before committing changes: rust-apt's
  fetcher (unlike apt-get) does not rebuild the archive directories if they've been
  deleted, so installs failed after wiping /var/cache/apt to reclaim space
- Bump rust-apt 0.10 -> 0.11.2: `commit()` now releases the APT lock on failure and
  returns an error instead of panicking when an install fails mid-transaction
- Better resolver error messages: errors are now separated from warnings (a stray APT
  warning can no longer displace the actual error as the headline), and empty
  messages are filtered correctly
- Package removal: `-` marks for removal, `=` holds a package at its current state,
  `+` marks for install/upgrade; `-` also works on a visual selection. Each key
  undoes its own mark when pressed again
- `j`/`k` navigation in every list and scrollable view
- Resolver errors are shown (status bar, mark confirmation, changes review) and
  block applying, like apt-get. Marked packages are protected in the resolver, so
  it reports a conflict instead of silently dropping a mark
- Space on a marked package whose mark the resolver could not satisfy now unmarks
  it (it used to re-mark it forever)
- Cancelling a confirmation now restores exactly the marks from before the action;
  cancelling an unmark of a dependency used to re-mark every affected dependency as
  a manual install
- Search works with `-`, `.` and `+` in the query (`python3-foo` used to be an SQL
  error)
- `apt update` keeps your marks (carried across by package name); packages that are
  new after an update are now visible, and a failed update no longer leaves a
  stale plan behind
- After applying changes the cache is reloaded at once; a failed install can no
  longer leave the app unusable
- Disk space change counts upgrades as the size difference, not the full new size;
  removals no longer count toward the download size
- Downgrades are shown as downgrades (they were labelled upgrades)
- Sorting by version uses Debian version order (`10.0` after `9.0`)
- Sizes use apt's SI units (kB = 1000 bytes)
- PgUp/PgDn move by a full page; End in the details pane and all scrollable views
  jumps to the real bottom
- Esc during search input restores the search that was active before
- Ctrl-c quits from every screen
- The terminal is restored if synh8 panics or exits with an error
- apt/dpkg output is captured in memory instead of a fixed file in /tmp
- Download and install errors stay visible in the output view after applying
- Removed the lock-file pre-check (it used `flock`, which never sees dpkg's `fcntl`
  locks); apt's own locking during update and commit reports conflicts instead
- Keybindings are defined in one registry that drives key handling, the help bar and
  `docs/keybindings.md`
- Removed the `debug-cli` binary

## 0.1.1

- Revert termion backend switch; restore crossterm (termion had rendering glitches on alt-tab)

## 0.1.0

- Initial release
