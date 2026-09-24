//! UI rendering functions

use ratatui::prelude::*;
use ratatui::widgets::{
    Block, Borders, Cell, Clear, List, ListItem, Paragraph, Row, Scrollbar, ScrollbarOrientation,
    ScrollbarState, Table, TableState, Wrap,
};

use crate::app::App;
use synh8::keymap::{self, Action, Context};
use synh8::progress::centered_rect;
use synh8::types::*;

#[hotpath::measure]
pub fn ui(frame: &mut Frame, app: &mut App) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(frame.area());

    render_title(frame, app, main_chunks[0]);

    match app.state {
        AppState::Listing
        | AppState::Searching
        | AppState::ShowingMarkConfirm
        | AppState::ConfirmExit => {
            // Three-pane base layout (shared by listing and its modal overlays)
            let panes = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(24),
                    Constraint::Min(40),
                    Constraint::Length(35),
                ])
                .split(main_chunks[1]);

            render_filter_pane(frame, app, panes[0]);
            render_package_table(frame, app, panes[1]);
            render_details_pane(frame, app, panes[2]);

            match app.state {
                AppState::ShowingMarkConfirm => {
                    render_mark_preview_modal(frame, app, main_chunks[1]);
                }
                AppState::ConfirmExit => render_exit_confirm_modal(frame, main_chunks[1]),
                _ => {}
            }
        }
        AppState::ShowingChanges => render_changes_modal(frame, app, main_chunks[1]),
        AppState::ShowingChangelog => render_changelog_view(frame, app, main_chunks[1]),
        AppState::ShowingSettings => render_settings_view(frame, app, main_chunks[1]),
        AppState::Done => render_output_view(frame, app, main_chunks[1]),
    }

    render_status_bar(frame, app, main_chunks[2]);

    let mut help_stack = app.help_contexts();
    if app.state == AppState::Listing && !app.ui.visual_mode {
        help_stack.push(Context::Global);
    }
    let help = Paragraph::new(keymap::help_line(&help_stack))
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);
    frame.render_widget(help, main_chunks[3]);
}

fn render_title(frame: &mut Frame, app: &App, area: Rect) {
    let changes_count = app.core.planned_changes().map_or(0, <[_]>::len);
    let title_text = if changes_count > 0 {
        format!(
            " synh8 │ {} changes │ {} download ",
            changes_count,
            size_str(app.core.download_size())
        )
    } else if app.core.has_intents() {
        format!(" synh8 │ {} marked ", app.core.intent_count())
    } else {
        " synh8 │ No changes pending ".to_string()
    };
    let title =
        Paragraph::new(title_text).style(Style::default().fg(Color::White).bg(Color::Blue).bold());
    frame.render_widget(title, area);
}

fn render_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let status_style = match app.state {
        AppState::Listing | AppState::ShowingSettings => Style::default().fg(Color::Yellow),
        AppState::Searching => Style::default().fg(Color::White),
        AppState::ShowingMarkConfirm => Style::default().fg(Color::Magenta),
        AppState::ShowingChanges | AppState::ShowingChangelog => Style::default().fg(Color::Cyan),
        AppState::ConfirmExit => Style::default().fg(Color::Red),
        AppState::Done => Style::default().fg(Color::Green),
    };

    let status_text = match app.state {
        AppState::Searching => format!("/{}", app.core.search_query()),
        _ if app.core.has_search_results() => format!(
            "[Search: {}] {}",
            app.core.search_query(),
            app.status_message
        ),
        _ => app.status_message.clone(),
    };
    let status = Paragraph::new(status_text)
        .style(status_style)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(status, area);

    if app.state == AppState::Searching {
        // Cursor after "/<query>" inside the border
        let query_width = Line::from(app.core.search_query()).width() as u16;
        frame.set_cursor_position((area.x + 2 + query_width, area.y + 1));
    }
}

fn pane_border(focused: bool) -> Style {
    if focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    }
}

