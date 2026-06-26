use crate::app::{App, AppMode, Endpoint, Panel, SidebarItem};
use crate::swagger::Schema;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use std::collections::HashMap;

fn method_color(method: &str) -> Color {
    match method {
        "GET" => Color::Green,
        "POST" => Color::Blue,
        "PUT" => Color::Yellow,
        "DELETE" => Color::Red,
        "PATCH" => Color::Magenta,
        _ => Color::White,
    }
}

pub fn draw(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let warn_badge = if app.warnings.is_empty() {
        String::new()
    } else {
        format!(" ⚠ {}", app.warnings.len())
    };
    let title = format!(
        " SwagTerm — {} v{}{}",
        app.spec.info.title, app.spec.info.version, warn_badge
    );
    f.render_widget(
        Paragraph::new(title).style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        chunks[0],
    );

    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    draw_sidebar(f, app, main[0]);
    match app.mode {
        AppMode::Browse => {
            if app.show_warnings {
                draw_warnings(f, app, main[1]);
            } else {
                draw_detail(f, app, main[1]);
            }
        }
        AppMode::TryIt => draw_try_it(f, app, main[1]),
        AppMode::AuthEdit => draw_auth(f, app, main[1]),
    }

    // Clear expired status messages (after 2 seconds)
    if let Some((_, time)) = &app.status_message {
        if time.elapsed() > std::time::Duration::from_secs(2) {
            app.status_message = None;
        }
    }

    let footer = if let Some((msg, _)) = &app.status_message {
        msg.clone()
    } else if app.searching {
        format!(" Search: {}█", app.search)
    } else if app.mode == AppMode::TryIt {
        let editing = app.try_it.as_ref().map(|s| s.editing).unwrap_or(false);
        if editing {
            " Type value │ Enter/Esc:done".into()
        } else {
            " j/k:select │ Enter:edit │ s:send │ c:curl │ Esc:back".into()
        }
    } else if app.mode == AppMode::AuthEdit {
        if app.auth_editing {
            " Type value │ Enter:apply │ Esc:cancel".into()
        } else {
            " j/k:select │ Enter:choose │ Esc:back".into()
        }
    } else {
        " j/k:nav │ Tab:switch │ /:search │ t:try it │ a:auth │ w:warnings │ q:quit".into()
    };

    let footer_style = if app.status_message.is_some() {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    f.render_widget(Paragraph::new(footer).style(footer_style), chunks[2]);
}

fn draw_sidebar(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.active_panel == Panel::Sidebar {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Endpoints ")
        .borders(Borders::ALL)
        .border_style(border_style);

    app.sidebar_height = area.height.saturating_sub(2);

    let mut lines: Vec<Line> = Vec::new();

    for (i, item) in app.sidebar_items.iter().enumerate() {
        let is_selected = i == app.selected;
        match item {
            SidebarItem::Tag(tag) => {
                let arrow = if app.collapsed_tags.contains(tag) { "▶" } else { "▼" };
                let style = if is_selected {
                    Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                };
                lines.push(Line::from(Span::styled(format!("{arrow} {tag}"), style)));
            }
            SidebarItem::Endpoint(idx) => {
                let ep = &app.endpoints[*idx];
                let style = if is_selected {
                    Style::default().bg(Color::DarkGray).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let method_span = Span::styled(
                    format!("{:>6}", ep.method),
                    Style::default().fg(method_color(&ep.method)),
                );
                let path_span = Span::styled(format!(" {}", ep.path), style);
                lines.push(Line::from(vec![method_span, path_span]));
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.sidebar_scroll, 0));
    f.render_widget(paragraph, area);
}

fn draw_detail(f: &mut Frame, app: &mut App, area: Rect) {
    let border_style = if app.active_panel == Panel::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    app.detail_height = area.height.saturating_sub(2);

    let Some(ep) = app.selected_endpoint() else {
        let block = Block::default()
            .title(" Detail ")
            .borders(Borders::ALL)
            .border_style(border_style);
        f.render_widget(Paragraph::new("No endpoint selected").block(block), area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    add_operation_header(&mut lines, ep);
    add_parameters_section(&mut lines, ep, &app.spec.definitions);
    add_responses_section(&mut lines, ep, &app.spec.definitions);

    let total_lines = lines.len() as u16;
    let scroll_info = if total_lines > app.detail_height {
        format!(" Detail [{}/{}] ", app.scroll + 1, total_lines.saturating_sub(app.detail_height) + 1)
    } else {
        " Detail ".into()
    };

    let block = Block::default()
        .title(scroll_info)
        .borders(Borders::ALL)
        .border_style(border_style);

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

fn add_operation_header(lines: &mut Vec<Line<'static>>, ep: &Endpoint) {
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", ep.method),
            Style::default()
                .fg(Color::White)
                .bg(method_color(&ep.method))
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {}", ep.path),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]));

    if let Some(op_id) = &ep.operation.operation_id {
        lines.push(Line::from(Span::styled(
            format!("operationId: {op_id}"),
            Style::default().fg(Color::DarkGray),
        )));
    }
    lines.push(Line::from(""));

    if let Some(summary) = &ep.operation.summary {
        lines.push(Line::from(Span::styled(
            summary.clone(),
            Style::default().fg(Color::White),
        )));
    }
    if let Some(desc) = &ep.operation.description {
        lines.push(Line::from(Span::styled(
            desc.clone(),
            Style::default().fg(Color::Gray),
        )));
    }
    lines.push(Line::from(""));

    if !ep.operation.consumes.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Consumes: ", Style::default().fg(Color::Yellow)),
            Span::raw(ep.operation.consumes.join(", ")),
        ]));
    }
    if !ep.operation.produces.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Produces: ", Style::default().fg(Color::Yellow)),
            Span::raw(ep.operation.produces.join(", ")),
        ]));
    }
    if !ep.operation.consumes.is_empty() || !ep.operation.produces.is_empty() {
        lines.push(Line::from(""));
    }
}

