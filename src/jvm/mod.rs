pub mod adoptium;
pub mod cache;
pub mod download;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use fs2::FileExt;
use indicatif::MultiProgress;

use crate::config::Target;
use crate::error::PackError;

const LOCK_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes
const LOCK_POLL_INTERVAL: Duration = Duration::from_millis(500);

pub async fn ensure_jdk(
    version: u8,
    target: &Target,
    mp: &MultiProgress,
) -> Result<PathBuf, PackError> {
    let cache_path = cache::cached_jdk_path(version, target)?;

    // Fast path: already cached, no lock needed
    if cache_path.exists() {
        tracing::info!("using cached JDK {} at {}", version, cache_path.display());
        return Ok(cache_path);
    }

    // Acquire file lock before download/extract
    let lock_path = cache_path.with_extension("lock");
    std::fs::create_dir_all(lock_path.parent().unwrap_or(&lock_path))?;
    let lock_file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&lock_path)?;

    let start = Instant::now();
    let mut warned = false;
    loop {
        match lock_file.try_lock_exclusive() {
            Ok(()) => break,
            Err(_) => {
                if start.elapsed() >= LOCK_TIMEOUT {
                    return Err(PackError::CacheLockTimeout {
                        version,
                        target: format!("{}-{}", target.adoptium_os(), target.adoptium_arch()),
                    });
                }
                if !warned {
                    tracing::warn!(
                        "waiting for another process to finish downloading JDK {version}..."
                    );
                    warned = true;
                }
                std::thread::sleep(LOCK_POLL_INTERVAL);
            }
        }
    }

    // Re-check after acquiring lock (another process may have populated the cache)
    if cache_path.exists() {
        tracing::info!("using cached JDK {} at {}", version, cache_path.display());
        lock_file.unlock().ok();
        return Ok(cache_path);
    }

    let result = async {
        let release = adoptium::fetch_latest_release(version, target).await?;
        let archive_path = download::download_jdk(&release, mp).await?;
        cache::extract_and_cache(version, target, &archive_path)
    }
    .await;

    lock_file.unlock().ok();

    result
}
