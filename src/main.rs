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

    /// Bearer token for Authorization header
    #[arg(long)]
    bearer: Option<String>,

    /// API key (format: HEADER_NAME=VALUE, e.g. X-API-Key=abc123)
    #[arg(long)]
    api_key: Option<String>,

    /// Basic auth (format: username:password)
    #[arg(long)]
    basic: Option<String>,

    /// Custom header (repeatable, format: Name=Value)
    #[arg(short = 'H', long = "header")]
    headers: Vec<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let spec = swagger::parse_source(&cli.source)?;
    let mut app = App::new(spec);

    // Apply auth from CLI flags
    if let Some(token) = cli.bearer {
        app.auth = app::AuthConfig::Bearer(token);
    } else if let Some(api_key) = cli.api_key {
        if let Some((header, value)) = api_key.split_once('=') {
            app.auth = app::AuthConfig::ApiKey { header: header.into(), value: value.into() };
        }
    } else if let Some(basic) = cli.basic {
        if let Some((user, pass)) = basic.split_once(':') {
            app.auth = app::AuthConfig::Basic { username: user.into(), password: pass.into() };
        }
    } else if !cli.headers.is_empty() {
        let headers: Vec<(String, String)> = cli.headers.iter()
            .filter_map(|h| h.split_once('=').map(|(k, v)| (k.into(), v.into())))
            .collect();
        if !headers.is_empty() {
            app.auth = app::AuthConfig::Custom(headers);
        }
    }

    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    loop {
        app.poll_response();
        terminal.draw(|f| ui::draw(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match app.mode {
                    AppMode::TryIt => handle_try_it_keys(&mut app, key.code),
                    AppMode::AuthEdit => handle_auth_keys(&mut app, key.code),
                    AppMode::Browse => {
                        if app.searching {
                            handle_search_keys(&mut app, key.code);
                        } else {
                            handle_browse_keys(&mut app, key.code);
                        }
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
        KeyCode::Char('a') => { app.mode = AppMode::AuthEdit; app.auth_selected = 0; app.auth_editing = false; }
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
        KeyCode::Char('c') => {
            let editing = app.try_it.as_ref().map(|s| s.editing).unwrap_or(false);
            if !editing {
                app.copy_as_curl();
                return;
            }
        }
        KeyCode::Char('j') | KeyCode::Down => {
            let editing = app.try_it.as_ref().map(|s| s.editing).unwrap_or(false);
            if !editing {
                let has_response = app.try_it.as_ref().map(|s| s.response.is_some()).unwrap_or(false);
                if has_response {
                    app.scroll = app.scroll.saturating_add(1);
                } else {
                    let has_body = app.selected_endpoint()
                        .map(|ep| ep.operation.parameters.iter().any(|p| p.location == "body"))
                        .unwrap_or(false);
                    if let Some(state) = app.try_it.as_mut() {
                        let total_fields = state.param_values.len() + if has_body { 1 } else { 0 };
                        if total_fields > 0 {
                            state.selected_field = (state.selected_field + 1) % total_fields;
                        }
                    }
                }
                return;
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            let editing = app.try_it.as_ref().map(|s| s.editing).unwrap_or(false);
            if !editing {
                let has_response = app.try_it.as_ref().map(|s| s.response.is_some()).unwrap_or(false);
                if has_response {
                    app.scroll = app.scroll.saturating_sub(1);
                } else {
                    let has_body = app.selected_endpoint()
                        .map(|ep| ep.operation.parameters.iter().any(|p| p.location == "body"))
                        .unwrap_or(false);
                    if let Some(state) = app.try_it.as_mut() {
                        let total_fields = state.param_values.len() + if has_body { 1 } else { 0 };
                        if total_fields > 0 {
                            state.selected_field = state.selected_field.checked_sub(1).unwrap_or(total_fields - 1);
                        }
                    }
                }
                return;
            }
        }
        _ => {}
    }

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

    match code {
        KeyCode::Enter => state.editing = true,
        _ => {}
    }
}

fn handle_auth_keys(app: &mut App, code: KeyCode) {
    // Auth menu options: None, Bearer, ApiKey, Basic
    const OPTIONS: &[&str] = &["None", "Bearer Token", "API Key", "Basic Auth"];

    if app.auth_editing {
        match code {
            KeyCode::Esc => app.auth_editing = false,
            KeyCode::Enter => {
                apply_auth_input(app);
                app.auth_editing = false;
                app.mode = AppMode::Browse;
                app.status_message = Some(("Auth configured!".into(), std::time::Instant::now()));
            }
            KeyCode::Backspace => { app.auth_input.pop(); }
            KeyCode::Char(c) => app.auth_input.push(c),
            _ => {}
        }
        return;
    }

    match code {
        KeyCode::Esc => app.mode = AppMode::Browse,
        KeyCode::Char('j') | KeyCode::Down => {
            app.auth_selected = (app.auth_selected + 1) % OPTIONS.len();
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.auth_selected = app.auth_selected.checked_sub(1).unwrap_or(OPTIONS.len() - 1);
        }
        KeyCode::Enter => {
            if app.auth_selected == 0 {
                app.auth = app::AuthConfig::None;
                app.mode = AppMode::Browse;
                app.status_message = Some(("Auth cleared".into(), std::time::Instant::now()));
            } else {
                app.auth_input.clear();
                app.auth_editing = true;
            }
        }
        _ => {}
    }
}

fn apply_auth_input(app: &mut App) {
    match app.auth_selected {
        1 => app.auth = app::AuthConfig::Bearer(app.auth_input.clone()),
        2 => {
            if let Some((header, value)) = app.auth_input.split_once('=') {
                app.auth = app::AuthConfig::ApiKey { header: header.into(), value: value.into() };
            }
        }
        3 => {
            if let Some((user, pass)) = app.auth_input.split_once(':') {
                app.auth = app::AuthConfig::Basic { username: user.into(), password: pass.into() };
            }
        }
        _ => {}
    }
}
