//! The behavior of `gameclockd`. It is not in `main`, so tests do not need
//! hardware or signals (`openspec/specs/daemon-lifecycle`).

use core::fmt;

use gameclock_core::{Brightness, Frame, Panel, test_pattern};

/// Why the daemon stopped with an error.
#[derive(Debug)]
pub enum RunError<E> {
    /// The panel failed to show a frame.
    Present(E),
}

impl<E: fmt::Display> fmt::Display for RunError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Present(e) => write!(f, "the panel did not show the frame: {e}"),
        }
    }
}

impl<E: std::error::Error + 'static> std::error::Error for RunError<E> {}

/// Show the test pattern on `panel` once, then block in
/// `wait_for_shutdown` until it returns.
///
/// # Errors
/// [`RunError::Present`] if the panel fails to show the frame.
pub fn run<P: Panel>(
    panel: &mut P,
    wait_for_shutdown: impl FnOnce(),
) -> Result<(), RunError<P::Error>> {
    let (width, height) = panel.dimensions();
    let mut frame = Frame::new(width, height);
    match test_pattern::render(&mut frame) {
        Ok(()) => {}
        Err(never) => match never {},
    }
    panel
        .present(&frame, Brightness::MAX)
        .map_err(RunError::Present)?;
    // Nothing changes until a stop signal comes. Block. Do not poll
    // (docs/adr/0006).
    wait_for_shutdown();
    Ok(())
}

/// An argument that isn't an hzeller `--led-*` driver flag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsageError(pub String);

impl fmt::Display for UsageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown argument '{}'. gameclockd accepts only --led-* driver flags.",
            self.0
        )
    }
}

impl std::error::Error for UsageError {}

/// Check the command line. Only `--led-*` driver flags are accepted. The
/// function returns them unchanged for the driver to parse
/// (`openspec/specs/daemon-lifecycle`, docs/adr/0009).
///
/// # Errors
/// [`UsageError`] naming the first argument that isn't a `--led-*` flag.
pub fn parse_args<I>(args: I) -> Result<Vec<String>, UsageError>
where
    I: IntoIterator<Item = String>,
{
    args.into_iter()
        .map(|arg| {
            if arg.starts_with("--led-") {
                Ok(arg)
            } else {
                Err(UsageError(arg))
            }
        })
        .collect()
}

/// A panel for builds without the LED driver. It only logs its presents, so
/// the daemon runs on any machine.
#[derive(Debug)]
pub struct LogPanel {
    size: (usize, usize),
}

impl LogPanel {
    /// A logging panel of `width` × `height` pixels.
    #[must_use]
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            size: (width, height),
        }
    }
}

impl Panel for LogPanel {
    type Error = core::convert::Infallible;

    fn dimensions(&self) -> (usize, usize) {
        self.size
    }

    fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Self::Error> {
        eprintln!(
            "gameclockd: presented a {}×{} frame at brightness {}%",
            frame.width(),
            frame.height(),
            brightness.percent()
        );
        Ok(())
    }
}

/// The process exit status for a finished run: 0 on success, 1 on error.
#[must_use]
pub fn exit_status<E>(result: &Result<(), RunError<E>>) -> u8 {
    u8::from(result.is_err())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::convert::Infallible;
    use std::time::{Duration, Instant};

    use gameclock_core::{Brightness, Frame, test_pattern};

    /// Records every present.
    #[derive(Debug)]
    struct Recorder {
        size: (usize, usize),
        presents: Vec<(Frame, Brightness)>,
    }

    impl Recorder {
        fn new(width: usize, height: usize) -> Self {
            Self {
                size: (width, height),
                presents: Vec::new(),
            }
        }
    }

    impl Panel for Recorder {
        type Error = Infallible;
        fn dimensions(&self) -> (usize, usize) {
            self.size
        }
        fn present(&mut self, frame: &Frame, brightness: Brightness) -> Result<(), Infallible> {
            self.presents.push((frame.clone(), brightness));
            Ok(())
        }
    }

    #[derive(Debug)]
    struct Broken;

    #[derive(Debug)]
    struct BrokenError;

    impl fmt::Display for BrokenError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("the LEDs are on fire")
        }
    }

    impl std::error::Error for BrokenError {}

    impl Panel for Broken {
        type Error = BrokenError;
        fn dimensions(&self) -> (usize, usize) {
            (64, 32)
        }
        fn present(&mut self, _: &Frame, _: Brightness) -> Result<(), BrokenError> {
            Err(BrokenError)
        }
    }

    fn expected_pattern(width: usize, height: usize) -> Frame {
        let mut f = Frame::new(width, height);
        test_pattern::render(&mut f).unwrap();
        f
    }

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn driver_flags_are_accepted_unchanged() {
        let given = args(&["--led-pwm-bits=7", "--led-show-refresh"]);
        assert_eq!(parse_args(given.clone()), Ok(given));
    }

    #[test]
    fn no_arguments_is_fine() {
        assert_eq!(parse_args(Vec::new()), Ok(Vec::new()));
    }

    #[test]
    fn unknown_argument_is_a_usage_error_naming_it() {
        let err = parse_args(args(&["--led-rows=32", "--frobnicate"])).unwrap_err();
        assert_eq!(err, UsageError("--frobnicate".into()));
        assert!(err.to_string().contains("--frobnicate"), "{err}");
    }

    #[test]
    fn bare_words_are_usage_errors() {
        assert!(parse_args(args(&["led-rows=32"])).is_err());
    }

    #[test]
    fn one_present_at_startup() {
        let mut panel = Recorder::new(64, 32);
        run(&mut panel, || {}).unwrap();
        assert_eq!(panel.presents.len(), 1);
        let (frame, brightness) = &panel.presents[0];
        assert_eq!(*frame, expected_pattern(64, 32));
        assert_eq!(brightness.percent(), 100);
    }

    #[test]
    fn frame_follows_the_panels_size() {
        let mut panel = Recorder::new(32, 16);
        run(&mut panel, || {}).unwrap();
        assert_eq!(panel.presents.len(), 1);
        assert_eq!(panel.presents[0].0, expected_pattern(32, 16));
    }

    #[test]
    fn no_presents_while_waiting() {
        let mut panel = Recorder::new(64, 32);
        let start = Instant::now();
        run(&mut panel, || {
            std::thread::sleep(Duration::from_millis(500));
        })
        .unwrap();
        assert!(
            start.elapsed() >= Duration::from_millis(500),
            "run must wait"
        );
        assert_eq!(panel.presents.len(), 1);
    }

    #[test]
    fn messages_are_exact() {
        let err = run(&mut Broken, || {}).unwrap_err();
        assert_eq!(
            err.to_string(),
            "the panel did not show the frame: the LEDs are on fire"
        );
        assert_eq!(
            UsageError("--frobnicate".into()).to_string(),
            "unknown argument '--frobnicate'. gameclockd accepts only --led-* driver flags."
        );
    }

    #[test]
    fn present_failure_is_reported_and_exits_non_zero() {
        let result = run(&mut Broken, || {});
        let err = result.as_ref().unwrap_err();
        assert!(
            err.to_string().contains("the LEDs are on fire"),
            "error was: {err}"
        );
        assert_ne!(exit_status(&result), 0);
    }

    #[test]
    fn success_exits_zero() {
        let result = run(&mut Recorder::new(64, 32), || {});
        assert_eq!(exit_status(&result), 0);
    }
}
