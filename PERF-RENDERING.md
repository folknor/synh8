# Per-frame rendering optimization

## Problem

`render_package_table()` in `ui.rs:261` builds a `Vec<Row>` containing one
`Row` per package in `app.core.list()` on every `terminal.draw()` call. For
the All Packages filter (81k packages), this means allocating ~81k Row objects
with 5-6 Cell objects each, every frame — even though only ~35 rows are
visible on screen.

The draw loop in `main.rs:39` fires every 100ms or on every keypress, so
scrolling through a large list triggers this full materialization repeatedly.
The Table widget only displays the visible window, but we currently
materialize every row before handing data to it.

This causes visible scroll lag on Installed, Not Installed, and All filters.

## Why lazy iteration doesn't help

`Table::new()` eagerly collects all rows into a `Vec<Row>` internally
(ratatui `table.rs:335`: `let rows = rows.into_iter().map(Into::into).collect()`).
Passing a lazy iterator or a skip/take chain would still allocate every row.
Manual windowing (pre-slicing) is the only way to avoid full materialization.

## Current flow

1. `terminal.draw()` calls `ui()` → `render_package_table()`
2. `app.core.list()` returns the full filtered `&[PackageInfo]` (up to 81k)
3. All entries are mapped to `Row` objects (lines 272-315)
4. The full `Vec<Row>` plus a `TableState` (with absolute selected index) is
   passed to `Table::new()`
5. Ratatui internally skips to the visible offset and renders ~35 rows

## Proposed fix

Only build rows for the visible window:

1. Capture `total_count = list.len()` before slicing (needed for the title
   and scrollbar).
2. Read `offset = app.ui.table_state.offset()` — the absolute scroll offset
   already maintained by the center-lock navigation logic in `app.rs`.
3. Compute `visible_rows = area.height.saturating_sub(3)` (borders + header).
4. Clamp offset: `offset = offset.min(total_count.saturating_sub(1))` to
   handle stale offsets when the list shrinks (e.g., search results change
   between frames, or `center_scroll_offset()` hasn't run yet after resize).
5. Slice `list[offset..min(offset + visible_rows, total_count)]`.
6. Build `Row` objects only for that slice, translating absolute indices to
   local indices within the map closure (see index translation below).
7. Use a **temporary, local** `TableState` with `offset = 0` and `selected`
   derived defensively from `app.ui.table_state.selected()`:
   - If `None` (empty list), propagate `None`.
   - If `Some(abs)` and `abs` falls within the slice range, use
     `Some(abs - offset)`.
   - Otherwise `None` (stale selection outside visible window).
   This state must not be written back to `app.ui.table_state`.
8. Use the original `app.ui.table_state` (absolute indices) for the scrollbar
   and `total_count` for `ScrollbarState::new()`.
9. Use `total_count` for the title: `format!(" Packages ({}) ", total_count)`.

## Index translation

Every index operation inside the row-building closure must translate between
local (slice-relative) and absolute (full-list) coordinates:

```rust
let abs_idx = offset + local_idx;
```

Use `abs_idx` for:
- `app.ui.multi_select.contains(&abs_idx)` (visual-mode highlighting)
- Any future index-based styling

Use `local_idx` for:
- Position within the `Row` vector (implicit from iterator)

`is_user_marked(pkg.id)` and `display_name(&pkg.name)` are id/name-based,
not index-based, so they need no translation.

Note: `display_name()` is pure Rust (`str::strip_suffix`), not FFI.

## Things to get right

- **Temporary TableState**: must have `offset = 0` and slice-relative
  `selected`. The shared `app.ui.table_state` must not be mutated to make
  indices relative — doing so would break navigation and visual-mode
  assumptions across frames.
- **Scrollbar**: rendered using the original absolute `table_state` and
  `total_count`. Two `TableState` instances will exist in the function scope:
  the app's (for scrollbar) and the local temporary (for the table widget).
- **Title count**: `format!(" Packages ({}) ", total_count)` — captured before
  slicing, so it shows the full filtered count, not the visible window size.
- **table_visible_rows feedback**: compute `visible_rows` once, use it for
  both the slice range and `app.ui.table_visible_rows`. This keeps the slice
  math and the center-lock scroll feedback using exactly the same viewport
  size. On terminal resize, the first frame after resize uses the previous
  frame's value. This is a one-frame lag that exists today and is acceptable.
- **Slice buffer**: adding +1 or +2 to the visible window is fine if it
  simplifies edge behavior, but is not necessary for correctness.
- **Edge cases**: selected index near start/end of list, list shorter than
  visible area, empty list after search/filter.

## Scope

This change should be local to `render_package_table()` in `ui.rs`. The
app-level selection, scroll state, and multi-select indices remain absolute
indices into the full list. Only the renderer translates to slice-relative
coordinates. No changes to `core.rs`, `app.rs`, or data structures.

## Expected impact

Row construction drops from O(total_packages) to O(visible_rows) per frame.
On large filters (Installed, Not Installed, All), this eliminates the vast
majority of per-frame Row/Cell allocations. On small filters like Upgradable,
the improvement is proportionally smaller.

## What this does NOT fix

Filter switching still triggers a full `rebuild_list()` which extracts
PackageInfo for all matching packages via rust-apt FFI. That cost is
addressed separately in PERF-FILTERSWITCH.md.
