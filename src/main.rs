mod files;

use files::{get_all_files, FileEntry};
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    style::{Style, Stylize},
    widgets::{Block, List, ListItem, ListState},
    DefaultTerminal, Frame,
};

struct App {
    files: Vec<FileEntry>,
    state: ListState,
}

impl App {
    fn new(files: Vec<FileEntry>) -> Self {
        let mut state = ListState::default();
        if !files.is_empty() {
            state.select(Some(0));
        }
        Self { files, state }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let items: Vec<ListItem> = self
            .files
            .iter()
            .map(|f| ListItem::new(format!("{:<40} {:>10}", f.name, human_size(f.size))))
            .collect();

        let list = List::new(items)
            .block(Block::bordered().title(" Files  (↑/↓ move, q quit) "))
            .highlight_style(Style::new().reversed())
            .highlight_symbol("> ");

        frame.render_stateful_widget(list, frame.area(), &mut self.state);
    }
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
        terminal.draw(|frame| app.draw(frame))?;
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                KeyCode::Down => app.state.select_next(),
                KeyCode::Up => app.state.select_previous(),
                _ => {}
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let files = get_all_files().await?;

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut App::new(files));
    ratatui::restore();
    Ok(result?)
}