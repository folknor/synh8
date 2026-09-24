mod app;
mod ui;

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self, Event, KeyEventKind};
use ratatui::DefaultTerminal;

use app::{App, Outcome};
use synh8::core::is_root;
use ui::ui;

fn main() -> Result<()> {
    color_eyre::install()?;

    if !is_root() {
        eprintln!(
            "synh8 must be run as root. Try: sudo {}",
            std::env::args().next().unwrap_or_else(|| "synh8".into())
        );
        std::process::exit(1);
    }

    // Suppress debconf prompts during commits (dpkg cannot ask: its output
    // is captured). Set before anything else runs, so no other thread can
    // be reading the environment concurrently.
    // SAFETY: the process is still single-threaded at this point.
    unsafe {
        std::env::set_var("DEBIAN_FRONTEND", "noninteractive");
    }

    // Open the cache before taking over the terminal, so a failure here is
    // printed normally.
    let mut app = App::new()?;

    let _hotpath = hotpath::HotpathGuardBuilder::new("synh8")
        .percentiles(&[50.0, 95.0, 99.0])
        .functions_limit(0)
        .build();

    // ratatui::init also installs a panic hook that restores the terminal
    // before the panic message is printed.
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // Poll without waiting during warm-up, so warm-up steps run between
        // frames while staying responsive to keys.
        let poll = if app.warm_step.is_some() { 0 } else { 100 };
        if !event::poll(Duration::from_millis(poll))? {
            app.warm_next();
            continue;
        }
        let Event::Key(key) = event::read()? else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        match app.handle_key(&key) {
            Outcome::Continue => {}
            Outcome::Quit => return Ok(()),
            Outcome::Commit => {
                terminal.clear()?;
                app.commit_changes_live();
                terminal.clear()?;
            }
            Outcome::Update => {
                app.update_packages_live();
                terminal.clear()?;
            }
        }
    }
}
