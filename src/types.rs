//! Common types used throughout the application

use std::collections::{HashMap, HashSet};

use ratatui::prelude::*;
use rust_apt::Marked;

// ============================================================================
// Core API Types (Typestate Pattern)
// ============================================================================

/// Opaque handle to a package. Valid only for the cache generation that
/// issued it: `AptCache::reload()` renumbers every package, so anything that
/// must survive a reload is carried across by full name instead.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PackageId(pub(crate) u32);

impl PackageId {
    /// Get the raw index (for internal use)
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// What the user explicitly wants for a package. Absence from the intent map
/// means "no opinion": APT decides.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum UserIntent {
    /// Install, or upgrade to the candidate version
    Install,
    /// Remove (without purging configuration files)
    Remove,
    /// Keep the current state: the resolver may not install, upgrade or
    /// remove this package, and "mark all upgrades" skips it
    Hold,
}

/// A snapshot of every user intent, used to undo a mark action wholesale.
pub type IntentSnapshot = HashMap<PackageId, UserIntent>;

/// Why a package is changing
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChangeReason {
    /// User explicitly requested this
    UserRequested,
    /// Pulled in, or pushed out, by the resolver to satisfy a user request
    Dependency,
}

/// Type of change to a package
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChangeAction {
    Install,
    Upgrade,
    Downgrade,
    Reinstall,
    Remove,
}

impl ChangeAction {
    /// Classify an APT mark. `None` for marks that change nothing on disk.
    pub fn from_marked(marked: Marked) -> Option<Self> {
        match marked {
            Marked::NewInstall | Marked::Install => Some(Self::Install),
            Marked::Upgrade => Some(Self::Upgrade),
            Marked::Downgrade => Some(Self::Downgrade),
            Marked::ReInstall => Some(Self::Reinstall),
            Marked::Remove | Marked::Purge => Some(Self::Remove),
            Marked::Keep | Marked::Held | Marked::None => None,
        }
    }

    /// Display status of a package carrying this change
    pub fn status(self) -> PackageStatus {
        match self {
            Self::Install | Self::Reinstall => PackageStatus::MarkedForInstall,
            Self::Upgrade => PackageStatus::MarkedForUpgrade,
            Self::Downgrade => PackageStatus::MarkedForDowngrade,
            Self::Remove => PackageStatus::MarkedForRemove,
        }
    }

    /// True for changes that fetch a package
    pub fn downloads(self) -> bool {
        !matches!(self, Self::Remove)
    }

    /// Heading used when listing changes of this kind
    pub fn label(self) -> &'static str {
        match self {
            Self::Install => "Install",
            Self::Upgrade => "Upgrade",
            Self::Downgrade => "Downgrade",
            Self::Reinstall => "Reinstall",
            Self::Remove => "Remove",
        }
    }

    pub fn all() -> &'static [ChangeAction] {
        &[
            Self::Upgrade,
            Self::Install,
            Self::Downgrade,
            Self::Reinstall,
            Self::Remove,
        ]
    }
}

/// A planned change to a package. Name is derived from PackageId, not stored.
#[derive(Clone, Debug)]
pub struct PlannedChange {
    pub package: PackageId,
    pub action: ChangeAction,
    pub reason: ChangeReason,
    pub download_size: u64,
    pub size_change: i64,
}

/// Problems reported by the dependency resolver (or by APT while planning).
/// Errors make the plan unappliable; warnings are informational.
#[derive(Clone, Debug, Default)]
pub struct PlanProblems {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl PlanProblems {
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty() && self.warnings.is_empty()
    }

    /// One-line summary. Errors take priority over warnings: a real error is
    /// always the headline, and warnings only surface when there are none.
    pub fn summary(&self) -> Option<String> {
        let (messages, label) = if self.errors.is_empty() {
            (&self.warnings, "warning")
        } else {
            (&self.errors, "error")
        };
        match messages.len() {
            0 => None,
            1 => Some(messages[0].clone()),
            n => Some(format!("{}; and {} more {label}(s)", messages[0], n - 1)),
        }
    }
}

// ============================================================================
// Typestate Markers
// ============================================================================

/// Clean state - no pending changes
pub struct Clean;

