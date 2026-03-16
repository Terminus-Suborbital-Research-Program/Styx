use crate::frame::Frame;

/// Yields successive `Frame`s with contigous payloads.
pub struct FrameIter<'a> {
    src: &'a [u8],
    seq: u8,
}

impl<'a> FrameIter<'a> {
    pub fn new(src: &'a [u8], seq: u8) -> Self {
        Self { src, seq }
    }

    pub fn first(src: &'a [u8]) -> Self {
        Self::new(src, 0)
    }
}

impl<'a> Iterator for FrameIter<'a> {
    type Item = Frame;

    fn next(&mut self) -> Option<Self::Item> {
        if !self.src.is_empty() {
            let (frame, used) = Frame::new(self.src, self.seq);
            self.seq = self.seq.wrapping_add(1);
            self.src = &self.src[used..];
            Some(frame)
        } else {
            None
        }
    }
}
