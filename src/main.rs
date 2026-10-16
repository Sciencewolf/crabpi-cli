//! crabpi-cli: terminal UI for the crabpi file server.
//!
//! api/       HTTP calls (list, upload, download, rename, delete, system info)
//! app.rs     state + actions; background tasks report back through channels
//! events.rs  main loop and key handling
//! ui.rs      drawing
//! input.rs   text field with cursor editing
//! complete.rs  Tab completion for the upload path prompt
//! progress.rs / util.rs  small shared helpers

mod api;
mod app;
mod complete;
mod events;
mod input;
mod progress;
mod ui;
mod util;

use api::{
    files::{get_all_files, FileEntry},
    sys::{get_sys_info, SysEntry},
};
use app::App;
use std::{
    sync::mpsc::{self, Receiver},
    time::Duration,
};

const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Fetches system info every `POLL_INTERVAL` and sends it to the UI.
fn spawn_sys_poller() -> Receiver<SysEntry> {
    let (tx, rx) = mpsc::channel();
    tokio::spawn(async move {
        loop {
            if let Ok(s) = get_sys_info().await {
                if tx.send(s).is_err() {
                    break;
                }
            }
            tokio::time::sleep(POLL_INTERVAL).await;
        }
    });
    rx
}

/// Re-fetches the file list every `POLL_INTERVAL` (the first list is loaded in `main`).
fn spawn_files_poller() -> Receiver<Vec<FileEntry>> {
    let (tx, rx) = mpsc::channel();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(POLL_INTERVAL).await;
            if let Ok(files) = get_all_files().await {
                if tx.send(files).is_err() {
                    break;
                }
            }
        }
    });
    rx
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let files = get_all_files().await?;
    let mut app = App::new(files, spawn_sys_poller(), spawn_files_poller());

    let mut terminal = ratatui::init();
    let result = events::run(&mut terminal, &mut app);
    ratatui::restore();
    Ok(result?)
}
