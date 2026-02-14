//! Handlers for ignore-list management: ignore, accept.

use super::{super::Conference, CommandResult};
use crate::users::{UserId, UserSet};

pub fn cmd_ignore(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    let Ok((targets, _rest)) = conf.parse_user_args(who, args) else {
        return CommandResult::Ok;
    };

    // Self-ignore is invalid (C: MAPBIT(map, who) check).
    if targets.contains(who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let who_idx = usize::from(who.0);

    if targets.is_empty() {
        // Ignore all (C: user_ignore(ALL); user_accept(self)).
        // Set all bits, then clear self.
        if let Some(Some(user)) = conf.users.get_mut(who_idx) {
            user.ignoring = UserSet(u64::MAX);
            user.ignoring.remove(who);
        }
    } else {
        // Specific targets — check subconference membership.
        let who_master = conf
            .users
            .get(who_idx)
            .and_then(|s| s.as_ref())
            .map_or(UserId(0), |u| u.master);

        for target_id in targets.iter() {
            let t_idx = usize::from(target_id.0);
            let target_master = conf
                .users
                .get(t_idx)
                .and_then(|s| s.as_ref())
                .map_or(UserId(0), |u| u.master);

            // C: USER(cp, ix)->mstr == USER(cp, who)->mstr ||
            //    USER(cp, ix)->mstr == who
            if target_master != who_master && target_master != who {
                let msg = conf.lang.invalid_arguments();
                conf.append_output(who, &msg);
                return CommandResult::Ok;
            }
        }

        if let Some(Some(user)) = conf.users.get_mut(who_idx) {
            for target_id in targets.iter() {
                user.ignoring.insert(target_id);
            }
        }
    }

    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    let Ok((targets, _rest)) = conf.parse_user_args(who, args) else {
        return CommandResult::Ok;
    };

    // Self-accept is invalid (C: MAPBIT(map, who) check).
    if targets.contains(who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let who_idx = usize::from(who.0);

    if targets.is_empty() {
        // Accept all (C: user_accept(ALL)).
        if let Some(Some(user)) = conf.users.get_mut(who_idx) {
            user.ignoring = UserSet::new();
        }
    } else {
        // Specific targets — check subconference membership.
        let who_master = conf
            .users
            .get(who_idx)
            .and_then(|s| s.as_ref())
            .map_or(UserId(0), |u| u.master);

        for target_id in targets.iter() {
            let t_idx = usize::from(target_id.0);
            let target_master = conf
                .users
                .get(t_idx)
                .and_then(|s| s.as_ref())
                .map_or(UserId(0), |u| u.master);

            if target_master != who_master && target_master != who {
                let msg = conf.lang.invalid_arguments();
                conf.append_output(who, &msg);
                return CommandResult::Ok;
            }
        }

        if let Some(Some(user)) = conf.users.get_mut(who_idx) {
            for target_id in targets.iter() {
                user.ignoring.remove(target_id);
            }
        }
    }

    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn ignore_specific_user() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "ig 1");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");

        // Direct tell from user1 should not be delivered.
        tc.input(user1, "t 0;Ignored message");
        tc.drain(user1);

        // Master should NOT see the ignored message (it has no
        // recipients after ignore filter).
        let out_m = tc.output(master);
        assert!(
            !out_m.contains("Ignored message"),
            "ignored message should not be delivered: {out_m}"
        );
    }

    #[test]
    fn accept_specific_user() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "ig 1");
        tc.drain(master);
        tc.input(master, "ac 1");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");

        // Now messages should be delivered again.
        tc.input(user1, "t 0;Accepted message");
        tc.drain(user1);

        let out_m = tc.output(master);
        assert!(
            out_m.contains("Accepted message"),
            "accepted message should be delivered: {out_m}"
        );
    }

    #[test]
    fn ignore_all() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");
        let user2 = tc.connect("Traveler");

        tc.input(master, "ig");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");

        // Messages from both users should be ignored.
        tc.input(user1, "t 0;From user1");
        tc.drain(user1);
        tc.input(user2, "t 0;From user2");
        tc.drain(user2);

        let out_m = tc.output(master);
        assert!(
            !out_m.contains("From user1"),
            "should ignore user1: {out_m}"
        );
        assert!(
            !out_m.contains("From user2"),
            "should ignore user2: {out_m}"
        );
    }

    #[test]
    fn accept_all() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "ig");
        tc.drain(master);
        tc.input(master, "ac");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");

        // Messages should be delivered again.
        tc.input(user1, "t 0;After accept all");
        tc.drain(user1);

        let out_m = tc.output(master);
        assert!(
            out_m.contains("After accept all"),
            "message should be delivered after accept all: {out_m}"
        );
    }

    #[test]
    fn ignore_self_rejected() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "ig 0");
        let out = tc.output(master);
        assert!(
            out.to_lowercase().contains("invalid")
                || out.to_lowercase().contains("argument"),
            "expected invalid arguments for self-ignore: {out}"
        );
    }

    #[test]
    fn who_shows_ignore_flags() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        // Master ignores user1.
        tc.input(master, "ig 1");
        tc.drain(master);

        tc.input(master, "who");
        let out = tc.output(master);
        // Should show "I" flag for user1 (master ignoring them).
        assert!(out.contains("I"), "expected I flag in who output: {out}");
    }

    #[test]
    fn who_shows_mutual_ignore() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        // Mutual ignore.
        tc.input(master, "ig 1");
        tc.drain(master);
        tc.input(user1, "ig 0");
        tc.drain(user1);

        tc.input(master, "who");
        let out = tc.output(master);
        // Should show "B" flag for user1 (both directions).
        assert!(
            out.contains("B"),
            "expected B flag for mutual ignore: {out}"
        );
    }
}
