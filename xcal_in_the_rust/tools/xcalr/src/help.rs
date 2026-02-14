#![allow(dead_code)]

use crate::matching::CommandPattern;

/// Embedded help file content, keyed by filename.
///
/// Each entry maps a `.hf` filename to its `include_str!` content.
/// The TOC parser resolves filenames against this table.
static HELP_FILES: &[(&str, &str)] = &[
    ("aa.hf", include_str!("../../../../help/aa.hf")),
    ("ab.hf", include_str!("../../../../help/ab.hf")),
    ("accept.hf", include_str!("../../../../help/accept.hf")),
    ("an.hf", include_str!("../../../../help/an.hf")),
    ("asia.hf", include_str!("../../../../help/asia.hf")),
    ("bye.hf", include_str!("../../../../help/bye.hf")),
    ("clock.hf", include_str!("../../../../help/clock.hf")),
    ("commands.hf", include_str!("../../../../help/commands.hf")),
    ("disable.hf", include_str!("../../../../help/disable.hf")),
    ("dw.hf", include_str!("../../../../help/dw.hf")),
    ("eambig.hf", include_str!("../../../../help/eambig.hf")),
    ("enable.hf", include_str!("../../../../help/enable.hf")),
    (
        "everything.hf",
        include_str!("../../../../help/everything.hf"),
    ),
    ("explain.hf", include_str!("../../../../help/explain.hf")),
    ("general.hf", include_str!("../../../../help/general.hf")),
    ("give.hf", include_str!("../../../../help/give.hf")),
    ("hawaii.hf", include_str!("../../../../help/hawaii.hf")),
    ("id.hf", include_str!("../../../../help/id.hf")),
    ("ignore.hf", include_str!("../../../../help/ignore.hf")),
    (
        "impending.hf",
        include_str!("../../../../help/impending.hf"),
    ),
    ("left.hf", include_str!("../../../../help/left.hf")),
    ("life.hf", include_str!("../../../../help/life.hf")),
    ("line.hf", include_str!("../../../../help/line.hf")),
    ("mark.hf", include_str!("../../../../help/mark.hf")),
    ("master.hf", include_str!("../../../../help/master.hf")),
    ("mty.hf", include_str!("../../../../help/mty.hf")),
    ("new.hf", include_str!("../../../../help/new.hf")),
    ("normal.hf", include_str!("../../../../help/normal.hf")),
    ("oc.hf", include_str!("../../../../help/oc.hf")),
    ("out.hf", include_str!("../../../../help/out.hf")),
    ("port.hf", include_str!("../../../../help/port.hf")),
    ("ra.hf", include_str!("../../../../help/ra.hf")),
    ("rb.hf", include_str!("../../../../help/rb.hf")),
    ("rc.hf", include_str!("../../../../help/rc.hf")),
    ("rn.hf", include_str!("../../../../help/rn.hf")),
    ("roadrun.hf", include_str!("../../../../help/roadrun.hf")),
    ("rp.hf", include_str!("../../../../help/rp.hf")),
    ("sight.hf", include_str!("../../../../help/sight.hf")),
    ("states.hf", include_str!("../../../../help/states.hf")),
    ("subcon.hf", include_str!("../../../../help/subcon.hf")),
    ("tell.hf", include_str!("../../../../help/tell.hf")),
    ("topics.hf", include_str!("../../../../help/topics.hf")),
    ("usnu.hf", include_str!("../../../../help/usnu.hf")),
    ("warn.hf", include_str!("../../../../help/warn.hf")),
    ("who.hf", include_str!("../../../../help/who.hf")),
];

/// The embedded table of contents.
static EXPTOC: &str = include_str!("../../../../help/exptoc.txt");

/// A single entry from the help table of contents.
///
/// Maps one or more topic patterns to a help file's content.
struct TocEntry {
    /// Topic patterns (e.g., `t*e*ll`), parsed into `CommandPattern`s.
    patterns: Vec<CommandPattern>,
    /// The help file content (an `&'static str` from `include_str!`).
    content: &'static str,
}

/// Help system backed by embedded `.hf` files and the `exptoc.txt`
/// table of contents.
///
/// Parses the TOC at construction time and provides topic lookup using
/// the `CommandPattern` abbreviation-matching algorithm.
pub struct HelpTopics {
    entries: Vec<TocEntry>,
}

