//! Handlers for user state transitions: id, out, line.

use super::{super::Conference, CommandResult};
use crate::users::{UserId, UserState};

pub fn cmd_id(conf: &mut Conference, who: UserId, args: &str) -> CommandResult {
    let trimmed = args.trim();

    if trimmed.is_empty() {
        // BUILD mode — prompt for new name (C: CHG_ST).
        let msg = conf.lang.new_name_prompt();
        conf.append_output(who, &msg);

        let idx = usize::from(who.0);
        if let Some(Some(user)) = conf.users.get_mut(idx) {
            user.state = UserState::ChangingName;
        }
    } else {
        // Inline rename (C: user_set_name + con_change).
        let idx = usize::from(who.0);
        if let Some(Some(user)) = conf.users.get_mut(idx) {
            user.name = trimmed.to_string();
        }

        // Send rename notification (C: con_change).
        let notification = conf.lang.name_changed(who, trimmed);
        conf.send_notification(&notification, who);

        let done = conf.lang.done();
        conf.append_output(who, &done);
    }

    CommandResult::Ok
}

pub fn cmd_out(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);

    // Clear the message queue (C: user_dequeue loop).
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.queue.clear();
    }

    let msg = conf.lang.you_are_out();
    conf.append_output(who, &msg);

    // Set Out state (C: SETSTATE OUT_ST).
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.state = UserState::Out;
    }

    CommandResult::Ok
}

pub fn cmd_line(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.command_line_prompt();
    conf.append_output(who, &msg);

    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.state = UserState::Line;
    }

    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn id_inline() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "id NewName");
        let out = tc.output(user1);
        assert!(out.contains("Done"), "expected Done: {out}");

        // Notification should go to master.
        let out_m = tc.output(master);
        assert!(
            out_m.contains("NewName"),
            "expected rename notification: {out_m}"
        );
    }

    #[test]
    fn id_build_mode() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "id");
        let out = tc.output(user1);
        assert!(
            out.contains("name") || out.contains("Name"),
            "expected name prompt: {out}"
        );

        tc.input(user1, "NewIdentity");
        let out = tc.output(user1);
        assert!(out.contains("Done"), "expected Done: {out}");

        // Notification should go to master.
        let out_m = tc.output(master);
        assert!(
            out_m.contains("NewIdentity"),
            "expected rename notification: {out_m}"
        );
    }

    #[test]
    fn out_and_resume() {
        let mut tc = TestConference::new();
        let _master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "out");
        let out = tc.output(user1);
        assert!(
            out.contains("out") || out.contains("Out"),
            "expected out message: {out}"
        );

        // Any input returns to Idle (C: user_poll OUT_ST).
        tc.input(user1, "anything");

        // After resume, commands should work again.
        tc.input(user1, "who");
        let out = tc.output(user1);
        assert!(
            out.contains("Explorer"),
            "expected who to work after resume: {out}"
        );
    }

    #[test]
    fn tell_to_out_user_rejected() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "out");
        tc.drain(user1);

        // Direct tell to Out user should be rejected.
        tc.input(master, "t 1;Hello while out");
        let out = tc.output(master);
        assert!(
            out.to_lowercase().contains("out"),
            "expected users-out error: {out}"
        );
    }

    #[test]
    fn out_clears_queue() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        // Put user1 in composing state so messages queue.
        tc.input(user1, "t 0");
        tc.drain(user1);

        // Master sends to user1 while composing (gets queued).
        tc.input(master, "t 1;Queued message");
        tc.drain(master);

        // User1 aborts composing and goes out.
        tc.input(user1, "%k");
        tc.input(user1, "");
        tc.drain(user1);
        tc.input(user1, "out");
        tc.drain(user1);

        // Resume — queued message should be gone.
        tc.input(user1, "resume");
        let out = tc.output(user1);
        assert!(
            !out.contains("Queued message"),
            "expected queue cleared by out: {out}"
        );
    }

    #[test]
    fn line_mode() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "line");
        let out = tc.output(user1);
        assert!(
            out.contains("Command line") || out.contains("command"),
            "expected line prompt: {out}"
        );

        // While in line mode, run a command — should work.
        tc.input(user1, "who");
        let out = tc.output(user1);
        assert!(
            out.contains("Explorer"),
            "expected who output in line mode: {out}"
        );

        // After the command, user should be back in idle
        // (dispatch_command sets Idle when state is still Command).
        // So messages should now be delivered.
        tc.input(master, "t 1;Line mode test");
        tc.drain(master);
        let out = tc.output(user1);
        assert!(
            out.contains("Line mode test"),
            "expected message after line command: {out}"
        );
    }
}