/// Dirty state - has user marks but no computed plan
pub struct Dirty;

/// Planned state - dependencies resolved, changeset computed
pub struct Planned {
    pub changes: Vec<PlannedChange>,
    pub download_size: u64,
    pub install_size_change: i64,
    pub problems: PlanProblems,
}

// ============================================================================
// Display types
// ============================================================================

/// Package status - no distinction between user-marked and dependency
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    // Base states (not marked)
    Installed,    // · Package is installed, no changes pending
    NotInstalled, //   Package is not installed, no changes pending
    Upgradable,   // ↑ Package can be upgraded (yellow)
    // Marked states (all marked packages look identical)
    MarkedForInstall,   // + Package will be installed
    MarkedForUpgrade,   // ↑ Package will be upgraded (green)
    MarkedForDowngrade, // ↓ Package will be downgraded
    MarkedForRemove,    // - Package will be removed
    Held,               // = User holds the package at its current state
}

impl PackageStatus {
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Upgradable | Self::MarkedForUpgrade => "↑",
            Self::MarkedForDowngrade => "↓",
            Self::MarkedForInstall => "+",
            Self::MarkedForRemove => "-",
            Self::Held => "=",
            Self::Installed => "·",
            Self::NotInstalled => " ",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Upgradable => Color::Yellow,
            Self::MarkedForUpgrade => Color::Green,
            Self::MarkedForInstall => Color::Green,
            Self::MarkedForDowngrade => Color::Magenta,
            Self::MarkedForRemove => Color::Red,
            Self::Held => Color::Blue,
            Self::Installed => Color::DarkGray,
            Self::NotInstalled => Color::Gray,
        }
    }

    /// Human-readable name for the details pane
    pub fn label(&self) -> &'static str {
        match self {
            Self::Installed => "Installed",
            Self::NotInstalled => "Not installed",
            Self::Upgradable => "Upgradable",
            Self::MarkedForInstall => "Marked for install",
            Self::MarkedForUpgrade => "Marked for upgrade",
            Self::MarkedForDowngrade => "Marked for downgrade",
            Self::MarkedForRemove => "Marked for removal",
            Self::Held => "Held",
        }
    }

    /// Check if this status represents a marked (pending change) state
    pub fn is_marked(&self) -> bool {
        matches!(
            self,
            Self::MarkedForInstall
                | Self::MarkedForUpgrade
                | Self::MarkedForDowngrade
                | Self::MarkedForRemove
        )
    }
}

/// Filter categories (left panel)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilterCategory {
    Upgradable,
    MarkedChanges,
    Installed,
    NotInstalled,
    All,
}

impl FilterCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Upgradable => "Upgradable",
            Self::MarkedChanges => "Marked Changes",
            Self::Installed => "Installed",
            Self::NotInstalled => "Not Installed",
            Self::All => "All Packages",
        }
    }

    pub fn all() -> &'static [FilterCategory] {
        &[
            Self::Upgradable,
            Self::MarkedChanges,
            Self::Installed,
            Self::NotInstalled,
            Self::All,
        ]
    }
}

/// Displayed package info (extracted from rust-apt Package).
/// `id` is the handle for everything within one cache generation; `name`
/// is what survives a reload (selection restore, intent carry-over).
#[derive(Debug, Clone)]
pub struct PackageInfo {
    pub id: PackageId,
    pub name: String, // Full name including arch (e.g., "libfoo:i386")
    pub status: PackageStatus,
    pub section: String,
    pub installed_version: String,
    pub candidate_version: String,
    pub installed_size: u64,
    pub download_size: u64,
    pub description: String,
    pub architecture: String,
}

impl PackageInfo {
    pub fn installed_size_str(&self) -> String {
        size_str(self.installed_size)
    }

    pub fn download_size_str(&self) -> String {
        size_str(self.download_size)
    }
}

/// Format a byte count the way apt does: SI units (1 kB = 1000 B).
pub fn size_str(bytes: u64) -> String {
    if bytes == 0 {
        return String::from("-");
    }
    const KB: u64 = 1000;
    const MB: u64 = KB * 1000;
    const GB: u64 = MB * 1000;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} kB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Format a signed byte delta, e.g. "+1.2 MB" / "-300 kB".
