xcalr uses raw TCP without telnet negotiation, so there is no IAC BRK
signal to reject or accept. The `rb` and `ab` commands exist as no-ops
for protocol compatibility, but this fixture requires actual telnet BRK
handling to test meaningfully.
