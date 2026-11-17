//! A small HTTP/1.1 subset for the web page (`openspec/specs/web-control`,
//! docs/adr/0020).
//!
//! These are pure functions on bytes. The listener thread only moves bytes.

/// The most bytes for the request line and the headers, with the empty line
/// at their end.
pub const MAX_HEAD: usize = 8 * 1024;

/// The most bytes for a request body.
pub const MAX_BODY: usize = 1024;

/// A response status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// 200: the request worked.
    Ok,
    /// 400: the request is not correct.
    BadRequest,
    /// 403: the request comes from a different site.
    Forbidden,
    /// 404: the path is not a route.
    NotFound,
    /// 405: the route does not have this method.
    MethodNotAllowed,
    /// 408: the request did not arrive in time.
    RequestTimeout,
    /// 411: a body with no `Content-Length`.
    LengthRequired,
    /// 413: the body is too large.
    PayloadTooLarge,
    /// 415: the body is not a form.
    UnsupportedMediaType,
    /// 431: the request line and the headers are too large.
    HeadersTooLarge,
    /// 503: the loop did not answer.
    Unavailable,
}

impl Status {
    /// The status code.
    #[must_use]
    pub fn code(self) -> u16 {
        match self {
            Self::Ok => 200,
            Self::BadRequest => 400,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::MethodNotAllowed => 405,
            Self::RequestTimeout => 408,
            Self::LengthRequired => 411,
            Self::PayloadTooLarge => 413,
            Self::UnsupportedMediaType => 415,
            Self::HeadersTooLarge => 431,
            Self::Unavailable => 503,
        }
    }

    /// The reason text of the status line.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::BadRequest => "Bad Request",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::RequestTimeout => "Request Timeout",
            Self::LengthRequired => "Length Required",
            Self::PayloadTooLarge => "Payload Too Large",
            Self::UnsupportedMediaType => "Unsupported Media Type",
            Self::HeadersTooLarge => "Request Header Fields Too Large",
            Self::Unavailable => "Service Unavailable",
        }
    }
}

/// A complete request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The method, for example `GET`.
    pub method: String,
    /// The path, with no query.
    pub path: String,
    /// The headers, in their order. The values have no spaces at the start
    /// and the end.
    pub headers: Vec<(String, String)>,
    /// The body. It has `Content-Length` bytes.
    pub body: Vec<u8>,
}

impl Request {
    /// The value of the first header with this name. The case of the
    /// letters does not count.
    #[must_use]
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// A response. Each response closes the connection (docs/adr/0020).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    /// The status.
    pub status: Status,
    /// The value of the `Content-Type` header.
    pub content_type: &'static str,
    /// More headers, after the fixed headers.
    pub headers: Vec<(&'static str, String)>,
    /// The body.
    pub body: Vec<u8>,
}

impl Response {
    /// A response with this status, type, and body.
    #[must_use]
    pub fn new(status: Status, content_type: &'static str, body: impl Into<Vec<u8>>) -> Self {
        Self {
            status,
            content_type,
            headers: Vec::new(),
            body: body.into(),
        }
    }

    /// A refusal: the status code and its reason as a text body.
    #[must_use]
    pub fn refusal(status: Status) -> Self {
        Self::new(
            status,
            "text/plain; charset=utf-8",
            format!("{} {}\n", status.code(), status.reason()),
        )
    }

    /// Add a header.
    #[must_use]
    pub fn with_header(mut self, name: &'static str, value: impl Into<String>) -> Self {
        self.headers.push((name, value.into()));
        self
    }

