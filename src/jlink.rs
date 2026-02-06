use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::PackError;
use crate::jvm::cache::jdk_bin;

pub fn detect_modules(jdk_path: &Path, jar_path: &Path) -> Result<String, PackError> {
    let jdeps = jdk_bin(jdk_path, "jdeps");

    let jar_str = jar_path
        .to_str()
        .ok_or_else(|| PackError::JdepsFailed("JAR path contains invalid UTF-8".into()))?;

    let args = [
        "--print-module-deps",
        "--ignore-missing-deps",
        "--multi-release",
        "base",
        jar_str,
    ];

    let cmd_str = format!("{} {}", jdeps.display(), args.join(" "));
    tracing::info!("running: {cmd_str}");

    let output = Command::new(&jdeps)
        .args(args)
        .output()
        .map_err(|e| PackError::JdepsFailed(format!("failed to run jdeps: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // jdeps can fail on some JARs, fall back to java.base
        tracing::warn!("jdeps failed, falling back to common modules: {stderr}");
        return Ok("java.base,java.logging,java.sql,java.naming,java.management,java.instrument,java.desktop,java.xml,java.net.http".to_string());
    }

    let modules = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if modules.is_empty() {
        return Ok("java.base".to_string());
    }

    Ok(modules)
}

pub fn create_runtime(
    jdk_path: &Path,
    modules: &str,
    output_dir: &Path,
) -> Result<PathBuf, PackError> {
    let jlink_bin = jdk_bin(jdk_path, "jlink");
    let runtime_path = output_dir.join("runtime");

    if runtime_path.exists() {
        std::fs::remove_dir_all(&runtime_path)?;
    }

    let runtime_str = runtime_path
        .to_str()
        .ok_or_else(|| PackError::JlinkFailed("runtime path contains invalid UTF-8".into()))?;

    let args = [
        "--add-modules",
        modules,
        "--strip-debug",
        "--no-man-pages",
        "--no-header-files",
        "--compress=zip-6",
        "--output",
        runtime_str,
    ];

    let cmd_str = format!("{} {}", jlink_bin.display(), args.join(" "));
    tracing::info!("running: {cmd_str}");

    let output = Command::new(&jlink_bin)
        .args(args)
        .output()
        .map_err(|e| PackError::JlinkFailed(format!("failed to run jlink: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut msg = format!("command: {cmd_str}\n");
        if !stderr.is_empty() {
            msg.push_str(&format!("stderr:\n{stderr}"));
        }
        if !stdout.is_empty() {
            msg.push_str(&format!("stdout:\n{stdout}"));
        }
        if stderr.is_empty() && stdout.is_empty() {
            msg.push_str(&format!(
                "process exited with {}",
                output
                    .status
                    .code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "signal".to_string())
            ));
        }
        return Err(PackError::JlinkFailed(msg));
    }

    Ok(runtime_path)
}
