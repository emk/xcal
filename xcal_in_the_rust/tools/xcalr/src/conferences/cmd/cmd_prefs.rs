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
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_tell_all = true;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept_alls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_tell_all = false;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_accept_notify(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_notifications = false;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_notify(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_notifications = true;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_ports(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_ports = true;
    }
    let msg = conf.lang.rejecting_ports();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_reject_controls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_controls = true;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

pub fn cmd_okay_controls(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_controls = false;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

/// No-op: xcalr uses raw TCP without telnet, so there is no IAC BRK
/// signal to reject. The flag is stored for protocol compatibility but
/// has no effect.
pub fn cmd_reject_breaks(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_break = true;
    }
    let msg = conf.lang.done();
    conf.append_output(who, &msg);
    CommandResult::Ok
}

/// No-op: see [`cmd_reject_breaks`].
pub fn cmd_accept_breaks(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let idx = usize::from(who.0);
    if let Some(Some(user)) = conf.users.get_mut(idx) {
        user.prefs.reject_break = false;
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
    fn reject_alls_outputs_done() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "ra");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");
    }

    #[test]
    fn accept_alls_outputs_done() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "aa");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");
    }

    #[test]
    fn reject_alls_sets_r_flag() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");

        // WHO before RA: no R flag
        tc.input(master, "who");
        let before = tc.output(master);
        assert!(!before.contains("R"), "R flag should not be set: {before}");

        // RA sets R flag
        tc.input(master, "ra");
        tc.drain(master);
        tc.input(master, "who");
        let after = tc.output(master);
        assert!(after.contains("R"), "R flag should be set: {after}");
    }

    #[test]
    fn accept_alls_clears_r_flag() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");

        tc.input(master, "ra");
        tc.drain(master);
        tc.input(master, "aa");
        tc.drain(master);
        tc.input(master, "who");
        let out = tc.output(master);
        assert!(!out.contains("R"), "R flag should be cleared: {out}");
    }

    #[test]
    fn reject_notify_outputs_done() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "rn");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");
    }

    #[test]
    fn accept_notify_outputs_done() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "an");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");
    }

    #[test]
    fn reject_notify_clears_n_flag() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");

        // Default: N flag is set (reject_notifications=false)
        tc.input(master, "who");
        let before = tc.output(master);
        assert!(
            before.contains("N"),
            "N flag should be set by default: {before}"
        );

        // RN clears N flag
        tc.input(master, "rn");
        tc.drain(master);
        tc.input(master, "who");
        let after = tc.output(master);
        assert!(!after.contains("N"), "N flag should be cleared: {after}");
    }

    #[test]
    fn reject_ports_outputs_rejecting() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "rp");
        let out = tc.output(master);
        assert!(
            out.contains("Rejecting ports"),
            "expected 'Rejecting ports': {out}"
        );
    }

    #[test]
    fn reject_ports_sets_p_flag() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");

        tc.input(master, "rp");
        tc.drain(master);
        tc.input(master, "who");
        let out = tc.output(master);
        assert!(out.contains("P"), "P flag should be set: {out}");
    }

    #[test]
    fn okay_controls_clears_c_flag() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");

        // Default: C flag is set (reject_controls=true from login)
        tc.input(master, "who");
        let before = tc.output(master);
        assert!(
            before.contains("CN"),
            "C flag should be set by default: {before}"
        );

        // OC clears C flag
        tc.input(master, "oc");
        tc.drain(master);
        tc.input(master, "who");
        let after = tc.output(master);
        assert!(!after.contains("CN"), "C flag should be cleared: {after}");
    }

    #[test]
    fn reject_controls_re_enables_c_flag() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");

        tc.input(master, "oc");
        tc.drain(master);
        tc.input(master, "rc");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");

        tc.input(master, "who");
        let who_out = tc.output(master);
        assert!(
            who_out.contains("CN"),
            "C flag should be re-enabled: {who_out}"
        );
    }

    #[test]
    fn reject_breaks_outputs_done() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "rb");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");
    }

    #[test]
    fn accept_breaks_outputs_done() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "ab");
        let out = tc.output(master);
        assert!(out.contains("Done"), "expected Done: {out}");
    }
}
