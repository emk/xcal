//! Handlers for help file lookup: help, explain.

use super::{super::Conference, CommandResult};
use crate::users::UserId;

pub fn cmd_help(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.not_implemented();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_explain(
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
