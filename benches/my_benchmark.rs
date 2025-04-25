use bladerf_adsb::{
    decode_bits, detect_preamble, find_adsb_preambles, iq_magnitude, process_adsb_frames,
};
use criterion::{Criterion, black_box, criterion_group, criterion_main}; // Import your function

fn bench_iq_magnitude(c: &mut Criterion) {
    // Simulate a typical I/Q buffer (e.g., 10k samples)
    let iq: Vec<i16> = (0..20_000).map(|x| (x % 256) as i16).collect();

    c.bench_function("iq_magnitude", |b| {
        b.iter(|| {
            let result = iq_magnitude(black_box(&iq));
            black_box(result);
        });
    });
}

fn bench_detect_preamble(c: &mut Criterion) {
    let signal: Vec<u16> = (0..1024)
        .map(|i| if i % 5 == 0 { 1000 } else { 10 })
        .collect();

    c.bench_function("detect_preamble", |b| {
        b.iter(|| {
            let mut count = 0;
            for i in 0..(signal.len() - 16) {
                if detect_preamble(black_box(&signal), i) {
                    count += 1;
                }
            }
            black_box(count);
        });
    });
}

fn bench_decode_bits(c: &mut Criterion) {
    // Fake signal with alternating strong/weak for bit extraction
    let mut signal = vec![0u16; 512];
    for i in 0..112 {
        let base = 16 + i * 2;
        signal[base] = 100;
        signal[base + 1] = 200;
    }

    c.bench_function("decode_bits", |b| {
        b.iter(|| {
            let result = decode_bits(black_box(&signal));
            black_box(result);
        });
    });
}

fn bench_process_adsb_frames(c: &mut Criterion) {
    let iq: Vec<i16> = (0..20_000).map(|x| (x % 256) as i16).collect();

    c.bench_function("process_adsb_frames", |b| {
        b.iter(|| {
            let decoded = process_adsb_frames(black_box(&iq));
            black_box(decoded);
        });
    });
}

fn bench_find_adsb_preambles(c: &mut Criterion) {
    // Simulate a signal with repeated preamble-like patterns
    let mut signal = vec![0u16; 4096];
    for i in (0..signal.len()).step_by(256) {
        if i + 16 < signal.len() {
            signal[i] = 1000;
            signal[i + 2] = 1000;
            signal[i + 7] = 1000;
            signal[i + 9] = 1000;
        }
    }

    c.bench_function("find_adsb_preambles", |b| {
        b.iter(|| {
            let result = find_adsb_preambles(black_box(&signal));
            black_box(result);
        });
    });
}

criterion_group!(
    benches,
    bench_iq_magnitude,
    bench_detect_preamble,
    bench_decode_bits,
    bench_decode_bits,
    bench_process_adsb_frames,
);

criterion_main!(benches);
