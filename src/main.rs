mod delete;
mod download;
mod files;
mod sys;

use delete::delete_file;
use download::download_file;
use files::{get_all_files, FileEntry};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, List, ListItem, ListState, Paragraph},
    DefaultTerminal, Frame,
};
use std::{
    path::Path,
    sync::mpsc::{self, Receiver, Sender},
    time::Duration,
};
use sys::{get_sys_info, SysEntry};

const INFO_LINES: u16 = 6;

const BORDER: Color = Color::Cyan;
const SELECTED_BG: Color = Color::Cyan;

enum Status {
    Idle,
    Busy(String),
    Done(String),
    Failed(String),
    Confirm(String),
}

struct App {
    files: Vec<FileEntry>,
    sys: Option<SysEntry>,
    rx: Receiver<SysEntry>,
    state: ListState,
    status: Status,
    dl_tx: Sender<Result<String, String>>,
    dl_rx: Receiver<Result<String, String>>,
    del_tx: Sender<Result<String, String>>,
    del_rx: Receiver<Result<String, String>>,
    pending_delete: Option<String>,
}

impl App {
    fn new(files: Vec<FileEntry>, rx: Receiver<SysEntry>) -> Self {
        let mut state = ListState::default();
        if !files.is_empty() {
            state.select(Some(0));
        }
        let (dl_tx, dl_rx) = mpsc::channel();
        let (del_tx, del_rx) = mpsc::channel();
        Self {
            files,
            sys: None,
            rx,
            state,
            status: Status::Idle,
            dl_tx,
            dl_rx,
            del_tx,
            del_rx,
            pending_delete: None,
        }
    }

    fn start_download(&mut self) {
        let Some(file) = self.state.selected().and_then(|i| self.files.get(i)) else {
            return;
        };
        let name = file.name.clone();
        let tx = self.dl_tx.clone();

        self.status = Status::Busy(format!("downloading {name}..."));

        tokio::spawn(async move {
            let msg = match download_file(&name).await {
                Ok(path) => Ok(format!("saved to {}", path.display())),
                Err(e) => Err(format!("download failed: {e}")),
            };
            let _ = tx.send(msg);
        });
    }

    fn ask_delete(&mut self) {
        let Some(file) = self.state.selected().and_then(|i| self.files.get(i)) else {
            return;
        };
        let name = file.name.clone();
        self.status = Status::Confirm(format!(
            "Delete {name}? Press y to confirm, any other key to cancel"
        ));
        self.pending_delete = Some(name);
    }

    fn confirm_delete(&mut self) {
        let Some(name) = self.pending_delete.take() else {
            return;
        };
        let tx = self.del_tx.clone();

        self.status = Status::Busy(format!("deleting {name}..."));

        tokio::spawn(async move {
            let msg = match delete_file(&name).await {
                Ok(()) => Ok(name),
                Err(e) => Err(format!("delete failed: {e}")),
            };
            let _ = tx.send(msg);
        });
    }

    fn cancel_delete(&mut self) {
        self.pending_delete = None;
        self.status = Status::Idle;
    }

    fn on_deleted(&mut self, name: &str) {
        self.files.retain(|f| f.name != name);
        if self.files.is_empty() {
            self.state.select(None);
        } else {
            let i = self.state.selected().unwrap_or(0).min(self.files.len() - 1);
            self.state.select(Some(i));
        }
        self.status = Status::Done(format!("deleted {name}"));
    }

    fn draw(&mut self, frame: &mut Frame) {
        let [top, body, status] = Layout::vertical([
            Constraint::Length(INFO_LINES + 2),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
            .areas(frame.area());

        let sys_text: Vec<Line<'static>> = match &self.sys {
            Some(s) => sys_lines(s),
            None => vec![Line::from(Span::styled(
                "loading system info...",
                Style::new().fg(Color::Yellow).italic(),
            ))],
        };

        frame.render_widget(
            Paragraph::new(sys_text).block(
                Block::bordered()
                    .border_style(Style::new().fg(BORDER))
                    .title(Span::styled(
                        " System ",
                        Style::new().fg(Color::Yellow).bold(),
                    )),
            ),
            top,
        );

        let items: Vec<ListItem> = self.files.iter().map(file_item).collect();

        let hint = |k: &'static str, t: &'static str| {
            [
                Span::styled(k, Style::new().fg(Color::Yellow)),
                Span::styled(format!(" {t}  "), Style::new().fg(Color::DarkGray)),
            ]
        };
        let mut title_spans = vec![
            Span::styled(" Files ", Style::new().fg(Color::Green).bold()),
            Span::styled("(", Style::new().fg(Color::DarkGray)),
        ];
        title_spans.extend(hint("↑/↓", "move"));
        title_spans.extend(hint("d", "download"));
        title_spans.extend(hint("x", "delete"));
        title_spans.extend(hint("q", "quit"));
        title_spans.push(Span::styled(") ", Style::new().fg(Color::DarkGray)));

        let list = List::new(items)
            .block(
                Block::bordered()
                    .border_style(Style::new().fg(BORDER))
                    .title(Line::from(title_spans)),
            )
            .highlight_style(Style::new().fg(Color::Black).bg(SELECTED_BG).bold())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, body, &mut self.state);

        frame.render_widget(Paragraph::new(self.status_line()), status);
    }

    fn status_line(&self) -> Line<'static> {
        match &self.status {
            Status::Idle => Line::default(),
            Status::Busy(m) => Line::from(Span::styled(
                format!(" … {m}"),
                Style::new().fg(Color::Yellow),
            )),
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

fn human_size(bytes: u64) -> String {
    let units = ["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut i = 0;
    while size >= 1024.0 && i < units.len() - 1 {
        size /= 1024.0;
        i += 1;
    }
    format!("{size:.1} {}", units[i])
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        while let Ok(s) = app.rx.try_recv() {
            app.sys = Some(s);
        }
        while let Ok(result) = app.dl_rx.try_recv() {
            app.status = match result {
                Ok(m) => Status::Done(m),
                Err(m) => Status::Failed(m),
            };
        }
        while let Ok(result) = app.del_rx.try_recv() {
            match result {
                Ok(name) => app.on_deleted(&name),
                Err(m) => app.status = Status::Failed(m),
            }
        }
        terminal.draw(|frame| app.draw(frame))?;

        if !event::poll(Duration::from_millis(250))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            if app.pending_delete.is_some() {
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
                    _ => app.cancel_delete(),
                }
                continue;
            }

            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Down => app.state.select_next(),
                KeyCode::Up => app.state.select_previous(),
                KeyCode::Char('d') | KeyCode::Char('D') => app.start_download(),
                KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Delete => app.ask_delete(),
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let files = get_all_files().await?;

    let (tx, rx) = mpsc::channel();
    tokio::spawn(async move {
        loop {
            if let Ok(s) = get_sys_info().await {
                if tx.send(s).is_err() {
                    break;
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut App::new(files, rx));
    ratatui::restore();
    Ok(result?)
}