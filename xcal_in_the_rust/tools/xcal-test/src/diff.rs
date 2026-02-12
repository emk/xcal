use std::fmt::Write;

use owo_colors::OwoColorize;
use similar::{ChangeTag, TextDiff};

/// Result of comparing a single connection's transcript.
pub struct ConnectionDiff {
    pub matches: bool,
    pub diff_output: String,
}

/// Compare expected vs actual transcript for a connection.
///
/// Returns a `ConnectionDiff` with colored unified diff output if they differ.
pub fn diff_transcript(
    conn: &str,
    expected: &str,
    actual: &str,
) -> ConnectionDiff {
    if expected == actual {
        return ConnectionDiff {
            matches: true,
            diff_output: String::new(),
        };
    }

    let diff = TextDiff::from_lines(expected, actual);
    let mut output = String::new();

    // Header
    writeln!(output, "{}", format!("--- {conn}.txt (expected)").bold())
        .unwrap();
    writeln!(output, "{}", format!("+++ {conn}.txt (actual)").bold()).unwrap();

    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        // Hunk header in cyan (print only the @@ header, not the full hunk)
        writeln!(output, "{}", hunk.header().cyan()).unwrap();
        for change in hunk.iter_changes() {
            match change.tag() {
                ChangeTag::Delete => {
                    write!(output, "{}", format!("-{change}").red()).unwrap();
                }
                ChangeTag::Insert => {
                    write!(output, "{}", format!("+{change}").green()).unwrap();
                }
                ChangeTag::Equal => {
                    write!(output, " {change}").unwrap();
                }
            };
            if change.missing_newline() {
                writeln!(output).unwrap();
            }
        }
    }

    ConnectionDiff {
        matches: false,
        diff_output: output,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_texts_match() {
        let result =
            diff_transcript("master", "hello\nworld\n", "hello\nworld\n");
        assert!(result.matches);
        assert!(result.diff_output.is_empty());
    }

    #[test]
    fn different_texts_produce_diff() {
        let result = diff_transcript("master", "hello\n", "goodbye\n");
        assert!(!result.matches);
        assert!(result.diff_output.contains("-hello"));
        assert!(result.diff_output.contains("+goodbye"));
    }
}
