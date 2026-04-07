mod app;
mod ui;

use std::io;

use color_eyre::Result;
use ratatui::prelude::*;
use termion::event::Key;
use termion::input::TermRead;
use termion::raw::IntoRawMode;
use termion::screen::IntoAlternateScreen;

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

    let stdout = io::stdout().into_raw_mode()?.into_alternate_screen()?;
    let mut terminal = Terminal::new(TermionBackend::new(stdout))?;

    let _hotpath = hotpath::HotpathGuardBuilder::new("synh8")
        .percentiles(&[50, 95, 99])
        .with_functions_limit(0)
        .build();

    let mut app = App::new()?;
    let stdin = termion::async_stdin();
    let mut keys = stdin.keys();

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Some(key_result) = keys.next() {
            let key = key_result?;

            match app.state {
                AppState::Listing => {
                    // === Global keys (any focused pane) ===
                    match key {
                        Key::Ctrl('c') => {
                            break; // Ctrl-c: quit immediately, no confirmation
                        }
                        Key::Char('q') => {
                            if app.has_pending_changes() {
                                app.state = AppState::ConfirmExit;
                            } else {
                                break;
                            }
                        }
                        Key::BackTab => app.cycle_focus_back(),
                        Key::Char('\t') => app.cycle_focus(),
                        Key::Char('s') => app.start_search(),
                        Key::F(2) => app.show_settings(),
                        Key::Char('u') => {
                            app.update_packages_live()?;
                            terminal.clear()?;
                        }
                        Key::Esc => {
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
                                FocusedPane::Filters => match key {
                                    Key::Up => app.move_filter_selection(-1),
                                    Key::Down => app.move_filter_selection(1),
                                    Key::PageUp => app.move_filter_selection(-1),
                                    Key::PageDown => app.move_filter_selection(1),
                                    Key::Home => app.select_first_filter(),
                                    Key::End => app.select_last_filter(),
                                    _ => {}
                                },
                                FocusedPane::Packages => match key {
                                    Key::Up => {
                                        app.move_package_selection(-1);
                                        app.update_visual_selection();
                                    }
                                    Key::Down => {
                                        app.move_package_selection(1);
                                        app.update_visual_selection();
                                    }
                                    Key::PageDown => {
                                        app.move_package_selection(10);
                                        app.update_visual_selection();
                                    }
                                    Key::PageUp => {
                                        app.move_package_selection(-10);
                                        app.update_visual_selection();
                                    }
                                    Key::Home => {
                                        app.select_first_package();
                                        app.update_visual_selection();
                                    }
                                    Key::End => {
                                        app.select_last_package();
                                        app.update_visual_selection();
                                    }
                                    Key::Char(' ') => {
                                        if app.ui.visual_mode {
                                            app.toggle_multi_select();
                                        } else {
                                            app.toggle_current();
                                        }
                                    }
                                    Key::Char('v') => app.start_visual_mode(),
                                    Key::Char('c') => app.show_changelog(),
                                    Key::Char('a') => app.show_changes_preview(),
                                    Key::Char('x') => app.mark_all_upgrades(),
                                    Key::Char('z') => app.unmark_all(),
                                    _ => {}
                                },
                                FocusedPane::Details => match key {
                                    Key::Up => {
                                        app.details.scroll = app.details.scroll.saturating_sub(1);
                                    }
                                    Key::Down => {
                                        app.details.scroll = app.details.scroll.saturating_add(1);
                                    }
                                    Key::PageDown => {
                                        app.details.scroll = app.details.scroll.saturating_add(10);
                                    }
                                    Key::PageUp => {
                                        app.details.scroll = app.details.scroll.saturating_sub(10);
                                    }
                                    Key::Home => {
                                        app.details.scroll = 0;
                                    }
                                    Key::End => {
                                        app.details.scroll = u16::MAX;
                                    }
                                    Key::Char(',') => app.prev_details_tab(),
                                    Key::Char('.') => app.next_details_tab(),
                                    _ => {}
                                },
                            }
                        }
                    }
                }
                AppState::Searching => match key {
                    Key::Esc => app.cancel_search(),
                    Key::Char('\n') => app.confirm_search(),
                    Key::Backspace => {
                        app.core.search_query_pop();
                        app.execute_search();
                    }
                    Key::Up | Key::Down
                    | Key::PageUp | Key::PageDown => {
                        app.confirm_search();
                        app.ui.focused_pane = FocusedPane::Packages;
                        let delta = match key {
                            Key::Up => -1,
                            Key::Down => 1,
                            Key::PageUp => -10,
                            Key::PageDown => 10,
                            _ => unreachable!(),
                        };
                        app.move_package_selection(delta);
                    }
                    Key::Char(c) => {
                        app.core.search_query_push(c);
                        app.execute_search();
                    }
                    _ => {}
                },
                AppState::ShowingMarkConfirm => match key {
                    Key::Char(' ') => app.confirm_mark(),
                    Key::Esc => app.cancel_mark(),
                    Key::Up => app.scroll_mark_confirm(-1),
                    Key::Down => app.scroll_mark_confirm(1),
                    Key::PageUp => app.scroll_mark_confirm(-10),
                    Key::PageDown => app.scroll_mark_confirm(10),
                    _ => {}
                },
                AppState::ShowingChanges => match key {
                    Key::Char(' ') => {
                        terminal.clear()?;
                        app.commit_changes_live()?;
                        terminal.clear()?;
                    }
                    Key::Esc => {
                        app.state = AppState::Listing;
                        app.refresh_ui_state();
                    }
                    Key::Up => app.scroll_changes(-1),
                    Key::Down => app.scroll_changes(1),
                    Key::PageUp => app.scroll_changes(-10),
                    Key::PageDown => app.scroll_changes(10),
                    _ => {}
                },
                AppState::ShowingChangelog => match key {
                    Key::Esc | Key::Char(' ') => {
                        app.state = AppState::Listing;
                    }
                    Key::Up => app.scroll_changelog(-1),
                    Key::Down => app.scroll_changelog(1),
                    Key::PageUp => app.scroll_changelog(-10),
                    Key::PageDown => app.scroll_changelog(10),
                    _ => {}
                },
                AppState::ConfirmExit => match key {
                    Key::Char(' ') => break,
                    Key::Esc => {
                        app.state = AppState::Listing;
                    }
                    _ => {}
                },
                AppState::ShowingSettings => match key {
                    Key::Esc => {
                        app.state = AppState::Listing;
                        app.apply_current_filter();
                    }
                    Key::Up => {
                        if app.settings_selection > 0 {
                            app.settings_selection -= 1;
                        }
                    }
                    Key::Down => {
                        if app.settings_selection < App::settings_item_count() - 1 {
                            app.settings_selection += 1;
                        }
                    }
                    Key::Char(' ') => {
                        app.toggle_setting();
                    }
                    _ => {}
                },
                AppState::Upgrading => {}
                AppState::Done => match key {
                    Key::Esc | Key::Char(' ') => {
                        app.state = AppState::Listing;
                        if let Err(e) = app.refresh_cache() {
                            app.status_message = format!("Refresh failed: {e}");
                        }
                    }
                    Key::Up => app.scroll_output(-1),
                    Key::Down => app.scroll_output(1),
                    Key::PageUp => app.scroll_output(-10),
                    Key::PageDown => app.scroll_output(10),
                    _ => {}
                },
            }
        } else if app.warm_step.is_some() {
            // No input available: do incremental warm-up work
            app.warm_next();
        } else {
            // Idle: avoid busy-waiting
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    }

    // Terminal cleanup is handled by Drop (RawTerminal + AlternateScreen)
    Ok(())
}
