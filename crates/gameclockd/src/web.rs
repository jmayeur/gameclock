//! The web page: the routes, the forms, and the page
//! (`openspec/specs/web-control`, docs/adr/0020).
//!
//! The routes are pure functions. The listener thread only moves bytes.

use gameclock_core::message::Message;

use crate::command::{Command, CommandError, NO_ANSWER, parse_color};
use crate::http::{Request, Response, Status};

/// The type of a form body.
const FORM_TYPE: &str = "application/x-www-form-urlencoded";

/// The type of the page.
const HTML: &str = "text/html; charset=utf-8";

/// The legacy page sends these `msg` values for its two buttons.
const LEGACY_ALERTS: [(&str, &str); 2] = [("panic88", "panic"), ("bedtime", "bedtime")];

/// The answer of the loop to one command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The reply line of the loop.
    Answer(String),
    /// The loop did not answer in time.
    NoAnswer,
}

/// Give the response to `request`. `ask` sends one command to the loop and
/// gives its reply.
pub fn route(request: &Request, ask: &mut dyn FnMut(Command) -> Reply) -> Response {
    let method = request.method.as_str();
    let (allow, action): (&str, Option<Action>) = match request.path.as_str() {
        "/" => ("GET", None),
        "/message" => ("POST", Some(Action::Message)),
        "/alert" => ("POST", Some(Action::Alert)),
        "/alert/stop" => ("POST", Some(Action::Stop)),
        "/clear" => ("POST", Some(Action::Clear)),
        _ => return Response::refusal(Status::NotFound),
    };
    if method != allow {
        return Response::refusal(Status::MethodNotAllowed).with_header("Allow", allow);
    }
    match action {
        None => page(Status::Ok, None, ask),
        Some(action) => post(request, action, ask),
    }
}

#[derive(Debug, Clone, Copy)]
enum Action {
    Message,
    Alert,
    Stop,
    Clear,
}

fn post(request: &Request, action: Action, ask: &mut dyn FnMut(Command) -> Reply) -> Response {
    if !request.body.is_empty() && !is_form(request.header("content-type")) {
        return Response::refusal(Status::UnsupportedMediaType);
    }
    let Ok(form) = decode_form(&request.body) else {
        return Response::refusal(Status::BadRequest);
    };
    let command = match action {
        Action::Message => match message_command(&form) {
            Ok(command) => command,
            Err(e) => return page(Status::BadRequest, Some(&e.reply()), ask),
        },
        Action::Alert => Command::Alert(form.get("name").map(str::to_owned)),
        Action::Stop => Command::StopAlert,
        Action::Clear => Command::Clear,
    };
    match ask(command) {
        Reply::Answer(reply) => {
            let status = if reply == "ok" || reply.starts_with("ok ") {
                Status::Ok
            } else {
                Status::BadRequest
            };
            page(status, Some(&reply), ask)
        }
        Reply::NoAnswer => page(Status::Unavailable, Some(NO_ANSWER), ask),
    }
}

/// The command for a message form. The legacy values start their alerts.
fn message_command(form: &Form) -> Result<Command, CommandError> {
    // The legacy rule: each run of white space is one space.
    let text = form
        .get("msg")
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if let Some((_, alert)) = LEGACY_ALERTS.iter().find(|(value, _)| *value == text) {
        return Ok(Command::Alert(Some((*alert).to_owned())));
    }
    let passes = match form.get("passes").unwrap_or_default() {
        "" => Message::DEFAULT_PASSES,
        value => value.parse().map_err(|_| CommandError::Passes)?,
    };
    let color = match form.get("color").unwrap_or_default() {
        "" => Message::DEFAULT_COLOR,
        value => {
            // An `<input type="color">` value starts with `#`.
            let digits = value.strip_prefix('#').unwrap_or(value);
            parse_color(digits).ok_or(CommandError::Color)?
        }
    };
    Ok(Command::Message(Message::new(text, passes, color)?))
}

/// The page, with the reply of an action.
fn page(status: Status, reply: Option<&str>, _ask: &mut dyn FnMut(Command) -> Reply) -> Response {
    let reply = reply.unwrap_or_default();
    Response::new(status, HTML, format!("<!doctype html><p>{reply}</p>"))
}

