use std::path::Path;

#[derive(Debug)]
pub struct SourceContext {
    pub lines: Vec<(usize, String)>,
    pub error_line_index: usize,
}

/// Reads source file and returns surrounding lines for context.
/// Returns None if file cannot be read or line number is invalid.
pub fn read_context(file: &Path, error_line: usize, context_lines: usize) -> Option<SourceContext> {
    let content = std::fs::read_to_string(file).ok()?;
    let all_lines: Vec<&str> = content.lines().collect();

    if error_line == 0 || error_line > all_lines.len() {
        return None;
    }

    let error_idx = error_line - 1; // 0-based index
    let start = error_idx.saturating_sub(context_lines);
    let end = (error_idx + context_lines + 1).min(all_lines.len());

    let lines: Vec<(usize, String)> = (start..end)
        .map(|i| (i + 1, all_lines[i].to_string())) // 1-based line numbers
        .collect();

    let error_line_index = error_idx - start;

    Some(SourceContext {
        lines,
        error_line_index,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_file(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f.flush().unwrap();
        f
    }

    #[test]
    fn read_context_middle_of_file() {
        let f = create_temp_file("line1\nline2\nline3\nline4\nline5\nline6\nline7\n");
        let ctx = read_context(f.path(), 4, 2).unwrap();

        assert_eq!(ctx.lines.len(), 5); // lines 2-6
        assert_eq!(ctx.lines[0], (2, "line2".to_string()));
        assert_eq!(ctx.lines[4], (6, "line6".to_string()));
        assert_eq!(ctx.error_line_index, 2); // line4 is at index 2
    }

    #[test]
    fn read_context_beginning_of_file() {
        let f = create_temp_file("line1\nline2\nline3\nline4\nline5\n");
        let ctx = read_context(f.path(), 1, 2).unwrap();

        assert_eq!(ctx.lines.len(), 3); // lines 1-3
        assert_eq!(ctx.lines[0], (1, "line1".to_string()));
        assert_eq!(ctx.error_line_index, 0);
    }

    #[test]
    fn read_context_end_of_file() {
        let f = create_temp_file("line1\nline2\nline3\nline4\nline5\n");
        let ctx = read_context(f.path(), 5, 2).unwrap();

        assert_eq!(ctx.lines.len(), 3); // lines 3-5
        assert_eq!(ctx.lines[2], (5, "line5".to_string()));
        assert_eq!(ctx.error_line_index, 2);
    }

    #[test]
    fn read_context_invalid_line_zero() {
        let f = create_temp_file("line1\nline2\n");
        assert!(read_context(f.path(), 0, 2).is_none());
    }

    #[test]
    fn read_context_line_beyond_file() {
        let f = create_temp_file("line1\nline2\n");
        assert!(read_context(f.path(), 100, 2).is_none());
    }

    #[test]
    fn read_context_nonexistent_file() {
        assert!(read_context(Path::new("/nonexistent/file.clj"), 5, 2).is_none());
    }

    #[test]
    fn read_context_single_line_file() {
        let f = create_temp_file("only line");
        let ctx = read_context(f.path(), 1, 2).unwrap();

        assert_eq!(ctx.lines.len(), 1);
        assert_eq!(ctx.lines[0], (1, "only line".to_string()));
        assert_eq!(ctx.error_line_index, 0);
    }
}
