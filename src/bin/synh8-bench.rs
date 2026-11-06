//! Headless workload for `brokkr bench`: drives the core hot paths without a
//! terminal and prints per-phase wall times as one JSON object on stdout.
//!
//! Read-only against APT (no commit, no lock), so it does not need root.
//!
//! Usage: synh8-bench [PACKAGE]
//!
//! PACKAGE is the full name to mark for install; without it the first
//! not-installed package (by the default sort) is used.

use std::time::Instant;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use synh8::core::ManagerState;
use synh8::types::FilterCategory;

fn ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let target = std::env::args().nth(1);

    let _hotpath = hotpath::HotpathGuardBuilder::new("synh8-bench")
        .percentiles(&[50.0, 95.0, 99.0])
        .functions_limit(0)
        .build();

    let mut phases: Vec<(&str, f64)> = Vec::new();

    let t = Instant::now();
    let mut state = ManagerState::new()?;
    phases.push(("open", ms(t)));

    // Cold: every filter's first build (the cache is empty after open,
    // apart from the initial filter).
    let t = Instant::now();
    for &filter in FilterCategory::all() {
        state.set_filter(filter);
        state.rebuild_list();
    }
    phases.push(("filters_cold", ms(t)));

    // Warm: the same switches served from the filter cache.
    let t = Instant::now();
    for &filter in FilterCategory::all() {
        state.set_filter(filter);
        state.rebuild_list();
    }
    phases.push(("filters_warm", ms(t)));

    state.set_filter(FilterCategory::NotInstalled);
    state.rebuild_list();
    let id = match &target {
        Some(name) => state
            .list()
            .iter()
            .find(|p| p.name == *name)
            .map(|p| p.id)
            .ok_or_else(|| eyre!("{name} is not a not-installed package"))?,
        None => state
            .list()
            .first()
            .map(|p| p.id)
            .ok_or_else(|| eyre!("no not-installed packages"))?,
    };

    let t = Instant::now();
    state.toggle(id);
    state.rebuild_list();
    phases.push(("mark", ms(t)));

    let t = Instant::now();
    state.toggle(id);
    state.rebuild_list();
    phases.push(("unmark", ms(t)));

    let t = Instant::now();
    state.ensure_search_index()?;
    phases.push(("search_index", ms(t)));

    let t = Instant::now();
    state.set_filter(FilterCategory::All);
    state.set_search_query("lib")?;
    state.rebuild_list();
    phases.push(("search_query", ms(t)));

    let body: Vec<String> = phases
        .iter()
        .map(|(name, v)| format!("\"{name}\":{v:.3}"))
        .collect();
    println!("{{{}}}", body.join(","));
    Ok(())
}
