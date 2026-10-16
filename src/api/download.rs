use crate::progress::Progress;
use std::{env, error::Error, path::PathBuf, sync::Arc};
use tokio::{fs::File, io::AsyncWriteExt};

pub fn file_url(name: &str) -> Result<reqwest::Url, Box<dyn Error + Send + Sync>> {
    let base = env::var("API_URL")?;
    let mut url = reqwest::Url::parse(&base)?;
    url.path_segments_mut()
        .map_err(|_| "API_URL cannot be used as a base URL")?
        .pop_if_empty()
        .push("files")
        .push(name);
    Ok(url)
}

pub async fn download_file(
    name: &str,
    progress: Arc<Progress>,
) -> Result<PathBuf, Box<dyn Error + Send + Sync>> {
    let url = file_url(name)?;

    let safe_name = std::path::Path::new(name)
        .file_name()
        .ok_or("invalid file name")?;
    let dir = dirs::download_dir().unwrap_or_else(|| PathBuf::from("."));
    let path = dir.join(safe_name);

    let mut response = reqwest::get(url).await?.error_for_status()?;
    progress.set_total(response.content_length().unwrap_or(0));
    let mut file = File::create(&path).await?;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        progress.add(chunk.len() as u64);
    }
    file.flush().await?;

    Ok(path)
}