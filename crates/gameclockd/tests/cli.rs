//! End-to-end tests of the host build of `gameclockd`
//! (`openspec/specs/daemon-lifecycle`). They start the real binary. So they
//! test argument handling, signal registration, and exit codes, which unit
//! tests cannot reach.
//!
//! Host builds only. With the `hzeller` feature, the binary needs the real
//! panel. The `log_panel` tests run without features. The `simulator` tests
//! run with `--features simulator`.
#![cfg(not(feature = "hzeller"))]

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

/// A running daemon with its stderr collected line by line, and its stdout
/// collected at exit.
struct Daemon {
    child: Child,
    lines: Receiver<String>,
    seen: Vec<String>,
    stdout: Receiver<String>,
}

impl Daemon {
    fn spawn(args: &[&str]) -> Self {
        Self::spawn_with_env(args, &[])
    }

    fn spawn_with_env(args: &[&str], env: &[(&str, &str)]) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gameclockd"))
            .args(args)
            .envs(env.iter().copied())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn gameclockd");
        let mut out = child.stdout.take().expect("piped stdout");
        let (out_tx, stdout) = mpsc::channel();
        std::thread::spawn(move || {
            let mut text = String::new();
            let _ = out.read_to_string(&mut text);
            let _ = out_tx.send(text);
        });
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
            stdout,
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

    #[cfg_attr(feature = "simulator", allow(dead_code))]
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

    /// Move the stderr lines that are there now into `seen`.
    #[cfg_attr(feature = "simulator", allow(dead_code))]
    fn collect(&mut self) {
        while let Ok(line) = self.lines.try_recv() {
            self.seen.push(line);
        }
    }

    /// The number of presents that the logging panel wrote so far.
    #[cfg_attr(feature = "simulator", allow(dead_code))]
    fn presents_seen(&self) -> usize {
        self.seen
            .iter()
            .filter(|l| l.contains("presented a"))
            .count()
    }

    /// All of stdout. Call it after the daemon exits.
    #[cfg_attr(not(feature = "simulator"), allow(dead_code))]
    fn stdout(&self) -> String {
        self.stdout
            .recv_timeout(Duration::from_secs(5))
            .expect("stdout after exit")
    }