fn add_parameters_section(lines: &mut Vec<Line<'static>>, ep: &Endpoint, definitions: &HashMap<String, Schema>) {
    if ep.operation.parameters.is_empty() {
        return;
    }

    lines.push(Line::from(Span::styled(
        "Parameters",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "─────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));
    lines.push(Line::from(vec![
        Span::styled(format!("{:<20}", "Name"), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:<10}", "In"), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(format!("{:<12}", "Type"), Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(String::from("Required"), Style::default().add_modifier(Modifier::BOLD)),
    ]));

    for p in &ep.operation.parameters {
        let type_str = p.param_type.as_deref().unwrap_or("object");
        let req = if p.required { "✓" } else { "" };
        lines.push(Line::from(vec![
            Span::raw(format!("{:<20}", p.name)),
            Span::styled(format!("{:<10}", p.location), Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:<12}", type_str)),
            Span::styled(String::from(req), Style::default().fg(Color::Green)),
        ]));
        if let Some(desc) = &p.description {
            lines.push(Line::from(Span::styled(
                format!("  └ {desc}"),
                Style::default().fg(Color::DarkGray),
            )));
        }
        if !p.enum_values.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  Available values: {}", p.enum_values.join(", ")),
                Style::default().fg(Color::Magenta),
            )));
        }
        // Show schema for body parameters
        if p.location == "body" {
            if let Some(schema) = &p.schema {
                render_schema(lines, schema, definitions, 2, 4);
            }
        }
    }
    lines.push(Line::from(""));
}

