use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use crate::error::PackError;
use crate::jvm::cache::jdk_bin;

/// Generate an AppCDS archive (.jsa) for the runtime + app.jar combination.
/// Falls back to base class list if the app doesn't start cleanly.
pub fn generate(
    runtime_dir: &Path,
    jar_path: &Path,
    work_dir: &Path,
) -> Result<PathBuf, PackError> {
    let java = jdk_bin(runtime_dir, "java");
    let classlist_path = work_dir.join("app.classlist");
    let jsa_path = work_dir.join("app.jsa");

    tracing::info!("generating AppCDS class list");

    // Try to generate classlist by running the app briefly
    let classlist_generated = generate_classlist_from_app(&java, jar_path, &classlist_path);

    // If app-based classlist failed or is empty, use base JDK classes
    if !classlist_generated || !classlist_path.exists() || is_file_empty(&classlist_path) {
        tracing::info!("app classlist failed, falling back to base JDK classes");
        generate_base_classlist(&java, &classlist_path)?;
    }

    // Generate the JSA archive from the class list
    tracing::info!("generating AppCDS archive (JSA)");
    let output = Command::new(&java)
        .arg("-Xshare:dump")
        .arg(format!(
            "-XX:SharedClassListFile={}",
            classlist_path.display()
        ))
        .arg(format!("-XX:SharedArchiveFile={}", jsa_path.display()))
        .arg("-cp")
        .arg(jar_path.as_os_str())
        .output()
        .map_err(|e| PackError::AppcdsGenerationFailed(format!("failed to run java: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PackError::AppcdsGenerationFailed(format!(
            "jsa dump failed: {stderr}"
        )));
    }

    if !jsa_path.exists() {
        return Err(PackError::AppcdsGenerationFailed(
            "JSA file was not created".into(),
        ));
    }

    Ok(jsa_path)
}

fn generate_classlist_from_app(java: &Path, jar_path: &Path, classlist_path: &Path) -> bool {
    // Run the app with class list dumping, kill after timeout
    let child = Command::new(java)
        .arg("-Xshare:off")
        .arg(format!(
            "-XX:DumpLoadedClassList={}",
            classlist_path.display()
        ))
        .arg("-jar")
        .arg(jar_path.as_os_str())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!("failed to spawn java for classlist: {e}");
            return false;
        }
    };

    // Wait up to 30 seconds for the app to load classes
    let timeout = Duration::from_secs(30);
    match child.wait_timeout(timeout) {
        Ok(Some(_status)) => {
            // Process exited on its own (maybe it's a CLI tool)
            true
        }
        Ok(None) => {
            // Timeout — kill the process, classlist should be populated
            let _ = child.kill();
            let _ = child.wait();
            true
        }
        Err(e) => {
            tracing::warn!("error waiting for classlist process: {e}");
            let _ = child.kill();
            let _ = child.wait();
            false
        }
    }
}

fn generate_base_classlist(java: &Path, classlist_path: &Path) -> Result<(), PackError> {
    let output = Command::new(java)
        .arg("-Xshare:off")
        .arg(format!(
            "-XX:DumpLoadedClassList={}",
            classlist_path.display()
        ))
        .arg("-version")
        .output()
        .map_err(|e| PackError::AppcdsGenerationFailed(format!("failed to run java: {e}")))?;

    if !output.status.success() {
        return Err(PackError::AppcdsGenerationFailed(
            "failed to generate base class list".into(),
        ));
    }

    Ok(())
}

fn is_file_empty(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.len() == 0)
        .unwrap_or(true)
}

/// Extension trait for child process timeout (portable)
trait WaitTimeout {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>>;
}

impl WaitTimeout for std::process::Child {
    fn wait_timeout(
        &mut self,
        timeout: Duration,
    ) -> std::io::Result<Option<std::process::ExitStatus>> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        loop {
            match self.try_wait()? {
                Some(status) => return Ok(Some(status)),
                None => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    std::thread::sleep(poll_interval);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn is_file_empty_returns_true_for_nonexistent() {
        assert!(is_file_empty(Path::new("/nonexistent/file.txt")));
    }

    #[test]
    fn is_file_empty_returns_true_for_empty() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("empty.txt");
        std::fs::write(&file, b"").unwrap();
        assert!(is_file_empty(&file));
    }

    #[test]
    fn is_file_empty_returns_false_for_nonempty() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("data.txt");
        std::fs::write(&file, b"content").unwrap();
        assert!(!is_file_empty(&file));
    }
}
