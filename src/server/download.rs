use anyhow::Result;
use reqwest::header::CONTENT_DISPOSITION;
use std::fs::File;
use std::io::Write;

pub async fn download_file(url: &str) -> Result<String> {
    let response = reqwest::get(url).await?;
    let content_disposition = response
        .headers()
        .get(CONTENT_DISPOSITION)
        .and_then(|h| h.to_str().ok())
        .map(String::from);

    let bytes = response.bytes().await?;

    let filename = if let Some(content_disposition) = content_disposition {
        content_disposition
            .split(';')
            .find_map(|part| part.trim().strip_prefix("filename="))
            .ok_or_else(|| anyhow::anyhow!("No filename in Content-Disposition"))?
            .trim_matches('"')
            .to_string()
    } else {
        format!("server-{}.jar", blake3::hash(&bytes))
    };

    let mut file = File::create(&filename)?;
    file.write_all(&bytes)?;
    Ok(filename)
}

pub async fn download_file_with_progress(
    url: &str,
    progress_bar: &indicatif::ProgressBar,
) -> Result<String> {
    progress_bar.set_message(format!("Downloading {url}"));
    let filename = download_file(url).await?;
    progress_bar.set_message(format!("Downloaded {filename}"));
    Ok(filename)
}
