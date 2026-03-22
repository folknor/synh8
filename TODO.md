# synh8 TODO

## Bugs

- [ ] Virtual package dependency resolution — cascade unmark doesn't work for packages with
  virtual package dependencies (e.g., nvidia packages). The dependency check only matches
  direct package names, not virtual package providers.
- [ ] Double-fault in commit leaves permanent Transitioning state — if commit fails AND
  `ManagerState::new()` also fails, every subsequent access panics.
- [ ] `AptCache::refresh()` keeps stale PackageId mappings — new packages after apt update
  won't have IDs, removed packages leave orphaned mappings.
- [ ] `commit_with_progress` in AptCache creates throwaway cache on error — `self.cache` and
  `fullname_to_id`/`id_to_fullname` maps can go out of sync if commit fails.
- [ ] Changelog fetches use fullname with arch suffix — `apt changelog vim:amd64` behavior
  is inconsistent across Debian/Ubuntu versions. Should use display name.

## Error Handling

- [ ] `StdioRedirect::Drop` uses `debug_assert!` for dup2 — release builds silently corrupt
  terminal if dup2 fails restoring stdout/stderr.
- [ ] `expect()` in `Dirty::reset()` and `plan()` panics if `clear_all_marks()` fails —
  should surface error to user instead of crashing.
- [ ] Interrupted operation — detect and show "Resuming interrupted dpkg operation"

## Performance

- [ ] Filter cache clone copies 81k PackageInfo with 7 Strings each — every cache-hit
  rebuild clones the entire list. Consider `Arc` or lazy overlay.
- [ ] Startup takes ~2s due to pre-warming all 5 filter caches
- [ ] Changelog fetched synchronously — UI freezes on slow connections
- [ ] Search results stored as `HashSet<String>` instead of `HashSet<PackageId>`
- [ ] `bulk_unmark` still scans full list twice for HashSets — should use
  `planned_changes()` diff like `toggle_unmark` does.
- [ ] `restore_selection` linear-scans by String — could compare PackageId (u32) instead.
- [ ] `package_depends_on` BFS uses String sets — could use PackageId sets.
- [ ] `download_size_str()` allocates String per visible row per frame — could precompute.
- [ ] `display_name()` does suffix scan per row per frame — could store offset in PackageInfo.
- [ ] `toggle_mark_impl` calls `rebuild_list()`, then `toggle_current` also calls
  `refresh_ui_state()` which rebuilds again — redundant rebuild mitigated by cache.

## Refactoring

- [ ] `bulk_mark`/`bulk_unmark` duplicate `toggle_current` pattern (`app.rs:507-670`)
- [ ] `commit_with_progress` error-recovery structure duplicated between ManagerState and
  PackageManager levels
- [ ] MarkPreview construction split between app.rs and core.rs — should all live in core
- [ ] `ManagerState::set_sort()` inlines sort logic instead of delegating to
  `PackageManager::sort_list()` — duplication risk if sort logic changes.
- [ ] `mark_preview_scroll` uses `skip()` instead of `Paragraph::scroll()` — inconsistent
  with other modals, doesn't account for line wrapping.

## UI/UX

- [ ] Scrollbar position indicator in modals
- [ ] Theming — load colors from config file
- [ ] Scroll max calculations use magic numbers instead of actual viewport size —
  changelog/output scroll past end of content because max is `len - 1` not `len - viewport`.

## Features

- [ ] Keybinding registry — centralized action→key mapping with TOML config and defaults.
  Would eliminate manually maintained help bar text, modal labels, and inline hints.
  All display strings derived from the registry. Enables user customization.
- [ ] Package removal — `-` key marks for removal
- [ ] Package pinning — `=` key holds package at current version
- [ ] Repository filter — filter by origin (main, universe, PPAs)
- [ ] Help screen — `?` shows keybindings grouped by context
- [ ] Confirm mark-all — prompt before `x` marks hundreds of packages
- [ ] Persist settings — save to ~/.config/synh8/config.toml
- [ ] Package history — show install/upgrade dates from /var/log/apt/history.log
- [ ] Custom filters — user-defined filters (e.g., "packages > 100MB")
- [ ] Fix broken packages — `B` attempts to resolve broken dependencies
- [ ] Version selection — picker when multiple candidates exist
- [ ] Debconf integration
- [ ] Conffile prompt handling

## Documentation

- [ ] CLI arguments — `--help`, `--version`, `--dry-run`
