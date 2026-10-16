use crate::api::{files::FileEntry, sys::SysEntry};
use crate::app::{App, Status};
use crate::complete::Completions;
use crate::input::TextInput;
use crate::progress::Progress;
use crate::util::human_size;
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::path::Path;

const INFO_LINES: u16 = 6;

const BORDER: Color = Color::Cyan;
const SELECTED_BG: Color = Color::Cyan;

pub fn draw(app: &mut App, frame: &mut Frame) {
    let [top, body, status] = Layout::vertical([
        Constraint::Length(INFO_LINES + 2),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    frame.render_widget(system_panel(app), top);

    let list = file_list(&app.files);
    frame.render_stateful_widget(list, body, &mut app.state);

    if let Some(c) = app.upload_input.as_ref().and_then(|p| p.completions.as_ref()) {
        completion_popup(frame, body, c);
    }

    frame.render_widget(Paragraph::new(status_line(app)), status);
}


fn system_panel(app: &App) -> Paragraph<'static> {
    let text: Vec<Line<'static>> = match &app.sys {
        Some(s) => sys_lines(s),
        None => vec![Line::from(Span::styled(
            "loading system info...",
            Style::new().fg(Color::Yellow).italic(),
        ))],
    };

    Paragraph::new(text).block(
        Block::bordered()
            .border_style(Style::new().fg(BORDER))
            .title(Span::styled(
                " System ",
                Style::new().fg(Color::Yellow).bold(),
            )),
    )
}

fn temp_color(t: f32) -> Color {
    if t < 60.0 {
        Color::Green
    } else if t < 75.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn usage_color(pct: f64) -> Color {
    if pct < 60.0 {
        Color::Green
    } else if pct < 85.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

fn sys_lines(s: &SysEntry) -> Vec<Line<'static>> {
    let label = |t: &str| Span::styled(format!("{t:<7}"), Style::new().fg(Color::DarkGray));

    let total = s.ram_total_mb.max(1);
    let used = s.used_mb();
    let pct = used as f64 / total as f64 * 100.0;
    let bar_w = 20usize;
    let filled = (((pct / 100.0) * bar_w as f64).round() as usize).min(bar_w);
    let ram_color = usage_color(pct);

    let temp = s.metrics.cpu.temp_c;

    vec![
        Line::from(vec![
            label("Model"),
            Span::styled(s.model.clone(), Style::new().fg(Color::LightMagenta).bold()),
        ]),
        Line::from(vec![
            label("CPU"),
            Span::styled(s.cpu.clone(), Style::new().fg(Color::LightCyan)),
        ]),
        Line::from(vec![
            label("OS"),
            Span::styled(s.os.clone(), Style::new().fg(Color::LightBlue)),
        ]),
        Line::from(vec![
            label("Temp"),
            Span::styled(
                format!("{temp:.1}°C"),
                Style::new().fg(temp_color(temp)).bold(),
            ),
        ]),
        Line::from(vec![
            label("RAM"),
            Span::styled("█".repeat(filled), Style::new().fg(ram_color)),
            Span::styled("░".repeat(bar_w - filled), Style::new().fg(Color::DarkGray)),
            Span::styled(
                format!(" {used}/{total} MB ({pct:.0}%)"),
                Style::new().fg(ram_color),
            ),
        ]),
        Line::from(vec![
            label("Uptime"),
            Span::styled(s.uptime.clone(), Style::new().fg(Color::LightGreen)),
        ]),
    ]
}


fn file_list(files: &[FileEntry]) -> List<'static> {
    let items: Vec<ListItem> = files.iter().map(file_item).collect();

    List::new(items)
        .block(
            Block::bordered()
                .border_style(Style::new().fg(BORDER))
                .title(Line::from(files_title())),
        )
        .highlight_style(Style::new().fg(Color::Black).bg(SELECTED_BG).bold())
        .highlight_symbol("> ")
}

fn files_title() -> Vec<Span<'static>> {
    let hint = |k: &'static str, t: &'static str| {
        [
            Span::styled(k, Style::new().fg(Color::Yellow)),
            Span::styled(format!(" {t}  "), Style::new().fg(Color::DarkGray)),
        ]
    };
    let mut spans = vec![
        Span::styled(" Files ", Style::new().fg(Color::Green).bold()),
        Span::styled("(", Style::new().fg(Color::DarkGray)),
    ];
    spans.extend(hint("↑/↓", "move"));
    spans.extend(hint("d", "download"));
    spans.extend(hint("u", "upload"));
    spans.extend(hint("r", "rename"));
    spans.extend(hint("x", "delete"));
    spans.extend(hint("q", "quit"));
    spans.push(Span::styled(") ", Style::new().fg(Color::DarkGray)));
    spans
}

fn ext_color(name: &str) -> Color {
    let ext = Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "pdf" => Color::LightRed,
        "doc" | "docx" => Color::LightBlue,
        "md" => Color::LightCyan,
        "txt" => Color::Gray,
        "xls" | "xlsx" | "csv" => Color::LightGreen,
        "zip" | "tar" | "gz" | "7z" => Color::Yellow,
        "png" | "jpg" | "jpeg" | "gif" | "svg" => Color::LightMagenta,
        _ => Color::White,
    }
}

