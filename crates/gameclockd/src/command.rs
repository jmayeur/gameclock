//! The commands of the control socket and their replies
//! (`openspec/specs/command-input`, docs/adr/0016).
//!
//! These are pure functions. The socket code only moves bytes.

use core::fmt;

use gameclock_core::Brightness;
use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
use gameclock_core::message::{Message, MessageError};
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Make the base scene of this mode the base scene.
    Mode(Mode),
    /// Give the mode, the time zone, the brightness, and the message count.
    Status,
    /// Show the message, or put it in the queue (docs/adr/0017).
    Message(Message),
    /// Empty the queue, and remove the message on the panel.
    Clear,
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
    /// A `message` command with no text.
    NoText,
    /// A `message` text with more than 64 characters.
    TextTooLong,
    /// A `--passes` value that is not a number from 1 to 10.
    Passes,
    /// A `--color` value that is not six hexadecimal digits.
    Color,
    /// A `message` option that is not `--passes` or `--color`.
    UnknownOption(String),
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
                "unknown command '{name}'. The commands are clock, test-pattern, status, message, and clear."
            ),
            Self::NoText => f.write_str("the message has no text."),
            Self::TextTooLong => write!(
                f,
                "the message is longer than {} characters.",
                Message::MAX_CHARS
            ),
            Self::Passes => write!(
                f,
                "--passes must be a number from 1 to {}.",
                Message::MAX_PASSES
            ),
            Self::Color => {
                f.write_str("--color must be six hexadecimal digits, for example 00ff00.")
            }
            Self::UnknownOption(option) => write!(
                f,
                "unknown message option '{option}'. The options are --passes and --color."
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
    let text = text.trim();
    let (word, rest) = text.split_once(char::is_whitespace).unwrap_or((text, ""));
    if word == "message" {
        return parse_message(rest).map(Command::Message);
    }
    match text {
        "clock" => Ok(Command::Mode(Mode::Clock)),
        "test-pattern" => Ok(Command::Mode(Mode::TestPattern)),
        "status" => Ok(Command::Status),
        "clear" => Ok(Command::Clear),
        other => Err(CommandError::Unknown(other.to_owned())),
    }
}

/// Parse the part of a `message` command after its name: the options, then
/// the text. Options start with `--`. A later option of the same name wins.
fn parse_message(rest: &str) -> Result<Message, CommandError> {
    let mut passes = Message::DEFAULT_PASSES;
    let mut color = Message::DEFAULT_COLOR;
    let mut rest = rest.trim_start();
    while rest.starts_with("--") {
        let (option, tail) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        if let Some(value) = option.strip_prefix("--passes=") {
            passes = value.parse().map_err(|_| CommandError::Passes)?;
        } else if let Some(value) = option.strip_prefix("--color=") {
            color = parse_color(value).ok_or(CommandError::Color)?;
        } else {
            return Err(CommandError::UnknownOption(option.to_owned()));
        }
        rest = tail.trim_start();
    }
    Message::new(rest, passes, color).map_err(|e| match e {
        MessageError::NoText => CommandError::NoText,
        MessageError::TooLong => CommandError::TextTooLong,
        MessageError::Passes => CommandError::Passes,
    })
}

/// The color of six hexadecimal digits `RRGGBB`, or `None`.
fn parse_color(value: &str) -> Option<Rgb888> {
    if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let rgb = u32::from_str_radix(value, 16).ok()?;
    let [_, r, g, b] = rgb.to_be_bytes();
    Some(Rgb888::new(r, g, b))
}

