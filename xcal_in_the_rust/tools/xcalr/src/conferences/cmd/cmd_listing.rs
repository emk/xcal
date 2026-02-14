//! Handlers for user-listing commands: who, port, everything, tty, left,
//! users, below, extended who.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

pub fn cmd_who(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let text = conf.format_who(who);
    conf.append_output(who, &text);
    CommandResult::Ok
}

pub fn cmd_port(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_everything(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_tty(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_left(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_users(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_below(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_xwho(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn who_shows_connected_user() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "who");
        let out = tc.output(master);

        assert!(
            out.contains("Explorer"),
            "expected name in who output: {out}"
        );
        assert!(
            out.contains("=>"),
            "expected => marker in who output: {out}"
        );
    }

    #[test]
    fn abbreviated_who() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "w");
        let out = tc.output(master);

        assert!(
            out.contains("Explorer"),
            "expected 'w' to dispatch as who: {out}"
        );
    }

    #[test]
    fn multi_user_who() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "who");
        let out = tc.output(master);

        assert!(
            out.contains("Explorer"),
            "expected master in who output: {out}"
        );
        assert!(
            out.contains("Wanderer"),
            "expected second user in who output: {out}"
        );
    }
}
