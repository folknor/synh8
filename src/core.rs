//! Core business logic - Typestate Package Manager
//!
//! - `user_intent: HashMap<PackageId, UserIntent>` is the single source of truth
//! - APT marks are derived from intent via `plan()`
//! - `PackageManager<S>` enforces the Clean -> Dirty -> Planned transitions at
//!   compile time; `ManagerState` holds whichever one is current for the TUI
//!   and always re-plans after an intent edit, so outside this module the
//!   state is only ever Clean (no intents) or Planned.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use color_eyre::Result;
use color_eyre::eyre::eyre;
use rust_apt::DepType;
use rust_apt::cache::PackageSort;

use crate::apt::{AptCache, plan_problems};
use crate::search::SearchIndex;
use crate::types::*;
use crate::version;

// ============================================================================
// Shared State (across all typestate variants)
// ============================================================================

/// Search state management
#[derive(Default)]
struct SearchState {
    index: Option<SearchIndex>,
    query: String,
    results: Option<HashSet<String>>,
}

/// State shared across all PackageManager states.
/// Fields are private to this module so only the typestate API mutates them.
struct SharedState {
    cache: AptCache,
    user_intent: HashMap<PackageId, UserIntent>,
    /// Packages currently protected in the APT resolver (see `plan()`)
    protected: Vec<PackageId>,
    search: SearchState,
    list: Vec<PackageInfo>,
    /// Per-filter memoization of the extracted list, with BASE statuses
    /// (before the intent/plan overlay). Search and sort are applied on top.
    /// MarkedChanges is never cached: it is built from the (small) set of
    /// intents and planned changes. Cleared when the cache is reloaded.
    filter_cache: HashMap<FilterCategory, (Vec<PackageInfo>, ColumnWidths)>,
    upgradable_count: usize,
    installed_count: usize,
    total_count: usize,
    selected_filter: FilterCategory,
    sort: SortSettings,
    /// APT failures from operations that have no plan to report them in
    /// (resetting marks). Drained by the UI via `take_apt_errors()`.
    apt_errors: Vec<String>,
}

impl SharedState {
    fn new(cache: AptCache) -> Self {
        let mut shared = Self {
            cache,
            user_intent: HashMap::new(),
            protected: Vec::new(),
            search: SearchState::default(),
            list: Vec::new(),
            filter_cache: HashMap::new(),
            upgradable_count: 0,
            installed_count: 0,
            total_count: 0,
            selected_filter: FilterCategory::Upgradable,
            sort: SortSettings::default(),
            apt_errors: Vec::new(),
        };
        shared.compute_cache_counts();
        shared
    }

    /// Everything derived from a cache generation, discarded after a reload.
    /// Intents are cleared too: their ids belong to the old generation.
    fn forget_generation(&mut self) {
        self.user_intent.clear();
        self.protected.clear();
        self.filter_cache.clear();
        self.search = SearchState::default();
        self.compute_cache_counts();
    }

    /// Compute and cache package counts from the APT cache
    fn compute_cache_counts(&mut self) {
        self.upgradable_count = 0;
        self.installed_count = 0;
        self.total_count = 0;

        for pkg in self.cache.packages(&PackageSort::default()) {
            self.total_count += 1;
            if pkg.is_installed() {
                self.installed_count += 1;
                if pkg.is_upgradable() {
                    self.upgradable_count += 1;
                }
            }
        }
    }

    /// Ids that belong in the MarkedChanges filter: every intent plus
    /// everything in the plan.
    fn marked_ids(&self, planned: Option<&[PlannedChange]>) -> HashSet<PackageId> {
        let mut ids: HashSet<PackageId> = self.user_intent.keys().copied().collect();
        ids.extend(planned.unwrap_or_default().iter().map(|c| c.package));
        ids
    }

