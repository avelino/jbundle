mod context;
mod parser;
mod render;

use std::path::{Path, PathBuf};

use crate::config::BuildSystem;

/// Common source directories to search when resolving file paths.
/// Clojure reports paths relative to classpath (e.g., `example/core.clj`),
/// so we try prefixing with common source roots.
const SOURCE_PREFIXES: &[&str] = &[
    "",
    "src",
    "src/main/java",
    "src/main/kotlin",
    "src/main/clj",
];

/// Formats build error output with source context and diagnostics.
///
/// Parses stderr/stdout for known error patterns, reads the relevant source file,
/// and produces a rustc-style diagnostic. Falls back to raw stderr if no patterns match.
pub fn format_build_error(
    stderr: &str,
    stdout: &str,
    system: BuildSystem,
    project_dir: &Path,
) -> String {
    let diagnostics = parser::parse(stderr, stdout, system);

    if diagnostics.is_empty() {
        return stderr.to_string();
    }

    let mut output = String::new();

    for diag in &diagnostics {
        let mut diag = diag.clone();

        // Resolve the source file path and read context
        let resolved = diag
            .file
            .as_ref()
            .map(|f| resolve_source_file(project_dir, f));
        let source_ctx = match (&resolved, diag.line) {
            (Some(path), Some(line)) => {
                if let Ok(rel) = path.strip_prefix(project_dir) {
                    diag.file = Some(rel.to_path_buf());
                }
                context::read_context(path, line, 2)
            }
            _ => None,
        };

        output.push_str(&render::render(&diag, source_ctx.as_ref()));
        output.push('\n');
    }

    output
}

/// Resolves a file path reported by the compiler to an actual file on disk.
/// Tries the path directly, then with common source prefixes.
fn resolve_source_file(project_dir: &Path, file: &Path) -> PathBuf {
    // If it's already absolute, use it directly
    if file.is_absolute() {
        return file.to_path_buf();
    }

    // Try direct join first
    let direct = project_dir.join(file);
    if direct.exists() {
        return direct;
    }

    // Try with common source prefixes
    for prefix in SOURCE_PREFIXES {
        if prefix.is_empty() {
            continue;
        }
        let candidate = project_dir.join(prefix).join(file);
        if candidate.exists() {
            return candidate;
        }
    }

    // Fallback: return direct path (context reader will handle missing file)
    direct
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn format_build_error_with_parseable_error() {
        let dir = tempdir().unwrap();
        let src_dir = dir.path().join("src/example");
        std::fs::create_dir_all(&src_dir).unwrap();

        let source = "(ns example.core)\n\n\
                      (defn greet [name]\n\
                        (str \"Hello, \" name))\n\n\
                      (defn process []\n\
                        (prntln \"hello\"))\n";
        let mut f = std::fs::File::create(src_dir.join("core.clj")).unwrap();
        f.write_all(source.as_bytes()).unwrap();

        let stderr = "Syntax error compiling at (src/example/core.clj:7:3).\n\
                      Unable to resolve symbol: prntln in this context";

        let result = format_build_error(stderr, "", BuildSystem::DepsEdn, dir.path());
        assert!(result.contains("error"));
        assert!(result.contains("prntln"));
        assert!(result.contains("src/example/core.clj:7:3"));
    }

    #[test]
    fn format_build_error_fallback_on_unknown_format() {
        let dir = tempdir().unwrap();
        let stderr = "Some unknown error format\nwith multiple lines";

        let result = format_build_error(stderr, "", BuildSystem::DepsEdn, dir.path());
        assert_eq!(result, stderr);
    }

    #[test]
    fn format_build_error_missing_source_file() {
        let dir = tempdir().unwrap();
        let stderr = "Syntax error compiling at (src/missing.clj:5:1).\n\
                      Unable to resolve symbol: xyz in this context";

        let result = format_build_error(stderr, "", BuildSystem::DepsEdn, dir.path());
        // Should still render the diagnostic, just without source context
        assert!(result.contains("error"));
        assert!(result.contains("xyz"));
    }
}
