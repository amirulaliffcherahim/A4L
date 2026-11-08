use sha2::{Digest, Sha256};

/// Normalizes compiler/test error stderr:
/// - Strips file paths (e.g., C:\... or /home/... or ./src/...)
/// - Strips line and column numbers (e.g. :12:34 or (line 12, col 5))
/// - Strips memory addresses (0x[0-9a-fA-F]+)
/// - Normalizes whitespace and trims
pub fn normalize_error(stderr: &str) -> String {
    let mut normalized = String::with_capacity(stderr.len());

    for line in stderr.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // If line is just "--> path:line:col", replace path/line with placeholder
        if trimmed.starts_with("-->") {
            normalized.push_str("--> <LOC>\n");
            continue;
        }

        // If line starts with line number and pipe, e.g. "42 | ..." or "42 |"
        let line_content = if let Some(idx) = trimmed.find('|') {
            let prefix = trimmed[..idx].trim();
            if prefix.chars().all(|c| c.is_ascii_digit()) {
                trimmed[idx..].trim()
            } else {
                trimmed
            }
        } else {
            trimmed
        };

        // Tokenize line and mask out file paths, line/col numbers, memory addresses, pure digits
        let mut line_buf = String::new();
        for word in line_content.split_whitespace() {
            if word.starts_with("0x") && word.len() > 6 {
                line_buf.push_str("<ADDR> ");
            } else if is_path_or_loc(word) {
                line_buf.push_str("<LOC> ");
            } else if word.chars().all(|c| c.is_ascii_digit()) {
                line_buf.push_str("<NUM> ");
            } else {
                line_buf.push_str(word);
                line_buf.push(' ');
            }
        }
        normalized.push_str(line_buf.trim_end());
        normalized.push('\n');
    }

    normalized.trim().to_string()
}

fn is_path_or_loc(word: &str) -> bool {
    // Check if word contains path separator or line indicator
    if word.contains('/')
        || word.contains('\\')
        || word.contains(".rs:")
        || word.contains(".ts:")
        || word.contains(".py:")
    {
        return true;
    }
    // Check if :digits:digits
    let parts: Vec<&str> = word.split(':').collect();
    if parts.len() >= 2
        && parts
            .iter()
            .skip(1)
            .all(|p| p.chars().all(|c| c.is_ascii_digit()))
    {
        return true;
    }
    false
}

pub fn compute_error_hash(compilation_unit: &str, stderr: &str) -> String {
    let norm = normalize_error(stderr);
    let mut hasher = Sha256::new();
    hasher.update(compilation_unit.as_bytes());
    hasher.update(b"::");
    hasher.update(norm.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_normalization_dedupes_line_numbers() {
        let err1 = "error[E0425]: cannot find value `foo` in this scope\n  --> src/main.rs:42:15\n   |\n42 |     let x = foo;\n   |             ^^^ not found";
        let err2 = "error[E0425]: cannot find value `foo` in this scope\n  --> src/main.rs:99:20\n   |\n99 |     let x = foo;\n   |             ^^^ not found";

        let hash1 = compute_error_hash("a4l-core", err1);
        let hash2 = compute_error_hash("a4l-core", err2);
        assert_eq!(
            hash1, hash2,
            "different line numbers should produce identical error hash"
        );
    }
}
