use std::path::PathBuf;
use std::time::Duration;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

use crate::error::PackError;

use super::adoptium::ReleaseAsset;

const MAX_ATTEMPTS: u32 = 3;
const INITIAL_BACKOFF_SECS: u64 = 1;

pub async fn download_jdk(
    release: &ReleaseAsset,
    mp: &MultiProgress,
) -> Result<PathBuf, PackError> {
    let url = &release.binary.package.link;
    let expected_sha = &release.binary.package.checksum;
    let file_name = &release.binary.package.name;

    let cache_dir = crate::config::BuildConfig::cache_dir()?;
    std::fs::create_dir_all(&cache_dir)?;
    let dest = cache_dir.join(file_name);

    if dest.exists() {
        if verify_checksum(&dest, expected_sha)? {
            tracing::info!("archive already downloaded and verified");
            return Ok(dest);
        }
        std::fs::remove_file(&dest)?;
    }

    tracing::info!("downloading JDK from {url}");

    let mut last_error = None;

    for attempt in 1..=MAX_ATTEMPTS {
        match try_download(url, &dest, release.binary.package.size, mp).await {
            Ok(()) => {
                let actual_hash = file_sha256(&dest)?;
                if actual_hash != *expected_sha {
                    std::fs::remove_file(&dest).ok();
                    return Err(PackError::ChecksumMismatch {
                        expected: expected_sha.clone(),
                        actual: actual_hash,
                    });
                }
                return Ok(dest);
            }
            Err(DownloadAttemptError::Retryable(msg)) => {
                last_error = Some(msg.clone());
                std::fs::remove_file(&dest).ok();

                if attempt < MAX_ATTEMPTS {
                    let delay = INITIAL_BACKOFF_SECS * 2u64.pow(attempt - 1);
                    tracing::warn!(
                        "download failed, retrying in {delay}s... (attempt {}/{MAX_ATTEMPTS}): {msg}",
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_secs(delay)).await;
                }
            }
            Err(DownloadAttemptError::Permanent(msg)) => {
                std::fs::remove_file(&dest).ok();
                return Err(PackError::JdkDownload(msg));
            }
            Err(DownloadAttemptError::RetryAfter(secs, msg)) => {
                last_error = Some(msg.clone());
                std::fs::remove_file(&dest).ok();

                if attempt < MAX_ATTEMPTS {
                    tracing::warn!(
                        "rate limited, retrying in {secs}s... (attempt {}/{MAX_ATTEMPTS}): {msg}",
                        attempt + 1
                    );
                    tokio::time::sleep(Duration::from_secs(secs)).await;
                }
            }
        }
    }

    Err(PackError::JdkDownload(format!(
        "download failed after {MAX_ATTEMPTS} attempts: {}",
        last_error.unwrap_or_else(|| "unknown error".into())
    )))
}

enum DownloadAttemptError {
    Retryable(String),
    Permanent(String),
    RetryAfter(u64, String),
}

async fn try_download(
    url: &str,
    dest: &PathBuf,
    fallback_size: u64,
    mp: &MultiProgress,
) -> Result<(), DownloadAttemptError> {
    let response = reqwest::get(url)
        .await
        .map_err(|e| classify_reqwest_error(&e))?;

    let status = response.status();
    if !status.is_success() {
        let msg = format!("HTTP {status}");
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(5);
            return Err(DownloadAttemptError::RetryAfter(retry_after, msg));
        }
        if status.is_server_error() {
            return Err(DownloadAttemptError::Retryable(msg));
        }
        return Err(DownloadAttemptError::Permanent(msg));
    }

    let total_size = response.content_length().unwrap_or(fallback_size);

    let pb = mp.add(ProgressBar::new(total_size));
    pb.set_style(
        ProgressStyle::default_bar()
            .template("  {msg} [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .expect("invalid progress bar template")
            .progress_chars("=> "),
    );
    pb.set_message("Downloading JDK");

    let mut file = tokio::fs::File::create(&dest)
        .await
        .map_err(|e| DownloadAttemptError::Permanent(format!("create file: {e}")))?;

    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| classify_reqwest_error(&e))?
    {
        file.write_all(&chunk)
            .await
            .map_err(|e| DownloadAttemptError::Permanent(format!("write file: {e}")))?;
        pb.inc(chunk.len() as u64);
    }
    file.flush()
        .await
        .map_err(|e| DownloadAttemptError::Permanent(format!("flush file: {e}")))?;

    pb.finish_and_clear();
    Ok(())
}

fn classify_reqwest_error(e: &reqwest::Error) -> DownloadAttemptError {
    if e.is_timeout() || e.is_connect() || e.is_request() {
        DownloadAttemptError::Retryable(e.to_string())
    } else if let Some(status) = e.status() {
        if status.is_server_error() || status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            DownloadAttemptError::Retryable(e.to_string())
        } else {
            DownloadAttemptError::Permanent(e.to_string())
        }
    } else {
        DownloadAttemptError::Retryable(e.to_string())
    }
}

fn file_sha256(path: &PathBuf) -> Result<String, PackError> {
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    std::io::copy(&mut reader, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

fn verify_checksum(path: &PathBuf, expected: &str) -> Result<bool, PackError> {
    Ok(file_sha256(path)? == expected)
}
