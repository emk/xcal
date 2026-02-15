//! Command parsing and dispatch.
//!
//! Splits an input line into a [`Command`] variant and the unparsed
//! argument tail, mirroring the C server's `con_command()` + `cmdtab[]`.

use super::Conference;
use crate::{matching::CommandPattern, users::UserId};

mod cmd_bye;
mod cmd_extended;
mod cmd_help;
mod cmd_ignore;
mod cmd_listing;
mod cmd_master;
mod cmd_prefs;
mod cmd_query;
mod cmd_session;
mod cmd_tell;
mod cmd_warn;

/// Every distinct command the conference recognises.
///
/// Aliases (e.g. `st*op` and `by*e`) map to the same variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    // Communication
    Tell,
    Impending,

    // Information
    Who,
    Port,
    Everything,
    Tty,
    Left,
    Users,
    Info,
    Clock,
    Time,
    Runtime,
    Version,
    DisplayWarning,

    // State toggles
    RejectAlls,
    AcceptAlls,
    Ignore,
    Accept,
    AcceptNotify,
    RejectNotify,
    RejectPorts,
    RejectControls,
    OkayControls,
    RejectBreaks,
    AcceptBreaks,

    // Session
    Id,
    Out,
    Line,
    Bye,

    // Help
    Help,
    Explain,

    // Master
    Kill,
    MakeTty,
    Normalize,
    Give,
    Enable,
    Disable,
    Warn,
    Below,

    // Unimplemented (distinct for future use)
    Bounce,
    Unbounce,
    ListBounce,
    BounceUser,

    // Extended / privileged
    ExtendedWho,
    Xdl,
    Xlang,
    Xyzzy,
    Xtend,
    Xreset,
}

/// Result of executing a command handler.
pub enum CommandResult {
    /// Command completed normally. Caller returns user to idle.
    Ok,
    /// The conference should end.
    EndConference,
}

/// Dispatch a parsed command to the appropriate handler.
pub fn dispatch(
    conf: &mut Conference,
    who: UserId,
    command: Command,
    args: &str,
) -> CommandResult {
    match command {
        // Listing
        Command::Who => cmd_listing::cmd_who(conf, who, args),
        Command::Port => cmd_listing::cmd_port(conf, who, args),
        Command::Everything => cmd_listing::cmd_everything(conf, who, args),
        Command::Tty => cmd_listing::cmd_tty(conf, who, args),
        Command::Left => cmd_listing::cmd_left(conf, who, args),
        Command::Users => cmd_listing::cmd_users(conf, who, args),
        Command::Below => cmd_listing::cmd_below(conf, who, args),
        Command::ExtendedWho => cmd_listing::cmd_xwho(conf, who, args),

        // Query
        Command::Info => cmd_query::cmd_info(conf, who, args),
        Command::Clock => cmd_query::cmd_clock(conf, who, args),
        Command::Time => cmd_query::cmd_time(conf, who, args),
        Command::Runtime => cmd_query::cmd_runtime(conf, who, args),
        Command::Version => cmd_query::cmd_version(conf, who, args),

        // Communication
        Command::Tell => cmd_tell::cmd_tell(conf, who, args),
        Command::Impending => cmd_tell::cmd_impending(conf, who, args),

        // Preferences
        Command::RejectAlls => cmd_prefs::cmd_reject_alls(conf, who, args),
        Command::AcceptAlls => cmd_prefs::cmd_accept_alls(conf, who, args),
        Command::AcceptNotify => cmd_prefs::cmd_accept_notify(conf, who, args),
        Command::RejectNotify => cmd_prefs::cmd_reject_notify(conf, who, args),
        Command::RejectPorts => cmd_prefs::cmd_reject_ports(conf, who, args),
        Command::RejectControls => {
            cmd_prefs::cmd_reject_controls(conf, who, args)
        }
        Command::OkayControls => cmd_prefs::cmd_okay_controls(conf, who, args),
        Command::RejectBreaks => cmd_prefs::cmd_reject_breaks(conf, who, args),
        Command::AcceptBreaks => cmd_prefs::cmd_accept_breaks(conf, who, args),

        // Ignore
        Command::Ignore => cmd_ignore::cmd_ignore(conf, who, args),
        Command::Accept => cmd_ignore::cmd_accept(conf, who, args),

        // Session
        Command::Id => cmd_session::cmd_id(conf, who, args),
        Command::Out => cmd_session::cmd_out(conf, who, args),
        Command::Line => cmd_session::cmd_line(conf, who, args),

        // Bye
        Command::Bye => cmd_bye::cmd_bye(conf, who, args),

        // Help
        Command::Help => cmd_help::cmd_help(conf, who, args),
        Command::Explain => cmd_help::cmd_explain(conf, who, args),

        // Warning
        Command::Warn => cmd_warn::cmd_warn(conf, who, args),
        Command::DisplayWarning => {
            cmd_warn::cmd_display_warning(conf, who, args)
        }

        // Master
        Command::Kill => cmd_master::cmd_kill(conf, who, args),
        Command::MakeTty => cmd_master::cmd_make_tty(conf, who, args),
        Command::Normalize => cmd_master::cmd_normalize(conf, who, args),
        Command::Give => cmd_master::cmd_give(conf, who, args),
        Command::Enable => cmd_master::cmd_enable(conf, who, args),
        Command::Disable => cmd_master::cmd_disable(conf, who, args),

        // Extended
        Command::Xyzzy => cmd_extended::cmd_xyzzy(conf, who, args),
        Command::Xdl => cmd_extended::cmd_xdl(conf, who, args),
        Command::Xlang => cmd_extended::cmd_xlang(conf, who, args),
        Command::Xtend => cmd_extended::cmd_xtend(conf, who, args),
        Command::Xreset => cmd_extended::cmd_xreset(conf, who, args),

        // Permanently unimplemented (C: cmd_nimp)
        Command::Bounce
        | Command::Unbounce
        | Command::ListBounce
        | Command::BounceUser => {
            let msg = conf.lang.not_implemented();
            conf.append_output(who, &msg);
            CommandResult::Ok
        }
    }
}

