//! APT cache operations and package management
//!
//! This module provides a thin wrapper around rust-apt with PackageId handles.
//! User intent tracking is handled by the core module, not here.

use std::collections::HashMap;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use rust_apt::cache::{Cache, PackageSort};
use rust_apt::error::AptErrors;
use rust_apt::progress::{AcquireProgress, InstallProgress};
use rust_apt::{DepType, Package, Version};

use crate::types::*;

/// Manages APT cache interactions with PackageId handles.
/// Each unique package (including multi-arch variants) gets its own PackageId.
/// Ids are renumbered by every `reload()`.
pub struct AptCache {
    cache: Cache,
    /// Map from package full name (e.g., "libfoo:amd64") to PackageId
    fullname_to_id: HashMap<String, PackageId>,
    /// Reverse map: PackageId -> full name
    id_to_fullname: Vec<String>,
    /// Native architecture (e.g., "amd64")
    native_arch: String,
    /// Cached suffix for display_name stripping (e.g., ":amd64")
    native_arch_suffix: String,
    /// Incremented by every successful `reload()`
    generation: u64,
}

impl AptCache {
    /// Open the APT cache and number every package.
    pub fn new() -> Result<Self> {
        let cache = Cache::new::<&str>(&[])?;
        // Cache::new initialises the APT configuration, so this is populated.
        let native_arch = rust_apt::config::Config::new().find("APT::Architecture", "");
        let native_arch_suffix = format!(":{native_arch}");
        let mut apt = Self {
            cache,
            fullname_to_id: HashMap::new(),
            id_to_fullname: Vec::new(),
            native_arch,
            native_arch_suffix,
            generation: 0,
        };
        apt.number_packages();
        Ok(apt)
    }

    /// Assign ids using FULL names, so libfoo:amd64 and libfoo:i386 differ.
    fn number_packages(&mut self) {
        self.fullname_to_id.clear();
        self.id_to_fullname.clear();
        for pkg in self.cache.packages(&PackageSort::default()) {
            let fullname = pkg.fullname(false);
            let id = PackageId(self.id_to_fullname.len() as u32);
            self.id_to_fullname.push(fullname.clone());
            self.fullname_to_id.insert(fullname, id);
        }
    }

    /// Re-open the cache from disk. Starts a new id generation: every
    /// PackageId issued before this call is invalid afterwards.
    pub fn reload(&mut self) -> Result<()> {
        self.cache = Cache::new::<&str>(&[])?;
        self.number_packages();
        self.generation += 1;
        Ok(())
    }

    /// Id generation: changes whenever ids are renumbered
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Get the native architecture (e.g., "amd64")
    pub fn native_arch(&self) -> &str {
        &self.native_arch
    }

