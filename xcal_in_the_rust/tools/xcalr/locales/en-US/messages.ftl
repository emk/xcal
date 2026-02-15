# English (US) messages for Xcaliber Mark II
# Converted from lang/en_us.lf
#
# These messages contain text content only. Wire framing (\r\n, BEL) is
# applied by the Rust messages module; transport converts \n to \r\n.

# Fluent terms for AM/PM (shared by clkmsg, upat, upat2)
-am = a.m.
-pm = p.m.

# --- Connection ---

# Conference full (Trailing)
confull = Xcaliber is full--try again later

# Conference terminated (Wrapped)
conterm = Conference "XYZ" has terminated.

# Welcome for normal users (Bare, multiline)
welcome =
    Hello.  Welcome to Xcaliber.
    Please enter your name--

# Name re-prompt after RP sign-on (Bare)
entname = Please enter your name--

# Welcome for master terminal (Bare, multiline, trailing space)
mwelc =
    Hello.  You are the master terminal.
    Please enter your name.{ " " }

# Intro after entering name (Wrapped, multiline)
intro =
    Enter command (type 'HELP' for instructions)
    Changes afoot!  Type EXPLAIN NEW for what's new!

# Entering conference/subcon (Trailing)
tlkwith = You are talking with #{ $number }: { $name }

# --- User notifications ---

# New user connected (Wrapped)
newuser = New user at #{ $number }: { $name }

# Command completed (Wrapped)
donemsg = Done

# Message header for single/multiple (Wrapped)
msgfrom = Message from #{ $number }: { $name }

# Message header for tell-all (Wrapped)
allfrom = Message to all from #{ $number }: { $name }

# Name change (Wrapped)
namenot = New name for #{ $number }: { $name }

# User disconnected (Wrapped)
exnot = Ex-user at #{ $number }: { $name }

# User killed (Wrapped)
killnot = User killed at #{ $number }: { $name }

# --- Composing ---

# Compose prompt (Wrapped)
speak = Speak!

# User(s) out (Wrapped)
outmsg = User(s) out

# Became out (Leading)
youout = You are now out...

# User(s) ignoring (Wrapped)
igmsg = User(s) ignoring you

# Disconnected from conference (Wrapped)
discmsg = You are no longer connected to conference "XYZ"

# No recipients left (Wrapped)
norcpt = Not sent--no recipients

# Message sent (Trailing)
sentmsg = Message sent

# Message not sent via abort (Trailing)
mnsmsg = Message not sent

# Empty message (Wrapped)
empty = Empty message

# --- Errors ---

# Command error (Wrapped)
cmderr = Command error

# Format error (Wrapped)
fmterr = Format error

# Invalid arguments (Wrapped)
invarg = Invalid argument(s)

# Not implemented (Wrapped)
unimp = Command not implemented

# Bad character in input (Wrapped)
badchar = Bad character

# --- Prompts ---

# New warning prompt (Leading)
newwarn = New warning--

# New name prompt (Leading)
newname = New name--

# No one has left (Wrapped)
noleft = No one has left

# No messages pending (Wrapped)
nomsgs = No messages

# Tell-all not enabled (Wrapped)
noalls = Tell-all not enabled

# Command line prompt (Leading)
linmsg = Command line--

# --- Ports/bounce ---

# Can't port while rejecting (Wrapped)
noport = Can't port while rejecting

# Rejecting ports (Wrapped)
rpmsg = Rejecting ports

# Bounced user (Trailing)
bounced = Sorry--you've been bounced

# Bounce list full (Wrapped)
blfull = Bounce list full

# No bounced ports (Wrapped)
nbport = No bounced ports

# --- Master transfer ---

# Became master (Wrapped)
nowmast = You are now a master terminal

# Lost masterhood (Wrapped)
normal = You are no longer a master terminal

# Been passed users (Wrapped)
passed = You have been passed

# Must pass to master (Wrapped)
mptmmsg = Must pass to a master terminal

# New users header for pass (Trailing, leading spaces)
pnumsg = {"  "}New users

# --- Time ---

# Time warning with BEL (BEL framing)
# $count: number, $unit: "second" or "minute"
timewrn = You have about { $count } { $unit ->
    [second] { $count ->
        [one] second
       *[other] seconds
      }
   *[minute] { $count ->
        [one] minute
       *[other] minutes
      }
  } left

# Clock time (Trailing)
clkmsg = Time now: { $hours }:{ $minutes }:{ $seconds } { $ampm ->
    [am] { -am }
   *[pm] { -pm }
  }

# Time left (Trailing)
timelft = Time left:  { $minutes } minute(s)

# --- Help/explain ---

# Can't explain (Wrapped)
cantexp = Can't explain that

# Help file not found (Wrapped)
fnfmsg = Help-file not available

# Help topic key (Bare)
helptop = help

# --- Left/users/info ---

# Left count header (Trailing)
leftmsg = { $count } Users have left Xcaliber

# Left list column header (Trailing)
lefthdr = Left  Name

# Per-user line in LEFT listing (Bare — caller handles newline)
leftentry = { $hours }:{ $minutes }{ $killed } { $name }

# Uptime (Wrapped)
upat = Xcaliber up at { $hours }:{ $minutes }:{ $seconds } { $ampm ->
    [am] { -am }
   *[pm] { -pm }
  }

# Uptime with date (Wrapped)
upat2 = Up at { $hours }:{ $minutes }:{ $seconds } { $ampm ->
    [am] { -am } on { $date }
   *[pm] { -pm } on { $date }
  }

# User count (Wrapped)
numusrs = { $count } user(s)

# Info display (Bare, multiline, leading spaces)
infomsg =
    { " " }Users: { $users }
    { "   " }max: { $max }
    Server: { $server } v.{ $version }

# Version (Wrapped, multiline)
xcalver =
    Xcaliber II v.{ $version } by Michael J. Fromberger
    Copyright (C) 1997-1998 All Rights Reserved

# --- Language ---

# New language prompt (Leading)
newlang = New language--

# Language not available (Wrapped)
nolang = Language not available

# Current language (Wrapped)
curlang = Current language: { $language }

# --- Resources ---

# CRU display (Wrapped, multiline)
crunow =
    CRUs now:  { $crus }
    { "     " }max:  { $max }

# Core size (Leading)
coresize = Core size: { $size }K

# --- Telnet ---

# Are you there response (Trailing)
aytmsg = [Yes]

# Break response (Trailing)
brkmsg = BREAK