fn render_filter_pane(frame: &mut Frame, app: &mut App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(7), Constraint::Length(9)])
        .split(area);

    let items: Vec<ListItem> = FilterCategory::all()
        .iter()
        .map(|cat| {
            let label = format!("{} ({})", cat.label(), app.core.filter_count(*cat));
            let style = if *cat == app.core.selected_filter() {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            ListItem::new(label).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Filters ")
                .borders(Borders::ALL)
                .border_style(pane_border(app.ui.focused_pane == FocusedPane::Filters)),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[0], &mut app.ui.filter_state);

    let legend = vec![
        Line::from(vec![
            Span::styled("↑", Style::default().fg(Color::Yellow)),
            Span::raw(" Upgradable"),
        ]),
        Line::from(vec![
            Span::styled("↑", Style::default().fg(Color::Green)),
            Span::raw(" Upgrade"),
        ]),
        Line::from(vec![
            Span::styled("↑", Style::default().fg(Color::Cyan)),
            Span::raw(" Auto-upg"),
        ]),
        Line::from(vec![
            Span::styled("+", Style::default().fg(Color::Green)),
            Span::raw(" Install"),
        ]),
        Line::from(vec![
            Span::styled("+", Style::default().fg(Color::Cyan)),
            Span::raw(" Auto-inst"),
        ]),
        Line::from(vec![
            Span::styled("-", Style::default().fg(Color::Red)),
            Span::raw(" Remove"),
        ]),
        Line::from(vec![
            Span::styled("·", Style::default().fg(Color::DarkGray)),
            Span::raw(" Installed"),
        ]),
    ];

    let legend_widget = Paragraph::new(legend).block(
        Block::default()
            .title(" Legend ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(legend_widget, chunks[1]);
}

#[hotpath::measure]
fn render_package_table(frame: &mut Frame, app: &mut App, area: Rect) {
    let visible_cols = Column::visible_columns(&app.settings);

    let header_cells: Vec<Cell> = visible_cols
        .iter()
        .map(|col| Cell::from(col.header()).style(Style::default().fg(Color::Cyan).bold()))
        .collect();
    let header = Row::new(header_cells).height(1);

    // Visible window: area height minus 2 borders minus the header row.
    let visible_rows = area.height.saturating_sub(3) as usize;
    if app.ui.table_visible_rows != visible_rows {
        app.ui.table_visible_rows = visible_rows;
        app.center_scroll_offset();
    }

    let list = app.core.list();
    let total_count = list.len();

    // Read absolute offset from app state (set by center_scroll_offset).
    // Clamp to handle stale offsets when the list shrinks between frames.
    let offset = app
        .ui
        .table_state
        .offset()
        .min(total_count.saturating_sub(1));
    let end = (offset + visible_rows).min(total_count);
    let visible_slice = &list[offset..end];

    // Build rows only for the visible window.
    let rows: Vec<Row> = visible_slice
        .iter()
        .enumerate()
        .map(|(local_idx, pkg)| {
            let abs_idx = offset + local_idx;
            let is_multi_selected = app
                .ui
                .visual_range
                .is_some_and(|(start, end)| (start..=end).contains(&abs_idx));
            let is_user_marked = app.core.intent_of(pkg.id).is_some();

            let cells: Vec<Cell> = visible_cols
                .iter()
                .map(|col| match col {
                    Column::Status => Cell::from(pkg.status.symbol())
                        .style(Style::default().fg(pkg.status.color())),
                    Column::Name => {
                        let style = if is_user_marked {
                            Style::default().fg(Color::White).bold()
                        } else {
                            Style::default()
                        };
                        Cell::from(app.core.cache().display_name(&pkg.name)).style(style)
                    }
                    Column::Section => Cell::from(pkg.section.as_str()),
                    Column::InstalledVersion => {
                        if pkg.installed_version.is_empty() {
                            Cell::from("-")
                        } else {
                            Cell::from(pkg.installed_version.as_str())
                        }
                    }
                    Column::CandidateVersion => Cell::from(pkg.candidate_version.as_str())
                        .style(Style::default().fg(Color::Green)),
                    Column::DownloadSize => Cell::from(pkg.download_size_str()),
                })
                .collect();

            let row = Row::new(cells);
            if is_multi_selected {
                row.style(Style::default().bg(Color::Blue))
            } else {
                row
            }
        })
        .collect();

    let widths: Vec<Constraint> = visible_cols
        .iter()
        .map(|col| col.width(&app.col_widths))
        .collect();

    // Temporary TableState for the sliced row set: offset = 0, selection
    // translated to slice-relative index. Never written back to app state.
    let relative_selected = app
        .ui
        .table_state
        .selected()
        .filter(|abs| (offset..end).contains(abs))
        .map(|abs| abs - offset);
    let mut temp_table_state = TableState::default();
    temp_table_state.select(relative_selected);

    let table = Table::new(rows, widths)
        .header(header)
        .block(
            Block::default()
                .title(format!(" Packages ({total_count}) "))
                .borders(Borders::ALL)
                .border_style(pane_border(app.ui.focused_pane == FocusedPane::Packages)),
        )
        .row_highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(table, area, &mut temp_table_state);

    // Scrollbar uses absolute indices from the original app table state.
    if total_count > 0 && area.width > 0 {
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        let mut scrollbar_state =
            ScrollbarState::new(total_count).position(app.ui.table_state.selected().unwrap_or(0));

        let scrollbar_area = Rect {
            x: area.x + area.width - 1,
            y: area.y + 1,
            width: 1,
            height: area.height.saturating_sub(2),
        };
        frame.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
    }
}

/// Render a bordered, wrapped, scrollable paragraph, recording its real
/// viewport and (wrapped) content height in `scroll`.
fn render_scrollable(
    frame: &mut Frame,
    area: Rect,
    block: Block,
    lines: Vec<Line>,
    scroll: &mut ScrollView,
) {
    let inner = block.inner(area);
    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: false });
    scroll.measure(inner.height as usize, paragraph.line_count(inner.width));
    frame.render_widget(block, area);
    frame.render_widget(paragraph.scroll((scroll.offset_u16(), 0)), inner);
}

fn heading(text: String) -> Line<'static> {
    Line::from(Span::styled(text, Style::default().fg(Color::Cyan).bold()))
}

