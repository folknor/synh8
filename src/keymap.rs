//! Keybinding registry: the single owner of which key does what.
//!
//! Key dispatch, the help bar, in-modal hints and the tables in
//! `docs/keybindings.md` are all derived from the tables below. A unit test
//! fails if the document drifts; run the tests with `SYNH8_BLESS=1` to
//! regenerate its tables.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// A set of bindings that is active in some UI state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Context {
    /// Active in every state except while an operation is running
    Anywhere,
    /// Main listing, whatever pane is focused
    Global,
    Filters,
    Packages,
    /// Packages pane while a visual selection is active
    Visual,
    Details,
    /// Main listing while a search filter is applied
    SearchActive,
    /// Typing a search query
    Search,
    MarkConfirm,
    Changes,
    Changelog,
    Settings,
    ConfirmExit,
    Done,
}

impl Context {
    pub fn all() -> &'static [Context] {
        &[
            Self::Anywhere,
            Self::Global,
            Self::Filters,
            Self::Packages,
            Self::Visual,
            Self::Details,
            Self::SearchActive,
            Self::Search,
            Self::MarkConfirm,
            Self::Changes,
            Self::Changelog,
            Self::Settings,
            Self::ConfirmExit,
            Self::Done,
        ]
    }

    /// Marker name used in docs/keybindings.md
    fn doc_id(self) -> &'static str {
        match self {
            Self::Anywhere => "anywhere",
            Self::Global => "global",
            Self::Filters => "filters",
            Self::Packages => "packages",
            Self::Visual => "visual",
            Self::Details => "details",
            Self::SearchActive => "search-active",
            Self::Search => "search",
            Self::MarkConfirm => "mark-confirm",
            Self::Changes => "changes",
            Self::Changelog => "changelog",
            Self::Settings => "settings",
            Self::ConfirmExit => "confirm-exit",
            Self::Done => "done",
        }
    }
}

/// What a key does. The meaning of the navigation actions depends on the
/// context (move a selection, scroll a view).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    ForceQuit,
    Quit,
    FocusNext,
    FocusPrev,
    StartSearch,
    OpenSettings,
    Update,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Toggle,
    MarkInstall,
    MarkRemove,
    MarkHold,
    VisualStart,
    VisualMark,
    VisualRemove,
    Changelog,
    ReviewChanges,
    MarkAllUpgrades,
    UnmarkAll,
    PrevTab,
    NextTab,
    ClearSearch,
    SearchConfirm,
    SearchBackspace,
    /// A printable character typed into the search query
    SearchInput,
    Confirm,
    Cancel,
}

/// A key a binding responds to
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Code(KeyCode),
    Ctrl(char),
    /// Any printable character
    AnyChar,
}

impl Key {
    fn matches(self, ev: &KeyEvent) -> bool {
        let ctrl_or_alt = ev
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        // Some terminals send Shift+Tab as Tab with SHIFT rather than BackTab.
        let code = match ev.code {
            KeyCode::Tab if ev.modifiers.contains(KeyModifiers::SHIFT) => KeyCode::BackTab,
            code => code,
        };
        match self {
            Key::Code(c) => !ctrl_or_alt && code == c,
            Key::Ctrl(c) => {
                ev.modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char(c)
            }
            Key::AnyChar => !ctrl_or_alt && matches!(code, KeyCode::Char(_)),
        }
    }
}

/// One entry in a context's table
#[derive(Debug, Clone, Copy)]
pub struct Binding {
    pub keys: &'static [Key],
    pub action: Action,
    /// How the key is written in the documentation
    pub label: &'static str,
    pub description: &'static str,
    /// Help bar entry as (key text, label); None keeps it out of the bar
    pub help: Option<(&'static str, &'static str)>,
}

const fn b(
    keys: &'static [Key],
    action: Action,
    label: &'static str,
    description: &'static str,
    help: Option<(&'static str, &'static str)>,
) -> Binding {
    Binding {
        keys,
        action,
        label,
        description,
        help,
    }
}

