# synh8 TODO

## Bugs

- [x] Hardcoded `amd64` in dependency traversal — now uses `cache.native_arch()`
- [ ] Virtual package dependency resolution — cascade unmark doesn't work for packages with
  virtual package dependencies (e.g., nvidia packages). The dependency check only matches
  direct package names, not virtual package providers.
- [x] `cancel_mark` — now uses cache ID lookup instead of searching filtered list
- [x] `filter_count` for MarkedChanges undercounts — now uses `planned_changes().len()`
- [x] `PackageId(u32::MAX)` sentinel — `extract_package_info` now returns `None`
- [ ] Double-fault in commit leaves permanent Transitioning state — if commit fails AND
  `ManagerState::new()` also fails, every subsequent access panics.

## Error Handling

- [x] `dup`/`dup2` return values now checked with error propagation
- [x] FD leak on `StdioRedirect::capture()` error path fixed with OwnedFd guard
- [x] `clear_marked()` failure now propagated through `plan()` as error
- [ ] Interrupted operation — detect and show "Resuming interrupted dpkg operation"

## Performance

- [x] Windowed table rendering — sub-millisecond per frame
- [x] Per-filter memoization — ~25ms cache hit vs ~450ms cold miss
- [x] Eliminated double rebuild_list() on filter switch
- [x] Replaced clear_all_marks() with single depcache().clear_marked()
- [x] Eliminated redundant rebuild_list() in toggle() via planned_changes() check
- [x] Title bar uses `user_mark_count()` instead of iterating full list every frame
- [x] `update_status_message()` uses `user_mark_count()` instead of list iteration
- [x] `download_size` uses precomputed accessor instead of re-summing every frame
- [x] First pass of rebuild_list collects `Vec<PackageId>` instead of `Vec<String>`
- [x] `toggle_mark_impl`/`toggle_unmark` diff `planned_changes()` instead of full-list HashSets
- [x] `multi_select` replaced with `visual_range: Option<(usize, usize)>` — O(1) membership
- [x] `visible_columns()` pre-allocates with capacity
- [ ] Filter cache clone copies 81k PackageInfo with 7 Strings each — every cache-hit
  rebuild clones the entire list. Consider `Arc` or lazy overlay.
- [ ] Startup takes ~2s due to pre-warming all 5 filter caches
- [ ] Changelog fetched synchronously — UI freezes on slow connections
- [ ] Search results stored as `HashSet<String>` instead of `HashSet<PackageId>`

## Refactoring

- [x] `check_apt_lock()` double-call removed — core layer no longer checks, app layer owns it
- [x] ManagerState dispatch boilerplate — added `shared()`/`shared_mut()` helpers, collapsed ~20
  methods from 4-arm matches to one-liners
- [x] Three-pane layout deduplicated from 3 copies to 1 with modal overlay
- [x] 4 scroll methods unified with `clamped_scroll()` helper
- [x] All dead code removed (18 items — see git history)
- [x] `MarkPreview` refactored from struct-with-bool to proper `Mark`/`Unmark` enum
- [x] `Settings` column visibility refactored from 6 bools to `HashSet<Column>`
- [x] Progress rendering refactored from 11 args to `ProgressSnapshot` struct
- [ ] `bulk_mark`/`bulk_unmark` duplicate `toggle_current` pattern (`app.rs:507-670`)
- [ ] `commit_with_progress` error-recovery structure duplicated between ManagerState and
  PackageManager levels
- [ ] MarkPreview construction split between app.rs and core.rs — should all live in core

## UI/UX

- [x] Navigation keys now pane-local
- [x] Details tab switching moved to `[`/`]`
- [x] Keybinding overhaul per KEYBINDINGS-SPEC.md
- [ ] Scrollbar position indicator in modals
- [ ] Theming — load colors from config file
- [ ] Scroll max calculations use magic numbers instead of actual viewport size

## Features

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
