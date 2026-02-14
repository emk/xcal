//! Handlers for master/subconference lifecycle commands: kill, make_tty,
//! normalize, give, enable, disable.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

pub fn cmd_kill(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_make_tty(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_normalize(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_give(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_enable(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_disable(
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
    #[allow(unused_imports)]
    use crate::conferences::test_harness::TestConference;
}
