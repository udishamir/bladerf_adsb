use std::cmp::Ordering;

pub fn iq_magnitude(buffer: &[i16]) -> Vec<u16> {
    let mut magnitude_buffer = Vec::with_capacity(buffer.len() / 2);
    for chunk in buffer.chunks_exact(2) {
        let i = chunk[0] as f64;
        let q = chunk[1] as f64;
        let magnitude = (i * i + q * q).sqrt();
        let scaled_magnitude = (magnitude * 10.0).min(u16::MAX as f64) as u16;
        magnitude_buffer.push(scaled_magnitude);
    }
    magnitude_buffer
}

pub fn detect_preamble(buf: &[u16], i: usize) -> bool {
    if i + 16 > buf.len() {
        return false;
    }

    let mut low = 0u16;
    let mut high = u16::MAX;

    for (i2, &sample) in buf[i..i + 16].iter().enumerate() {
        match i2 {
            0 | 2 | 7 | 9 => high = high.min(sample),
            _ => low = low.max(sample),
        }
        if high <= low {
            return false;
        }
    }
    true
}

pub fn find_adsb_preambles(magnitude: &[u16]) -> Vec<usize> {
    (0..magnitude.len() - 16)
        .filter(|&i| detect_preamble(magnitude, i))
        .collect()
}

pub fn extract_messages(magnitude: &[u16], preambles: &[usize]) -> Vec<Vec<u16>> {
    const FRAME_LEN: usize = 512;
    preambles
        .iter()
        .filter_map(|&start| {
            let end = start + FRAME_LEN;
            if end <= magnitude.len() {
                Some(magnitude[start..end].to_vec())
            } else {
                None
            }
        })
        .collect()
}

pub fn decode_bits(magnitude: &[u16]) -> Option<String> {
    if magnitude.len() < 512 {
        return None;
    }
    let mut bits = String::with_capacity(112);
    for bit in 0..112 {
        let i = 16 + bit * 2;
        let s0 = magnitude[i];
        let s1 = magnitude[i + 1];
        match s0.cmp(&s1) {
            Ordering::Greater => bits.push('0'),
            Ordering::Less => bits.push('1'),
            Ordering::Equal => return None,
        }
    }
    Some(bits)
}

pub fn binary_to_bytes(bits: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    (0..bits.len())
        .step_by(8)
        .map(|i| u8::from_str_radix(&bits[i..i + 8], 2))
        .collect()
}

pub fn icao_bytes_to_hex(bytes: [u8; 3]) -> String {
    format!("{:02X}{:02X}{:02X}", bytes[0], bytes[1], bytes[2])
}

pub fn process_adsb_frames(iq: &[i16]) -> Vec<(String, Vec<u16>)> {
    let magnitude = iq_magnitude(iq);
    let preambles = find_adsb_preambles(&magnitude);
    let message_windows = extract_messages(&magnitude, &preambles);
    let mut decoded_messages = Vec::new();
    for window in message_windows {
        if let Some(bits) = decode_bits(&window) {
            decoded_messages.push((bits, window));
        }
    }
    decoded_messages
}
