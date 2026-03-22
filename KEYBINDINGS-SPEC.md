# Keybindings Spec

## Goals

- Keep navigation predictable and pane-local.
- Keep package actions mnemonic and consistent.
- Use the same confirm/cancel keys in every modal.
- Avoid global key conflicts between pane navigation, details tabs, and package actions.

## Core Rules

1. `Tab` and `Shift-Tab` change focused pane only.
2. Navigation keys act on the focused pane only.
3. `Esc` always backs out one layer of transient state:
   - close modal
   - exit visual mode
   - clear active search
   - cancel active search input
4. Confirmation is always `Space`.
5. Cancellation is always `Esc`.
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
| `s` | Start search | Opens search-input mode |
| `F2` | Open settings | Column visibility, sort order |
| `u` | Run `apt update` | Live download progress |
| `?` | Open help overlay | Future feature |

- Ctrl-c also quits, with zero confirmation.

## Main Listing: Filters Pane

These keys apply only when the Filters pane is focused.

| Key | Action | Notes |
|---|---|---|
| `Down` | Move to next filter | |
| `Up` | Move to previous filter | |
| `PgDn` | Move to next filter | Clamps to last |
| `PgUp` | Move to previous filter | Clamps to first |
| `Home` | Jump to first filter | |
| `End` | Jump to last filter | |

## Main Listing: Packages Pane

These keys apply only when the Packages pane is focused.

### Navigation

| Key | Action |
|---|---|
| `Down` | Move to next package |
| `Up` | Move to previous package |
| `PgDn` | Move down by one page |
| `PgUp` | Move up by one page |
| `Home` | Jump to first package |
| `End` | Jump to last package |

### Package Actions

| Key | Action | Notes |
|---|---|---|
| `Space` | Toggle current package mark | Mark or unmark with preview/cascade rules |
| `+` | Mark install/upgrade | Future feature; reserved |
| `-` | Mark remove | Future feature; reserved |
| `=` | Hold / keep | Future feature; reserved |
| `v` | Enter visual mode | Selection mode for batch actions |
| `c` | Show changelog | Opens changelog for current package |
| `a` | Apply pending changes | Opens changes preview modal |
| `x` | Mark all upgradable | Marks every upgradable package |
| `z` | Unmark all | Clears all marks |

## Main Listing: Details Pane

These keys apply only when the Details pane is focused.

### Navigation

| Key | Action |
|---|---|
| `Down` | Scroll down |
| `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |

### Tab Switching

| Key | Action | Notes |
|---|---|---|
| `,` | Previous details tab | Info / Deps / RDeps |
| `.` | Next details tab | Info / Deps / RDeps |

## Search Mode

Entered via `s`. All keys are captured by the search input.

| Key | Action |
|---|---|
| Printable characters | Append to query |
| `Backspace` | Delete previous character |
| `Enter` | Confirm search and return to listing |
| `Up`, `Down`, `PgUp`, `PgDn` | Confirm search and navigate results |
| `Esc` | Cancel search input without changing applied search |

After confirming, the search filter remains active until cleared with `Esc` from the listing.

## Visual Mode

Entered via `v` in the Packages pane. Extends selection as the cursor moves.

| Key | Action | Notes |
|---|---|---|
| `Down` | Extend selection downward | |
| `Up` | Extend selection upward | |
| `PgDn` | Extend selection by one page down | |
| `PgUp` | Extend selection by one page up | |
| `Home` | Extend selection to first package | |
| `End` | Extend selection to last package | |
| `Space` | Toggle all selected packages | Batch mark/unmark |
| `+` | Mark all selected | Future feature; reserved |
| `-` | Remove all selected | Future feature; reserved |
| `v` | Exit visual mode | |
| `Esc` | Cancel visual mode | |

## Mark Confirm Modal

Shown when marking a package requires additional dependencies or unmarking triggers a cascade.

| Key | Action |
|---|---|
| `Space` | Confirm |
| `Esc` | Cancel |
| `Down` | Scroll down |
| `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |

## Changes Review Modal

Shown via `a` in the Packages pane. Lists all pending changes grouped by action.

| Key | Action |
|---|---|
| `Space` | Apply all changes |
| `Esc` | Cancel and return to listing |
| `Down` | Scroll down |
| `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |

## Exit Confirmation

Shown when quitting with pending changes.

| Key | Action |
|---|---|
| `Space` | Quit |
| `Esc` | Cancel and return to listing |

## Changelog View

Read-only view of the selected package's changelog.

| Key | Action |
|---|---|
| `Down` | Scroll down |
| `Up` | Scroll up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |
| `Esc`, `Space` | Close |

## Settings View

Opened via `F2`. Configure column visibility and sort order.

| Key | Action |
|---|---|
| `Down` | Move to next setting |
| `Up` | Move to previous setting |
| `Space` | Toggle / advance setting |
| `Esc` | Close and apply |

## Upgrading (Progress View)

Shown during `apt update` or package installation. No keys are active — the operation runs to completion.

## Done (Post-Commit)

Shown after changes are applied. Displays apt output. Any dismiss key reloads the cache and returns to the listing.

| Key | Action |
|---|---|
| `Esc`, `Space` | Dismiss (reload cache, return to listing) |
| `Down` | Scroll output down |
| `Up` | Scroll output up |
| `PgDn` | Scroll down by one page |
| `PgUp` | Scroll up by one page |