const UP: Key = Key::Code(KeyCode::Up);
const DOWN: Key = Key::Code(KeyCode::Down);
const PGUP: Key = Key::Code(KeyCode::PageUp);
const PGDN: Key = Key::Code(KeyCode::PageDown);
const HOME: Key = Key::Code(KeyCode::Home);
const END: Key = Key::Code(KeyCode::End);
const ESC: Key = Key::Code(KeyCode::Esc);
const SPACE: Key = Key::Code(KeyCode::Char(' '));

const fn ch(c: char) -> Key {
    Key::Code(KeyCode::Char(c))
}

use Action as A;

const ANYWHERE: &[Binding] = &[b(
    &[Key::Ctrl('c')],
    A::ForceQuit,
    "Ctrl-c",
    "Quit immediately, without confirmation",
    None,
)];

const GLOBAL: &[Binding] = &[
    b(
        &[Key::Code(KeyCode::Tab)],
        A::FocusNext,
        "Tab",
        "Focus next pane (Filters -> Packages -> Details)",
        None,
    ),
    b(
        &[Key::Code(KeyCode::BackTab)],
        A::FocusPrev,
        "Shift-Tab",
        "Focus previous pane",
        None,
    ),
    b(
        &[ch('s')],
        A::StartSearch,
        "s",
        "Start search",
        Some(("s", "Search")),
    ),
    b(
        &[ch('u')],
        A::Update,
        "u",
        "Run `apt update`; marks are kept",
        Some(("u", "Update")),
    ),
    b(
        &[Key::Code(KeyCode::F(2))],
        A::OpenSettings,
        "F2",
        "Settings (columns, sort order)",
        Some(("F2", "Settings")),
    ),
    b(
        &[ch('q')],
        A::Quit,
        "q",
        "Quit (asks first if changes are marked)",
        Some(("q", "Quit")),
    ),
];

const FILTERS: &[Binding] = &[
    b(
        &[UP, ch('k')],
        A::Up,
        "↑ / k",
        "Previous filter",
        Some(("↑↓", "Filter")),
    ),
    b(&[DOWN, ch('j')], A::Down, "↓ / j", "Next filter", None),
    b(&[PGUP], A::PageUp, "PgUp", "Previous filter", None),
    b(&[PGDN], A::PageDown, "PgDn", "Next filter", None),
    b(&[HOME], A::Home, "Home", "First filter", None),
    b(&[END], A::End, "End", "Last filter", None),
];

const PACKAGES: &[Binding] = &[
    b(&[UP, ch('k')], A::Up, "↑ / k", "Previous package", None),
    b(&[DOWN, ch('j')], A::Down, "↓ / j", "Next package", None),
    b(&[PGUP], A::PageUp, "PgUp", "Up one page", None),
    b(&[PGDN], A::PageDown, "PgDn", "Down one page", None),
    b(&[HOME], A::Home, "Home", "First package", None),
    b(&[END], A::End, "End", "Last package", None),
    b(
        &[SPACE],
        A::Toggle,
        "Space",
        "Toggle mark: mark for install/upgrade, or undo a mark",
        Some(("Space", "Mark")),
    ),
    b(
        &[ch('+')],
        A::MarkInstall,
        "+",
        "Mark for install/upgrade (again to unmark)",
        None,
    ),
    b(
        &[ch('-')],
        A::MarkRemove,
        "-",
        "Mark for removal (again to unmark)",
        Some(("-", "Remove")),
    ),
    b(
        &[ch('=')],
        A::MarkHold,
        "=",
        "Hold at the current state (again to release)",
        Some(("=", "Hold")),
    ),
    b(
        &[ch('v')],
        A::VisualStart,
        "v",
        "Start a visual selection",
        Some(("v", "Visual")),
    ),
    b(&[ch('c')], A::Changelog, "c", "Show changelog", None),
    b(
        &[ch('a')],
        A::ReviewChanges,
        "a",
        "Review and apply marked changes",
        Some(("a", "Apply")),
    ),
    b(
        &[ch('x')],
        A::MarkAllUpgrades,
        "x",
        "Mark all upgradable packages",
        Some(("x", "All")),
    ),
    b(
        &[ch('z')],
        A::UnmarkAll,
        "z",
        "Unmark everything",
        Some(("z", "None")),
    ),
];

