//! Handlers for read-only server state queries: info, clock, time,
//! runtime, version.

use std::time::Instant;

use chrono::Utc;

use super::{super::Conference, CommandResult};
use crate::users::UserId;

/// Version string for the Rust reimplementation.
const VERSION: &str = "2.0r";

/// Server name (placeholder).
const SERVER_NAME: &str = "xcalr";

/// INFO command — full server status: time, users, resources.
pub fn cmd_info(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    output_time_section(conf, who);

    let users = u16::try_from(conf.current_count).unwrap_or(u16::MAX);
    let max = u16::try_from(conf.peak_count).unwrap_or(u16::MAX);
    let info_msg = conf.lang.info(users, max, SERVER_NAME, VERSION);
    conf.append_output(who, &info_msg);

    let core_msg = conf.lang.core_size("0");
    conf.append_output(who, &core_msg);

    let cru_msg = conf.lang.cru_now("0.000", "-1");
    conf.append_output(who, &cru_msg);

    CommandResult::Ok
}

/// CLOCK command — display current time.
pub fn cmd_clock(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    // C: cmd_clock writes "\r\n" before the clock message.
    conf.append_output(who, "\n");
    let msg = conf.lang.clock_time(Utc::now());
    conf.append_output(who, &msg);
    CommandResult::Ok
}

/// TIME command — display uptime, current time, and time remaining.
pub fn cmd_time(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    output_time_section(conf, who);
    CommandResult::Ok
}

/// RUNTIME command — display CRU (CPU Resource Unit) usage.
pub fn cmd_runtime(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let cru_msg = conf.lang.cru_now("0.000", "-1");
    conf.append_output(who, &cru_msg);
    CommandResult::Ok
}

/// VERSION command — display server version.
pub fn cmd_version(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let msg = conf.lang.version(VERSION);
    conf.append_output(who, &msg);
    CommandResult::Ok
}

// ── Private helpers ─────────────────────────────────────────────────

/// Output the time section: uptime with date, current clock, time left.
fn output_time_section(conf: &mut Conference, who: UserId) {
    let now = Utc::now();
    let elapsed = Instant::now().saturating_duration_since(conf.started_at);
    let chrono_elapsed =
        chrono::Duration::from_std(elapsed).unwrap_or_default();
    let start = now.checked_sub_signed(chrono_elapsed).unwrap_or(now);

    let uptime_msg = conf.lang.uptime_with_date(start);
    conf.append_output(who, &uptime_msg);

    let clock_msg = conf.lang.clock_time(now);
    conf.append_output(who, &clock_msg);

    // Time left.
    let remaining = conf.ends_at.saturating_duration_since(Instant::now());
    let minutes_left =
        u32::try_from(remaining.as_secs().checked_div(60).unwrap_or(0))
            .unwrap_or(u32::MAX);
    let time_left_msg = conf.lang.time_left(minutes_left);
    conf.append_output(who, &time_left_msg);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn version_outputs_version_string() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "version");
        let out = tc.output(master);
        assert!(
            out.contains("Xcaliber II"),
            "expected version output: {out}"
        );
        assert!(out.contains("2.0r"), "expected version 2.0r: {out}");
    }

    #[test]
    fn clock_outputs_time_format() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "clock");
        let out = tc.output(master);
        assert!(out.contains("Time now:"), "expected 'Time now:': {out}");
        assert!(
            out.contains("a.m.") || out.contains("p.m."),
            "expected a.m./p.m.: {out}"
        );
    }

    #[test]
    fn time_outputs_all_fields() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "time");
        let out = tc.output(master);
        assert!(out.contains("Up at"), "expected 'Up at': {out}");
        assert!(out.contains("Time now:"), "expected 'Time now:': {out}");
        assert!(out.contains("minute(s)"), "expected 'minute(s)': {out}");
    }

    #[test]
    fn runtime_outputs_cru() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "runtime");
        let out = tc.output(master);
        assert!(out.contains("CRUs now:"), "expected CRU output: {out}");
    }

    #[test]
    fn info_outputs_all_sections() {
        let mut tc = TestConference::new();
        let master = tc.connect("Tester");
        tc.input(master, "info");
        let out = tc.output(master);
        assert!(out.contains("Up at"), "expected uptime: {out}");
        assert!(out.contains("Time now:"), "expected clock: {out}");
        assert!(out.contains("minute(s)"), "expected time left: {out}");
        assert!(out.contains("Users:"), "expected user info: {out}");
        assert!(out.contains("Server:"), "expected server info: {out}");
        assert!(out.contains("Core size:"), "expected core size: {out}");
        assert!(out.contains("CRUs now:"), "expected CRU info: {out}");
    }
}
