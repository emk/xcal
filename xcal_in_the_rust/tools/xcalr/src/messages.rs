//! Localized message formatting for Xcaliber.
//!
//! Messages are stored in Project Fluent `.ftl` files. The Fluent text contains
//! human-readable content only; wire framing (`\n` prefix/suffix, BEL bytes) is
//! applied by this module. The transport layer converts `\n` to `\r\n`.

use fluent_bundle::{FluentBundle, FluentResource};
use unic_langid::LanguageIdentifier;

pub use fluent_bundle::FluentArgs;

/// All 64 message keys, matching the original C identifiers from `lang/*.lf`.
#[allow(missing_docs)]
pub mod keys {
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

    /// All 64 message keys for validation.
    pub const ALL: &[&str] = &[
        CONFULL, CONTERM, WELCOME, ENTNAME, MWELC, INTRO, TLKWITH, NEWUSER,
        DONEMSG, MSGFROM, ALLFROM, NAMENOT, EXNOT, KILLNOT, SPEAK, OUTMSG,
        YOUOUT, IGMSG, DISCMSG, NORCPT, SENTMSG, MNSMSG, EMPTY, CMDERR, FMTERR,
        INVARG, UNIMP, BADCHAR, NEWWARN, NEWNAME, NOLEFT, NOMSGS, NOALLS,
        LINMSG, NOPORT, RPMSG, BOUNCED, BLFULL, NBPORT, NOWMAST, NORMAL,
        PASSED, MPTMMSG, PNUMSG, TIMEWRN, CLKMSG, TIMELFT, CANTEXP, FNFMSG,
        HELPTOP, LEFTMSG, LEFTHDR, UPAT, UPAT2, NUMUSRS, INFOMSG, XCALVER,
        NEWLANG, NOLANG, CURLANG, CRUNOW, CORESIZE, AYTMSG, BRKMSG,
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
        // Bare (5)
        keys::ENTNAME
        | keys::WELCOME
        | keys::MWELC
        | keys::HELPTOP
        | keys::INFOMSG => ("", ""),

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

/// Localized message formatter backed by Project Fluent.
///
/// Loads a `.ftl` resource at construction time and validates that all
/// expected message keys are present. Format methods apply wire framing
/// (newline prefix/suffix, BEL bytes) around the Fluent text content.
pub struct Messages {
    bundle: FluentBundle<FluentResource>,
}

impl Messages {
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

        Messages { bundle }
    }

    /// Format a message with wire framing applied.
    ///
    /// The returned string includes the protocol prefix/suffix (`\n`, BEL)
    /// appropriate for the message key. Use [`format_raw`](Self::format_raw)
    /// when the message is used as a data value rather than direct output.
    pub fn format(&self, key: &str, args: Option<&FluentArgs<'_>>) -> String {
        let (prefix, suffix) = framing(key);
        let text = self.format_raw(key, args);
        format!("{prefix}{text}{suffix}")
    }

    /// Format a message without wire framing.
    ///
    /// Returns the Fluent text content only, with no prefix/suffix bytes.
    /// Useful for messages used as data values (e.g. `helptop` is a topic
    /// name passed to another function, not output directly).
    pub fn format_raw(
        &self,
        key: &str,
        args: Option<&FluentArgs<'_>>,
    ) -> String {
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
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn msgs() -> Messages {
        Messages::new()
    }

    #[test]
    fn all_keys_present() {
        let m = msgs();
        assert_eq!(keys::ALL.len(), 64);
        // Messages::new() already validates all keys exist in the bundle.
        // Verify we can at least resolve no-arg messages without error.
        let _ = m.format_raw(keys::SPEAK, None);
        let _ = m.format_raw(keys::HELPTOP, None);
    }

    #[test]
    fn bare_simple() {
        let m = msgs();
        assert_eq!(m.format(keys::HELPTOP, None), "help");
        assert_eq!(m.format(keys::ENTNAME, None), "Please enter your name--");
    }

    #[test]
    fn bare_multiline() {
        let m = msgs();
        assert_eq!(
            m.format(keys::WELCOME, None),
            "Hello.  Welcome to Xcaliber.\nPlease enter your name--"
        );
    }

    #[test]
    fn bare_multiline_trailing_space() {
        let m = msgs();
        assert_eq!(
            m.format(keys::MWELC, None),
            "Hello.  You are the master terminal.\nPlease enter your name. "
        );
    }

    #[test]
    fn wrapped_simple() {
        let m = msgs();
        assert_eq!(m.format(keys::SPEAK, None), "\nSpeak!\n");
        assert_eq!(m.format(keys::DONEMSG, None), "\nDone\n");
    }

    #[test]
    fn wrapped_with_args() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("number", "0");
        args.set("name", "Explorer");
        assert_eq!(
            m.format(keys::NEWUSER, Some(&args)),
            "\nNew user at #0: Explorer\n"
        );
    }

    #[test]
    fn wrapped_multiline() {
        let m = msgs();
        assert_eq!(
            m.format(keys::INTRO, None),
            "\nEnter command (type 'HELP' for instructions)\n\
             Changes afoot!  Type EXPLAIN NEW for what's new!\n"
        );
    }

    #[test]
    fn trailing() {
        let m = msgs();
        assert_eq!(m.format(keys::SENTMSG, None), "Message sent\n");
        assert_eq!(
            m.format(keys::CONFULL, None),
            "Xcaliber is full--try again later\n"
        );
    }

    #[test]
    fn trailing_with_args() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("number", "0");
        args.set("name", "Explorer");
        assert_eq!(
            m.format(keys::TLKWITH, Some(&args)),
            "You are talking with #0: Explorer\n"
        );
    }