const VISUAL: &[Binding] = &[
    b(
        &[UP, ch('k')],
        A::Up,
        "↑ / k",
        "Extend selection up",
        Some(("↑↓", "Extend")),
    ),
    b(
        &[DOWN, ch('j')],
        A::Down,
        "↓ / j",
        "Extend selection down",
        None,
    ),
    b(
        &[PGUP],
        A::PageUp,
        "PgUp",
        "Extend selection one page up",
        None,
    ),
    b(
        &[PGDN],
        A::PageDown,
        "PgDn",
        "Extend selection one page down",
        None,
    ),
    b(
        &[HOME],
        A::Home,
        "Home",
        "Extend selection to the first package",
        None,
    ),
    b(
        &[END],
        A::End,
        "End",
        "Extend selection to the last package",
        None,
    ),
    b(
        &[SPACE, ch('v'), ch('+')],
        A::VisualMark,
        "Space / v / +",
        "Mark the selection (or unmark it, if the first row is marked)",
        Some(("Space/v", "Mark selected")),
    ),
    b(
        &[ch('-')],
        A::VisualRemove,
        "-",
        "Mark the selection for removal",
        Some(("-", "Remove selected")),
    ),
    b(
        &[ESC],
        A::Cancel,
        "Esc",
        "Cancel the selection",
        Some(("Esc", "Cancel")),
    ),
];

const DETAILS: &[Binding] = &[
    b(
        &[UP, ch('k')],
        A::Up,
        "↑ / k",
        "Scroll up",
        Some(("↑↓", "Scroll")),
    ),
    b(&[DOWN, ch('j')], A::Down, "↓ / j", "Scroll down", None),
    b(&[PGUP], A::PageUp, "PgUp", "Scroll up one page", None),
    b(&[PGDN], A::PageDown, "PgDn", "Scroll down one page", None),
    b(&[HOME], A::Home, "Home", "Jump to top", None),
    b(&[END], A::End, "End", "Jump to bottom", None),
    b(
        &[ch(',')],
        A::PrevTab,
        ",",
        "Previous tab (Info / Deps / RDeps)",
        Some((",.", "Tab")),
    ),
    b(&[ch('.')], A::NextTab, ".", "Next tab", None),
];

const SEARCH_ACTIVE: &[Binding] = &[b(
    &[ESC],
    A::ClearSearch,
    "Esc",
    "Clear the search filter",
    Some(("Esc", "Clear search")),
)];

const SEARCH: &[Binding] = &[
    b(
        &[Key::Code(KeyCode::Enter)],
        A::SearchConfirm,
        "Enter",
        "Confirm search and return to the list",
        Some(("Enter", "Confirm")),
    ),
    b(
        &[ESC],
        A::Cancel,
        "Esc",
        "Cancel: restore the search that was active before",
        Some(("Esc", "Cancel")),
    ),
    b(
        &[Key::Code(KeyCode::Backspace)],
        A::SearchBackspace,
        "Backspace",
        "Delete the previous character",
        None,
    ),
    b(
        &[UP],
        A::Up,
        "↑",
        "Confirm search and move up the results",
        None,
    ),
    b(
        &[DOWN],
        A::Down,
        "↓",
        "Confirm search and move down the results",
        None,
    ),
    b(
        &[PGUP],
        A::PageUp,
        "PgUp",
        "Confirm search and page up the results",
        None,
    ),
    b(
        &[PGDN],
        A::PageDown,
        "PgDn",
        "Confirm search and page down the results",
        None,
    ),
    b(
        &[Key::AnyChar],
        A::SearchInput,
        "Printable characters",
        "Append to the query (results update as you type)",
        Some(("", "Type to search...")),
    ),
];

const SCROLL: [Binding; 6] = [
    b(
        &[UP, ch('k')],
        A::Up,
        "↑ / k",
        "Scroll up",
        Some(("↑↓", "Scroll")),
    ),
    b(&[DOWN, ch('j')], A::Down, "↓ / j", "Scroll down", None),
    b(&[PGUP], A::PageUp, "PgUp", "Scroll up one page", None),
    b(&[PGDN], A::PageDown, "PgDn", "Scroll down one page", None),
    b(&[HOME], A::Home, "Home", "Jump to top", None),
    b(&[END], A::End, "End", "Jump to bottom", None),
];

