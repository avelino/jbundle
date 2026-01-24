use std::path::{Path, PathBuf};

use flate2::write::GzEncoder;
use flate2::Compression;
use sha2::{Digest, Sha256};

use crate::error::PackError;

pub fn create_runtime_archive(runtime_dir: &Path, work_dir: &Path) -> Result<PathBuf, PackError> {
    let archive_path = work_dir.join("runtime.tar.gz");
    let file = std::fs::File::create(&archive_path)?;
    let encoder = GzEncoder::new(file, Compression::default());
    let mut tar = tar::Builder::new(encoder);

    tracing::info!("creating runtime archive");

    tar.append_dir_all(".", runtime_dir)?;

    let encoder = tar.into_inner()?;
    encoder.finish()?;

    Ok(archive_path)
}

pub fn hash_file(path: &Path) -> Result<String, PackError> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut reader = std::io::BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = format!("{:x}", hasher.finalize());
    Ok(hash[..16].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn hash_file_is_deterministic() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.bin");
        std::fs::write(&file, b"hello world").unwrap();

        let h1 = hash_file(&file).unwrap();
        let h2 = hash_file(&file).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn hash_file_is_16_chars() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("test.bin");
        std::fs::write(&file, b"content").unwrap();

        let hash = hash_file(&file).unwrap();
        assert_eq!(hash.len(), 16);
    }

    #[test]
    fn hash_file_different_content_different_hash() {
        let dir = tempdir().unwrap();
        let f1 = dir.path().join("a.bin");
        let f2 = dir.path().join("b.bin");
        std::fs::write(&f1, b"content A").unwrap();
        std::fs::write(&f2, b"content B").unwrap();

        let h1 = hash_file(&f1).unwrap();
        let h2 = hash_file(&f2).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn create_runtime_archive_produces_tar_gz() {
        let dir = tempdir().unwrap();
        let runtime = dir.path().join("runtime");
        std::fs::create_dir_all(runtime.join("bin")).unwrap();
        std::fs::write(runtime.join("bin").join("java"), b"fake java").unwrap();

        let work = tempdir().unwrap();
        let archive = create_runtime_archive(&runtime, work.path()).unwrap();

        assert!(archive.exists());
        assert!(std::fs::metadata(&archive).unwrap().len() > 0);
    }

    #[test]
    fn create_runtime_archive_contains_bin() {
        let dir = tempdir().unwrap();
        let runtime = dir.path().join("runtime");
        std::fs::create_dir_all(runtime.join("bin")).unwrap();
        std::fs::write(runtime.join("bin").join("java"), b"fake").unwrap();

        let work = tempdir().unwrap();
        let archive = create_runtime_archive(&runtime, work.path()).unwrap();

        let file = std::fs::File::open(&archive).unwrap();
        let decoder = flate2::read::GzDecoder::new(file);
        let mut tar_archive = tar::Archive::new(decoder);

        let entries: Vec<String> = tar_archive
            .entries()
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(entries.iter().any(|e| e.contains("bin/java")));
    }
}
