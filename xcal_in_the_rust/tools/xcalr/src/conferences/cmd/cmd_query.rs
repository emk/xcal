//! Handlers for read-only server state queries: info, clock, time,
//! runtime, version.

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use super::{super::Conference, CommandResult};
use crate::{
    lang::{AmPm, ClockTime},
    users::UserId,
};

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
    let current = UtcDateTime::now();
    output_clock_time(conf, who, &current);
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
    let now_instant = Instant::now();
    let elapsed = now_instant.saturating_duration_since(conf.started_at);
    let current_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let start_epoch = current_epoch.saturating_sub(elapsed.as_secs());

    // Uptime with date.
    let start = UtcDateTime::from_epoch_secs(start_epoch);
    let start_ct = start.to_clock_time();
    let date_str = start.format_date();
    let uptime_msg = conf
        .lang
        .uptime_with_date(&start_ct.to_lang_clock_time(), &date_str);
    conf.append_output(who, &uptime_msg);

    // Current clock time.
    let current = UtcDateTime::from_epoch_secs(current_epoch);
    output_clock_time(conf, who, &current);

    // Time left.
    let remaining = conf.ends_at.saturating_duration_since(now_instant);
    let minutes_left =
        u32::try_from(remaining.as_secs().checked_div(60).unwrap_or(0))
            .unwrap_or(u32::MAX);
    let time_left_msg = conf.lang.time_left(minutes_left);
    conf.append_output(who, &time_left_msg);
}

/// Output a formatted clock time line.
fn output_clock_time(conf: &mut Conference, who: UserId, utc: &UtcDateTime) {
    let ct = utc.to_clock_time();
    let msg = conf.lang.clock_time(&ct.to_lang_clock_time());
    conf.append_output(who, &msg);
}

// ── UTC date/time conversion ────────────────────────────────────────

/// Formatted 12-hour clock components (owned strings).
struct FormattedClock {
    hours: String,
    minutes: String,
    seconds: String,
    ampm: AmPm,
}

impl FormattedClock {
    /// Borrow as a [`ClockTime`] for passing to lang methods.
    fn to_lang_clock_time(&self) -> ClockTime<'_> {
        ClockTime {
            hours: &self.hours,
            minutes: &self.minutes,
            seconds: &self.seconds,
            ampm: self.ampm,
        }
    }
}

/// UTC date and time components, computed from epoch seconds.
struct UtcDateTime {
    year: i32,
    month: u32,
    day: u32,
    hour: u32,
    minute: u32,
    second: u32,
}

impl UtcDateTime {
    /// Get the current UTC date and time.
    fn now() -> Self {
        let epoch_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::from_epoch_secs(epoch_secs)
    }