/// The reply to `status`. `messages` is the number of messages on the panel
/// and in the queue.
#[must_use]
pub fn status_reply(
    mode: Mode,
    zone: &TimeZone,
    brightness: Brightness,
    messages: usize,
) -> String {
    let zone = zone.iana_name().unwrap_or("unnamed");
    format!(
        "ok mode={} zone={zone} brightness={} messages={messages}",
        mode.name(),
        brightness.percent()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Mode;
    use gameclock_core::Brightness;
    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::message::Message;
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
            "error: unknown command 'frobnicate'. The commands are clock, test-pattern, status, message, and clear."
        );
    }

    #[test]
    fn commands_are_exact() {
        for text in [
            "Clock",
            "STATUS",
            "clock now",
            "clear now",
            "Message hi",
            "",
        ] {
            assert!(parse_command(text.as_bytes()).is_err(), "{text:?}");
        }
    }

    #[test]
    fn clear_parses() {
        assert_eq!(parse_command(b"clear"), Ok(Command::Clear));
    }

    // The message command.

    fn message(text: &str, passes: u8, color: Rgb888) -> Command {
        Command::Message(Message::new(text, passes, color).unwrap())
    }

    fn reply(line: &str) -> String {
        parse_command(line.as_bytes()).unwrap_err().reply()
    }

    #[test]
    fn message_with_the_defaults() {
        assert_eq!(
            parse_command(b"message hello"),
            Ok(message("hello", 4, Rgb888::new(0, 150, 0)))
        );
    }

    #[test]
    fn message_with_options() {
        assert_eq!(
            parse_command(b"message --passes=2 --color=ff8c00 good night"),
            Ok(message("good night", 2, Rgb888::new(255, 140, 0)))
        );
        assert_eq!(
            parse_command(b"message --color=00FF00 hi"),
            Ok(message("hi", 4, Rgb888::new(0, 255, 0)))
        );
    }

    #[test]
    fn later_option_wins() {
        assert_eq!(
            parse_command(b"message --passes=2 --passes=3 hi"),
            Ok(message("hi", 3, Message::DEFAULT_COLOR))
        );
    }

    #[test]
    fn spaces_in_the_message_stay() {
        assert_eq!(
            parse_command(b"message a  b"),
            Ok(message("a  b", 4, Message::DEFAULT_COLOR))
        );
    }

    #[test]
    fn options_only_before_the_text() {
        assert_eq!(
            parse_command(b"message hi --passes=2"),
            Ok(message("hi --passes=2", 4, Message::DEFAULT_COLOR))
        );
    }

    #[test]
    fn message_with_no_text() {
        for line in ["message", "message --passes=2", "message  "] {
            assert_eq!(reply(line), "error: the message has no text.", "{line:?}");
        }
    }

    #[test]
    fn message_text_too_long() {
        let line = format!("message {}", "a".repeat(65));
        assert_eq!(
            reply(&line),
            "error: the message is longer than 64 characters."
        );
    }

    #[test]
    fn pass_count_out_of_range() {
        for line in [
            "message --passes=0 hi",
            "message --passes=11 hi",
            "message --passes=x hi",
            "message --passes= hi",
        ] {
            assert_eq!(
                reply(line),
                "error: --passes must be a number from 1 to 10.",
                "{line:?}"
            );
        }
    }

    #[test]
    fn color_that_is_not_correct() {
        for line in [
            "message --color=green hi",
            "message --color=12345 hi",
            "message --color=+12345 hi",
            "message --color=1234567 hi",
        ] {
            assert_eq!(
                reply(line),
                "error: --color must be six hexadecimal digits, for example 00ff00.",
                "{line:?}"
            );
        }
    }

    #[test]
    fn unknown_option() {
        assert_eq!(
            reply("message --speed=2 hi"),
            "error: unknown message option '--speed=2'. The options are --passes and --color."
        );
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
        let reply = status_reply(Mode::Clock, &TimeZone::UTC, Brightness::new(25), 0);
        assert_eq!(reply, "ok mode=clock zone=UTC brightness=25 messages=0");
    }

    #[test]
    fn status_of_the_test_pattern() {
        let reply = status_reply(Mode::TestPattern, &TimeZone::UTC, Brightness::MAX, 0);
        assert_eq!(
            reply,
            "ok mode=test-pattern zone=UTC brightness=100 messages=0"
        );
    }

    #[test]
    fn status_with_an_unnamed_zone() {
        let zone = TimeZone::fixed(jiff::tz::offset(2));
        let reply = status_reply(Mode::Clock, &zone, Brightness::new(60), 0);
        assert_eq!(reply, "ok mode=clock zone=unnamed brightness=60 messages=0");
    }

    #[test]
    fn status_with_three_messages() {
        let reply = status_reply(Mode::Clock, &TimeZone::UTC, Brightness::new(60), 3);
        assert!(reply.ends_with(" messages=3"), "{reply}");
    }

    #[test]
    fn limit_replies() {
        assert_eq!(NO_COMMAND, "error: no command in 1 s.");
        assert_eq!(NO_ANSWER, "error: the daemon did not answer in 2 s.");
    }
}
