use crate::progress::Progress;
use futures_util::TryStreamExt;
use reqwest::{multipart, Body};
use std::env;
use std::path::Path;
use std::sync::Arc;
use tokio_util::codec::{BytesCodec, FramedRead};


pub async fn upload_file(
    path: &Path,
    progress: Arc<Progress>,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let url = env::var("API_URL").map_err(|_| "API_URL is not set")?.to_string() + "/files";

    let file_name = path.file_name().and_then(|n| n.to_str())
        .ok_or("invalid file name")?
        .to_string();

    let file = tokio::fs::File::open(path).await?;
    let len = file.metadata().await?.len();
    progress.set_total(len);

    let stream = FramedRead::new(file, BytesCodec::new())
        .inspect_ok(move |chunk| progress.add(chunk.len() as u64));
    let body = Body::wrap_stream(stream);
    let mime = mime_guess::from_path(path).first_or_octet_stream();

    let part = multipart::Part::stream_with_length(body, len)
        .file_name(file_name)
        .mime_str(mime.as_ref())?;

    let form = multipart::Form::new().part("files", part);

    let resp = reqwest::Client::new()
        .post(url)
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;

    Ok(resp.json::<serde_json::Value>().await?)
}
