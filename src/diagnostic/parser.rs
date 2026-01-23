use std::path::PathBuf;

use regex::Regex;

use crate::config::BuildSystem;

#[derive(Debug, Clone, PartialEq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub file: Option<PathBuf>,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

pub fn parse(stderr: &str, stdout: &str, system: BuildSystem) -> Vec<Diagnostic> {
    match system {
        BuildSystem::DepsEdn | BuildSystem::Leiningen => parse_clojure(stderr, stdout),
        BuildSystem::Maven => parse_maven(stderr, stdout),
        BuildSystem::Gradle => parse_gradle(stderr, stdout),
    }
}

fn parse_clojure(stderr: &str, stdout: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let combined = format!("{stderr}\n{stdout}");
    let lines: Vec<&str> = combined.lines().collect();

    // Pattern: "Syntax error compiling at (file:line:col)"
    let syntax_re =
        Regex::new(r"(?i)(?:Syntax error|Compiler Exception).* at \(([^:]+):(\d+):(\d+)\)")
            .unwrap();

    // Pattern: "Unable to resolve symbol: xyz in this context"
    let symbol_re = Regex::new(r"Unable to resolve symbol:\s*(\S+)").unwrap();

    // Pattern: clojure.lang.Compiler$CompilerException with location
    let compiler_ex_re =
        Regex::new(r"CompilerException.*?(?:compiling|at)\s*\(([^:]+):(\d+):(\d+)\)").unwrap();

    for (i, line) in lines.iter().enumerate() {
        if let Some(caps) = syntax_re.captures(line) {
            let file = caps.get(1).map(|m| PathBuf::from(m.as_str()));
            let line_num = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col = caps.get(3).and_then(|m| m.as_str().parse().ok());

            // Look at subsequent lines for the actual error message
            let message = find_clojure_message(&lines, i);

            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message,
                file,
                line: line_num,
                column: col,
            });
        } else if let Some(caps) = compiler_ex_re.captures(line) {
            let file = caps.get(1).map(|m| PathBuf::from(m.as_str()));
            let line_num = caps.get(2).and_then(|m| m.as_str().parse().ok());
            let col = caps.get(3).and_then(|m| m.as_str().parse().ok());

            let message = find_clojure_message(&lines, i);

            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message,
                file,
                line: line_num,
                column: col,
            });
        } else if diagnostics.is_empty() {
            // Try to find standalone "Unable to resolve symbol" without location
            if let Some(caps) = symbol_re.captures(line) {
                let symbol = caps.get(1).unwrap().as_str();
                diagnostics.push(Diagnostic {
                    severity: Severity::Error,
                    message: format!("Unable to resolve symbol: {symbol}"),
                    file: None,
                    line: None,
                    column: None,
                });
            }
        }
    }

    diagnostics
}

fn find_clojure_message(lines: &[&str], error_line_idx: usize) -> String {
    let symbol_re = Regex::new(r"Unable to resolve symbol:\s*(\S+)").unwrap();
    let caused_by_re = Regex::new(r"Caused by:.*?:\s*(.+)").unwrap();

    // First pass: search nearby lines for "Unable to resolve symbol" (highest priority)
    for offset in 0..10 {
        let idx = error_line_idx + offset;
        if idx >= lines.len() {
            break;
        }
        if let Some(caps) = symbol_re.captures(lines[idx]) {
            return format!(
                "Unable to resolve symbol: {}",
                caps.get(1).unwrap().as_str()
            );
        }
    }

    // Also search before the error line for symbol errors
    for offset in 1..5 {
        if offset > error_line_idx {
            break;
        }
        let idx = error_line_idx - offset;
        if let Some(caps) = symbol_re.captures(lines[idx]) {
            return format!(
                "Unable to resolve symbol: {}",
                caps.get(1).unwrap().as_str()
            );
        }
    }

    // Second pass: look for "Caused by" messages (lower priority)
    for offset in 0..10 {
        let idx = error_line_idx + offset;
        if idx >= lines.len() {
            break;
        }
        if let Some(caps) = caused_by_re.captures(lines[idx]) {
            let msg = caps.get(1).unwrap().as_str().trim();
            // Skip generic "Syntax error compiling" messages from Caused by lines
            if !msg.starts_with("Syntax error") {
                return msg.to_string();
            }
        }
    }

    // Fallback: use the error line itself, cleaned up
    let line = lines[error_line_idx];
    if let Some(msg) = line.strip_prefix("Syntax error") {
        return msg.trim().to_string();
    }

    line.trim().to_string()
}

fn parse_maven(stderr: &str, stdout: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let combined = format!("{stderr}\n{stdout}");

    // Maven compiler error: [ERROR] /path/to/File.java:[line,col] message
    let error_re = Regex::new(r"\[ERROR\]\s+(/[^:]+):\[(\d+),(\d+)\]\s+(.*)").unwrap();
    // Maven warning: [WARNING] /path/to/File.java:[line,col] message
    let warn_re = Regex::new(r"\[WARNING\]\s+(/[^:]+):\[(\d+),(\d+)\]\s+(.*)").unwrap();

    for line in combined.lines() {
        if let Some(caps) = error_re.captures(line) {
            diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: caps.get(4).unwrap().as_str().trim().to_string(),
                file: Some(PathBuf::from(caps.get(1).unwrap().as_str())),
                line: caps.get(2).and_then(|m| m.as_str().parse().ok()),
                column: caps.get(3).and_then(|m| m.as_str().parse().ok()),
            });
        } else if let Some(caps) = warn_re.captures(line) {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                message: caps.get(4).unwrap().as_str().trim().to_string(),
                file: Some(PathBuf::from(caps.get(1).unwrap().as_str())),
                line: caps.get(2).and_then(|m| m.as_str().parse().ok()),
                column: caps.get(3).and_then(|m| m.as_str().parse().ok()),
            });
        }
    }

    diagnostics
}