    /// Rebuild the package list for the current filter and search, then
    /// overlay intents and planned changes and sort.
    fn rebuild_list(&mut self, planned: Option<&[PlannedChange]>) -> ColumnWidths {
        let filter = self.selected_filter;
        let has_search = self.search.results.is_some();

        let cached = if filter == FilterCategory::MarkedChanges || has_search {
            None
        } else {
            self.filter_cache.get(&filter).cloned()
        };

        let (mut list, col_widths) = match cached {
            Some(hit) => hit,
            None => {
                let ids = if filter == FilterCategory::MarkedChanges {
                    // Small set: look the packages up directly instead of
                    // walking the whole cache.
                    let search_results = self.search.results.as_ref();
                    self.marked_ids(planned)
                        .into_iter()
                        .filter(|&id| {
                            search_results.is_none_or(|r| {
                                self.cache
                                    .get_by_id(id)
                                    .is_some_and(|pkg| r.contains(pkg.name()))
                            })
                        })
                        .collect()
                } else {
                    self.matching_ids(|pkg| match filter {
                        FilterCategory::Upgradable => pkg.is_upgradable(),
                        FilterCategory::Installed => pkg.is_installed(),
                        FilterCategory::NotInstalled => !pkg.is_installed(),
                        FilterCategory::All | FilterCategory::MarkedChanges => true,
                    })
                };
                let list: Vec<PackageInfo> = ids
                    .into_iter()
                    .filter_map(|id| {
                        self.cache
                            .get_by_id(id)
                            .and_then(|pkg| self.cache.extract_package_info(&pkg))
                    })
                    .collect();
                let col_widths = self.column_widths(&list);
                if !has_search && filter != FilterCategory::MarkedChanges {
                    self.filter_cache
                        .insert(filter, (list.clone(), col_widths.clone()));
                }
                (list, col_widths)
            }
        };

        overlay_statuses(&mut list, &self.user_intent, planned);
        self.list = list;
        self.sort_list();
        col_widths
    }

    /// Ids of packages passing `keep` and the active search
    fn matching_ids(&self, keep: impl Fn(&rust_apt::Package) -> bool) -> Vec<PackageId> {
        let sort = if self.selected_filter == FilterCategory::Upgradable {
            PackageSort::default().upgradable()
        } else {
            PackageSort::default()
        };
        let search_results = self.search.results.as_ref();
        self.cache
            .packages(&sort)
            .filter(|pkg| search_results.is_none_or(|r| r.contains(pkg.name())))
            .filter(|pkg| keep(pkg))
            .filter_map(|pkg| self.cache.id_of(&pkg))
            .collect()
    }

    fn column_widths(&self, list: &[PackageInfo]) -> ColumnWidths {
        let mut w = ColumnWidths::default();
        for pkg in list {
            w.name = w.name.max(self.cache.display_name(&pkg.name).len() as u16);
            w.section = w.section.max(pkg.section.len() as u16);
            w.installed = w.installed.max(pkg.installed_version.len() as u16);
            w.candidate = w.candidate.max(pkg.candidate_version.len() as u16);
        }
        w
    }

    fn sort_list(&mut self) {
        let SortSettings { sort_by, ascending } = self.sort;
        self.list.sort_by(|a, b| {
            let ord = compare_packages(a, b, sort_by);
            if ascending { ord } else { ord.reverse() }
        });
    }

    fn set_search_query(&mut self, query: &str) -> Result<()> {
        self.search.query = query.to_string();
        if query.is_empty() {
            self.search.results = None;
        } else {
            let index = match self.search.index.take() {
                Some(index) => index,
                None => SearchIndex::build(&self.cache)?,
            };
            let results = index.search(query);
            self.search.index = Some(index);
            self.search.results = Some(results?);
        }
        Ok(())
    }

    /// Clear APT marks and resolver protection. Errors are kept for the UI.
    fn clear_apt_state(&mut self) -> Result<(), String> {
        for id in self.protected.drain(..) {
            self.cache.unprotect(id);
        }
        self.cache.clear_all_marks()
    }
}

/// Order two packages by a sort column, falling back to name.
fn compare_packages(a: &PackageInfo, b: &PackageInfo, sort_by: SortBy) -> Ordering {
    let primary = match sort_by {
        SortBy::Name => Ordering::Equal,
        SortBy::Section => a.section.cmp(&b.section),
        SortBy::InstalledVersion => compare_versions(&a.installed_version, &b.installed_version),
        SortBy::CandidateVersion => compare_versions(&a.candidate_version, &b.candidate_version),
    };
    primary.then_with(|| a.name.cmp(&b.name))
}

/// Debian version order, with "no version" sorting first.
fn compare_versions(a: &str, b: &str) -> Ordering {
    match (a.is_empty(), b.is_empty()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Less,
        (false, true) => Ordering::Greater,
        (false, false) => version::compare(a, b),
    }
}

