# synh8

Synaptic-inspired TUI for managing APT packages on Debian/Ubuntu. Rust + ratatui + rust-apt.

## Bash rules
- Never use sed, find, awk, or complex bash commands
- Never chain commands with &&
- Never chain commands with ;
- Never pipe commands with |
- Never read or write from /tmp

## Build

Requires `libapt-pkg-dev`. Linux-only.

```bash
sudo apt install libapt-pkg-dev
cargo build --release
```

Must run as root:
```bash
sudo ./target/release/synh8
```

## Feature flags

- `hotpath` — function-level timing profiling (zero overhead when disabled)
- `hotpath-alloc` — allocation tracking

```bash
cargo build --release --features hotpath
sudo ./target/release/synh8
# Report prints to stderr on exit
```

Instrumented functions: `core::plan`, `core::rebuild_list`, `core::toggle`,
`ui::ui`, `ui::render_package_table`, `app::refresh_ui_state`,
`app::restore_selection`, `app::update_cached_deps`.

## Architecture

### Source layout

- `main.rs` — event loop, key dispatch
- `app.rs` — TUI application state, wraps `ManagerState` with UI state
- `ui.rs` — ratatui rendering (windowed table rendering for large lists)
- `core.rs` — typestate package manager (~1600 lines), filter caching
- `types.rs` — enums, structs, type definitions
- `apt.rs` — thin wrapper around rust-apt with stable `PackageId` handles
- `search.rs` — SQLite FTS5 full-text search across package names
- `progress.rs` — terminal progress rendering for downloads/installs

### Typestate pattern (core.rs)

`PackageManager<S>` uses compile-time states:

- **`Clean`** — no user marks. Can mark packages (→ Dirty).
- **`Dirty`** — has marks, no computed plan. Can plan (→ Planned) or reset (→ Clean).
- **`Planned`** — dependencies resolved, changeset computed. Can commit or modify (→ Dirty).

```rust
pub struct PackageManager<S> {
    shared: SharedState,
    state: S,
    _phantom: PhantomData<S>,
}
```

Because consuming-self types can't be held in the TUI, `ManagerState` enum wraps all three variants plus a `Transitioning` placeholder used during `std::mem::take` transitions. `Transitioning` must never be observed outside a `&mut self` method body.

### User intent model

`user_intent: HashMap<PackageId, UserIntent>` is the single source of truth for what the user wants. APT marks are derived from intent via `plan()`, not set directly. Display status is overlaid at render time from base status + user_intent + planned_changes.

### Filter caching

`rebuild_list()` results are memoized per `FilterCategory` with base statuses (before user_intent overlay). Cache entries persist across switches and are cloned on restore with fresh overlay. Pre-warmed at startup.

Invalidation: `compute_plan()` clears MarkedChanges only; `refresh()`/`commit()` clears all entries.

### Windowed rendering

`render_package_table()` only builds `Row`/`Cell` objects for the visible ~35 rows, using a temporary slice-relative `TableState`. The app-level `TableState` stays absolute. Scrollbar uses the original absolute state.

## Conventions

- Strict clippy lints: `unwrap_used` is NOT denied (unlike pbfhogg), but several style and correctness lints are deny-level. See `[lints.clippy]` in Cargo.toml.
- No test suite. Manual TUI testing only (requires root + apt packages).
- Single-threaded — no async, no threading. rust-apt types are not Send.
- `PackageId` is an opaque handle valid for one cache generation. Maps to full package names (including arch, e.g., "libfoo:amd64").

## Key dependencies

- **rust-apt 0.9** — APT cache bindings (main FFI dependency)
- **ratatui 0.30** — TUI rendering
- **crossterm 0.28** — terminal I/O
- **rusqlite 0.39 (bundled)** — SQLite FTS5 search index
- **hotpath 0.14** — function-level profiling

## Performance notes

The dominant cost in this codebase is rust-apt FFI property extraction when iterating large package sets (~81k total packages). Key findings:

- `cache.resolve()` (dependency resolver): ~45ms. Not the bottleneck.
- Per-package FFI extraction: ~150ms for 81k packages. Cannot be reduced by Rust-side restructuring.
- `depcache().clear_marked()` replaced per-package mark_keep loop: 401ms → 114ms.
- Filter cache eliminates FFI on repeated filter switches: ~25ms cache hit vs ~450ms cold miss.
- Windowed rendering: ~300µs per frame regardless of list size.
