//! The page fetches of the loop (`openspec/specs/sensor-pages`,
//! docs/adr/0021).

use std::io::{self, Read};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use crate::{Event, Fetch, Fetched, Job, THREAD_STACK};

/// The limits of one fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most time for the whole fetch.
    pub max_time: Duration,
    /// The most bytes of the body.
    pub max_bytes: usize,
}

impl Limits {
    /// The limits of a page fetch (`openspec/specs/sensor-pages`).
    pub const PAGE: Self = Self {
        max_time: Duration::from_secs(10),
        max_bytes: 256 * 1024,
    };
}

/// The `User-Agent` of each fetch. NWS refuses a request with none.
const USER_AGENT: &str = concat!("gameclockd/", env!("CARGO_PKG_VERSION"));

/// The arguments of `curl` for `url`. The URL is the last argument, after
/// `--`, so `curl` cannot read it as an option. No shell reads them.
#[must_use]
pub fn curl_args(url: &str, limits: &Limits) -> Vec<String> {
    let time = limits.max_time.as_secs_f64().to_string();
    [
        "--silent",
        "--show-error",
        "--fail",
        "--max-time",
        &time,
        "--proto",
        "=http,https",
        "--user-agent",
        USER_AGENT,
        "--",
        url,
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// Fetch `url` with the system `curl` (docs/adr/0021). Block until the
/// fetch ends.
///
/// # Errors
/// The reason: `curl` cannot start, `curl` fails, or the body is larger than
/// the limit.
pub fn get(url: &str, limits: &Limits) -> Result<Vec<u8>, String> {
    let mut child = Command::new("curl")
        .args(curl_args(url, limits))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("cannot start curl: {e}"))?;
    let mut body = Vec::new();
    let read = match child.stdout.take() {
        Some(out) => {
            let most = u64::try_from(limits.max_bytes).unwrap_or(u64::MAX);
            out.take(most.saturating_add(1)).read_to_end(&mut body)
        }
        None => Ok(0),
    };
    if body.len() > limits.max_bytes {
        // Stop the transfer. The rest of the body is not necessary.
        let _ = child.kill();
        let _ = child.wait();
        return Err(format!(
            "the body is larger than {} KiB",
            limits.max_bytes / 1024
        ));
    }
    let mut message = String::new();
    if let Some(mut err) = child.stderr.take() {
        let _ = err.read_to_string(&mut message);
    }
    let status = child
        .wait()
        .map_err(|e| format!("curl did not stop: {e}"))?;
    if let Err(e) = read {
        return Err(format!("cannot read the curl output: {e}"));
    }
    if !status.success() {
        let message = message.trim();
        if message.is_empty() {
            return Err(format!("curl stopped with {status}"));
        }
        return Err(message.to_owned());
    }
    Ok(body)
}

/// Start the `fetch` thread. It fetches each job with [`get`] and sends the
/// result to the loop on `events`. Give the [`Fetch`] that sends it jobs.
///
/// # Errors
/// The error of the thread start.
pub fn spawn(events: mpsc::Sender<Event>) -> io::Result<ChannelFetch> {
    let (jobs, receiver) = mpsc::channel::<Job>();
    std::thread::Builder::new()
        .name("fetch".into())
        .stack_size(THREAD_STACK)
        .spawn(move || {
            while let Ok(mut job) = receiver.recv() {
                // Only the newest request shows, so skip the older jobs.
                while let Ok(newer) = receiver.try_recv() {
                    job = newer;
                }
                let body = get(&job.url, &Limits::PAGE);
                let fetched = Fetched {
                    seq: job.seq,
                    page: job.page,
                    body,
                };
                if events.send(Event::Fetched(fetched)).is_err() {
                    return;
                }
            }
        })?;
    Ok(ChannelFetch::new(jobs))
}

/// Sends each job to the `fetch` thread, and writes each warning to standard
/// error.
#[derive(Debug)]
pub struct ChannelFetch {
    jobs: mpsc::Sender<Job>,
}

impl ChannelFetch {
    /// A fetch that sends its jobs on `jobs`.
    #[must_use]
    pub fn new(jobs: mpsc::Sender<Job>) -> Self {
        Self { jobs }
    }
}

impl Fetch for ChannelFetch {
    fn start(&mut self, job: Job) {
        let page = job.page;
        if self.jobs.send(job).is_err() {
            eprintln!(
                "gameclockd: the page '{}' has no data: the fetch thread stopped.",
                page.name()
            );
        }
    }

    fn warn(&mut self, line: &str) {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;
    use std::time::Duration;

    /// What the test server does with its one request.
    #[derive(Debug, Clone, Copy)]
    enum Reply {
        /// Status 200 and this body, with a length.
        Body(&'static str),
        /// Status 200 and this many bytes, with no length.
        Bytes(usize),
        /// Status 404.
        NotFound,
        /// No response for this time.
        Silent(Duration),
    }

    /// A server for one request on 127.0.0.1. Give its URL and the request
    /// head that it received.
    fn server(reply: Reply) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/realtime.json", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut head = Vec::new();
            let mut byte = [0u8; 1];
            while !head.ends_with(b"\r\n\r\n") && stream.read(&mut byte).unwrap_or(0) == 1 {
                head.push(byte[0]);
            }
            let _ = tx.send(String::from_utf8_lossy(&head).into_owned());
            let _ = match reply {
                Reply::Body(body) => write!(
                    stream,
                    "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                ),
                Reply::Bytes(n) => {
                    let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n");
                    stream.write_all(&vec![b'a'; n])
                }
                Reply::NotFound => stream.write_all(
                    b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
                ),
                Reply::Silent(time) => {
                    thread::sleep(time);
                    Ok(())
                }
            };
        });
        (url, rx)
    }

    #[test]
    fn a_good_body() {
        let (url, _) = server(Reply::Body(r#"{"data":[]}"#));
        assert_eq!(get(&url, &Limits::PAGE), Ok(br#"{"data":[]}"#.to_vec()));
    }

    #[test]
    fn slow_server() {
        let (url, _) = server(Reply::Silent(Duration::from_secs(3)));
        let limits = Limits {
            max_time: Duration::from_secs(1),
            ..Limits::PAGE
        };
        let started = std::time::Instant::now();
        let error = get(&url, &limits).unwrap_err();
        assert!(started.elapsed() < Duration::from_secs(3), "{error}");
        assert!(error.starts_with("curl: (28)"), "{error}");
    }

    #[test]
    fn the_page_limits() {
        assert_eq!(Limits::PAGE.max_time, Duration::from_secs(10));
        assert_eq!(Limits::PAGE.max_bytes, 256 * 1024);
    }

    #[test]
    fn body_too_large() {
        let (url, _) = server(Reply::Bytes(300 * 1024));
        assert_eq!(
            get(&url, &Limits::PAGE),
            Err("the body is larger than 256 KiB".to_owned())
        );
    }

    #[test]
    fn not_found() {
        let (url, _) = server(Reply::NotFound);
        let error = get(&url, &Limits::PAGE).unwrap_err();
        assert!(error.starts_with("curl: (22)"), "{error}");
    }

    #[test]
    fn user_agent() {
        let (url, head) = server(Reply::Body("{}"));
        get(&url, &Limits::PAGE).unwrap();
        let head = head.recv().unwrap().to_ascii_lowercase();
        assert!(head.contains("\r\nuser-agent: gameclockd/"), "{head}");
    }

    #[test]
    fn the_url_goes_after_two_dashes() {
        let args = curl_args("http://a.example/x", &Limits::PAGE);
        let dashes = args.iter().position(|a| a == "--").unwrap();
        assert_eq!(args[dashes + 1..], ["http://a.example/x"]);
        assert!(args.windows(2).any(|w| w == ["--proto", "=http,https"]));
        assert!(args.windows(2).any(|w| w == ["--max-time", "10"]));
    }

    #[test]
    fn the_thread_sends_the_result_as_an_event() {
        let (url, _) = server(Reply::Body("{}"));
        let (events, receiver) = mpsc::channel();
        let mut fetch = spawn(events).unwrap();
        fetch.start(Job {
            seq: 7,
            page: gameclock_core::page::PageName::Indoor,
            url,
        });
        let event = receiver.recv_timeout(Duration::from_secs(5)).unwrap();
        let Event::Fetched(fetched) = event else {
            panic!("{event:?}");
        };
        assert_eq!(fetched.seq, 7);
        assert_eq!(fetched.body, Ok(b"{}".to_vec()));
    }
}
