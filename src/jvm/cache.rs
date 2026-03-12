use std::path::{Path, PathBuf};

use crate::config::{BuildConfig, Target};
use crate::error::PackError;

pub fn cached_jdk_path(version: u8, target: &Target) -> Result<PathBuf, PackError> {
    let dir_name = format!(
        "jdk-{}-{}-{}",
        version,
        target.adoptium_os(),
        target.adoptium_arch()
    );
    Ok(BuildConfig::cache_dir()?.join(dir_name))
}

pub fn extract_and_cache(
    version: u8,
    target: &Target,
    archive: &Path,
) -> Result<PathBuf, PackError> {
    let dest = cached_jdk_path(version, target)?;
    if dest.exists() {
        std::fs::remove_dir_all(&dest)?;
    }
    std::fs::create_dir_all(&dest)?;

    let file_name = archive
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| PackError::JdkDownload("invalid archive path".into()))?;

    if file_name.ends_with(".tar.gz") || file_name.ends_with(".tgz") {
        extract_tar_gz(archive, &dest)?;
    } else if file_name.ends_with(".zip") {
        extract_zip(archive, &dest)?;
    } else {
        return Err(PackError::JdkDownload(format!(
            "unknown archive format: {file_name}"
        )));
    }

    // Adoptium archives have a top-level directory, flatten it
    flatten_single_subdir(&dest)?;

    Ok(dest)
}

fn extract_tar_gz(archive: &Path, dest: &Path) -> Result<(), PackError> {
    let file = std::fs::File::open(archive)?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(decoder);
    tar.unpack(dest)?;
    Ok(())
}

fn extract_zip(archive: &Path, dest: &Path) -> Result<(), PackError> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    zip.extract(dest)?;
    Ok(())
}

fn flatten_single_subdir(dir: &Path) -> Result<(), PackError> {
    let entries: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();

    if entries.len() == 1 && entries[0].path().is_dir() {
        let subdir = entries[0].path();
        let temp = dir.join("__flatten_temp__");
        std::fs::rename(&subdir, &temp)?;

        for entry in std::fs::read_dir(&temp)? {
            let entry = entry?;
            std::fs::rename(entry.path(), dir.join(entry.file_name()))?;
        }
        std::fs::remove_dir(&temp)?;
    }

    Ok(())
}

pub fn jdk_bin(jdk_path: &Path, tool: &str) -> PathBuf {
    let tool_name = if cfg!(target_os = "windows") {
        format!("{tool}.exe")
    } else {
        tool.to_string()
    };

    // macOS JDK has Contents/Home structure
    let macos_bin = jdk_path
        .join("Contents")
        .join("Home")
        .join("bin")
        .join(&tool_name);
    if macos_bin.exists() {
        return macos_bin;
    }
    jdk_path.join("bin").join(&tool_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{TargetArch, TargetOs};
    use tempfile::tempdir;

    #[test]
    fn cached_jdk_path_contains_version_and_target() {
        let target = Target {
            os: TargetOs::Linux,
            arch: TargetArch::X86_64,
        };
        let path = cached_jdk_path(21, &target).unwrap();
        let name = path.file_name().unwrap().to_str().unwrap();
        assert_eq!(name, "jdk-21-linux-x64");
    }

    #[test]
    fn jdk_bin_returns_linux_path_by_default() {
        let dir = tempdir().unwrap();
        let path = jdk_bin(dir.path(), "java");
        assert_eq!(path, dir.path().join("bin").join("java"));
    }

    #[test]
    fn jdk_bin_returns_macos_path_when_exists() {
        let dir = tempdir().unwrap();
        let macos_bin = dir.path().join("Contents").join("Home").join("bin");
        std::fs::create_dir_all(&macos_bin).unwrap();
        std::fs::write(macos_bin.join("java"), b"fake").unwrap();

        let path = jdk_bin(dir.path(), "java");
        assert_eq!(path, macos_bin.join("java"));
    }

    #[test]
    fn flatten_single_subdir_flattens() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("jdk-21.0.1");
        std::fs::create_dir_all(subdir.join("bin")).unwrap();
        std::fs::write(subdir.join("bin").join("java"), b"java").unwrap();
        std::fs::write(subdir.join("release"), b"release").unwrap();

        flatten_single_subdir(dir.path()).unwrap();

        assert!(dir.path().join("bin").join("java").exists());
        assert!(dir.path().join("release").exists());
        assert!(!dir.path().join("jdk-21.0.1").exists());
    }

    #[test]
    fn flatten_single_subdir_noop_for_multiple_entries() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("a")).unwrap();
        std::fs::create_dir(dir.path().join("b")).unwrap();

        flatten_single_subdir(dir.path()).unwrap();

        assert!(dir.path().join("a").exists());
        assert!(dir.path().join("b").exists());
    }

    #[test]
    fn flatten_single_subdir_noop_for_file() {
        let dir = tempdir().unwrap();
        std::fs::write(dir.path().join("file.txt"), b"data").unwrap();

        flatten_single_subdir(dir.path()).unwrap();

        assert!(dir.path().join("file.txt").exists());
    }
}
