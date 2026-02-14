//! Handlers for ignore-list management: ignore, accept.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

pub fn cmd_ignore(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept(
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
