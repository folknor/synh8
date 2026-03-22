# synh8 TODO

## Bugs

- [x] Hardcoded `amd64` in dependency traversal — now uses `cache.native_arch()`
- [ ] Virtual package dependency resolution — cascade unmark doesn't work for packages with
  virtual package dependencies (e.g., nvidia packages). The dependency check only matches
  direct package names, not virtual package providers.
- [ ] `cancel_mark` searches filtered list by display name (`app.rs:373`) — if the package isn't
  in the current filtered view, cancel silently fails and the mark stays.
- [x] `filter_count` for MarkedChanges undercounts — now uses `planned_changes().len()`
- [x] `PackageId(u32::MAX)` sentinel — `extract_package_info` now returns `None`
- [ ] Double-fault in commit leaves permanent Transitioning state — if commit fails AND
  `ManagerState::new()` also fails, every subsequent access panics.

## Error Handling

- [ ] `dup`/`dup2` return values unchecked (`progress.rs:47-53, 74-76`) — failure leaves
  stdout/stderr broken.
- [ ] FD leak on `StdioRedirect::capture()` error path (`progress.rs:46-50`) — early return
  after `dup()` leaks file descriptors.
- [ ] `clear_marked()` failure swallowed (`apt.rs:183`) — eprintln goes nowhere during commit
  (stderr redirected), leaves stale APT marks.
- [ ] Interrupted operation — detect and show "Resuming interrupted dpkg operation"

## Dead Code (to remove)

- [x] `PendingChanges` struct — removed
- [x] `AptPackageState` / `get_apt_status` — removed
- [x] `PackageManager::set_intent` — removed
- [x] `PackageManager::get_package_by_id` — removed
- [x] `ManagerState::is_clean`/`is_dirty`/`is_planned` — removed
- [x] `ManagerState::plan_errors` — removed
- [x] `ManagerState::sort_settings` — removed
- [x] `ManagerState::commit()` non-progress version — removed
- [x] `ManagerState::mark_remove()` — removed
- [x] `PackageManager::mark_remove()` (Clean and Dirty) — removed
- [x] `PackageManager::shared()`/`shared_mut()` — removed (eliminated warnings)
- [x] `AptCache::commit()` non-progress version — removed
- [x] `AptCache::count_upgradable()` — removed
- [x] `PackageManager<Planned>::download_size()`/`install_size_change()`/`has_errors()` — removed
- [x] `ColumnWidths::reset()` — removed
- [x] `PhantomData<S>` — removed (redundant with `state: S`)
- [x] `ModalState::mark_confirm_scroll` — removed

## Performance

- [x] Windowed table rendering — sub-millisecond per frame
- [x] Per-filter memoization — ~25ms cache hit vs ~450ms cold miss
- [x] Eliminated double rebuild_list() on filter switch
- [x] Replaced clear_all_marks() with single depcache().clear_marked()
- [x] Eliminated redundant rebuild_list() in toggle() via planned_changes() check
- [x] Title bar uses `user_mark_count()` instead of iterating full list every frame
- [x] `update_status_message()` uses `user_mark_count()` instead of list iteration
- [x] `download_size` uses precomputed accessor instead of re-summing every frame
- [ ] Filter cache clone copies 81k PackageInfo with 7 Strings each (`core.rs:477`) — every
  cache-hit rebuild clones the entire list. Consider `Arc` or lazy overlay.
- [ ] First pass of rebuild_list collects 81k Strings instead of PackageIds (`core.rs:495`) —
  collecting `PackageId` (4 bytes) instead of String (~30 bytes + heap) would eliminate 81k
  String allocations on cache miss.
- [ ] `toggle_mark_impl`/`toggle_unmark` build HashSets from full list (`core.rs:1145`) — could
  diff `planned_changes()` before vs after instead.
- [ ] Startup takes ~2s due to pre-warming all 5 filter caches
- [ ] Changelog fetched synchronously — UI freezes on slow connections
- [ ] Search results stored as `HashSet<String>` instead of `HashSet<PackageId>` (`core.rs:29`)
- [ ] `visible_columns()` allocates Vec every frame (`types.rs:391`)
- [ ] `multi_select` uses HashSet for contiguous range (`app.rs:20`) — a `(start, end)` pair
  would be O(1)

## Refactoring

- [x] `check_apt_lock()` double-call removed — core layer no longer checks, app layer owns it
- [ ] ManagerState dispatch boilerplate — ~28 methods that just forward to inner `PackageManager<S>`.
  Add `shared()`/`shared_mut()` helpers on ManagerState to collapse most of them.
- [ ] `bulk_mark`/`bulk_unmark` duplicate `toggle_current` pattern (`app.rs:507-670`)
- [ ] Three-pane layout rendered identically in 3 branches of `ui.rs` — extract helper
- [ ] 4 scroll methods with same clamped-scroll pattern (`app.rs:832-854`)
- [ ] `commit_with_progress` error-recovery structure duplicated between ManagerState and
  PackageManager levels
- [ ] `MarkPreview.additional_upgrades` reused for "also unmarked" — semantic abuse
- [ ] MarkPreview construction split between app.rs and core.rs — should all live in core
- [ ] `compute_plan()` + `rebuild_list()` always called together — consider combined method
- [ ] `Settings` uses 6 separate bools for column visibility — `HashSet<Column>` more maintainable
- [ ] Progress rendering passes 11 args — extract renderable state struct

## UI/UX

- [x] Navigation keys now pane-local
- [x] Details tab switching moved to `[`/`]`
- [x] Keybinding overhaul per KEYBINDINGS-SPEC.md
- [ ] Scrollbar position indicator in modals
- [ ] Theming — load colors from config file
- [ ] Scroll max calculations use magic numbers instead of actual viewport size (`app.rs:839, 845`)

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