pub fn size_change_str(delta: i64) -> String {
    let sign = if delta < 0 { "-" } else { "+" };
    format!("{sign}{}", size_str(delta.unsigned_abs()))
}

/// Which pane has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    Filters,
    Packages,
    Details,
}

/// Which tab is shown in details pane
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetailsTab {
    Info,
    Dependencies,
    ReverseDeps,
}

/// Application state machine
#[derive(Debug, PartialEq, Eq)]
pub enum AppState {
    Listing,
    Searching,          // User is typing a search query
    ShowingMarkConfirm, // Popup showing additional changes when marking a package
    ShowingChanges,     // Final confirmation before applying all changes
    ShowingChangelog,   // Viewing package changelog
    ShowingSettings,    // Settings/preferences view
    ConfirmExit,        // Confirm exit with pending changes
    Done,               // Showing apt/dpkg output after applying changes
}

/// Sort options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortBy {
    Name,
    Section,
    InstalledVersion,
    CandidateVersion,
}

impl SortBy {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Section => "Section",
            Self::InstalledVersion => "Installed version",
            Self::CandidateVersion => "Candidate version",
        }
    }

    pub fn all() -> &'static [SortBy] {
        &[
            Self::Name,
            Self::Section,
            Self::InstalledVersion,
            Self::CandidateVersion,
        ]
    }
}

/// Sort configuration. The single owner of the default sort order.
#[derive(Debug, Clone, Copy)]
pub struct SortSettings {
    pub sort_by: SortBy,
    pub ascending: bool,
}

impl Default for SortSettings {
    fn default() -> Self {
        Self {
            sort_by: SortBy::CandidateVersion,
            ascending: true,
        }
    }
}

/// User settings (not persisted yet)
#[derive(Debug, Clone)]
pub struct Settings {
    pub visible_columns: HashSet<Column>,
    pub sort: SortSettings,
}

impl Default for Settings {
    fn default() -> Self {
        let mut visible_columns = HashSet::new();
        visible_columns.insert(Column::Status);
        visible_columns.insert(Column::Name);
        visible_columns.insert(Column::CandidateVersion);
        Self {
            visible_columns,
            sort: SortSettings::default(),
        }
    }
}

/// Preview of what a mark action did to the plan, shown in a confirmation
/// modal. The action has already been applied; cancelling restores `undo`.
#[derive(Debug, Clone)]
pub struct MarkPreview {
    pub headline: String,
    /// Packages newly planned beyond the ones acted on, by display name
    pub added: Vec<(String, ChangeAction)>,
    /// Packages that left the plan beyond the ones acted on
    pub dropped: Vec<String>,
    /// Download size of everything the action newly planned
    pub download_size: u64,
    /// User intents as they were before the action
    pub undo: IntentSnapshot,
}

/// Column configuration for the package table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    Status,
    Name,
    Section,
    InstalledVersion,
    CandidateVersion,
    DownloadSize,
}

impl Column {
    /// All columns in display order.
    pub fn all() -> &'static [Column] {
        &[
            Self::Status,
            Self::Name,
            Self::Section,
            Self::InstalledVersion,
            Self::CandidateVersion,
            Self::DownloadSize,
        ]
    }

    /// Human-readable label for the settings UI.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Status => "Status column (S)",
            Self::Name => "Name column",
            Self::Section => "Section column",
            Self::InstalledVersion => "Installed version column",
            Self::CandidateVersion => "Candidate version column",
            Self::DownloadSize => "Download size column",
        }
    }

    pub fn header(&self) -> &'static str {
        match self {
            Self::Status => "S",
            Self::Name => "Package",
            Self::Section => "Section",
            Self::InstalledVersion => "Installed",
            Self::CandidateVersion => "Candidate",
            Self::DownloadSize => "Download",
        }
    }

    pub fn width(&self, col_widths: &ColumnWidths) -> Constraint {
        match self {
            Self::Status => Constraint::Length(3),
            Self::Name => Constraint::Min(col_widths.name),
            Self::Section => Constraint::Length(col_widths.section),
            Self::InstalledVersion => Constraint::Length(col_widths.installed),
            Self::CandidateVersion => Constraint::Length(col_widths.candidate),
            Self::DownloadSize => Constraint::Length(10),
        }
    }

    pub fn visible_columns(settings: &Settings) -> Vec<Column> {
        Self::all()
            .iter()
            .copied()
            .filter(|col| settings.visible_columns.contains(col))
            .collect()
    }
}