    /// Get display name for a package (strips native arch suffix)
    /// "libfoo:amd64" -> "libfoo" (if amd64 is native)
    /// "libfoo:i386" -> "libfoo:i386" (keeps non-native arch)
    pub fn display_name<'a>(&self, fullname: &'a str) -> &'a str {
        fullname
            .strip_suffix(&self.native_arch_suffix)
            .unwrap_or(fullname)
    }

    /// Display name for a PackageId, with a visible placeholder for ids
    /// from another cache generation.
    pub fn display_name_of(&self, id: PackageId) -> String {
        self.fullname_of(id).map_or_else(
            || format!("(unknown:{})", id.index()),
            |n| self.display_name(n).to_string(),
        )
    }

    // ========================================================================
    // PackageId management
    // ========================================================================

    /// Get the PackageId for a full name (returns None if not known)
    pub fn get_id(&self, fullname: &str) -> Option<PackageId> {
        self.fullname_to_id.get(fullname).copied()
    }

    /// Get the PackageId of a package from this cache
    pub fn id_of(&self, pkg: &Package) -> Option<PackageId> {
        self.get_id(&pkg.fullname(false))
    }

    /// Get the full name for a PackageId
    pub fn fullname_of(&self, id: PackageId) -> Option<&str> {
        self.id_to_fullname
            .get(id.index())
            .map(std::string::String::as_str)
    }

    /// Get a package by PackageId
    pub fn get_by_id(&self, id: PackageId) -> Option<Package<'_>> {
        self.fullname_of(id)
            .and_then(|fullname| self.cache.get(fullname))
    }

    // ========================================================================
    // Package iteration
    // ========================================================================

    /// Get an iterator over packages with the given sort
    pub fn packages(&self, sort: &PackageSort) -> impl Iterator<Item = Package<'_>> {
        self.cache.packages(sort)
    }

    /// Get packages with pending changes
    pub fn get_changes(&self) -> impl Iterator<Item = Package<'_>> {
        self.cache.get_changes(false)
    }

    // ========================================================================
    // Marking operations. Called only by core.rs's plan(), which derives
    // every APT mark from user intent.
    // ========================================================================

    /// Mark a package for install/upgrade, pulling in its dependencies
    pub(crate) fn mark_install(&self, id: PackageId) {
        if let Some(pkg) = self.get_by_id(id) {
            pkg.mark_install(true, true);
        }
    }

    /// Mark a package for removal (configuration files are kept)
    pub(crate) fn mark_delete(&self, id: PackageId) {
        if let Some(pkg) = self.get_by_id(id) {
            pkg.mark_delete(false);
        }
    }

    /// Mark a package to stay in its current state
    pub(crate) fn mark_keep(&self, id: PackageId) {
        if let Some(pkg) = self.get_by_id(id) {
            pkg.mark_keep();
        }
    }

    /// Forbid the resolver from changing this package's mark. Protection
    /// lives in the resolver, not the depcache, so `clear_all_marks()` does
    /// not reset it; `unprotect()` does.
    pub(crate) fn protect(&self, id: PackageId) {
        if let Some(pkg) = self.get_by_id(id) {
            pkg.protect();
        }
    }

    /// Undo `protect()`
    pub(crate) fn unprotect(&self, id: PackageId) {
        if let Some(pkg) = self.get_by_id(id) {
            self.cache.resolver().clear(&pkg);
        }
    }

    /// Clear all marks on all packages in a single depcache re-init.
    pub(crate) fn clear_all_marks(&self) -> Result<(), String> {
        self.cache
            .depcache()
            .clear_marked()
            .map_err(|e| format!("Failed to reset APT marks: {e}"))
    }

    /// Resolve dependencies
    pub(crate) fn resolve(&self) -> Result<(), AptErrors> {
        self.cache.resolve(true)
    }

    // ========================================================================
    // Package info extraction (status determined by APT state only)
    // ========================================================================

    /// Extract package info from an APT Package.
    /// Returns BASE status (installed/upgradable/not-installed) - ignores APT marks.
    /// The core module will compute final display status based on user_intent.
    pub fn extract_package_info(&self, pkg: &Package) -> Option<PackageInfo> {
        let candidate = pkg.candidate()?;

        let status = if pkg.is_installed() {
            if pkg.is_upgradable() {
                PackageStatus::Upgradable
            } else {
                PackageStatus::Installed
            }
        } else {
            PackageStatus::NotInstalled
        };

        let installed_version = pkg
            .installed()
            .map(|v: Version| v.version().to_string())
            .unwrap_or_default();

        let fullname = pkg.fullname(false);
        let id = self.get_id(&fullname)?;

        Some(PackageInfo {
            id,
            name: fullname,
            status,
            section: candidate.section().unwrap_or("unknown").to_string(),
            installed_version,
            candidate_version: candidate.version().to_string(),
            installed_size: candidate.installed_size(),
            download_size: candidate.size(),
            description: candidate.summary().unwrap_or_default().clone(),
            architecture: candidate.arch().to_string(),
        })
    }

    // ========================================================================
    // Dependency queries
    // ========================================================================

    /// Forward dependencies of a package's candidate version
    pub fn get_dependencies(&self, fullname: &str) -> Vec<(DepType, String)> {
        let mut deps = Vec::new();
        if let Some(pkg) = self.cache.get(fullname)
            && let Some(version) = pkg.candidate()
            && let Some(dependencies) = version.dependencies()
        {
            for dep in dependencies {
                for base_dep in dep.iter() {
                    deps.push((base_dep.dep_type(), base_dep.name().to_string()));
                }
            }
        }
        sort_deps(&mut deps);
        deps
    }

    /// Reverse dependencies of a package
    pub fn get_reverse_dependencies(&self, fullname: &str) -> Vec<(DepType, String)> {
        let mut rdeps = Vec::new();
        if let Some(pkg) = self.cache.get(fullname) {
            for (dep_type, deps) in pkg.rdepends() {
                for dep in deps {
                    for base_dep in dep.iter() {
                        rdeps.push((dep_type.clone(), base_dep.name().to_string()));
                    }
                }
            }
        }
        sort_deps(&mut rdeps);
        rdeps
    }

    /// Ids of every package that can satisfy a hard or recommended
    /// dependency of `id`'s candidate version (or installed version, when
    /// `installed` is set). Virtual packages resolve to their providers.
    pub(crate) fn dependency_ids(&self, id: PackageId, installed: bool) -> Vec<PackageId> {
        let Some(pkg) = self.get_by_id(id) else {
            return Vec::new();
        };
        let version = if installed {
            pkg.installed()
        } else {
            pkg.candidate()
        };
        let Some(deps) = version.as_ref().and_then(Version::dependencies) else {
            return Vec::new();
        };
        let mut ids = Vec::new();
        for dep in deps {
            if !matches!(
                dep.dep_type(),
                DepType::Depends | DepType::PreDepends | DepType::Recommends
            ) {
                continue;
            }
            for base_dep in dep.iter() {
                for target in base_dep.all_targets() {
                    if let Some(target_id) = self.id_of(&target.parent()) {
                        ids.push(target_id);
                    }
                }
            }
        }
        ids
    }

    // ========================================================================
    // Cache lifecycle
    // ========================================================================

    /// Download and install the marked changes. Whatever happens, the cache
    /// is reloaded afterwards so it reflects the system as it now is; if the
    /// reload itself fails the pre-commit cache stays in place (stale but
    /// usable) and the error is reported.
    pub(crate) fn commit(
        &mut self,
        acquire_progress: &mut AcquireProgress,
        install_progress: &mut InstallProgress,
    ) -> Result<()> {
        ensure_archive_dirs()?;
        // commit() consumes the cache, so a stand-in is needed.
        let marked = std::mem::replace(&mut self.cache, Cache::new::<&str>(&[])?);
        let result = marked
            .commit(acquire_progress, install_progress)
            .map_err(|e| eyre!("{}", format_apt_errors(&e)));
        combine(result, self.reload())
    }

    /// Run `apt update` (refresh package lists), then reload the cache.
    /// Like `commit()`, always leaves a usable cache behind.
    pub(crate) fn update(&mut self, acquire_progress: &mut AcquireProgress) -> Result<()> {
        let old = std::mem::replace(&mut self.cache, Cache::new::<&str>(&[])?);
        let result = old
            .update(acquire_progress)
            .map_err(|e| eyre!("{}", format_apt_errors(&e)));
        combine(result, self.reload())
    }
}

