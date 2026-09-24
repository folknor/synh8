//! TUI progress display for APT download and install operations.
//!
//! Uses `Rc<RefCell<ProgressState>>` to share terminal access between
//! `TuiAcquireProgress` (download phase) and `TuiInstallProgress` (install phase).
//! Both phases render into the same ratatui terminal as a centered modal.
//!
//! The progress terminal writes to `/dev/tty` directly, while `StdioRedirect`
//! captures fd 1/2 so dpkg's output neither corrupts the screen nor is lost.

use std::cell::RefCell;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::os::fd::FromRawFd;
use std::rc::Rc;

use ratatui::Terminal;
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Gauge, Paragraph, Wrap};
use rust_apt::raw::{AcqTextStatus, ItemDesc, PkgAcquire};

use crate::types::size_str;

// ============================================================================
// Output capture
// ============================================================================

/// Redirects stdout/stderr into an anonymous in-memory file (memfd), so the
/// capture has no filesystem path another user could race or redirect.
/// Call `finish()` to restore the descriptors and collect the output; if the
/// guard is dropped instead, the descriptors are restored best-effort.
pub struct StdioRedirect {
    saved_stdout: libc::c_int,
    saved_stderr: libc::c_int,
    capture: File,
    restored: bool,
}

impl StdioRedirect {
    pub fn capture() -> std::io::Result<Self> {
        // SAFETY: plain fd syscalls; every fd opened here is either owned by
        // the returned guard or closed on the error path.
        unsafe {
            let fd = libc::memfd_create(c"synh8-apt-output".as_ptr(), libc::MFD_CLOEXEC);
            if fd == -1 {
                return Err(std::io::Error::last_os_error());
            }
            let capture = File::from_raw_fd(fd);

            let saved_stdout = libc::dup(libc::STDOUT_FILENO);
            if saved_stdout == -1 {
                return Err(std::io::Error::last_os_error());
            }
            let saved_stderr = libc::dup(libc::STDERR_FILENO);
            if saved_stderr == -1 {
                let err = std::io::Error::last_os_error();
                libc::close(saved_stdout);
                return Err(err);
            }
            let mut guard = Self {
                saved_stdout,
                saved_stderr,
                capture,
                restored: true,
            };
            // dup2 clears FD_CLOEXEC, so dpkg children inherit fds 1 and 2.
            if libc::dup2(fd, libc::STDOUT_FILENO) == -1 {
                return Err(std::io::Error::last_os_error());
            }
            guard.restored = false;
            if libc::dup2(fd, libc::STDERR_FILENO) == -1 {
                let err = std::io::Error::last_os_error();
                drop(guard.restore());
                return Err(err);
            }
            Ok(guard)
        }
    }

    fn restore(&mut self) -> std::io::Result<()> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        // SAFETY: the saved fds are owned by this guard and still open.
        unsafe {
            libc::fflush(std::ptr::null_mut());
            let out = libc::dup2(self.saved_stdout, libc::STDOUT_FILENO);
            let err = libc::dup2(self.saved_stderr, libc::STDERR_FILENO);
            if out == -1 || err == -1 {
                return Err(std::io::Error::last_os_error());
            }
        }
        Ok(())
    }

    /// Restore stdout/stderr and return the captured output. Output is
    /// returned even if restoring failed, alongside the error.
    pub fn finish(mut self) -> (Vec<String>, std::io::Result<()>) {
        let restored = self.restore();
        let mut bytes = Vec::new();
        let read = self
            .capture
            .seek(SeekFrom::Start(0))
            .and_then(|_| self.capture.read_to_end(&mut bytes));
        let lines = String::from_utf8_lossy(&bytes)
            .lines()
            .map(String::from)
            .collect();
        (lines, restored.and(read.map(drop)))
    }
}

impl Drop for StdioRedirect {
    fn drop(&mut self) {
        // Nowhere to report a failure from here; finish() is the checked path.
        drop(self.restore());
        // SAFETY: closing fds owned by this guard exactly once.
        unsafe {
            libc::close(self.saved_stdout);
            libc::close(self.saved_stderr);
        }
    }
}

