use std::path::Path;

use miette::{IntoDiagnostic, Result};
use regex::Regex;
use serde::Deserialize;

#[derive(Deserialize)]
struct NormalizeConfig {
    #[serde(default)]
    rules: Vec<Rule>,
}

#[derive(Deserialize)]
struct Rule {
    pattern: String,
    replace: String,
}

/// Applies regex normalization rules to transcript text.
pub struct Normalizer {
    rules: Vec<(Regex, String)>,
}

impl Normalizer {
    /// Load normalization rules from a TOML file.
    ///
    /// Returns an empty normalizer if the file does not exist.
    pub fn from_path(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self { rules: Vec::new() });
        }
        let content = std::fs::read_to_string(path).into_diagnostic()?;
        let config: NormalizeConfig =
            toml::from_str(&content).into_diagnostic()?;
        let rules = config
            .rules
            .into_iter()
            .map(|r| {
                let re = Regex::new(&r.pattern).into_diagnostic()?;
                Ok((re, r.replace))
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { rules })
    }

    /// Apply all normalization rules to the given text.
    pub fn apply(&self, text: &str) -> String {
        let mut result = text.to_string();
        for (re, replacement) in &self.rules {
            result = re.replace_all(&result, replacement.as_str()).into_owned();
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_normalizer_returns_input_unchanged() {
        let norm = Normalizer { rules: Vec::new() };
        assert_eq!(norm.apply("hello world"), "hello world");
    }

    #[test]
    fn replaces_timestamp_pattern() {
        let norm = Normalizer {
            rules: vec![(
                Regex::new(r"\d{1,2}:\d{2}:\d{2} [ap]\.m\.").unwrap(),
                "HH:MM:SS xm".to_string(),
            )],
        };
        let input = "Up at 2:48:31 p.m. on 2/10/26";
        assert_eq!(norm.apply(input), "Up at HH:MM:SS xm on 2/10/26");
    }

    #[test]
    fn applies_multiple_rules_in_order() {
        let norm = Normalizer {
            rules: vec![
                (Regex::new(r"cat").unwrap(), "dog".to_string()),
                (Regex::new(r"dog").unwrap(), "fish".to_string()),
            ],
        };
        // "cat" -> "dog" -> "fish" (rules chain)
        assert_eq!(norm.apply("I have a cat"), "I have a fish");
    }

    #[test]
    fn from_path_returns_empty_for_missing_file() {
        let norm =
            Normalizer::from_path(Path::new("/nonexistent/normalize.toml"))
                .unwrap();
        assert_eq!(norm.apply("unchanged"), "unchanged");
    }
}
