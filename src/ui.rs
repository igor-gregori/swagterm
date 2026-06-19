use crate::app::{App, Endpoint, Panel};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

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

pub fn draw(f: &mut Frame, app: &App) {
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

fn draw_sidebar(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::Sidebar {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Endpoints ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let mut lines: Vec<Line> = Vec::new();
    let mut current_tag = String::new();

    for (i, &idx) in app.filtered.iter().enumerate() {
        let ep = &app.endpoints[idx];
        if ep.tag != current_tag {
            current_tag = ep.tag.clone();
            lines.push(Line::from(Span::styled(
                format!("▼ {current_tag}"),
                Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
            )));
        }

        let style = if i == app.selected {
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

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}

fn draw_detail(f: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.active_panel == Panel::Detail {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let block = Block::default()
        .title(" Detail ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(ep) = app.selected_endpoint() else {
        f.render_widget(Paragraph::new("No endpoint selected").block(block), area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();
    add_operation_header(&mut lines, ep);
    add_parameters_section(&mut lines, ep);
    add_responses_section(&mut lines, ep);

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
}

fn add_parameters_section(lines: &mut Vec<Line<'static>>, ep: &Endpoint) {
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
    }
    lines.push(Line::from(""));
}

fn add_responses_section(lines: &mut Vec<Line<'static>>, ep: &Endpoint) {
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
    }
    lines.push(Line::from(""));
}