/// Turn base statuses into display statuses: intents first, then the plan
/// (which wins, since it is what will actually happen).
fn overlay_statuses(
    list: &mut [PackageInfo],
    user_intent: &HashMap<PackageId, UserIntent>,
    planned: Option<&[PlannedChange]>,
) {
    let planned: HashMap<PackageId, ChangeAction> = planned
        .unwrap_or_default()
        .iter()
        .map(|c| (c.package, c.action))
        .collect();
    for info in list.iter_mut() {
        if let Some(&intent) = user_intent.get(&info.id) {
            info.status = match intent {
                UserIntent::Install if info.status == PackageStatus::Upgradable => {
                    PackageStatus::MarkedForUpgrade
                }
                UserIntent::Install => PackageStatus::MarkedForInstall,
                UserIntent::Remove => PackageStatus::MarkedForRemove,
                UserIntent::Hold => PackageStatus::Held,
            };
        }
        if let Some(action) = planned.get(&info.id) {
            info.status = action.status();
        }
    }
}

/// Download size and installed-size delta of one planned change.
/// `new` is the (download, installed) size of the version being installed,
/// `old_installed` the installed size of the version on disk.
fn change_sizes(
    action: ChangeAction,
    new: Option<(u64, u64)>,
    old_installed: Option<u64>,
) -> (u64, i64) {
    let old = old_installed.unwrap_or(0) as i64;
    let (download, new_installed) = new.map_or((0, 0), |(d, i)| (d, i as i64));
    match action {
        ChangeAction::Remove => (0, -old),
        ChangeAction::Install => (download, new_installed),
        ChangeAction::Upgrade | ChangeAction::Downgrade => (download, new_installed - old),
        ChangeAction::Reinstall => (download, 0),
    }
}

// ============================================================================
// Typestate Package Manager
// ============================================================================

/// Package manager with compile-time state tracking
pub struct PackageManager<S> {
    shared: SharedState,
    state: S,
}

impl<S> PackageManager<S> {
    fn with_state<T>(self, state: T) -> PackageManager<T> {
        PackageManager {
            shared: self.shared,
            state,
        }
    }
}

// Clean state - no user intents, no APT marks
impl PackageManager<Clean> {
    /// Create a new PackageManager in Clean state
    pub fn new() -> Result<Self> {
        let mut mgr = Self {
            shared: SharedState::new(AptCache::new()?),
            state: Clean,
        };
        mgr.shared.rebuild_list(None);
        Ok(mgr)
    }

    /// Start editing intents
    pub fn edit(self) -> PackageManager<Dirty> {
        self.with_state(Dirty)
    }
}

// Dirty state - intents edited, APT marks not yet derived from them
impl PackageManager<Dirty> {
    pub fn set_intent(mut self, id: PackageId, intent: UserIntent) -> Self {
        self.shared.user_intent.insert(id, intent);
        self
    }

    pub fn clear_intent(mut self, id: PackageId) -> Self {
        self.shared.user_intent.remove(&id);
        self
    }

    pub fn has_intents(&self) -> bool {
        !self.shared.user_intent.is_empty()
    }

    /// Drop all intents and APT marks, returning to Clean state
    pub fn reset(mut self) -> PackageManager<Clean> {
        self.shared.user_intent.clear();
        if let Err(e) = self.shared.clear_apt_state() {
            self.shared.apt_errors.push(e);
        }
        self.with_state(Clean)
    }

    /// Derive APT marks from intents and resolve, transitioning to Planned.
    ///
    /// Every intent is protected in the resolver, as apt-get protects the
    /// packages named on its command line: the resolver must satisfy them or
    /// report an error, never quietly undo them.
    #[hotpath::measure]
    pub fn plan(mut self) -> PackageManager<Planned> {
        let mut problems = PlanProblems::default();
        let shared = &mut self.shared;

        if let Err(e) = shared.clear_apt_state() {
            problems.errors.push(e);
        }

        // Holds and removals first, so installs are resolved around them.
        let mut intents: Vec<(PackageId, UserIntent)> =
            shared.user_intent.iter().map(|(&id, &i)| (id, i)).collect();
        intents.sort_by_key(|&(id, intent)| {
            let rank = match intent {
                UserIntent::Hold => 0,
                UserIntent::Remove => 1,
                UserIntent::Install => 2,
            };
            (rank, id.index())
        });
        for (id, intent) in intents {
            match intent {
                UserIntent::Install => shared.cache.mark_install(id),
                UserIntent::Remove => shared.cache.mark_delete(id),
                UserIntent::Hold => shared.cache.mark_keep(id),
            }
            shared.cache.protect(id);
            shared.protected.push(id);
        }

        if let Err(e) = shared.cache.resolve() {
            let resolved = plan_problems(&e);
            problems.errors.extend(resolved.errors);
            problems.warnings.extend(resolved.warnings);
            if problems.errors.is_empty() && problems.warnings.is_empty() {
                problems
                    .errors
                    .push("Dependency resolution failed (no details available)".to_string());
            }
        }

        let mut changes = Vec::new();
        for pkg in shared.cache.get_changes() {
            let Some(action) = ChangeAction::from_marked(pkg.marked()) else {
                continue;
            };
            let Some(id) = shared.cache.id_of(&pkg) else {
                continue;
            };
            let reason = if shared.user_intent.contains_key(&id) {
                ChangeReason::UserRequested
            } else {
                ChangeReason::Dependency
            };
            let new = pkg.candidate().map(|c| (c.size(), c.installed_size()));
            let old = pkg.installed().map(|v| v.installed_size());
            let (download_size, size_change) = change_sizes(action, new, old);
            changes.push(PlannedChange {
                package: id,
                action,
                reason,
                download_size,
                size_change,
            });
        }

        let planned = Planned {
            download_size: changes.iter().map(|c| c.download_size).sum(),
            install_size_change: changes.iter().map(|c| c.size_change).sum(),
            changes,
            problems,
        };
        self.with_state(planned)
    }
}