    /// The bytes to write on the connection.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut head = format!(
            "HTTP/1.1 {} {}\r\n\
             Content-Type: {}\r\n\
             Content-Length: {}\r\n\
             Connection: close\r\n\
             Cache-Control: no-store\r\n\
             X-Content-Type-Options: nosniff\r\n",
            self.status.code(),
            self.status.reason(),
            self.content_type,
            self.body.len(),
        );
        for (name, value) in &self.headers {
            head.push_str(name);
            head.push_str(": ");
            head.push_str(value);
            head.push_str("\r\n");
        }
        head.push_str("\r\n");
        let mut bytes = head.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

/// The result of a parse of the bytes that arrived.
#[derive(Debug)]
pub enum Parse {
    /// More bytes are necessary.
    Partial,
    /// The bytes have a complete request.
    Complete(Request),
    /// The listener must refuse the request with this status.
    Refused(Status),
}

/// Parse the bytes that arrived on a connection.
#[must_use]
pub fn parse(bytes: &[u8]) -> Parse {
    let Some(head_end) = head_end(bytes) else {
        return if bytes.len() > MAX_HEAD {
            Parse::Refused(Status::HeadersTooLarge)
        } else {
            Parse::Partial
        };
    };
    if head_end > MAX_HEAD {
        return Parse::Refused(Status::HeadersTooLarge);
    }
    let Some(head) = bytes
        .get(..head_end)
        .and_then(|h| core::str::from_utf8(h).ok())
    else {
        return Parse::Refused(Status::BadRequest);
    };
    let (method, path, headers) = match parse_head(head) {
        Ok(parts) => parts,
        Err(status) => return Parse::Refused(status),
    };
    let length = match body_length(&method, &headers) {
        Ok(length) => length,
        Err(status) => return Parse::Refused(status),
    };
    let rest = bytes.get(head_end..).unwrap_or_default();
    match rest.get(..length) {
        Some(body) => Parse::Complete(Request {
            method,
            path,
            headers,
            body: body.to_vec(),
        }),
        None => Parse::Partial,
    }
}

/// The index after the empty line that ends the head, or `None`.
fn head_end(bytes: &[u8]) -> Option<usize> {
    let mut start = 0;
    for (at, &byte) in bytes.iter().enumerate() {
        if byte == b'\n' {
            let line = bytes.get(start..at).unwrap_or_default();
            if line.is_empty() || line == b"\r" {
                return Some(at + 1);
            }
            start = at + 1;
        }
    }
    None
}

type Head = (String, String, Vec<(String, String)>);

fn parse_head(head: &str) -> Result<Head, Status> {
    let mut lines = head
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line));
    let (method, path) = parse_request_line(lines.next().unwrap_or_default())?;
    let mut headers = Vec::new();
    for line in lines.take_while(|line| !line.is_empty()) {
        if line.starts_with([' ', '\t']) {
            // A folded header line is obsolete.
            return Err(Status::BadRequest);
        }
        let (name, value) = line.split_once(':').ok_or(Status::BadRequest)?;
        if name.is_empty() || !name.bytes().all(is_token_byte) {
            return Err(Status::BadRequest);
        }
        headers.push((name.to_owned(), value.trim_matches([' ', '\t']).to_owned()));
    }
    Ok((method, path, headers))
}

fn parse_request_line(line: &str) -> Result<(String, String), Status> {
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(Status::BadRequest);
    };
    if method.is_empty() || !method.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(Status::BadRequest);
    }
    if !target.starts_with('/') || target.bytes().any(|b| b.is_ascii_control()) {
        return Err(Status::BadRequest);
    }
    if version != "HTTP/1.0" && version != "HTTP/1.1" {
        return Err(Status::BadRequest);
    }
    let path = target.split_once('?').map_or(target, |(path, _)| path);
    Ok((method.to_owned(), path.to_owned()))
}

/// A byte that a header name can have (RFC 9110, `tchar`).
fn is_token_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b)
}

