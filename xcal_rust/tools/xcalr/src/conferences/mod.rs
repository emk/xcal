//! Core conference domain types.
//!
//! Models the conference-level data from the C implementation (`con.h`):
//! departure records, bounce lists, and the main conference struct.

#![allow(dead_code)]

use std::{
    collections::HashMap,
    fmt::Write as _,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

use bytes::BytesMut;
use dcts::ports::{Port, PortId, PortMessage};
use miette::Report;
use tracing::{debug, info, warn};

use crate::{
    help::HelpTopics,
    lang::Lang,
    messages::{Message, MessageKind},
    users::{Role, User, UserId, UserSet, UserState},
};

mod cmd;
#[cfg(test)]
mod fixture_runner;
#[cfg(test)]
mod fixture_tests;
#[cfg(test)]
mod test_harness;

/// Record of a user who has left the conference (for the `left` command).
#[derive(Debug, Clone)]
pub struct LeftUser {
    pub id: UserId,
    pub name: String,
    pub left_at: Instant,
    /// Wall-clock time the user departed (for time-of-day display).
    pub departed_at: SystemTime,
    pub was_killed: bool,
}

/// A banned address.
#[derive(Debug, Clone)]
pub struct BounceEntry {
    pub addr: std::net::SocketAddr,
    pub hidden: bool,
}

/// A message sent to the conference from the outside world.
#[derive(Debug)]
pub enum ConferenceMessage {
    /// An event from the port/transport layer.
    Port(PortMessage),
}

/// The main conference — holds all live users and conference state.
pub struct Conference {
    /// Localized message formatter.
    pub(crate) lang: Lang,
    /// Help file lookup.
    pub(crate) help: HelpTopics,
    /// Map from [`PortId`] to [`UserId`].
    pub port_to_user: HashMap<PortId, UserId>,
    /// User slots — `None` for empty slots, `Some` for connected users.
    pub users: Vec<Option<User>>,
    /// Users who have left (for the `left` command).
    pub left: Vec<LeftUser>,
    /// Current conference warning, if any.
    pub current_warning: Option<Arc<Message>>,
    /// When the conference was started.
    pub started_at: Instant,
    /// When the conference is scheduled to end.
    pub ends_at: Instant,
    /// Which master receives new user notifications.
    pub new_user_master: UserId,
    /// Transmission block size in bytes (C: `trblk`).
    pub send_block_size: usize,
    /// Banned addresses.
    pub bounced: Vec<BounceEntry>,
    /// Current number of connected users.
    pub current_count: usize,
    /// Peak number of simultaneous users.
    pub peak_count: usize,
}

/// Which columns to include in a WHO-style entry line.
#[derive(Debug, Clone, Copy)]
pub(crate) enum WhoColumns {
    /// Role, state, flags, name (who/tty/below).
    StatName,
    /// Port address, name (port command).
    PortName,
    /// Role, state, flags, port address, name (everything).
    Everything,
}

impl Conference {
    /// Create a new empty conference.
    pub fn new() -> Self {
        let now = Instant::now();
        // C default: 45 minutes.
        let ends_at =
            now.checked_add(Duration::from_secs(45 * 60)).unwrap_or(now);
        Self {
            lang: Lang::new(),
            help: HelpTopics::new(),
            port_to_user: HashMap::new(),
            users: Vec::new(),
            left: Vec::new(),
            current_warning: None,
            started_at: now,
            ends_at,
            new_user_master: UserId(0),
            send_block_size: 512,
            bounced: Vec::new(),
            current_count: 0,
            peak_count: 0,
        }
    }

    // ── Event handlers ──────────────────────────────────────────────

    /// Handle a new connection.
    pub fn handle_new(&mut self, port: Box<dyn Port>) -> Result<(), Report> {
        let idx =
            self.users
                .iter()
                .position(|u| u.is_none())
                .unwrap_or_else(|| {
                    self.users.push(None);
                    self.users.len().saturating_sub(1)
                });

        let id = UserId(
            u8::try_from(idx).map_err(|_| miette::miette!("too many users"))?,
        );
        let port_id = port.id();

        let is_first = self.current_count == 0 && self.peak_count == 0;
        let role = if is_first {
            Role::Master {
                tell_all_enabled: true,
            }
        } else {
            Role::Normal
        };

        let welcome = match role {
            Role::Master { .. } => self.lang.master_welcome(),
            _ => self.lang.welcome(),
        };

        let mut user = User::new(id, port, role);
        // Set master assignment (C: nup->mstr).
        user.master = if is_first { id } else { self.new_user_master };
        user.output.extend_from_slice(welcome.as_bytes());
        Self::flush_user_output(&mut user);

        // Queue existing warning for new user (C: user_enqueue(nup, cp->warn)).
        if let Some(ref warning) = self.current_warning {
            user.queue.push_back(Arc::clone(warning));
        }

        self.port_to_user.insert(port_id, id);
        self.users[idx] = Some(user);
        self.current_count = self.current_count.saturating_add(1);
        if self.current_count > self.peak_count {
            self.peak_count = self.current_count;
        }

        info!(%id, "user connected");
        Ok(())
    }

    /// Handle a line of input from a user.
    ///
    /// Returns `Ok(true)` if the conference has ended (master typed `bye`).
    pub fn handle_input(
        &mut self,
        port_id: PortId,
        input: &BytesMut,
    ) -> Result<bool, Report> {
        let Some(&id) = self.port_to_user.get(&port_id) else {
            warn!(?port_id, "input from unknown port");
            return Ok(false);
        };

        let idx = usize::from(id.0);
        let Some(Some(ref user)) = self.users.get(idx) else {
            return Ok(false);
        };
        let state = user.state;

        match state {
            UserState::Login => {
                let name = String::from_utf8_lossy(input).to_string();

                // Look up the master's name for the talking_with message.
                let master_id = self
                    .users
                    .get(idx)
                    .and_then(|s| s.as_ref())
                    .map_or(UserId(0), |u| u.master);
                let master_idx = usize::from(master_id.0);

                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.name = name.clone();
                    user.prefs.reject_controls = true;
                    user.prefs.reject_notifications = false;
                    user.state = UserState::Idle;
                }

                // Build intro + talking_with output.
                let intro = self.lang.intro();
                // For master: master_id == id, so we show our own name.
                // For others: we show the master's name.
                let master_name = if master_id == id {
                    name.clone()
                } else {
                    self.users
                        .get(master_idx)
                        .and_then(|s| s.as_ref())
                        .map_or_else(String::new, |u| u.name.clone())
                };
                let talking = self.lang.talking_with(master_id, &master_name);

                self.append_output(id, &intro);
                self.append_output(id, &talking);

                // Send join notification to other users (C: con_login).
                let notification = self.lang.new_user(id, &name);
                self.send_notification(&notification, id);

                // Deliver any queued messages (e.g., warning).
                self.deliver_queued(id);
                self.flush_output(id);
            }

            UserState::Idle | UserState::Line => {
                if self.dispatch_command(id, input)? {
                    return Ok(true);
                }
            }

            UserState::Out => {
                // Any input returns to Idle (C: user_poll OUT_ST).
                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.state = UserState::Idle;
                }
                self.deliver_queued(id);
                self.flush_output(id);
            }

            UserState::Composing => {
                let line = String::from_utf8_lossy(input);
                let trimmed = line.trim();

                if trimmed.is_empty() {
                    // Blank line: check for %k abort, then deliver.
                    // C: checks if accumulated body ends with %k\r\n.
                    let should_abort = self
                        .users
                        .get(idx)
                        .and_then(|s| s.as_ref())
                        .and_then(|u| u.composing.as_ref())
                        .is_some_and(|msg| {
                            let b = &msg.body;
                            b.len() >= 3
                                && b[b.len().saturating_sub(3)] == b'%'
                                && b[b.len().saturating_sub(2)]
                                    .eq_ignore_ascii_case(&b'k')
                                && b[b.len().saturating_sub(1)] == b'\n'
                        });

                    if should_abort {
                        if let Some(Some(user)) = self.users.get_mut(idx) {
                            user.composing = None;
                            user.state = UserState::Idle;
                        }
                        let msg = self.lang.message_not_sent();
                        self.append_output(id, &msg);
                    } else {
                        self.deliver_composed(id);
                        if let Some(Some(user)) = self.users.get_mut(idx) {
                            user.state = UserState::Idle;
                        }
                    }
                    self.deliver_queued(id);
                    self.flush_output(id);
                } else {
                    // Accumulate line into composing message body.
                    if let Some(Some(user)) = self.users.get_mut(idx) {
                        if let Some(ref mut msg) = user.composing {
                            msg.body.extend_from_slice(line.as_bytes());
                            msg.body.push(b'\n');
                        }
                    }
                }
            }

            UserState::ChangingName => {
                let new_name =
                    String::from_utf8_lossy(input).trim().to_string();

                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.name = new_name.clone();
                    user.state = UserState::Idle;
                }

                // Send rename notification (C: con_change).
                let notification = self.lang.name_changed(id, &new_name);
                self.send_notification(&notification, id);

                let done = self.lang.done();
                self.append_output(id, &done);
                self.deliver_queued(id);
                self.flush_output(id);
            }

            UserState::ComposingWarning => {
                let text = String::from_utf8_lossy(input).to_string();

                if let Some(Some(user)) = self.users.get_mut(idx) {
                    user.state = UserState::Idle;
                }

                // Post the warning (C: con_newwarn → con_warn).
                self.post_warning(&text);
                self.deliver_queued(id);
                self.flush_output(id);
            }

            _ => {
                debug!(%id, ?state, "ignoring input in unexpected state");
            }
        }

        Ok(false)
    }

    /// Parse and dispatch a command line. Returns `true` if the
    /// conference should end.
    fn dispatch_command(
        &mut self,
        id: UserId,
        input: &[u8],
    ) -> Result<bool, Report> {
        let idx = usize::from(id.0);
        let line = String::from_utf8_lossy(input);

        if let Some(Some(user)) = self.users.get_mut(idx) {
            user.state = UserState::Command;
        }

        let result = match cmd::parse(&line) {
            Some((command, args)) => cmd::dispatch(self, id, command, args),
            None => {
                let err = self.lang.command_error();
                self.append_output(id, &err);
                cmd::CommandResult::Ok
            }
        };

        match result {
            cmd::CommandResult::Ok => {
                // Only return to Idle if the command didn't change state
                // (C: con_command default logic).
                if let Some(Some(user)) = self.users.get_mut(idx) {
                    if user.state == UserState::Command {
                        user.state = UserState::Idle;
                    }
                }
                self.deliver_queued(id);
                self.flush_output(id);
                Ok(false)
            }
            cmd::CommandResult::EndConference => {
                self.terminate_all();
                Ok(true)
            }
        }
    }

    /// Handle send completion — flush any remaining output.
    pub fn handle_send_complete(&mut self, port_id: PortId) {
        if let Some(&id) = self.port_to_user.get(&port_id) {
            self.flush_output(id);
        }
    }

    /// Handle a disconnection. Returns `true` if the conference should end.
    pub fn handle_disconnected(&mut self, port_id: PortId) -> bool {
        let Some(&id) = self.port_to_user.get(&port_id) else {
            return false;
        };

        let idx = usize::from(id.0);

        // Capture info before removing the user.
        let name = self
            .users
            .get(idx)
            .and_then(|s| s.as_ref())
            .map_or_else(String::new, |u| u.name.clone());

        // Auto-normalize: if this user is a submaster, reparent their
        // subordinates back to their parent master before removing.
        self.auto_normalize(id);

        // Send exit notification BEFORE clearing user slot so
        // send_notification can determine subconference membership.
        if !name.is_empty() {
            let notification = self.lang.user_exited(id, &name);
            self.send_notification(&notification, id);
        }

        // Record in left list.
        if !name.is_empty() {
            self.left.push(LeftUser {
                id,
                name: name.clone(),
                left_at: Instant::now(),
                departed_at: SystemTime::now(),
                was_killed: false,
            });
        }

        self.port_to_user.remove(&port_id);

        if let Some(slot) = self.users.get_mut(idx) {
            *slot = None;
        }

        self.current_count = self.current_count.saturating_sub(1);
        info!(%id, "user disconnected");

        self.current_count == 0 && self.peak_count > 0
    }

    // ── Output helpers ──────────────────────────────────────────────

    /// Append text to a user's output buffer.
    pub(crate) fn append_output(&mut self, id: UserId, text: &str) {
        let idx = usize::from(id.0);
        if let Some(Some(user)) = self.users.get_mut(idx) {
            user.output.extend_from_slice(text.as_bytes());
        }
    }

    /// Append raw bytes to a user's output buffer.
    pub(crate) fn append_output_bytes(&mut self, id: UserId, data: &[u8]) {
        let idx = usize::from(id.0);
        if let Some(Some(user)) = self.users.get_mut(idx) {
            user.output.extend_from_slice(data);
        }
    }

    /// Try to send the user's buffered output to the port.
    pub(crate) fn flush_output(&mut self, id: UserId) {
        let idx = usize::from(id.0);
        if let Some(Some(user)) = self.users.get_mut(idx) {
            Self::flush_user_output(user);
        }
    }

    /// Flush output for a user reference (usable before insertion into vec).
    fn flush_user_output(user: &mut User) {
        if !user.output.is_empty() {
            let data = user.output.split().freeze();
            if let Err(data) = user.port.try_send(data) {
                user.output.extend_from_slice(&data);
            }
        }
    }

    // ── Notification system ─────────────────────────────────────────

    /// Send a notification to users who accept notifications, scoped to
    /// the excluded user's subconference.
    ///
    /// Skips users in Login state, users with reject_notifications set,
    /// and users who are Out (C: `con_notify`).
    ///
    /// The `exclude` user must still be present in the users array so
    /// that subconference membership can be determined.
    pub(crate) fn send_notification(&mut self, text: &str, exclude: UserId) {
        // Get subconference info for the excluded user.
        let exclude_idx = usize::from(exclude.0);
        let exclude_master = self
            .users
            .get(exclude_idx)
            .and_then(|s| s.as_ref())
            .map(|u| u.master);

        let mut recipients = UserSet::new();
        for slot in &self.users {
            let Some(user) = slot else { continue };
            if user.id == exclude {
                continue;
            }
            if user.state == UserState::Login {
                continue;
            }
            if user.prefs.reject_notifications {
                continue;
            }
            if user.state == UserState::Out {
                continue;
            }
            // Subconference scoping: only notify users visible from
            // exclude's subconference.
            if let Some(excl_master) = exclude_master {
                if !(user.master == excl_master
                    || user.master == exclude
                    || user.id == excl_master)
                {
                    continue;
                }
            }
            recipients.insert(user.id);
        }

        if recipients.is_empty() {
            return;
        }

        let msg = Arc::new(Message {
            from: None,
            kind: MessageKind::Notification,
            recipients,
            body: text.as_bytes().to_vec(),
        });

        for target_id in recipients.iter() {
            let t_idx = usize::from(target_id.0);
            if let Some(Some(user)) = self.users.get_mut(t_idx) {
                user.queue.push_back(Arc::clone(&msg));
            }
        }

        for target_id in recipients.iter() {
            self.deliver_queued(target_id);
        }
    }

    // ── Message delivery ────────────────────────────────────────────

    /// Deliver a composed message from a user (C: `con_deliver`).
    fn deliver_composed(&mut self, from_id: UserId) {
        let from_idx = usize::from(from_id.0);

        // Take the composing message.
        let msg = if let Some(Some(user)) = self.users.get_mut(from_idx) {
            user.composing.take()
        } else {
            return;
        };
        let Some(msg) = msg else { return };

        // Get sender name.
        let from_name = if let Some(Some(user)) = self.users.get(from_idx) {
            user.name.clone()
        } else {
            return;
        };

        // Empty body → "Empty message" (C: stream_length == 0).
        if msg.body.is_empty() {
            let empty = self.lang.empty_message();
            self.append_output(from_id, &empty);
            return;
        }

        // Build header.
        let header = match msg.kind {
            MessageKind::TellAll => {
                self.lang.message_to_all_from(from_id, &from_name)
            }
            _ => self.lang.message_from(from_id, &from_name),
        };

        // Prepend header to body.
        let mut body =
            Vec::with_capacity(header.len().saturating_add(msg.body.len()));
        body.extend_from_slice(header.as_bytes());
        body.extend_from_slice(&msg.body);

        let arc_msg = Arc::new(Message {
            from: Some(from_id),
            kind: msg.kind,
            recipients: msg.recipients,
            body,
        });

        // Determine who can receive (C: con_post / con_post_one).
        let mut to_enqueue = UserSet::new();
        for target_id in msg.recipients.iter() {
            let t_idx = usize::from(target_id.0);
            let Some(Some(target)) = self.users.get(t_idx) else {
                continue;
            };
            if target.ignoring.contains(from_id) {
                continue;
            }
            if target.state == UserState::Out {
                continue;
            }
            if msg.kind == MessageKind::TellAll && target.prefs.reject_tell_all
            {
                continue;
            }
            to_enqueue.insert(target_id);
        }

        // Enqueue to eligible recipients.
        for target_id in to_enqueue.iter() {
            let t_idx = usize::from(target_id.0);
            if let Some(Some(user)) = self.users.get_mut(t_idx) {
                user.queue.push_back(Arc::clone(&arc_msg));
            }
        }

        // Report to sender.
        if to_enqueue.count() > 0 {
            let sent = self.lang.message_sent();
            self.append_output(from_id, &sent);
        } else {
            let no_rcpt = self.lang.no_recipients();
            self.append_output(from_id, &no_rcpt);
        }

        // Deliver to idle recipients.
        for target_id in to_enqueue.iter() {
            self.deliver_queued(target_id);
        }
    }

    /// Deliver queued messages to a user who is idle, then flush output.
    pub(crate) fn deliver_queued(&mut self, id: UserId) {
        let idx = usize::from(id.0);
        let Some(Some(user)) = self.users.get_mut(idx) else {
            return;
        };
        if user.state != UserState::Idle {
            return;
        }
        while let Some(msg) = user.queue.pop_front() {
            if user.prefs.reject_controls {
                // C: user_string strips control chars, keeping whitespace.
                for &b in &msg.body {
                    if b.is_ascii_control() {
                        if b.is_ascii_whitespace() {
                            user.output.extend_from_slice(&[b]);
                        }
                        // else: drop the control char
                    } else {
                        user.output.extend_from_slice(&[b]);
                    }
                }
            } else {
                user.output.extend_from_slice(&msg.body);
            }
        }
        Self::flush_user_output(user);
    }

    // ── Warning system ──────────────────────────────────────────────

    /// Post a warning to all connected users (C: `con_warn`).
    pub(crate) fn post_warning(&mut self, text: &str) {
        // Warning body: "\n{text}\n" (C: "\r\n" + text + "\r\n").
        let mut body = Vec::with_capacity(text.len().saturating_add(2));
        body.push(b'\n');
        body.extend_from_slice(text.as_bytes());
        body.push(b'\n');

        let msg = Arc::new(Message {
            from: None,
            kind: MessageKind::Warning,
            recipients: UserSet::new(), // Not used for warnings.
            body,
        });

        // Enqueue to all connected users (warnings cannot be ignored).
        for slot in &mut self.users {
            let Some(user) = slot else { continue };
            user.queue.push_back(Arc::clone(&msg));
        }

        // Store as current warning.
        self.current_warning = Some(msg);

        // Deliver to idle users.
        let idle_ids: Vec<UserId> = self
            .users
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|u| u.state == UserState::Idle)
            .map(|u| u.id)
            .collect();
        for uid in idle_ids {
            self.deliver_queued(uid);
        }
    }

    // ── User lifecycle ──────────────────────────────────────────────

    /// Terminate the conference: send `conterm` to all users, flush
    /// output, shut down all ports, and clear state (C: `con_shutdown`).
    fn terminate_all(&mut self) {
        let msg = self.lang.conference_terminated();
        for slot in &mut self.users {
            let Some(user) = slot else { continue };
            user.output.extend_from_slice(msg.as_bytes());
            Self::flush_user_output(user);
            let _ = user.port.shutdown();
        }
        self.users.clear();
        self.port_to_user.clear();
        self.current_count = 0;
    }

    /// Disconnect a user — record in left list, send notification.
    ///
    /// Used by voluntary disconnect (bye). The port shutdown
    /// is the caller's responsibility.
    pub(crate) fn disconnect_user(&mut self, id: UserId, was_killed: bool) {
        let idx = usize::from(id.0);

        let (name, port_id) = if let Some(Some(user)) = self.users.get(idx) {
            (user.name.clone(), Some(user.port.id()))
        } else {
            return;
        };

        // Auto-normalize: if this user is a submaster, reparent their
        // subordinates back to their parent master before removing.
        self.auto_normalize(id);

        // Send notification BEFORE clearing user slot so
        // send_notification can determine subconference membership.
        let notification = if was_killed {
            self.lang.user_killed(id, &name)
        } else {
            self.lang.user_exited(id, &name)
        };
        self.send_notification(&notification, id);

        // Record in left list.
        self.left.push(LeftUser {
            id,
            name: name.clone(),
            left_at: Instant::now(),
            departed_at: SystemTime::now(),
            was_killed,
        });

        // Remove port mapping.
        if let Some(pid) = port_id {
            self.port_to_user.remove(&pid);
        }

        // Clear user slot.
        if let Some(slot) = self.users.get_mut(idx) {
            *slot = None;
        }

        self.current_count = self.current_count.saturating_sub(1);
    }

    // ── Subconference lifecycle ───────────────────────────────────

    /// Auto-normalize: if `id` is a submaster, reparent their
    /// subordinates to `id`'s parent master, send "talking with" to
    /// each subordinate, and send "passed" + listing to the parent.
    ///
    /// Called before removing a submaster (disconnect) so their
    /// subordinates are returned to the parent. The user at `id` must
    /// still be present in the users array.
    pub(crate) fn auto_normalize(&mut self, id: UserId) {
        if let Some((parent, subs)) = self.auto_normalize_reparent(id) {
            self.send_passed_listing(parent, &subs);
        }
    }

    /// Reparent subordinates of submaster `id` to their parent master
    /// and send "talking with" to each. Returns `(parent_master,
    /// subordinate_ids)` if `id` was a submaster, or `None` otherwise.
    ///
    /// Does NOT send "passed" + listing — caller decides when.
    pub(crate) fn auto_normalize_reparent(
        &mut self,
        id: UserId,
    ) -> Option<(UserId, Vec<UserId>)> {
        let idx = usize::from(id.0);

        let (is_submaster, parent_master) =
            if let Some(Some(user)) = self.users.get(idx) {
                (matches!(user.role, Role::Submaster { .. }), user.master)
            } else {
                return None;
            };

        if !is_submaster {
            return None;
        }

        // Find subordinates (users whose master is `id`, excluding self).
        let subordinates: Vec<UserId> = self
            .users
            .iter()
            .filter_map(|s| s.as_ref())
            .filter(|u| u.master == id && u.id != id)
            .map(|u| u.id)
            .collect();

        // Reparent subordinates to the parent master.
        for &sub_id in &subordinates {
            let sub_idx = usize::from(sub_id.0);
            if let Some(Some(user)) = self.users.get_mut(sub_idx) {
                user.master = parent_master;
            }
        }

        // Send "talking with" to each reparented subordinate.
        let parent_name = self
            .users
            .get(usize::from(parent_master.0))
            .and_then(|s| s.as_ref())
            .map(|u| u.name.clone())
            .unwrap_or_default();
        for &sub_id in &subordinates {
            let talking = self.lang.talking_with(parent_master, &parent_name);
            self.append_output(sub_id, "\n");
            self.append_output(sub_id, &talking);
            self.flush_output(sub_id);
        }

        Some((parent_master, subordinates))
    }

    /// Send "you have been passed" + WHO listing to `master_id`.
    pub(crate) fn send_passed_listing(
        &mut self,
        master_id: UserId,
        subordinates: &[UserId],
    ) {
        let passed = self.lang.you_are_passed();
        self.append_output(master_id, &passed);

        let mut listing = String::new();
        for &sub_id in subordinates {
            self.format_who_entry(sub_id, master_id, &mut listing);
        }
        if !listing.is_empty() {
            self.append_output(master_id, &listing);
        }
        self.flush_output(master_id);
    }

    // ── Argument parsing ────────────────────────────────────────────

    /// Parse comma-separated user numbers from command args (C: `cmd_argmap`).
    ///
    /// Returns `(target_set, rest_after_semicolon)`.
    /// An empty target set means "no numbers given" (tell-all).
    /// Returns `Err(())` on invalid input (error already sent to user).
    pub(crate) fn parse_user_args<'a>(
        &mut self,
        who: UserId,
        args: &'a str,
    ) -> Result<(UserSet, &'a str), ()> {
        let trimmed = args.trim_start();

        // Split on first ';'.
        let (nums_part, rest) = match trimmed.find(';') {
            Some(pos) => {
                // Safety: pos is a valid char boundary from find().
                let (before, after) = trimmed.split_at(pos);
                // Skip the ';' itself.
                (before, &after[1..])
            }
            None => (trimmed, ""),
        };

        let nums_trimmed = nums_part.trim();
        if nums_trimmed.is_empty() {
            return Ok((UserSet::new(), rest));
        }

        let mut targets = UserSet::new();

        for part in nums_trimmed.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            let val: u8 = match part.parse() {
                Ok(v) => v,
                Err(_) => {
                    let msg = self.lang.invalid_arguments();
                    self.append_output(who, &msg);
                    return Err(());
                }
            };
            let target_id = UserId(val);
            let t_idx = usize::from(val);

            // Check user exists and is connected.
            let valid =
                self.users.get(t_idx).and_then(|s| s.as_ref()).is_some();
            if !valid {
                let msg = self.lang.invalid_arguments();
                self.append_output(who, &msg);
                return Err(());
            }

            targets.insert(target_id);
        }

        Ok((targets, rest))
    }

    // ── Command formatting ──────────────────────────────────────────

    /// Format the composing-state display for WHO/IM entries.
    fn format_composing_state(user: &User) -> String {
        let Some(ref msg) = user.composing else {
            return "---".to_string();
        };
        if msg.kind == MessageKind::TellAll {
            return "B/A".to_string();
        }
        if msg.recipients.count() > 1 {
            return "B/P".to_string();
        }
        // Single recipient — show B-N.
        if let Some(target) = msg.recipients.iter().next() {
            let mut s = String::with_capacity(3);
            let _ = write!(s, "B-{}", target.0);
            return s;
        }
        "---".to_string()
    }

    /// Format the state column for a user in WHO/IM display (C: `cmd_who_entry`).
    fn format_state_display(user: &User) -> String {
        match user.state {
            UserState::Idle | UserState::JustLoggedIn => "---".to_string(),
            UserState::Composing | UserState::Composed => {
                Self::format_composing_state(user)
            }
            UserState::Command => "COM".to_string(),
            UserState::Login => "E/N".to_string(),
            UserState::Receiving(_) => "R/M".to_string(),
            UserState::ChangingName | UserState::NameChanged => {
                "C/N".to_string()
            }
            UserState::Out => "OUT".to_string(),
            UserState::ComposingWarning | UserState::WarningComposed => {
                "WRN".to_string()
            }
            UserState::Line => "LIN".to_string(),
        }
    }

    /// Format the ignore column for a WHO entry (C logic for I/X/B flags).
    ///
    /// - `I`: viewer is ignoring the listed user
    /// - `X`: listed user is ignoring the viewer
    /// - `B`: mutual ignore (both directions)
    fn format_ignore_flag(
        listed: &User,
        viewer_id: UserId,
        viewer_ignoring_listed: bool,
    ) -> &'static str {
        let listed_ignoring_viewer = listed.ignoring.contains(viewer_id);
        match (listed_ignoring_viewer, viewer_ignoring_listed) {
            (true, true) => "B",
            (true, false) => "X",
            (false, true) => "I",
            (false, false) => " ",
        }
    }

    /// Format a single WHO/IM entry line (C: `cmd_who_entry` with
    /// FMT_STAT | FMT_NAME).
    pub(crate) fn format_who_entry(
        &self,
        user_id: UserId,
        viewer_id: UserId,
        output: &mut String,
    ) {
        self.format_who_entry_ex(
            user_id,
            viewer_id,
            WhoColumns::StatName,
            output,
        );
    }

    /// Format a WHO-style entry with configurable columns (C:
    /// `cmd_who_entry` with format bitmask).
    pub(crate) fn format_who_entry_ex(
        &self,
        user_id: UserId,
        viewer_id: UserId,
        columns: WhoColumns,
        output: &mut String,
    ) {
        let u_idx = usize::from(user_id.0);
        let Some(Some(user)) = self.users.get(u_idx) else {
            return;
        };

        let marker = if user_id == self.new_user_master {
            "=>"
        } else {
            "  "
        };

        match columns {
            WhoColumns::PortName => {
                let location = if user.prefs.reject_ports {
                    "Rejecting ports"
                } else {
                    user.port.port_address()
                };
                let _ = writeln!(
                    output,
                    "{marker}[#{id}]  {location} {name}",
                    id = user.id,
                    name = user.name,
                );
            }
            WhoColumns::StatName | WhoColumns::Everything => {
                let role = match user.role {
                    Role::Master { .. } | Role::Submaster { .. } => "M",
                    Role::Normal => " ",
                };
                let state = Self::format_state_display(user);

                let v_idx = usize::from(viewer_id.0);
                let viewer_ignoring_listed = self
                    .users
                    .get(v_idx)
                    .and_then(|s| s.as_ref())
                    .is_some_and(|v| v.ignoring.contains(user_id));

                let r = if user.prefs.reject_tell_all { "R" } else { " " };
                let ig = Self::format_ignore_flag(
                    user,
                    viewer_id,
                    viewer_ignoring_listed,
                );
                let p = if user.prefs.reject_ports { "P" } else { " " };
                let c = if user.prefs.reject_controls { "C" } else { " " };
                let n = if !user.prefs.reject_notifications {
                    "N"
                } else {
                    " "
                };

                if matches!(columns, WhoColumns::Everything) {
                    let location = if user.prefs.reject_ports {
                        "Rejecting ports"
                    } else {
                        user.port.port_address()
                    };
                    let _ = writeln!(
                        output,
                        "{marker}[#{id}] {role} {state} {r}{ig}{p}{c}{n}  {location} {name}",
                        id = user.id,
                        name = user.name,
                    );
                } else {
                    let _ = writeln!(
                        output,
                        "{marker}[#{id}] {role} {state} {r}{ig}{p}{c}{n}  {name}",
                        id = user.id,
                        name = user.name,
                    );
                }
            }
        }
    }

    /// Check if `user_id` is visible from `viewer_id`'s subconference.
    ///
    /// Uses the C 3-way scoping rule:
    /// - Same master (siblings + self)
    /// - User's master is viewer (subordinates)
    /// - User IS viewer's master
    pub(crate) fn is_in_subconference(
        &self,
        user_id: UserId,
        viewer_id: UserId,
    ) -> bool {
        let u_idx = usize::from(user_id.0);
        let v_idx = usize::from(viewer_id.0);

        let Some(Some(user)) = self.users.get(u_idx) else {
            return false;
        };
        let Some(Some(viewer)) = self.users.get(v_idx) else {
            return false;
        };

        user.master == viewer.master
            || user.master == viewer_id
            || user.id == viewer.master
    }

    /// Format the WHO listing.
    pub(crate) fn format_who(&self, requester_id: UserId) -> String {
        let mut output = String::from("\n");

        for slot in &self.users {
            let Some(user) = slot else { continue };
            if !self.is_in_subconference(user.id, requester_id) {
                continue;
            }
            self.format_who_entry(user.id, requester_id, &mut output);
        }

        output
    }
}
