# synh8 TODO

## Pending decisions

- [ ] `v` in visual mode currently marks the selection (and ends visual mode).
  Revisit whether `v` should instead only end the selection.
- [ ] Legend box: the cyan "Auto-upg" / "Auto-inst" entries describe colours the
  package list never draws (dependencies are drawn like user marks, only the bold
  name tells them apart). Either drop the entries or draw dependencies in cyan.

## Error Handling

- [ ] Interrupted operation - detect and show "Resuming interrupted dpkg operation"

## Performance

- [ ] Filter cache clone copies 81k PackageInfo with 7 Strings each - every cache-hit
  rebuild clones the entire list. Consider `Arc` or lazy overlay.
- [ ] Every intent edit re-plans (clear marks + resolve, ~150-250ms). Fine for single
  marks; batch edits already re-plan once.
- [ ] Changelog fetched synchronously - UI freezes on slow connections
- [ ] Search results stored as `HashSet<String>` instead of `HashSet<PackageId>`
- [ ] `download_size_str()` allocates String per visible row per frame - could precompute.
- [ ] `display_name()` does suffix scan per row per frame - could store offset in PackageInfo.

## UI/UX

- [ ] Scrollbar position indicator in modals
- [ ] Theming - load colors from config file
- [ ] Filter counts (from the whole cache) can differ from list lengths (packages
  without a candidate version are not listed).

## Features

- [ ] Configurable keybindings - load overrides from TOML into the keymap registry
- [ ] Help screen - `?` shows keybindings grouped by context (render from the registry)
- [ ] Persistent holds - `apt-mark hold` / pinning (the `=` hold lasts one session)
- [ ] Repository filter - filter by origin (main, universe, PPAs)
- [ ] Persist settings - save to ~/.config/synh8/config.toml
- [ ] Package history - show install/upgrade dates from /var/log/apt/history.log
- [ ] Custom filters - user-defined filters (e.g., "packages > 100MB")
- [ ] Fix broken packages - `B` attempts to resolve broken dependencies
- [ ] Version selection - picker when multiple candidates exist
- [ ] Debconf integration
- [ ] Conffile prompt handling

## Documentation

- [ ] CLI arguments - `--help`, `--version`, `--dry-run`
