//! The web page: the routes, the forms, and the page
//! (`openspec/specs/web-control`, docs/adr/0020).
//!
//! The routes are pure functions. The listener thread only moves bytes.

/// The type of a form body.
const FORM_TYPE: &str = "application/x-www-form-urlencoded";

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