fn parse_gradle(stderr: &str, stdout: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let combined = format!("{stderr}\n{stdout}");

    // javac: File.java:line: error|warning: message
    let javac_re = Regex::new(r"([^\s:]+\.java):(\d+):\s*(error|warning):\s+(.+)").unwrap();
    // kotlinc: e: file:///path:line:col message  or  w: file:///path:line:col message
    let kotlin_re = Regex::new(r"([ew]):\s*file://(/[^:]+):(\d+):(\d+)\s+(.+)").unwrap();

    for line in combined.lines() {
        if let Some(caps) = javac_re.captures(line) {
            let severity = match caps.get(3).unwrap().as_str() {
                "error" => Severity::Error,
                _ => Severity::Warning,
            };
            diagnostics.push(Diagnostic {
                severity,
                message: caps.get(4).unwrap().as_str().trim().to_string(),
                file: Some(PathBuf::from(caps.get(1).unwrap().as_str())),
                line: caps.get(2).and_then(|m| m.as_str().parse().ok()),
                column: None,
            });
        } else if let Some(caps) = kotlin_re.captures(line) {
            let severity = match caps.get(1).unwrap().as_str() {
                "e" => Severity::Error,
                _ => Severity::Warning,
            };
            diagnostics.push(Diagnostic {
                severity,
                message: caps.get(5).unwrap().as_str().trim().to_string(),
                file: Some(PathBuf::from(caps.get(2).unwrap().as_str())),
                line: caps.get(3).and_then(|m| m.as_str().parse().ok()),
                column: caps.get(4).and_then(|m| m.as_str().parse().ok()),
            });
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clojure_syntax_error() {
        let stderr = r#"Syntax error compiling at (src/example/core.clj:9:5).
Unable to resolve symbol: prntln in this context"#;

        let diags = parse(stderr, "", BuildSystem::DepsEdn);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(
            diags[0].file.as_ref().unwrap().to_str().unwrap(),
            "src/example/core.clj"
        );
        assert_eq!(diags[0].line, Some(9));
        assert_eq!(diags[0].column, Some(5));
        assert!(diags[0].message.contains("prntln"));
    }

    #[test]
    fn parse_clojure_compiler_exception() {
        let stderr = "Caused by: clojure.lang.Compiler$CompilerException: Syntax error compiling at (src/app.clj:15:3).\n\
                      Caused by: java.lang.RuntimeException: Unable to resolve symbol: foo in this context";

        let diags = parse(stderr, "", BuildSystem::Leiningen);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].line, Some(15));
        assert!(diags[0].message.contains("foo"));
    }

    #[test]
    fn parse_maven_error() {
        let stdout =
            "[ERROR] /home/user/project/src/main/java/App.java:[12,15] cannot find symbol\n\
                      [WARNING] /home/user/project/src/main/java/App.java:[5,1] unchecked cast";

        let diags = parse("", stdout, BuildSystem::Maven);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].line, Some(12));
        assert_eq!(diags[0].column, Some(15));
        assert!(diags[0].message.contains("cannot find symbol"));
        assert_eq!(diags[1].severity, Severity::Warning);
    }

    #[test]
    fn parse_gradle_javac_error() {
        let stderr = "src/main/java/App.java:10: error: cannot find symbol\n\
                      src/main/java/App.java:20: warning: deprecated API";

        let diags = parse(stderr, "", BuildSystem::Gradle);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].line, Some(10));
        assert_eq!(diags[1].severity, Severity::Warning);
        assert_eq!(diags[1].line, Some(20));
    }

    #[test]
    fn parse_gradle_kotlin_error() {
        let stderr = "e: file:///home/user/project/src/main/kotlin/App.kt:5:10 Unresolved reference: foo\n\
                      w: file:///home/user/project/src/main/kotlin/App.kt:8:1 Variable is never used";

        let diags = parse(stderr, "", BuildSystem::Gradle);
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0].severity, Severity::Error);
        assert_eq!(diags[0].line, Some(5));
        assert_eq!(diags[0].column, Some(10));
        assert!(diags[0].message.contains("Unresolved reference"));
        assert_eq!(diags[1].severity, Severity::Warning);
    }

    #[test]
    fn parse_empty_stderr_returns_empty() {
        let diags = parse("", "", BuildSystem::DepsEdn);
        assert!(diags.is_empty());
    }

    #[test]
    fn parse_unrecognized_format_returns_empty() {
        let stderr = "Some random error that doesn't match any pattern";
        let diags = parse(stderr, "", BuildSystem::DepsEdn);
        assert!(diags.is_empty());
    }
}
