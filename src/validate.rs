use std::io::Read;
use std::path::Path;

use indicatif::MultiProgress;

use crate::error::PackError;

struct ClassVersionInfo {
    major_version: u16,
    java_version: u8,
    class_file: String,
}

pub fn resolve_java_version(
    jar_path: &Path,
    configured: u8,
    explicit: bool,
    mp: &MultiProgress,
) -> Result<u8, PackError> {
    let info = detect_max_class_version(jar_path)?;

    let Some(info) = info else {
        return Ok(configured);
    };

    if info.java_version <= configured {
        return Ok(configured);
    }

    if explicit {
        return Err(PackError::JavaVersionMismatch {
            required: info.java_version,
            configured,
            class_version: info.major_version,
            class_file: info.class_file,
        });
    }

    let sp = mp.add(indicatif::ProgressBar::new_spinner());
    sp.set_style(
        indicatif::ProgressStyle::default_spinner()
            .template("  {msg}")
            .expect("invalid spinner template"),
    );
    sp.finish_with_message(format!(
        "\x1b[33mℹ\x1b[0m Auto-detected Java {} (class version {} in {})",
        info.java_version, info.major_version, info.class_file
    ));

    Ok(info.java_version)
}

fn detect_max_class_version(jar_path: &Path) -> Result<Option<ClassVersionInfo>, PackError> {
    let file = std::fs::File::open(jar_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    let mut max: Option<ClassVersionInfo> = None;
    let mut checked = 0u32;
    let limit = 200;

    for i in 0..archive.len() {
        if checked >= limit {
            break;
        }

        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        if !name.ends_with(".class") {
            continue;
        }
        if name.starts_with("META-INF/versions/") {
            continue;
        }

        checked += 1;

        let mut buf = [0u8; 8];
        if entry.read_exact(&mut buf).is_err() {
            continue;
        }

        let Some(major) = read_class_major_version(&buf) else {
            continue;
        };

        let java_ver = major.saturating_sub(44) as u8;

        let dominated = max.as_ref().is_some_and(|m| m.major_version >= major);
        if !dominated {
            max = Some(ClassVersionInfo {
                major_version: major,
                java_version: java_ver,
                class_file: name,
            });
        }
    }

    Ok(max)
}

fn read_class_major_version(data: &[u8]) -> Option<u16> {
    if data.len() < 8 {
        return None;
    }
    // magic: 0xCAFEBABE
    if data[0..4] != [0xCA, 0xFE, 0xBA, 0xBE] {
        return None;
    }
    // bytes 6-7: major version (big-endian)
    let major = u16::from_be_bytes([data[6], data[7]]);
    Some(major)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn make_class_bytes(major: u16) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]); // magic
        buf.extend_from_slice(&[0x00, 0x00]); // minor version
        buf.extend_from_slice(&major.to_be_bytes()); // major version
        buf
    }

    fn create_test_jar(classes: &[(&str, u16)]) -> tempfile::NamedTempFile {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::File::create(tmp.path()).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        for (name, major) in classes {
            zip.start_file(name.to_string(), options).unwrap();
            zip.write_all(&make_class_bytes(*major)).unwrap();
        }

        zip.finish().unwrap();
        tmp
    }

    #[test]
    fn compatible_version_returns_configured() {
        // JAR compiled with Java 17 (major 61), configured Java 21
        let jar = create_test_jar(&[("com/example/Main.class", 61)]);
        let mp = MultiProgress::new();
        let result = resolve_java_version(jar.path(), 21, false, &mp).unwrap();
        assert_eq!(result, 21);
    }

    #[test]
    fn explicit_incompatible_errors() {
        // JAR compiled with Java 21 (major 65), configured Java 17, explicit
        let jar = create_test_jar(&[("com/example/Main.class", 65)]);
        let mp = MultiProgress::new();
        let result = resolve_java_version(jar.path(), 17, true, &mp);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            PackError::JavaVersionMismatch {
                required,
                configured,
                class_version,
                ..
            } => {
                assert_eq!(required, 21);
                assert_eq!(configured, 17);
                assert_eq!(class_version, 65);
            }
            other => panic!("expected JavaVersionMismatch, got: {other}"),
        }
    }

    #[test]
    fn implicit_incompatible_upgrades() {
        // JAR compiled with Java 21 (major 65), configured Java 17, implicit
        let jar = create_test_jar(&[("com/example/Main.class", 65)]);
        let mp = MultiProgress::new();
        let result = resolve_java_version(jar.path(), 17, false, &mp).unwrap();
        assert_eq!(result, 21);
    }

    #[test]
    fn jar_without_classes_returns_configured() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::File::create(tmp.path()).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();
        zip.start_file("META-INF/MANIFEST.MF", options).unwrap();
        zip.write_all(b"Manifest-Version: 1.0\n").unwrap();
        zip.finish().unwrap();

        let mp = MultiProgress::new();
        let result = resolve_java_version(tmp.path(), 21, false, &mp).unwrap();
        assert_eq!(result, 21);
    }

    #[test]
    fn finds_highest_among_multiple() {
        let jar = create_test_jar(&[
            ("com/example/A.class", 55), // Java 11
            ("com/example/B.class", 65), // Java 21
            ("com/example/C.class", 61), // Java 17
        ]);
        let mp = MultiProgress::new();
        let result = resolve_java_version(jar.path(), 11, false, &mp).unwrap();
        assert_eq!(result, 21);
    }

    #[test]
    fn skips_multi_release_entries() {
        let jar = create_test_jar(&[
            ("com/example/Main.class", 55),             // Java 11
            ("META-INF/versions/21/com/Foo.class", 65), // multi-release, should be skipped
        ]);
        let mp = MultiProgress::new();
        let result = resolve_java_version(jar.path(), 11, false, &mp).unwrap();
        assert_eq!(result, 11);
    }

    #[test]
    fn skips_invalid_magic() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::File::create(tmp.path()).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        zip.start_file("Bad.class", options).unwrap();
        zip.write_all(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 65])
            .unwrap();

        zip.start_file("Good.class", options).unwrap();
        zip.write_all(&make_class_bytes(55)).unwrap();

        zip.finish().unwrap();

        let mp = MultiProgress::new();
        let result = resolve_java_version(tmp.path(), 11, false, &mp).unwrap();
        assert_eq!(result, 11);
    }

    #[test]
    fn handles_short_data() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        let file = std::fs::File::create(tmp.path()).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        // .class file with only 4 bytes (too short)
        zip.start_file("Short.class", options).unwrap();
        zip.write_all(&[0xCA, 0xFE, 0xBA, 0xBE]).unwrap();

        zip.finish().unwrap();

        let mp = MultiProgress::new();
        let result = resolve_java_version(tmp.path(), 21, false, &mp).unwrap();
        assert_eq!(result, 21);
    }
}
