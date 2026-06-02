use crate::error::{Error, Result};

const COOKIE_NAME: &str = "ponymail";

pub fn parse_ponymail_cookie(input: &str) -> Result<String> {
    if let Some(value) = parse_netscape_cookie(input) {
        return Ok(value);
    }

    let normalized = input.trim();
    let normalized = normalized
        .strip_prefix("Cookie:")
        .or_else(|| normalized.strip_prefix("cookie:"))
        .unwrap_or(normalized);

    for part in normalized.split(';') {
        let part = part.trim();
        if let Some((name, value)) = part.split_once('=') {
            if name.trim() == COOKIE_NAME {
                return validate_cookie_value(value.trim());
            }
        }
    }

    Err(Error::InvalidCookie(
        "could not find a ponymail cookie".to_string(),
    ))
}

fn parse_netscape_cookie(input: &str) -> Option<String> {
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() < 7 {
            continue;
        }

        let domain = fields[0].trim_start_matches('.');
        let name = fields[5];
        let value = fields[6];
        if domain == "lists.apache.org" && name == COOKIE_NAME {
            return validate_cookie_value(value).ok();
        }
    }

    None
}

fn validate_cookie_value(value: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(Error::InvalidCookie(
            "ponymail cookie value is empty".to_string(),
        ));
    }
    if value.contains(';') || value.contains('\n') || value.contains('\r') {
        return Err(Error::InvalidCookie(
            "ponymail cookie value contains invalid characters".to_string(),
        ));
    }
    Ok(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_ponymail_cookie;

    #[test]
    fn parse_plain_cookie() {
        let cookie = parse_ponymail_cookie("ponymail=abc-123").unwrap();
        assert_eq!(cookie, "abc-123");
    }

    #[test]
    fn parse_cookie_header() {
        let cookie = parse_ponymail_cookie("Cookie: other=x; ponymail=abc; foo=bar").unwrap();
        assert_eq!(cookie, "abc");
    }

    #[test]
    fn parse_netscape_cookie() {
        let input = ".lists.apache.org\tTRUE\t/\tTRUE\t0\tponymail\tabc";
        let cookie = parse_ponymail_cookie(input).unwrap();
        assert_eq!(cookie, "abc");
    }
}
