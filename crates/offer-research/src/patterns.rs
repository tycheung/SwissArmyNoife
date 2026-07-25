//! Curated pattern helpers for research indexing (`sak245`).
//!
//! Exported from `offer-research` for brief/fetch post-processing:
//!
//! - [`extract_urls`] — scan free text for `http://` / `https://` URLs
//! - [`keyword_hits`] — case-insensitive needle list against body text
//! - [`extract_domains`] — unique hosts from URLs plus bare domain-like tokens

use std::collections::HashSet;

fn url_terminates(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r' | ')' | ']' | '"' | '\'' | '<')
}

fn host_from_url(url: &str) -> Option<String> {
    let rest = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let host = rest.split('/').next()?.split(':').next()?.trim();
    if host.is_empty() || !host.contains('.') {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

fn is_domain_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '.'
}

fn looks_like_domain(token: &str) -> bool {
    if token.len() < 4 || token.starts_with('.') || token.ends_with('.') {
        return false;
    }
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() < 2 {
        return false;
    }
    parts.iter().all(|part| {
        !part.is_empty()
            && part.len() <= 63
            && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
    }) && parts
        .last()
        .is_some_and(|t| t.len() >= 2 && t.chars().all(|c| c.is_ascii_alphabetic()))
}

fn bare_domains(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut token = String::new();
    for c in text.chars().chain(std::iter::once(' ')) {
        if is_domain_char(c) {
            token.push(c);
        } else if !token.is_empty() {
            if looks_like_domain(&token) {
                out.push(token.to_ascii_lowercase());
            }
            token.clear();
        }
    }
    out
}

/// Extract unique domains from URLs (preferred) and bare domain-like tokens.
///
/// URL hosts are collected first; bare tokens follow. Order is preserved; duplicates removed.
#[must_use]
pub fn extract_domains(text: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for url in extract_urls(text) {
        if let Some(host) = host_from_url(&url) {
            if seen.insert(host.clone()) {
                out.push(host);
            }
        }
    }
    for domain in bare_domains(text) {
        if seen.insert(domain.clone()) {
            out.push(domain);
        }
    }
    out
}

/// Extract `http://` and `https://` URLs from free text (simple scan, no HTML parsing).
#[must_use]
pub fn extract_urls(text: &str) -> Vec<String> {
    let mut urls = Vec::new();
    let mut search_from = 0;
    while search_from < text.len() {
        let slice = &text[search_from..];
        let rel_http = slice.find("http://");
        let rel_https = slice.find("https://");
        let rel = match (rel_http, rel_https) {
            (Some(h), Some(s)) => Some(h.min(s)),
            (Some(h), None) => Some(h),
            (None, Some(s)) => Some(s),
            (None, None) => None,
        };
        let Some(rel) = rel else {
            break;
        };
        let start = search_from + rel;
        let rest = &text[start..];
        let end = rest.find(url_terminates).unwrap_or(rest.len());
        urls.push(rest[..end].to_string());
        search_from = start + end;
    }
    urls
}

/// Return needles that appear in `text` (case-insensitive), preserving input order.
#[must_use]
pub fn keyword_hits(text: &str, needles: &[&str]) -> Vec<String> {
    let haystack = text.to_lowercase();
    needles
        .iter()
        .filter(|needle| haystack.contains(&needle.to_lowercase()))
        .map(|needle| (*needle).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_urls_finds_http_and_https() {
        let text = "See https://example.com/a and http://foo.test/path).";
        assert_eq!(
            extract_urls(text),
            vec![
                "https://example.com/a".to_string(),
                "http://foo.test/path".to_string(),
            ]
        );
    }

    #[test]
    fn extract_urls_skips_when_none() {
        assert!(extract_urls("no links here").is_empty());
    }

    #[test]
    fn keyword_hits_case_insensitive_preserves_order() {
        let text = "Rust MCP broker";
        assert_eq!(
            keyword_hits(text, &["mcp", "python", "broker"]),
            vec!["mcp".to_string(), "broker".to_string()]
        );
    }

    #[test]
    fn keyword_hits_empty_when_no_match() {
        assert!(keyword_hits("hello", &["world"]).is_empty());
    }

    #[test]
    fn extract_domains_prefers_url_hosts_then_bare() {
        let text = "Visit https://Example.COM/a and also docs.rust-lang.org offline.";
        assert_eq!(
            extract_domains(text),
            vec!["example.com".to_string(), "docs.rust-lang.org".to_string(),]
        );
    }

    #[test]
    fn extract_domains_dedupes_preserving_order() {
        let text = "https://foo.test/x foo.test bar.co.uk";
        assert_eq!(
            extract_domains(text),
            vec!["foo.test".to_string(), "bar.co.uk".to_string()]
        );
    }

    #[test]
    fn extract_domains_empty_when_none() {
        assert!(extract_domains("no domains here").is_empty());
    }
}