// ============================================================================
// Progress state
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgressPhase {
    Downloading,
    Installing,
}

/// What the progress modal shows
struct ProgressView {
    phase: ProgressPhase,
    percent: f64,
    current_bytes: u64,
    total_bytes: u64,
    speed_bps: u64,
    install_steps_done: u64,
    install_total_steps: u64,
    install_action: String,
    errors: Vec<String>,
    title: String,
}

/// Shared progress state, owned by `Rc<RefCell<_>>`.
///
/// The terminal writes to `/dev/tty` directly, bypassing stdout, so dpkg's
/// output can be captured by `StdioRedirect` without affecting rendering.
pub struct ProgressState {
    terminal: Terminal<CrosstermBackend<File>>,
    view: ProgressView,
}

impl ProgressState {
    pub fn new(title: &str) -> std::io::Result<Self> {
        let tty = std::fs::OpenOptions::new().write(true).open("/dev/tty")?;
        let terminal = Terminal::new(CrosstermBackend::new(tty))?;
        Ok(Self {
            terminal,
            view: ProgressView {
                phase: ProgressPhase::Downloading,
                percent: 0.0,
                current_bytes: 0,
                total_bytes: 0,
                speed_bps: 0,
                install_steps_done: 0,
                install_total_steps: 0,
                install_action: String::new(),
                errors: Vec::new(),
                title: title.to_string(),
            },
        })
    }

    /// Errors reported by the download and install phases
    pub fn errors(&self) -> &[String] {
        &self.view.errors
    }

    fn draw(&mut self) {
        let view = &self.view;
        // A failed progress frame is cosmetic: the operation itself carries
        // on and reports its own result.
        drop(
            self.terminal
                .draw(|frame| render_progress_modal(frame, view)),
        );
    }
}

// ============================================================================
// DynAcquireProgress implementation
// ============================================================================

pub struct TuiAcquireProgress {
    state: Rc<RefCell<ProgressState>>,
}

impl TuiAcquireProgress {
    pub fn new(state: Rc<RefCell<ProgressState>>) -> Self {
        Self { state }
    }
}

impl rust_apt::progress::DynAcquireProgress for TuiAcquireProgress {
    fn pulse_interval(&self) -> usize {
        500_000 // 500ms
    }

    fn hit(&mut self, _item: &ItemDesc) {}

    fn fetch(&mut self, _item: &ItemDesc) {}

    fn done(&mut self, _item: &ItemDesc) {}

    fn fail(&mut self, item: &ItemDesc) {
        let owner = item.owner();
        let error_text = owner.error_text();
        if !error_text.is_empty() {
            let mut state = self.state.borrow_mut();
            state
                .view
                .errors
                .push(format!("{}: {error_text}", item.short_desc()));
            state.draw();
        }
    }

    fn pulse(&mut self, status: &AcqTextStatus, _owner: &PkgAcquire) {
        let mut state = self.state.borrow_mut();
        state.view.percent = status.percent();
        state.view.current_bytes = status.current_bytes();
        state.view.total_bytes = status.total_bytes();
        state.view.speed_bps = status.current_cps();
        state.draw();
    }

    fn start(&mut self) {
        let mut state = self.state.borrow_mut();
        state.view.phase = ProgressPhase::Downloading;
        state.draw();
    }

    fn stop(&mut self, _status: &AcqTextStatus) {
        // Phase transition handled externally
    }
}

// ============================================================================
// DynInstallProgress implementation
// ============================================================================

pub struct TuiInstallProgress {
    state: Rc<RefCell<ProgressState>>,
}

impl TuiInstallProgress {
    pub fn new(state: Rc<RefCell<ProgressState>>) -> Self {
        Self { state }
    }
}

impl rust_apt::progress::DynInstallProgress for TuiInstallProgress {
    fn status_changed(
        &mut self,
        pkgname: String,
        steps_done: u64,
        total_steps: u64,
        action: String,
    ) {
        let mut state = self.state.borrow_mut();
        state.view.phase = ProgressPhase::Installing;
        state.view.install_steps_done = steps_done;
        state.view.install_total_steps = total_steps;
        state.view.install_action = if pkgname.is_empty() {
            action
        } else {
            format!("{action} {pkgname}")
        };
        state.draw();
    }

