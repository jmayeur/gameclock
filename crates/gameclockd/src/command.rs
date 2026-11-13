//! The commands of the control socket and their replies
//! (`openspec/specs/command-input`, docs/adr/0016).
//!
//! These are pure functions. The socket code only moves bytes.

use core::fmt;

use gameclock_core::Brightness;
use jiff::tz::TimeZone;

use crate::Mode;

/// The longest command line, in bytes, without the line feed.
pub const MAX_LINE: usize = 256;

/// The reply to a command that worked and has no data.
pub const OK: &str = "ok";

/// The reply when a client sends no complete line in time.
pub const NO_COMMAND: &str = "error: no command in 1 s.";

/// The reply when the loop does not answer in time.
pub const NO_ANSWER: &str = "error: the daemon did not answer in 2 s.";

/// A command from the control socket.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Make the base scene of this mode the base scene.
    Mode(Mode),
    /// Give the mode, the time zone, and the brightness.
    Status,
}

/// A command line that is not a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    /// The line has more than [`MAX_LINE`] bytes.
    TooLong,
    /// The line is not UTF-8 text.
    NotUtf8,
    /// The line is not the name of a command.
    Unknown(String),
}

impl CommandError {
    /// The reply line for this error.
    #[must_use]
    pub fn reply(&self) -> String {
        format!("error: {self}")
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong => write!(f, "the command is longer than {MAX_LINE} bytes."),
            Self::NotUtf8 => f.write_str("the command is not UTF-8 text."),
            Self::Unknown(name) => write!(
                f,
                "unknown command '{name}'. The commands are clock, test-pattern, and status."
            ),
        }
    }
}

impl std::error::Error for CommandError {}

/// Parse one command line, without its line feed. Spaces at the start and
/// the end do not count.
///
/// # Errors
/// [`CommandError`] if the line is too long, is not UTF-8, or is not a
/// command.
pub fn parse_command(line: &[u8]) -> Result<Command, CommandError> {
    if line.len() > MAX_LINE {
        return Err(CommandError::TooLong);
    }
    let text = core::str::from_utf8(line).map_err(|_| CommandError::NotUtf8)?;
    match text.trim() {
        "clock" => Ok(Command::Mode(Mode::Clock)),
        "test-pattern" => Ok(Command::Mode(Mode::TestPattern)),
        "status" => Ok(Command::Status),
        other => Err(CommandError::Unknown(other.to_owned())),
    }
}

/// The reply to `status`.
#[must_use]
pub fn status_reply(mode: Mode, zone: &TimeZone, brightness: Brightness) -> String {
    let zone = zone.iana_name().unwrap_or("unnamed");
    format!(
        "ok mode={} zone={zone} brightness={}",
        mode.name(),
        brightness.percent()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;
    use gameclock_core::Brightness;
    use jiff::tz::TimeZone;

    #[test]
    fn the_three_commands_parse() {
        assert_eq!(parse_command(b"clock"), Ok(Command::Mode(Mode::Clock)));
        assert_eq!(
            parse_command(b"test-pattern"),
            Ok(Command::Mode(Mode::TestPattern))
        );
        assert_eq!(parse_command(b"status"), Ok(Command::Status));
    }

    #[test]
    fn unknown_command() {
        let err = parse_command(b"frobnicate").unwrap_err();
        assert_eq!(err, CommandError::Unknown("frobnicate".into()));
        assert_eq!(
            err.reply(),
            "error: unknown command 'frobnicate'. The commands are clock, test-pattern, and status."
        );
    }

    #[test]
    fn commands_are_exact() {
        for text in ["Clock", "STATUS", "clock now", ""] {
            assert!(parse_command(text.as_bytes()).is_err(), "{text:?}");
        }
    }

    #[test]
    fn line_too_long() {
        let err = parse_command(&[b'a'; 300]).unwrap_err();
        assert_eq!(err, CommandError::TooLong);
        assert_eq!(err.reply(), "error: the command is longer than 256 bytes.");
    }

    #[test]
    fn a_line_of_256_bytes_is_not_too_long() {
        let mut line = b"status".to_vec();
        line.resize(256, b' ');
        assert_eq!(parse_command(&line), Ok(Command::Status));
    }

    #[test]
    fn text_that_is_not_utf8() {
        let err = parse_command(&[0xFF, 0xFE]).unwrap_err();
        assert_eq!(err, CommandError::NotUtf8);
        assert_eq!(err.reply(), "error: the command is not UTF-8 text.");
    }

    #[test]
    fn spaces_around_the_command() {
        assert_eq!(parse_command(b" status "), Ok(Command::Status));
        assert_eq!(parse_command(b"\tclock\r"), Ok(Command::Mode(Mode::Clock)));
    }

    #[test]
    fn mode_reply() {
        assert_eq!(OK, "ok");
    }

    #[test]
    fn status_of_the_clock() {
        let reply = status_reply(Mode::Clock, &TimeZone::UTC, Brightness::new(25));
        assert_eq!(reply, "ok mode=clock zone=UTC brightness=25");
    }

    #[test]
    fn status_of_the_test_pattern() {
        let reply = status_reply(Mode::TestPattern, &TimeZone::UTC, Brightness::MAX);
        assert_eq!(reply, "ok mode=test-pattern zone=UTC brightness=100");
    }

    #[test]
    fn status_with_an_unnamed_zone() {
        let zone = TimeZone::fixed(jiff::tz::offset(2));
        let reply = status_reply(Mode::Clock, &zone, Brightness::new(60));
        assert_eq!(reply, "ok mode=clock zone=unnamed brightness=60");
    }

    #[test]
    fn limit_replies() {
        assert_eq!(NO_COMMAND, "error: no command in 1 s.");
        assert_eq!(NO_ANSWER, "error: the daemon did not answer in 2 s.");
    }
}
