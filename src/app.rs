use crate::api::{
    delete::delete_file,
    download::download_file,
    files::{get_all_files, FileEntry},
    rename::rename_file,
    sys::SysEntry,
    upload::upload_file,
};
use crate::complete::PathPrompt;
use crate::input::TextInput;
use crate::progress::Progress;
use crate::util::parse_path;
use ratatui::widgets::ListState;
use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    time::Instant,
};

pub enum Status {
    Idle,
    Busy(String, Option<Arc<Progress>>),
    Done(String),
    Failed(String),
    Confirm(String),
}

pub struct App {
    pub files: Vec<FileEntry>,
    pub sys: Option<SysEntry>,
    pub state: ListState,
    pub status: Status,
    pub started: Instant,

    pub upload_input: Option<PathPrompt>,
    pub rename_input: Option<(String, TextInput)>,
    pub pending_delete: Option<String>,

    sys_rx: Receiver<SysEntry>,
    files_rx: Receiver<Vec<FileEntry>>,

    dl_tx: Sender<Result<String, String>>,
    dl_rx: Receiver<Result<String, String>>,
    del_tx: Sender<Result<String, String>>,
    del_rx: Receiver<Result<String, String>>,
    rn_tx: Sender<Result<(String, String), String>>,
    rn_rx: Receiver<Result<(String, String), String>>,
    up_tx: Sender<Result<(String, Vec<FileEntry>), String>>,
    up_rx: Receiver<Result<(String, Vec<FileEntry>), String>>,
}

impl App {
    pub fn new(
        files: Vec<FileEntry>,
        sys_rx: Receiver<SysEntry>,
        files_rx: Receiver<Vec<FileEntry>>,
    ) -> Self {
        let mut state = ListState::default();
        if !files.is_empty() {
            state.select(Some(0));
        }
        let (dl_tx, dl_rx) = mpsc::channel();
        let (del_tx, del_rx) = mpsc::channel();
        let (rn_tx, rn_rx) = mpsc::channel();
        let (up_tx, up_rx) = mpsc::channel();
        Self {
            files,
            sys: None,
            state,
            status: Status::Idle,
            started: Instant::now(),
            upload_input: None,
            rename_input: None,
            pending_delete: None,
            sys_rx,
            files_rx,
            dl_tx,
            dl_rx,
            del_tx,
            del_rx,
            rn_tx,
            rn_rx,
            up_tx,
            up_rx,
        }
    }

    pub fn poll_background(&mut self) {
        while let Ok(s) = self.sys_rx.try_recv() {
            self.sys = Some(s);
        }
        if let Some(files) = self.files_rx.try_iter().last() {
            self.set_files(files);
        }
        while let Ok(result) = self.dl_rx.try_recv() {
            self.status = match result {
                Ok(m) => Status::Done(m),
                Err(m) => Status::Failed(m),
            };
        }
        while let Ok(result) = self.rn_rx.try_recv() {
            match result {
                Ok((old, new)) => self.on_renamed(&old, &new),
                Err(m) => self.status = Status::Failed(m),
            }
        }
        while let Ok(result) = self.del_rx.try_recv() {
            match result {
                Ok(name) => self.on_deleted(&name),
                Err(m) => self.status = Status::Failed(m),
            }
        }
        while let Ok(result) = self.up_rx.try_recv() {
            match result {
                Ok((name, files)) => self.on_uploaded(&name, files),
                Err(m) => self.status = Status::Failed(m),
            }
        }
    }

    
    fn set_files(&mut self, files: Vec<FileEntry>) {
        let selected_name = self
            .state
            .selected()
            .and_then(|i| self.files.get(i))
            .map(|f| f.name.clone());
        let old_index = self.state.selected().unwrap_or(0);

        self.files = files;
        if self.files.is_empty() {
            self.state.select(None);
            return;
        }
        let i = selected_name
            .and_then(|n| self.files.iter().position(|f| f.name == n))
            .unwrap_or_else(|| old_index.min(self.files.len() - 1));
        self.state.select(Some(i));
    }


    pub fn ask_upload(&mut self) {
        self.upload_input = Some(PathPrompt::default());
        self.status = Status::Idle;
    }

