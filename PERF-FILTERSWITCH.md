# Filter-switch latency optimization

## Problem

Switching to a large filter (Installed: ~75k, Not Installed, All: ~81k)
triggers `rebuild_list()` which iterates the APT cache via FFI and extracts
a `PackageInfo` for every matching package. Profiling shows this takes
400ms+ for the large filters.

The per-package cost is dominated by rust-apt FFI calls during info
extraction. The filter predicate evaluation itself is cheap (~22ms).

## Profiling baseline (post-PERF-RENDERING)

With windowed rendering landed, scrolling is sub-millisecond. The remaining
user-visible cost is the one-time delay on filter switch:

- `rebuild_list()` total: ~162ms avg, 447ms P95 (large filters)
- `render_package_table()`: ~290µs avg (no longer a factor)

## Latency target

If filter-switch latency is under **100ms** after investigation, close this
issue. 100ms is the threshold for "feels instant" in interactive UIs. Above
that, pursue optimization. Re-profile after each step before proceeding to
the next.

## Step 1: Investigate single-pass rebuild_list()

**Status: previously attempted, regressed. Worth a second focused look.**

The current two-pass design exists because of a borrow conflict
(`core.rs:469`): iterating `cache.packages()` borrows the cache, so calling
`extract_package_info_by_name()` (which does `cache.get(name)` — another
cache borrow) during the same iteration fails.

However, `extract_package_info(&pkg)` takes an already-held `&Package`
reference. The only thing it needs from `&self` is `self.get_id(&fullname)`,
which reads from the `fullname_to_id` HashMap. If the borrow conflict is
specifically with `cache.get()` and not with `extract_package_info(&pkg)`,
then calling the latter directly during the first pass — with `fullname_to_id`
borrowed separately — may compile.

The previous attempt inlined extraction into the closure and collected into
a local `Vec<PackageInfo>`. It compiled but **did not produce a measurable
speedup** — the per-package FFI property extraction cost was the same whether
done in one pass or two. The string re-lookup overhead we expected to save
was not the dominant cost.

**Before revisiting:** verify with instrumentation whether `cache.get(name)`
itself is a meaningful fraction of the second pass, or whether the cost is
entirely in property extraction calls on the `Package` object. If the latter,
single-pass won't help regardless of borrow structure.

## Step 2: Per-filter memoization (if Step 1 is insufficient)

A simpler alternative to a full master cache: memoize the `rebuild_list()`
result per filter category. First switch to "Installed" pays 400ms; subsequent
switches are free until invalidated.

**Cache key:** `FilterCategory` only. Search and sort are not part of the
key — they are pure Rust operations applied to the cached list in memory:
- Search: re-filter the cached `Vec<PackageInfo>` (string comparison)
- Sort: re-sort the cached list in place (`Vec::sort_by`)
- Neither requires FFI, so neither requires cache invalidation

**Invalidation (per-filter, not global):**
- `compute_plan()` → invalidate **only MarkedChanges** entry. Other filters
  (Installed/NotInstalled/Upgradable/All) are determined by base APT status,
  which doesn't change until commit/refresh.
- `refresh()` or `commit()` → invalidate **all** entries (APT cache changed).

**Storage:** `HashMap<FilterCategory, Vec<PackageInfo>>` alongside the
current `list` field. On filter switch, use `std::mem::swap` between the
cache entry and `self.shared.list`. This makes switching constant-time
(swapping two Vec pointers), and both the cache and the active list stay
populated for switching back.

This is intentionally a **stopgap**. Because `compute_plan()` invalidates
MarkedChanges on every mark/unmark, the cache has limited benefit during
mark-heavy sessions. Its main win is repeated filter switching without
state changes.

**Pros:**
- Minimal architectural change — `rebuild_list()` stays as-is
- No new types, no borrow model changes
- Amortizes cost: first access per filter is slow, subsequent are instant
- Swap-based switching is O(1)

**Cons:**
- Memory: up to 5 cached lists (one per filter), potentially large
- Still pays the full FFI cost on first access and after invalidation
- MarkedChanges cache invalidated frequently during interactive use

**Column widths:** recompute per filter from the cached/swapped list,
same as today. Could also be cached per filter alongside the list.

## Step 3: Master metadata cache (if Step 2 is insufficient)

Only pursue this if per-filter memoization doesn't meet the latency target.
This is a significant architectural change.

### Architecture

Three distinct layers:

1. **Master metadata cache** (`Vec<PackageMetadata>`, indexed by PackageId)
   - Stable package fields:
     - `fullname` (String): identity key, e.g. "libfoo:amd64"
     - `base_name` (String): for search matching and display, e.g. "libfoo"
     - `section` (String)
     - `installed_version` (String)
     - `candidate_version` (String)
     - `installed_size` (u64)
     - `download_size` (u64)
     - `architecture` (String)
     - `base_status` (Installed/Upgradable/NotInstalled)
   - **Excludes description** — fetched on demand for the selected package
     in the details pane only
   - Built once from FFI at startup, rebuilt on `refresh()` / `commit()`

