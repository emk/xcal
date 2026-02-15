# Proposed UX Improvements for Xcaliber Rust

As described in [`DCTS_XCAL_BACKGROUND.md`](./DCTS_XCAL_BACKGROUND.md), the original Xcaliber conference system ran on 45-minute cycle. This may have been because of limits on how long a single interactive job could run. But it led to fascinating mechanism where whoever "linked" a given copy of Xcaliber as "XYZ" would be the global moderator for the next 45 minutes. When the conference restarted, people would scramble to see who could relink it first, sometimes even resorting to batch files. And during the heyday of Xcaliber, it was running almost constantly:

> Whole social groups sprang up around "the con", and at its height, XYZ was linked and active from 07.00 when D1 came up 'til 03.00 when it went down for backups.
>
> — [Xcaliber Mark II README](../README.md)

But this activity cycle effectively means that Xcaliber is a _synchronous_ conference system, one that relies on a critical level of people engaging with it when a user connects. And this is what effectively killed Xcaliber, when Dartmouth's BlitzMail introduced email that was delivered at instant-messaging speeds.

So any serious attempt to rebuild Xcaliber needs to figure out _some_ way to work around the synchronous, emphemeral nature of the system.

And this, in turn, raises several issues:

- The historical experience of Xcaliber is that it was up and running, and that people were there. Without recreating this, you can't recreate the feeling.
- But we _do_ want to preserve as many features of the historic experience as possible. So any changes we make make will need to occur at the "edges" of the system.
- And of course, there are other UI warts we might want to address.

## Issue 1: Emphemeral conferences and crtical mass

The solution here, I suspect, is to introduce a "lobby": A web page which can remain open indefinitely, which:

- Provides a count of "online" users who are waiting for a conference.
- Allows "linking" a conference.
- Notifies other waiting users with a background sound that a conference has started.

Optionally, this might show the handles of logged-in users when a conference is active, allowing people to decide if it's worth joining?

This would be fairly intricate: It would exist _outside_ the "port" abstractions provided by the app. It would need to support web sockets with connection resumption. It should maintain identity and "connections" over page reloads, connection back to the same DCTS-layer "port". It needs, at minimum, to be able to detect conference lifecycle events.

Eventually, the "lobby" page could be a mini-museum: Some history of DCTS and Xcaliber, with interesting quotes. Conference software so good, people once committed federal crimes to use it!

## Issue 2: Visually Unfamiliar UI

Unlike Unix, Xcaliber doesn't even have _prompt characters_. This makes it hard for modern eyes to understand screen shots. At a minimum, we should consider using two levels of text brightness: normal for output, and "bright" for user input. This was available on many monochrome phosphor terminals, even if it was unlikely to have been used this way for most DCTS terminals. Still, it's a gentle improvement, one which doesn't disrupt the experience.

## Issue 3: Onboarding

The initial few minutes of a user's experience are critical. They can already _receive_ messages, but they also need to send them. We might want to bind key protocol features to buttons:

- Help: Run the `help` command.
- Who: Run the `who` command, to show online users.
- Talk: Enter a single line of text, send with `t;Text goes here`.

Ideally, we would build these commands for the user, and print them on-screen as regular user input. The actual UI widgets could be two left-justified buttons below the terminal UI, and "Talk" could open show a single-line input field filling the space on the bottom.

## Issue 4: Lifecycle

There are several issues we might want to consider around conference lifecycle.

- Should we extend conference lifecycles beyond the authentic 45 minutes? Going with 3 hours or 12 hours might preserve _some_ of the synchronous feeling, without requiring users to be actively flunking out of Dartmouth because they're XYZ addicts.
- In Xcaliber Mark II, when the conference master disconnects, it immediately ends the current conference. Would it be better to _promote_ the oldest user to master?

These could both be controlled under command line flags, perhaps?

## Issue 5: Nobody is ever there!

I almost hate to suggest this, but it seems like it might even be worth adding a `bot` command that the master user could use to summon a single LLM-based chatbot, probably something small and cheap. This would likely require adding a custom DCTS port type, and some kind of rate-limited LLM client.