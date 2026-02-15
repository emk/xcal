//! Localized message formatting for Xcaliber.
//!
//! Messages are stored in Project Fluent `.ftl` files. The Fluent text contains
//! human-readable content only; wire framing (`\n` prefix/suffix, BEL bytes) is
//! applied by this module. The transport layer converts `\n` to `\r\n`.

#![allow(dead_code)]

use chrono::{DateTime, Datelike, Timelike, Utc};
use fluent_bundle::{FluentArgs, FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

use crate::users::UserId;

/// Time unit for time warnings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Second,
    Minute,
}

/// All 65 message keys, matching the original C identifiers from `lang/*.lf`.
#[allow(missing_docs)]
mod keys {
    // Connection
    pub const CONFULL: &str = "confull";
    pub const CONTERM: &str = "conterm";
    pub const WELCOME: &str = "welcome";
    pub const ENTNAME: &str = "entname";
    pub const MWELC: &str = "mwelc";
    pub const INTRO: &str = "intro";
    pub const TLKWITH: &str = "tlkwith";

    // User notifications
    pub const NEWUSER: &str = "newuser";
    pub const DONEMSG: &str = "donemsg";
    pub const MSGFROM: &str = "msgfrom";
    pub const ALLFROM: &str = "allfrom";
    pub const NAMENOT: &str = "namenot";
    pub const EXNOT: &str = "exnot";
    pub const KILLNOT: &str = "killnot";

    // Composing
    pub const SPEAK: &str = "speak";
    pub const OUTMSG: &str = "outmsg";
    pub const YOUOUT: &str = "youout";
    pub const IGMSG: &str = "igmsg";
    pub const DISCMSG: &str = "discmsg";
    pub const NORCPT: &str = "norcpt";
    pub const SENTMSG: &str = "sentmsg";
    pub const MNSMSG: &str = "mnsmsg";
    pub const EMPTY: &str = "empty";

    // Errors
    pub const CMDERR: &str = "cmderr";
    pub const FMTERR: &str = "fmterr";
    pub const INVARG: &str = "invarg";
    pub const UNIMP: &str = "unimp";
    pub const BADCHAR: &str = "badchar";

    // Prompts
    pub const NEWWARN: &str = "newwarn";
    pub const NEWNAME: &str = "newname";
    pub const NOLEFT: &str = "noleft";
    pub const NOMSGS: &str = "nomsgs";
    pub const NOALLS: &str = "noalls";
    pub const LINMSG: &str = "linmsg";

    // Ports/bounce
    pub const NOPORT: &str = "noport";
    pub const RPMSG: &str = "rpmsg";
    pub const BOUNCED: &str = "bounced";
    pub const BLFULL: &str = "blfull";
    pub const NBPORT: &str = "nbport";

    // Master transfer
    pub const NOWMAST: &str = "nowmast";
    pub const NORMAL: &str = "normal";
    pub const PASSED: &str = "passed";
    pub const MPTMMSG: &str = "mptmmsg";
    pub const PNUMSG: &str = "pnumsg";

    // Time
    pub const TIMEWRN: &str = "timewrn";
    pub const CLKMSG: &str = "clkmsg";
    pub const TIMELFT: &str = "timelft";

    // Help/explain
    pub const CANTEXP: &str = "cantexp";
    pub const FNFMSG: &str = "fnfmsg";
    pub const HELPTOP: &str = "helptop";

    // Left/users/info
    pub const LEFTMSG: &str = "leftmsg";
    pub const LEFTHDR: &str = "lefthdr";
    pub const LEFTENTRY: &str = "leftentry";
    pub const UPAT: &str = "upat";
    pub const UPAT2: &str = "upat2";
    pub const NUMUSRS: &str = "numusrs";
    pub const INFOMSG: &str = "infomsg";
    pub const XCALVER: &str = "xcalver";

