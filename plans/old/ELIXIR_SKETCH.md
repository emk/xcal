# Xcaliber Mark II — Elixir Rewrite Sketch

## Approach

Elixir + Phoenix + LiveView. The conference is a GenServer process, each user
connection is a LiveView process, and the real-time UI is server-rendered HTML
pushed over WebSocket — with zero custom JavaScript.


## Why Elixir is a natural fit

The original Xcaliber Mark II is a hand-built actor model in C: a single
`select()` loop multiplexing messages between independent user sessions, each
with their own state machine and message queue. Elixir/OTP *is* that model
natively:

| Original C concept                  | Elixir equivalent                   |
|--------------------------------------|-------------------------------------|
| `select()` main loop                | The BEAM scheduler                  |
| Per-user state + message queue       | One process per user (with mailbox) |
| Conference shared state             | GenServer holding conference state  |
| `user_enqueue()` / `user_transmit()`| `send()` / `receive`               |
| `con_notify()` broadcast            | `Phoenix.PubSub.broadcast()`       |
| Status line push updates            | LiveView re-render                  |
| Subconference hierarchy             | Data structure in GenServer state   |
| `signal(SIGINT, ...)`               | `Process.monitor()` / supervisors  |


## Dependencies

- **Phoenix** — Web framework with WebSocket/channel infrastructure.
- **Phoenix LiveView** — Real-time server-rendered UI over WebSocket. This
  eliminates the need to write any JavaScript for the chat interface.
- **Phoenix PubSub** — Pub/sub for broadcasting events (join/leave/messages)
  to LiveView processes.

That's essentially it. No database needed. No external services.


## Architecture

```
Browser ←—LiveView WebSocket—→ LiveView process (one per connection)
                                    ↕ GenServer.call/cast
                               Conference GenServer (owns all shared state)
                                    ↕ PubSub broadcasts
                               All LiveView processes (receive updates)
```

### The conference GenServer

Holds all shared state. All mutations go through it.

```elixir
defmodule Xcal.Conference do
  use GenServer

  defstruct [
    users: %{},          # %{user_id => %User{}}
    left: [],            # recently disconnected users
    warning: nil,        # current conference-wide warning
    master: nil,         # user_id of master receiving new connections
    started_at: nil,
    duration: nil,
    next_id: 0
  ]

  defmodule User do
    defstruct [
      :id, :name, :state, :flags, :master_id,
      :ignore_set, :connected_at, :pid
    ]
  end
end
```

### User states

```elixir
# States that need tracking in the conference
@type user_state ::
  :login        # entering name
  | :idle       # waiting, messages delivered
  | {:composing, recipients, lines}  # building a message
  | :out        # unavailable
  | :line       # line mode, commands only
```

Unlike the C version, there's no RECV_ST — message delivery is just a
`send()` to the LiveView process, which is non-blocking. No DONE_ST, CMD_ST,
or other transient states either — those become synchronous function calls
within a `GenServer.handle_cast`.


### LiveView process (one per user)

Each browser connection is a LiveView process that:
1. Holds its own connection-specific state (composing buffer, pending messages)
2. Sends commands to the Conference GenServer
3. Subscribes to PubSub topics for real-time updates
4. Renders the UI server-side and pushes diffs to the browser

```elixir
defmodule XcalWeb.ConferenceLive do
  use XcalWeb, :live_view

  def mount(_params, _session, socket) do
    Phoenix.PubSub.subscribe(Xcal.PubSub, "conference")
    {:ok, assign(socket, state: :login, messages: [], composing: nil)}
  end

  # User submits a line of text
  def handle_event("submit_line", %{"text" => text}, socket) do
    case socket.assigns.state do
      :login ->
        Xcal.Conference.login(socket.assigns.user_id, text)
        {:noreply, assign(socket, state: :idle)}

      :idle ->
        # Parse as command
        Xcal.Conference.command(socket.assigns.user_id, text)
        {:noreply, socket}

      {:composing, _recipients, _lines} ->
        handle_compose_line(socket, text)
    end
  end

  # Conference broadcasts a message for this user
  def handle_info({:message, msg}, socket) do
    {:noreply, assign(socket, messages: socket.assigns.messages ++ [msg])}
  end

  # Conference broadcasts a who-list update
  def handle_info({:who_update, users}, socket) do
    {:noreply, assign(socket, who_list: users)}
  end
end
```

### Messages don't interrupt composition

This is trivially natural in Elixir. When the user is composing:

1. Messages arrive in the LiveView process mailbox via PubSub.
2. The `handle_info` callback appends them to a `pending_messages` list.
3. The template renders them in a separate area (or hides them).
4. When composition finishes, pending messages appear in the main scrollback.

Or even simpler: always show messages in the scrollback area, and keep the
composition area separate. The user sees messages arrive but their typing
isn't disrupted — which is arguably better than the original's behavior.

The key insight is that Erlang's process mailbox *is* the message queue that
the C code hand-builds with `mqh`/`mqt` linked lists. Not reading from your
mailbox is the default state — you have to actively `receive` to get messages.


## Command system

The wildcard abbreviation matching can be clean pattern matching:

```elixir
defmodule Xcal.Command do
  @commands [
    {"*t*e*ll", :tell},
    {"w*ho", :who},
    {"by*e", :bye},
    {"c*l*ock", :clock},
    # ...
  ]

  def parse(input) do
    {cmd, args} = split_first_word(input)
    case match_command(cmd, @commands) do
      {:ok, handler} -> {handler, String.trim(args)}
      :error -> {:unknown, input}
    end
  end

  defp match_command(input, [{pattern, handler} | rest]) do
    if wildcard_match?(pattern, input), do: {:ok, handler}, else: match_command(input, rest)
  end
end
```

Command handlers are functions on the Conference GenServer:

```elixir
def handle_cast({:command, user_id, :tell, args}, state) do
  {recipients, body} = parse_tell_args(args)
  # validate recipients, check ignore lists, post message
  {:noreply, post_message(state, user_id, recipients, body)}
end
```


## Phoenix LiveView UI

The entire glass TTY interface is a LiveView template — server-rendered HTML
pushed as diffs over the WebSocket. **No custom JavaScript needed** for the
real-time behavior.

```heex
<div class="terminal">
  <div class="scrollback">
    <%= for msg <- @messages do %>
      <div class={"message #{msg.type}"}>
        <%= format_message(msg) %>
      </div>
    <% end %>
  </div>

  <div class="who-list">
    <%= for user <- @who_list do %>
      <div class="status-line"><%= format_status_line(user) %></div>
    <% end %>
  </div>

  <form phx-submit="submit_line">
    <input type="text" name="text" class="input-line"
           placeholder={prompt_for(@state)} autofocus />
  </form>
</div>
```

### Glass TTY theme

```css
body {
    background: #0a0a0a;
    color: #33ff33;
    font-family: "IBM Plex Mono", "Courier New", monospace;
    text-shadow: 0 0 5px rgba(51, 255, 51, 0.4);
}
.message.warning { color: #ff3333; }
.message.notify  { color: #33ff99; font-style: italic; }
.status-line     { white-space: pre; }
.input-line      { background: transparent; color: inherit;
                   border: none; outline: none; width: 100%;
                   font: inherit; caret-color: #33ff33; }
```

### Real-time who-list updates

When any user's state changes (starts composing, goes out, joins, leaves),
the Conference GenServer broadcasts via PubSub:

```elixir
Phoenix.PubSub.broadcast(Xcal.PubSub, "conference", {:who_update, users})
```

Every connected LiveView re-renders its who-list panel. You'd see status
codes change in real time — someone's `---` flips to `B/A` as they start
composing a tell-all. This matches the original's behavior when you re-issued
`who`, but now it's live.


## Module layout

```
lib/
  xcal/
    application.ex        — Supervision tree (Conference, PubSub, Endpoint)
    conference.ex         — GenServer: conference state, message routing
    conference/
      user.ex             — User struct, flags, state
      message.ex          — Message struct and types
      command.ex          — Command table, wildcard matching, dispatch
      commands/
        tell.ex           — Tell command (message sending)
        who.ex            — Who/port/status formatting
        master.ex         — Kill, warn, give, mty, norm
        priv.ex           — xyzzy, xtend, xreset
        filter.ex         — ignore, accept, ra/aa, rn/an, rp, rc/oc
        info.ex           — time, clock, info, left, version
      lang.ex             — i18n string tables
      help.ex             — Help text
  xcal_web/
    router.ex             — Phoenix router
    live/
      conference_live.ex  — LiveView: connection lifecycle, event handling
      conference_live.html.heex  — Template: glass TTY UI
    components.ex         — Shared components (status line, message formatting)
```


## Supervision tree

```
Application
├── Phoenix.PubSub (Xcal.PubSub)
├── Xcal.Conference (GenServer)
└── XcalWeb.Endpoint (Phoenix HTTP/WS)
    └── LiveView processes (one per browser connection, dynamic)
```

The Conference GenServer is supervised. If it crashes (unlikely for pure
computation), the supervisor restarts it — though connected users would need
to reconnect. For a chat server, this is acceptable.


## Build and run

```
mix deps.get
mix phx.server
# Open http://localhost:4000 in a browser
```

Or build a release:

```
MIX_ENV=prod mix release
_build/prod/rel/xcal/bin/xcal start
```

Deployment is a single release tarball. The BEAM VM is included.
Hot code upgrades are possible but not necessary for this use case.


## What Elixir gives you that Rust doesn't (for this project)

- **LiveView eliminates the frontend.** No HTML/CSS/JS to write and maintain
  separately. The UI is a server-side template.
- **PubSub is built in.** Broadcasting state changes to all users is one
  function call, not a channel fan-out you build yourself.
- **Process mailboxes are the message queue.** The core Xcaliber abstraction
  — per-user message queues with selective receive — is the BEAM's native
  execution model.
- **Pattern matching everywhere.** Command dispatch, state transitions, and
  message handling all read naturally.
- **Fault isolation for free.** One user's connection crashing doesn't affect
  others.

## What Rust gives you that Elixir doesn't

- **Single static binary.** No runtime, no BEAM VM.
- **Lower resource footprint.** Matters if deploying to constrained environments.
- **Compile-time type checking.** The `enum UserState` with exhaustive matching
  catches more bugs at compile time than Elixir's runtime pattern matching.
- **Raw performance.** Not that a chat server needs it, but if you ever wanted
  to handle thousands of conferences simultaneously, Rust would scale further.
