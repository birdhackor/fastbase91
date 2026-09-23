//! Fixed-protocol encode/decode throughput benchmark for the core APIs.
//!
//! Protocol:
//! - Input sizes are exactly 1 KiB, 64 KiB, and 1 MiB (1,024, 65,536, and
//!   1,048,576 bytes).
//! - Input bytes come from a deterministic 64-bit LCG with seed
//!   `0x4d595df4d0f33173`.
//! - Streaming operations use fixed 4 KiB chunks.
//! - Every case has 3 warmup operations followed by 9 measured operations;
//!   the median elapsed time is reported.
//! - Throughput is MiB/s based on the bytes consumed by that operation. For
//!   decode cases this is the encoded input length, while the displayed case
//!   size remains the corresponding original input size.
//! - Output buffers for slice and streaming cases are allocated before timing;
//!   one-shot cases include their API allocation, as required by their API.
//!
//! The generated input is fixed so results are reproducible across runs and
//! changes in content do not accidentally become a benchmark variable. The
//! pseudo-random data also avoids making the baseline specific to an unusually
//! compressible all-zero input; basE91 throughput is expected to be content
//! independent.

use std::hint::black_box;
use std::time::{Duration, Instant};

use fastbase91_core::{
    decode, decode_into, encode, encode_into, max_decoded_len, max_encoded_len, DecodeOptions,
    Decoder, Encoder,
};

const INPUT_SIZES: &[usize] = &[1 << 10, 1 << 16, 1 << 20];
const STREAM_CHUNK_BYTES: usize = 4 << 10;
const WARMUP_OPERATIONS: usize = 3;
const MEASURED_OPERATIONS: usize = 9;
const LCG_SEED: u64 = 0x4d59_5df4_d0f3_3173;
const MIB: f64 = 1024.0 * 1024.0;

fn main() {
    println!("fastbase91 throughput benchmark");
    println!(
        "protocol=sizes:1024,65536,1048576B seed:0x{LCG_SEED:016x} chunk:{STREAM_CHUNK_BYTES}B warmup:{WARMUP_OPERATIONS} samples:{MEASURED_OPERATIONS} statistic:median metric:MiB/s"
    );

    for &input_size in INPUT_SIZES {
        run_size_case(input_size);
    }
}

fn run_size_case(input_size: usize) {
    let input = make_input(input_size);
    let encoded = encode(&input).expect("benchmark input must encode");
    assert_eq!(
        decode(&encoded, DecodeOptions::new()).expect("encoded input must decode"),
        input
    );

    let mut encoded_output =
        vec![0_u8; max_encoded_len(input.len()).expect("encoded benchmark input length must fit")];
    let mut decoded_output = vec![
        0_u8;
        max_decoded_len(encoded.len())
            .expect("decoded benchmark input length must fit")
    ];

    let elapsed = measure(|| bench_encode_one_shot(&input));
    print_result("encode-one-shot", input_size, input.len(), elapsed);

    let elapsed = measure(|| bench_encode_slice(&input, &mut encoded_output));
    print_result("encode-slice", input_size, input.len(), elapsed);

    let elapsed = measure(|| bench_encode_streaming(&input, &mut encoded_output));
    print_result("encode-streaming", input_size, input.len(), elapsed);

    let elapsed = measure(|| bench_decode_one_shot(&encoded));
    print_result("decode-one-shot", input_size, encoded.len(), elapsed);

    let elapsed = measure(|| bench_decode_slice(&encoded, &mut decoded_output));
    print_result("decode-slice", input_size, encoded.len(), elapsed);

    let elapsed = measure(|| bench_decode_streaming(&encoded, &mut decoded_output));
    print_result("decode-streaming", input_size, encoded.len(), elapsed);
}

fn make_input(length: usize) -> Vec<u8> {
    let mut state = LCG_SEED;
    let mut input = vec![0_u8; length];
    for byte in &mut input {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        *byte = (state >> 32) as u8;
    }
    input
}

fn measure<F>(mut operation: F) -> Duration
where
    F: FnMut() -> usize,
{
    for _ in 0..WARMUP_OPERATIONS {
        black_box(operation());
    }

    let mut samples = [Duration::ZERO; MEASURED_OPERATIONS];
    for sample in &mut samples {
        let started = Instant::now();
        black_box(operation());
        *sample = started.elapsed();
    }
    samples.sort_unstable();
    samples[MEASURED_OPERATIONS / 2]
}

fn print_result(case: &str, input_size: usize, input_bytes: usize, elapsed: Duration) {
    let elapsed_nanos = elapsed.as_nanos();
    let seconds = (elapsed_nanos.max(1) as f64) / 1_000_000_000.0;
    let throughput = input_bytes as f64 / MIB / seconds;
    println!(
        "case={case} size={input_size}B input_bytes={input_bytes} elapsed_ns={elapsed_nanos} throughput_mib_s={throughput:.2}"
    );
}

fn bench_encode_one_shot(input: &[u8]) -> usize {
    let output = encode(black_box(input)).expect("one-shot encoding must succeed");
    let output = black_box(output);
    black_box(output.len())
}

fn bench_encode_slice(input: &[u8], output: &mut [u8]) -> usize {
    let written = encode_into(black_box(input), output).expect("slice encoding must succeed");
    black_box(&output[..written]);
    black_box(written)
}

fn bench_encode_streaming(input: &[u8], output: &mut [u8]) -> usize {
    let mut encoder = Encoder::new();
    let mut written = 0;
    for chunk in input.chunks(STREAM_CHUNK_BYTES) {
        written += encoder
            .update(black_box(chunk), &mut output[written..])
            .expect("streaming encoding must succeed");
    }
    let (tail, tail_len) = encoder.finish();
    output[written..written + tail_len].copy_from_slice(&tail[..tail_len]);
    written += tail_len;
    black_box(&output[..written]);
    black_box(written)
}

fn bench_decode_one_shot(encoded: &[u8]) -> usize {
    let output =
        decode(black_box(encoded), DecodeOptions::new()).expect("one-shot decoding must succeed");
    let output = black_box(output);
    black_box(output.len())
}

fn bench_decode_slice(encoded: &[u8], output: &mut [u8]) -> usize {
    let written = decode_into(black_box(encoded), output, DecodeOptions::new())
        .expect("slice decoding must succeed");
    black_box(&output[..written]);
    black_box(written)
}

fn bench_decode_streaming(encoded: &[u8], output: &mut [u8]) -> usize {
    let mut decoder = Decoder::new(DecodeOptions::new());
    let mut written = 0;
    for chunk in encoded.chunks(STREAM_CHUNK_BYTES) {
        written += decoder
            .update(black_box(chunk), &mut output[written..])
            .expect("streaming decoding must succeed");
    }
    if let Some(byte) = decoder.finish().expect("streaming decoding must finish") {
        output[written] = byte;
        written += 1;
    }
    black_box(&output[..written]);
    black_box(written)
}