    // Language
    pub const NEWLANG: &str = "newlang";
    pub const NOLANG: &str = "nolang";
    pub const CURLANG: &str = "curlang";

    // Resources
    pub const CRUNOW: &str = "crunow";
    pub const CORESIZE: &str = "coresize";

    // Telnet
    pub const AYTMSG: &str = "aytmsg";
    pub const BRKMSG: &str = "brkmsg";

    /// All 65 message keys for validation.
    pub const ALL: &[&str] = &[
        CONFULL, CONTERM, WELCOME, ENTNAME, MWELC, INTRO, TLKWITH, NEWUSER,
        DONEMSG, MSGFROM, ALLFROM, NAMENOT, EXNOT, KILLNOT, SPEAK, OUTMSG,
        YOUOUT, IGMSG, DISCMSG, NORCPT, SENTMSG, MNSMSG, EMPTY, CMDERR, FMTERR,
        INVARG, UNIMP, BADCHAR, NEWWARN, NEWNAME, NOLEFT, NOMSGS, NOALLS,
        LINMSG, NOPORT, RPMSG, BOUNCED, BLFULL, NBPORT, NOWMAST, NORMAL,
        PASSED, MPTMMSG, PNUMSG, TIMEWRN, CLKMSG, TIMELFT, CANTEXP, FNFMSG,
        HELPTOP, LEFTMSG, LEFTHDR, LEFTENTRY, UPAT, UPAT2, NUMUSRS, INFOMSG,
        XCALVER, NEWLANG, NOLANG, CURLANG, CRUNOW, CORESIZE, AYTMSG, BRKMSG,
    ];
}

/// Returns the wire framing `(prefix, suffix)` for a message key.
///
/// Protocol-level framing is the same across all locales:
/// - **Wrapped** `("\n", "\n")`: most messages (default)
/// - **Trailing** `("", "\n")`: confirmations, status lines
/// - **Leading** `("\n", "")`: prompts that expect user input
/// - **Bare** `("", "")`: data values, multiline displays
/// - **BEL** `("\n\x07\x07", "\n")`: `timewrn` only
fn framing(key: &str) -> (&'static str, &'static str) {
    match key {
        // Bare (6)
        keys::ENTNAME
        | keys::WELCOME
        | keys::MWELC
        | keys::HELPTOP
        | keys::INFOMSG
        | keys::LEFTENTRY => ("", ""),

        // Leading (6)
        keys::YOUOUT
        | keys::NEWWARN
        | keys::NEWNAME
        | keys::LINMSG
        | keys::NEWLANG
        | keys::CORESIZE => ("\n", ""),

        // Trailing (12)
        keys::CONFULL
        | keys::TLKWITH
        | keys::SENTMSG
        | keys::MNSMSG
        | keys::BOUNCED
        | keys::LEFTMSG
        | keys::LEFTHDR
        | keys::CLKMSG
        | keys::TIMELFT
        | keys::AYTMSG
        | keys::BRKMSG
        | keys::PNUMSG => ("", "\n"),

        // BEL (1)
        keys::TIMEWRN => ("\n\x07\x07", "\n"),

        // Wrapped (40) — default
        _ => ("\n", "\n"),
    }
}

/// Decompose a `DateTime` into `FluentArgs` for clock display.
fn datetime_clock_args(dt: DateTime<Utc>) -> FluentArgs<'static> {
    let (is_pm, h12) = dt.hour12();
    let mut args = FluentArgs::new();
    args.set("hours", format!("{h12:2}"));
    args.set("minutes", format!("{:02}", dt.minute()));
    args.set("seconds", format!("{:02}", dt.second()));
    args.set("ampm", if is_pm { "pm" } else { "am" });
    args
}

