mod app;
mod swagger;
mod ui;

use app::{App, AppMode, Panel};
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
            match app.mode {
                AppMode::TryIt => handle_try_it_keys(&mut app, key.code),
                AppMode::Browse => {
                    if app.searching {
                        handle_search_keys(&mut app, key.code);
                    } else {
                        handle_browse_keys(&mut app, key.code);
                    }
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

fn handle_search_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => {
            app.searching = false;
            app.search.clear();
            app.apply_filter();
        }
        KeyCode::Enter => app.searching = false,
        KeyCode::Backspace => { app.search.pop(); app.apply_filter(); }
        KeyCode::Char(c) => { app.search.push(c); app.apply_filter(); }
        _ => {}
    }
}

fn handle_browse_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Char('q') => app.quit = true,
        KeyCode::Char('/') => app.searching = true,
        KeyCode::Char('t') => app.enter_try_it(),
        KeyCode::Char('j') | KeyCode::Down => {
            if app.active_panel == Panel::Sidebar { app.next(); } else { app.scroll_down(); }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if app.active_panel == Panel::Sidebar { app.prev(); } else { app.scroll_up(); }
        }
        KeyCode::Enter | KeyCode::Char(' ') => {
            if app.active_panel == Panel::Sidebar { app.toggle_tag(); }
        }
        KeyCode::Tab => {
            app.active_panel = match app.active_panel {
                Panel::Sidebar => Panel::Detail,
                Panel::Detail => Panel::Sidebar,
            };
        }
        KeyCode::Esc => {
            if !app.search.is_empty() { app.search.clear(); app.apply_filter(); }
        }
        KeyCode::PageDown => app.page_down(),
        KeyCode::PageUp => app.page_up(),
        _ => {}
    }
}

fn handle_try_it_keys(app: &mut App, code: KeyCode) {
    // Handle keys that need full app ownership first
    match code {
        KeyCode::Esc => {
            let editing = app.try_it.as_ref().map(|s| s.editing).unwrap_or(false);
            if editing {
                if let Some(state) = app.try_it.as_mut() { state.editing = false; }
            } else {
                app.exit_try_it();
            }
            return;
        }
        KeyCode::Char('s') => {
            let editing = app.try_it.as_ref().map(|s| s.editing).unwrap_or(false);
            if !editing {
                app.execute_request();
                return;
            }
        }
        _ => {}
    }

    let has_body = app.selected_endpoint()
        .map(|ep| ep.operation.parameters.iter().any(|p| p.location == "body"))
        .unwrap_or(false);

    let Some(state) = app.try_it.as_mut() else { return };

    if state.editing {
        match code {
            KeyCode::Enter => state.editing = false,
            KeyCode::Backspace => {
                let total_params = state.param_values.len();
                if state.selected_field < total_params {
                    state.param_values[state.selected_field].2.pop();
                } else {
                    state.body.pop();
                }
            }
            KeyCode::Char(c) => {
                let total_params = state.param_values.len();
                if state.selected_field < total_params {
                    state.param_values[state.selected_field].2.push(c);
                } else {
                    state.body.push(c);
                }
            }
            _ => {}
        }
        return;
    }

    let total_fields = state.param_values.len() + if has_body { 1 } else { 0 };

    match code {
        KeyCode::Enter => state.editing = true,
        KeyCode::Char('j') | KeyCode::Down => {
            if total_fields > 0 {
                state.selected_field = (state.selected_field + 1) % total_fields;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            if total_fields > 0 {
                state.selected_field = state.selected_field.checked_sub(1).unwrap_or(total_fields - 1);
            }
        }
        _ => {}
    }
}