    /// Collect whatever stderr is left after exit.
    #[cfg_attr(feature = "simulator", allow(dead_code))]
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

/// A new, empty directory for one test, and a socket path in it.
#[cfg_attr(feature = "simulator", allow(dead_code))]
fn socket_in_temp_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("gcd-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("make the temporary directory");
    dir.join("control.sock")
}

#[cfg_attr(feature = "simulator", allow(dead_code))]
fn socket_arg(path: &std::path::Path) -> String {
    format!("--control-socket={}", path.display())
}

/// Run `gameclockd ctl` with `args`. Give the exit code, stdout, and stderr.
#[cfg_attr(feature = "simulator", allow(dead_code))]
fn ctl(args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_gameclockd"))
        .arg("ctl")
        .args(args)
        .output()
        .expect("run gameclockd ctl");
    (
        out.status.code(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Tests of the default host build, which uses the logging panel.
#[cfg(not(feature = "simulator"))]
mod log_panel {
    use super::*;

    /// The logging panel prints this after the daemon presents. At that time,
    /// the signal handlers are registered.
    const READY: &str = "presented a 64×32 frame";

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
    fn time_zone_from_the_environment() {
        let mut d = Daemon::spawn_with_env(&[], &[("TZ", "America/Los_Angeles")]);
        d.wait_for(READY);
        let line = "gameclockd: the time zone is America/Los_Angeles.";
        assert!(d.seen.iter().any(|l| l == line), "stderr: {:?}", d.seen);
    }

    #[test]
    fn host_build_announces_the_logging_panel() {
        let mut d = Daemon::spawn(&[]);
        d.wait_for(READY);
        let notice = "gameclockd: no LED driver is built in. Frames are only logged.";
        assert!(d.seen.iter().any(|l| l == notice), "stderr: {:?}", d.seen);
    }

    // The control socket (openspec/specs/command-input).

    #[test]
    fn socket_at_the_given_path() {
        use std::os::unix::fs::{FileTypeExt, PermissionsExt};
        let path = socket_in_temp_dir("given");
        let mut d = Daemon::spawn(&[&socket_arg(&path)]);
        d.wait_for(READY);
        let meta = std::fs::metadata(&path).expect("a socket file");
        assert!(meta.file_type().is_socket());
        assert_eq!(meta.permissions().mode() & 0o777, 0o600);
    }

    #[test]
    fn the_socket_file_goes_away_at_shutdown() {
        let path = socket_in_temp_dir("shutdown");
        let mut d = Daemon::spawn(&[&socket_arg(&path)]);
        d.wait_for(READY);
        d.signal("TERM");
        let status = d.exit_within(Duration::from_secs(1));
        assert_eq!(status.code(), Some(0));
        assert!(!path.exists(), "{} is still there", path.display());
    }

    #[test]
    fn socket_directory_does_not_exist() {
        let path = socket_in_temp_dir("missing").with_file_name("no-such-dir/control.sock");
        let mut d = Daemon::spawn(&[&socket_arg(&path)]);
        d.wait_for(READY);
        let warning = format!("gameclockd: cannot listen on {}:", path.display());
        assert!(
            d.seen.iter().any(|l| l.starts_with(&warning)
                && l.ends_with("The daemon runs with no command input.")),
            "stderr: {:?}",
            d.seen
        );
        d.signal("TERM");
        assert_eq!(d.exit_within(Duration::from_secs(1)).code(), Some(0));
    }

    #[test]
    fn the_heartbeat_continues_during_a_silent_connection() {
        let path = socket_in_temp_dir("silent");
        let mut d = Daemon::spawn(&["--test-pattern", &socket_arg(&path)]);
        d.wait_for(READY);
        let _silent = std::os::unix::net::UnixStream::connect(&path).expect("connect");
        let before = d.presents_seen();
        std::thread::sleep(Duration::from_millis(1_100));
        d.collect();
        assert!(
            d.presents_seen() > before,
            "no present during the connection"
        );
    }

    #[test]
    fn status_through_the_client() {
        let path = socket_in_temp_dir("status");
        let mut d = Daemon::spawn(&[&socket_arg(&path)]);
        d.wait_for(READY);
        let (code, out, _) = ctl(&[&socket_arg(&path), "status"]);
        assert_eq!(code, Some(0), "stdout: {out}");
        assert!(out.starts_with("ok mode=clock"), "{out}");
    }

    #[test]
    fn message_through_the_client() {
        let path = socket_in_temp_dir("message");
        let mut d = Daemon::spawn(&[&socket_arg(&path)]);
        d.wait_for(READY);
        let (code, out, _) = ctl(&[&socket_arg(&path), "message", "hello", "world"]);
        assert_eq!(code, Some(0), "stdout: {out}");
        assert_eq!(out, "ok\n");
    }

    #[test]
    fn error_reply_through_the_client() {
        let path = socket_in_temp_dir("error");
        let mut d = Daemon::spawn(&[&socket_arg(&path)]);
        d.wait_for(READY);
        let (code, out, _) = ctl(&[&socket_arg(&path), "frobnicate"]);
        assert_eq!(code, Some(1));
        assert!(
            out.starts_with("error: unknown command 'frobnicate'."),
            "{out}"
        );
    }

    #[test]
    fn no_daemon() {
        let path = socket_in_temp_dir("none");
        let (code, _, err) = ctl(&[&socket_arg(&path), "status"]);
        assert_eq!(code, Some(1));
        assert!(err.contains(&path.display().to_string()), "{err}");
    }

    #[test]
    fn missing_command() {
        let (code, _, err) = ctl(&[]);
        assert_eq!(code, Some(2));
        assert!(err.contains("usage: gameclockd ctl"), "{err}");
    }

    #[test]
    fn host_build_names_the_clock() {
        let mut d = Daemon::spawn(&[]);
        d.wait_for(READY);
        let line = "gameclockd: the base scene is the clock.";
        assert!(d.seen.iter().any(|l| l == line), "stderr: {:?}", d.seen);
    }

    #[test]
    fn host_build_names_the_test_pattern() {
        let mut d = Daemon::spawn(&["--test-pattern"]);
        d.wait_for(READY);
        let line = "gameclockd: the base scene is the test pattern.";
        assert!(d.seen.iter().any(|l| l == line), "stderr: {:?}", d.seen);
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
}

/// Tests of the simulator build (`openspec/specs/terminal-simulator`).
#[cfg(feature = "simulator")]
mod simulator {
    use super::*;

    #[test]
    fn simulator_build_draws_in_the_terminal() {
        let mut d = Daemon::spawn(&[]);
        // Signal handlers are registered before this notice.
        d.wait_for("Frames are drawn in the terminal.");
        d.signal("TERM");
        let status = d.exit_within(Duration::from_secs(1));
        assert_eq!(status.code(), Some(0), "exit status after SIGTERM");
        let out = d.stdout();
        // The clock can present more than one frame, for example at a new
        // minute. Each frame starts with the cursor-home sequence. Check the
        // last frame.
        let last = out.rsplit("\x1b[H").next().unwrap_or_default();
        let rows: Vec<&str> = last.lines().filter(|l| l.contains('▀')).collect();
        assert_eq!(rows.len(), 16, "stdout: {out:?}");
        assert!(rows.iter().all(|r| r.matches('▀').count() == 64));
    }
}
