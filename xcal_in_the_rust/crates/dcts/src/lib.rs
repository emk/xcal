//! DCTS "OS layer" abstractions.
//!
//! Simulates a few OS-level abstractions from the Dartmouth College
//! Time-Sharing System (DCTS), which predates Unix. This is not intended to be
//! especially historically authentic in detail, but it _is_ intended to prevent
//! casually mixing "OS-level" concerns like ports with "application-level"
//! concerns.

pub mod ports;