impl HelpTopics {
    /// Parse the embedded `exptoc.txt` and validate all referenced files.
    ///
    /// # Panics
    ///
    /// Panics if `exptoc.txt` references a `.hf` file that is not in the
    /// embedded file table. This is a compile-time data integrity check.
    pub fn new() -> Self {
        let mut entries = Vec::new();

        for line in EXPTOC.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let Some((filename, patterns_str)) = line.split_once(':') else {
                panic!("invalid TOC line (no colon): {line}");
            };

            let content = HELP_FILES
                .iter()
                .find(|(name, _)| *name == filename)
                .unwrap_or_else(|| {
                    panic!("TOC references missing file: {filename}")
                })
                .1;

            let patterns = patterns_str
                .split(',')
                .map(|p| CommandPattern::new(p.trim()))
                .collect();

            entries.push(TocEntry { patterns, content });
        }

        Self { entries }
    }

    /// Look up a help topic by user input.
    ///
    /// Returns the help file content on match, or `None` if no topic
    /// pattern matches the input. Matching uses `CommandPattern` — the
    /// same abbreviation-matching algorithm as the C server.
    ///
    /// The returned text contains literal `\n` and `\t` characters. The
    /// caller is responsible for tab expansion and `\n` → `\r\n`
    /// conversion at output time.
    pub fn lookup(&self, topic: &str) -> Option<&'static str> {
        let topic = topic.trim_start();
        if topic.is_empty() {
            return None;
        }

        for entry in &self.entries {
            for pattern in &entry.patterns {
                if pattern.matches(topic) {
                    return Some(entry.content);
                }
            }
        }

        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn construction_succeeds() {
        let _h = HelpTopics::new();
    }

    #[test]
    fn lookup_exact_topic() {
        let h = HelpTopics::new();
        let text = h.lookup("tell");
        assert!(text.is_some());
        assert!(text.unwrap().contains("TELL"));
    }

    #[test]
    fn lookup_abbreviated_topic() {
        let h = HelpTopics::new();
        assert!(h.lookup("t").is_some()); // t*e*ll
        assert!(h.lookup("te").is_some()); // t*e*ll
        assert!(h.lookup("w").is_some()); // w*ho
    }

    #[test]
    fn lookup_no_match() {
        let h = HelpTopics::new();
        assert!(h.lookup("zzz").is_none());
        assert!(h.lookup("").is_none());
    }

    #[test]
    fn lookup_case_insensitive() {
        let h = HelpTopics::new();
        assert!(h.lookup("TELL").is_some());
        assert!(h.lookup("Tell").is_some());
        assert!(h.lookup("WHO").is_some());
    }

    #[test]
    fn lookup_help_resolves_to_general() {
        let h = HelpTopics::new();
        let text = h.lookup("help");
        assert!(text.is_some());
        assert!(text.unwrap().contains("General information"));
    }

    #[test]
    fn lookup_eambig() {
        let h = HelpTopics::new();
        let text = h.lookup("e");
        assert!(text.is_some());
        assert!(text.unwrap().contains("can't explain"));
    }

    #[test]
    fn lookup_mark_brown_easter_egg() {
        let h = HelpTopics::new();
        assert!(h.lookup("mark brown").is_some());
        assert!(h.lookup("mark").is_some()); // abbreviation
    }

    #[test]
    fn lookup_everything_via_a() {
        let h = HelpTopics::new();
        let text = h.lookup("a");
        assert!(text.is_some());
        // "a" matches "ev*ery*thing,a" — the second pattern
        assert!(text.unwrap().contains("EVerything"));
    }

    #[test]
    fn lookup_tel_does_not_match_tell() {
        // This is the critical edge case. "tel" does NOT match "t*e*ll"
        // because ok is cleared by the first 'l' match, and the second
        // 'l' in the pattern remains unmatched.
        let h = HelpTopics::new();
        assert!(h.lookup("tel").is_none());
    }

    #[test]
    fn lookup_leading_whitespace_stripped() {
        let h = HelpTopics::new();
        assert!(h.lookup("  tell").is_some());
        assert!(h.lookup("\ttell").is_some());
    }

    #[test]
    fn all_toc_entries_have_content() {
        // HelpTopics::new() already validates this via panic, but this
        // test documents the invariant explicitly.
        let h = HelpTopics::new();
        assert_eq!(h.entries.len(), 45);
    }
}
