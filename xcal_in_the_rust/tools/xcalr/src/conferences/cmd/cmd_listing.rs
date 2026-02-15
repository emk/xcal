//! Handlers for user-listing commands: who, port, everything, tty, left,
//! users, below, extended who.

use std::fmt::Write as _;

use chrono::{DateTime, Utc};

use super::{
    super::{Conference, WhoColumns},
    CommandResult,
};
use crate::users::{Role, UserId};

pub fn cmd_who(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let text = conf.format_who(who);
    conf.append_output(who, &text);
    CommandResult::Ok
}

pub fn cmd_tty(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let mut line = String::new();
    conf.format_who_entry(who, who, &mut line);
    conf.append_output(who, &line);
    CommandResult::Ok
}

pub fn cmd_users(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    #[allow(clippy::expect_used)]
    let count =
        u16::try_from(conf.current_count).expect("current_count fits u16");
    let msg = conf.lang.user_count(count);
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_port(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    // Check if the requester has reject_ports enabled.
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get(idx) {
        if user.prefs.reject_ports {
            let msg = conf.lang.cant_port();
            conf.append_output(who, &msg);
            return CommandResult::Ok;
        }
    }

    let all_ids: Vec<UserId> = conf
        .users
        .iter()
        .filter_map(|s| s.as_ref())
        .map(|u| u.id)
        .collect();

    let mut output = String::from("\n");
    for uid in all_ids {
        if !conf.is_in_subconference(uid, who) {
            continue;
        }
        conf.format_who_entry_ex(uid, who, WhoColumns::PortName, &mut output);
    }

    conf.append_output(who, &output);
    CommandResult::Ok
}

pub fn cmd_everything(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let all_ids: Vec<UserId> = conf
        .users
        .iter()
        .filter_map(|s| s.as_ref())
        .map(|u| u.id)
        .collect();

    let mut output = String::from("\n");
    for uid in all_ids {
        conf.format_who_entry_ex(uid, who, WhoColumns::Everything, &mut output);
    }

    conf.append_output(who, &output);
    CommandResult::Ok
}

pub fn cmd_below(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);

    // Check that the caller is master or submaster.
    let is_master_or_sub = conf
        .users
        .get(idx)
        .and_then(|s| s.as_ref())
        .is_some_and(|u| {
            matches!(u.role, Role::Master { .. } | Role::Submaster { .. })
        });

    if !is_master_or_sub {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let below_ids: Vec<UserId> = conf
        .users
        .iter()
        .filter_map(|s| s.as_ref())
        .filter(|u| u.master == who || u.id == who)
        .map(|u| u.id)
        .collect();

    let mut output = String::from("\n");
    for uid in below_ids {
        conf.format_who_entry(uid, who, &mut output);
    }

    conf.append_output(who, &output);
    CommandResult::Ok
}

pub fn cmd_left(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    if conf.left.is_empty() {
        let msg = conf.lang.no_one_left();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    // C: cmd_left outputs upat (conference start time) first.
    let now = Utc::now();
    let elapsed = conf.started_at.elapsed();
    let chrono_elapsed = chrono::Duration::from_std(elapsed).unwrap_or_default();
    let start = now.checked_sub_signed(chrono_elapsed).unwrap_or(now);
    let uptime = conf.lang.uptime(start);

    #[allow(clippy::expect_used)]
    let count = u16::try_from(conf.left.len()).expect("left count fits u16");
    let count_msg = conf.lang.left_count(count);

    let header = conf.lang.left_header();

    let mut output = String::new();
    output.push_str(&uptime);
    output.push_str(&count_msg);

    // C: aligns lefthdr so the first word lines up with the 5-char
    // "HH:MM" time column. Find position of first space; if < 5,
    // prepend spaces to reach 5.
    let first_space = header.find(' ').unwrap_or(header.len());
    if first_space < 5 {
        for _ in 0..5_usize.saturating_sub(first_space) {
            output.push(' ');
        }
    }
    output.push_str(&header);

    for left_user in &conf.left {
        let dt: DateTime<Utc> = left_user.departed_at.into();
        let entry = conf.lang.left_entry(dt, left_user.was_killed, &left_user.name);
        let _ = writeln!(output, "{entry}");
    }

    conf.append_output(who, &output);
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

    #[test]
    fn tty_shows_own_line() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "tty");
        let out = tc.output(master);

        assert!(
            out.contains("=>"),
            "expected => marker in tty output: {out}"
        );
        assert!(
            out.contains("[#0]"),
            "expected user number in tty output: {out}"
        );
        assert!(
            out.contains("Explorer"),
            "expected name in tty output: {out}"
        );
        assert!(
            out.contains("M"),
            "expected master role in tty output: {out}"
        );
        assert!(
            out.contains("COM"),
            "expected COM state in tty output: {out}"
        );
    }

    #[test]
    fn users_shows_count() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "users");
        let out = tc.output(master);

        assert!(
            out.contains("2"),
            "expected count of 2 in users output: {out}"
        );
        assert!(
            out.contains("user"),
            "expected 'user' in users output: {out}"
        );
    }

    #[test]
    fn number_shows_count() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "number");
        let out = tc.output(master);

        assert!(
            out.contains("1"),
            "expected count of 1 in number output: {out}"
        );
    }

    #[test]
    fn port_shows_location() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "port");
        let out = tc.output(master);

        assert!(
            out.contains("Explorer"),
            "expected name in port output: {out}"
        );
        assert!(
            out.contains("Wanderer"),
            "expected second user in port output: {out}"
        );
        assert!(
            out.contains("=>"),
            "expected => marker in port output: {out}"
        );
    }

    #[test]
    fn port_rejects_when_rp_set() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "rp");
        tc.drain(master);
        tc.input(master, "port");
        let out = tc.output(master);

        assert!(
            out.contains("Rejecting") || out.contains("reject"),
            "expected rejecting ports message: {out}"
        );
    }

    #[test]
    fn everything_shows_all_users() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "everything");
        let out = tc.output(master);

        assert!(
            out.contains("Explorer"),
            "expected master in everything: {out}"
        );
        assert!(
            out.contains("Wanderer"),
            "expected user in everything: {out}"
        );
        assert!(out.contains("M"), "expected role in everything: {out}");
    }

    #[test]
    fn a_alias_for_everything() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "a");
        let out = tc.output(master);

        assert!(
            out.contains("Explorer"),
            "expected 'a' to dispatch as everything: {out}"
        );
    }

    #[test]
    fn left_empty() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "left");
        let out = tc.output(master);

        assert!(
            out.contains("No one has left") || out.contains("no one"),
            "expected no-one-left message: {out}"
        );
    }
}
