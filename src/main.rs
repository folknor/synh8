mod app;
mod ui;

use std::io;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;

use app::App;
use synh8::core::is_root;
use synh8::types::*;
use ui::ui;

fn main() -> Result<()> {
    color_eyre::install()?;

    if !is_root() {
        eprintln!("synh8 must be run as root. Try: sudo {}", std::env::args().next().unwrap_or_else(|| "synh8".into()));
        std::process::exit(1);
    }

    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

    let _hotpath = hotpath::HotpathGuardBuilder::new("synh8")
        .percentiles(&[50, 95, 99])
        .with_functions_limit(0)
        .build();

    let mut app = App::new()?;

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))?
            && let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match app.state {
                    AppState::Listing => {
                        // === Global keys (any focused pane) ===
                        match key.code {
                            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                                if app.has_pending_changes() {
                                    app.state = AppState::ConfirmExit;
                                } else {
                                    break;
                                }
                                continue;
                            }
                            KeyCode::Char('q') => {
                                if app.has_pending_changes() {
                                    app.state = AppState::ConfirmExit;
                                } else {
                                    break;
                                }
                            }
                            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                                app.cycle_focus_back();
                            }
                            KeyCode::Tab => app.cycle_focus(),
                            KeyCode::BackTab => app.cycle_focus_back(),
                            KeyCode::Char('/') | KeyCode::Char('s') => app.start_search(),
                            KeyCode::Char(',') => app.show_settings(),
                            KeyCode::Char('u') => {
                                // apt update with live progress
                                app.update_packages_live()?;
                                terminal.clear()?;
                            }
                            KeyCode::Esc => {
                                if app.ui.visual_mode {
                                    app.cancel_visual_mode();
                                } else if app.core.has_search_results() {
                                    app.core.clear_search();
                                    app.apply_current_filter();
                                    app.update_status_message();
                                }
                            }
                            _ => {
                                // === Pane-local keys ===
                                match app.ui.focused_pane {
                                    FocusedPane::Filters => match key.code {
                                        KeyCode::Up | KeyCode::Char('k') => app.move_filter_selection(-1),
                                        KeyCode::Down | KeyCode::Char('j') => app.move_filter_selection(1),
                                        KeyCode::PageUp => app.move_filter_selection(-1),
                                        KeyCode::PageDown => app.move_filter_selection(1),
                                        KeyCode::Home | KeyCode::Char('g') => app.select_first_filter(),
                                        KeyCode::End | KeyCode::Char('G') => app.select_last_filter(),
                                        KeyCode::Enter | KeyCode::Char(' ') => app.apply_current_filter(),
                                        _ => {}
                                    },
                                    FocusedPane::Packages => match key.code {
                                        // Navigation
                                        KeyCode::Up | KeyCode::Char('k') => {
                                            app.move_package_selection(-1);
                                            app.update_visual_selection();
                                        }
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            app.move_package_selection(1);
                                            app.update_visual_selection();
                                        }
                                        KeyCode::PageDown => {
                                            app.move_package_selection(10);
                                            app.update_visual_selection();
                                        }
                                        KeyCode::PageUp => {
                                            app.move_package_selection(-10);
                                            app.update_visual_selection();
                                        }
                                        KeyCode::Home | KeyCode::Char('g') => {
                                            app.select_first_package();
                                            app.update_visual_selection();
                                        }
                                        KeyCode::End | KeyCode::Char('G') => {
                                            app.select_last_package();
                                            app.update_visual_selection();
                                        }
                                        // Package actions
                                        KeyCode::Char(' ') => {
                                            if app.ui.visual_mode {
                                                app.toggle_multi_select();
                                            } else {
                                                app.toggle_current();
                                            }
                                        }
                                        KeyCode::Char('v') => app.start_visual_mode(),
                                        KeyCode::Char('c') => app.show_changelog(),
                                        KeyCode::Char('r') => app.show_changes_preview(),
                                        KeyCode::Char('x') => app.mark_all_upgrades(),
                                        KeyCode::Char('X') => app.unmark_all(),
                                        _ => {}
                                    },
                                    FocusedPane::Details => match key.code {
                                        // Navigation
                                        KeyCode::Up | KeyCode::Char('k') => {
                                            app.details.scroll = app.details.scroll.saturating_sub(1);
                                        }
                                        KeyCode::Down | KeyCode::Char('j') => {
                                            app.details.scroll = app.details.scroll.saturating_add(1);
                                        }
                                        KeyCode::PageDown => {
                                            app.details.scroll = app.details.scroll.saturating_add(10);
                                        }
                                        KeyCode::PageUp => {
                                            app.details.scroll = app.details.scroll.saturating_sub(10);
                                        }
                                        KeyCode::Home | KeyCode::Char('g') => {
                                            app.details.scroll = 0;
                                        }
                                        KeyCode::End | KeyCode::Char('G') => {
                                            app.details.scroll = u16::MAX;
                                        }
                                        // Tab switching
                                        KeyCode::Char('[') => app.prev_details_tab(),
                                        KeyCode::Char(']') => app.next_details_tab(),
                                        _ => {}
                                    },
                                }
                            }
                        }
                    }
                    AppState::Searching => match key.code {
                        KeyCode::Esc => app.cancel_search(),
                        KeyCode::Enter => app.confirm_search(),
                        KeyCode::Backspace => {
                            app.core.search_query_pop();
                            app.execute_search();
                        }
                        KeyCode::Char(c) => {
                            app.core.search_query_push(c);
                            app.execute_search();
                        }
                        _ => {}
                    },
                    AppState::ShowingMarkConfirm => match key.code {
                        KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => {
                            app.confirm_mark();
                        }
                        KeyCode::Char('n') | KeyCode::Esc => app.cancel_mark(),
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_mark_confirm(-1),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_mark_confirm(1),
                        KeyCode::PageUp => app.scroll_mark_confirm(-10),
                        KeyCode::PageDown => app.scroll_mark_confirm(10),
                        _ => {}
                    },
                    AppState::ShowingChanges => match key.code {
                        KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => {
                            terminal.clear()?;
                            app.commit_changes_live()?;
                            terminal.clear()?;
                        }
                        KeyCode::Char('n') | KeyCode::Esc => {
                            app.state = AppState::Listing;
                            app.refresh_ui_state();
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_changes(-1),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_changes(1),
                        KeyCode::PageUp => app.scroll_changes(-10),
                        KeyCode::PageDown => app.scroll_changes(10),
                        _ => {}
                    },
                    AppState::ShowingChangelog => match key.code {
                        KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => {
                            app.state = AppState::Listing;
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_changelog(-1),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_changelog(1),
                        KeyCode::PageUp => app.scroll_changelog(-10),
                        KeyCode::PageDown => app.scroll_changelog(10),
                        _ => {}
                    },
                    AppState::ConfirmExit => match key.code {
                        KeyCode::Char('y') | KeyCode::Enter | KeyCode::Char(' ') => break,
                        KeyCode::Char('n') | KeyCode::Esc => {
                            app.state = AppState::Listing;
                        }
                        _ => {}
                    },
                    AppState::ShowingSettings => match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            app.state = AppState::Listing;
                            app.apply_current_filter();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if app.settings_selection > 0 {
                                app.settings_selection -= 1;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if app.settings_selection < App::settings_item_count() - 1 {
                                app.settings_selection += 1;
                            }
                        }
                        KeyCode::Enter | KeyCode::Char(' ') => {
                            app.toggle_setting();
                        }
                        _ => {}
                    },
                    AppState::Upgrading => {}
                    AppState::Done => match key.code {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('r') => {
                            app.state = AppState::Listing;
                            if let Err(e) = app.refresh_cache() {
                                app.status_message = format!("Refresh failed: {e}");
                            }
                        }
                        KeyCode::Up | KeyCode::Char('k') => app.scroll_output(-1),
                        KeyCode::Down | KeyCode::Char('j') => app.scroll_output(1),
                        KeyCode::PageUp => app.scroll_output(-10),
                        KeyCode::PageDown => app.scroll_output(10),
                        _ => {}
                    },
                }
            }
    }

    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