const MARK_CONFIRM: &[Binding] = &[
    b(
        &[SPACE],
        A::Confirm,
        "Space",
        "Confirm",
        Some(("Space", "Confirm")),
    ),
    b(
        &[ESC],
        A::Cancel,
        "Esc",
        "Cancel: undo the mark",
        Some(("Esc", "Cancel")),
    ),
    SCROLL[0],
    SCROLL[1],
    SCROLL[2],
    SCROLL[3],
    SCROLL[4],
    SCROLL[5],
];

const CHANGES: &[Binding] = &[
    b(
        &[SPACE],
        A::Confirm,
        "Space",
        "Apply all changes",
        Some(("Space", "Apply")),
    ),
    b(
        &[ESC],
        A::Cancel,
        "Esc",
        "Back to the list",
        Some(("Esc", "Cancel")),
    ),
    SCROLL[0],
    SCROLL[1],
    SCROLL[2],
    SCROLL[3],
    SCROLL[4],
    SCROLL[5],
];

const CHANGELOG: &[Binding] = &[
    b(
        &[ESC, SPACE],
        A::Cancel,
        "Esc / Space",
        "Close",
        Some(("Esc/Space", "Close")),
    ),
    SCROLL[0],
    SCROLL[1],
    SCROLL[2],
    SCROLL[3],
    SCROLL[4],
    SCROLL[5],
];

const SETTINGS: &[Binding] = &[
    b(
        &[UP, ch('k')],
        A::Up,
        "↑ / k",
        "Previous setting",
        Some(("↑↓", "Navigate")),
    ),
    b(&[DOWN, ch('j')], A::Down, "↓ / j", "Next setting", None),
    b(
        &[SPACE],
        A::Confirm,
        "Space",
        "Toggle / advance the setting",
        Some(("Space", "Toggle")),
    ),
    b(
        &[ESC],
        A::Cancel,
        "Esc",
        "Close and apply",
        Some(("Esc", "Close")),
    ),
];

const CONFIRM_EXIT: &[Binding] = &[
    b(
        &[SPACE],
        A::Confirm,
        "Space",
        "Quit without applying",
        Some(("Space", "Quit")),
    ),
    b(
        &[ESC],
        A::Cancel,
        "Esc",
        "Back to the list",
        Some(("Esc", "Cancel")),
    ),
];

const DONE: &[Binding] = &[
    b(
        &[ESC, SPACE],
        A::Cancel,
        "Esc / Space",
        "Back to the list",
        Some(("Space", "Continue")),
    ),
    SCROLL[0],
    SCROLL[1],
    SCROLL[2],
    SCROLL[3],
    SCROLL[4],
    SCROLL[5],
];

pub fn bindings(ctx: Context) -> &'static [Binding] {
    match ctx {
        Context::Anywhere => ANYWHERE,
        Context::Global => GLOBAL,
        Context::Filters => FILTERS,
        Context::Packages => PACKAGES,
        Context::Visual => VISUAL,
        Context::Details => DETAILS,
        Context::SearchActive => SEARCH_ACTIVE,
        Context::Search => SEARCH,
        Context::MarkConfirm => MARK_CONFIRM,
        Context::Changes => CHANGES,
        Context::Changelog => CHANGELOG,
        Context::Settings => SETTINGS,
        Context::ConfirmExit => CONFIRM_EXIT,
        Context::Done => DONE,
    }
}

/// The action for a key event, searching contexts in priority order
pub fn lookup(stack: &[Context], ev: &KeyEvent) -> Option<Action> {
    stack
        .iter()
        .flat_map(|&ctx| bindings(ctx))
        .find(|binding| binding.keys.iter().any(|k| k.matches(ev)))
        .map(|binding| binding.action)
}

/// Help bar text for the given contexts, in order
pub fn help_line(stack: &[Context]) -> String {
    stack
        .iter()
        .flat_map(|&ctx| bindings(ctx))
        .filter_map(|binding| binding.help)
        .map(|(key, label)| {
            if key.is_empty() {
                label.to_string()
            } else {
                format!("{key}:{label}")
            }
        })
        .collect::<Vec<_>>()
        .join(" │ ")
}

