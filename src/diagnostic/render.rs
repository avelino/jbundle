use std::io::IsTerminal;

use super::context::SourceContext;
use super::parser::{Diagnostic, Severity};

/// Whether to use ANSI colors in output.
fn use_colors() -> bool {
    std::io::stderr().is_terminal()
}

fn red(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;31m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn yellow(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;33m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn blue(text: &str) -> String {
    if use_colors() {
        format!("\x1b[1;34m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Renders a single diagnostic with source context into a formatted string.
pub fn render(diagnostic: &Diagnostic, context: Option<&SourceContext>) -> String {
    let mut out = String::new();

    // Header: "error: message" or "warning: message"
    let severity_label = match diagnostic.severity {
        Severity::Error => red("error"),
        Severity::Warning => yellow("warning"),
    };
    out.push_str(&format!("{severity_label}: {}\n", diagnostic.message));

    // Location: " --> file:line:col"
    if let Some(file) = &diagnostic.file {
        let location = match (diagnostic.line, diagnostic.column) {
            (Some(l), Some(c)) => format!("{}:{l}:{c}", file.display()),
            (Some(l), None) => format!("{}:{l}", file.display()),
            _ => format!("{}", file.display()),
        };
        out.push_str(&format!(" {} {location}\n", blue("-->")));
    }

    // Source context with line numbers and caret
    if let Some(ctx) = context {
        let max_line_num = ctx.lines.last().map(|(n, _)| *n).unwrap_or(0);
        let gutter_width = max_line_num.to_string().len();

        // Opening gutter
        out.push_str(&format!("{} {}\n", " ".repeat(gutter_width), blue("|")));

        for (i, (line_num, content)) in ctx.lines.iter().enumerate() {
            let num_str = format!("{:>width$}", line_num, width = gutter_width);
            out.push_str(&format!("{} {} {content}\n", blue(&num_str), blue("|")));

            // Caret line under the error
            if i == ctx.error_line_index {
                if let Some(col) = diagnostic.column {
                    let col_offset = col.saturating_sub(1);
                    let caret_padding = " ".repeat(col_offset);
                    // Try to determine the length of the problematic token
                    let token_len = extract_token_length(content, col_offset);
                    let carets = "^".repeat(token_len.max(1));
                    let annotation = short_annotation(&diagnostic.message);

                    let caret_line = match diagnostic.severity {
                        Severity::Error => red(&format!("{caret_padding}{carets} {annotation}")),
                        Severity::Warning => {
                            yellow(&format!("{caret_padding}{carets} {annotation}"))
                        }
                    };
                    out.push_str(&format!(
                        "{} {} {caret_line}\n",
                        " ".repeat(gutter_width),
                        blue("|")
                    ));
                }
            }
        }

        // Closing gutter
        out.push_str(&format!("{} {}\n", " ".repeat(gutter_width), blue("|")));
    }

    out
}

/// Extracts the length of the token at the given column offset.
/// Skips leading open-parens/brackets since Clojure errors often point to
/// the start of the s-expression rather than the symbol itself.
fn extract_token_length(line: &str, col_offset: usize) -> usize {
    let chars: Vec<char> = line.chars().collect();
    if col_offset >= chars.len() {
        return 1;
    }

    // Skip opening delimiters to find the actual token
    let mut start = col_offset;
    while start < chars.len() && (chars[start] == '(' || chars[start] == '[') {
        start += 1;
    }

    let mut len = 0;
    for &ch in &chars[start..] {
        if ch.is_whitespace() || ch == '(' || ch == ')' || ch == '[' || ch == ']' || ch == '"' {
            break;
        }
        len += 1;
    }

    // Total length includes skipped delimiters + token
    let total = (start - col_offset) + len;
    total.max(1)
}

/// Creates a short annotation from the error message.
fn short_annotation(message: &str) -> String {
    if message.contains("Unable to resolve symbol") {
        return "symbol not found".to_string();
    }
    if message.contains("cannot find symbol") {
        return "symbol not found".to_string();
    }
    if message.contains("Unresolved reference") {
        return "unresolved reference".to_string();
    }
    // Truncate long messages
    if message.len() > 40 {
        format!("{}...", &message[..37])
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn render_error_without_context() {
        let diag = Diagnostic {
            severity: Severity::Error,
            message: "Unable to resolve symbol: prntln".to_string(),
            file: Some(PathBuf::from("src/example/core.clj")),
            line: Some(9),
            column: Some(5),
        };

        let output = render(&diag, None);
        assert!(output.contains("error"));
        assert!(output.contains("Unable to resolve symbol: prntln"));
        assert!(output.contains("src/example/core.clj:9:5"));
    }

    #[test]
    fn render_error_with_context() {
        let diag = Diagnostic {
            severity: Severity::Error,
            message: "Unable to resolve symbol: prntln".to_string(),
            file: Some(PathBuf::from("src/example/core.clj")),
            line: Some(9),
            column: Some(5),
        };

        let ctx = SourceContext {
            lines: vec![
                (7, "(defn process-data [data]".to_string()),
                (8, "  (let [result (map inc data)]".to_string()),
                (9, "    (prntln \"Processing:\" result)".to_string()),
                (10, "    (reduce + result)))".to_string()),
            ],
            error_line_index: 2,
        };

        let output = render(&diag, Some(&ctx));
        assert!(output.contains("error"));
        assert!(output.contains("prntln"));
        assert!(output.contains("symbol not found"));
    }

    #[test]
    fn render_warning() {
        let diag = Diagnostic {
            severity: Severity::Warning,
            message: "unchecked cast".to_string(),
            file: Some(PathBuf::from("App.java")),
            line: Some(5),
            column: Some(1),
        };

        let output = render(&diag, None);
        assert!(output.contains("warning"));
        assert!(output.contains("unchecked cast"));
    }

    #[test]
    fn extract_token_length_simple() {
        assert_eq!(extract_token_length("    (prntln \"hello\")", 5), 6);
    }

    #[test]
    fn extract_token_length_with_paren_prefix() {
        // Column points to '(' before the symbol
        assert_eq!(extract_token_length("    (prntln \"hello\")", 4), 7);
    }

    #[test]
    fn extract_token_length_at_end() {
        assert_eq!(extract_token_length("foo", 0), 3);
    }

    #[test]
    fn extract_token_length_beyond_line() {
        assert_eq!(extract_token_length("hi", 10), 1);
    }

    #[test]
    fn short_annotation_symbol() {
        assert_eq!(
            short_annotation("Unable to resolve symbol: prntln"),
            "symbol not found"
        );
    }

    #[test]
    fn short_annotation_long_message() {
        let msg = "This is a very long error message that should be truncated for display";
        let result = short_annotation(msg);
        assert!(result.len() <= 43); // 37 + "..."
        assert!(result.ends_with("..."));
    }
}
