//! End-to-end tests of the host build of `gameclockd` (`specs/daemon-lifecycle`).
//! They spawn the real binary, so they cover argument handling, signal
//! registration, and exit codes that unit tests can't reach.
//!
//! Host build only: with the `hzeller` feature the binary needs the real panel.
#![cfg(not(feature = "hzeller"))]

use std::io::{BufRead, BufReader};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// Printed by the logging panel once the daemon has presented, by which point
/// its signal handlers are registered.
const READY: &str = "presented a 64×32 frame";

/// A running daemon with its stderr collected line by line.
struct Daemon {
    child: Child,
    lines: Receiver<String>,
    seen: Vec<String>,
}

impl Daemon {
    fn spawn(args: &[&str]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gameclockd"))
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn gameclockd");
        let stderr = child.stderr.take().expect("piped stderr");
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            lines,
            seen: Vec::new(),
        }
    }

    /// Wait until a stderr line contains `needle`, or panic after 5 s.
    fn wait_for(&mut self, needle: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !self.seen.iter().any(|l| l.contains(needle)) {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.lines.recv_timeout(left) {
                Ok(line) => self.seen.push(line),
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    panic!("no stderr line containing {needle:?}; saw {:?}", self.seen)
                }
            }
        }
    }

    fn stderr_contains(&self, needle: &str) -> bool {
        self.seen.iter().any(|l| l.contains(needle))
    }

    fn signal(&self, name: &str) {
        let status = Command::new("kill")
            .arg(format!("-{name}"))
            .arg(self.child.id().to_string())
            .status()
            .expect("run kill");
        assert!(status.success(), "kill -{name} failed");
    }

    /// Wait for exit, or kill the daemon and panic after `limit`.
    fn exit_within(&mut self, limit: Duration) -> ExitStatus {
        let deadline = Instant::now() + limit;
        loop {
            if let Some(status) = self.child.try_wait().expect("try_wait") {
                return status;
            }
            if Instant::now() >= deadline {
                let _ = self.child.kill();
                panic!("gameclockd did not exit within {limit:?}");
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Collect whatever stderr is left after exit.
    fn drain(&mut self) {
        while let Ok(line) = self.lines.recv_timeout(Duration::from_millis(200)) {
            self.seen.push(line);
        }
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn stops_cleanly_on(signal: &str) {
    let mut d = Daemon::spawn(&[]);
    d.wait_for(READY);
    d.signal(signal);
    let status = d.exit_within(Duration::from_secs(1));
    assert_eq!(status.code(), Some(0), "exit status after SIG{signal}");
}

#[test]
fn sigterm_exits_with_success() {
    stops_cleanly_on("TERM");
}

#[test]
fn sigint_exits_with_success() {
    stops_cleanly_on("INT");
}

#[test]
fn unknown_argument_is_a_usage_error() {
    let mut d = Daemon::spawn(&["--frobnicate"]);
    let status = d.exit_within(Duration::from_secs(5));
    d.drain();
    assert_eq!(status.code(), Some(2));
    assert!(d.stderr_contains("--frobnicate"), "stderr: {:?}", d.seen);
}

#[test]
fn host_build_announces_the_logging_panel() {
    let mut d = Daemon::spawn(&[]);
    d.wait_for(READY);
    assert!(d.stderr_contains("no LED driver"), "stderr: {:?}", d.seen);
}

#[test]
fn driver_flags_are_ignored_without_the_driver() {
    let mut d = Daemon::spawn(&["--led-pwm-bits=7"]);
    d.wait_for(READY);
    assert!(
        d.stderr_contains("--led-* flags are ignored"),
        "stderr: {:?}",
        d.seen
    );
}
