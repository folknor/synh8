# Keybindings

The tables below are generated from `src/keymap.rs`, which also drives key
dispatch and the help bar. Do not edit them by hand: change the registry and
run the tests with `SYNH8_BLESS=1` to regenerate them. A test fails when this
file and the registry disagree.

## Rules

1. `Tab` and `Shift-Tab` change the focused pane only.
2. Navigation keys act on the focused pane only. `j`/`k` work like `↓`/`↑`.
3. `Esc` backs out one layer of transient state: close a modal, end a visual
   selection, clear an applied search, cancel search input.
4. Confirmation is always `Space`. Cancellation is always `Esc`.
5. Package actions are active only when the Packages pane is focused.
6. Marks are intents: the plan (what APT will actually do) is recomputed after
   every mark, and anything the plan changes beyond the packages you acted on
   is shown for confirmation. Cancelling undoes the whole action.

## Anywhere

<!-- keymap:anywhere -->
| Key | Action |
|---|---|
| `Ctrl-c` | Quit immediately, without confirmation |
<!-- /keymap -->

## Main listing (any pane)

<!-- keymap:global -->
| Key | Action |
|---|---|
| `Tab` | Focus next pane (Filters -> Packages -> Details) |
| `Shift-Tab` | Focus previous pane |
| `s` | Start search |
| `u` | Run `apt update`; marks are kept |
| `F2` | Settings (columns, sort order) |
| `q` | Quit (asks first if changes are marked) |
<!-- /keymap -->

## Filters pane

<!-- keymap:filters -->
| Key | Action |
|---|---|
| `↑` / `k` | Previous filter |
| `↓` / `j` | Next filter |
| `PgUp` | Previous filter |
| `PgDn` | Next filter |
| `Home` | First filter |
| `End` | Last filter |
<!-- /keymap -->

## Packages pane

<!-- keymap:packages -->
| Key | Action |
|---|---|
| `↑` / `k` | Previous package |
| `↓` / `j` | Next package |
| `PgUp` | Up one page |
| `PgDn` | Down one page |
| `Home` | First package |
| `End` | Last package |
| `Space` | Toggle mark: mark for install/upgrade, or undo a mark |
| `+` | Mark for install/upgrade (again to unmark) |
| `-` | Mark for removal (again to unmark) |
| `=` | Hold at the current state (again to release) |
| `v` | Start a visual selection |
| `c` | Show changelog |
| `a` | Review and apply marked changes |
| `x` | Mark all upgradable packages |
| `z` | Unmark everything |
<!-- /keymap -->

## Visual selection

Started with `v` in the Packages pane. The selection runs from the row where
it started to the cursor. Whether the selection is marked or unmarked is
decided by the row it started on.

<!-- keymap:visual -->
| Key | Action |
|---|---|
| `↑` / `k` | Extend selection up |
| `↓` / `j` | Extend selection down |
| `PgUp` | Extend selection one page up |
| `PgDn` | Extend selection one page down |
| `Home` | Extend selection to the first package |
| `End` | Extend selection to the last package |
| `Space` / `v` / `+` | Mark the selection (or unmark it, if the first row is marked) |
| `-` | Mark the selection for removal |
| `Esc` | Cancel the selection |
<!-- /keymap -->

## Details pane

<!-- keymap:details -->
| Key | Action |
|---|---|
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `PgUp` | Scroll up one page |
| `PgDn` | Scroll down one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |
| `,` | Previous tab (Info / Deps / RDeps) |
| `.` | Next tab |
<!-- /keymap -->

## While a search is applied

<!-- keymap:search-active -->
| Key | Action |
|---|---|
| `Esc` | Clear the search filter |
<!-- /keymap -->

## Search input

Entered with `s`. Results update as you type.

<!-- keymap:search -->
| Key | Action |
|---|---|
| `Enter` | Confirm search and return to the list |
| `Esc` | Cancel: restore the search that was active before |
| `Backspace` | Delete the previous character |
| `↑` | Confirm search and move up the results |
| `↓` | Confirm search and move down the results |
| `PgUp` | Confirm search and page up the results |
| `PgDn` | Confirm search and page down the results |
| `Printable characters` | Append to the query (results update as you type) |
<!-- /keymap -->

## Mark confirmation

Shown when a mark changes the plan beyond the packages acted on: new
installs, removals or downgrades, or packages dropping out of the plan.

<!-- keymap:mark-confirm -->
| Key | Action |
|---|---|
| `Space` | Confirm |
| `Esc` | Cancel: undo the mark |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `PgUp` | Scroll up one page |
| `PgDn` | Scroll down one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |
<!-- /keymap -->

## Changes review

Shown with `a`. Lists every planned change, grouped by action. Changes cannot
be applied while the resolver reports errors.

<!-- keymap:changes -->
| Key | Action |
|---|---|
| `Space` | Apply all changes |
| `Esc` | Back to the list |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `PgUp` | Scroll up one page |
| `PgDn` | Scroll down one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |
<!-- /keymap -->

## Changelog

<!-- keymap:changelog -->
| Key | Action |
|---|---|
| `Esc` / `Space` | Close |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `PgUp` | Scroll up one page |
| `PgDn` | Scroll down one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |
<!-- /keymap -->

## Settings

Opened with `F2`.

<!-- keymap:settings -->
| Key | Action |
|---|---|
| `↑` / `k` | Previous setting |
| `↓` / `j` | Next setting |
| `Space` | Toggle / advance the setting |
| `Esc` | Close and apply |
<!-- /keymap -->

## Exit confirmation

Shown when quitting with marked changes.

<!-- keymap:confirm-exit -->
| Key | Action |
|---|---|
| `Space` | Quit without applying |
| `Esc` | Back to the list |
<!-- /keymap -->

## After applying changes

Shows the apt/dpkg output and any errors. The package cache has already been
reloaded.

<!-- keymap:done -->
| Key | Action |
|---|---|
| `Esc` / `Space` | Back to the list |
| `↑` / `k` | Scroll up |
| `↓` / `j` | Scroll down |
| `PgUp` | Scroll up one page |
| `PgDn` | Scroll down one page |
| `Home` | Jump to top |
| `End` | Jump to bottom |
<!-- /keymap -->

## While applying or updating

No keys are active: the operation runs to completion.