// Planned state - dependencies resolved, changeset computed
impl PackageManager<Planned> {
    /// Go back to editing intents (discards the plan)
    pub fn modify(self) -> PackageManager<Dirty> {
        self.with_state(Dirty)
    }

    /// Commit the plan. The cache is reloaded afterwards whatever happens,
    /// which starts a new id generation and so ends in Clean state. The one
    /// exception is a failure before anything was attempted (the cache could
    /// not be opened), which leaves the plan in place.
    pub fn commit(
        mut self,
        acquire_progress: &mut rust_apt::progress::AcquireProgress,
        install_progress: &mut rust_apt::progress::InstallProgress,
    ) -> (ManagerState, Result<()>) {
        if let Some(summary) = self.state.problems.errors.first() {
            let err = eyre!("the plan has unresolved problems: {summary}");
            return (ManagerState::Planned(self), Err(err));
        }
        let generation = self.shared.cache.generation();
        let result = self.shared.cache.commit(acquire_progress, install_progress);
        if self.shared.cache.generation() == generation {
            return (ManagerState::Planned(self), result);
        }
        self.shared.forget_generation();
        let mut clean = self.with_state(Clean);
        clean.shared.rebuild_list(None);
        (ManagerState::Clean(clean), result)
    }
}

// ============================================================================
// Manager Wrapper (for TUI that can't hold consuming-self types)
// ============================================================================

/// Outcome of toggling a package with Space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// The package gained an Install intent
    Marked,
    /// The package's intent, or the intents that pulled it in, were removed
    Unmarked,
    /// The package is in the plan but no intent that explains it was found
    NotUnmarkable,
}

/// Result of `ManagerState::diff_since()`: how the plan changed around the
/// packages an action targeted.
#[derive(Debug, Default)]
pub struct PlanDiff {
    /// Changes that are newly planned, excluding the acted-on packages
    pub added: Vec<PlannedChange>,
    /// Packages no longer planned, excluding the acted-on packages
    pub dropped: Vec<PackageId>,
    /// Download size of the newly planned changes and the acted-on packages
    pub download_size: u64,
}

/// Wrapper enum that allows mutable access without consuming self
#[derive(Default)]
pub enum ManagerState {
    Clean(PackageManager<Clean>),
    Dirty(PackageManager<Dirty>),
    Planned(PackageManager<Planned>),
    /// Placeholder that exists only inside `transition()`. Every transition
    /// is infallible, so it is never observable from outside this module
    /// unless a transition panics - which ends the program.
    #[default]
    Transitioning,
}

impl ManagerState {
    /// Create a new manager in Clean state
    pub fn new() -> Result<Self> {
        Ok(ManagerState::Clean(PackageManager::new()?))
    }

    fn transition(&mut self, f: impl FnOnce(ManagerState) -> ManagerState) {
        let old = std::mem::take(self);
        *self = f(old);
    }

    fn shared(&self) -> &SharedState {
        match self {
            ManagerState::Clean(m) => &m.shared,
            ManagerState::Dirty(m) => &m.shared,
            ManagerState::Planned(m) => &m.shared,
            ManagerState::Transitioning => unreachable!("Transitioning state observed"),
        }
    }

