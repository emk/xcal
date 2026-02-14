pub mod diff;
pub mod normalize;

use serde::{Deserialize, Serialize};

/// A single action in a JSONL test script.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Connect {
        conn: String,
    },
    Send {
        conn: String,
        text: String,
    },
    SendRaw {
        conn: String,
        bytes: String,
    },
    SendBytes {
        conn: String,
        hex: String,
    },
    Expect {
        conn: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        contains: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        matches: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        timeout_ms: Option<u64>,
    },
    Drain {
        conn: String,
        #[serde(default = "default_quiet_ms")]
        quiet_ms: u64,
    },
    Disconnect {
        conn: String,
    },
    Sleep {
        ms: u64,
    },
}

/// A single line from a JSONL script — either a comment or an action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ScriptLine {
    Comment { comment: String },
    Action(Action),
}

fn default_quiet_ms() -> u64 {
    100
}

/// Parse a JSONL script from a string, returning one `ScriptLine` per line.
pub fn parse_script(input: &str) -> Result<Vec<ScriptLine>, serde_json::Error> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(serde_json::from_str)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_connect() {
        let action = Action::Connect {
            conn: "master".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_send() {
        let action = Action::Send {
            conn: "master".into(),
            text: "hello".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_send_bytes() {
        let action = Action::SendBytes {
            conn: "user1".into(),
            hex: "ff f3".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_expect_contains() {
        let action = Action::Expect {
            conn: "master".into(),
            contains: Some("welcome".into()),
            matches: None,
            timeout_ms: None,
        };
        let json = serde_json::to_string(&action).unwrap();
        assert!(!json.contains("matches"));
        assert!(!json.contains("timeout_ms"));
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_expect_matches() {
        let action = Action::Expect {
            conn: "master".into(),
            contains: None,
            matches: Some(r"Explorer.*idle".into()),
            timeout_ms: Some(5000),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_drain_default() {
        let json = r#"{"action":"drain","conn":"master"}"#;
        let parsed: Action = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            Action::Drain {
                conn: "master".into(),
                quiet_ms: 100,
            }
        );
    }

    #[test]
    fn round_trip_drain_explicit() {
        let action = Action::Drain {
            conn: "master".into(),
            quiet_ms: 500,
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_disconnect() {
        let action = Action::Disconnect {
            conn: "master".into(),
        };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn round_trip_sleep() {
        let action = Action::Sleep { ms: 1000 };
        let json = serde_json::to_string(&action).unwrap();
        let parsed: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(action, parsed);
    }

    #[test]
    fn script_line_comment() {
        let json = r#"{"comment": "This is a comment"}"#;
        let parsed: ScriptLine = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            ScriptLine::Comment {
                comment: "This is a comment".into()
            }
        );
    }

    #[test]
    fn script_line_action() {
        let json = r#"{"action":"connect","conn":"master"}"#;
        let parsed: ScriptLine = serde_json::from_str(json).unwrap();
        assert_eq!(
            parsed,
            ScriptLine::Action(Action::Connect {
                conn: "master".into()
            })
        );
    }

    #[test]
    fn parse_full_script() {
        let script = r#"{"comment": "Tracer bullet"}
{"conn": "master", "action": "connect"}
{"conn": "master", "action": "send", "text": "hello"}
{"conn": "master", "action": "expect", "contains": "welcome"}
{"conn": "master", "action": "drain", "quiet_ms": 100}
{"conn": "master", "action": "disconnect"}
"#;
        let lines = parse_script(script).unwrap();
        assert_eq!(lines.len(), 6);
        assert!(matches!(&lines[0], ScriptLine::Comment { .. }));
        assert!(matches!(
            &lines[1],
            ScriptLine::Action(Action::Connect { .. })
        ));
        assert!(matches!(
            &lines[5],
            ScriptLine::Action(Action::Disconnect { .. })
        ));
    }
}
