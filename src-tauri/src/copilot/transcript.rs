use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptChunk {
    pub session_id: i64,
    pub start_ms:   u64,
    pub end_ms:     u64,
    pub text:       String,
    pub is_final:   bool,
}

pub struct TranscriptBuffer {
    chunks:    VecDeque<TranscriptChunk>,
    max_age_s: u64,
    /// index into `chunks` marking where "since last trigger" begins
    trigger_mark: usize,
}

impl TranscriptBuffer {
    pub fn new(max_age_s: u64) -> Self {
        Self { chunks: VecDeque::new(), max_age_s, trigger_mark: 0 }
    }

    pub fn append(&mut self, chunk: TranscriptChunk) {
        let cutoff = chunk.end_ms.saturating_sub(self.max_age_s * 1000);
        self.chunks.push_back(chunk);
        // Drop chunks older than max_age
        while let Some(front) = self.chunks.front() {
            if front.end_ms < cutoff {
                self.chunks.pop_front();
                self.trigger_mark = self.trigger_mark.saturating_sub(1);
            } else {
                break;
            }
        }
    }

    /// Return joined text of all final chunks in the last `last_seconds` window.
    pub fn window(&self, last_seconds: u64) -> String {
        let Some(latest_end) = self.chunks.back().map(|c| c.end_ms) else {
            return String::new();
        };
        let cutoff = latest_end.saturating_sub(last_seconds * 1000);
        self.chunks
            .iter()
            .filter(|c| c.is_final && c.end_ms > cutoff)
            .map(|c| c.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Chunks appended since last `mark_triggered()`.
    pub fn since_last_trigger(&self) -> Vec<&TranscriptChunk> {
        self.chunks.iter().skip(self.trigger_mark).collect()
    }

    pub fn mark_triggered(&mut self) {
        self.trigger_mark = self.chunks.len();
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.trigger_mark = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(start_ms: u64, end_ms: u64, text: &str) -> TranscriptChunk {
        TranscriptChunk {
            session_id: 1, start_ms, end_ms, text: text.into(), is_final: true,
        }
    }

    #[test]
    fn empty_window_returns_empty_string() {
        let b = TranscriptBuffer::new(60);
        assert_eq!(b.window(60), "");
    }

    #[test]
    fn window_joins_recent_chunks() {
        let mut b = TranscriptBuffer::new(60);
        b.append(chunk(0, 1000, "hello"));
        b.append(chunk(1000, 2000, "world"));
        assert_eq!(b.window(60), "hello world");
    }

    #[test]
    fn window_excludes_chunks_older_than_window() {
        let mut b = TranscriptBuffer::new(600);
        b.append(chunk(0, 1000, "old"));
        b.append(chunk(60_000, 61_000, "new"));
        // current time = end of latest chunk = 61000ms; window 30s = last 30000ms
        assert_eq!(b.window(30), "new");
    }

    #[test]
    fn max_age_purges_ancient_chunks() {
        let mut b = TranscriptBuffer::new(5); // 5s max age
        b.append(chunk(0, 1000, "old"));
        b.append(chunk(10_000, 11_000, "new"));
        // After 2nd append, latest = 11s, max_age = 5s → purge anything older than 6s
        assert_eq!(b.chunks.len(), 1);
        assert_eq!(b.chunks[0].text, "new");
    }

    #[test]
    fn since_last_trigger_returns_new_chunks_only() {
        let mut b = TranscriptBuffer::new(60);
        b.append(chunk(0, 1000, "a"));
        b.mark_triggered();
        b.append(chunk(1000, 2000, "b"));
        b.append(chunk(2000, 3000, "c"));
        let new = b.since_last_trigger();
        assert_eq!(new.len(), 2);
        assert_eq!(new[0].text, "b");
        assert_eq!(new[1].text, "c");
    }

    #[test]
    fn clear_resets_state() {
        let mut b = TranscriptBuffer::new(60);
        b.append(chunk(0, 1000, "a"));
        b.clear();
        assert_eq!(b.window(60), "");
        assert_eq!(b.since_last_trigger().len(), 0);
    }
}