/// Documentation label of the key bound to `action` in `ctx`
pub fn key_label(ctx: Context, action: Action) -> &'static str {
    bindings(ctx)
        .iter()
        .find(|binding| binding.action == action)
        .map_or("?", |binding| binding.label)
}

/// Markdown table for one context
fn markdown_table(ctx: Context) -> String {
    let mut out = String::from("| Key | Action |\n|---|---|\n");
    for binding in bindings(ctx) {
        let keys = binding
            .label
            .split(" / ")
            .map(|k| format!("`{k}`"))
            .collect::<Vec<_>>()
            .join(" / ");
        out.push_str(&format!("| {keys} | {} |\n", binding.description));
    }
    out
}

/// Replace every `<!-- keymap:ID -->` ... `<!-- /keymap -->` block in a
/// document with the generated table for that context.
pub fn render_doc(doc: &str) -> String {
    let mut out = String::with_capacity(doc.len());
    let mut rest = doc;
    while let Some(start) = rest.find("<!-- keymap:") {
        let open_end = start + rest[start..].find("-->").map_or(0, |i| i + 3);
        let id = rest[start + "<!-- keymap:".len()..open_end - 3].trim();
        let Some(close) = rest[open_end..].find("<!-- /keymap -->") else {
            break;
        };
        out.push_str(&rest[..open_end]);
        out.push('\n');
        match Context::all().iter().find(|c| c.doc_id() == id) {
            Some(&ctx) => out.push_str(&markdown_table(ctx)),
            None => out.push_str(&format!("(unknown keymap context `{id}`)\n")),
        }
        rest = &rest[open_end + close..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, modifiers)
    }

    #[test]
    fn earlier_context_wins() {
        let space = key(KeyCode::Char(' '), KeyModifiers::NONE);
        assert_eq!(
            lookup(&[Context::Visual, Context::Packages], &space),
            Some(Action::VisualMark)
        );
        assert_eq!(lookup(&[Context::Packages], &space), Some(Action::Toggle));
    }

    #[test]
    fn ctrl_c_is_not_a_search_character() {
        let ctrl_c = key(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let stack = [Context::Anywhere, Context::Search];
        assert_eq!(lookup(&stack, &ctrl_c), Some(Action::ForceQuit));
        let c = key(KeyCode::Char('c'), KeyModifiers::NONE);
        assert_eq!(lookup(&stack, &c), Some(Action::SearchInput));
    }

    #[test]
    fn shifted_symbols_and_shift_tab() {
        let plus = key(KeyCode::Char('+'), KeyModifiers::SHIFT);
        assert_eq!(
            lookup(&[Context::Packages], &plus),
            Some(Action::MarkInstall)
        );
        let shift_tab = key(KeyCode::Tab, KeyModifiers::SHIFT);
        assert_eq!(
            lookup(&[Context::Global], &shift_tab),
            Some(Action::FocusPrev)
        );
    }

    #[test]
    fn no_key_is_bound_twice_in_one_context() {
        for &ctx in Context::all() {
            let keys: Vec<Key> = bindings(ctx)
                .iter()
                .flat_map(|b| b.keys.iter().copied())
                .collect();
            for (i, k) in keys.iter().enumerate() {
                assert!(!keys[i + 1..].contains(k), "{ctx:?} binds {k:?} twice");
            }
        }
    }

    #[test]
    fn help_line_joins_entries() {
        assert_eq!(
            help_line(&[Context::ConfirmExit]),
            "Space:Quit │ Esc:Cancel"
        );
    }

    #[test]
    fn keybindings_doc_matches_registry() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/keybindings.md");
        let doc = std::fs::read_to_string(path).unwrap();
        let rendered = render_doc(&doc);
        if std::env::var_os("SYNH8_BLESS").is_some() {
            std::fs::write(path, &rendered).unwrap();
            return;
        }
        for &ctx in Context::all() {
            assert!(
                doc.contains(&format!("<!-- keymap:{} -->", ctx.doc_id())),
                "docs/keybindings.md has no table for {ctx:?}"
            );
        }
        assert!(
            rendered == doc,
            "docs/keybindings.md is out of date with src/keymap.rs; rerun the tests with SYNH8_BLESS=1"
        );
    }
}
