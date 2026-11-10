//! The control socket: a Unix stream socket with one command line for each
//! connection (`openspec/specs/command-input`, docs/adr/0016).
//!
//! The socket thread only moves bytes. `crate::command` parses, and the loop
//! makes the replies.

use core::fmt;
use std::io::{self, Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::command::{CommandError, MAX_LINE, NO_ANSWER, NO_COMMAND, parse_command};
use crate::{Event, Request, THREAD_STACK};

/// The time a client has to send a complete line.
pub const READ_LIMIT: Duration = Duration::from_secs(1);

/// The time the loop has to answer a command.
pub const ANSWER_LIMIT: Duration = Duration::from_secs(2);

/// Why the daemon cannot listen on the socket path.
#[derive(Debug)]
pub enum ListenError {
    /// A process listens on the path. The daemon does not take it.
    InUse,
    /// The bind or a file operation failed.
    Io(io::Error),
}

impl fmt::Display for ListenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InUse => f.write_str("another process listens on it"),
            Self::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ListenError {}

impl From<io::Error> for ListenError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

/// Listen on `path`. Replace a stale socket file that no process listens
/// on. Give the file access for its owner only.
///
/// # Errors
/// [`ListenError::InUse`] if a process listens on `path`.
/// [`ListenError::Io`] if the bind or a file operation fails.
pub fn listen(path: &Path) -> Result<UnixListener, ListenError> {
    if path.exists() {
        if UnixStream::connect(path).is_ok() {
            return Err(ListenError::InUse);
        }
        std::fs::remove_file(path)?;
    }
    let listener = UnixListener::bind(path)?;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    Ok(listener)
}

/// Start the socket thread. It accepts one connection at a time and sends
/// each command to the loop on `events`.
///
/// # Errors
/// The error of the thread start.
pub fn spawn(listener: UnixListener, events: mpsc::Sender<Event>) -> io::Result<()> {
    std::thread::Builder::new()
        .name("control".into())
        .stack_size(THREAD_STACK)
        .spawn(move || serve(&listener, &events))?;
    Ok(())
}

fn serve(listener: &UnixListener, events: &mpsc::Sender<Event>) {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle(stream, events),
            // A failed accept affects one client only. Pause, so that an
            // error that stays (for example, no free file descriptors) does
            // not use the CPU.
            Err(_) => std::thread::sleep(Duration::from_millis(100)),
        }
    }
}

/// Read one line, get the reply, write it, and close the connection.
fn handle(mut stream: UnixStream, events: &mpsc::Sender<Event>) {
    let reply = match read_line(&mut stream) {
        Line::Complete(bytes) => match parse_command(&bytes) {
            Ok(command) => ask(events, command),
            Err(e) => e.reply(),
        },
        Line::TooLong => CommandError::TooLong.reply(),
        Line::Late => NO_COMMAND.to_owned(),
    };
    // The client can be gone. There is nobody to tell.
    let _ = stream.write_all(format!("{reply}\n").as_bytes());
}

/// Send `command` to the loop and wait for its reply.
fn ask(events: &mpsc::Sender<Event>, command: crate::command::Command) -> String {
    let (reply, answer) = mpsc::channel();
    if events
        .send(Event::Command(Request { command, reply }))
        .is_err()
    {
        return NO_ANSWER.to_owned();
    }
    answer
        .recv_timeout(ANSWER_LIMIT)
        .unwrap_or_else(|_| NO_ANSWER.to_owned())
}

/// The time the client waits for the reply.
pub const CLIENT_LIMIT: Duration = Duration::from_secs(3);

/// Send `command` to the socket at `path` and write the reply line to `out`.
/// Write a connection error to `err`. Give the exit status: 0 for an `ok`
/// reply, 1 for an error reply or a failure.
pub fn send_command(path: &Path, command: &str, out: &mut impl Write, err: &mut impl Write) -> u8 {
    match exchange(path, command) {
        Ok(reply) => {
            let _ = writeln!(out, "{reply}");
            u8::from(!(reply == "ok" || reply.starts_with("ok ")))
        }
        Err(e) => {
            let _ = writeln!(
                err,
                "gameclockd: cannot get a reply from {}: {e}",
                path.display()
            );
            1
        }
    }
}

