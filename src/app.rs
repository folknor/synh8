//! TUI application state and logic
//!
//! This module contains TUI-specific state and acts as an adapter between
//! the core business logic (ManagerState) and the ratatui UI.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::widgets::{ListState, TableState};
use rust_apt::DepType;

use synh8::apt::fetch_changelog;
use synh8::core::{ManagerState, Toggle};
use synh8::keymap::{self, Action, Context};
use synh8::progress::{ProgressState, StdioRedirect, TuiAcquireProgress, TuiInstallProgress};
use synh8::types::*;

/// UI widget state for the main views
pub struct UiState {
    pub table_state: TableState,
    pub filter_state: ListState,
    pub focused_pane: FocusedPane,
    /// Visual mode selection range (start, end) inclusive. None when not selecting.
    pub visual_range: Option<(usize, usize)>,
    pub selection_anchor: Option<usize>,
    pub visual_mode: bool,
    /// Visible row count in the package table (set by renderer each frame)
    pub table_visible_rows: usize,
}

/// Details pane state and cached data
pub struct DetailsState {
    pub scroll: ScrollView,
    pub tab: DetailsTab,
    pub cached_deps: Vec<(DepType, String)>,
    pub cached_rdeps: Vec<(DepType, String)>,
    pub cached_pkg_name: String,
}

/// Scroll positions and content of modal views
#[derive(Default)]
pub struct ModalState {
    pub changes: ScrollView,
    pub changelog: ScrollView,
    pub changelog_title: String,
    pub changelog_content: Vec<String>,
    pub mark_preview: ScrollView,
    pub output: ScrollView,
}

/// What the event loop must do after a key was handled
pub enum Outcome {
    Continue,
    Quit,
    /// Apply the plan (needs the terminal cleared around it)
    Commit,
    /// Run `apt update` (needs the terminal cleared afterwards)
    Update,
}

/// TUI Application - wraps ManagerState with UI state
pub struct App {
    /// Core business logic (typestate package manager wrapped in enum)
    pub core: ManagerState,

    /// TUI-specific state
    pub ui: UiState,
    pub details: DetailsState,
    pub modals: ModalState,
    pub state: AppState,
    pub settings: Settings,
    pub settings_selection: usize,
    pub col_widths: ColumnWidths,
    pub status_message: String,
    pub output_lines: Vec<String>,

    /// Mark preview shown in the ShowingMarkConfirm modal
    pub mark_preview: Option<MarkPreview>,

    /// Search query that was active when search input started, restored
    /// if the input is cancelled
    search_before: Option<String>,

    /// Incremental startup warm-up: index into FilterCategory::all() for next
    /// filter to pre-warm, then search index. None = warm-up complete.
    pub warm_step: Option<usize>,
}

impl App {
    pub fn new() -> color_eyre::Result<Self> {
        let core = ManagerState::new()?;
        let mut filter_state = ListState::default();
        filter_state.select(Some(0));

        let mut app = Self {
            core,
            ui: UiState {
                table_state: TableState::default(),
                filter_state,
                focused_pane: FocusedPane::Packages,
                visual_range: None,
                selection_anchor: None,
                visual_mode: false,
                table_visible_rows: 0,
            },
            details: DetailsState {
                scroll: ScrollView::default(),
                tab: DetailsTab::Info,
                cached_deps: Vec::new(),
                cached_rdeps: Vec::new(),
                cached_pkg_name: String::new(),
            },
            modals: ModalState::default(),
            state: AppState::Listing,
            settings: Settings::default(),
            settings_selection: 0,
            col_widths: ColumnWidths::default(),
            status_message: String::new(),
            output_lines: Vec::new(),
            mark_preview: None,
            search_before: None,
            warm_step: Some(0),
        };

        app.core.set_sort(app.settings.sort);
        app.refresh_ui_state();
        app.update_status_message();
        // Filter cache and search index are warmed incrementally via
        // warm_next() during idle event loop cycles.
        Ok(app)
    }

