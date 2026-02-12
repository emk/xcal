//! Xcaliber Mark II — Rust edition (stub).

mod messages;

use messages::{keys, FluentArgs, Messages};

fn main() {
    let m = Messages::new();

    // Master welcome sequence: mwelc (Bare) + intro (Wrapped) + tlkwith (Trailing)
    let mwelc = m.format(keys::MWELC, None);
    let intro = m.format(keys::INTRO, None);

    let mut args = FluentArgs::new();
    args.set("number", "0");
    args.set("name", "Demo");
    let tlkwith = m.format(keys::TLKWITH, Some(&args));

    print!("{mwelc}{intro}{tlkwith}");
}