/// Column width storage
#[derive(Debug, Clone)]
pub struct ColumnWidths {
    pub name: u16,
    pub section: u16,
    pub installed: u16,
    pub candidate: u16,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            name: 10,
            section: 7,
            installed: 9,
            candidate: 9,
        }
    }
}

/// Scroll position of a scrollable view. The renderer reports `viewport` and
/// `content` each frame; key handlers move `offset` against those, so page
/// size and the scroll limit come from what is actually on screen.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScrollView {
    pub offset: usize,
    pub viewport: usize,
    pub content: usize,
}

impl ScrollView {
    /// Largest offset that still fills the viewport
    pub fn max_offset(&self) -> usize {
        self.content.saturating_sub(self.viewport)
    }

    /// Rows moved by a page key
    pub fn page(&self) -> usize {
        self.viewport.max(1)
    }

    pub fn scroll_by(&mut self, delta: isize) {
        self.offset = self
            .offset
            .saturating_add_signed(delta)
            .min(self.max_offset());
    }

    pub fn page_by(&mut self, pages: isize) {
        self.scroll_by(pages * self.page() as isize);
    }

    pub fn home(&mut self) {
        self.offset = 0;
    }

    pub fn end(&mut self) {
        self.offset = self.max_offset();
    }

    /// Record what the renderer measured and re-clamp.
    pub fn measure(&mut self, viewport: usize, content: usize) {
        self.viewport = viewport;
        self.content = content;
        self.offset = self.offset.min(self.max_offset());
    }

    /// Offset as ratatui's `Paragraph::scroll` wants it
    pub fn offset_u16(&self) -> u16 {
        u16::try_from(self.offset).unwrap_or(u16::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_str_uses_si_units() {
        assert_eq!(size_str(0), "-");
        assert_eq!(size_str(999), "999 B");
        assert_eq!(size_str(1000), "1.0 kB");
        assert_eq!(size_str(1_500_000), "1.5 MB");
        assert_eq!(size_str(2_000_000_000), "2.0 GB");
    }

    #[test]
    fn size_change_str_signs() {
        assert_eq!(size_change_str(1000), "+1.0 kB");
        assert_eq!(size_change_str(-1000), "-1.0 kB");
        assert_eq!(size_change_str(i64::MIN), "-9223372036.9 GB");
    }

    #[test]
    fn plan_problems_summary_prefers_errors() {
        let p = PlanProblems {
            errors: vec!["broken".into(), "also broken".into()],
            warnings: vec!["meh".into()],
        };
        assert_eq!(p.summary().unwrap(), "broken; and 1 more error(s)");
        let w = PlanProblems {
            errors: vec![],
            warnings: vec!["meh".into()],
        };
        assert_eq!(w.summary().unwrap(), "meh");
        assert!(PlanProblems::default().summary().is_none());
    }

    #[test]
    fn scroll_view_clamps_to_content() {
        let mut s = ScrollView::default();
        s.measure(10, 25);
        s.page_by(1);
        assert_eq!(s.offset, 10);
        s.page_by(1);
        assert_eq!(s.offset, 15);
        s.scroll_by(-100);
        assert_eq!(s.offset, 0);
        s.end();
        assert_eq!(s.offset, 15);
        s.measure(10, 12);
        assert_eq!(s.offset, 2);
    }

    #[test]
    fn scroll_view_short_content_does_not_scroll() {
        let mut s = ScrollView::default();
        s.measure(10, 3);
        s.end();
        assert_eq!(s.offset, 0);
    }

    #[test]
    fn change_action_classification() {
        assert_eq!(
            ChangeAction::from_marked(Marked::Downgrade),
            Some(ChangeAction::Downgrade)
        );
        assert_eq!(
            ChangeAction::from_marked(Marked::Purge),
            Some(ChangeAction::Remove)
        );
        assert_eq!(ChangeAction::from_marked(Marked::Held), None);
        assert!(!ChangeAction::Remove.downloads());
    }
}