/// A form value that the decoder cannot read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormError {
    /// A `%` with no two hexadecimal digits after it.
    BadCode,
    /// The bytes are not UTF-8 text.
    NotUtf8,
}

/// The fields of a form body, in their order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Form(Vec<(String, String)>);

impl Form {
    /// The last value of the field `name`.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .rev()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// True if `content_type` is a form type. A `charset` part does not count.
#[must_use]
pub fn is_form(content_type: Option<&str>) -> bool {
    content_type.is_some_and(|value| {
        let main = value.split_once(';').map_or(value, |(main, _)| main);
        main.trim().eq_ignore_ascii_case(FORM_TYPE)
    })
}

/// Decode an `application/x-www-form-urlencoded` body.
///
/// # Errors
/// [`FormError`] if a `%` code is bad or a field is not UTF-8.
pub fn decode_form(body: &[u8]) -> Result<Form, FormError> {
    let mut fields = Vec::new();
    for pair in body.split(|&b| b == b'&').filter(|pair| !pair.is_empty()) {
        let (name, value) = match pair.iter().position(|&b| b == b'=') {
            Some(at) => (
                pair.get(..at).unwrap_or_default(),
                pair.get(at + 1..).unwrap_or_default(),
            ),
            None => (pair, &b""[..]),
        };
        fields.push((decode_part(name)?, decode_part(value)?));
    }
    Ok(Form(fields))
}

fn decode_part(part: &[u8]) -> Result<String, FormError> {
    let mut bytes = Vec::with_capacity(part.len());
    let mut rest = part.iter();
    while let Some(&byte) = rest.next() {
        bytes.push(match byte {
            b'+' => b' ',
            b'%' => {
                let high = rest.next().and_then(|&b| hex(b));
                let low = rest.next().and_then(|&b| hex(b));
                match (high, low) {
                    (Some(high), Some(low)) => high << 4 | low,
                    _ => return Err(FormError::BadCode),
                }
            }
            other => other,
        });
    }
    String::from_utf8(bytes).map_err(|_| FormError::NotUtf8)
}

fn hex(b: u8) -> Option<u8> {
    char::from(b)
        .to_digit(16)
        .and_then(|d| u8::try_from(d).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use crate::http::{Parse, Request, Status, parse};
    use gameclock_core::embedded_graphics::pixelcolor::Rgb888;
    use gameclock_core::message::Message;

    const STATUS: &str = "ok mode=clock zone=UTC brightness=60 messages=2 alert=none";
    const NAMES: [&str; 3] = ["panic", "bedtime", "bed-flash"];

    fn request(bytes: &str) -> Request {
        match parse(bytes.as_bytes()) {
            Parse::Complete(request) => request,
            other => panic!("not complete: {other:?}"),
        }
    }

    fn post(path: &str, body: &str) -> Request {
        request(&format!(
            "POST {path} HTTP/1.1\r\nHost: clock.local\r\n\
             Content-Type: application/x-www-form-urlencoded\r\n\
             Content-Length: {}\r\n\r\n{body}",
            body.len()
        ))
    }

    /// The reply of a loop with the built-in alerts.
    fn loop_reply(command: &Command) -> Reply {
        Reply::Answer(match command {
            Command::Status => STATUS.to_owned(),
            Command::AlertNames => format!("ok {}", NAMES.join(" ")),
            Command::Alert(Some(name)) if !NAMES.contains(&name.as_str()) => format!(
                "error: unknown alert '{name}'. The alerts are panic, bedtime, and bed-flash."
            ),
            _ => "ok".to_owned(),
        })
    }

    /// Route `request` with a fake loop. Give the response and the commands
    /// that change the panel. The page also asks for the status and the alert
    /// names. Those do not count.
    fn send(request: &Request) -> (Response, Vec<Command>) {
        send_with(request, loop_reply)
    }

    fn send_with(request: &Request, reply: fn(&Command) -> Reply) -> (Response, Vec<Command>) {
        let mut seen = Vec::new();
        let response = route(request, &mut |command| {
            let answer = reply(&command);
            seen.push(command);
            answer
        });
        let actions = seen
            .into_iter()
            .filter(|c| !matches!(c, Command::Status | Command::AlertNames))
            .collect();
        (response, actions)
    }

    fn body(response: &Response) -> String {
        String::from_utf8(response.body.clone()).unwrap()
    }

    fn message(text: &str, passes: u8, color: Rgb888) -> Command {
        Command::Message(Message::new(text, passes, color).unwrap())
    }

    #[test]
    fn send_a_message() {
        let (response, actions) = send(&post("/message", "msg=Go+team%21&passes=2&color=ff0000"));
        assert_eq!(actions, [message("Go team!", 2, Rgb888::new(255, 0, 0))]);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(response.content_type, "text/html; charset=utf-8");
        assert!(body(&response).contains("ok"));
    }

    #[test]
    fn message_with_the_defaults() {
        for form in ["msg=hi", "msg=hi&passes=&color="] {
            let (_, actions) = send(&post("/message", form));
            assert_eq!(
                actions,
                [message(
                    "hi",
                    Message::DEFAULT_PASSES,
                    Message::DEFAULT_COLOR
                )],
                "{form}"
            );
        }
        assert_eq!(Message::DEFAULT_PASSES, 4);
        assert_eq!(Message::DEFAULT_COLOR, Rgb888::new(0, 150, 0));
    }

    #[test]
    fn message_with_white_space() {
        let (_, actions) = send(&post("/message", "msg=%20a%0D%0Ab%09%09c%20"));
        assert_eq!(actions, [message("a b c", 4, Message::DEFAULT_COLOR)]);
    }

    #[test]
    fn color_from_a_color_input() {
        let (_, actions) = send(&post("/message", "msg=hi&color=%2300FF00"));
        assert_eq!(actions, [message("hi", 4, Rgb888::new(0, 255, 0))]);
    }

    #[test]
    fn message_that_is_too_long() {
        let (response, actions) = send(&post("/message", &format!("msg={}", "a".repeat(65))));
        assert_eq!(response.status, Status::BadRequest);
        assert!(body(&response).contains("the message is longer than 64 characters."));
        assert_eq!(actions, []);
    }

    #[test]
    fn empty_message() {
        let (response, actions) = send(&post("/message", "msg=+++"));
        assert_eq!(response.status, Status::BadRequest);
        assert!(body(&response).contains("the message has no text."));
        assert_eq!(actions, []);
        let (response, _) = send(&post("/message", "passes=2"));
        assert_eq!(response.status, Status::BadRequest);
    }

    #[test]
    fn bad_passes_or_color() {
        for form in ["msg=hi&passes=11", "msg=hi&passes=0", "msg=hi&passes=two"] {
            let (response, actions) = send(&post("/message", form));
            assert_eq!(response.status, Status::BadRequest, "{form}");
            assert!(body(&response).contains("--passes must be a number from 1 to 10."));
            assert_eq!(actions, []);
        }
        for form in [
            "msg=hi&color=red",
            "msg=hi&color=%23%23ff0000",
            "msg=hi&color=ff00001",
        ] {
            let (response, actions) = send(&post("/message", form));
            assert_eq!(response.status, Status::BadRequest, "{form}");
            assert!(body(&response).contains("--color must be six hexadecimal digits"));
            assert_eq!(actions, []);
        }
    }

    #[test]
    fn start_an_alert() {
        let (response, actions) = send(&post("/alert", "name=panic"));
        assert_eq!(actions, [Command::Alert(Some("panic".into()))]);
        assert_eq!(response.status, Status::Ok);
    }

    #[test]
    fn unknown_alert() {
        let (response, actions) = send(&post("/alert", "name=dinner"));
        assert_eq!(actions, [Command::Alert(Some("dinner".into()))]);
        assert_eq!(response.status, Status::BadRequest);
        assert!(body(&response).contains("The alerts are panic, bedtime, and bed-flash."));
    }

    #[test]
    fn alert_with_no_name() {
        let (_, actions) = send(&post("/alert", ""));
        assert_eq!(actions, [Command::Alert(None)]);
    }

    #[test]
    fn stop_and_clear() {
        let (stop, a1) = send(&post("/alert/stop", ""));
        let (clear, a2) = send(&post("/clear", ""));
        assert_eq!(a1, [Command::StopAlert]);
        assert_eq!(a2, [Command::Clear]);
        assert_eq!(stop.status, Status::Ok);
        assert_eq!(clear.status, Status::Ok);
    }

    #[test]
    fn the_loop_does_not_answer() {
        let (response, actions) = send_with(&post("/message", "msg=hi"), |_| Reply::NoAnswer);
        assert_eq!(response.status, Status::Unavailable);
        assert!(body(&response).contains("the daemon did not answer in 2 s."));
        assert_eq!(actions, [message("hi", 4, Message::DEFAULT_COLOR)]);
    }

    #[test]
    fn legacy_buttons() {
        let (_, panic) = send(&post("/message", "msg=panic88"));
        let (_, bedtime) = send(&post("/message", "msg=bedtime"));
        assert_eq!(panic, [Command::Alert(Some("panic".into()))]);
        assert_eq!(bedtime, [Command::Alert(Some("bedtime".into()))]);
    }

    #[test]
    fn post_that_is_not_a_form() {
        let request = request(
            "POST /message HTTP/1.1\r\nContent-Type: application/json\r\n\
             Content-Length: 12\r\n\r\n{\"msg\":\"hi\"}",
        );
        let (response, actions) = send(&request);
        assert_eq!(response.status, Status::UnsupportedMediaType);
        assert_eq!(actions, []);
    }

    #[test]
    fn empty_post_with_no_type() {
        let request = request("POST /clear HTTP/1.1\r\nContent-Length: 0\r\n\r\n");
        let (response, actions) = send(&request);
        assert_eq!(response.status, Status::Ok);
        assert_eq!(actions, [Command::Clear]);
    }

    #[test]
    fn bad_percent_code() {
        for form in ["msg=%G1", "msg=%FF"] {
            let (response, actions) = send(&post("/message", form));
            assert_eq!(response.status, Status::BadRequest, "{form}");
            assert_eq!(actions, []);
        }
    }

    #[test]
    fn unknown_path_and_wrong_method() {
        let (missing, _) = send(&request("GET /nothing HTTP/1.1\r\n\r\n"));
        assert_eq!(missing.status, Status::NotFound);
        for (head, allow) in [
            ("DELETE / HTTP/1.1\r\n\r\n", "GET"),
            ("OPTIONS / HTTP/1.1\r\n\r\n", "GET"),
            ("GET /message HTTP/1.1\r\n\r\n", "POST"),
        ] {
            let (response, actions) = send(&request(head));
            assert_eq!(response.status, Status::MethodNotAllowed, "{head:?}");
            assert_eq!(response.headers, [("Allow", allow.to_owned())], "{head:?}");
            assert_eq!(actions, []);
        }
    }

    #[test]
    fn plus_and_percent_codes() {
        let form = decode_form(b"msg=Go+team%21&color=%23ff0000").unwrap();
        assert_eq!(form.get("msg"), Some("Go team!"));
        assert_eq!(form.get("color"), Some("#ff0000"));
        assert_eq!(form.get("passes"), None);
    }

    #[test]
    fn utf8_from_percent_codes() {
        let form = decode_form(b"msg=caf%C3%A9").unwrap();
        assert_eq!(form.get("msg"), Some("café"));
    }

    #[test]
    fn bad_percent_codes() {
        for body in [&b"msg=%G1"[..], b"msg=%4", b"msg=%", b"m%zz=1"] {
            assert_eq!(decode_form(body), Err(FormError::BadCode), "{body:?}");
        }
    }

    #[test]
    fn not_utf8() {
        assert_eq!(decode_form(b"msg=%FF"), Err(FormError::NotUtf8));
        assert_eq!(decode_form(b"msg=\xff"), Err(FormError::NotUtf8));
    }

    #[test]
    fn the_last_value_wins() {
        let form = decode_form(b"name=panic&name=bedtime").unwrap();
        assert_eq!(form.get("name"), Some("bedtime"));
    }

    #[test]
    fn fields_with_no_value() {
        let form = decode_form(b"msg&passes=&&").unwrap();
        assert_eq!(form.get("msg"), Some(""));
        assert_eq!(form.get("passes"), Some(""));
        assert_eq!(decode_form(b"").unwrap().get("msg"), None);
    }

    #[test]
    fn form_content_types() {
        assert!(is_form(Some("application/x-www-form-urlencoded")));
        assert!(is_form(Some(
            "application/x-www-form-urlencoded; charset=UTF-8"
        )));
        assert!(is_form(Some("Application/X-WWW-Form-Urlencoded")));
        assert!(!is_form(Some("application/json")));
        assert!(!is_form(Some("multipart/form-data; boundary=x")));
        assert!(!is_form(None));
    }
}
