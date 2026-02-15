//! Handlers for master/subconference lifecycle commands: kill, make_tty,
//! normalize, give, enable, disable.

use std::time::{Instant, SystemTime};

use super::{super::Conference, CommandResult};
use crate::{
    conferences::LeftUser,
    users::{Role, UserId, UserState},
};

/// Check if the caller is master or submaster.
fn is_master_or_sub(conf: &Conference, who: UserId) -> bool {
    let idx = usize::from(who.0);
    conf.users
        .get(idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| {
            matches!(u.role, Role::Master { .. } | Role::Submaster { .. })
        })
}

/// Check if `who` can manage `target` (master/submaster can manage
/// users whose `master` field points to `who`).
fn can_manage(conf: &Conference, who: UserId, target: UserId) -> bool {
    let who_idx = usize::from(who.0);
    let target_idx = usize::from(target.0);

    let is_master_or_sub = conf
        .users
        .get(who_idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| {
            matches!(u.role, Role::Master { .. } | Role::Submaster { .. })
        });

    if !is_master_or_sub {
        return false;
    }

    // Both master and submaster can only manage users whose master
    // is `who` (C: `mstr == who`).
    conf.users
        .get(target_idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| u.master == who)
}

pub fn cmd_kill(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    if !is_master_or_sub(conf, who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let Ok((targets, _)) = conf.parse_user_args(who, args) else {
        return CommandResult::Ok;
    };
    if targets.is_empty() {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    // Collect info for post-loop notifications:
    // (target_id, name, target's master for subcon scoping).
    let mut killed: Vec<(UserId, String, UserId)> = Vec::new();
    // Auto-normalize results: (parent_master, subordinate_ids).
    let mut normalize_results: Vec<(UserId, Vec<UserId>)> = Vec::new();

    for target_id in targets.iter() {
        if target_id == who {
            continue; // Can't kill yourself.
        }

        if !can_manage(conf, who, target_id) {
            continue;
        }

        let target_idx = usize::from(target_id.0);
        let (name, target_master) =
            conf.users[target_idx].as_ref().map_or_else(
                || (String::new(), UserId(0)),
                |u| (u.name.clone(), u.master),
            );

        // Auto-normalize: reparent subordinates and send "talking with"
        // before removing. "Passed" + listing is deferred until after Done.
        if let Some(result) = conf.auto_normalize_reparent(target_id) {
            normalize_results.push(result);
        }

        // Send disconnect message to the killed user before shutdown.
        let disc_msg = conf.lang.disconnected();
        conf.append_output(target_id, &disc_msg);
        conf.flush_output(target_id);

        // Shut down the port.
        if let Some(Some(user)) = conf.users.get_mut(target_idx) {
            let _ = user.port.shutdown();
        }

        // Remove port_to_user mapping.
        if let Some(Some(user)) = conf.users.get(target_idx) {
            let port_id = user.port.id();
            conf.port_to_user.remove(&port_id);
        }

        // Remove the user slot.
        if let Some(slot) = conf.users.get_mut(target_idx) {
            *slot = None;
        }

        conf.current_count = conf.current_count.saturating_sub(1);

        // Add to left list.
        conf.left.push(LeftUser {
            id: target_id,
            name: name.clone(),
            left_at: Instant::now(),
            departed_at: SystemTime::now(),
            was_killed: true,
        });

        killed.push((target_id, name, target_master));
    }

    // Send "Done" to caller.
    let done = conf.lang.done();
    conf.append_output(who, &done);

    // Send "passed" + listing for each auto-normalized submaster.
    for (parent, subs) in &normalize_results {
        conf.send_passed_listing(*parent, subs);
    }

    // Send kill notification scoped to the killed user's subconference.
    for (target_id, name, target_master) in &killed {
        let kill_msg = conf.lang.user_killed(*target_id, name);

        let notify_ids: Vec<UserId> = conf
            .users
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|u| {
                !u.prefs.reject_notifications
                    && u.state != UserState::Login
                    && u.state != UserState::Out
                    // Subconference scoping using the killed user's
                    // master (captured before removal).
                    && (u.master == *target_master
                        || u.master == *target_id
                        || u.id == *target_master)
            })
            .map(|u| u.id)
            .collect();

        for notify_id in notify_ids {
            conf.append_output(notify_id, &kill_msg);
            if notify_id != who {
                conf.flush_output(notify_id);
            }
        }
    }

    CommandResult::Ok
}

pub fn cmd_make_tty(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    if !is_master_or_sub(conf, who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let Ok((targets, _)) = conf.parse_user_args(who, args) else {
        return CommandResult::Ok;
    };
    // Exactly one target required.
    let target_id = if targets.count() == 1 {
        targets.iter().next()
    } else {
        None
    };
    let Some(target_id) = target_id else {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    };

    let target_idx = usize::from(target_id.0);

    // Promote target to submaster.
    if let Some(Some(user)) = conf.users.get_mut(target_idx) {
        user.role = Role::Submaster {
            tell_all_enabled: true,
        };
        user.master = who;
    }

    // Send "now master" to the promoted user.
    let now_master = conf.lang.now_master();
    conf.append_output(target_id, &now_master);
    conf.flush_output(target_id);

    let done = conf.lang.done();
    conf.append_output(who, &done);
    CommandResult::Ok
}

pub fn cmd_normalize(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    if !is_master_or_sub(conf, who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let trimmed = args.trim();

    // Collect targets: either specific user number(s) or all submasters under caller.
    let targets: Vec<UserId> = if trimmed.is_empty() {
        // Bare "no" — normalize all submasters under caller.
        conf.users
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|u| {
                matches!(u.role, Role::Submaster { .. }) && u.master == who
            })
            .map(|u| u.id)
            .collect()
    } else {
        let Ok((target_set, _)) = conf.parse_user_args(who, trimmed) else {
            return CommandResult::Ok;
        };
        target_set.iter().collect()
    };

    for &target_id in &targets {
        let target_idx = usize::from(target_id.0);

        // Move subordinates of the demoted submaster back to caller.
        let subordinates: Vec<UserId> = conf
            .users
            .iter()
            .filter_map(|slot| slot.as_ref())
            .filter(|u| u.master == target_id && u.id != target_id)
            .map(|u| u.id)
            .collect();

        for sub_id in &subordinates {
            let sub_idx = usize::from(sub_id.0);
            if let Some(Some(user)) = conf.users.get_mut(sub_idx) {
                user.master = who;
            }
        }

        // Send "talking with" to each reparented subordinate.
        let who_idx = usize::from(who.0);
        let who_name = conf
            .users
            .get(who_idx)
            .and_then(|s| s.as_ref())
            .map(|u| u.name.clone())
            .unwrap_or_default();
        for sub_id in &subordinates {
            let talking = conf.lang.talking_with(who, &who_name);
            conf.append_output(*sub_id, "\n");
            conf.append_output(*sub_id, &talking);
            conf.flush_output(*sub_id);
        }

        // Demote the target.
        if let Some(Some(user)) = conf.users.get_mut(target_idx) {
            user.role = Role::Normal;
            user.master = who;
        }

        // Send "no longer master" to demoted user.
        let normal = conf.lang.no_longer_master();
        conf.append_output(target_id, &normal);
        conf.flush_output(target_id);
    }

    let done = conf.lang.done();
    conf.append_output(who, &done);

    // Send "you have been passed" notification + WHO-like listing
    // for each subordinate that was moved back.
    let passed = conf.lang.you_are_passed();
    conf.append_output(who, &passed);

    // Show the newly-acquired subordinates in a WHO-like listing
    // (excludes the demoted submasters themselves).
    let subordinate_ids: Vec<UserId> = conf
        .users
        .iter()
        .filter_map(|s| s.as_ref())
        .filter(|u| {
            u.master == who
                && u.id != who
                && !targets.contains(&u.id)
                && !matches!(
                    u.role,
                    Role::Submaster { .. } | Role::Master { .. }
                )
        })
        .map(|u| u.id)
        .collect();

    if !subordinate_ids.is_empty() {
        let mut listing = String::new();
        for &uid in &subordinate_ids {
            conf.format_who_entry(uid, who, &mut listing);
        }
        conf.append_output(who, &listing);
    }

    CommandResult::Ok
}

pub fn cmd_give(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    if !is_master_or_sub(conf, who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    // parse_user_args splits on ';', giving us (target, rest).
    let Ok((target_set, rest)) = conf.parse_user_args(who, args) else {
        return CommandResult::Ok;
    };
    // Exactly one target required.
    let target_id = if target_set.count() == 1 {
        target_set.iter().next()
    } else {
        None
    };
    let Some(target_id) = target_id else {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    };

    // Check that target is a submaster (or master).
    let target_idx = usize::from(target_id.0);
    let target_is_sub = conf
        .users
        .get(target_idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| {
            matches!(u.role, Role::Submaster { .. } | Role::Master { .. })
        });

    if !target_is_sub {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let user_list = rest.trim();

    if user_list.eq_ignore_ascii_case("new") {
        // "gi TARGET;new" — set new_user_master to target.
        conf.new_user_master = target_id;

        // Send "you have been passed" to target.
        let passed = conf.lang.you_are_passed();
        conf.append_output(target_id, &passed);
        conf.flush_output(target_id);

        // Send "new users" header to target.
        let header = conf.lang.new_users_header();
        conf.append_output(target_id, &header);
        conf.flush_output(target_id);
    } else if user_list.is_empty() {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    } else {
        // Parse user numbers to move.
        let Ok((move_set, _)) = conf.parse_user_args(who, user_list) else {
            return CommandResult::Ok;
        };

        if move_set.is_empty() {
            let msg = conf.lang.invalid_arguments();
            conf.append_output(who, &msg);
            return CommandResult::Ok;
        }

        let move_ids: Vec<UserId> = move_set.iter().collect();

        for &move_id in &move_ids {
            let move_idx = usize::from(move_id.0);
            if let Some(Some(user)) = conf.users.get_mut(move_idx) {
                user.master = target_id;
            }
        }

        // Send "you have been passed" to target.
        let passed = conf.lang.you_are_passed();
        conf.append_output(target_id, &passed);
        conf.flush_output(target_id);

        // Send WHO-like listing of moved users to target.
        let mut listing = String::new();
        for &move_id in &move_ids {
            conf.format_who_entry(move_id, target_id, &mut listing);
        }
        if !listing.is_empty() {
            conf.append_output(target_id, &listing);
            conf.flush_output(target_id);
        }

        // Send "talking with" to each moved user.
        for &move_id in &move_ids {
            let target_name = conf
                .users
                .get(target_idx)
                .and_then(|s| s.as_ref())
                .map(|u| u.name.clone())
                .unwrap_or_default();
            let talking = conf.lang.talking_with(target_id, &target_name);
            conf.append_output(move_id, "\n");
            conf.append_output(move_id, &talking);
            conf.flush_output(move_id);
        }
    }

    let done = conf.lang.done();
    conf.append_output(who, &done);
    CommandResult::Ok
}

pub fn cmd_enable(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    let is_normal = {
        let Some(Some(user)) = conf.users.get_mut(idx) else {
            return CommandResult::Ok;
        };
        match &mut user.role {
            Role::Master {
                ref mut tell_all_enabled,
            }
            | Role::Submaster {
                ref mut tell_all_enabled,
            } => {
                *tell_all_enabled = true;
                false
            }
            Role::Normal => true,
        }
    };

    if is_normal {
        let msg = conf.lang.command_error();
        conf.append_output(who, &msg);
    } else {
        let done = conf.lang.done();
        conf.append_output(who, &done);
    }
    CommandResult::Ok
}

pub fn cmd_disable(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    let is_normal = {
        let Some(Some(user)) = conf.users.get_mut(idx) else {
            return CommandResult::Ok;
        };
        match &mut user.role {
            Role::Master {
                ref mut tell_all_enabled,
            }
            | Role::Submaster {
                ref mut tell_all_enabled,
            } => {
                *tell_all_enabled = false;
                false
            }
            Role::Normal => true,
        }
    };

    if is_normal {
        let msg = conf.lang.command_error();
        conf.append_output(who, &msg);
    } else {
        let done = conf.lang.done();
        conf.append_output(who, &done);
    }
    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn make_tty_promotes_user() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Alpha");

        tc.input(master, "mt 1");
        let master_out = tc.output(master);
        assert!(master_out.contains("Done"), "expected Done: {master_out}");

        let user1_out = tc.output(user1);
        assert!(
            user1_out.contains("master"),
            "expected 'master' notification: {user1_out}"
        );
    }

    #[test]
    fn kill_removes_user() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "ki 1");
        let out = tc.output(master);

        assert!(out.contains("Done"), "expected Done: {out}");
        assert!(
            out.contains("killed") || out.contains("Wanderer"),
            "expected kill notification: {out}"
        );
    }

    #[test]
    fn kill_adds_to_left_list() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "ki 1");
        tc.drain(master);

        tc.input(master, "left");
        let out = tc.output(master);

        assert!(out.contains("Wanderer"), "expected Wanderer in left: {out}");
        assert!(out.contains("K"), "expected K marker for killed: {out}");
    }

    #[test]
    fn enable_disable_toggle() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "en");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done for enable: {out}");

        tc.input(master, "di");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done for disable: {out}");
    }

    #[test]
    fn give_transfers_users() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Alpha");
        let _user2 = tc.connect("Bravo");
        let _user3 = tc.connect("Charlie");

        // Make Alpha a submaster.
        tc.input(master, "mt 1");
        tc.drain(master);
        tc.drain(user1);

        // Give users 2,3 to Alpha.
        tc.input(master, "gi 1;2,3");
        let master_out = tc.output(master);
        assert!(
            master_out.contains("Done"),
            "expected Done for give: {master_out}"
        );

        let user1_out = tc.output(user1);
        assert!(
            user1_out.contains("passed"),
            "expected 'passed' notification: {user1_out}"
        );
    }

    #[test]
    fn normalize_demotes_submaster() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Alpha");
        let _user2 = tc.connect("Bravo");

        // Make Alpha a submaster.
        tc.input(master, "mt 1");
        tc.drain(master);
        tc.drain(user1);

        // Give user 2 to Alpha.
        tc.input(master, "gi 1;2");
        tc.drain(master);
        tc.drain(user1);

        // Normalize Alpha.
        tc.input(master, "no 1");
        let master_out = tc.output(master);
        assert!(
            master_out.contains("Done"),
            "expected Done for normalize: {master_out}"
        );

        let user1_out = tc.output(user1);
        assert!(
            user1_out.contains("no longer") || user1_out.contains("Normal"),
            "expected demotion notification: {user1_out}"
        );
    }

    #[test]
    fn normalize_bare_demotes_all_submasters() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Alpha");
        let _user2 = tc.connect("Bravo");

        // Make Alpha a submaster.
        tc.input(master, "mt 1");
        tc.drain(master);
        tc.drain(user1);

        // Give user 2 to Alpha.
        tc.input(master, "gi 1;2");
        tc.drain(master);
        tc.drain(user1);

        // Bare normalize — demote all.
        tc.input(master, "no");
        let master_out = tc.output(master);
        assert!(
            master_out.contains("Done"),
            "expected Done for bare normalize: {master_out}"
        );
    }

    #[test]
    fn give_new_sets_new_user_master() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Alpha");

        // Make Alpha a submaster.
        tc.input(master, "mt 1");
        tc.drain(master);
        tc.drain(user1);

        // Give new users to Alpha.
        tc.input(master, "gi 1;new");
        let master_out = tc.output(master);
        assert!(
            master_out.contains("Done"),
            "expected Done for gi new: {master_out}"
        );

        let user1_out = tc.output(user1);
        assert!(
            user1_out.contains("passed"),
            "expected 'passed' notification: {user1_out}"
        );
    }
}
