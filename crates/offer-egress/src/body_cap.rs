//! Enforce response body size against [`ResponseByteCap`].

use types::ErrorCode;

use crate::byte_cap::ResponseByteCap;

/// Reject bodies larger than the binding cap.
///
/// # Errors
/// [`ErrorCode::BudgetExhausted`] when over cap.
pub fn enforce_response_bytes(cap: &ResponseByteCap, body: &[u8]) -> Result<(), ErrorCode> {
    cap.permits_len(body.len() as u64)
}

/// Read from `chunks` until EOF or cap breach (stops early on oversize).
///
/// # Errors
/// [`ErrorCode::BudgetExhausted`] when accumulated size would exceed the cap.
pub fn collect_capped(
    cap: &ResponseByteCap,
    chunks: impl IntoIterator<Item = impl AsRef<[u8]>>,
) -> Result<Vec<u8>, ErrorCode> {
    let mut out = Vec::new();
    for chunk in chunks {
        let chunk = chunk.as_ref();
        let next = (out.len() as u64).saturating_add(chunk.len() as u64);
        cap.permits_len(next)?;
        out.extend_from_slice(chunk);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforce_and_collect() {
        let cap = ResponseByteCap::new(5);
        assert!(enforce_response_bytes(&cap, b"12345").is_ok());
        assert_eq!(
            enforce_response_bytes(&cap, b"123456"),
            Err(ErrorCode::BudgetExhausted)
        );
        let ok = collect_capped(&cap, [b"ab".as_slice(), b"cd", b"e"]).unwrap();
        assert_eq!(ok, b"abcde");
        assert_eq!(
            collect_capped(&cap, [b"ab".as_slice(), b"cdef"]),
            Err(ErrorCode::BudgetExhausted)
        );
    }
}
