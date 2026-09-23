#![no_main]

use fastbase91_core::{decode, encode, max_decoded_len, DecodeOptions, Decoder};
use libfuzzer_sys::fuzz_target;

fn strict_options() -> DecodeOptions {
    let mut options = DecodeOptions::new();
    options.reject_non_alphabet = true;
    options
}

fn streaming_decode(input: &[u8], chunk_size: usize) -> Vec<u8> {
    let mut decoder = Decoder::new(DecodeOptions::new());
    let mut decoded = Vec::new();
    for chunk in input.chunks(chunk_size) {
        let mut output = vec![0; max_decoded_len(chunk.len()).unwrap()];
        let written = decoder.update(chunk, &mut output).unwrap();
        decoded.extend_from_slice(&output[..written]);
    }
    if let Some(tail) = decoder.finish().unwrap() {
        decoded.push(tail);
    }
    decoded
}

fuzz_target!(|input: &[u8]| {
    let chunk_size = input.first().map_or(1, |byte| usize::from(*byte) % 31 + 1);

    let arbitrary = decode(input, DecodeOptions::new()).expect("fuzz-sized allocation succeeds");
    assert_eq!(streaming_decode(input, chunk_size), arbitrary);
    let _strict_result = decode(input, strict_options());

    let encoded = encode(input).expect("fuzz-sized output allocation succeeds");
    assert_eq!(
        decode(&encoded, DecodeOptions::new()).unwrap(),
        input,
        "lenient round trip"
    );
    assert_eq!(
        decode(&encoded, strict_options()).unwrap(),
        input,
        "strict round trip"
    );
});
