//! Handlers for user-listing commands: who, port, everything, tty, left,
//! users, below, extended who.

use std::{
    fmt::Write as _,
    time::{SystemTime, UNIX_EPOCH},
};

use super::{
    super::{Conference, WhoColumns},
    CommandResult,
};
use crate::{
    lang::AmPm,
    users::{Role, UserId},
};

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
        .filter(|u| u.master == who)
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
    let start_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(conf.started_at.elapsed().as_secs());
    let start_ct = epoch_to_12h(start_epoch);
    let ct = start_ct.as_clock_time();
    let uptime = conf.lang.uptime(&ct);

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
        // C: uses absolute time-of-day (localtime) not elapsed time.
        let epoch_secs = left_user
            .departed_at
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let day_secs = epoch_secs.checked_rem(86400).unwrap_or(0);
        let hour_24 =
            u32::try_from(day_secs.checked_div(3600).unwrap_or(0)).unwrap_or(0);
        let mins = u32::try_from(
            day_secs
                .checked_rem(3600)
                .unwrap_or(0)
                .checked_div(60)
                .unwrap_or(0),
        )
        .unwrap_or(0);
        let hour_12 = match hour_24 {
            0 => 12,
            h @ 1..=12 => h,
            h => h.saturating_sub(12),
        };

        let k = if left_user.was_killed { "K" } else { " " };

        let _ = writeln!(
            output,
            "{hour_12:02}:{mins:02}{k} {name}",
            name = left_user.name
        );
    }

    conf.append_output(who, &output);
    CommandResult::Ok
}

/// Owned 12-hour clock components, computed from epoch seconds.
struct Clock12 {
    hours: String,
    minutes: String,
    seconds: String,
    ampm: AmPm,
}

impl Clock12 {
    fn as_clock_time(&self) -> crate::lang::ClockTime<'_> {
        crate::lang::ClockTime {
            hours: &self.hours,
            minutes: &self.minutes,
            seconds: &self.seconds,
            ampm: self.ampm,
        }
    }
}

/// Convert epoch seconds to owned 12-hour clock components.
fn epoch_to_12h(epoch_secs: u64) -> Clock12 {
    let day_secs = epoch_secs.checked_rem(86400).unwrap_or(0);
    let hour =
        u32::try_from(day_secs.checked_div(3600).unwrap_or(0)).unwrap_or(0);
    let minute = u32::try_from(
        day_secs
            .checked_rem(3600)
            .unwrap_or(0)
            .checked_div(60)
            .unwrap_or(0),
    )
    .unwrap_or(0);
    let second =
        u32::try_from(day_secs.checked_rem(60).unwrap_or(0)).unwrap_or(0);

    let (h12, ampm) = match hour {
        0 => (12, AmPm::Am),
        h @ 1..=11 => (h, AmPm::Am),
        12 => (12, AmPm::Pm),
        h => (h.saturating_sub(12), AmPm::Pm),
    };

    Clock12 {
        // C upat uses %2d — right-justify hour in 2 chars.
        hours: format!("{h12:2}"),
        minutes: format!("{minute:02}"),
        seconds: format!("{second:02}"),
        ampm,
    }
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
