# Filter-switch latency optimization

## Problem

Switching to a large filter (Installed: ~75k, Not Installed, All: ~81k)
triggers `rebuild_list()` which iterates the APT cache via FFI and extracts
a `PackageInfo` for every matching package. Profiling shows this takes
400ms+ for the large filters.

The per-package cost is dominated by rust-apt FFI calls: `cache.get(name)`,
`pkg.candidate()`, `pkg.is_installed()`, `pkg.is_upgradable()`, version
string extraction, section, arch, summary, and sizes. Restructuring the
Rust-side loop (single-pass vs two-pass) did not measurably reduce this
cost — the bottleneck is the FFI boundary itself, scaled by package count.

## Current flow

1. User switches filter → `apply_filter()` → `rebuild_list()`
2. First pass: iterate `cache.packages(&sort)`, apply filter predicate,
   collect matching fullnames as `Vec<String>`
3. Second pass: for each fullname, call `cache.get(name)` (FFI re-fetch)
   and extract ~10 properties into `PackageInfo`
4. Sort and compute column widths

Both passes are expensive for large sets. The first pass iterates the full
APT cache even for narrow filters (except Upgradable, which uses a
PackageSort hint). The second pass does a string-based cache lookup per
package.

## What we know from profiling

- `rebuild_list()` total: ~183ms avg, 400ms+ P95 (large filters)
- First pass (filter + collect names): ~22ms avg
- Second pass (re-fetch + extract info): ~149ms avg (dominates)
- Sort: ~10ms avg
- Column widths: <1ms

The expensive part is materializing `PackageInfo` from the APT cache for
large package sets. The filter predicate evaluation is cheap.

## Design direction: cached stable metadata

The core idea is to separate stable package metadata (changes only on
`apt update` / cache refresh) from dynamic display state (changes on every
mark/plan).

### Stable metadata (cache once, refresh on apt update)

Per package, fetched once from FFI:
- name / fullname
- PackageId
- section
- installed version string
- candidate version string
- installed size
- download size
- architecture
- base status (Installed / Upgradable / NotInstalled)

Possibly deferred or excluded from the base cache:
- description / summary (large strings, only shown in details pane for the
  selected package — fetching 81k summaries may add significant memory)

### Dynamic state (derived per plan/mark cycle)

- PackageStatus with mark overlay (MarkedForUpgrade, MarkedForInstall, etc.)
- Whether the package is in the planned changeset

This can be computed by scanning `user_intent` and `planned_changes()`,
both of which are small sets, and overlaying onto the cached base status.

### Filtered views

Filter switching becomes an in-memory scan of the cached metadata with a
predicate, producing a `Vec<usize>` of indices (or a `Vec<PackageId>`) into
the master cache. No FFI calls. Cost: O(N) Rust-side comparison for N cached
packages — likely 1-5ms for 81k items.

Sorting operates on the filtered index set using cached string fields.

### Invalidation

The master cache must be rebuilt when the underlying APT data changes:
- After `apt update` (cache refresh)
- After `commit()` (packages installed/removed)
- After `refresh()` explicit reload

It does NOT need rebuilding on:
- Filter switch
- Search
- Mark/unmark (only dynamic status changes)
- Sort order change

## Open questions

1. **Memory cost.** 81k PackageInfo structs with owned Strings (name,
   section, two version strings, arch) is non-trivial. If description is
   included, it could be much larger. Need to measure or estimate before
   committing. Consider excluding description from the cache and fetching
   it on demand for the selected package only.

2. **Borrow architecture.** The current `SharedState` holds `list: Vec<PackageInfo>`
   as the display-ready filtered list. A master cache would either replace this
   with a two-level structure (master + filtered view) or sit alongside it.
   The filtered view could be `Vec<usize>` indices into the master cache,
   but rendering would then need to index through the master. This affects
   the borrow patterns in `rebuild_list()`, `render_package_table()`, and
   anywhere that reads `self.shared.list`.

3. **Mark status overlay.** Currently `rebuild_list()` applies `user_intent`
   to each `PackageInfo.status` during construction. With a master cache,
   the options are: (a) mutate cached entries in place on plan changes,
   (b) store base status in the cache and compute display status at render
   time from base + user_intent + planned_changes. Option (b) is cleaner
   but means the renderer must do a lookup per visible row.

4. **Interaction with PERF-RENDERING.** If per-frame rendering is fixed
   first (only building rows for visible packages), the urgency of this
   optimization decreases for scrolling. The remaining cost is the one-time
   filter-switch delay. Re-profile after the rendering fix to decide
   whether this is still worth pursuing.

## Recommended sequence

1. Fix per-frame rendering first (PERF-RENDERING.md).
2. Re-profile filter-switch latency specifically.
3. If still unacceptable, prototype the master metadata cache with
   description excluded.
4. Measure memory impact before committing to the design.
5. Only then restructure `rebuild_list()` to filter from the cache
   instead of querying APT.
