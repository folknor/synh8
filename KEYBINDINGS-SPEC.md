# Keybindings Spec

## Goals

- Keep navigation predictable and pane-local.
- Keep package actions mnemonic and consistent.
- Use the same confirm/cancel keys in every modal.
- Avoid global key conflicts between pane navigation, details tabs, and package actions.
- Support vim-style usage without making arrows unusable.

## Core Rules

1. `Tab` and `Shift-Tab` change focused pane only.
2. Navigation keys act on the focused pane only.
3. `Esc` always backs out one layer of transient state:
   - close modal
   - exit visual mode
   - cancel active search input
   - otherwise no-op
4. Confirmation is always `Enter`, `y`, or `Space`.
5. Cancellation is always `Esc` or `n`.
6. Package actions are active only when the package pane is focused, unless explicitly marked global.
7. Details-tab switching is separate from pane navigation.

## Focus Model

Three panes exist in the main listing view:

- Filters
- Packages
- Details

The focused pane receives navigation keys. Focus is visible in the UI.

## Main Listing: Global Keys

These keys work regardless of focused pane unless a modal is open.

| Key | Action | Notes |
|---|---|---|
| `Tab` | Focus next pane | Filters -> Packages -> Details -> Filters |
| `Shift-Tab` | Focus previous pane | Reverse cycle |
| `q` | Quit | If pending changes exist, opens exit confirmation |
| `Ctrl-c` | Quit | Same behavior as `q` |
| `/`, `s` | Start search | Opens search-input mode |
| `,` | Open settings | Column visibility, sort order |
| `u` | Run `apt update` | Live download progress |
| `Esc` | Clear active search | Returns to unfiltered view |
| `Esc` | Cancel visual mode | No-op if visual mode not active |
| `?` | Open help overlay | Future feature |

## Main Listing: Filters Pane

These keys apply only when the Filters pane is focused.

| Key | Action | Notes |
|---|---|---|
| `j`, `Down` | Move to next filter | |
| `k`, `Up` | Move to previous filter | |
| `PgDn` | Move to next filter | Clamps to last |
| `PgUp` | Move to previous filter | Clamps to first |
| `g`, `Home` | Jump to first filter | |
| `G`, `End` | Jump to last filter | |
| `Enter`, `Space` | Apply selected filter | Switches filter and rebuilds list |

## Main Listing: Packages Pane

These keys apply only when the Packages pane is focused.

### Navigation

| Key | Action |
|---|---|
| `j`, `Down` | Move to next package |
| `k`, `Up` | Move to previous package |
| `PgDn` | Move down by one page |
| `PgUp` | Move up by one page |
| `g`, `Home` | Jump to first package |
| `G`, `End` | Jump to last package |

### Package Actions

| Key | Action | Notes |
|---|---|---|
| `Space` | Toggle current package mark | Mark or unmark with preview/cascade rules |
| `+` | Mark install/upgrade | Future feature; reserved |
| `-` | Mark remove | Future feature; reserved |
| `=` | Hold / keep | Future feature; reserved |
| `v` | Enter visual mode | Selection mode for batch actions |
| `c` | Show changelog | Opens changelog for current package |
| `r` | Review pending changes | Opens changes preview modal |
| `x` | Mark all upgradable | Marks every upgradable package |
| `X` | Unmark all | Clears all marks |

## Main Listing: Details Pane

These keys apply only when the Details pane is focused.

### Navigation

| Key | Action |
|---|---|
| `j`, `Down` | Scroll down |
| `k`, `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |
| `g`, `Home` | Jump to top |
| `G`, `End` | Jump to bottom |

### Tab Switching

| Key | Action | Notes |
|---|---|---|
| `[` | Previous details tab | Info / Deps / RDeps |
| `]` | Next details tab | Info / Deps / RDeps |

## Search Mode

Entered via `/` or `s`. All keys are captured by the search input.

| Key | Action |
|---|---|
| Printable characters | Append to query |
| `Backspace` | Delete previous character |
| `Enter` | Confirm search and return to listing |
| `Esc` | Cancel search input without changing applied search |

After confirming, the search filter remains active until cleared with `\`.

## Visual Mode

Entered via `v` in the Packages pane. Extends selection as the cursor moves.

| Key | Action | Notes |
|---|---|---|
| `j`, `Down` | Extend selection downward | |
| `k`, `Up` | Extend selection upward | |
| `PgDn` | Extend selection by one page down | |
| `PgUp` | Extend selection by one page up | |
| `g`, `Home` | Extend selection to first package | |
| `G`, `End` | Extend selection to last package | |
| `Space` | Toggle all selected packages | Batch mark/unmark |
| `+` | Mark all selected | Future feature; reserved |
| `-` | Remove all selected | Future feature; reserved |
| `v` | Exit visual mode | |
| `Esc` | Cancel visual mode | |

## Mark Confirm Modal

Shown when marking a package requires additional dependencies or unmarking triggers a cascade.

| Key | Action |
|---|---|
| `y`, `Enter`, `Space` | Confirm |
| `n`, `Esc` | Cancel |
| `j`, `Down` | Scroll down |
| `k`, `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |

## Changes Review Modal

Shown via `r` in the Packages pane. Lists all pending changes grouped by action.

| Key | Action |
|---|---|
| `y`, `Enter`, `Space` | Apply all changes |
| `n`, `Esc` | Cancel and return to listing |
| `j`, `Down` | Scroll down |
| `k`, `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll down by one page |

## Exit Confirmation

Shown when quitting with pending changes.

| Key | Action |
|---|---|
| `y`, `Enter`, `Space` | Quit |
| `n`, `Esc` | Cancel and return to listing |

## Changelog View

Read-only view of the selected package's changelog.

| Key | Action |
|---|---|
| `j`, `Down` | Scroll down |
| `k`, `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |
| `y`, `Enter`, `Space` | Close |

## Settings View

Opened via `,`. Configure column visibility and sort order.

| Key | Action |
|---|---|
| `j`, `Down` | Move to next setting |
| `k`, `Up` | Move to previous setting |
| `Enter`, `Space` | Toggle / advance setting |
| `y`, `Enter`, `Space` | Close and apply |

## Upgrading (Progress View)

Shown during `apt update` or package installation. No keys are active — the operation runs to completion.

## Done (Post-Commit)

Shown after changes are applied. Displays apt output.

| Key | Action |
|---|---|
| `q` | Quit |
| `r` | Return to listing (reloads cache) |
| `j`, `Down` | Scroll output down |
| `k`, `Up` | Scroll output up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |

## Key Conflicts Resolved

These conflicts have been removed from the implementation:

- `h` / `l` no longer globally switch details tabs.
- `Left` / `Right` no longer globally switch details tabs.
- `PgUp` / `PgDn` / `g` / `G` no longer always control the package list regardless of focus.
- Pane focus movement and details-tab movement are separate concepts.
- Search clearing uses `\`, not `Esc`.
- `d` no longer switches details tabs (was conflicting with potential package actions).
- `s` is search, not settings. Settings use `,`.
- `u` is apt update, not review changes. Review uses `r`.
- `N` no longer unmarks all. Unmark all uses `X`.
- `U` no longer runs apt update. Apt update uses `u`.

