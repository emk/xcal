//! Handlers for help file lookup: help, explain.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

/// HELP command — display general help (sugar for `explain "help"`).
pub fn cmd_help(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let topic = conf.lang.help_topic();
    cmd_explain_impl(conf, who, &topic)
}

/// EXPLAIN command — look up a help topic by name.
pub fn cmd_explain(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    cmd_explain_impl(conf, who, args)
}

/// Shared implementation for help/explain lookup.
fn cmd_explain_impl(
    conf: &mut Conference,
    who: UserId,
    topic: &str,
) -> CommandResult {
    match conf.help.lookup(topic) {
        Some(content) => {
            // C: "\r\n" + content (with tab expansion) + "\r\n"
            conf.append_output(who, "\n");
            // Expand tabs to 4 spaces, matching C's TABSTR.
            let expanded = content.replace('\t', "    ");
            conf.append_output(who, &expanded);
            conf.append_output(who, "\n");
        }
        None => {
            let msg = conf.lang.cant_explain();
            conf.append_output(who, &msg);
        }
    }
    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn help_shows_general_info() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "help");
        let out = tc.output(master);
        assert!(
            out.contains("General information"),
            "expected general help content: {out}"
        );
    }

    #[test]
    fn explain_shows_topic_content() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "explain who");
        let out = tc.output(master);
        assert!(out.contains("WHO"), "expected WHO help content: {out}");
    }

    #[test]
    fn explain_abbreviated_topic() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "explain te");
        let out = tc.output(master);
        assert!(out.contains("TELL"), "expected TELL help content: {out}");
    }

    #[test]
    fn explain_unknown_topic() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "explain zzznotreal");
        let out = tc.output(master);
        assert!(
            out.contains("Can't explain"),
            "expected cant_explain message: {out}"
        );
    }

    #[test]
    fn explain_empty_shows_cant_explain() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "explain");
        let out = tc.output(master);
        assert!(
            out.contains("Can't explain"),
            "expected cant_explain for empty topic: {out}"
        );
    }

    #[test]
    fn explain_abbreviated_e() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        // "e who" → explain who
        tc.input(master, "e who");
        let out = tc.output(master);
        assert!(out.contains("WHO"), "expected WHO help from 'e who': {out}");
    }
}