    /// Do one unit of startup warm-up work. Called during idle event loop
    /// cycles so the UI stays responsive.
    pub fn warm_next(&mut self) {
        let Some(step) = self.warm_step else {
            return;
        };

        let filters = FilterCategory::all();
        if let Some(&filter) = filters.get(step) {
            // Only the cached filters are worth warming, and the cache is
            // bypassed while a search is active.
            let current = self.core.selected_filter();
            if filter != current
                && filter != FilterCategory::MarkedChanges
                && !self.core.has_search_results()
            {
                self.core.set_filter(filter);
                self.core.rebuild_list();
                self.core.set_filter(current);
                self.col_widths = self.core.rebuild_list();
            }
            self.warm_step = Some(step + 1);
        } else {
            self.warm_step = None;
            if let Err(e) = self.core.ensure_search_index() {
                self.status_message = format!("Failed to build search index: {e}");
            }
        }
    }

    /// Refresh UI state after core changes, preserving selection by package name
    #[hotpath::measure]
    pub fn refresh_ui_state(&mut self) {
        let selected_name = self.selected_package().map(|p| p.name.clone());
        self.col_widths = self.core.rebuild_list();
        self.restore_selection(selected_name);
        self.update_cached_deps();
    }

    /// Restore selection by package name, or reset to 0 if not found
    #[hotpath::measure]
    fn restore_selection(&mut self, package_name: Option<String>) {
        self.ui.visual_range = None;
        self.ui.selection_anchor = None;
        self.ui.visual_mode = false;

        let new_idx = package_name
            .and_then(|name| self.core.list().iter().position(|p| p.name == name))
            .unwrap_or(0);

        self.ui
            .table_state
            .select((self.core.package_count() > 0).then_some(new_idx));
        self.center_scroll_offset();
    }

    // === Accessors ===

    pub fn selected_package(&self) -> Option<&PackageInfo> {
        self.ui
            .table_state
            .selected()
            .and_then(|i| self.core.get_package(i))
    }

    fn selected_display_name(&self) -> Option<String> {
        self.selected_package()
            .map(|p| self.core.cache().display_name(&p.name).to_string())
    }

    #[hotpath::measure]
    pub fn update_cached_deps(&mut self) {
        let pkg_name = self
            .selected_package()
            .map(|p| p.name.clone())
            .unwrap_or_default();

        if pkg_name == self.details.cached_pkg_name {
            return;
        }
        self.details.cached_deps = self.core.get_dependencies(&pkg_name);
        self.details.cached_rdeps = self.core.get_reverse_dependencies(&pkg_name);
        self.details.cached_pkg_name = pkg_name;
    }

    // === Key handling ===

    /// Contexts whose bindings are active, highest priority first
    pub fn key_contexts(&self) -> Vec<Context> {
        let mut stack = vec![Context::Anywhere];
        stack.extend(self.help_contexts());
        if self.state == AppState::Listing {
            stack.push(Context::Global);
        }
        stack
    }