fn dep_lines(deps: &[(rust_apt::DepType, String)]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    let mut current: Option<&rust_apt::DepType> = None;
    for (dep_type, target) in deps {
        if current != Some(dep_type) {
            if current.is_some() {
                lines.push(Line::from(""));
            }
            lines.push(heading(format!("{dep_type}:")));
            current = Some(dep_type);
        }
        lines.push(Line::from(format!("  {target}")));
    }
    lines
}

fn render_details_pane(frame: &mut Frame, app: &mut App, area: Rect) {
    let tab_style = |tab| {
        if app.details.tab == tab {
            Style::default().fg(Color::Yellow).bold()
        } else {
            Style::default().fg(Color::DarkGray)
        }
    };

    let mut content = vec![
        Line::from(vec![
            Span::styled("[Info]", tab_style(DetailsTab::Info)),
            Span::raw(" "),
            Span::styled("[Deps]", tab_style(DetailsTab::Dependencies)),
            Span::raw(" "),
            Span::styled("[RDeps]", tab_style(DetailsTab::ReverseDeps)),
        ]),
        Line::from(Span::styled(
            format!(
                "  ({} / {} to switch)",
                keymap::key_label(Context::Details, Action::PrevTab),
                keymap::key_label(Context::Details, Action::NextTab)
            ),
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
    ];

    match app.selected_package() {
        None => content.push(Line::from(Span::styled(
            "No package selected",
            Style::default().fg(Color::DarkGray),
        ))),
        Some(pkg) => match app.details.tab {
            DetailsTab::Info => {
                let field = |name: &'static str, value: String| {
                    Line::from(vec![
                        Span::styled(name, Style::default().fg(Color::Cyan)),
                        Span::raw(value),
                    ])
                };
                let installed = if pkg.installed_version.is_empty() {
                    "(none)".to_string()
                } else {
                    pkg.installed_version.clone()
                };
                content.extend([
                    Line::from(vec![
                        Span::styled("Package: ", Style::default().fg(Color::Cyan).bold()),
                        Span::raw(app.core.cache().display_name(&pkg.name).to_string()),
                    ]),
                    Line::from(""),
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::Cyan)),
                        Span::styled(pkg.status.symbol(), Style::default().fg(pkg.status.color())),
                        Span::raw(format!(" {}", pkg.status.label())),
                    ]),
                    field("Section: ", pkg.section.clone()),
                    field("Arch: ", pkg.architecture.clone()),
                    Line::from(""),
                    field("Installed: ", installed),
                    Line::from(vec![
                        Span::styled("Candidate: ", Style::default().fg(Color::Green)),
                        Span::raw(pkg.candidate_version.clone()),
                    ]),
                    Line::from(""),
                    field("Download: ", pkg.download_size_str()),
                    field("Inst Size: ", pkg.installed_size_str()),
                    Line::from(""),
                    Line::from(Span::styled(
                        "Description:",
                        Style::default().fg(Color::Cyan).bold(),
                    )),
                    Line::from(pkg.description.clone()),
                ]);
            }
            DetailsTab::Dependencies => {
                if app.details.cached_deps.is_empty() {
                    content.push(Line::from(Span::styled(
                        "No dependencies",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    content.extend(dep_lines(&app.details.cached_deps));
                }
            }
            DetailsTab::ReverseDeps => {
                if app.details.cached_rdeps.is_empty() {
                    content.push(Line::from(Span::styled(
                        "No reverse dependencies",
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    content.push(heading(format!(
                        "{} packages depend on this:",
                        app.details.cached_rdeps.len()
                    )));
                    content.push(Line::from(""));
                    content.extend(dep_lines(&app.details.cached_rdeps));
                }
            }
        },
    }

    let title = match app.details.tab {
        DetailsTab::Info => " Details ",
        DetailsTab::Dependencies => " Dependencies ",
        DetailsTab::ReverseDeps => " Reverse Deps ",
    };
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(pane_border(app.ui.focused_pane == FocusedPane::Details));
    let content: Vec<Line<'static>> = content.into_iter().map(owned_line).collect();
    render_scrollable(frame, area, block, content, &mut app.details.scroll);
}

/// Detach a line from the data it borrows
fn owned_line(line: Line<'_>) -> Line<'static> {
    Line::from(
        line.spans
            .into_iter()
            .map(|s| Span::styled(s.content.into_owned(), s.style))
            .collect::<Vec<_>>(),
    )
    .style(line.style)
}

/// Resolver errors and warnings as modal lines
fn problem_lines(app: &App) -> Vec<Line<'static>> {
    let Some(problems) = app.core.plan_problems() else {
        return Vec::new();
    };
    let mut lines = Vec::new();
    if !problems.errors.is_empty() {
        lines.push(Line::from(Span::styled(
            "Cannot apply - the resolver reported errors:",
            Style::default().fg(Color::Red).bold(),
        )));
        for e in &problems.errors {
            lines.push(Line::from(Span::styled(
                format!("  {e}"),
                Style::default().fg(Color::Red),
            )));
        }
        lines.push(Line::from(""));
    }
    if !problems.warnings.is_empty() {
        lines.push(Line::from(Span::styled(
            "Warnings:",
            Style::default().fg(Color::Yellow).bold(),
        )));
        for w in &problems.warnings {
            lines.push(Line::from(format!("  {w}")));
        }
        lines.push(Line::from(""));
    }
    lines
}

/// Section heading style for a group of changes
fn change_group_style(action: ChangeAction, reason: ChangeReason) -> Style {
    let color = match (action, reason) {
        (ChangeAction::Upgrade, ChangeReason::UserRequested) => Color::Yellow,
        (ChangeAction::Install | ChangeAction::Reinstall, ChangeReason::UserRequested) => {
            Color::Green
        }
        (ChangeAction::Remove, ChangeReason::UserRequested) => Color::Red,
        (ChangeAction::Remove, ChangeReason::Dependency) | (ChangeAction::Downgrade, _) => {
            Color::Magenta
        }
        _ => Color::Cyan,
    };
    Style::default().fg(color).bold()
}

fn render_changes_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let modal_area = centered_rect(
        area,
        70.min(area.width.saturating_sub(4)),
        area.height.saturating_sub(2),
    );
    frame.render_widget(Clear, modal_area);

    let mut lines = problem_lines(app);
    lines.push(Line::from(Span::styled(
        "The following changes will be made:",
        Style::default().bold(),
    )));
    lines.push(Line::from(""));

    let changes = app.core.planned_changes().unwrap_or_default();
    let cache = app.core.cache();
    for &action in ChangeAction::all() {
        for reason in [ChangeReason::UserRequested, ChangeReason::Dependency] {
            let group: Vec<&PlannedChange> = changes
                .iter()
                .filter(|c| c.action == action && c.reason == reason)
                .collect();
            if group.is_empty() {
                continue;
            }
            let suffix = match reason {
                ChangeReason::UserRequested => "",
                ChangeReason::Dependency => " (required by other changes)",
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "{}{suffix} ({}):",
                    action.label().to_uppercase(),
                    group.len()
                ),
                change_group_style(action, reason),
            )));
            for c in group {
                lines.push(Line::from(format!(
                    "  {} {}",
                    action.status().symbol(),
                    cache.display_name_of(c.package)
                )));
            }
            lines.push(Line::from(""));
        }
    }
    if changes.is_empty() {
        lines.push(Line::from(
            "Nothing to do: every mark is already satisfied.",
        ));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(format!(
        "Download size: {}",
        size_str(app.core.download_size())
    )));
    lines.push(Line::from(format!(
        "Disk space change: {}",
        size_change_str(app.core.install_size_change())
    )));

    let block = Block::default()
        .title(" Confirm Changes ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    render_scrollable(frame, modal_area, block, lines, &mut app.modals.changes);
}

