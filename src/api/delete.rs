use super::download::file_url;
use std::error::Error;

pub async fn delete_file(name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = file_url(name)?;
    reqwest::Client::new()
        .delete(url)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}