    fn shared_mut(&mut self) -> &mut SharedState {
        match self {
            ManagerState::Clean(m) => &mut m.shared,
            ManagerState::Dirty(m) => &mut m.shared,
            ManagerState::Planned(m) => &mut m.shared,
            ManagerState::Transitioning => unreachable!("Transitioning state observed"),
        }
    }

    fn planned(&self) -> Option<&Planned> {
        match self {
            ManagerState::Planned(m) => Some(&m.state),
            _ => None,
        }
    }

    // === Plan accessors ===

    /// The planned changes (only in Planned state)
    pub fn planned_changes(&self) -> Option<&[PlannedChange]> {
        self.planned().map(|p| p.changes.as_slice())
    }

    /// Ids of every planned change
    pub fn planned_ids(&self) -> HashSet<PackageId> {
        self.planned_changes()
            .unwrap_or_default()
            .iter()
            .map(|c| c.package)
            .collect()
    }

    /// Resolver problems of the current plan
    pub fn plan_problems(&self) -> Option<&PlanProblems> {
        self.planned().map(|p| &p.problems)
    }

    /// True if there is a plan with no errors and at least one change
    pub fn can_apply(&self) -> bool {
        self.planned()
            .is_some_and(|p| p.problems.errors.is_empty() && !p.changes.is_empty())
    }

    pub fn download_size(&self) -> u64 {
        self.planned().map_or(0, |p| p.download_size)
    }

    pub fn install_size_change(&self) -> i64 {
        self.planned().map_or(0, |p| p.install_size_change)
    }

    // === Intent accessors ===

    pub fn has_intents(&self) -> bool {
        !self.shared().user_intent.is_empty()
    }

    pub fn intent_count(&self) -> usize {
        self.shared().user_intent.len()
    }

    pub fn intent_of(&self, id: PackageId) -> Option<UserIntent> {
        self.shared().user_intent.get(&id).copied()
    }

    /// Snapshot of all intents, for undo
    pub fn intents(&self) -> IntentSnapshot {
        self.shared().user_intent.clone()
    }

    /// APT failures that had no plan to report them in
    pub fn take_apt_errors(&mut self) -> Vec<String> {
        std::mem::take(&mut self.shared_mut().apt_errors)
    }

    // === List accessors ===

    pub fn list(&self) -> &[PackageInfo] {
        self.shared().list.as_slice()
    }

    pub fn get_package(&self, index: usize) -> Option<&PackageInfo> {
        self.shared().list.get(index)
    }

    pub fn package_count(&self) -> usize {
        self.shared().list.len()
    }

    pub fn selected_filter(&self) -> FilterCategory {
        self.shared().selected_filter
    }

    pub fn upgradable_count(&self) -> usize {
        self.shared().upgradable_count
    }

    /// Number of packages a filter shows (before search)
    pub fn filter_count(&self, filter: FilterCategory) -> usize {
        let shared = self.shared();
        match filter {
            FilterCategory::Upgradable => shared.upgradable_count,
            FilterCategory::MarkedChanges => shared.marked_ids(self.planned_changes()).len(),
            FilterCategory::Installed => shared.installed_count,
            FilterCategory::NotInstalled => shared.total_count - shared.installed_count,
            FilterCategory::All => shared.total_count,
        }
    }

    /// Get reference to the APT cache for name lookups
    pub fn cache(&self) -> &AptCache {
        &self.shared().cache
    }

    pub fn get_dependencies(&self, fullname: &str) -> Vec<(DepType, String)> {
        self.cache().get_dependencies(fullname)
    }

    pub fn get_reverse_dependencies(&self, fullname: &str) -> Vec<(DepType, String)> {
        self.cache().get_reverse_dependencies(fullname)
    }

    // === List operations ===

    /// Rebuild the list for the current filter, search and plan
    #[hotpath::measure]
    pub fn rebuild_list(&mut self) -> ColumnWidths {
        match self {
            ManagerState::Clean(m) => m.shared.rebuild_list(None),
            ManagerState::Dirty(m) => m.shared.rebuild_list(None),
            ManagerState::Planned(m) => m.shared.rebuild_list(Some(&m.state.changes)),
            ManagerState::Transitioning => unreachable!("Transitioning state observed"),
        }
    }

    /// Set the filter category without rebuilding the list.
    /// Caller is responsible for calling rebuild_list() afterwards.
    pub fn set_filter(&mut self, filter: FilterCategory) {
        self.shared_mut().selected_filter = filter;
    }