    fn error(&mut self, pkgname: String, _steps_done: u64, _total_steps: u64, error: String) {
        let mut state = self.state.borrow_mut();
        state.view.errors.push(format!("{pkgname}: {error}"));
        state.draw();
    }
}

// ============================================================================
// Rendering - compact centered modal
// ============================================================================

/// Rect of at most `width` x `height`, centered in `area` and clipped to it.
pub fn centered_rect(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

fn render_progress_modal(frame: &mut Frame, view: &ProgressView) {
    let area = frame.area();

    // border(1) + pad(1) + status(1) + pad(1) + gauge(1) + pad(1) + detail(1) + pad(1) + border(1)
    let base_height: u16 = 9;
    let error_height = if view.errors.is_empty() {
        0
    } else {
        // 1 for separator + up to 4 error lines
        1 + (view.errors.len() as u16).min(4)
    };
    let modal_area = centered_rect(
        area,
        70.min(area.width.saturating_sub(4)),
        (base_height + error_height).min(area.height.saturating_sub(2)),
    );

    frame.render_widget(Clear, modal_area);

    let accent = match view.phase {
        ProgressPhase::Downloading => Color::Cyan,
        ProgressPhase::Installing => Color::Green,
    };
    let block = Block::default()
        .title(format!(" {} ", view.title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent));
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Inner layout: pad, status, pad, gauge, pad, detail, pad, [errors]
    let mut constraints = vec![Constraint::Length(1); 7];
    if error_height > 0 {
        constraints.push(Constraint::Min(error_height));
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    let (status, ratio, detail) = match view.phase {
        ProgressPhase::Downloading => {
            let speed = if view.speed_bps > 0 {
                format!("  {}/s", size_str(view.speed_bps))
            } else {
                String::new()
            };
            let status = Line::from(vec![
                Span::styled("Downloading... ", Style::default().fg(accent)),
                Span::styled(
                    format!("{:.0}%", view.percent),
                    Style::default().fg(Color::White).bold(),
                ),
                Span::styled(speed, Style::default().fg(Color::DarkGray)),
            ]);
            let detail = format!(
                "{} / {}",
                size_str(view.current_bytes),
                size_str(view.total_bytes)
            );
            (status, view.percent / 100.0, detail)
        }
        ProgressPhase::Installing => {
            let status = Line::from(vec![
                Span::styled("Installing... ", Style::default().fg(accent)),
                Span::styled(
                    format!(
                        "Step {} / {}",
                        view.install_steps_done, view.install_total_steps
                    ),
                    Style::default().fg(Color::White).bold(),
                ),
            ]);
            let ratio = if view.install_total_steps > 0 {
                view.install_steps_done as f64 / view.install_total_steps as f64
            } else {
                0.0
            };
            (status, ratio, view.install_action.clone())
        }
    };

    frame.render_widget(Paragraph::new(status), chunks[1]);
    let gauge = Gauge::default()
        .gauge_style(Style::default().fg(accent).bg(Color::DarkGray))
        .ratio(ratio.clamp(0.0, 1.0));
    frame.render_widget(gauge, chunks[3]);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            detail,
            Style::default().fg(Color::DarkGray),
        ))),
        chunks[5],
    );

    if error_height > 0 && chunks.len() > 7 {
        let error_lines: Vec<Line> = view
            .errors
            .iter()
            .rev()
            .take(4)
            .rev()
            .map(|e| Line::from(Span::styled(e.as_str(), Style::default().fg(Color::Red))))
            .collect();
        let error_para = Paragraph::new(error_lines).wrap(Wrap { trim: false });
        frame.render_widget(error_para, chunks[7]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_rect_never_exceeds_area() {
        let area = Rect::new(2, 3, 10, 4);
        let r = centered_rect(area, 50, 7);
        assert_eq!(r, area);
        let r = centered_rect(Rect::new(0, 0, 20, 10), 10, 4);
        assert_eq!(r, Rect::new(5, 3, 10, 4));
    }
}
