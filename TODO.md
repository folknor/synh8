# synh8 TODO

## Bugs

- [ ] Hardcoded `amd64` in dependency traversal (`core.rs:1267`) — `find_user_intent_depending_on`
  falls back to `format!("{dep_name}:amd64")` instead of `cache.native_arch()`. Breaks cascade
  unmark on arm64/i386 systems.
- [ ] Virtual package dependency resolution — cascade unmark doesn't work for packages with
  virtual package dependencies (e.g., nvidia packages). The dependency check only matches
  direct package names, not virtual package providers.
- [ ] `cancel_mark` searches filtered list by display name (`app.rs:373`) — if the package isn't
  in the current filtered view, cancel silently fails and the mark stays.
- [ ] `filter_count` for MarkedChanges undercounts (`core.rs:1032`) — shows `user_intent.len()`
  but actual filter includes dependency-marked packages from the plan. Sidebar shows "1" but
  clicking shows 6.
- [ ] `PackageId(u32::MAX)` sentinel (`apt.rs:239`) — `extract_package_info` should return `None`
  when `get_id` fails instead of propagating a sentinel.
- [ ] Double-fault in commit leaves permanent Transitioning state (`core.rs:1359`) — if commit
  fails AND `ManagerState::new()` also fails, every subsequent access panics.

## Error Handling

- [ ] `dup`/`dup2` return values unchecked (`progress.rs:47-53, 74-76`) — failure leaves
  stdout/stderr broken.
- [ ] FD leak on `StdioRedirect::capture()` error path (`progress.rs:46-50`) — early return
  after `dup()` leaks file descriptors.
- [ ] `clear_marked()` failure swallowed (`apt.rs:183`) — eprintln goes nowhere during commit
  (stderr redirected), leaves stale APT marks.
- [ ] Interrupted operation — detect and show "Resuming interrupted dpkg operation"

## Dead Code (to remove)

- [ ] `PendingChanges` struct (`types.rs:346`) — never used
- [ ] `AptPackageState` / `get_apt_status` (`apt.rs:198-206, 374-381`) — never called
- [ ] `PackageManager::set_intent` (`core.rs:175`) — never called
- [ ] `PackageManager::get_package_by_id` (`core.rs:416`) — never called
- [ ] `ManagerState::is_clean`/`is_dirty`/`is_planned` (`core.rs:737-749`) — never called
- [ ] `ManagerState::plan_errors` (`core.rs:760-765`) — never called
- [ ] `ManagerState::sort_settings` (`core.rs:1056-1063`) — never called
- [ ] `ManagerState::commit()` non-progress version (`core.rs:1333`) — never called
- [ ] `ManagerState::mark_remove()` (`core.rs:1093-1100`) — never called
- [ ] `AptCache::commit()` non-progress version (`apt.rs:339-347`) — never called
- [ ] `AptCache::native_arch()` (`apt.rs:66-68`) — never called (but needed for amd64 bug fix)
- [ ] `AptCache::count_upgradable()` (`apt.rs:320-325`) — never called
- [ ] `PackageManager<Planned>::download_size()`/`install_size_change()`/`has_errors()`
  (`core.rs:336-348`) — never called
- [ ] `ColumnWidths::reset()` (`types.rs:434-439`) — never called
- [ ] `PackageStatus::Broken` (`types.rs:111`) — never constructed
- [ ] `PhantomData<S>` on `PackageManager` (`core.rs:113`) — redundant, `state: S` exists

## Performance

- [x] Windowed table rendering — sub-millisecond per frame
- [x] Per-filter memoization — ~25ms cache hit vs ~450ms cold miss
- [x] Eliminated double rebuild_list() on filter switch
- [x] Replaced clear_all_marks() with single depcache().clear_marked()
- [x] Eliminated redundant rebuild_list() in toggle() via planned_changes() check
- [ ] Filter cache clone copies 81k PackageInfo with 7 Strings each (`core.rs:477`) — every
  cache-hit rebuild clones the entire list. Consider `Arc` or lazy overlay.
- [ ] First pass of rebuild_list collects 81k Strings instead of PackageIds (`core.rs:495`) —
  collecting `PackageId` (4 bytes) instead of String (~30 bytes + heap) would eliminate 81k
  String allocations on cache miss.
- [ ] Title bar counts user marks by iterating full list every frame (`ui.rs:38`) — 81k HashMap
  lookups on All Packages filter. Use `user_intent.len()` instead.
- [ ] `update_status_message()` iterates list + hash lookups (`app.rs:909`) — same issue,
  `user_intent.len()` gives the count directly.
- [ ] `toggle_mark_impl`/`toggle_unmark` build HashSets from full list (`core.rs:1145`) — could
  diff `planned_changes()` before vs after instead.
- [ ] Startup takes ~2s due to pre-warming all 5 filter caches
- [ ] Changelog fetched synchronously — UI freezes on slow connections
- [ ] Search results stored as `HashSet<String>` instead of `HashSet<PackageId>` (`core.rs:29`)
- [ ] `download_size` re-summed every frame despite being precomputed in Planned state (`ui.rs:26`)
- [ ] `visible_columns()` allocates Vec every frame (`types.rs:391`)
- [ ] `multi_select` uses HashSet for contiguous range (`app.rs:20`) — a `(start, end)` pair
  would be O(1)

## Refactoring

- [ ] ManagerState dispatch boilerplate — ~28 methods that just forward to inner `PackageManager<S>`.
  Add `shared()`/`shared_mut()` helpers on ManagerState to collapse most of them.
- [ ] `bulk_mark`/`bulk_unmark` duplicate `toggle_current` pattern (`app.rs:507-670`)
- [ ] Three-pane layout rendered identically in 3 branches of `ui.rs` — extract helper
- [ ] 4 scroll methods with same clamped-scroll pattern (`app.rs:832-854`)
- [ ] `commit`/`commit_with_progress` — same take-match-recover structure duplicated (`core.rs`)
- [ ] `MarkPreview.additional_upgrades` reused for "also unmarked" — semantic abuse
- [ ] MarkPreview construction split between app.rs and core.rs — should all live in core
- [ ] `check_apt_lock()` called at both app and core layers — double-checked on refresh path
- [ ] `compute_plan()` + `rebuild_list()` always called together — consider combined method
- [ ] `Settings` uses 6 separate bools for column visibility — `HashSet<Column>` more maintainable
- [ ] Progress rendering passes 11 args — extract renderable state struct

## UI/UX

- [x] Navigation keys now pane-local
- [x] Details tab switching moved to `[`/`]`
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