    /// Update sort settings and re-sort the current list
    pub fn set_sort(&mut self, sort: SortSettings) {
        let shared = self.shared_mut();
        shared.sort = sort;
        shared.sort_list();
    }

    // === Search ===

    /// Build the search index if it does not exist yet. Returns the build
    /// time, or zero if it already existed.
    pub fn ensure_search_index(&mut self) -> Result<std::time::Duration> {
        let shared = self.shared_mut();
        if shared.search.index.is_some() {
            return Ok(std::time::Duration::ZERO);
        }
        let start = std::time::Instant::now();
        shared.search.index = Some(SearchIndex::build(&shared.cache)?);
        Ok(start.elapsed())
    }

    /// Set the search query and run it (does not rebuild the list)
    pub fn set_search_query(&mut self, query: &str) -> Result<()> {
        self.shared_mut().set_search_query(query)
    }

    pub fn search_query(&self) -> &str {
        &self.shared().search.query
    }

    pub fn search_result_count(&self) -> Option<usize> {
        self.shared().search.results.as_ref().map(HashSet::len)
    }

    pub fn has_search_results(&self) -> bool {
        self.shared().search.results.is_some()
    }

    pub fn clear_search(&mut self) {
        let search = &mut self.shared_mut().search;
        search.query.clear();
        search.results = None;
    }

    // === Intent editing ===

    /// Apply intent edits (`None` clears an intent) and re-plan once.
    /// Ends in Planned state, or Clean if no intents remain.
    pub fn edit_intents(
        &mut self,
        edits: impl IntoIterator<Item = (PackageId, Option<UserIntent>)>,
    ) {
        self.transition(|state| {
            let mut dirty = match state {
                ManagerState::Clean(m) => m.edit(),
                ManagerState::Dirty(m) => m,
                ManagerState::Planned(m) => m.modify(),
                ManagerState::Transitioning => unreachable!("Transitioning state observed"),
            };
            for (id, intent) in edits {
                dirty = match intent {
                    Some(intent) => dirty.set_intent(id, intent),
                    None => dirty.clear_intent(id),
                };
            }
            if dirty.has_intents() {
                ManagerState::Planned(dirty.plan())
            } else {
                ManagerState::Clean(dirty.reset())
            }
        });
    }

    /// Replace all intents with a snapshot and re-plan
    pub fn restore_intents(&mut self, snapshot: IntentSnapshot) {
        let mut edits: Vec<(PackageId, Option<UserIntent>)> = self
            .shared()
            .user_intent
            .keys()
            .filter(|id| !snapshot.contains_key(id))
            .map(|&id| (id, None))
            .collect();
        edits.extend(snapshot.into_iter().map(|(id, i)| (id, Some(i))));
        self.edit_intents(edits);
    }

    /// Drop every intent
    pub fn reset(&mut self) {
        self.transition(|state| match state {
            ManagerState::Clean(m) => ManagerState::Clean(m),
            ManagerState::Dirty(m) => ManagerState::Clean(m.reset()),
            ManagerState::Planned(m) => ManagerState::Clean(m.modify().reset()),
            ManagerState::Transitioning => unreachable!("Transitioning state observed"),
        });
    }

    /// Toggle a package with Space: clear its intent if it has one; if it is
    /// only in the plan as a dependency, clear the intents that pulled it in;
    /// otherwise mark it for install.
    #[hotpath::measure]
    pub fn toggle(&mut self, id: PackageId) -> Toggle {
        if self.intent_of(id).is_some() {
            self.edit_intents([(id, None)]);
            return Toggle::Unmarked;
        }
        if !self.planned_ids().contains(&id) {
            self.edit_intents([(id, Some(UserIntent::Install))]);
            return Toggle::Marked;
        }
        let owners = self.intents_requiring(id);
        if owners.is_empty() {
            return Toggle::NotUnmarkable;
        }
        let undo = self.intents();
        self.edit_intents(owners.into_iter().map(|o| (o, None)));
        if self.planned_ids().contains(&id) {
            self.restore_intents(undo);
            return Toggle::NotUnmarkable;
        }
        Toggle::Unmarked
    }

