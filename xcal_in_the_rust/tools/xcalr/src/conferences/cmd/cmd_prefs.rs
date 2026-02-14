//! Handlers for user preference toggles: reject_alls, accept_alls,
//! accept_notify, reject_notify, reject_ports, reject_controls,
//! okay_controls, reject_breaks, accept_breaks.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

pub fn cmd_reject_alls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept_alls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept_notify(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_notify(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_ports(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_controls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_okay_controls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_breaks(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept_breaks(
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
