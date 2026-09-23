#![no_main]

use fastbase91_core::{encode, encode_into, max_encoded_len, Encoder};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    let capacity = max_encoded_len(input.len()).expect("a real slice has a usize length bound");

    let one_shot = encode(input).expect("fuzz-sized output allocation succeeds");
    assert!(one_shot.len() <= capacity);

    let mut slice_output = vec![0; capacity];
    let slice_len = encode_into(input, &mut slice_output).expect("capacity bound is sufficient");
    assert_eq!(&slice_output[..slice_len], one_shot);

    let chunk_size = input.first().map_or(1, |byte| usize::from(*byte) % 31 + 1);
    let mut encoder = Encoder::new();
    let mut streaming = Vec::new();
    for chunk in input.chunks(chunk_size) {
        let mut output = vec![0; max_encoded_len(chunk.len()).unwrap()];
        let written = encoder.update(chunk, &mut output).unwrap();
        streaming.extend_from_slice(&output[..written]);
    }
    let (tail, tail_len) = encoder.finish();
    streaming.extend_from_slice(&tail[..tail_len]);
    assert_eq!(streaming, one_shot);
});