/// Generate a no-argument message method.
macro_rules! no_arg_message {
    ($(#[$meta:meta])* $method:ident, $key:expr) => {
        $(#[$meta])*
        pub fn $method(&self) -> String {
            self.format($key, None)
        }
    };
}

/// Generate a method taking `(UserId, &str)` for port+name messages.
macro_rules! port_name_message {
    ($(#[$meta:meta])* $method:ident, $key:expr) => {
        $(#[$meta])*
        pub fn $method(&self, id: UserId, name: &str) -> String {
            let mut args = FluentArgs::new();
            args.set("number", id.to_string());
            args.set("name", name);
            self.format($key, Some(&args))
        }
    };
}

/// Localized message formatter backed by Project Fluent.
///
/// Loads a `.ftl` resource at construction time and validates that all
/// expected message keys are present. Format methods apply wire framing
/// (newline prefix/suffix, BEL bytes) around the Fluent text content.
pub struct Lang {
    bundle: FluentBundle<FluentResource>,
}

impl Lang {
    /// Load the en-US message bundle and validate all keys.
    ///
    /// # Panics
    ///
    /// Panics if the embedded `.ftl` resource fails to parse or is missing
    /// any of the 64 expected message keys.
    pub fn new() -> Self {
        let ftl_string = include_str!("../locales/en-US/messages.ftl");
        let resource = FluentResource::try_new(ftl_string.to_string())
            .unwrap_or_else(|(_, errors)| {
                panic!("Failed to parse FTL: {errors:?}")
            });

        let langid: LanguageIdentifier = "en-US".parse().unwrap_or_else(|e| {
            panic!("Failed to parse language identifier: {e}")
        });

        let mut bundle = FluentBundle::new(vec![langid]);
        bundle.set_use_isolating(false);
        if let Err(errors) = bundle.add_resource(resource) {
            panic!("Failed to add FTL resource: {errors:?}");
        }

        for key in keys::ALL {
            assert!(bundle.has_message(key), "Missing message key: {key}");
        }

        Lang { bundle }
    }

    /// Format a message with wire framing applied.
    fn format(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        let (prefix, suffix) = framing(key);
        let text = self.format_raw(key, args);
        format!("{prefix}{text}{suffix}")
    }

    /// Format a message without wire framing.
    fn format_raw(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        let msg = self
            .bundle
            .get_message(key)
            .unwrap_or_else(|| panic!("Message not found: {key}"));
        let pattern = msg
            .value()
            .unwrap_or_else(|| panic!("Message has no value: {key}"));
        let mut errors = vec![];
        let result = self.bundle.format_pattern(pattern, args, &mut errors);
        assert!(errors.is_empty(), "Errors formatting {key}: {errors:?}");
        result.into_owned()
    }

    // ── No-arg methods (45) ──────────────────────────────────────────

    // Connection
    no_arg_message!(conference_full, keys::CONFULL);
    no_arg_message!(conference_terminated, keys::CONTERM);
    no_arg_message!(welcome, keys::WELCOME);
    no_arg_message!(enter_name, keys::ENTNAME);
    no_arg_message!(master_welcome, keys::MWELC);
    no_arg_message!(intro, keys::INTRO);

    // Composing
    no_arg_message!(done, keys::DONEMSG);
    no_arg_message!(speak, keys::SPEAK);
    no_arg_message!(users_out, keys::OUTMSG);
    no_arg_message!(you_are_out, keys::YOUOUT);
    no_arg_message!(users_ignoring, keys::IGMSG);
    no_arg_message!(disconnected, keys::DISCMSG);
    no_arg_message!(no_recipients, keys::NORCPT);
    no_arg_message!(message_sent, keys::SENTMSG);
    no_arg_message!(message_not_sent, keys::MNSMSG);
    no_arg_message!(empty_message, keys::EMPTY);

    // Errors
    no_arg_message!(command_error, keys::CMDERR);
    no_arg_message!(format_error, keys::FMTERR);
    no_arg_message!(invalid_arguments, keys::INVARG);
    no_arg_message!(not_implemented, keys::UNIMP);
    no_arg_message!(bad_character, keys::BADCHAR);

    // Prompts
    no_arg_message!(new_warning_prompt, keys::NEWWARN);
    no_arg_message!(new_name_prompt, keys::NEWNAME);
    no_arg_message!(no_one_left, keys::NOLEFT);
    no_arg_message!(no_messages, keys::NOMSGS);
    no_arg_message!(tell_all_disabled, keys::NOALLS);
    no_arg_message!(command_line_prompt, keys::LINMSG);

    // Ports/bounce
    no_arg_message!(cant_port, keys::NOPORT);
    no_arg_message!(rejecting_ports, keys::RPMSG);
    no_arg_message!(you_are_bounced, keys::BOUNCED);
    no_arg_message!(bounce_list_full, keys::BLFULL);
    no_arg_message!(no_bounced_ports, keys::NBPORT);

    // Master transfer
    no_arg_message!(now_master, keys::NOWMAST);
    no_arg_message!(no_longer_master, keys::NORMAL);
    no_arg_message!(you_are_passed, keys::PASSED);
    no_arg_message!(must_pass_to_master, keys::MPTMMSG);
    no_arg_message!(new_users_header, keys::PNUMSG);

    // Help/explain
    no_arg_message!(cant_explain, keys::CANTEXP);
    no_arg_message!(help_file_not_found, keys::FNFMSG);
    no_arg_message!(help_topic, keys::HELPTOP);

    // Left
    no_arg_message!(left_header, keys::LEFTHDR);

    // Language
    no_arg_message!(new_language_prompt, keys::NEWLANG);
    no_arg_message!(language_not_available, keys::NOLANG);

    // Telnet
    no_arg_message!(are_you_there, keys::AYTMSG);
    no_arg_message!(break_response, keys::BRKMSG);

    // ── Port+name methods (7) ────────────────────────────────────────

    port_name_message!(talking_with, keys::TLKWITH);
    port_name_message!(new_user, keys::NEWUSER);
    port_name_message!(message_from, keys::MSGFROM);
    port_name_message!(message_to_all_from, keys::ALLFROM);
    port_name_message!(name_changed, keys::NAMENOT);
    port_name_message!(user_exited, keys::EXNOT);
    port_name_message!(user_killed, keys::KILLNOT);

    // ── Hand-written methods (12) ────────────────────────────────────

    pub fn time_warning(&self, count: i64, unit: TimeUnit) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count);
        args.set(
            "unit",
            match unit {
                TimeUnit::Second => "second",
                TimeUnit::Minute => "minute",
            },
        );
        self.format(keys::TIMEWRN, Some(&args))
    }

    /// Format the current clock time.
    pub fn clock_time(&self, dt: DateTime<Utc>) -> String {
        let args = datetime_clock_args(dt);
        self.format(keys::CLKMSG, Some(&args))
    }

    pub fn time_left(&self, minutes: u32) -> String {
        let mut args = FluentArgs::new();
        args.set("minutes", minutes.to_string());
        self.format(keys::TIMELFT, Some(&args))
    }

    /// Format "Xcaliber up at ..." message.
    pub fn uptime(&self, dt: DateTime<Utc>) -> String {
        let args = datetime_clock_args(dt);
        self.format(keys::UPAT, Some(&args))
    }

    /// Format "Up at ... on M/D/YY" message.
    pub fn uptime_with_date(&self, dt: DateTime<Utc>) -> String {
        let mut args = datetime_clock_args(dt);
        let short_year = dt.year().checked_rem(100).unwrap_or(0);
        args.set(
            "date",
            format!("{}/{}/{:02}", dt.month(), dt.day(), short_year),
        );
        self.format(keys::UPAT2, Some(&args))
    }

    /// Format a single line in the LEFT listing.
    pub fn left_entry(
        &self,
        dt: DateTime<Utc>,
        was_killed: bool,
        name: &str,
    ) -> String {
        let (is_pm, h12) = dt.hour12();
        let _ = is_pm; // hour12() for 12h conversion; AM/PM not shown in left listing
        let mut args = FluentArgs::new();
        args.set("hours", format!("{h12:02}"));
        args.set("minutes", format!("{:02}", dt.minute()));
        args.set("killed", if was_killed { "K" } else { " " });
        args.set("name", name.to_string());
        self.format(keys::LEFTENTRY, Some(&args))
    }

    pub fn left_count(&self, count: u16) -> String {
        let mut args = FluentArgs::new();
        args.set("count", format!("{count:03}"));
        self.format(keys::LEFTMSG, Some(&args))
    }

    pub fn user_count(&self, count: u16) -> String {
        let mut args = FluentArgs::new();
        args.set("count", count.to_string());
        self.format(keys::NUMUSRS, Some(&args))
    }

    pub fn info(
        &self,
        users: u16,
        max: u16,
        server: &str,
        version: &str,
    ) -> String {
        let mut args = FluentArgs::new();
        args.set("users", users.to_string());
        args.set("max", max.to_string());
        args.set("server", server);
        args.set("version", version);
        self.format(keys::INFOMSG, Some(&args))
    }

    pub fn version(&self, version: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("version", version);
        self.format(keys::XCALVER, Some(&args))
    }

    pub fn current_language(&self, language: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("language", language);
        self.format(keys::CURLANG, Some(&args))
    }

    pub fn cru_now(&self, crus: &str, max: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("crus", crus);
        args.set("max", max);
        self.format(keys::CRUNOW, Some(&args))
    }

    pub fn core_size(&self, size: &str) -> String {
        let mut args = FluentArgs::new();
        args.set("size", size);
        self.format(keys::CORESIZE, Some(&args))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use chrono::TimeZone;

    use super::*;

    fn msgs() -> Lang {
        Lang::new()
    }

    #[test]
    fn all_keys_present() {
        let m = msgs();
        assert_eq!(keys::ALL.len(), 65);
        // Messages::new() already validates all keys exist in the bundle.
        // Verify we can at least resolve no-arg messages without error.
        let _ = m.speak();
        let _ = m.help_topic();
    }

    #[test]
    fn bare_simple() {
        let m = msgs();
        assert_eq!(m.help_topic(), "help");
        assert_eq!(m.enter_name(), "Please enter your name--");
    }

    #[test]
    fn bare_multiline() {
        let m = msgs();
        assert_eq!(
            m.welcome(),
            "Hello.  Welcome to Xcaliber.\nPlease enter your name--"
        );
    }

    #[test]
    fn bare_multiline_trailing_space() {
        let m = msgs();
        assert_eq!(
            m.master_welcome(),
            "Hello.  You are the master terminal.\nPlease enter your name. "
        );
    }

    #[test]
    fn wrapped_simple() {
        let m = msgs();
        assert_eq!(m.speak(), "\nSpeak!\n");
        assert_eq!(m.done(), "\nDone\n");
    }

    #[test]
    fn wrapped_with_args() {
        let m = msgs();
        assert_eq!(
            m.new_user(UserId(0), "Explorer"),
            "\nNew user at #0: Explorer\n"
        );
    }

    #[test]
    fn wrapped_multiline() {
        let m = msgs();
        assert_eq!(
            m.intro(),
            "\nEnter command (type 'HELP' for instructions)\n\
             Changes afoot!  Type EXPLAIN NEW for what's new!\n"
        );
    }

    #[test]
    fn trailing() {
        let m = msgs();
        assert_eq!(m.message_sent(), "Message sent\n");
        assert_eq!(m.conference_full(), "Xcaliber is full--try again later\n");
    }

    #[test]
    fn trailing_with_args() {
        let m = msgs();
        assert_eq!(
            m.talking_with(UserId(0), "Explorer"),
            "You are talking with #0: Explorer\n"
        );
    }

    #[test]
    fn leading() {
        let m = msgs();
        assert_eq!(m.you_are_out(), "\nYou are now out...");
        assert_eq!(m.new_warning_prompt(), "\nNew warning--");
        assert_eq!(m.command_line_prompt(), "\nCommand line--");
    }

    #[test]
    fn leading_spaces() {
        let m = msgs();
        assert_eq!(m.new_users_header(), "  New users\n");
    }

    #[test]
    fn bel_framing() {
        let m = msgs();
        assert_eq!(
            m.time_warning(5, TimeUnit::Minute),
            "\n\x07\x07You have about 5 minutes left\n"
        );
    }

    #[test]
    fn timewrn_singular() {
        let m = msgs();
        assert_eq!(
            m.time_warning(1, TimeUnit::Minute),
            "\n\x07\x07You have about 1 minute left\n"
        );
        assert_eq!(
            m.time_warning(1, TimeUnit::Second),
            "\n\x07\x07You have about 1 second left\n"
        );
    }

    #[test]
    fn timewrn_plural_seconds() {
        let m = msgs();
        assert_eq!(
            m.time_warning(30, TimeUnit::Second),
            "\n\x07\x07You have about 30 seconds left\n"
        );
    }

    #[test]
    fn clock_pm() {
        let m = msgs();
        // 14:30:05 UTC
        let dt = Utc.with_ymd_and_hms(2026, 2, 11, 14, 30, 5).unwrap();
        assert_eq!(m.clock_time(dt), "Time now:  2:30:05 p.m.\n");
    }

    #[test]
    fn clock_am() {
        let m = msgs();
        // 10:00:00 UTC
        let dt = Utc.with_ymd_and_hms(2026, 2, 11, 10, 0, 0).unwrap();
        assert_eq!(m.clock_time(dt), "Time now: 10:00:00 a.m.\n");
    }

    #[test]
    fn uptime_with_date() {
        let m = msgs();
        // 14:30:05 UTC on 2/11/26
        let dt = Utc.with_ymd_and_hms(2026, 2, 11, 14, 30, 5).unwrap();
        assert_eq!(
            m.uptime_with_date(dt),
            "\nUp at  2:30:05 p.m. on 2/11/26\n"
        );
    }

    #[test]
    fn left_entry_normal() {
        let m = msgs();
        // 14:30 UTC
        let dt = Utc.with_ymd_and_hms(2026, 2, 11, 14, 30, 0).unwrap();
        assert_eq!(m.left_entry(dt, false, "Explorer"), "02:30  Explorer");
    }

    #[test]
    fn left_entry_killed() {
        let m = msgs();
        // 09:05 UTC
        let dt = Utc.with_ymd_and_hms(2026, 2, 11, 9, 5, 0).unwrap();
        assert_eq!(m.left_entry(dt, true, "Wanderer"), "09:05K Wanderer");
    }

    #[test]
    fn infomsg_multiline() {
        let m = msgs();
        assert_eq!(
            m.info(5, 10, "localhost", "2.0"),
            " Users: 5\n   max: 10\nServer: localhost v.2.0"
        );
    }

    #[test]
    fn leftmsg_preformatted() {
        let m = msgs();
        assert_eq!(m.left_count(3), "003 Users have left Xcaliber\n");
    }

    #[test]
    fn version_multiline() {
        let m = msgs();
        assert_eq!(
            m.version("2.0"),
            "\nXcaliber II v.2.0 by Michael J. Fromberger\n\
             Copyright (C) 1997-1998 All Rights Reserved\n"
        );
    }

    #[test]
    fn crunow_multiline() {
        let m = msgs();
        assert_eq!(
            m.cru_now("1.234", "100"),
            "\nCRUs now:  1.234\n     max:  100\n"
        );
    }

    #[test]
    fn numusrs_preformatted() {
        let m = msgs();
        assert_eq!(m.user_count(7), "\n7 user(s)\n");
    }
}
