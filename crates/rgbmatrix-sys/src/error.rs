//! Errors from the hzeller panel.

use core::fmt;

/// Why a panel could not be created or a frame could not be shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The frame's `(width, height)` differs from the panel's.
    DimensionMismatch {
        /// The frame's size.
        frame: (usize, usize),
        /// The panel's size.
        panel: (usize, usize),
    },
    /// hzeller could not create the matrix (bad flags, or no GPIO access).
    Init,
    /// Arguments hzeller did not recognize as `--led-*` flags.
    UnknownArgs(Vec<String>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch { frame, panel } => write!(
                f,
                "frame is {}×{} but the panel is {}×{}",
                frame.0, frame.1, panel.0, panel.1
            ),
            Self::Init => f.write_str(
                "LED panel initialization failed (see the driver's messages above; \
                 GPIO access needs root)",
            ),
            Self::UnknownArgs(args) => {
                write!(f, "unrecognized driver arguments: {}", args.join(" "))
            }
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;

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
