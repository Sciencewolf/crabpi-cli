use serde::Deserialize;
use std::{env, error::Error};

#[derive(Debug, Deserialize)]
struct FilesResponse {
    files: Vec<FileEntry>,
}

#[derive(Debug, Deserialize)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub size: u64,
    #[serde(rename = "type")]
    pub kind: String,
}

pub async fn get_all_files() -> Result<Vec<FileEntry>, Box<dyn Error>> {
    let url = env::var("API_URL").map_err(|_| "API_URL is not set")?;
    let body = reqwest::Client::new()
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .json::<FilesResponse>()
        .await?;
    Ok(body.files)
}