/// Command table in C `cmdtab` order — first match wins.
///
/// Note: `tell`'s pattern is `t*e*ll` (no leading `*`). The C version
/// used `*t*e*ll` so an empty alpha prefix from numeric input would
/// match; we handle that case explicitly in [`parse`].
static COMMAND_TABLE: &[(&str, Command)] = &[
    ("t*e*ll", Command::Tell),
    ("w*ho", Command::Who),
    ("by*e", Command::Bye),
    ("p*ort", Command::Port),
    ("id", Command::Id),
    ("im*p", Command::Impending),
    ("ra*ll", Command::RejectAlls),
    ("aa*ll", Command::AcceptAlls),
    ("ig*nore", Command::Ignore),
    ("ac*cept", Command::Accept),
    ("an", Command::AcceptNotify),
    ("rn", Command::RejectNotify),
    ("rp", Command::RejectPorts),
    ("rc", Command::RejectControls),
    ("oc", Command::OkayControls),
    ("ou*t", Command::Out),
    ("ti*me", Command::Time),
    ("tt*y", Command::Tty),
    ("le*ft", Command::Left),
    ("rb", Command::RejectBreaks),
    ("ab", Command::AcceptBreaks),
    ("c*l*ock", Command::Clock),
    ("in*f*o", Command::Info),
    ("dw", Command::DisplayWarning),
    ("us*ers", Command::Users),
    ("nu*mber", Command::Users),
    ("a", Command::Everything),
    ("ev*erything", Command::Everything),
    ("wa*rn", Command::Warn),
    ("bo", Command::Bounce),
    ("nb*o", Command::Unbounce),
    ("lb*o", Command::ListBounce),
    ("bu", Command::BounceUser),
    ("ki*ll", Command::Kill),
    ("al*l", Command::Everything),
    ("he*lp", Command::Help),
    ("en*able", Command::Enable),
    ("di*s*able", Command::Disable),
    ("xw*h*o", Command::ExtendedWho),
    ("l*in*e", Command::Line),
    ("mt*y", Command::MakeTty),
    ("gi*ve", Command::Give),
    ("e*x*p*l*ain", Command::Explain),
    ("no*r*m", Command::Normalize),
    ("st*op", Command::Bye),
    ("ve*r*s*ion", Command::Version),
    ("be*l*ow", Command::Below),
    ("xdl", Command::Xdl),
    ("xlang", Command::Xlang),
    ("xyzzy", Command::Xyzzy),
    ("xtend", Command::Xtend),
    ("xreset", Command::Xreset),
    ("ru*n*time", Command::Runtime),
];

