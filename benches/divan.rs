use bladerf_adsb::{
    decode_bits, extract_messages, find_adsb_preambles, iq_magnitude, process_adsb_frames,
};
use divan::{Bencher, bench};

fn main() {
    divan::main(); // must be in main
}

#[bench]
fn bench_iq_magnitude(b: Bencher) {
    let iq: Vec<i16> = (0..20_000).map(|x| (x % 256) as i16).collect();
    b.bench(|| iq_magnitude(&iq));
}

#[bench]
fn bench_decode_bits(b: Bencher) {
    let mut signal = vec![0u16; 512];
    for i in 0..112 {
        let idx = 16 + i * 2;
        signal[idx] = 100;
        signal[idx + 1] = 200;
    }

    b.bench(|| decode_bits(&signal));
}

#[bench]
fn bench_find_adsb_preambles(b: Bencher) {
    let mut signal = vec![0u16; 4096];
    for i in (0..signal.len()).step_by(256) {
        if i + 16 < signal.len() {
            signal[i] = 1000;
            signal[i + 2] = 1000;
            signal[i + 7] = 1000;
            signal[i + 9] = 1000;
        }
    }

    b.bench(|| find_adsb_preambles(&signal));
}

#[bench]
fn bench_process_adsb_frames(b: Bencher) {
    let iq: Vec<i16> = (0..20_000).map(|x| (x % 256) as i16).collect();
    b.bench(|| process_adsb_frames(&iq));
}

