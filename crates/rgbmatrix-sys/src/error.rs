//! Errors from the hzeller panel.

use core::fmt;

/// The reason that a panel cannot start or cannot show a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The frame's `(width, height)` is different from the panel's.
    DimensionMismatch {
        /// The frame's size.
        frame: (usize, usize),
        /// The panel's size.
        panel: (usize, usize),
    },
    /// hzeller cannot create the matrix (bad flags, or no GPIO access).
    Init,
    /// Arguments that hzeller does not know as `--led-*` flags.
    UnknownArgs(Vec<String>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { frame, panel } => write!(
                f,
                "the frame is {}×{}, but the panel is {}×{}",
                frame.0, frame.1, panel.0, panel.1
            ),
            Self::Init => {
                f.write_str("LED panel initialization failed. See the driver messages above.")
            }
            Self::UnknownArgs(args) => {
                write!(f, "unknown driver arguments: {}", args.join(" "))
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_exact() {
        let mismatch = Error::DimensionMismatch {
            frame: (32, 16),
            panel: (64, 32),
        };
        assert_eq!(
            mismatch.to_string(),
            "the frame is 32×16, but the panel is 64×32"
        );
        assert_eq!(
            Error::Init.to_string(),
            "LED panel initialization failed. See the driver messages above."
        );
        assert_eq!(
            Error::UnknownArgs(vec!["--led-a".into(), "--led-b".into()]).to_string(),
            "unknown driver arguments: --led-a --led-b"
        );
    }

    #[test]
    fn dimension_mismatch_names_both_sizes() {
        let e = Error::DimensionMismatch {
            frame: (32, 16),
            panel: (64, 32),
        };
        let msg = e.to_string();
        assert!(msg.contains("32×16"), "{msg}");
        assert!(msg.contains("64×32"), "{msg}");
    }

    #[test]
    fn init_failure_mentions_panel_initialization() {
        let msg = Error::Init.to_string();
        assert!(msg.contains("panel initialization"), "{msg}");
    }

    #[test]
    fn unknown_args_are_listed() {
        let msg = Error::UnknownArgs(vec!["--led-bogus".into()]).to_string();
        assert!(msg.contains("--led-bogus"), "{msg}");
    }
}
