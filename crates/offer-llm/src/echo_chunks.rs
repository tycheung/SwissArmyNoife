//! Echo chunk splitting (`sak137-b` helper).

use provider_core::ChatChunk;

/// Split full assistant text into whitespace-ish deltas + a final done chunk.
#[must_use]
pub fn echo_chunks(full: &str) -> Vec<ChatChunk> {
    let mut out = Vec::new();
    let mut buf = String::new();
    for ch in full.chars() {
        buf.push(ch);
        if ch.is_whitespace() {
            out.push(ChatChunk::delta(std::mem::take(&mut buf)));
        }
    }
    if !buf.is_empty() {
        out.push(ChatChunk::delta(buf));
    }
    if out.is_empty() {
        out.push(ChatChunk::delta(String::new()));
    }
    out.push(ChatChunk::final_chunk());
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_and_marks_done() {
        let c = echo_chunks("echo:a b");
        assert!(c.last().unwrap().done);
        let text: String = c.iter().map(|x| x.delta.as_str()).collect();
        assert_eq!(text, "echo:a b");
    }
}
