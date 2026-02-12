/// A pattern with `*` abbreviation points for matching user input.
///
/// Used for both help topic lookup and command dispatch. The `*` marks
/// positions where the user's input is allowed to stop — this is NOT
/// glob matching. Comparison is case-insensitive.
///
/// # Examples
///
/// ```ignore
/// use xcalr::matching::CommandPattern;
/// let p = CommandPattern::new("t*e*ll");
/// assert!(p.matches("t"));     // stops at first *
/// assert!(p.matches("te"));    // stops at second *
/// assert!(!p.matches("tel"));  // ok cleared by first l, second l remains
/// assert!(p.matches("tell"));  // both strings exhausted
/// ```
pub struct CommandPattern {
    pattern: String,
}

impl CommandPattern {
    /// Create a new pattern. The pattern string should contain `*` at
    /// abbreviation points (e.g., `"t*e*ll"`).
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            pattern: pattern.into(),
        }
    }

    /// Test whether `input` matches this pattern.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn matches(&self, input: &str) -> bool {
        let input = input.as_bytes();
        let pattern = self.pattern.as_bytes();
        let mut sx = 0;
        let mut cx = 0;
        let mut ok = false;

        while sx < input.len() && cx < pattern.len() {
            if input[sx].eq_ignore_ascii_case(&pattern[cx]) {
                sx += 1;
                cx += 1;
                ok = false;
            } else if pattern[cx] == b'*' {
                ok = true;
                cx += 1;
            } else {
                return false;
            }
        }

        // Input exhausted. Match if pattern is also exhausted, or if
        // we stopped at an abbreviation point (ok flag set, or next
        // pattern char is '*').
        sx == input.len()
            && (cx == pattern.len()
                || ok
                || (cx < pattern.len() && pattern[cx] == b'*'))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn exact_match() {
        let p = CommandPattern::new("t*e*ll");
        assert!(p.matches("tell"));
    }

    #[test]
    fn abbreviation_at_star_positions() {
        let p = CommandPattern::new("t*e*ll");
        assert!(p.matches("t"));
        assert!(p.matches("te"));
    }

    #[test]
    fn partial_after_star_rejection() {
        // "tel" does NOT match "t*e*ll" — ok cleared by first 'l',
        // second 'l' remains unmatched.
        let p = CommandPattern::new("t*e*ll");
        assert!(!p.matches("tel"));
    }

    #[test]
    fn case_insensitive() {
        let p = CommandPattern::new("t*e*ll");
        assert!(p.matches("TELL"));
        assert!(p.matches("Tell"));
        assert!(p.matches("T"));
        assert!(p.matches("TE"));
    }

    #[test]
    fn input_too_long() {
        let p = CommandPattern::new("t*e*ll");
        assert!(!p.matches("tells"));
    }

    #[test]
    fn no_stars() {
        let p = CommandPattern::new("asia");
        assert!(p.matches("asia"));
        assert!(!p.matches("asi"));
        assert!(!p.matches("asias"));
    }

    #[test]
    fn wrong_chars() {
        let p = CommandPattern::new("t*e*ll");
        assert!(!p.matches("tx"));
    }

    #[test]
    fn empty_input() {
        let p = CommandPattern::new("t*e*ll");
        assert!(!p.matches(""));
    }

    #[test]
    fn space_in_pattern() {
        let p = CommandPattern::new("mark* brown");
        assert!(p.matches("mark brown"));
        assert!(p.matches("mark"));
        assert!(!p.matches("mark b"));
    }

    #[test]
    fn single_char_pattern() {
        let p = CommandPattern::new("e");
        assert!(p.matches("e"));
        assert!(p.matches("E"));
        assert!(!p.matches(""));
        assert!(!p.matches("ex"));
    }

    #[test]
    fn pattern_ending_with_star() {
        let p = CommandPattern::new("accept all*s");
        assert!(p.matches("accept all"));
        assert!(p.matches("accept alls"));
        assert!(!p.matches("accept al"));
    }
}