fn render_changelog_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let lines: Vec<Line> = app
        .modals
        .changelog_content
        .iter()
        .map(|s| Line::from(s.clone()))
        .collect();
    let block = Block::default()
        .title(format!(" Changelog: {} ", app.modals.changelog_title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    render_scrollable(frame, area, block, lines, &mut app.modals.changelog);
}

fn render_output_view(frame: &mut Frame, app: &mut App, area: Rect) {
    let lines: Vec<Line> = app
        .output_lines
        .iter()
        .map(|s| Line::from(s.clone()))
        .collect();
    let block = Block::default()
        .title(" APT Output ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    render_scrollable(frame, area, block, lines, &mut app.modals.output);
}

fn render_settings_view(frame: &mut Frame, app: &App, area: Rect) {
    let all_cols = Column::all();
    let col_count = all_cols.len();
    let selected_style = |idx: usize| {
        if idx == app.settings_selection {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        }
    };

    let mut items: Vec<ListItem> = all_cols
        .iter()
        .enumerate()
        .map(|(idx, col)| {
            let checkbox = if app.settings.visible_columns.contains(col) {
                "[X]"
            } else {
                "[ ]"
            };
            ListItem::new(format!("{checkbox} {}", col.label())).style(selected_style(idx))
        })
        .collect();

    items.push(ListItem::new(""));
    items.push(
        ListItem::new(format!("Sort by: {}", app.settings.sort.sort_by.label()))
            .style(selected_style(col_count)),
    );
    let order = if app.settings.sort.ascending {
        "Ascending"
    } else {
        "Descending"
    };
    items.push(ListItem::new(format!("Sort order: {order}")).style(selected_style(col_count + 1)));

    let settings_list = List::new(items).block(
        Block::default()
            .title(" Settings ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(settings_list, area);
}

fn render_mark_preview_modal(frame: &mut Frame, app: &mut App, area: Rect) {
    let Some(preview) = &app.mark_preview else {
        return;
    };

    let modal_area = centered_rect(
        area,
        60.min(area.width.saturating_sub(4)),
        20.min(area.height.saturating_sub(4)),
    );
    frame.render_widget(Clear, modal_area);

    let mut lines = vec![
        Line::from(Span::styled(
            preview.headline.clone(),
            Style::default().bold(),
        )),
        Line::from(""),
    ];

    for &action in ChangeAction::all() {
        let names: Vec<&String> = preview
            .added
            .iter()
            .filter(|(_, a)| *a == action)
            .map(|(n, _)| n)
            .collect();
        if names.is_empty() {
            continue;
        }
        lines.push(Line::from(Span::styled(
            format!(
                "This also marks {} for {}:",
                names.len(),
                action.label().to_lowercase()
            ),
            Style::default().fg(action.status().color()),
        )));
        for name in names {
            lines.push(Line::from(format!("  {} {name}", action.status().symbol())));
        }
        lines.push(Line::from(""));
    }

    if !preview.dropped.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("This also unmarks {} packages:", preview.dropped.len()),
            Style::default().fg(Color::Yellow),
        )));
        for name in &preview.dropped {
            lines.push(Line::from(format!("  {name}")));
        }
        lines.push(Line::from(""));
    }

    if preview.download_size > 0 {
        lines.push(Line::from(Span::styled(
            format!("Download size: {}", size_str(preview.download_size)),
            Style::default().fg(Color::Cyan),
        )));
    }
    lines.extend(problem_lines(app));

    let block = Block::default()
        .title(" Confirm ")
        .title_bottom(
            Line::from(format!(" {} ", keymap::help_line(&[Context::MarkConfirm]))).centered(),
        )
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    render_scrollable(
        frame,
        modal_area,
        block,
        lines,
        &mut app.modals.mark_preview,
    );
}

fn render_exit_confirm_modal(frame: &mut Frame, area: Rect) {
    let modal_area = centered_rect(area, 50.min(area.width.saturating_sub(4)), 7);
    frame.render_widget(Clear, modal_area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            "You have unapplied changes!",
            Style::default().fg(Color::Red).bold(),
        )),
        Line::from(""),
        Line::from("Really quit without applying?"),
        Line::from(""),
        Line::from(Span::styled(
            keymap::help_line(&[Context::ConfirmExit]),
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let modal = Paragraph::new(lines)
        .block(
            Block::default()
                .title(" Confirm Exit ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Red)),
        )
        .alignment(Alignment::Center);

    frame.render_widget(modal, modal_area);
}