    /// Contexts shown in the help bar
    pub fn help_contexts(&self) -> Vec<Context> {
        match self.state {
            AppState::Listing => {
                let mut stack = Vec::new();
                if self.ui.visual_mode {
                    stack.push(Context::Visual);
                } else if self.core.has_search_results() {
                    stack.push(Context::SearchActive);
                }
                stack.push(match self.ui.focused_pane {
                    FocusedPane::Filters => Context::Filters,
                    FocusedPane::Packages => Context::Packages,
                    FocusedPane::Details => Context::Details,
                });
                stack
            }
            AppState::Searching => vec![Context::Search],
            AppState::ShowingMarkConfirm => vec![Context::MarkConfirm],
            AppState::ShowingChanges => vec![Context::Changes],
            AppState::ShowingChangelog => vec![Context::Changelog],
            AppState::ShowingSettings => vec![Context::Settings],
            AppState::ConfirmExit => vec![Context::ConfirmExit],
            AppState::Done => vec![Context::Done],
        }
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> Outcome {
        let Some(action) = keymap::lookup(&self.key_contexts(), key) else {
            return Outcome::Continue;
        };
        if action == Action::ForceQuit {
            return Outcome::Quit;
        }
        match self.state {
            AppState::Listing => return self.listing_action(action),
            AppState::Searching => self.search_action(action, key),
            AppState::ShowingMarkConfirm => match action {
                Action::Confirm => self.confirm_mark(),
                Action::Cancel => self.cancel_mark(),
                nav => scroll(&mut self.modals.mark_preview, nav),
            },
            AppState::ShowingChanges => match action {
                Action::Confirm => {
                    if self.core.can_apply() {
                        return Outcome::Commit;
                    }
                    self.status_message = self.plan_status();
                }
                Action::Cancel => {
                    self.state = AppState::Listing;
                    self.refresh_ui_state();
                }
                nav => scroll(&mut self.modals.changes, nav),
            },
            AppState::ShowingChangelog => match action {
                Action::Cancel => self.state = AppState::Listing,
                nav => scroll(&mut self.modals.changelog, nav),
            },
            AppState::ShowingSettings => match action {
                Action::Up => self.settings_selection = self.settings_selection.saturating_sub(1),
                Action::Down => {
                    self.settings_selection =
                        (self.settings_selection + 1).min(Self::settings_item_count() - 1);
                }
                Action::Confirm => self.toggle_setting(),
                Action::Cancel => {
                    self.state = AppState::Listing;
                    self.refresh_ui_state();
                }
                _ => {}
            },
            AppState::ConfirmExit => match action {
                Action::Confirm => return Outcome::Quit,
                Action::Cancel => self.state = AppState::Listing,
                _ => {}
            },
            AppState::Done => match action {
                Action::Cancel => {
                    self.state = AppState::Listing;
                    self.refresh_ui_state();
                    self.update_status_message();
                }
                nav => scroll(&mut self.modals.output, nav),
            },
        }
        Outcome::Continue
    }

    fn listing_action(&mut self, action: Action) -> Outcome {
        match action {
            Action::Quit => {
                if self.core.has_intents() {
                    self.state = AppState::ConfirmExit;
                } else {
                    return Outcome::Quit;
                }
            }
            Action::Update => return Outcome::Update,
            Action::FocusNext => self.cycle_focus(1),
            Action::FocusPrev => self.cycle_focus(2),
            Action::StartSearch => self.start_search(),
            Action::OpenSettings => {
                self.settings_selection = 0;
                self.state = AppState::ShowingSettings;
            }
            Action::ClearSearch => {
                self.core.clear_search();
                self.refresh_ui_state();
                self.update_status_message();
            }
            Action::Up
            | Action::Down
            | Action::PageUp
            | Action::PageDown
            | Action::Home
            | Action::End => {
                self.navigate(action);
            }
            Action::Toggle => self.toggle_current(),
            Action::MarkInstall => self.set_intent_current(UserIntent::Install),
            Action::MarkRemove => self.set_intent_current(UserIntent::Remove),
            Action::MarkHold => self.set_intent_current(UserIntent::Hold),
            Action::VisualStart => self.start_visual_mode(),
            Action::VisualMark => self.mark_selection(),
            Action::VisualRemove => self.remove_selection(),
            Action::Cancel => self.cancel_visual_mode(),
            Action::Changelog => self.show_changelog(),
            Action::ReviewChanges => self.show_changes_preview(),
            Action::MarkAllUpgrades => self.mark_all_upgrades(),
            Action::UnmarkAll => {
                self.core.reset();
                self.refresh_ui_state();
                self.update_status_message();
            }
            Action::PrevTab => self.switch_details_tab(2),
            Action::NextTab => self.switch_details_tab(1),
            _ => {}
        }
        Outcome::Continue
    }

    fn navigate(&mut self, action: Action) {
        match self.ui.focused_pane {
            FocusedPane::Filters => {
                let last = FilterCategory::all().len() - 1;
                let current = self.ui.filter_state.selected().unwrap_or(0);
                let target = match action {
                    Action::Up | Action::PageUp => current.saturating_sub(1),
                    Action::Down | Action::PageDown => (current + 1).min(last),
                    Action::Home => 0,
                    _ => last,
                };
                self.select_filter(target);
            }
            FocusedPane::Packages => {
                let page = self.ui.table_visible_rows.max(1) as isize;
                let count = self.core.package_count() as isize;
                let delta = match action {
                    Action::Up => -1,
                    Action::Down => 1,
                    Action::PageUp => -page,
                    Action::PageDown => page,
                    Action::Home => -count,
                    _ => count,
                };
                self.move_package_selection(delta);
            }
            FocusedPane::Details => scroll(&mut self.details.scroll, action),
        }
    }

    // === Search ===

    fn start_search(&mut self) {
        match self.core.ensure_search_index() {
            Ok(duration) if !duration.is_zero() => {
                self.status_message = format!(
                    "Search index built in {:.0}ms",
                    duration.as_secs_f64() * 1000.0
                );
            }
            Ok(_) => {}
            Err(e) => {
                self.status_message = format!("Failed to build search index: {e}");
                return;
            }
        }
        self.search_before = Some(self.core.search_query().to_string());
        self.state = AppState::Searching;
    }

    fn search_action(&mut self, action: Action, key: &KeyEvent) {
        match action {
            Action::SearchInput => {
                if let KeyCode::Char(c) = key.code {
                    let mut query = self.core.search_query().to_string();
                    query.push(c);
                    self.run_search(&query);
                }
            }
            Action::SearchBackspace => {
                let mut query = self.core.search_query().to_string();
                query.pop();
                self.run_search(&query);
            }
            Action::SearchConfirm => self.confirm_search(),
            Action::Cancel => {
                let before = self.search_before.take().unwrap_or_default();
                self.run_search(&before);
                self.state = AppState::Listing;
                self.update_status_message();
            }
            nav => {
                self.confirm_search();
                self.ui.focused_pane = FocusedPane::Packages;
                self.navigate(nav);
            }
        }
    }

    fn run_search(&mut self, query: &str) {
        if let Err(e) = self.core.set_search_query(query) {
            self.status_message = format!("Search error: {e}");
        }
        self.refresh_ui_state();
    }

    fn confirm_search(&mut self) {
        self.state = AppState::Listing;
        self.search_before = None;
        if let Some(count) = self.core.search_result_count() {
            self.status_message = format!(
                "Found {} packages matching '{}'",
                count,
                self.core.search_query()
            );
        }
    }

    // === Filter ===

    fn select_filter(&mut self, index: usize) {
        if self.ui.visual_mode {
            self.cancel_visual_mode();
        }
        self.ui.filter_state.select(Some(index));
        self.core.set_filter(FilterCategory::all()[index]);
        self.refresh_ui_state();
    }

    // === Marking ===

    /// Run a mark action and show what it did beyond the packages it targeted.
    ///
    /// `edit` applies the change to the core and returns the headline for the
    /// confirmation modal, or an error for the status bar (in which case it
    /// must not have changed anything). The modal is only shown when the plan
    /// changed beyond `acted` in a way worth confirming: new installs,
    /// removals or downgrades, or packages dropping out of the plan.
    fn run_mark_action(
        &mut self,
        acted: &[PackageId],
        edit: impl FnOnce(&mut ManagerState) -> Result<String, String>,
    ) {
        let undo = self.core.intents();
        let before = self.core.planned_ids();
        let headline = match edit(&mut self.core) {
            Ok(headline) => headline,
            Err(message) => {
                self.status_message = message;
                return;
            }
        };
        let acted: HashSet<PackageId> = acted.iter().copied().collect();
        let diff = self.core.diff_since(&before, &acted);
        let needs_confirm = !diff.dropped.is_empty()
            || diff.added.iter().any(|c| c.action != ChangeAction::Upgrade);

        self.refresh_ui_state();
        self.update_status_message();
        if !needs_confirm {
            return;
        }
        let cache = self.core.cache();
        self.mark_preview = Some(MarkPreview {
            headline,
            added: diff
                .added
                .iter()
                .map(|c| (cache.display_name_of(c.package), c.action))
                .collect(),
            dropped: diff
                .dropped
                .iter()
                .map(|&id| cache.display_name_of(id))
                .collect(),
            download_size: diff.download_size,
            undo,
        });
        self.modals.mark_preview = ScrollView::default();
        self.state = AppState::ShowingMarkConfirm;
    }

    /// Space: mark for install/upgrade, or undo whatever marked the package
    fn toggle_current(&mut self) {
        let Some(pkg) = self.selected_package() else {
            return;
        };
        let (id, status) = (pkg.id, pkg.status);
        let name = self.selected_display_name().unwrap_or_default();

        if status == PackageStatus::Installed {
            self.status_message = format!(
                "{name} is installed and up to date ('{}' removes it)",
                keymap::key_label(Context::Packages, Action::MarkRemove)
            );
            return;
        }
        self.run_mark_action(&[id], |core| match core.toggle(id) {
            Toggle::Marked => {
                let verb = if status == PackageStatus::Upgradable {
                    "upgrade"
                } else {
                    "install"
                };
                Ok(format!("Marked '{name}' for {verb}"))
            }
            Toggle::Unmarked => Ok(format!("Unmarked '{name}'")),
            Toggle::NotUnmarkable => Err(format!(
                "{name} is required by other changes - unmark the package that needs it"
            )),
        });
    }

    /// +, -, =: set an explicit intent, or clear it if it is already set
    fn set_intent_current(&mut self, intent: UserIntent) {
        let Some(pkg) = self.selected_package() else {
            return;
        };
        let id = pkg.id;
        let installed = !pkg.installed_version.is_empty();
        let up_to_date = installed && pkg.installed_version == pkg.candidate_version;
        let name = self.selected_display_name().unwrap_or_default();
        let current = self.core.intent_of(id);

        if current != Some(intent) {
            match intent {
                UserIntent::Remove if !installed => {
                    self.status_message = format!("{name} is not installed");
                    return;
                }
                UserIntent::Install if up_to_date => {
                    self.status_message = format!("{name} is installed and up to date");
                    return;
                }
                _ => {}
            }
        }

        self.run_mark_action(&[id], |core| {
            if current == Some(intent) {
                core.edit_intents([(id, None)]);
                return Ok(format!("Unmarked '{name}'"));
            }
            core.edit_intents([(id, Some(intent))]);
            Ok(match intent {
                UserIntent::Install => format!("Marked '{name}' for install/upgrade"),
                UserIntent::Remove => format!("Marked '{name}' for removal"),
                UserIntent::Hold => format!("Holding '{name}' at its current state"),
            })
        });
    }

    fn confirm_mark(&mut self) {
        self.mark_preview = None;
        self.state = AppState::Listing;
        self.update_status_message();
    }

    fn cancel_mark(&mut self) {
        if let Some(preview) = self.mark_preview.take() {
            self.core.restore_intents(preview.undo);
        }
        self.state = AppState::Listing;
        self.refresh_ui_state();
        self.update_status_message();
    }

    fn mark_all_upgrades(&mut self) {
        if self.core.mark_all_upgradable() == 0 {
            self.status_message = "No unmarked upgradable packages".to_string();
            return;
        }
        self.refresh_ui_state();
        self.show_changes_preview();
    }

    // === Visual mode ===

    fn start_visual_mode(&mut self) {
        let current_idx = self.ui.table_state.selected().unwrap_or(0);
        self.ui.visual_mode = true;
        self.ui.selection_anchor = Some(current_idx);
        self.ui.visual_range = Some((current_idx, current_idx));
        self.status_message = "-- VISUAL --".to_string();
    }

    fn update_visual_selection(&mut self) {
        if !self.ui.visual_mode {
            return;
        }
        let current_idx = self.ui.table_state.selected().unwrap_or(0);
        if let Some(anchor) = self.ui.selection_anchor {
            self.ui.visual_range = Some((anchor.min(current_idx), anchor.max(current_idx)));
        }
    }

    fn cancel_visual_mode(&mut self) {
        self.ui.visual_mode = false;
        self.ui.visual_range = None;
        self.ui.selection_anchor = None;
        self.update_status_message();
    }

    /// End visual mode, returning the anchor row and the selected rows
    fn take_selection(&mut self) -> Option<(usize, Vec<PackageInfo>)> {
        let anchor = self.ui.selection_anchor;
        let range = self.ui.visual_range;
        self.cancel_visual_mode();
        let (start, end) = range?;
        let rows = (start..=end)
            .filter_map(|i| self.core.get_package(i).cloned())
            .collect();
        Some((anchor?, rows))
    }

    fn selection_label(&self, ids: &[PackageId]) -> String {
        match ids {
            [one] => format!("'{}'", self.core.cache().display_name_of(*one)),
            many => format!("{} packages", many.len()),
        }
    }

    /// Mark the selection for install/upgrade - or unmark it, if the anchor
    /// row is marked
    fn mark_selection(&mut self) {
        let Some((anchor, rows)) = self.take_selection() else {
            return;
        };
        let anchor_marked = self
            .core
            .get_package(anchor)
            .is_some_and(|p| p.status.is_marked() || p.status == PackageStatus::Held);

        if anchor_marked {
            let ids: Vec<PackageId> = rows
                .iter()
                .filter(|p| self.core.intent_of(p.id).is_some())
                .map(|p| p.id)
                .collect();
            if ids.is_empty() {
                self.status_message = "No user-marked packages in selection".to_string();
                return;
            }
            let label = self.selection_label(&ids);
            self.run_mark_action(&ids, |core| {
                core.edit_intents(ids.iter().map(|&id| (id, None)));
                Ok(format!("Unmarked {label}"))
            });
        } else {
            let ids: Vec<PackageId> = rows
                .iter()
                .filter(|p| {
                    self.core.intent_of(p.id).is_none()
                        && matches!(
                            p.status,
                            PackageStatus::Upgradable | PackageStatus::NotInstalled
                        )
                })
                .map(|p| p.id)
                .collect();
            if ids.is_empty() {
                self.status_message = "No packages to mark in selection".to_string();
                return;
            }
            let label = self.selection_label(&ids);
            self.run_mark_action(&ids, |core| {
                core.edit_intents(ids.iter().map(|&id| (id, Some(UserIntent::Install))));
                Ok(format!("Marked {label} for install/upgrade"))
            });
        }
    }

    /// Mark every installed package in the selection for removal
    fn remove_selection(&mut self) {
        let Some((_, rows)) = self.take_selection() else {
            return;
        };
        let ids: Vec<PackageId> = rows
            .iter()
            .filter(|p| {
                !p.installed_version.is_empty()
                    && self.core.intent_of(p.id) != Some(UserIntent::Remove)
            })
            .map(|p| p.id)
            .collect();
        if ids.is_empty() {
            self.status_message = "No installed packages to remove in selection".to_string();
            return;
        }
        let label = self.selection_label(&ids);
        self.run_mark_action(&ids, |core| {
            core.edit_intents(ids.iter().map(|&id| (id, Some(UserIntent::Remove))));
            Ok(format!("Marked {label} for removal"))
        });
    }

    // === Navigation ===

    fn move_package_selection(&mut self, delta: isize) {
        let count = self.core.package_count();
        if count == 0 {
            return;
        }
        let current = self.ui.table_state.selected().unwrap_or(0);
        let new_idx = current.saturating_add_signed(delta).min(count - 1);
        self.ui.table_state.select(Some(new_idx));
        self.center_scroll_offset();
        self.details.scroll.home();
        self.update_cached_deps();
        self.update_visual_selection();
    }

    /// Set the table viewport offset so the selected row stays vertically centered.
    ///
    /// When the selection is in the top half of the list or the bottom half,
    /// the highlight moves normally (can't center without content above/below).
    /// In between, the list scrolls under a pinned highlight at the midpoint.
    pub fn center_scroll_offset(&mut self) {
        let visible = self.ui.table_visible_rows;
        if visible == 0 {
            return;
        }
        let selected = self.ui.table_state.selected().unwrap_or(0);
        let max_offset = self.core.package_count().saturating_sub(visible);
        *self.ui.table_state.offset_mut() = selected.saturating_sub(visible / 2).min(max_offset);
    }

    /// Step the details tab forward by `steps` (mod 3)
    fn switch_details_tab(&mut self, steps: usize) {
        const TABS: [DetailsTab; 3] = [
            DetailsTab::Info,
            DetailsTab::Dependencies,
            DetailsTab::ReverseDeps,
        ];
        let current = TABS
            .iter()
            .position(|&t| t == self.details.tab)
            .unwrap_or(0);
        self.details.tab = TABS[(current + steps) % TABS.len()];
        self.details.scroll.home();
    }

    /// Step pane focus forward by `steps` (mod 3)
    fn cycle_focus(&mut self, steps: usize) {
        const PANES: [FocusedPane; 3] = [
            FocusedPane::Filters,
            FocusedPane::Packages,
            FocusedPane::Details,
        ];
        let current = PANES
            .iter()
            .position(|&p| p == self.ui.focused_pane)
            .unwrap_or(0);
        self.ui.focused_pane = PANES[(current + steps) % PANES.len()];
    }

    // === Modals ===

    fn show_changelog(&mut self) {
        let Some(name) = self.selected_display_name() else {
            self.status_message = "No package selected".to_string();
            return;
        };
        self.modals.changelog_content = match fetch_changelog(&name) {
            Ok(lines) => lines,
            Err(e) => vec![e.to_string()],
        };
        self.modals.changelog_title = name;
        self.modals.changelog = ScrollView::default();
        self.state = AppState::ShowingChangelog;
    }

    fn toggle_setting(&mut self) {
        let all_cols = Column::all();
        let col_count = all_cols.len();
        if let Some(&col) = all_cols.get(self.settings_selection) {
            if !self.settings.visible_columns.remove(&col) {
                self.settings.visible_columns.insert(col);
            }
            return;
        }
        if self.settings_selection == col_count {
            let all = SortBy::all();
            let idx = all
                .iter()
                .position(|&s| s == self.settings.sort.sort_by)
                .unwrap_or(0);
            self.settings.sort.sort_by = all[(idx + 1) % all.len()];
        } else {
            self.settings.sort.ascending = !self.settings.sort.ascending;
        }
        self.core.set_sort(self.settings.sort);
    }

    pub fn settings_item_count() -> usize {
        Column::all().len() + 2
    }

    fn show_changes_preview(&mut self) {
        if self.core.has_intents() {
            self.state = AppState::ShowingChanges;
            self.modals.changes = ScrollView::default();
            self.status_message = self.plan_status();
        } else {
            self.status_message = "No changes to apply".to_string();
        }
    }

    // === Status message ===

    /// Plan problems, or the default status line
    fn plan_status(&self) -> String {
        if let Some(problems) = self.core.plan_problems()
            && let Some(summary) = problems.summary()
        {
            return if problems.errors.is_empty() {
                format!("Warning: {summary}")
            } else {
                format!("Cannot apply: {summary}")
            };
        }
        let upgradable = self.core.upgradable_count();
        match self.core.intent_count() {
            0 => format!("{upgradable} packages upgradable"),
            n => format!(
                "{n} packages marked | {upgradable} upgradable | Press '{}' to apply",
                keymap::key_label(Context::Packages, Action::ReviewChanges)
            ),
        }
    }

    pub fn update_status_message(&mut self) {
        let apt_errors = self.core.take_apt_errors();
        self.status_message = if apt_errors.is_empty() {
            self.plan_status()
        } else {
            format!("APT error: {}", apt_errors.join("; "))
        };
    }

    // === System operations ===

    /// Commit the plan with live progress. The progress modal draws on its
    /// own `/dev/tty` terminal while stdout/stderr are captured, and the
    /// captured output is shown afterwards in the Done view. Failures are
    /// reported there, not returned.
    pub fn commit_changes_live(&mut self) {
        self.state = AppState::Done;
        self.output_lines.clear();
        self.modals.output = ScrollView::default();

        let progress_state = match ProgressState::new("Applying Changes") {
            Ok(state) => Rc::new(RefCell::new(state)),
            Err(e) => {
                self.status_message = format!("Cannot open /dev/tty for progress: {e}");
                return;
            }
        };
        let mut acquire_progress = rust_apt::progress::AcquireProgress::new(
            TuiAcquireProgress::new(Rc::clone(&progress_state)),
        );
        let mut install_progress = rust_apt::progress::InstallProgress::new(
            TuiInstallProgress::new(Rc::clone(&progress_state)),
        );

        // Keep existing config files without prompting: dpkg cannot ask,
        // since its output is being captured.
        rust_apt::config::Config::new()
            .set_vector("Dpkg::Options", &vec!["--force-confdef", "--force-confold"]);

        let redirect = match StdioRedirect::capture() {
            Ok(redirect) => redirect,
            Err(e) => {
                self.status_message = format!("Cannot capture apt output: {e}");
                return;
            }
        };
        let result = self
            .core
            .commit(&mut acquire_progress, &mut install_progress);
        let (output, restored) = redirect.finish();

        self.output_lines = output;
        let progress_errors = progress_state.borrow().errors().to_vec();
        if !progress_errors.is_empty() {
            self.output_lines.push(String::new());
            self.output_lines.push("Errors:".to_string());
            self.output_lines.extend(progress_errors);
        }
        if let Err(e) = restored {
            self.output_lines
                .push(format!("Failed to restore stdout/stderr: {e}"));
        }

        self.status_message = match result {
            Ok(()) => "Changes applied successfully.".to_string(),
            Err(e) => format!("Error: {e}"),
        };
    }

    /// Run `apt update` with live progress. Marks are carried across.
    pub fn update_packages_live(&mut self) {
        let progress_state = match ProgressState::new("Updating Package Lists") {
            Ok(state) => Rc::new(RefCell::new(state)),
            Err(e) => {
                self.status_message = format!("Cannot open /dev/tty for progress: {e}");
                return;
            }
        };
        let mut acquire_progress = rust_apt::progress::AcquireProgress::new(
            TuiAcquireProgress::new(Rc::clone(&progress_state)),
        );

        let (lost, result) = self.core.update(&mut acquire_progress);
        self.refresh_ui_state();
        self.update_status_message();
        if !lost.is_empty() {
            self.status_message = format!(
                "Dropped marks for packages that no longer exist: {}",
                lost.join(", ")
            );
        }
        if let Err(e) = result {
            self.status_message = format!("Update failed: {e}");
        }
    }
}

/// Apply a navigation action to a scroll view (other actions are ignored)
fn scroll(view: &mut ScrollView, action: Action) {
    match action {
        Action::Up => view.scroll_by(-1),
        Action::Down => view.scroll_by(1),
        Action::PageUp => view.page_by(-1),
        Action::PageDown => view.page_by(1),
        Action::Home => view.home(),
        Action::End => view.end(),
        _ => {}
    }
}