    /// Convert epoch seconds to UTC date and time components.
    ///
    /// All `checked_div`/`checked_rem` calls use known non-zero constant
    /// divisors; the `unwrap_or` fallbacks can never activate.
    fn from_epoch_secs(epoch_secs: u64) -> Self {
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

        let total_days =
            i64::try_from(epoch_secs.checked_div(86400).unwrap_or(0))
                .unwrap_or(0);
        let (year, month, day) = civil_from_days(total_days);

        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Convert 24-hour time to 12-hour with AM/PM.
    fn hour_12(&self) -> (u32, AmPm) {
        match self.hour {
            0 => (12, AmPm::Am),
            h @ 1..=11 => (h, AmPm::Am),
            12 => (12, AmPm::Pm),
            h => (h.saturating_sub(12), AmPm::Pm),
        }
    }

    /// Format as owned 12-hour clock strings.
    fn to_clock_time(&self) -> FormattedClock {
        let (h, ampm) = self.hour_12();
        FormattedClock {
            hours: format!("{h:2}"),
            minutes: format!("{:02}", self.minute),
            seconds: format!("{:02}", self.second),
            ampm,
        }
    }

    /// Format date as `M/D/YY`.
    fn format_date(&self) -> String {
        let short_year = self.year.checked_rem(100).unwrap_or(0);
        format!("{}/{}/{:02}", self.month, self.day, short_year)
    }
}

/// Convert days since 1970-01-01 to (year, month, day) in the civil
/// calendar.
///
/// Implements Howard Hinnant's `civil_from_days` algorithm. All
/// intermediate values are in known bounded ranges for dates after 1970.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    let z = z.saturating_add(719468);
    let era = if z >= 0 {
        z.checked_div(146097).unwrap_or(0)
    } else {
        z.saturating_sub(146096).checked_div(146097).unwrap_or(0)
    };
    let doe = z.saturating_sub(era.saturating_mul(146097));
    let yoe = doe
        .saturating_sub(doe.checked_div(1460).unwrap_or(0))
        .saturating_add(doe.checked_div(36524).unwrap_or(0))
        .saturating_sub(doe.checked_div(146096).unwrap_or(0))
        .checked_div(365)
        .unwrap_or(0);
    let y = yoe.saturating_add(era.saturating_mul(400));
    let doy = doe.saturating_sub(
        yoe.saturating_mul(365)
            .saturating_add(yoe.checked_div(4).unwrap_or(0))
            .saturating_sub(yoe.checked_div(100).unwrap_or(0)),
    );
    let mp = doy
        .saturating_mul(5)
        .saturating_add(2)
        .checked_div(153)
        .unwrap_or(0);
    let d = doy
        .saturating_sub(
            mp.saturating_mul(153)
                .saturating_add(2)
                .checked_div(5)
                .unwrap_or(0),
        )
        .saturating_add(1);
    let m = if mp < 10 {
        mp.saturating_add(3)
    } else {
        mp.saturating_sub(9)
    };
    let y = if m <= 2 { y.saturating_add(1) } else { y };

    (
        i32::try_from(y).unwrap_or(0),
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
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

    // ── Calendar conversion tests ───────────────────────────────────

    #[test]
    fn civil_from_days_epoch() {
        let (y, m, d) = civil_from_days(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn civil_from_days_known_date() {
        // 2000-03-01 is 11017 days after the Unix epoch.
        let (y, m, d) = civil_from_days(11017);
        assert_eq!((y, m, d), (2000, 3, 1));
    }

    #[test]
    fn civil_from_days_leap_day() {
        // 2000-02-29 is 11016 days after the Unix epoch.
        let (y, m, d) = civil_from_days(11016);
        assert_eq!((y, m, d), (2000, 2, 29));
    }

    #[test]
    fn utc_from_epoch_midnight() {
        let utc = UtcDateTime::from_epoch_secs(0);
        assert_eq!(utc.year, 1970);
        assert_eq!(utc.month, 1);
        assert_eq!(utc.day, 1);
        assert_eq!(utc.hour, 0);
        assert_eq!(utc.minute, 0);
        assert_eq!(utc.second, 0);
    }

    #[test]
    fn hour_12_midnight() {
        let midnight = UtcDateTime::from_epoch_secs(0);
        assert_eq!(midnight.hour_12(), (12, AmPm::Am));
    }

    #[test]
    fn hour_12_noon() {
        // 12:00 UTC = 43200 seconds.
        let noon = UtcDateTime::from_epoch_secs(43200);
        assert_eq!(noon.hour_12(), (12, AmPm::Pm));
    }

    #[test]
    fn hour_12_afternoon() {
        // 13:00 UTC = 46800 seconds.
        let one_pm = UtcDateTime::from_epoch_secs(46800);
        assert_eq!(one_pm.hour_12(), (1, AmPm::Pm));
    }

    #[test]
    fn format_date_standard() {
        let utc = UtcDateTime {
            year: 2026,
            month: 2,
            day: 14,
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert_eq!(utc.format_date(), "2/14/26");
    }

    #[test]
    fn format_date_zero_padded_year() {
        let utc = UtcDateTime {
            year: 2005,
            month: 1,
            day: 1,
            hour: 0,
            minute: 0,
            second: 0,
        };
        assert_eq!(utc.format_date(), "1/1/05");
    }
}
