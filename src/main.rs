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
                                break; // Ctrl-c: quit immediately, no confirmation
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
                            KeyCode::Char('s') => app.start_search(),
                            KeyCode::F(2) => app.show_settings(),
                            KeyCode::Char('u') => {
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
                                        KeyCode::Up => app.move_filter_selection(-1),
                                        KeyCode::Down => app.move_filter_selection(1),
                                        KeyCode::PageUp => app.move_filter_selection(-1),
                                        KeyCode::PageDown => app.move_filter_selection(1),
                                        KeyCode::Home => app.select_first_filter(),
                                        KeyCode::End => app.select_last_filter(),
                                        _ => {}
                                    },
                                    FocusedPane::Packages => match key.code {
                                        KeyCode::Up => {
                                            app.move_package_selection(-1);
                                            app.update_visual_selection();
                                        }
                                        KeyCode::Down => {
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
                                        KeyCode::Home => {
                                            app.select_first_package();
                                            app.update_visual_selection();
                                        }
                                        KeyCode::End => {
                                            app.select_last_package();
                                            app.update_visual_selection();
                                        }
                                        KeyCode::Char(' ') => {
                                            if app.ui.visual_mode {
                                                app.toggle_multi_select();
                                            } else {
                                                app.toggle_current();
                                            }
                                        }
                                        KeyCode::Char('v') => app.start_visual_mode(),
                                        KeyCode::Char('c') => app.show_changelog(),
                                        KeyCode::Char('a') => app.show_changes_preview(),
                                        KeyCode::Char('x') => app.mark_all_upgrades(),
                                        KeyCode::Char('z') => app.unmark_all(),
                                        _ => {}
                                    },
                                    FocusedPane::Details => match key.code {
                                        KeyCode::Up => {
                                            app.details.scroll = app.details.scroll.saturating_sub(1);
                                        }
                                        KeyCode::Down => {
                                            app.details.scroll = app.details.scroll.saturating_add(1);
                                        }
                                        KeyCode::PageDown => {
                                            app.details.scroll = app.details.scroll.saturating_add(10);
                                        }
                                        KeyCode::PageUp => {
                                            app.details.scroll = app.details.scroll.saturating_sub(10);
                                        }
                                        KeyCode::Home => {
                                            app.details.scroll = 0;
                                        }
                                        KeyCode::End => {
                                            app.details.scroll = u16::MAX;
                                        }
                                        KeyCode::Char(',') => app.prev_details_tab(),
                                        KeyCode::Char('.') => app.next_details_tab(),
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
                        KeyCode::Up | KeyCode::Down
                        | KeyCode::PageUp | KeyCode::PageDown => {
                            app.confirm_search();
                            app.ui.focused_pane = FocusedPane::Packages;
                            let delta = match key.code {
                                KeyCode::Up => -1,
                                KeyCode::Down => 1,
                                KeyCode::PageUp => -10,
                                KeyCode::PageDown => 10,
                                _ => unreachable!(),
                            };
                            app.move_package_selection(delta);
                        }
                        KeyCode::Char(c) => {
                            app.core.search_query_push(c);
                            app.execute_search();
                        }
                        _ => {}
                    },
                    AppState::ShowingMarkConfirm => match key.code {
                        KeyCode::Char(' ') => app.confirm_mark(),
                        KeyCode::Esc => app.cancel_mark(),
                        KeyCode::Up => app.scroll_mark_confirm(-1),
                        KeyCode::Down => app.scroll_mark_confirm(1),
                        KeyCode::PageUp => app.scroll_mark_confirm(-10),
                        KeyCode::PageDown => app.scroll_mark_confirm(10),
                        _ => {}
                    },
                    AppState::ShowingChanges => match key.code {
                        KeyCode::Char(' ') => {
                            terminal.clear()?;
                            app.commit_changes_live()?;
                            terminal.clear()?;
                        }
                        KeyCode::Esc => {
                            app.state = AppState::Listing;
                            app.refresh_ui_state();
                        }
                        KeyCode::Up => app.scroll_changes(-1),
                        KeyCode::Down => app.scroll_changes(1),
                        KeyCode::PageUp => app.scroll_changes(-10),
                        KeyCode::PageDown => app.scroll_changes(10),
                        _ => {}
                    },
                    AppState::ShowingChangelog => match key.code {
                        KeyCode::Esc | KeyCode::Char(' ') => {
                            app.state = AppState::Listing;
                        }
                        KeyCode::Up => app.scroll_changelog(-1),
                        KeyCode::Down => app.scroll_changelog(1),
                        KeyCode::PageUp => app.scroll_changelog(-10),
                        KeyCode::PageDown => app.scroll_changelog(10),
                        _ => {}
                    },
                    AppState::ConfirmExit => match key.code {
                        KeyCode::Char(' ') => break,
                        KeyCode::Esc => {
                            app.state = AppState::Listing;
                        }
                        _ => {}
                    },
                    AppState::ShowingSettings => match key.code {
                        KeyCode::Esc => {
                            app.state = AppState::Listing;
                            app.apply_current_filter();
                        }
                        KeyCode::Up => {
                            if app.settings_selection > 0 {
                                app.settings_selection -= 1;
                            }
                        }
                        KeyCode::Down => {
                            if app.settings_selection < App::settings_item_count() - 1 {
                                app.settings_selection += 1;
                            }
                        }
                        KeyCode::Char(' ') => {
                            app.toggle_setting();
                        }
                        _ => {}
                    },
                    AppState::Upgrading => {}
                    AppState::Done => match key.code {
                        KeyCode::Esc | KeyCode::Char(' ') => {
                            app.state = AppState::Listing;
                            if let Err(e) = app.refresh_cache() {
                                app.status_message = format!("Refresh failed: {e}");
                            }
                        }
                        KeyCode::Up => app.scroll_output(-1),
                        KeyCode::Down => app.scroll_output(1),
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