fn file_item(f: &FileEntry) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(format!("{:<40}", f.name), Style::new().fg(ext_color(&f.name))),
        Span::styled(
            format!("{:>10}", human_size(f.size)),
            Style::new().fg(Color::DarkGray),
        ),
    ]))
}


fn status_line(app: &App) -> Line<'static> {
    if let Some((_, input)) = &app.rename_input {
        return prompt_line("Rename to:", input, "Enter save, Esc cancel");
    }
    if let Some(prompt) = &app.upload_input {
        return prompt_line(
            "Upload path:",
            &prompt.input,
            "Enter send, Tab complete, Esc cancel",
        );
    }
    match &app.status {
        Status::Idle => Line::default(),
        Status::Busy(m, progress) => busy_line(app, m, progress.as_deref()),
        Status::Done(m) => Line::from(Span::styled(
            format!(" ✓ {m}"),
            Style::new().fg(Color::Green).bold(),
        )),
        Status::Failed(m) => Line::from(Span::styled(
            format!(" ✗ {m}"),
            Style::new().fg(Color::Red).bold(),
        )),
        Status::Confirm(m) => Line::from(Span::styled(
            format!(" ? {m}"),
            Style::new().fg(Color::LightRed).bold(),
        )),
    }
}

fn prompt_line(label: &str, input: &TextInput, hint: &str) -> Line<'static> {
    let cursor = match input.at_cursor() {
        Some(c) => c.to_string(),
        None => " ".to_string(),
    };
    Line::from(vec![
        Span::styled(format!(" {label} "), Style::new().fg(Color::Yellow).bold()),
        Span::raw(input.before_cursor().to_string()),
        Span::styled(cursor, Style::new().fg(Color::Black).bg(Color::Yellow)),
        Span::raw(input.after_cursor().to_string()),
        Span::styled(format!("  ({hint})"), Style::new().fg(Color::DarkGray)),
    ])
}

fn completion_popup(frame: &mut Frame, area: Rect, c: &Completions) {
    const MAX_ROWS: u16 = 8;

    let widest = c.names.iter().map(|n| n.chars().count()).max().unwrap_or(0) as u16;
    let width = (widest + 6).max(24).min(area.width.saturating_sub(2).max(1));
    let height = (c.names.len() as u16).min(MAX_ROWS) + 2;
    let height = height.min(area.height);
    let rect = Rect {
        x: area.x + 1,
        y: area.bottom().saturating_sub(height),
        width,
        height,
    };

    let items: Vec<ListItem> = c
        .names
        .iter()
        .map(|n| {
            let color = if n.ends_with('/') {
                Color::LightBlue
            } else {
                ext_color(n)
            };
            ListItem::new(Span::styled(n.clone(), Style::new().fg(color)))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::bordered()
                .border_style(Style::new().fg(Color::Yellow))
                .title(Span::styled(
                    format!(" {} matches ", c.names.len()),
                    Style::new().fg(Color::Yellow).bold(),
                )),
        )
        .highlight_style(Style::new().fg(Color::Black).bg(SELECTED_BG).bold())
        .highlight_symbol("> ");

    let mut state = ListState::default().with_selected(c.selected);
    frame.render_widget(Clear, rect);
    frame.render_stateful_widget(list, rect, &mut state);
}

fn busy_line(app: &App, msg: &str, progress: Option<&Progress>) -> Line<'static> {
    const BAR_W: usize = 24;
    let yellow = Style::new().fg(Color::Yellow);
    let dim = Style::new().fg(Color::DarkGray);

    let fraction = progress.and_then(|p| p.fraction());
    let (filled, empty, tail) = match (fraction, progress) {
        (Some(f), Some(p)) => {
            let filled = (f * BAR_W as f64).round() as usize;
            let tail = format!(
                " {:>3.0}%  {} / {}",
                f * 100.0,
                human_size(p.done()),
                human_size(p.total())
            );
            (filled, BAR_W - filled, tail)
        }
        _ => {
            let block = 6;
            let span = BAR_W - block;
            let step = (app.started.elapsed().as_millis() / 60) as usize % (span * 2);
            let pos = if step <= span { step } else { span * 2 - step };
            let tail = match progress {
                Some(p) if p.done() > 0 => format!(" {}", human_size(p.done())),
                _ => String::new(),
            };
            let mut bar = " ".repeat(pos);
            bar.push_str(&"█".repeat(block));
            return Line::from(vec![
                Span::styled(format!(" … {msg} "), yellow),
                Span::styled("[", dim),
                Span::styled(bar, yellow),
                Span::styled(" ".repeat(span - pos), yellow),
                Span::styled("]", dim),
                Span::styled(tail, dim),
            ]);
        }
    };

    Line::from(vec![
        Span::styled(format!(" … {msg} "), yellow),
        Span::styled("[", dim),
        Span::styled("█".repeat(filled), Style::new().fg(Color::Green)),
        Span::styled("░".repeat(empty), dim),
        Span::styled("]", dim),
        Span::styled(tail, yellow),
    ])
}