    pub fn cancel_upload(&mut self) {
        self.upload_input = None;
        self.status = Status::Idle;
    }

    pub fn start_upload(&mut self) {
        let Some(prompt) = self.upload_input.take() else {
            return;
        };
        let path = parse_path(prompt.input.text());
        if path.as_os_str().is_empty() {
            self.status = Status::Idle;
            return;
        }
        if !path.is_file() {
            self.status = Status::Failed(format!("not a file: {}", path.display()));
            return;
        }

        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let tx = self.up_tx.clone();

        let progress = Arc::new(Progress::default());
        self.status = Status::Busy(format!("uploading {name}"), Some(progress.clone()));

        tokio::spawn(async move {
            let msg = match upload_file(&path, progress).await {
                Ok(_) => match get_all_files().await {
                    Ok(files) => Ok((name, files)),
                    Err(e) => Err(format!("uploaded {name}, but refreshing the list failed: {e}")),
                },
                Err(e) => Err(format!("upload failed: {e}")),
            };
            let _ = tx.send(msg);
        });
    }

    fn on_uploaded(&mut self, name: &str, files: Vec<FileEntry>) {
        self.set_files(files);
        self.status = Status::Done(format!("uploaded {name}"));
    }


    pub fn start_download(&mut self) {
        let Some(file) = self.state.selected().and_then(|i| self.files.get(i)) else {
            return;
        };
        let name = file.name.clone();
        let tx = self.dl_tx.clone();

        let progress = Arc::new(Progress::default());
        self.status = Status::Busy(format!("downloading {name}"), Some(progress.clone()));

        tokio::spawn(async move {
            let msg = match download_file(&name, progress).await {
                Ok(path) => Ok(format!("saved to {}", path.display())),
                Err(e) => Err(format!("download failed: {e}")),
            };
            let _ = tx.send(msg);
        });
    }


    pub fn ask_rename(&mut self) {
        let Some(file) = self.state.selected().and_then(|i| self.files.get(i)) else {
            return;
        };
        self.rename_input = Some((file.name.clone(), TextInput::with_text(file.name.clone())));
        self.status = Status::Idle;
    }

    pub fn cancel_rename(&mut self) {
        self.rename_input = None;
        self.status = Status::Idle;
    }

    pub fn start_rename(&mut self) {
        let Some((old, new)) = self.rename_input.take() else {
            return;
        };
        let new = new.text().trim().to_string();
        if new == old {
            self.status = Status::Idle;
            return;
        }
        if new.is_empty() || new.contains('/') || new.contains('\\') {
            self.status = Status::Failed(format!("invalid file name: {new:?}"));
            return;
        }
        if self.files.iter().any(|f| f.name == new) {
            self.status = Status::Failed(format!("{new} already exists"));
            return;
        }
        let tx = self.rn_tx.clone();

        self.status = Status::Busy(format!("renaming {old} → {new}"), None);

        tokio::spawn(async move {
            let msg = match rename_file(&old, &new).await {
                Ok(()) => Ok((old, new)),
                Err(e) => Err(format!("rename failed: {e}")),
            };
            let _ = tx.send(msg);
        });
    }

    fn on_renamed(&mut self, old: &str, new: &str) {
        if let Some(f) = self.files.iter_mut().find(|f| f.name == old) {
            f.name = new.to_string();
        }
        self.status = Status::Done(format!("renamed {old} → {new}"));
    }


    pub fn ask_delete(&mut self) {
        let Some(file) = self.state.selected().and_then(|i| self.files.get(i)) else {
            return;
        };
        let name = file.name.clone();
        self.status = Status::Confirm(format!(
            "Delete {name}? Press y to confirm, any other key to cancel"
        ));
        self.pending_delete = Some(name);
    }

    pub fn confirm_delete(&mut self) {
        let Some(name) = self.pending_delete.take() else {
            return;
        };
        let tx = self.del_tx.clone();

        self.status = Status::Busy(format!("deleting {name}"), None);

        tokio::spawn(async move {
            let msg = match delete_file(&name).await {
                Ok(()) => Ok(name),
                Err(e) => Err(format!("delete failed: {e}")),
            };
            let _ = tx.send(msg);
        });
    }

    pub fn cancel_delete(&mut self) {
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
}
