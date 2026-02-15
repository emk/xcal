//! Per-fixture replay tests.
//!
//! Each test replays a JSONL fixture against [`Conference`] in-process
//! and compares the output transcripts against saved `.txt` files.

use std::path::Path;

/// Run the named fixture, skipping if `SKIP.md` exists.
fn run(name: &str) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    if dir.join("SKIP.md").exists() {
        eprintln!("SKIP: {name}");
        return;
    }
    if let Err(e) = super::fixture_runner::run_fixture(&dir) {
        panic!("{e}");
    }
}

macro_rules! fixture_test {
    ($name:ident) => {
        #[test]
        fn $name() {
            run(stringify!($name));
        }
    };
    ($name:ident, ignore = $reason:literal) => {
        #[test]
        #[ignore = $reason]
        fn $name() {
            run(stringify!($name));
        }
    };
}

fixture_test!(hello);
fixture_test!(cmd_an_rn);
fixture_test!(cmd_clock);
fixture_test!(cmd_explain);
fixture_test!(cmd_help);
fixture_test!(cmd_id);
fixture_test!(cmd_ignore);
fixture_test!(cmd_im);
fixture_test!(cmd_info);
fixture_test!(cmd_kill);
fixture_test!(cmd_left);
fixture_test!(cmd_line);
fixture_test!(cmd_listing);
fixture_test!(cmd_out);
fixture_test!(cmd_rall);
fixture_test!(cmd_rb_ab);
fixture_test!(cmd_rc_oc);
fixture_test!(cmd_rp);
fixture_test!(cmd_runtime);
fixture_test!(cmd_stop);
fixture_test!(cmd_tell);
fixture_test!(cmd_time);
fixture_test!(cmd_tty);
fixture_test!(cmd_version);
fixture_test!(cmd_warn);
fixture_test!(subcon_basic);
fixture_test!(subcon_disconnect);
fixture_test!(subcon_enable_disable);
fixture_test!(subcon_give_new);
fixture_test!(subcon_kill_submaster);
fixture_test!(subcon_nested);
fixture_test!(subcon_normalize);
fixture_test!(subcon_who);
