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
