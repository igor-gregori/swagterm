use crate::app::{App, Endpoint, Panel, SidebarItem};
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

    let title = format!(
        " SwagTerm — {} v{}",
        app.spec.info.title, app.spec.info.version
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
    draw_detail(f, app, main[1]);

    let footer = if app.searching {
        format!(" Search: {}█", app.search)
    } else {
        " j/k:nav │ Tab:switch │ /:search │ Esc:clear │ q:quit │ ↑↓:scroll detail".into()
    };
    f.render_widget(
        Paragraph::new(footer).style(Style::default().fg(Color::DarkGray)),
        chunks[2],
    );
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
            render_schema(lines, schema, definitions, 2, 4);
        }
    }
    lines.push(Line::from(""));
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