fn exchange(path: &Path, command: &str) -> io::Result<String> {
    let mut stream = UnixStream::connect(path)?;
    stream.write_all(command.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.shutdown(std::net::Shutdown::Write)?;
    stream.set_read_timeout(Some(CLIENT_LIMIT))?;
    let mut reply = String::new();
    stream.read_to_string(&mut reply)?;
    Ok(reply.trim_end_matches('\n').to_owned())
}

enum Line {
    /// The bytes before the line feed or the end of the stream.
    Complete(Vec<u8>),
    /// More than [`MAX_LINE`] bytes before the line feed.
    TooLong,
    /// No complete line in [`READ_LIMIT`].
    Late,
}

fn read_line(stream: &mut UnixStream) -> Line {
    let end = Instant::now() + READ_LIMIT;
    let mut line = Vec::with_capacity(MAX_LINE + 1);
    let mut chunk = [0_u8; 64];
    loop {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() || stream.set_read_timeout(Some(left)).is_err() {
            return Line::Late;
        }
        match stream.read(&mut chunk) {
            Ok(0) => return Line::Complete(line),
            Ok(n) => {
                let got = chunk.get(..n).unwrap_or_default();
                if let Some(at) = got.iter().position(|&b| b == b'\n') {
                    line.extend_from_slice(got.get(..at).unwrap_or_default());
                    return if line.len() > MAX_LINE {
                        Line::TooLong
                    } else {
                        Line::Complete(line)
                    };
                }
                line.extend_from_slice(got);
                if line.len() > MAX_LINE {
                    return Line::TooLong;
                }
            }
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return Line::Late,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{Command, NO_ANSWER, NO_COMMAND};
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::net::UnixStream;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::{Duration, Instant};

    /// A new, empty directory for one test.
    fn temp_dir() -> PathBuf {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("gcd-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A loop that answers each command with its debug text.
    fn answering_loop() -> mpsc::Sender<crate::Event> {
        let (tx, rx) = mpsc::channel::<crate::Event>();
        std::thread::spawn(move || {
            for event in rx {
                if let crate::Event::Command(r) = event {
                    let _ = r.reply.send(format!("ok {:?}", r.command));
                }
            }
        });
        tx
    }

    fn send(path: &Path, bytes: &[u8]) -> String {
        let mut stream = UnixStream::connect(path).unwrap();
        stream.write_all(bytes).unwrap();
        read_reply(stream)
    }

    fn read_reply(stream: UnixStream) -> String {
        let mut line = String::new();
        BufReader::new(stream).read_line(&mut line).unwrap();
        line.trim_end_matches('\n').to_owned()
    }

    fn serving(dir: &Path, events: mpsc::Sender<crate::Event>) -> PathBuf {
        let path = dir.join("control.sock");
        let listener = listen(&path).unwrap();
        spawn(listener, events).unwrap();
        path
    }

    #[test]
    fn only_the_owner_can_connect() {
        let path = temp_dir().join("control.sock");
        let _listener = listen(&path).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "{mode:o}");
    }

    #[test]
    fn a_stale_socket_file_is_replaced() {
        let path = temp_dir().join("control.sock");
        drop(std::os::unix::net::UnixListener::bind(&path).unwrap());
        assert!(path.exists(), "the stale file stays after the drop");
        let listener = listen(&path).unwrap();
        spawn(listener, answering_loop()).unwrap();
        assert_eq!(send(&path, b"status\n"), "ok Status");
    }

    #[test]
    fn a_live_socket_is_not_taken() {
        let path = temp_dir().join("control.sock");
        let _first = listen(&path).unwrap();
        let err = listen(&path).unwrap_err();
        assert!(matches!(err, ListenError::InUse), "{err}");
        assert_eq!(err.to_string(), "another process listens on it");
    }

    #[test]
    fn missing_directory_is_an_error() {
        let path = temp_dir().join("no-such-dir").join("control.sock");
        assert!(matches!(listen(&path), Err(ListenError::Io(_))));
    }

    #[test]
    fn a_command_gets_the_reply_of_the_loop() {
        let path = serving(&temp_dir(), answering_loop());
        assert_eq!(
            send(&path, b"clock\n"),
            format!("ok {:?}", Command::Mode(crate::Mode::Clock))
        );
    }

    #[test]
    fn end_of_stream_ends_the_line() {
        let path = serving(&temp_dir(), answering_loop());
        let mut stream = UnixStream::connect(&path).unwrap();
        stream.write_all(b"status").unwrap();
        stream.shutdown(std::net::Shutdown::Write).unwrap();
        assert_eq!(read_reply(stream), "ok Status");
    }

    #[test]
    fn parse_errors_get_their_reply() {
        let path = serving(&temp_dir(), answering_loop());
        assert_eq!(
            send(&path, b"frobnicate\n"),
            "error: unknown command 'frobnicate'. The commands are clock, test-pattern, and status."
        );
        assert_eq!(
            send(&path, &[b'a'; 300]),
            "error: the command is longer than 256 bytes."
        );
        assert_eq!(
            send(&path, b"\xFF\xFE\n"),
            "error: the command is not UTF-8 text."
        );
    }

    #[test]
    fn silent_client_is_closed() {
        let path = serving(&temp_dir(), answering_loop());
        let stream = UnixStream::connect(&path).unwrap();
        let start = Instant::now();
        let reply = read_reply(stream);
        let waited = start.elapsed();
        assert_eq!(reply, NO_COMMAND);
        assert!(
            waited >= Duration::from_millis(1_000) && waited < Duration::from_millis(2_000),
            "{waited:?}"
        );
    }

    #[test]
    fn loop_does_not_answer() {
        let (tx, rx) = mpsc::channel::<crate::Event>();
        // Keep each request, so that its reply sender stays open, and never
        // answer.
        let holder = std::thread::spawn(move || rx.into_iter().collect::<Vec<_>>());
        let path = serving(&temp_dir(), tx);
        let start = Instant::now();
        let reply = send(&path, b"status\n");
        let waited = start.elapsed();
        assert_eq!(reply, NO_ANSWER);
        assert!(
            waited >= Duration::from_millis(2_000) && waited < Duration::from_millis(3_000),
            "{waited:?}"
        );
        drop(holder);
    }

    #[test]
    fn a_closed_loop_gets_no_answer_reply() {
        let (tx, rx) = mpsc::channel::<crate::Event>();
        drop(rx);
        let path = serving(&temp_dir(), tx);
        assert_eq!(send(&path, b"status\n"), NO_ANSWER);
    }
}
