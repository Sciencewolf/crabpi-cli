use super::download::file_url;
use serde_json::json;
use std::error::Error;

pub async fn rename_file(name: &str, new_name: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    let url = file_url(name)?;
    reqwest::Client::new()
        .patch(url)
        .json(&json!({ "name": new_name }))
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
