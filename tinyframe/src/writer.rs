//! Simple framing “transcoder”: turn a raw byte stream into a sequence of
//! [`TinyFrame`] packets, packed back-to-back into a destination buffer.
//
// NB: we *return the number of source bytes consumed*, not how many were
// written, because a caller that hits the destination-full condition will
// typically resume later with the *unread* tail of the source.

use crate::packets::{FrameError, TinyFrame};

/// Transcodes `source` into `destination`, packetising it as `TinyFrame`s.
///
/// *Returns* the number of bytes **read** from `source`.  
/// If `destination` fills before `source` is exhausted the function exits early;
/// the caller can invoke it again with the remaining source tail.
pub fn transcode_to_frames(source: &[u8], destination: &mut [u8]) -> usize {
    let mut src_read = 0; // how many bytes we have consumed from `source`
    let mut dst_filled = 0; // how many bytes we have produced into `destination`

    while src_read < source.len() {
        // Create a frame from the *remaining* chunk of the source.
        let (frame, used) = TinyFrame::create_from_slice(&source[src_read..]);

        // Try to encode into the *remaining* slice of the destination.
        match frame.encode_into_slice(&mut destination[dst_filled..]) {
            Ok(written) => {
                src_read += used;
                dst_filled += written;
            }
            // Most likely `UnexpectedEOF` – out of space in destination.
            Err(FrameError::UnexpectedEOF) => break,
            // Any other error means the frame itself is logically bad, but that
            // should be impossible because `create_from_slice` always produces a
            // self-consistent frame. Bubble the error up as a panic for debug
            // builds; real code might return a `Result`.
            Err(e) => panic!("transcode_stream: unexpected error: {e:?}"),
        }
    }

    src_read
}

#[cfg(test)]
mod transcode_tests {
    use super::*;
    use crate::packets::{MAX_PACKET_LEN, SOF};

    /// Helper that makes a deterministic “payload”
    fn payload(n: usize) -> Vec<u8> {
        (0..n).map(|i| i as u8).collect()
    }

    #[test]
    fn roundtrip_full_transcode() {
        let src = payload(300);

        // Destination big enough for worst-case: every 128-byte chunk becomes +5.
        let mut dst = [0u8; (300 / MAX_PACKET_LEN + 2) * (MAX_PACKET_LEN + 5)];

        let read = transcode_to_frames(&src, &mut dst);
        assert_eq!(read, src.len());

        // Decode packets back out and rebuild the original stream.
        let mut recovered = Vec::<u8>::new();
        let mut cursor = 0;
        while cursor < dst.len() && dst[cursor] == SOF {
            let need = dst[cursor + 1] as usize;
            let (frame, consumed) =
                crate::packets::TinyFrame::decode_from_slice(&dst[cursor..cursor + need]).unwrap();
            cursor += consumed;
            recovered.extend_from_slice(frame.data());
        }

        assert_eq!(recovered[..src.len()], src[..]);
    }

    #[test]
    fn destination_full_consumes_only_first_frame() {
        let src = payload(300);
        // Buffer that fits *exactly one* 128-byte frame (+5 overhead).
        let mut dst = [0u8; MAX_PACKET_LEN + 5];

        let read = transcode_to_frames(&src, &mut dst);
        assert_eq!(read, MAX_PACKET_LEN); // only first 128 bytes consumed
        assert_eq!(dst[0], SOF); // packet really present
        // Source tail (bytes 128..) remains unconsumed.
    }
}
