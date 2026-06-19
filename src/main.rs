mod app;
mod swagger;
mod ui;

use app::{App, Panel};
use clap::Parser;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use std::io::stdout;

#[derive(Parser)]
#[command(name = "swagterm", about = "Terminal Swagger/OpenAPI viewer")]
struct Cli {
    /// Path or URL to Swagger/OpenAPI spec (JSON/YAML)
    source: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let spec = swagger::parse_source(&cli.source)?;
    let mut app = App::new(spec);

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if app.searching {
                match key.code {
                    KeyCode::Esc => {
                        app.searching = false;
                        app.search.clear();
                        app.apply_filter();
                    }
                    KeyCode::Enter => {
                        app.searching = false;
                    }
                    KeyCode::Backspace => {
                        app.search.pop();
                        app.apply_filter();
                    }
                    KeyCode::Char(c) => {
                        app.search.push(c);
                        app.apply_filter();
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('q') => app.quit = true,
                    KeyCode::Char('/') => app.searching = true,
                    KeyCode::Char('j') | KeyCode::Down => {
                        if app.active_panel == Panel::Sidebar {
                            app.next();
                        } else {
                            app.scroll_down();
                        }
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        if app.active_panel == Panel::Sidebar {
                            app.prev();
                        } else {
                            app.scroll_up();
                        }
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        if app.active_panel == Panel::Sidebar {
                            app.toggle_tag();
                        }
                    }
                    KeyCode::Tab => {
                        app.active_panel = match app.active_panel {
                            Panel::Sidebar => Panel::Detail,
                            Panel::Detail => Panel::Sidebar,
                        };
                    }
                    KeyCode::Esc => {
                        if !app.search.is_empty() {
                            app.search.clear();
                            app.apply_filter();
                        }
                    }
                    KeyCode::PageDown => app.page_down(),
                    KeyCode::PageUp => app.page_up(),
                    _ => {}
                }
            }
        }

        if app.quit {
            break;
        }
    }

    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}
