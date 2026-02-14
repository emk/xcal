//! Handler for the `bye` (and `stop`) command.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

pub fn cmd_bye(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.conference_terminated();
    conf.append_output(who, &msg);
    conf.flush_output(who);
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        let _ = user.port.shutdown();
    }
    CommandResult::EndConference
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn bye_terminates() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "bye");
        let out = tc.output(master);

        assert!(
            out.to_lowercase().contains("terminated"),
            "expected 'terminated' in bye output: {out}"
        );
    }
}