/// Merge an operation result with the reload that follows it, keeping both
/// errors if both failed.
fn combine(op: Result<()>, reload: Result<()>) -> Result<()> {
    match (op, reload) {
        (Ok(()), r) => r,
        (Err(e), Ok(())) => Err(e),
        (Err(e), Err(r)) => Err(e.wrap_err(format!(
            "additionally, reloading the package cache failed: {r}"
        ))),
    }
}

/// Fetch a package's changelog with `apt changelog`.
pub fn fetch_changelog(display_name: &str) -> Result<Vec<String>> {
    let output = std::process::Command::new("apt")
        .args(["changelog", display_name])
        .output()
        .map_err(|e| eyre!("Failed to run apt changelog: {e}"))?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("apt changelog {display_name}: {}", err.trim()));
    }
    let lines: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(std::string::ToString::to_string)
        .collect();
    if lines.is_empty() {
        Ok(vec!["No changelog available.".to_string()])
    } else {
        Ok(lines)
    }
}

/// Recreate `Dir::Cache::Archives` and its `partial/` subdirectory if missing.
///
/// apt-get runs libapt's `SetupAPTPartialDirectory()` (via `pkgAcquire::GetLock`)
/// before every fetch, so a wiped /var/cache/apt is silently rebuilt there.
/// rust-apt's `commit()` constructs a bare `pkgAcquire` and skips that step,
/// so without this guard every download item fails immediately when the user
/// has deleted the cache directory between runs. Matches apt's layout:
/// `partial/` is 0700 and owned by `_apt`, since the sandboxed download
/// methods drop privileges before writing into it.
fn ensure_archive_dirs() -> Result<()> {
    let config = rust_apt::config::Config::new();
    let archive_dir = config.dir("Dir::Cache::Archives", "/var/cache/apt/archives/");
    let partial = Path::new(&archive_dir).join("partial");
    if partial.is_dir() {
        return Ok(());
    }
    std::fs::create_dir_all(&partial)?;
    std::fs::set_permissions(&partial, std::fs::Permissions::from_mode(0o700))?;
    // Best effort, like apt: if the _apt user is missing, apt warns and
    // downloads as root instead of failing, and so do we.
    drop(
        std::process::Command::new("chown")
            .arg("_apt:root")
            .arg(&partial)
            .status(),
    );
    Ok(())
}

/// Order dependency types by importance, then by name
fn sort_deps(deps: &mut [(DepType, String)]) {
    deps.sort_by(|a, b| {
        dep_type_order(&a.0)
            .cmp(&dep_type_order(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });
}

fn dep_type_order(t: &DepType) -> u8 {
    match t {
        DepType::PreDepends => 0,
        DepType::Depends => 1,
        DepType::Recommends => 2,
        DepType::Suggests => 3,
        DepType::Enhances => 4,
        DepType::Conflicts | DepType::DpkgBreaks | DepType::Replaces | DepType::Obsoletes => 5,
    }
}

/// Split AptErrors into errors and warnings
pub fn plan_problems(errors: &AptErrors) -> PlanProblems {
    let mut problems = PlanProblems::default();
    for error in errors.iter() {
        let msg = error.msg.trim();
        if msg.is_empty() {
            continue;
        }
        if error.is_error {
            problems.errors.push(msg.to_string());
        } else {
            problems.warnings.push(msg.to_string());
        }
    }
    problems
}

/// Format AptErrors into a one-line, user-friendly string.
pub fn format_apt_errors(errors: &AptErrors) -> String {
    plan_problems(errors)
        .summary()
        .unwrap_or_else(|| "APT reported a failure without details".to_string())
}
