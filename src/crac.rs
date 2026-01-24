use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use flate2::write::GzEncoder;
use flate2::Compression;

use crate::error::PackError;
use crate::jvm::cache::jdk_bin;

/// Create a CRaC checkpoint for instant restore.
/// Returns the path to a tar.gz containing the checkpoint directory.
/// Uses `runtime_dir` for java (jlinked runtime) and `jdk_path` for jcmd
/// (which is not included in minimal jlinked runtimes).
pub fn create_checkpoint(
    runtime_dir: &Path,
    jdk_path: &Path,
    jar_path: &Path,
    work_dir: &Path,
) -> Result<PathBuf, PackError> {
    let java = jdk_bin(runtime_dir, "java");
    let jcmd = jdk_bin(jdk_path, "jcmd");
    let cr_dir = work_dir.join("cr");

    if !jcmd.exists() {
        return Err(PackError::CracCheckpointFailed(format!(
            "jcmd not found at {}. CRaC requires jcmd from the full JDK",
            jcmd.display()
        )));
    }

    // Verify CRaC support
    verify_crac_support(&java)?;

    std::fs::create_dir_all(&cr_dir)?;

    tracing::info!("launching app for CRaC checkpoint");

    // Launch the app with CRaC checkpoint target
    let mut child = Command::new(&java)
        .arg(format!("-XX:CRaCCheckpointTo={}", cr_dir.display()))
        .arg("-jar")
        .arg(jar_path.as_os_str())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| PackError::CracCheckpointFailed(format!("failed to spawn java: {e}")))?;

    let pid = child.id();

    // Wait for warmup
    tracing::info!("waiting for app warmup (10s)");
    std::thread::sleep(Duration::from_secs(10));

    // Trigger checkpoint via jcmd
    tracing::info!("triggering CRaC checkpoint via jcmd");
    let jcmd_output = Command::new(&jcmd)
        .arg(pid.to_string())
        .arg("JDK.checkpoint")
        .output()
        .map_err(|e| PackError::CracCheckpointFailed(format!("failed to run jcmd: {e}")))?;

    if !jcmd_output.status.success() {
        let _ = child.kill();
        let _ = child.wait();
        let stderr = String::from_utf8_lossy(&jcmd_output.stderr);
        return Err(PackError::CracCheckpointFailed(format!(
            "jcmd checkpoint failed: {stderr}"
        )));
    }

    // Wait for the process to exit after checkpoint
    let wait_result = wait_for_exit(&mut child, Duration::from_secs(30));
    if wait_result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }

    // Verify checkpoint was created
    if !cr_dir.join("core").exists() && !cr_dir.join("dump4.log").exists() {
        // Check if any files were written to the checkpoint dir
        let entries: Vec<_> = std::fs::read_dir(&cr_dir)
            .map(|r| r.filter_map(|e| e.ok()).collect())
            .unwrap_or_default();
        if entries.is_empty() {
            return Err(PackError::CracCheckpointFailed(
                "checkpoint directory is empty".into(),
            ));
        }
    }

    // Package checkpoint as tar.gz
    let archive_path = work_dir.join("crac.tar.gz");
    package_checkpoint(&cr_dir, &archive_path)?;

    Ok(archive_path)
}

fn verify_crac_support(java: &Path) -> Result<(), PackError> {
    let output = Command::new(java)
        .arg("-XX:CRaCCheckpointTo=/dev/null")
        .arg("-version")
        .output()
        .map_err(|e| PackError::CracCheckpointFailed(format!("failed to run java: {e}")))?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    // If the flag is unrecognized, CRaC is not supported
    if stderr.contains("Unrecognized VM option")
        || stderr.contains("Could not create the Java Virtual Machine")
    {
        return Err(PackError::CracNotSupported);
    }

    Ok(())
}

fn wait_for_exit(
    child: &mut std::process::Child,
    timeout: Duration,
) -> Result<std::process::ExitStatus, ()> {
    let start = std::time::Instant::now();
    let poll_interval = Duration::from_millis(100);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => {
                if start.elapsed() >= timeout {
                    return Err(());
                }
                std::thread::sleep(poll_interval);
            }
            Err(_) => return Err(()),
        }
    }
}

fn package_checkpoint(cr_dir: &Path, output: &Path) -> Result<(), PackError> {
    let file = std::fs::File::create(output)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(encoder);

    tar.append_dir_all("cr", cr_dir)?;

    let encoder = tar.into_inner()?;
    encoder.finish()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn package_checkpoint_creates_archive() {
        let dir = tempdir().unwrap();
        let cr_dir = dir.path().join("cr");
        std::fs::create_dir_all(&cr_dir).unwrap();
        std::fs::write(cr_dir.join("core"), b"fake checkpoint").unwrap();
        std::fs::write(cr_dir.join("dump4.log"), b"log data").unwrap();

        let output = dir.path().join("crac.tar.gz");
        package_checkpoint(&cr_dir, &output).unwrap();

        assert!(output.exists());
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
    }

    #[test]
    fn package_checkpoint_contains_cr_dir() {
        let dir = tempdir().unwrap();
        let cr_dir = dir.path().join("cr");
        std::fs::create_dir_all(&cr_dir).unwrap();
        std::fs::write(cr_dir.join("data"), b"checkpoint data").unwrap();

        let output = dir.path().join("crac.tar.gz");
        package_checkpoint(&cr_dir, &output).unwrap();

        let file = std::fs::File::open(&output).unwrap();
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        let entries: Vec<String> = archive
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(entries.iter().any(|e| e.starts_with("cr/")));
    }
}