    #[test]
    fn leading() {
        let m = msgs();
        assert_eq!(m.format(keys::YOUOUT, None), "\nYou are now out...");
        assert_eq!(m.format(keys::NEWWARN, None), "\nNew warning--");
        assert_eq!(m.format(keys::LINMSG, None), "\nCommand line--");
    }

    #[test]
    fn leading_spaces() {
        let m = msgs();
        assert_eq!(m.format(keys::PNUMSG, None), "  New users\n");
    }

    #[test]
    fn bel_framing() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("count", 5_i64);
        args.set("unit", "minute");
        assert_eq!(
            m.format(keys::TIMEWRN, Some(&args)),
            "\n\x07\x07You have about 5 minutes left\n"
        );
    }

    #[test]
    fn timewrn_singular() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("count", 1_i64);
        args.set("unit", "minute");
        assert_eq!(
            m.format(keys::TIMEWRN, Some(&args)),
            "\n\x07\x07You have about 1 minute left\n"
        );

        args.set("count", 1_i64);
        args.set("unit", "second");
        assert_eq!(
            m.format(keys::TIMEWRN, Some(&args)),
            "\n\x07\x07You have about 1 second left\n"
        );
    }

    #[test]
    fn timewrn_plural_seconds() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("count", 30_i64);
        args.set("unit", "second");
        assert_eq!(
            m.format(keys::TIMEWRN, Some(&args)),
            "\n\x07\x07You have about 30 seconds left\n"
        );
    }

    #[test]
    fn clock_pm() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("hours", "2");
        args.set("minutes", "30");
        args.set("seconds", "05");
        args.set("ampm", "pm");
        assert_eq!(
            m.format(keys::CLKMSG, Some(&args)),
            "Time now: 2:30:05 p.m.\n"
        );
    }

    #[test]
    fn clock_am() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("hours", "10");
        args.set("minutes", "00");
        args.set("seconds", "00");
        args.set("ampm", "am");
        assert_eq!(
            m.format(keys::CLKMSG, Some(&args)),
            "Time now: 10:00:00 a.m.\n"
        );
    }

    #[test]
    fn uptime_with_date() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("hours", "2");
        args.set("minutes", "30");
        args.set("seconds", "05");
        args.set("ampm", "pm");
        args.set("date", "2/11/26");
        assert_eq!(
            m.format(keys::UPAT2, Some(&args)),
            "\nUp at 2:30:05 p.m. on 2/11/26\n"
        );
    }

    #[test]
    fn infomsg_multiline() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("users", "5");
        args.set("max", "10");
        args.set("server", "localhost");
        args.set("version", "2.0");
        assert_eq!(
            m.format(keys::INFOMSG, Some(&args)),
            " Users: 5\n   max: 10\nServer: localhost v.2.0"
        );
    }

    #[test]
    fn leftmsg_preformatted() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("count", "003");
        assert_eq!(
            m.format(keys::LEFTMSG, Some(&args)),
            "003 Users have left Xcaliber\n"
        );
    }

    #[test]
    fn format_raw_no_framing() {
        let m = msgs();
        // speak is Wrapped, but format_raw returns just the text
        assert_eq!(m.format_raw(keys::SPEAK, None), "Speak!");
        assert_eq!(
            m.format_raw(keys::CONFULL, None),
            "Xcaliber is full--try again later"
        );
    }

    #[test]
    fn version_multiline() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("version", "2.0");
        assert_eq!(
            m.format(keys::XCALVER, Some(&args)),
            "\nXcaliber II v.2.0 by Michael J. Fromberger\n\
             Copyright (C) 1997-1998 All Rights Reserved\n"
        );
    }

    #[test]
    fn crunow_multiline() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("crus", "1.234");
        args.set("max", "100");
        assert_eq!(
            m.format(keys::CRUNOW, Some(&args)),
            "\nCRUs now:  1.234\n     max:  100\n"
        );
    }

    #[test]
    fn numusrs_preformatted() {
        let m = msgs();
        let mut args = FluentArgs::new();
        args.set("count", "7");
        assert_eq!(m.format(keys::NUMUSRS, Some(&args)), "\n7 user(s)\n");
    }
}