/// The length of the body from the headers.
fn body_length(method: &str, headers: &[(String, String)]) -> Result<usize, Status> {
    let chunked = headers
        .iter()
        .any(|(name, _)| name.eq_ignore_ascii_case("transfer-encoding"));
    let mut length: Option<usize> = None;
    for (_, value) in headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("content-length"))
    {
        if value.is_empty() || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Status::BadRequest);
        }
        // A number that is too large for usize is also too large for the body.
        let value = value.parse().unwrap_or(usize::MAX);
        if length.is_some_and(|l| l != value) {
            return Err(Status::BadRequest);
        }
        length = Some(value);
    }
    match (length, chunked) {
        // Two lengths can give two different bodies.
        (Some(_), true) => Err(Status::BadRequest),
        (None, true) => Err(Status::LengthRequired),
        (None, false) if method == "POST" => Err(Status::LengthRequired),
        (None, false) => Ok(0),
        (Some(length), false) if length > MAX_BODY => Err(Status::PayloadTooLarge),
        (Some(length), false) => Ok(length),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn complete(bytes: &[u8]) -> Request {
        match parse(bytes) {
            Parse::Complete(request) => request,
            other => panic!("not complete: {other:?}"),
        }
    }

    fn refused(bytes: &[u8]) -> Status {
        match parse(bytes) {
            Parse::Refused(status) => status,
            other => panic!("not refused: {other:?}"),
        }
    }

    #[test]
    fn complete_get() {
        let request = complete(b"GET /health?x=1 HTTP/1.1\r\nHost: clock.local\r\n\r\n");
        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/health");
        assert_eq!(request.header("host"), Some("clock.local"));
        assert_eq!(request.header("HOST"), Some("clock.local"));
        assert_eq!(request.header("origin"), None);
        assert_eq!(request.body, b"");
    }

    #[test]
    fn post_with_a_body() {
        let request = complete(
            b"POST /message HTTP/1.0\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: 6\r\n\r\nmsg=hiEXTRA",
        );
        assert_eq!(request.method, "POST");
        assert_eq!(request.path, "/message");
        assert_eq!(request.body, b"msg=hi");
    }

    #[test]
    fn header_value_spaces_do_not_count() {
        let request = complete(b"GET / HTTP/1.1\r\nHost:   clock.local  \r\n\r\n");
        assert_eq!(request.header("Host"), Some("clock.local"));
    }

    #[test]
    fn partial_head() {
        assert!(matches!(parse(b""), Parse::Partial));
        assert!(matches!(
            parse(b"GET / HTTP/1.1\r\nHost: a"),
            Parse::Partial
        ));
        assert!(matches!(parse(b"GET / HTTP/1.1\r\n"), Parse::Partial));
    }

    #[test]
    fn partial_body() {
        assert!(matches!(
            parse(b"POST /clear HTTP/1.1\r\nContent-Length: 10\r\n\r\nabc"),
            Parse::Partial
        ));
    }

    #[test]
    fn line_feeds_without_carriage_returns() {
        let request = complete(b"POST /alert HTTP/1.1\nContent-Length: 10\n\nname=panic");
        assert_eq!(request.path, "/alert");
        assert_eq!(request.body, b"name=panic");
    }

    #[test]
    fn head_over_the_limit_with_no_line_end() {
        let mut bytes = b"GET / HTTP/1.1\r\nX-Long: ".to_vec();
        bytes.extend(std::iter::repeat_n(b'a', 9000));
        assert_eq!(refused(&bytes), Status::HeadersTooLarge);
    }

    #[test]
    fn complete_head_over_the_limit() {
        let mut bytes = b"GET / HTTP/1.1\r\nX-Long: ".to_vec();
        bytes.extend(std::iter::repeat_n(b'a', MAX_HEAD));
        bytes.extend_from_slice(b"\r\n\r\n");
        assert_eq!(refused(&bytes), Status::HeadersTooLarge);
    }

    #[test]
    fn head_at_the_limit_is_read() {
        let start = b"GET / HTTP/1.1\r\nX-Long: ";
        let end = b"\r\n\r\n";
        let mut bytes = start.to_vec();
        bytes.extend(std::iter::repeat_n(
            b'a',
            MAX_HEAD - start.len() - end.len(),
        ));
        bytes.extend_from_slice(end);
        assert_eq!(bytes.len(), MAX_HEAD);
        assert_eq!(complete(&bytes).path, "/");
    }

    #[test]
    fn body_over_the_limit() {
        assert_eq!(
            refused(b"POST /message HTTP/1.1\r\nContent-Length: 2000\r\n\r\n"),
            Status::PayloadTooLarge
        );
        let at_limit = format!("POST /message HTTP/1.1\r\nContent-Length: {MAX_BODY}\r\n\r\n");
        assert!(matches!(parse(at_limit.as_bytes()), Parse::Partial));
    }

    #[test]
    fn post_with_no_length() {
        assert_eq!(
            refused(
                b"POST /message HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n2\r\nhi\r\n0\r\n\r\n"
            ),
            Status::LengthRequired
        );
        assert_eq!(
            refused(b"POST /message HTTP/1.1\r\n\r\n"),
            Status::LengthRequired
        );
    }

    #[test]
    fn get_with_no_length_has_no_body() {
        assert_eq!(complete(b"GET / HTTP/1.1\r\n\r\n").body, b"");
    }

    #[test]
    fn bad_or_doubled_length() {
        for head in [
            "POST / HTTP/1.1\r\nContent-Length: abc\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: -1\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: \r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: 1\r\nContent-Length: 2\r\n\r\nab",
            "POST / HTTP/1.1\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\nab",
        ] {
            assert_eq!(refused(head.as_bytes()), Status::BadRequest, "{head:?}");
        }
        // The same value two times is one length.
        let request =
            complete(b"POST / HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 2\r\n\r\nab");
        assert_eq!(request.body, b"ab");
    }

    /// A small xorshift generator. A fixed seed gives the same inputs each run.
    struct Random(u64);

    impl Random {
        fn next(&mut self) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0
        }

        fn below(&mut self, n: usize) -> usize {
            usize::try_from(self.next() % n as u64).unwrap()
        }
    }

    /// A good request with headers of random sizes. Some heads are over the
    /// limit.
    fn random_request(random: &mut Random) -> Vec<u8> {
        let mut bytes = if random.below(2) == 0 {
            b"GET /health HTTP/1.1\r\n".to_vec()
        } else {
            b"POST /message HTTP/1.1\r\nContent-Length: 6\r\n".to_vec()
        };
        for _ in 0..random.below(6) {
            bytes.extend_from_slice(b"X-Pad: ");
            bytes.extend(std::iter::repeat_n(b'a', random.below(3000)));
            bytes.extend_from_slice(b"\r\n");
        }
        bytes.extend_from_slice(b"\r\nmsg=hi");
        bytes
    }

    /// Check one parse: no panic, and no complete request with a head over
    /// the limit.
    fn check(bytes: &[u8]) {
        if let Parse::Complete(request) = parse(bytes) {
            let head = head_end(bytes).unwrap();
            assert!(head <= MAX_HEAD, "head of {head} bytes");
            assert!(request.body.len() <= MAX_BODY);
        }
    }

    #[test]
    fn random_bytes() {
        let mut random = Random(0x5eed_1234_abcd_0001);
        for _ in 0..10_000 {
            let length = random.below(MAX_HEAD + 2 * MAX_BODY);
            let bytes: Vec<u8> = (0..length)
                .map(|_| {
                    // Many line ends, so that some heads end.
                    if random.below(8) == 0 {
                        b'\n'
                    } else {
                        random.next().to_le_bytes()[0]
                    }
                })
                .collect();
            check(&bytes);
        }
        for _ in 0..2_000 {
            let mut bytes = random_request(&mut random);
            check(&bytes);
            let cut = random.below(bytes.len() + 1);
            check(bytes.get(..cut).unwrap());
            let at = random.below(bytes.len());
            bytes[at] = random.next().to_le_bytes()[0];
            check(&bytes);
        }
    }

    fn text_of(response: &Response) -> String {
        String::from_utf8(response.to_bytes()).unwrap()
    }

    #[test]
    fn response_bytes() {
        let response = Response::new(Status::Ok, "text/html; charset=utf-8", "<p>hi</p>");
        assert_eq!(
            text_of(&response),
            "HTTP/1.1 200 OK\r\n\
             Content-Type: text/html; charset=utf-8\r\n\
             Content-Length: 9\r\n\
             Connection: close\r\n\
             Cache-Control: no-store\r\n\
             X-Content-Type-Options: nosniff\r\n\
             \r\n\
             <p>hi</p>"
        );
    }

    #[test]
    fn response_with_a_header() {
        let response =
            Response::new(Status::MethodNotAllowed, "text/plain", "").with_header("Allow", "GET");
        let text = text_of(&response);
        assert!(
            text.starts_with("HTTP/1.1 405 Method Not Allowed\r\n"),
            "{text}"
        );
        assert!(text.contains("\r\nAllow: GET\r\n"), "{text}");
        assert!(text.contains("\r\nContent-Length: 0\r\n"), "{text}");
        assert!(text.ends_with("\r\n\r\n"), "{text}");
    }

    #[test]
    fn length_counts_bytes_not_characters() {
        let response = Response::new(Status::Ok, "text/plain; charset=utf-8", "é");
        assert!(text_of(&response).contains("Content-Length: 2\r\n"));
    }

    #[test]
    fn refusal_has_a_short_text_body() {
        let text = text_of(&Response::refusal(Status::HeadersTooLarge));
        assert!(
            text.starts_with("HTTP/1.1 431 Request Header Fields Too Large\r\n"),
            "{text}"
        );
        assert!(
            text.contains("Content-Type: text/plain; charset=utf-8\r\n"),
            "{text}"
        );
        assert!(
            text.ends_with("\r\n\r\n431 Request Header Fields Too Large\n"),
            "{text}"
        );
    }

    #[test]
    fn no_cors_headers() {
        for status in [Status::Ok, Status::Forbidden, Status::NotFound] {
            let text = text_of(&Response::refusal(status));
            assert!(
                !text.to_ascii_lowercase().contains("access-control-"),
                "{text}"
            );
        }
    }

    #[test]
    fn bad_request_lines_and_headers() {
        for head in [
            "GET / HTTP/2.0\r\n\r\n",
            "GET / HTTP/1.1 extra\r\n\r\n",
            "GET /\r\n\r\n",
            "GET  / HTTP/1.1\r\n\r\n",
            "get / HTTP/1.1\r\n\r\n",
            "GET nothing HTTP/1.1\r\n\r\n",
            "\r\n\r\n",
            "GET / HTTP/1.1\r\nNo colon\r\n\r\n",
            "GET / HTTP/1.1\r\n: no name\r\n\r\n",
            "GET / HTTP/1.1\r\nBad name: x\r\n\r\n",
            "GET / HTTP/1.1\r\nHost: a\r\n folded\r\n\r\n",
        ] {
            assert_eq!(refused(head.as_bytes()), Status::BadRequest, "{head:?}");
        }
        assert_eq!(
            refused(b"GET / HTTP/1.1\r\nX: \xff\r\n\r\n"),
            Status::BadRequest
        );
    }
}