2. **Dynamic overlay** (`HashMap<PackageId, ChangeAction>`)
   - Derived from `user_intent` + `planned_changes()` after each
     `compute_plan()`
   - Contains both user-requested and dependency-driven marks
   - This is critical for MarkedChanges filter correctness: dependency-marked
     packages (not in `user_intent`) must appear via `planned_changes()`

3. **Filtered view** (`Vec<PackageId>` or `Vec<usize>`)
   - Indices/IDs into the master cache matching the current filter + search
   - Recomputed on filter/search change — pure Rust scan, no FFI
   - Expected cost: 1-5ms for 81k items

### Filter predicates (must match current semantics exactly)

- **Upgradable:** `base_status == Upgradable`
- **Installed:** `base_status == Installed || base_status == Upgradable`
- **NotInstalled:** `base_status == NotInstalled`
- **MarkedChanges:** `user_intent.contains_key(&id) || overlay.contains_key(&id)`
  (overlay includes dependency-driven marks from planned_changes)
- **All:** no filter
- **Search:** matches `base_name` field (not fullname), matching current
  `pkg.name()` semantics in `core.rs:495`

### Display status derivation

Decided: compute at render time from base_status + overlay. Post
PERF-RENDERING, this means ~35 lookups into small HashMaps per frame —
negligible cost. Do not mutate cached entries on mark changes.

### Accessor surface changes

The current API returns `&[PackageInfo]` from `list()`. With a master cache
+ filtered view, the options are:

**A. Filtered indices + master lookup:** `list()` returns `&[PackageId]`,
callers index into master cache. Changes every consumer of `list()` including
`app.rs` selection logic, `toggle()`, `build_mark_preview()`, etc.

**B. Materialized filtered list:** on filter switch, build
`Vec<PackageInfo>` from master cache (Rust-side copy, no FFI). Keeps the
current `list()` API unchanged. Cost: ~1-5ms to copy metadata for the
filtered set. This is the pragmatic choice — same API, 100x faster than FFI.

**C. Filtered list of borrowed refs:** `Vec<&PackageMetadata>` — lifetime
issues with `SharedState` owning both master and borrows. Avoid.

**Recommendation:** Option B. It preserves the existing API contract, is
fast enough (Rust memcpy vs FFI), and avoids a broad refactor of every
`list()` consumer.

### Memory estimate (excluding description)

Per package (stable fields only), rough and likely optimistic:
- 6 Strings × 24 bytes stack each = 144 bytes stack
- 6 heap allocations × ~20 bytes avg content + ~12 bytes allocator overhead = ~192 bytes heap
- id, base_status, sizes: ~24 bytes
- Rough total: ~200-220 bytes per package

81k packages × 210 bytes ≈ **17 MB**

With description included: add ~100-200 bytes avg per package → 25-33 MB
additional. This is why description is excluded by default.

### Startup cost

The master cache is built once from FFI at startup — the same 400ms+ cost
as a full `rebuild_list()` on a large filter. Currently the app starts on
the Upgradable filter, which only materializes the small upgradable set.
With a master cache, startup always pays the full 81k extraction cost.

Options:
- Accept the startup cost (400ms one-time is reasonable for a root tool)
- Build the master cache asynchronously after the first render, showing
  the default filter immediately and populating the cache in background
- Prioritize the default filter's data first, then build the rest lazily

Decision deferred until Step 3 is actually needed.

### Invalidation

Rebuild master cache from FFI on:
- `refresh()` (apt update)
- `commit()` (packages installed/removed)

Rebuild dynamic overlay on:
- `compute_plan()` (marks changed)
- **Ordering constraint:** overlay must be rebuilt *before* the filtered
  view is recomputed. MarkedChanges correctness depends on the overlay
  containing dependency-driven marks — a stale overlay will silently
  drop dependency-marked packages from the filter.

Recompute or rematerialize the current filtered list on:
- Filter category change
- Search query change (re-filter from master, not FFI)
- Sort order change (re-sort in place, not FFI)
- Overlay rebuild — because display status changes in *all* filters
  (a package in the Installed view may now show as MarkedForUpgrade),
  and inclusion changes specifically for MarkedChanges

## Recommended sequence

1. Re-profile filter-switch latency specifically (post-PERF-RENDERING).
2. If above 100ms: investigate single-pass rebuild_list() with focused
   instrumentation on `cache.get(name)` cost vs property extraction cost.
3. If single-pass doesn't help: implement per-filter memoization (Step 2).
4. If memoization invalidation is too frequent or memory too high: implement
   master metadata cache (Step 3).
5. At each step, re-profile before proceeding to the next.
