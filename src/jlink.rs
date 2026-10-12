use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::PackError;
use crate::jvm::cache::jdk_bin;

/// Resolve the jmods directory for a JDK, handling macOS Contents/Home structure.
fn jdk_jmods(jdk_path: &Path) -> PathBuf {
    let macos_jmods = jdk_path.join("Contents").join("Home").join("jmods");
    if macos_jmods.exists() {
        return macos_jmods;
    }
    jdk_path.join("jmods")
}

/// Pick the `--compress` flag from `jlink --help` output. Newer JDKs accept the
/// `zip-N` spelling; older ones only accept the numeric `0/1/2`. Both mean "no
/// internal compression" here (`zip-0` / `0`). Defaults to the numeric form,
/// which every supported JDK understands (still valid, if deprecated, on new
/// JDKs) unless the help text shows `zip-N` is available.
fn compress_flag_from_help(help: &str) -> &'static str {
    if help.contains("zip-") {
        "--compress=zip-0"
    } else {
        "--compress=0"
    }
}

/// Probe the given jlink binary to decide which `--compress` spelling it takes.
/// Falls back to the numeric form if the probe can't run.
fn jlink_compress_flag(jlink_bin: &Path) -> &'static str {
    match Command::new(jlink_bin).arg("--help").output() {
        Ok(out) => {
            let mut help = String::from_utf8_lossy(&out.stdout).into_owned();
            help.push_str(&String::from_utf8_lossy(&out.stderr));
            compress_flag_from_help(&help)
        }
        Err(_) => "--compress=0",
    }
}

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
        // jdeps can fail on some JARs, fall back to common modules
        eprintln!("  ⚠ jdeps failed, falling back to common modules");
        eprintln!("    command: {cmd_str}");
        if !stderr.trim().is_empty() {
            for line in stderr.trim().lines() {
                eprintln!("    {line}");
            }
        }
        tracing::info!("jdeps fallback triggered: {stderr}");
        return Ok("java.base,java.logging,java.sql,java.naming,java.management,java.instrument,java.desktop,java.xml,java.net.http".to_string());
    }

    let modules = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if modules.is_empty() {
        return Ok("java.base".to_string());
    }

    Ok(modules)
}

/// Create a minimal JVM runtime using jlink.
///
/// When cross-compiling, `target_jdk_path` provides the path to the target
/// platform's JDK so jlink can use its jmods. The jlink binary itself is
/// always taken from `jdk_path` (the host JDK).
pub fn create_runtime(
    jdk_path: &Path,
    modules: &str,
    output_dir: &Path,
    target_jdk_path: Option<&Path>,
) -> Result<PathBuf, PackError> {
    let jlink_bin = jdk_bin(jdk_path, "jlink");
    let runtime_path = output_dir.join("runtime");

    if runtime_path.exists() {
        std::fs::remove_dir_all(&runtime_path)?;
    }

    let runtime_str = runtime_path
        .to_str()
        .ok_or_else(|| PackError::JlinkFailed("runtime path contains invalid UTF-8".into()))?;

    // No internal compression: the outer tar.gz compresses more efficiently and
    // double-compressing just wastes CPU. The flag spelling changed across JDKs
    // (numeric `0` vs `zip-0`), and the same major version differs across builds
    // (GraalVM 21 accepts zip-N, Temurin 21 doesn't), so probe what this jlink
    // actually takes instead of guessing from the version number.
    let compress_flag = jlink_compress_flag(&jlink_bin);

    // When cross-compiling, point jlink to target JDK's jmods
    let module_path_str;
    let mut args = vec![];
    if let Some(target_jdk) = target_jdk_path {
        let jmods = jdk_jmods(target_jdk);
        if !jmods.exists() {
            return Err(PackError::JlinkFailed(format!(
                "target JDK jmods directory not found: {}",
                jmods.display()
            )));
        }
        module_path_str = jmods.to_string_lossy().to_string();
        args.push("--module-path");
        args.push(&module_path_str);
    }

    args.extend_from_slice(&[
        "--add-modules",
        modules,
        "--strip-debug",
        "--no-man-pages",
        "--no-header-files",
        compress_flag,
        "--output",
        runtime_str,
    ]);

    let cmd_str = format!("{} {}", jlink_bin.display(), args.join(" "));
    tracing::info!("running: {cmd_str}");

    let output = Command::new(&jlink_bin)
        .args(&args)
        .output()
        .map_err(|e| PackError::JlinkFailed(format!("failed to run jlink: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let exit_code = output
            .status
            .code()
            .map(|c| c.to_string())
            .unwrap_or_else(|| "signal".to_string());

        let mut msg = format!("exit code {exit_code}\n");
        msg.push_str(&format!("  command: {cmd_str}\n"));
        if !stderr.trim().is_empty() {
            msg.push_str("  stderr:\n");
            for line in stderr.trim().lines() {
                msg.push_str(&format!("    {line}\n"));
            }
        }
        if !stdout.trim().is_empty() {
            msg.push_str("  stdout:\n");
            for line in stdout.trim().lines() {
                msg.push_str(&format!("    {line}\n"));
            }
        }
        return Err(PackError::JlinkFailed(msg));
    }

    Ok(runtime_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compress_flag_prefers_zip_when_supported() {
        // Modern jlink (JDK 21 GraalVM, 22+) advertises the zip-N spelling.
        let help = "\
      --compress <compress>  Compression to use in compressing resources:
                             Accepted values are:
                             zip-[0-9], where zip-0 provides no compression,
                             and zip-9 provides the best compression.";
        assert_eq!(compress_flag_from_help(help), "--compress=zip-0");
    }

    #[test]
    fn compress_flag_falls_back_to_numeric() {
        // Older jlink (and some JDK 21 builds like Temurin) only take 0/1/2.
        let help = "\
      --compress=<0|1|2>  Enable compression of resources:
                          Level 0: No compression
                          Level 1: Constant string sharing
                          Level 2: ZIP";
        assert_eq!(compress_flag_from_help(help), "--compress=0");
    }
}
