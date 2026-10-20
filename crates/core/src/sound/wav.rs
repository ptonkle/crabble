//! WAV files: RIFF, 16-bit mono PCM (Ape Race's writer), and a reader for
//! the tests and the listening script.

/// A complete WAV file holding `pcm` at `rate` samples a second.
pub fn file(pcm: &[i16], rate: u32) -> Vec<u8> {
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes()); // PCM
    out.extend_from_slice(&1u16.to_le_bytes()); // mono
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out
}

/// The sample rate and samples of a file `file` wrote, or `None` for
/// anything else.
pub fn parse(bytes: &[u8]) -> Option<(u32, Vec<i16>)> {
    let word = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..16)? != b"WAVEfmt " || bytes.get(36..40)? != b"data" {
        return None;
    }
    let rate = word(24)?;
    let len = word(40)? as usize;
    let data = bytes.get(44..44 + len)?;
    let pcm = data
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect();
    Some((rate, pcm))
}