fn add_responses_section(lines: &mut Vec<Line<'static>>, ep: &Endpoint, definitions: &HashMap<String, Schema>) {
    if ep.operation.responses.is_empty() {
        return;
    }

    lines.push(Line::from(Span::styled(
        "Responses",
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(Span::styled(
        "─────────────────────────────────────────────────",
        Style::default().fg(Color::DarkGray),
    )));

    for (code, resp) in &ep.operation.responses {
        let color = match code.chars().next() {
            Some('2') => Color::Green,
            Some('3') => Color::Yellow,
            Some('4') => Color::Red,
            Some('5') => Color::Red,
            _ => Color::White,
        };
        let desc = resp.description.clone().unwrap_or_default();
        lines.push(Line::from(vec![
            Span::styled(format!("  {code} "), Style::default().fg(color).add_modifier(Modifier::BOLD)),
            Span::raw(desc),
        ]));
        if let Some(schema) = &resp.schema {
            lines.push(Line::from(Span::styled(
                "    Example:",
                Style::default().fg(Color::DarkGray),
            )));
            let example = generate_example(schema, definitions, 0, 6);
            for line in example.lines() {
                lines.push(Line::from(Span::styled(
                    format!("      {line}"),
                    Style::default().fg(Color::Green),
                )));
            }
        }
    }
    lines.push(Line::from(""));
}

fn draw_warnings(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(format!(" Warnings ({}) ", app.warnings.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let mut lines: Vec<Line> = Vec::new();

    if app.warnings.is_empty() {
        lines.push(Line::from(Span::styled(
            "  ✓ No issues found",
            Style::default().fg(Color::Green),
        )));
    } else {
        for w in &app.warnings {
            lines.push(Line::from(vec![
                Span::styled("⚠ ", Style::default().fg(Color::Yellow)),
                Span::styled(w.path.clone(), Style::default().fg(Color::Cyan)),
            ]));
            lines.push(Line::from(Span::styled(
                format!("  {}", w.message),
                Style::default().fg(Color::White),
            )));
            lines.push(Line::from(""));
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

fn draw_auth(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .title(" Auth Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let options = ["None", "Bearer Token", "API Key", "Basic Auth"];
    let hints = ["", "Enter token value", "Format: HEADER_NAME=VALUE", "Format: username:password"];

    let mut lines: Vec<Line> = Vec::new();

    // Current auth display
    let current = match &app.auth {
        crate::app::AuthConfig::None => "None".into(),
        crate::app::AuthConfig::Bearer(_) => "Bearer ****".into(),
        crate::app::AuthConfig::ApiKey { header, .. } => format!("API Key ({header})"),
        crate::app::AuthConfig::Basic { username, .. } => format!("Basic ({username}:****)"),
        crate::app::AuthConfig::Custom(h) => format!("Custom ({} headers)", h.len()),
    };
    lines.push(Line::from(vec![
        Span::styled("Current: ", Style::default().fg(Color::Cyan)),
        Span::styled(current, Style::default().fg(Color::White)),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Select auth type:",
        Style::default().add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(""));

    for (i, opt) in options.iter().enumerate() {
        let is_selected = i == app.auth_selected;
        let arrow = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(format!("{arrow}{opt}"), style)));
    }

    if app.auth_editing {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            hints[app.auth_selected],
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::styled("> ", Style::default().fg(Color::Yellow)),
            Span::styled(format!("{}█", app.auth_input), Style::default().fg(Color::White)),
        ]));
    }

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_try_it(f: &mut Frame, app: &mut App, area: Rect) {
    let block = Block::default()
        .title(" Try it out ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    app.detail_height = area.height.saturating_sub(2);

    let Some(ep) = app.selected_endpoint() else {
        f.render_widget(Paragraph::new("No endpoint selected").block(block), area);
        return;
    };
    let Some(state) = &app.try_it else {
        f.render_widget(Paragraph::new("").block(block), area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {} ", ep.method),
            Style::default().fg(Color::White).bg(method_color(&ep.method)).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {}", ep.path), Style::default().add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(Span::styled(
        format!("→ {}{}", app.spec.base_url, ep.path),
        Style::default().fg(Color::DarkGray),
    )));
    match &app.auth {
        crate::app::AuthConfig::None => {}
        crate::app::AuthConfig::Bearer(_) => {
            lines.push(Line::from(Span::styled("  🔒 Bearer", Style::default().fg(Color::Magenta))));
        }
        crate::app::AuthConfig::ApiKey { header, .. } => {
            lines.push(Line::from(Span::styled(format!("  🔒 API Key ({header})"), Style::default().fg(Color::Magenta))));
        }
        crate::app::AuthConfig::Basic { username, .. } => {
            lines.push(Line::from(Span::styled(format!("  🔒 Basic ({username})"), Style::default().fg(Color::Magenta))));
        }
        crate::app::AuthConfig::Custom(h) => {
            lines.push(Line::from(Span::styled(format!("  🔒 Custom ({} headers)", h.len()), Style::default().fg(Color::Magenta))));
        }
    }
    lines.push(Line::from(""));

    // Parameters
    if !state.param_values.is_empty() {
        lines.push(Line::from(Span::styled(
            "Parameters",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));

        for (i, (name, location, value)) in state.param_values.iter().enumerate() {
            let is_selected = i == state.selected_field;
            let cursor = if is_selected && state.editing { "█" } else { "" };
            let arrow = if is_selected { "▸ " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{arrow}{name}"), style),
                Span::styled(format!(" ({location}): "), Style::default().fg(Color::DarkGray)),
                Span::styled(format!("{value}{cursor}"), Style::default().fg(Color::White)),
            ]));
            // Show available values if parameter has enum
            if let Some(param) = ep.operation.parameters.iter().find(|p| p.name == *name) {
                if !param.enum_values.is_empty() {
                    lines.push(Line::from(Span::styled(
                        format!("    Available values: {}", param.enum_values.join(", ")),
                        Style::default().fg(Color::Magenta),
                    )));
                }
            }
        }
        lines.push(Line::from(""));
    }

    // Body field
    let has_body = ep.operation.parameters.iter().any(|p| p.location == "body");
    if has_body {
        let body_idx = state.param_values.len();
        let is_selected = state.selected_field == body_idx;
        let cursor = if is_selected && state.editing { "█" } else { "" };
        let arrow = if is_selected { "▸ " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };

        lines.push(Line::from(Span::styled(
            "Request Body",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::styled(format!("{arrow}"), style),
            Span::styled(format!("{}{cursor}", state.body), Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(""));
    }

    // Loading
    if state.loading {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let idx = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_millis() / 100) as usize % frames.len();
        lines.push(Line::from(Span::styled(
            format!("{} Sending request...", frames[idx]),
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )));
    }

    // Response
    if let Some(resp) = &state.response {
        let status_color = match resp.status {
            200..=299 => Color::Green,
            300..=399 => Color::Yellow,
            _ => Color::Red,
        };
        lines.push(Line::from(Span::styled(
            format!("Response — {}", resp.status),
            Style::default().fg(status_color).add_modifier(Modifier::BOLD),
        )));
        lines.push(Line::from(Span::styled(
            "─────────────────────────────────────────",
            Style::default().fg(Color::DarkGray),
        )));

        // Headers
        for (k, v) in &resp.headers {
            lines.push(Line::from(vec![
                Span::styled(format!("{k}: "), Style::default().fg(Color::Yellow)),
                Span::raw(v.clone()),
            ]));
        }
        lines.push(Line::from(""));

        // Body
        lines.push(Line::from(Span::styled("Body", Style::default().fg(Color::Cyan))));
        for line in resp.body.lines() {
            lines.push(Line::from(Span::raw(line.to_string())));
        }
    }

    if let Some(err) = &state.error {
        lines.push(Line::from(Span::styled(
            format!("Error: {err}"),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        )));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((app.scroll, 0));
    f.render_widget(paragraph, area);
}

fn render_schema(
    lines: &mut Vec<Line<'static>>,
    schema: &Schema,
    definitions: &HashMap<String, Schema>,
    indent: usize,
    max_depth: usize,
) {
    if indent > max_depth {
        let pad = " ".repeat(indent * 2);
        lines.push(Line::from(Span::styled(
            format!("{pad}..."),
            Style::default().fg(Color::DarkGray),
        )));
        return;
    }

    // Handle $ref
    if let Some(ref_path) = &schema.reference {
        let ref_name = ref_path
            .strip_prefix("#/definitions/")
            .or_else(|| ref_path.strip_prefix("#/components/schemas/"))
            .unwrap_or(ref_path);
        if let Some(resolved) = definitions.get(ref_name) {
            let pad = " ".repeat(indent * 2);
            lines.push(Line::from(Span::styled(
                format!("{pad}{ref_name} {{"),
                Style::default().fg(Color::Magenta),
            )));
            render_schema(lines, resolved, definitions, indent + 1, max_depth);
            lines.push(Line::from(Span::styled(
                format!("{pad}}}"),
                Style::default().fg(Color::Magenta),
            )));
        } else {
            let pad = " ".repeat(indent * 2);
            lines.push(Line::from(Span::styled(
                format!("{pad}$ref: {ref_path}"),
                Style::default().fg(Color::Yellow),
            )));
        }
        return;
    }

    // Handle array
    if schema.schema_type.as_deref() == Some("array") {
        if let Some(items) = &schema.items {
            let pad = " ".repeat(indent * 2);
            lines.push(Line::from(Span::styled(
                format!("{pad}array ["),
                Style::default().fg(Color::Cyan),
            )));
            render_schema(lines, items, definitions, indent + 1, max_depth);
            lines.push(Line::from(Span::styled(
                format!("{pad}]"),
                Style::default().fg(Color::Cyan),
            )));
        }
        return;
    }

    // Handle object with properties
    if !schema.properties.is_empty() {
        let pad = " ".repeat(indent * 2);
        for (name, prop) in &schema.properties {
            let type_str = prop_type_str(prop);
            let req = if schema.required.contains(name) { "*" } else { "" };
            lines.push(Line::from(vec![
                Span::styled(format!("{pad}{name}"), Style::default().fg(Color::White)),
                Span::styled(format!("{req}"), Style::default().fg(Color::Red)),
                Span::styled(format!(": {type_str}"), Style::default().fg(Color::Gray)),
            ]));
            // Recurse into nested objects/refs
            if prop.reference.is_some()
                || !prop.properties.is_empty()
                || (prop.schema_type.as_deref() == Some("array") && prop.items.is_some())
            {
                render_schema(lines, prop, definitions, indent + 1, max_depth);
            }
        }
        return;
    }

    // Simple type
    let pad = " ".repeat(indent * 2);
    let t = prop_type_str(schema);
    if !t.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("{pad}{t}"),
            Style::default().fg(Color::Gray),
        )));
    }
}

fn prop_type_str(schema: &Schema) -> String {
    if let Some(ref_path) = &schema.reference {
        return ref_path
            .strip_prefix("#/definitions/")
            .or_else(|| ref_path.strip_prefix("#/components/schemas/"))
            .unwrap_or(ref_path)
            .to_string();
    }
    let base = schema.schema_type.as_deref().unwrap_or("object");
    if base == "array" {
        if let Some(items) = &schema.items {
            let inner = prop_type_str(items);
            return format!("[{inner}]");
        }
        return "array".into();
    }
    if let Some(fmt) = &schema.format {
        return format!("{base}({fmt})");
    }
    base.to_string()
}

fn generate_example(
    schema: &Schema,
    definitions: &HashMap<String, Schema>,
    depth: usize,
    max_depth: usize,
) -> String {
    let value = build_example_value(schema, definitions, depth, max_depth);
    serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".into())
}

fn build_example_value(
    schema: &Schema,
    definitions: &HashMap<String, Schema>,
    depth: usize,
    max_depth: usize,
) -> serde_json::Value {
    if depth > max_depth {
        return serde_json::Value::Object(Default::default());
    }

    // Use explicit example if present
    if let Some(example) = &schema.example {
        return example.clone();
    }

    // Handle $ref
    if let Some(ref_path) = &schema.reference {
        let ref_name = ref_path
            .strip_prefix("#/definitions/")
            .or_else(|| ref_path.strip_prefix("#/components/schemas/"))
            .unwrap_or(ref_path);
        if let Some(resolved) = definitions.get(ref_name) {
            return build_example_value(resolved, definitions, depth, max_depth);
        }
        return serde_json::Value::String(format!("<{ref_name}>"));
    }

    // Handle array
    if schema.schema_type.as_deref() == Some("array") {
        if let Some(items) = &schema.items {
            let item = build_example_value(items, definitions, depth + 1, max_depth);
            return serde_json::Value::Array(vec![item]);
        }
        return serde_json::Value::Array(vec![]);
    }

    // Handle object with properties
    if !schema.properties.is_empty() {
        let mut map = serde_json::Map::new();
        for (name, prop) in &schema.properties {
            map.insert(name.clone(), build_example_value(prop, definitions, depth + 1, max_depth));
        }
        return serde_json::Value::Object(map);
    }

    // Simple types
    match schema.schema_type.as_deref() {
        Some("string") => {
            if !schema.enum_values.is_empty() {
                schema.enum_values[0].clone()
            } else if schema.format.as_deref() == Some("date-time") {
                serde_json::Value::String("2024-01-01T00:00:00Z".into())
            } else if schema.format.as_deref() == Some("date") {
                serde_json::Value::String("2024-01-01".into())
            } else {
                serde_json::Value::String("string".into())
            }
        }
        Some("integer") | Some("number") => serde_json::json!(0),
        Some("boolean") => serde_json::json!(true),
        _ => serde_json::Value::Object(Default::default()),
    }
}
