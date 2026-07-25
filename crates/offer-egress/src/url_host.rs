//! Extract hostname from a URL or bare host string.

use types::ErrorCode;

use crate::allowlist::HostnameAllowlist;

/// Parse host from `https://user:pass@api.example.com:443/path?q=1` or bare `api.example.com`.
///
/// # Errors
/// [`ErrorCode::SchemaInvalid`] when empty or unparsable.
pub fn host_from_url(raw: &str) -> Result<String, ErrorCode> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(ErrorCode::SchemaInvalid);
    }
    let after_scheme = if let Some(rest) = s.split_once("://") {
        rest.1
    } else {
        s
    };
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or("")
        .trim();
    if authority.is_empty() {
        return Err(ErrorCode::SchemaInvalid);
    }
    let hostport = authority.rsplit('@').next().unwrap_or(authority);
    let host = if hostport.starts_with('[') {
        // IPv6 literal [::1]:port
        hostport
            .trim_start_matches('[')
            .split(']')
            .next()
            .unwrap_or("")
    } else {
        hostport.split(':').next().unwrap_or(hostport)
    };
    let host = host.trim().to_ascii_lowercase();
    if host.is_empty() {
        return Err(ErrorCode::SchemaInvalid);
    }
    Ok(host)
}

/// Extract host then run [`HostnameAllowlist::permits`].
///
/// # Errors
/// Schema or egress deny from underlying helpers.
pub fn check_url(allow: &HostnameAllowlist, url: &str) -> Result<String, ErrorCode> {
    let host = host_from_url(url)?;
    allow.permits(&host)?;
    Ok(host)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HostnameAllowlist;
    use serde_json::json;

    #[test]
    fn parses_urls_and_bare() {
        assert_eq!(
            host_from_url("https://User:x@API.Example.com:443/v1?q=1").unwrap(),
            "api.example.com"
        );
        assert_eq!(host_from_url("api.example.com").unwrap(), "api.example.com");
    }

    #[test]
    fn check_url_allow_deny() {
        let a = HostnameAllowlist::from_policy(&json!({
            "egress": { "allow_hosts": ["api.example.com"] }
        }));
        assert!(check_url(&a, "https://api.example.com/x").is_ok());
        assert_eq!(
            check_url(&a, "https://evil.com/"),
            Err(ErrorCode::EgressDenied)
        );
    }
}
