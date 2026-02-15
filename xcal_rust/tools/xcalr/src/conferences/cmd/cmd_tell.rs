//! Handlers for message composition and delivery: tell, impending.

use super::{super::Conference, CommandResult};
use crate::{
    messages::{Message, MessageKind},
    users::{UserId, UserSet, UserState},
};

pub fn cmd_tell(
    conf: &mut Conference,
    who: UserId,
    args: &str,
) -> CommandResult {
    // Parse targets and optional inline message.
    let Ok((targets, rest)) = conf.parse_user_args(who, args) else {
        return CommandResult::Ok;
    };

    // Check for self-send.
    if targets.contains(who) {
        let msg = conf.lang.invalid_arguments();
        conf.append_output(who, &msg);
        return CommandResult::Ok;
    }

    let is_tell_all = targets.is_empty();

    if is_tell_all {
        // Tell-all: check if enabled on the sender's master.
        let who_idx = usize::from(who.0);
        let master_id = conf
            .users
            .get(who_idx)
            .and_then(|s| s.as_ref())
            .map_or(UserId(0), |u| u.master);
        let master_idx = usize::from(master_id.0);

        let tell_all_enabled = conf
            .users
            .get(master_idx)
            .and_then(|s| s.as_ref())
            .is_some_and(|m| match m.role {
                crate::users::Role::Master { tell_all_enabled }
                | crate::users::Role::Submaster { tell_all_enabled } => {
                    tell_all_enabled
                }
                crate::users::Role::Normal => false,
            });

        if !tell_all_enabled {
            let msg = conf.lang.tell_all_disabled();
            conf.append_output(who, &msg);
            return CommandResult::Ok;
        }

        // Build tell-all recipient set: all users in same subconference,
        // excluding self, excluding Out, excluding RA, excluding ignoring.
        let mut recipients = UserSet::new();
        for slot in &conf.users {
            let Some(user) = slot else { continue };
            if user.id == who {
                continue;
            }
            if user.state == UserState::Out {
                continue;
            }
            if user.prefs.reject_tell_all {
                continue;
            }
            if user.ignoring.contains(who) {
                continue;
            }
            if user.state == UserState::Login {
                continue;
            }
            // Subconference check: same master, or is master, or master of.
            let who_master = conf
                .users
                .get(who_idx)
                .and_then(|s| s.as_ref())
                .map_or(UserId(0), |u| u.master);
            if user.master == who_master
                || user.master == who
                || user.id == who_master
            {
                recipients.insert(user.id);
            }
        }

        if recipients.is_empty() {
            let msg = conf.lang.invalid_arguments();
            conf.append_output(who, &msg);
            return CommandResult::Ok;
        }

        let inline = rest.trim_start();
        if inline.is_empty() {
            // BUILD mode — enter composing state.
            let msg = conf.lang.speak();
            conf.append_output(who, &msg);

            let who_idx = usize::from(who.0);
            if let Some(Some(user)) = conf.users.get_mut(who_idx) {
                user.composing = Some(Message {
                    from: Some(who),
                    kind: MessageKind::TellAll,
                    recipients,
                    body: Vec::new(),
                });
                user.state = UserState::Composing;
            }
        } else {
            // Inline tell-all.
            let mut body = Vec::new();
            body.extend_from_slice(inline.as_bytes());
            body.push(b'\n');

            let who_idx = usize::from(who.0);
            if let Some(Some(user)) = conf.users.get_mut(who_idx) {
                user.composing = Some(Message {
                    from: Some(who),
                    kind: MessageKind::TellAll,
                    recipients,
                    body,
                });
            }

            // Output extra newline before delivery (C: net_write CRLF).
            conf.append_output(who, "\n");
            conf.deliver_composed(who);
        }
    } else {
        // Direct tell.
        // Subconference check: targets must be visible to sender.
        for target_id in targets.iter() {
            if !conf.is_in_subconference(target_id, who) {
                let msg = conf.lang.invalid_arguments();
                conf.append_output(who, &msg);
                return CommandResult::Ok;
            }
        }

        // Pre-check: for direct messages, validate targets before composing
        // (C: checks ignoring and out before setting up message).
        for target_id in targets.iter() {
            let t_idx = usize::from(target_id.0);
            let Some(Some(target)) = conf.users.get(t_idx) else {
                let msg = conf.lang.invalid_arguments();
                conf.append_output(who, &msg);
                return CommandResult::Ok;
            };
            if target.ignoring.contains(who) {
                let msg = conf.lang.users_ignoring();
                conf.append_output(who, &msg);
                return CommandResult::Ok;
            }
            if target.state == UserState::Out {
                let msg = conf.lang.users_out();
                conf.append_output(who, &msg);
                return CommandResult::Ok;
            }
        }

        let inline = rest.trim_start();
        if inline.is_empty() {
            // BUILD mode.
            let msg = conf.lang.speak();
            conf.append_output(who, &msg);

            let who_idx = usize::from(who.0);
            if let Some(Some(user)) = conf.users.get_mut(who_idx) {
                user.composing = Some(Message {
                    from: Some(who),
                    kind: MessageKind::Direct,
                    recipients: targets,
                    body: Vec::new(),
                });
                user.state = UserState::Composing;
            }
        } else {
            // Inline direct tell.
            let mut body = Vec::new();
            body.extend_from_slice(inline.as_bytes());
            body.push(b'\n');

            let who_idx = usize::from(who.0);
            if let Some(Some(user)) = conf.users.get_mut(who_idx) {
                user.composing = Some(Message {
                    from: Some(who),
                    kind: MessageKind::Direct,
                    recipients: targets,
                    body,
                });
            }

            // Output extra newline before delivery (C: net_write CRLF).
            conf.append_output(who, "\n");
            conf.deliver_composed(who);
        }
    }

    CommandResult::Ok
}

