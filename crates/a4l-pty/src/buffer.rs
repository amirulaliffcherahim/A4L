use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct RollingBuffer {
    head_capacity: usize,
    tail_capacity: usize,
    head: Vec<u8>,
    tail: VecDeque<u8>,
    total_bytes: usize,
}

impl RollingBuffer {
    pub fn new(head_capacity: usize, tail_capacity: usize) -> Self {
        Self {
            head_capacity,
            tail_capacity,
            head: Vec::with_capacity(head_capacity),
            tail: VecDeque::with_capacity(tail_capacity),
            total_bytes: 0,
        }
    }

    pub fn feed(&mut self, chunk: &[u8]) {
        self.total_bytes += chunk.len();

        for &b in chunk {
            if self.head.len() < self.head_capacity {
                self.head.push(b);
            } else {
                if self.tail.len() >= self.tail_capacity {
                    self.tail.pop_front();
                }
                self.tail.push_back(b);
            }
        }
    }

    pub fn total_bytes(&self) -> usize {
        self.total_bytes
    }

    pub fn is_truncated(&self) -> bool {
        self.total_bytes > (self.head_capacity + self.tail.len())
    }

    pub fn elided_bytes(&self) -> usize {
        self.total_bytes
            .saturating_sub(self.head.len() + self.tail.len())
    }

    pub fn render_summary(&self) -> String {
        let head_str = String::from_utf8_lossy(&self.head);
        let tail_slice: Vec<u8> = self.tail.iter().copied().collect();
        let tail_str = String::from_utf8_lossy(&tail_slice);

        if self.is_truncated() {
            format!(
                "{}\n\n[... <elided {} bytes> ...]\n\n{}",
                head_str.trim_end(),
                self.elided_bytes(),
                tail_str.trim_start()
            )
        } else if !self.tail.is_empty() {
            format!("{}{}", head_str, tail_str)
        } else {
            head_str.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rolling_buffer_under_capacity() {
        let mut buf = RollingBuffer::new(50, 50);
        buf.feed(b"Hello World");
        assert_eq!(buf.total_bytes(), 11);
        assert!(!buf.is_truncated());
        assert_eq!(buf.render_summary(), "Hello World");
    }

    #[test]
    fn test_rolling_buffer_truncation_and_elision() {
        let mut buf = RollingBuffer::new(10, 10);
        // Feed 100 bytes: "0123456789...99"
        let data = b"HEAD012345MIDDLEREMOVEDSECTIONTAIL987654";
        buf.feed(data);

        assert!(buf.is_truncated());
        let elided = buf.elided_bytes();
        assert!(elided > 0);

        let summary = buf.render_summary();
        assert!(summary.contains("[... <elided "));
        assert!(summary.starts_with("HEAD012345"));
        assert!(summary.ends_with("TAIL987654"));
    }
}