    /// Intents that (transitively) cause a planned dependency change.
    ///
    /// A dependency install/upgrade is caused by Install intents whose
    /// candidate dependencies reach it through other planned changes; a
    /// dependency removal is caused by Remove intents that its installed
    /// dependencies reach through other planned removals. Virtual packages
    /// count via their providers. Changes forced by Conflicts/Breaks are not
    /// traced.
    fn intents_requiring(&self, target: PackageId) -> Vec<PackageId> {
        let Some(changes) = self.planned_changes() else {
            return Vec::new();
        };
        let actions: HashMap<PackageId, ChangeAction> =
            changes.iter().map(|c| (c.package, c.action)).collect();
        let Some(&target_action) = actions.get(&target) else {
            return Vec::new();
        };
        let cache = self.cache();
        let intents = &self.shared().user_intent;

        if target_action == ChangeAction::Remove {
            // Walk the target's installed dependencies through removals.
            let reachable = reachable_from(target, |node| {
                cache
                    .dependency_ids(node, true)
                    .into_iter()
                    .filter(|d| actions.get(d) == Some(&ChangeAction::Remove))
                    .collect()
            });
            return intents
                .iter()
                .filter(|&(id, &i)| i == UserIntent::Remove && reachable.contains(id))
                .map(|(&id, _)| id)
                .collect();
        }

        intents
            .iter()
            .filter(|&(_, &i)| i == UserIntent::Install)
            .map(|(&id, _)| id)
            .filter(|&owner| {
                reachable_from(owner, |node| {
                    cache
                        .dependency_ids(node, false)
                        .into_iter()
                        .filter(|d| actions.get(d).is_some_and(|a| *a != ChangeAction::Remove))
                        .collect()
                })
                .contains(&target)
            })
            .collect()
    }

    /// Mark every upgradable package without an intent for upgrade.
    /// Returns how many were marked.
    pub fn mark_all_upgradable(&mut self) -> usize {
        let ids: Vec<PackageId> = {
            let cache = self.cache();
            let intents = &self.shared().user_intent;
            cache
                .packages(&PackageSort::default().upgradable())
                .filter_map(|pkg| cache.id_of(&pkg))
                .filter(|id| !intents.contains_key(id))
                .collect()
        };
        let count = ids.len();
        if count > 0 {
            self.edit_intents(ids.into_iter().map(|id| (id, Some(UserIntent::Install))));
        }
        count
    }

    /// How the plan changed relative to `before` (the planned ids prior to
    /// an action), ignoring the `acted` packages themselves.
    pub fn diff_since(&self, before: &HashSet<PackageId>, acted: &HashSet<PackageId>) -> PlanDiff {
        let changes = self.planned_changes().unwrap_or_default();
        let after = self.planned_ids();
        let mut diff = PlanDiff::default();
        for change in changes {
            if acted.contains(&change.package) {
                if !before.contains(&change.package) {
                    diff.download_size += change.download_size;
                }
            } else if !before.contains(&change.package) {
                diff.download_size += change.download_size;
                diff.added.push(change.clone());
            }
        }
        diff.dropped = before
            .iter()
            .filter(|id| !after.contains(id) && !acted.contains(id))
            .copied()
            .collect();
        diff
    }

    // === System operations ===

    /// Commit the current plan. Refuses (and changes nothing) if the plan
    /// has resolver errors.
    pub fn commit(
        &mut self,
        acquire_progress: &mut rust_apt::progress::AcquireProgress,
        install_progress: &mut rust_apt::progress::InstallProgress,
    ) -> Result<()> {
        let mut result = Ok(());
        self.transition(|state| match state {
            ManagerState::Planned(m) => {
                let (next, r) = m.commit(acquire_progress, install_progress);
                result = r;
                next
            }
            other => {
                result = Err(eyre!("nothing to apply"));
                other
            }
        });
        result
    }

    /// Run `apt update`, then reload. Intents are carried across the reload
    /// by package name and re-planned; returns the names of intents whose
    /// package no longer exists. On failure the cache is still reloaded
    /// (lists may be partially updated) unless it could not be reopened.
    pub fn update(
        &mut self,
        acquire_progress: &mut rust_apt::progress::AcquireProgress,
    ) -> (Vec<String>, Result<()>) {
        let generation = self.cache().generation();
        let carried: Vec<(String, UserIntent)> = {
            let cache = self.cache();
            self.shared()
                .user_intent
                .iter()
                .filter_map(|(&id, &i)| Some((cache.fullname_of(id)?.to_string(), i)))
                .collect()
        };

        let result = self.shared_mut().cache.update(acquire_progress);
        if self.cache().generation() == generation {
            return (Vec::new(), result);
        }

        self.transition(|state| {
            let mut shared = match state {
                ManagerState::Clean(m) => m.shared,
                ManagerState::Dirty(m) => m.shared,
                ManagerState::Planned(m) => m.shared,
                ManagerState::Transitioning => unreachable!("Transitioning state observed"),
            };
            shared.forget_generation();
            ManagerState::Clean(PackageManager {
                shared,
                state: Clean,
            })
        });

        let mut lost = Vec::new();
        let mut edits = Vec::new();
        for (name, intent) in carried {
            match self.cache().get_id(&name) {
                Some(id) => edits.push((id, Some(intent))),
                None => lost.push(self.cache().display_name(&name).to_string()),
            }
        }
        if !edits.is_empty() {
            self.edit_intents(edits);
        }
        (lost, result)
    }
}