/// Parse a command line into a [`Command`] and its unparsed arguments.
///
/// Returns `None` for unrecognised commands.
pub fn parse(input: &str) -> Option<(Command, &str)> {
    let trimmed = input.trim_start();

    // Empty input → impending message.
    if trimmed.is_empty() {
        return Some((Command::Impending, ""));
    }

    // Leading digit → tell (C handled this via `*t*e*ll`'s leading `*`
    // matching an empty alpha prefix; we make it explicit).
    if trimmed.as_bytes()[0].is_ascii_digit() {
        return Some((Command::Tell, trimmed));
    }

    // Extract the alphabetic prefix.
    let alpha_end = trimmed
        .bytes()
        .position(|b| !b.is_ascii_alphabetic())
        .unwrap_or(trimmed.len());
    let prefix = &trimmed[..alpha_end];
    let args = &trimmed[alpha_end..];

    // First-match scan through the command table.
    for &(pattern, cmd) in COMMAND_TABLE {
        if CommandPattern::new(pattern).matches(prefix) {
            return Some((cmd, args));
        }
    }

    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn empty_input_is_impending() {
        assert_eq!(parse(""), Some((Command::Impending, "")));
        assert_eq!(parse("   "), Some((Command::Impending, "")));
    }

    #[test]
    fn who_exact() {
        assert_eq!(parse("who"), Some((Command::Who, "")));
    }

    #[test]
    fn who_abbreviated() {
        assert_eq!(parse("w"), Some((Command::Who, "")));
    }

    #[test]
    fn tell_alpha_prefix() {
        assert_eq!(parse("t1;Hello"), Some((Command::Tell, "1;Hello")));
    }

    #[test]
    fn tell_numeric_prefix() {
        assert_eq!(parse("3;Hello"), Some((Command::Tell, "3;Hello")));
    }

    #[test]
    fn leading_whitespace_stripped() {
        assert_eq!(parse("  who"), Some((Command::Who, "")));
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(parse("WHO"), Some((Command::Who, "")));
        assert_eq!(parse("Who"), Some((Command::Who, "")));
    }

    #[test]
    fn bye_alias() {
        assert_eq!(parse("by"), Some((Command::Bye, "")));
        assert_eq!(parse("bye"), Some((Command::Bye, "")));
    }

    #[test]
    fn stop_alias() {
        assert_eq!(parse("st"), Some((Command::Bye, "")));
        assert_eq!(parse("stop"), Some((Command::Bye, "")));
    }

    #[test]
    fn xyzzy_exact() {
        assert_eq!(parse("xyzzy"), Some((Command::Xyzzy, "")));
    }

    #[test]
    fn no_match() {
        assert_eq!(parse("zzzz"), None);
    }

    #[test]
    fn tell_all_abbreviation() {
        assert_eq!(parse("te"), Some((Command::Tell, "")));
    }

    #[test]
    fn single_char_everything() {
        assert_eq!(parse("a"), Some((Command::Everything, "")));
    }

    #[test]
    fn args_include_space_after_prefix() {
        assert_eq!(parse("tell 3;hello"), Some((Command::Tell, " 3;hello")));
    }

    #[test]
    fn explain_abbreviation() {
        assert_eq!(parse("e"), Some((Command::Explain, "")));
        assert_eq!(parse("ex"), Some((Command::Explain, "")));
        assert_eq!(parse("explain"), Some((Command::Explain, "")));
    }

    #[test]
    fn clock_abbreviation() {
        assert_eq!(parse("c"), Some((Command::Clock, "")));
        assert_eq!(parse("cl"), Some((Command::Clock, "")));
        assert_eq!(parse("clock"), Some((Command::Clock, "")));
    }

    #[test]
    fn line_abbreviation() {
        assert_eq!(parse("l"), Some((Command::Line, "")));
        assert_eq!(parse("lin"), Some((Command::Line, "")));
        assert_eq!(parse("line"), Some((Command::Line, "")));
    }

    #[test]
    fn unknown_command_error() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "zzzz");
        let out = tc.output(master);

        assert!(
            out.contains("Command error"),
            "expected 'Command error' for unknown command: {out}"
        );
    }

    #[test]
    fn stub_returns_not_implemented() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        // Use xdl — an extended command that will remain a stub.
        tc.input(master, "xdl");
        let out = tc.output(master);

        assert!(
            out.contains("not implemented") || out.contains("Not implemented"),
            "expected not-implemented message for stubbed command: {out}"
        );
    }
}
