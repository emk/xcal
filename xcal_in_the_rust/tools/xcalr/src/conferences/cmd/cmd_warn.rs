//! Handlers for the warning system: warn, display_warning.

use super::{super::Conference, CommandResult};
use crate::users::{Role, UserId, UserState};

pub fn cmd_warn(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    // Master only (C: ISMASTER check).
    let idx = usize::from(who.0);
    let is_master = conf
        .users
        .get(idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| matches!(u.role, Role::Master { .. }));

    if !is_master {
        let msg = conf.lang.command_error();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let trimmed = args.trim();

    if trimmed.is_empty() {
        // BUILD mode — prompt for warning text (C: WARN_ST).
        let msg = conf.lang.new_warning_prompt();
        conf.append_output(who, &msg);

        if let Some(Some(user)) = conf.users.get_mut(idx) {
            user.state = UserState::ComposingWarning;
        }
    } else {
        // Inline warning (C: con_warn directly).
        conf.post_warning(trimmed);
    }

    CommandResult::Ok
}

pub fn cmd_display_warning(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    if let Some(ref warning) = conf.current_warning {
        // Output the stored warning body (already has \n framing).
        let body = warning.body.clone();
        conf.append_output_bytes(who, &body);
    } else {
        // No warning: output BEL BEL (C: "\r\n\a\a", 4 bytes).
        conf.append_output(who, "\n\x07\x07");
    }

    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn dw_no_warning() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "dw");
        let out = tc.output(master);
        assert!(
            out.contains('\x07'),
            "expected BEL when no warning: {out:?}"
        );
    }

    #[test]
    fn warn_inline() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "warn Server closing soon");
        tc.drain(master);

        // User1 should receive the warning.
        let out = tc.output(user1);
        assert!(
            out.contains("Server closing soon"),
            "expected warning: {out}"
        );
    }

    #[test]
    fn warn_build_mode() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "warn");
        let out = tc.output(master);
        assert!(
            out.contains("warning") || out.contains("Warning"),
            "expected warning prompt: {out}"
        );

        tc.input(master, "Maintenance in 5 minutes");
        tc.drain(master);

        // User1 should receive the warning.
        let out = tc.output(user1);
        assert!(
            out.contains("Maintenance in 5 minutes"),
            "expected warning: {out}"
        );
    }

    #[test]
    fn dw_shows_stored_warning() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "warn Server notice");
        tc.drain(master);

        tc.input(master, "dw");
        let out = tc.output(master);
        assert!(
            out.contains("Server notice"),
            "expected stored warning: {out}"
        );
    }

    #[test]
    fn warn_not_master_rejected() {
        let mut tc = TestConference::new();
        let _master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "warn Should fail");
        let out = tc.output(user1);
        assert!(
            out.to_lowercase().contains("error")
                || out.to_lowercase().contains("command"),
            "expected error for non-master warn: {out}"
        );
    }
}
