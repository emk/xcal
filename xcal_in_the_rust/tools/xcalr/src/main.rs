//! Xcaliber Mark II — Rust edition (stub).

mod conferences;
mod help;
mod lang;
mod matching;
mod messages;
mod users;

use help::HelpTopics;
use lang::Messages;
use users::UserId;

fn main() {
    let m = Messages::new();

    // Master welcome sequence: mwelc (Bare) + intro (Wrapped) + tlkwith (Trailing)
    let mwelc = m.master_welcome();
    let intro = m.intro();
    let tlkwith = m.talking_with(UserId(0), "Demo");

    print!("{mwelc}{intro}{tlkwith}");

    // Show help system output.
    let h = HelpTopics::new();
    if let Some(text) = h.lookup("help") {
        print!("{text}");
    }
}
