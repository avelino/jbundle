use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::error::PackError;

pub struct ShrinkResult {
    pub jar_path: PathBuf,
    pub original_size: u64,
    pub shrunk_size: u64,
}

/// Repacks a JAR removing non-essential files and using maximum compression.
/// Safe for Clojure apps: keeps all .clj, .class, and resource files.
pub fn shrink_jar(jar_path: &Path) -> Result<ShrinkResult, PackError> {
    let original_size = std::fs::metadata(jar_path)
        .map_err(|e| PackError::ShrinkFailed(format!("cannot stat JAR: {e}")))?
        .len();

    let output_path = jar_path.with_extension("shrunk.jar");

    let src_file = std::fs::File::open(jar_path)
        .map_err(|e| PackError::ShrinkFailed(format!("cannot open JAR: {e}")))?;
    let mut archive = ZipArchive::new(src_file)
        .map_err(|e| PackError::ShrinkFailed(format!("cannot read JAR: {e}")))?;

    let out_file = std::fs::File::create(&output_path)
        .map_err(|e| PackError::ShrinkFailed(format!("cannot create output: {e}")))?;
    let mut writer = ZipWriter::new(out_file);

    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(Some(9));

    let mut buf = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive
            .by_index(i)
            .map_err(|e| PackError::ShrinkFailed(format!("zip entry error: {e}")))?;

        let name = entry.name().to_string();

        if should_skip(&name) {
            continue;
        }

        if entry.is_dir() {
            writer
                .add_directory(&name, SimpleFileOptions::default())
                .map_err(|e| PackError::ShrinkFailed(format!("zip write error: {e}")))?;
        } else {
            buf.clear();
            entry
                .read_to_end(&mut buf)
                .map_err(|e| PackError::ShrinkFailed(format!("zip read error: {e}")))?;

            writer
                .start_file(&name, options)
                .map_err(|e| PackError::ShrinkFailed(format!("zip write error: {e}")))?;
            writer
                .write_all(&buf)
                .map_err(|e| PackError::ShrinkFailed(format!("zip write error: {e}")))?;
        }
    }

    writer
        .finish()
        .map_err(|e| PackError::ShrinkFailed(format!("zip finalize error: {e}")))?;

    let shrunk_size = std::fs::metadata(&output_path)
        .map_err(|e| PackError::ShrinkFailed(format!("cannot stat output: {e}")))?
        .len();

    Ok(ShrinkResult {
        jar_path: output_path,
        original_size,
        shrunk_size,
    })
}

fn should_skip(name: &str) -> bool {
    // Maven build metadata
    if name.starts_with("META-INF/maven/") {
        return true;
    }

    // JAR signatures (invalid in uberjars anyway)
    if let Some(file_name) = name.strip_prefix("META-INF/") {
        if file_name.ends_with(".SF")
            || file_name.ends_with(".DSA")
            || file_name.ends_with(".RSA")
            || file_name.ends_with(".EC")
        {
            return true;
        }
    }

    // Java source files (not needed at runtime)
    if name.ends_with(".java") {
        return true;
    }

    // Build tool files
    if name == "META-INF/leiningen/" || name.starts_with("META-INF/leiningen/") {
        return true;
    }
    if name == "project.clj" {
        return true;
    }

    // Documentation files in META-INF
    if name.starts_with("META-INF/") {
        let lower = name.to_lowercase();
        if lower.ends_with(".md") || lower.ends_with(".txt") || lower.ends_with(".html") {
            // Keep LICENSE and NOTICE (legal compliance)
            let file_name = lower.rsplit('/').next().unwrap_or("");
            if !file_name.starts_with("license") && !file_name.starts_with("notice") {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as IoWrite;
    use tempfile::NamedTempFile;
    use zip::write::SimpleFileOptions as TestOptions;

    fn create_test_jar(entries: &[(&str, &[u8])]) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        let mut zip = ZipWriter::new(file.reopen().unwrap());
        let options = TestOptions::default();
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content).unwrap();
        }
        zip.finish().unwrap();
        file
    }

    #[test]
    fn skip_maven_metadata() {
        assert!(should_skip("META-INF/maven/org.clojure/clojure/pom.xml"));
        assert!(should_skip(
            "META-INF/maven/org.clojure/clojure/pom.properties"
        ));
    }

    #[test]
    fn skip_signatures() {
        assert!(should_skip("META-INF/CERT.SF"));
        assert!(should_skip("META-INF/CERT.DSA"));
        assert!(should_skip("META-INF/CERT.RSA"));
    }

    #[test]
    fn skip_java_sources() {
        assert!(should_skip("com/example/Main.java"));
        assert!(should_skip("org/apache/SomeClass.java"));
    }

    #[test]
    fn skip_leiningen_metadata() {
        assert!(should_skip("META-INF/leiningen/myapp/project.clj"));
        assert!(should_skip("project.clj"));
    }

    #[test]
    fn keep_class_files() {
        assert!(!should_skip("com/example/Main.class"));
        assert!(!should_skip("clojure/core__init.class"));
    }

    #[test]
    fn keep_clj_sources() {
        assert!(!should_skip("clojure/core.clj"));
        assert!(!should_skip("myapp/core.clj"));
    }

    #[test]
    fn keep_resources() {
        assert!(!should_skip("config.edn"));
        assert!(!should_skip("logback.xml"));
        assert!(!should_skip("META-INF/MANIFEST.MF"));
    }

    #[test]
    fn keep_license_files() {
        assert!(!should_skip("META-INF/LICENSE.txt"));
        assert!(!should_skip("META-INF/NOTICE.txt"));
    }

    #[test]
    fn skip_meta_inf_docs() {
        assert!(should_skip("META-INF/README.md"));
        assert!(should_skip("META-INF/CHANGELOG.md"));
    }

    #[test]
    fn shrink_removes_skippable_entries() {
        let jar = create_test_jar(&[
            ("META-INF/MANIFEST.MF", b"Manifest-Version: 1.0\n"),
            ("com/example/Main.class", b"fake class bytes"),
            ("myapp/core.clj", b"(ns myapp.core)"),
            ("META-INF/maven/com/pom.xml", b"<project/>"),
            ("META-INF/CERT.SF", b"signature"),
            ("com/example/Main.java", b"class Main {}"),
        ]);

        let result = shrink_jar(jar.path()).unwrap();

        // Verify the output JAR doesn't contain skipped entries
        let out_file = std::fs::File::open(&result.jar_path).unwrap();
        let out_archive = ZipArchive::new(out_file).unwrap();
        let names: Vec<String> = (0..out_archive.len())
            .map(|i| out_archive.name_for_index(i).unwrap().to_string())
            .collect();

        assert!(names.contains(&"META-INF/MANIFEST.MF".to_string()));
        assert!(names.contains(&"com/example/Main.class".to_string()));
        assert!(names.contains(&"myapp/core.clj".to_string()));
        assert!(!names.contains(&"META-INF/maven/com/pom.xml".to_string()));
        assert!(!names.contains(&"META-INF/CERT.SF".to_string()));
        assert!(!names.contains(&"com/example/Main.java".to_string()));

        // Clean up
        let _ = std::fs::remove_file(&result.jar_path);
    }
}
