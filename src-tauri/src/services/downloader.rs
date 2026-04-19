use reqwest::Client;
use std::fs::{OpenOptions, metadata};
use std::io::Write;
use futures_util::StreamExt;

pub async fn download_with_resume<F>(
    url: &str,
    path: &std::path::Path,
    mut progress: F,
) -> Result<(), String> 
where 
    F: FnMut(u64, u64) + Send + 'static 
{
    let client = Client::builder()
        .user_agent("Operarius-Orchestrator/1.0 (Macintosh; Apple Silicon)")
        .build()
        .map_err(|e| format!("Client init failed: {}", e))?;

    let existing_size = metadata(path).map(|m| m.len()).unwrap_or(0);

    let mut request = client.get(url);

    if existing_size > 0 {
        request = request.header("Range", format!("bytes={}-", existing_size));
    }

    let response = request.send().await.map_err(|e| format!("Request failed: {}", e))?;
    
    // If we get a 416 (Range Not Satisfiable), it likely means we are done or the file changed
    if response.status() == 416 {
        return Ok(());
    }

    let content_length = response.content_length().unwrap_or(0);
    let total_size = content_length + existing_size;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("File open failed: {}", e))?;

    let mut downloaded = existing_size;
    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&chunk).map_err(|e| format!("Write failed: {}", e))?;
        downloaded += chunk.len() as u64;

        progress(downloaded, total_size);
    }

    Ok(())
}
