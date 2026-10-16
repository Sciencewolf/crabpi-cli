use crate::app::App;
use crate::ui;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    DefaultTerminal,
};
use std::time::Duration;

pub fn run(terminal: &mut DefaultTerminal, app: &mut App) -> std::io::Result<()> {
    loop {
        app.poll_background();
        terminal.draw(|frame| ui::draw(app, frame))?;

        if !event::poll(Duration::from_millis(60))? {
            continue;
        }
        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            if handle_key(app, key) {
                return Ok(());
            }
        }
    }
}

fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;

    if let Some((_, input)) = app.rename_input.as_mut() {
        match code {
            KeyCode::Enter => app.start_rename(),
            KeyCode::Esc => app.cancel_rename(),
            _ => input.handle_key(key),
        }
        return false;
    }

    if let Some(prompt) = app.upload_input.as_mut() {
        match code {
            KeyCode::Enter => app.start_upload(),
            KeyCode::Esc => app.cancel_upload(),
            KeyCode::Tab => prompt.complete(false),
            KeyCode::BackTab => prompt.complete(true),
            _ => prompt.edit(key),
        }
        return false;
    }

    if app.pending_delete.is_some() {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => app.confirm_delete(),
            _ => app.cancel_delete(),
        }
        return false;
    }

    match code {
        KeyCode::Char('q') | KeyCode::Char('Q') | KeyCode::Esc => return true,
        KeyCode::Down => app.state.select_next(),
        KeyCode::Up => app.state.select_previous(),
        KeyCode::Char('d') | KeyCode::Char('D') => app.start_download(),
        KeyCode::Char('u') | KeyCode::Char('U') => app.ask_upload(),
        KeyCode::Char('r') | KeyCode::Char('R') => app.ask_rename(),
        KeyCode::Char('x') | KeyCode::Char('X') | KeyCode::Delete => app.ask_delete(),
        _ => {}
    }
    false
}
