//! Minimal telnet protocol handling.
//!
//! Strips IAC sequences from received data and generates polite refusal
//! responses (WONT/DONT) for any option negotiations.

const IAC: u8 = 0xFF;
const WILL: u8 = 0xFB;
const WONT: u8 = 0xFC;
const DO: u8 = 0xFD;
const DONT: u8 = 0xFE;
const SB: u8 = 0xFA;
const SE: u8 = 0xF0;

/// Result of processing raw bytes from a telnet connection.
pub struct TelnetOutput {
    /// Clean data with all IAC sequences removed.
    pub data: Vec<u8>,
    /// Response bytes to send back (WONT/DONT replies).
    pub responses: Vec<u8>,
}

/// Strip IAC sequences from `input`, returning clean data and any responses
/// that should be sent back to the server.
pub fn process(input: &[u8]) -> TelnetOutput {
    let mut data = Vec::with_capacity(input.len());
    let mut responses = Vec::new();
    let mut i = 0;

    while i < input.len() {
        if input[i] != IAC {
            data.push(input[i]);
            i += 1;
            continue;
        }

        // IAC at end of buffer — skip it (incomplete sequence)
        if i + 1 >= input.len() {
            break;
        }

        let cmd = input[i + 1];
        match cmd {
            // Double IAC = literal 0xFF
            IAC => {
                data.push(IAC);
                i += 2;
            }
            // WILL <opt> → respond DONT <opt>
            WILL => {
                if i + 2 < input.len() {
                    responses.extend_from_slice(&[IAC, DONT, input[i + 2]]);
                    i += 3;
                } else {
                    break;
                }
            }
            // DO <opt> → respond WONT <opt>
            DO => {
                if i + 2 < input.len() {
                    responses.extend_from_slice(&[IAC, WONT, input[i + 2]]);
                    i += 3;
                } else {
                    break;
                }
            }
            // WONT, DONT — 3-byte sequences, just strip
            WONT | DONT => {
                if i + 2 < input.len() {
                    i += 3;
                } else {
                    break;
                }
            }
            // Subnegotiation: skip until IAC SE
            SB => {
                i += 2;
                while i + 1 < input.len() {
                    if input[i] == IAC && input[i + 1] == SE {
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            // All other 2-byte IAC commands (NOP, BRK, etc.) — strip
            _ => {
                i += 2;
            }
        }
    }

    TelnetOutput { data, responses }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_passes_through() {
        let input = b"Hello, world!";
        let out = process(input);
        assert_eq!(out.data, b"Hello, world!");
        assert!(out.responses.is_empty());
    }

    #[test]
    fn double_iac_becomes_literal() {
        let input = &[b'A', IAC, IAC, b'B'];
        let out = process(input);
        assert_eq!(out.data, &[b'A', 0xFF, b'B']);
        assert!(out.responses.is_empty());
    }

    #[test]
    fn will_gets_dont_response() {
        let opt = 0x01; // ECHO
        let input = &[IAC, WILL, opt, b'X'];
        let out = process(input);
        assert_eq!(out.data, b"X");
        assert_eq!(out.responses, &[IAC, DONT, opt]);
    }

    #[test]
    fn do_gets_wont_response() {
        let opt = 0x03; // SGA
        let input = &[IAC, DO, opt, b'Y'];
        let out = process(input);
        assert_eq!(out.data, b"Y");
        assert_eq!(out.responses, &[IAC, WONT, opt]);
    }

    #[test]
    fn wont_and_dont_are_stripped() {
        let input = &[IAC, WONT, 0x01, IAC, DONT, 0x03, b'Z'];
        let out = process(input);
        assert_eq!(out.data, b"Z");
        assert!(out.responses.is_empty());
    }

    #[test]
    fn subnegotiation_is_stripped() {
        let input = &[IAC, SB, 0x18, 0x01, IAC, SE, b'A'];
        let out = process(input);
        assert_eq!(out.data, b"A");
        assert!(out.responses.is_empty());
    }

    #[test]
    fn mixed_telnet_and_text() {
        let input = &[
            IAC, WILL, 0x01, // WILL ECHO → respond DONT ECHO
            b'H', b'i', IAC, DO, 0x03, // DO SGA → respond WONT SGA
            b'!',
        ];
        let out = process(input);
        assert_eq!(out.data, b"Hi!");
        assert_eq!(out.responses, &[IAC, DONT, 0x01, IAC, WONT, 0x03]);
    }

    #[test]
    fn nop_is_stripped() {
        let nop = 0xF1;
        let input = &[IAC, nop, b'A'];
        let out = process(input);
        assert_eq!(out.data, b"A");
        assert!(out.responses.is_empty());
    }
}