/// Every node reachable from `start` (inclusive) via `next`.
fn reachable_from(
    start: PackageId,
    mut next: impl FnMut(PackageId) -> Vec<PackageId>,
) -> HashSet<PackageId> {
    let mut seen = HashSet::from([start]);
    let mut stack = vec![start];
    while let Some(node) = stack.pop() {
        for n in next(node) {
            if seen.insert(n) {
                stack.push(n);
            }
        }
    }
    seen
}

// ============================================================================
// Standalone utility functions
// ============================================================================

/// Check if running as root
pub fn is_root() -> bool {
    unsafe { libc::geteuid() == 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn info(id: u32, status: PackageStatus) -> PackageInfo {
        PackageInfo {
            id: PackageId(id),
            name: format!("pkg{id}"),
            status,
            section: String::new(),
            installed_version: String::new(),
            candidate_version: String::new(),
            installed_size: 0,
            download_size: 0,
            description: String::new(),
            architecture: String::new(),
        }
    }

    #[test]
    fn overlay_intents_then_plan() {
        let mut list = vec![
            info(0, PackageStatus::Upgradable),
            info(1, PackageStatus::NotInstalled),
            info(2, PackageStatus::Installed),
            info(3, PackageStatus::Installed),
            info(4, PackageStatus::Installed),
        ];
        let intents = HashMap::from([
            (PackageId(0), UserIntent::Install),
            (PackageId(1), UserIntent::Install),
            (PackageId(2), UserIntent::Hold),
            (PackageId(3), UserIntent::Remove),
        ]);
        let planned = [PlannedChange {
            package: PackageId(4),
            action: ChangeAction::Remove,
            reason: ChangeReason::Dependency,
            download_size: 0,
            size_change: 0,
        }];
        overlay_statuses(&mut list, &intents, Some(&planned));
        let statuses: Vec<_> = list.iter().map(|p| p.status).collect();
        assert_eq!(
            statuses,
            [
                PackageStatus::MarkedForUpgrade,
                PackageStatus::MarkedForInstall,
                PackageStatus::Held,
                PackageStatus::MarkedForRemove,
                PackageStatus::MarkedForRemove,
            ]
        );
    }

    #[test]
    fn upgrade_size_is_a_delta() {
        let new = Some((5_000, 101_000_000));
        assert_eq!(
            change_sizes(ChangeAction::Upgrade, new, Some(100_000_000)),
            (5_000, 1_000_000)
        );
        assert_eq!(
            change_sizes(ChangeAction::Install, new, None),
            (5_000, 101_000_000)
        );
        assert_eq!(
            change_sizes(ChangeAction::Remove, new, Some(100_000_000)),
            (0, -100_000_000)
        );
        assert_eq!(
            change_sizes(ChangeAction::Reinstall, new, Some(1)),
            (5_000, 0)
        );
    }

    #[test]
    fn version_sort_with_missing_versions() {
        let mut a = info(0, PackageStatus::Installed);
        let mut b = info(1, PackageStatus::Installed);
        a.installed_version = "10.0".into();
        b.installed_version = "9.0".into();
        assert_eq!(
            compare_packages(&a, &b, SortBy::InstalledVersion),
            Ordering::Greater
        );
        b.installed_version.clear();
        assert_eq!(
            compare_packages(&a, &b, SortBy::InstalledVersion),
            Ordering::Greater
        );
    }

    #[test]
    fn reachable_follows_edges_once() {
        let edges = HashMap::from([(0u32, vec![1, 2]), (1, vec![2, 0]), (2, vec![3])]);
        let seen = reachable_from(PackageId(0), |n| {
            edges
                .get(&n.0)
                .map(|v| v.iter().map(|&i| PackageId(i)).collect())
                .unwrap_or_default()
        });
        assert_eq!(seen.len(), 4);
    }
}
