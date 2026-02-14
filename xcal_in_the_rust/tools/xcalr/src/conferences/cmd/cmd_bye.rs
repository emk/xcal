//! Handler for the `bye` (and `stop`) command.

use super::{super::Conference, CommandResult};
use crate::users::{Role, UserId};

pub fn cmd_bye(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    let is_master = conf
        .users
        .get(idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| matches!(u.role, Role::Master { .. }));

    if is_master {
        // Master bye → terminate the entire conference (C: *flag = 1).
        CommandResult::EndConference
    } else {
        // Regular user bye → disconnect just this user (C: con_exuser).
        let msg = conf.lang.disconnected();
        conf.append_output(who, &msg);
        conf.flush_output(who);
        conf.disconnect_user(who, false);
        CommandResult::Ok
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn master_bye_terminates() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "bye");

        // Master should see "Conference terminated".
        let out = tc.output(master);
        assert!(
            out.to_lowercase().contains("terminated"),
            "expected 'terminated' in master bye output: {out}"
        );

        // Other user should also see "Conference terminated".
        let out1 = tc.output_if_connected(1);
        assert!(
            out1.to_lowercase().contains("terminated"),
            "expected 'terminated' for user1: {out1}"
        );
    }

    #[test]
    fn user_bye_disconnects_only_self() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "bye");
        let out = tc.output(user1);
        assert!(
            out.to_lowercase().contains("no longer connected"),
            "expected disconnected message: {out}"
        );

        // Master should see exit notification, not termination.
        let out_m = tc.output(master);
        assert!(
            out_m.contains("Wanderer"),
            "expected exit notification: {out_m}"
        );
        assert!(
            !out_m.to_lowercase().contains("terminated"),
            "user bye should not terminate conference: {out_m}"
        );
    }

    #[test]
    fn stop_alias_works() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "stop");
        let out = tc.output(master);
        assert!(
            out.to_lowercase().contains("terminated"),
            "expected 'terminated' for stop: {out}"
        );
    }
}