pub fn cmd_impending(
    conf: &mut Conference,
    who: UserId,
    _args: &str,
) -> CommandResult {
    let mut output = String::new();

    for slot in &conf.users {
        let Some(user) = slot else { continue };
        if user.state != UserState::Composing {
            continue;
        }
        let Some(ref msg) = user.composing else {
            continue;
        };

        // Check if this user's message would reach the requester.
        if msg.kind == MessageKind::TellAll {
            // Tell-all: visible if requester is a recipient and not RA
            // and not ignoring the composer.
            let who_idx = usize::from(who.0);
            let who_ra = conf
                .users
                .get(who_idx)
                .and_then(|s| s.as_ref())
                .is_some_and(|u| u.prefs.reject_tell_all);
            let who_ignoring = conf
                .users
                .get(who_idx)
                .and_then(|s| s.as_ref())
                .is_some_and(|u| u.ignoring.contains(user.id));

            if !who_ra && msg.recipients.contains(who) && !who_ignoring {
                conf.format_who_entry(user.id, who, &mut output);
            }
        } else {
            // Direct: visible if requester is a recipient and not
            // ignoring the composer.
            let who_idx = usize::from(who.0);
            let who_ignoring = conf
                .users
                .get(who_idx)
                .and_then(|s| s.as_ref())
                .is_some_and(|u| u.ignoring.contains(user.id));

            if msg.recipients.contains(who) && !who_ignoring {
                conf.format_who_entry(user.id, who, &mut output);
            }
        }
    }

    if output.is_empty() {
        let msg = conf.lang.no_messages();
        conf.append_output(who, &msg);
    } else {
        conf.append_output(who, &output);
    }

    CommandResult::Ok
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::conferences::test_harness::TestConference;

    #[test]
    fn tell_build_mode() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "t 1");
        let out = tc.output(master);
        assert!(out.contains("Speak"), "expected Speak prompt: {out}");

        tc.input(master, "Hello from master");
        tc.input(master, "");
        let out = tc.output(master);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out = tc.output(user1);
        assert!(
            out.contains("Hello from master"),
            "expected message body: {out}"
        );
        assert!(
            out.contains("Message from #0: Explorer"),
            "expected header: {out}"
        );
    }

    #[test]
    fn tell_inline() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "t 1;Inline direct");
        let out = tc.output(master);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out = tc.output(user1);
        assert!(
            out.contains("Inline direct"),
            "expected inline message: {out}"
        );
    }

    #[test]
    fn bare_number_tell() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "1");
        let out = tc.output(master);
        assert!(out.contains("Speak"), "expected Speak: {out}");

        tc.input(master, "Bare number works");
        tc.input(master, "");
        let out = tc.output(master);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out = tc.output(user1);
        assert!(out.contains("Bare number works"), "expected message: {out}");
    }

    #[test]
    fn bare_number_inline() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "1;Inline bare");
        let out = tc.output(master);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out = tc.output(user1);
        assert!(out.contains("Inline bare"), "expected message: {out}");
    }

    #[test]
    fn multi_recipient() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");
        let user2 = tc.connect("Traveler");

        tc.input(master, "1,2");
        let out = tc.output(master);
        assert!(out.contains("Speak"), "expected Speak: {out}");

        tc.input(master, "To both of you");
        tc.input(master, "");
        let out = tc.output(master);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out1 = tc.output(user1);
        assert!(
            out1.contains("To both of you"),
            "expected message for user1: {out1}"
        );
        let out2 = tc.output(user2);
        assert!(
            out2.contains("To both of you"),
            "expected message for user2: {out2}"
        );
    }

    #[test]
    fn multi_line_message() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(user1, "t 0");
        tc.drain(user1);
        tc.input(user1, "Line one");
        tc.input(user1, "Line two");
        tc.input(user1, "Line three");
        tc.input(user1, "");
        tc.drain(user1);

        let out = tc.output(master);
        assert!(out.contains("Line one"), "expected line one: {out}");
        assert!(out.contains("Line three"), "expected line three: {out}");
    }

    #[test]
    fn tell_all_build() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");
        let user2 = tc.connect("Traveler");

        tc.input(user1, "te");
        let out = tc.output(user1);
        assert!(out.contains("Speak"), "expected Speak: {out}");

        tc.input(user1, "Greetings everyone");
        tc.input(user1, "");
        let out = tc.output(user1);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out_m = tc.output(master);
        assert!(
            out_m.contains("Greetings everyone"),
            "expected master got message: {out_m}"
        );
        assert!(
            out_m.contains("Message to all from #1"),
            "expected tell-all header: {out_m}"
        );

        let out2 = tc.output(user2);
        assert!(
            out2.contains("Greetings everyone"),
            "expected user2 got message: {out2}"
        );
    }

    #[test]
    fn tell_all_inline() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");
        let user2 = tc.connect("Traveler");

        tc.input(master, "te;Broadcast inline");
        let out = tc.output(master);
        assert!(out.contains("Message sent"), "expected sent: {out}");

        let out1 = tc.output(user1);
        assert!(
            out1.contains("Broadcast inline"),
            "expected user1 got broadcast: {out1}"
        );
        let out2 = tc.output(user2);
        assert!(
            out2.contains("Broadcast inline"),
            "expected user2 got broadcast: {out2}"
        );
    }

    #[test]
    fn abort_with_percent_k() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        tc.input(master, "t 1");
        tc.drain(master);
        tc.input(master, "You should not see this%k");
        tc.input(master, "");
        let out = tc.output(master);
        assert!(
            out.contains("Message not sent"),
            "expected abort message: {out}"
        );

        // User1 should NOT have received the message.
        let out1 = tc.output(user1);
        assert!(
            !out1.contains("You should not see this"),
            "aborted message should not be delivered: {out1}"
        );
    }

    #[test]
    fn impending_no_messages() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let _user1 = tc.connect("Wanderer");

        tc.input(master, "im");
        let out = tc.output(master);
        assert!(out.contains("No messages"), "expected no messages: {out}");
    }

    #[test]
    fn impending_shows_building() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        // User1 starts composing to master.
        tc.input(user1, "t 0");
        tc.drain(user1);
        tc.input(user1, "Hello there");

        // Master checks impending.
        tc.input(master, "im");
        let out = tc.output(master);
        assert!(out.contains("B-0"), "expected B-0 state: {out}");
        assert!(out.contains("Wanderer"), "expected Wanderer name: {out}");
    }

    #[test]
    fn impending_via_empty_line() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");
        let user1 = tc.connect("Wanderer");

        // User1 starts tell-all.
        tc.input(user1, "te");
        tc.drain(user1);
        tc.input(user1, "Greetings all");

        // Master uses empty line (= impending).
        tc.input(master, "");
        let out = tc.output(master);
        assert!(out.contains("B/A"), "expected B/A state: {out}");
    }

    #[test]
    fn tell_self_rejected() {
        let mut tc = TestConference::new();
        let master = tc.connect("Explorer");

        tc.input(master, "t 0");
        let out = tc.output(master);
        assert!(
            out.to_lowercase().contains("invalid")
                || out.to_lowercase().contains("argument"),
            "expected invalid arguments for self-tell: {out}"
        );
    }
}
