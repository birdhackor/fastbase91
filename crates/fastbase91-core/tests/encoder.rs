mod reference;

use fastbase91_core::{encode_into, max_encoded_len, Encoder};

fn rust_encode(input: &[u8]) -> Vec<u8> {
    let mut output = vec![0; max_encoded_len(input.len()).expect("test length has a bound")];
    let written = encode_into(input, &mut output).expect("upper bound is sufficient");
    output.truncate(written);
    output
}

fn fixed_random(length: usize) -> Vec<u8> {
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    (0..length)
        .map(|_| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state as u8
        })
        .collect()
}

fn streaming_encode<'a>(chunks: impl IntoIterator<Item = &'a [u8]>) -> Vec<u8> {
    let mut encoder = Encoder::new();
    let mut encoded = Vec::new();
    for chunk in chunks {
        let mut output = vec![0; max_encoded_len(chunk.len()).expect("test length has a bound")];
        let written = encoder
            .update(chunk, &mut output)
            .expect("state-independent bound is sufficient");
        encoded.extend_from_slice(&output[..written]);
    }
    let (tail, tail_len) = encoder.finish();
    encoded.extend_from_slice(&tail[..tail_len]);
    encoded
}

#[test]
fn hello_matches_named_vector_and_c_reference() {
    let c_encoded = reference::encode(b"hello");
    assert_eq!(c_encoded, b"TPwJh>A");
    assert_eq!(rust_encode(b"hello"), c_encoded);

    #[cfg(feature = "alloc")]
    assert_eq!(fastbase91_core::encode(b"hello").unwrap(), c_encoded);
}

#[test]
fn strict_comparison_boundary_vectors_match_c_reference() {
    const VECTORS: &[(&str, &[u8], &[u8])] = &[
        ("value == 87", b"\x57\x00\x01", b"|AEA"),
        ("value == 88", b"\x58\x00\x01", b"}AEA"),
        ("value == 89", b"\x59\x00\x01", b"~AIA"),
        ("queue == 89", b"\x00\x00\x00\x00\x00\xb3", b"AAAABt~"),
        ("queue == 90", b"\x00\x00\x00\x00\x00\xb5", b"AAAABt\""),
        ("queue == 91", b"\x00\x00\x00\x00\x00\xb7", b"AAAABtAB"),
    ];

    for &(name, input, expected) in VECTORS {
        let c_encoded = reference::encode(input);
        assert_eq!(c_encoded, expected, "C oracle changed for {name}");
        assert_eq!(rust_encode(input), c_encoded, "Rust mismatch for {name}");
    }
}

#[test]
fn differential_257_lengths_times_four_contents() {
    let mut cases = 0;
    for length in 0..=256 {
        let inputs = [
            vec![0; length],
            vec![0xff; length],
            (0..length).map(|index| index as u8).collect(),
            fixed_random(length),
        ];
        for input in inputs {
            assert_eq!(
                rust_encode(&input),
                reference::encode(&input),
                "C reference mismatch at length {length}"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 257 * 4);
    eprintln!("C reference differential cases: {cases} (257 lengths x 4 contents)");
}

#[test]
fn max_length_matches_wide_integer_formula_at_boundaries() {
    let samples = [
        0,
        1,
        12,
        13,
        14,
        255,
        256,
        isize::MAX as usize,
        usize::MAX - 1,
        usize::MAX,
    ];
    for input_len in samples {
        let wide = 2_u128 * (8_u128 * input_len as u128 + 13).div_ceil(13);
        let expected = usize::try_from(wide).ok();
        assert_eq!(
            max_encoded_len(input_len),
            expected,
            "input_len={input_len}"
        );
    }

    let mut low = 0_usize;
    let mut high = usize::MAX;
    while low < high {
        let middle = low + (high - low) / 2 + 1;
        if max_encoded_len(middle).is_some() {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    assert!(max_encoded_len(low).is_some());
    assert!(max_encoded_len(low + 1).is_none());

    let alloc_boundary = max_encoded_len(isize::MAX as usize).unwrap();
    assert!(alloc_boundary > isize::MAX as usize);
    eprintln!(
        "largest input with usize math bound: {low}; max_encoded_len(isize::MAX)={alloc_boundary}"
    );
}

#[test]
fn bound_covers_nonempty_states_update_and_finish() {
    let data = fixed_random(160);
    for prefix_len in 0..=64 {
        for chunk_len in 0..=64 {
            let mut encoder = Encoder::new();
            let mut prefix_output = vec![0; max_encoded_len(prefix_len).unwrap()];
            encoder
                .update(&data[..prefix_len], &mut prefix_output)
                .unwrap();

            let chunk = &data[64..64 + chunk_len];
            let mut measuring = encoder.clone();
            let required = match measuring.update(chunk, &mut []) {
                Ok(written) => {
                    assert_eq!(written, 0);
                    0
                }
                Err(error) => error.required(),
            };
            let mut claimed_capacity = vec![0; required];
            let written = encoder.update(chunk, &mut claimed_capacity).unwrap();
            assert!(written <= required);

            let bound = max_encoded_len(chunk_len).unwrap();
            let (_, tail_len) = encoder.finish();
            assert!(written + tail_len <= bound);
        }
    }
}

#[test]
fn output_too_small_is_transactional_and_retryable() {
    let prefix = b"ab";
    let chunk = b"cdefghijk";
    let mut encoder = Encoder::new();
    let mut prefix_output = [0_u8; 8];
    let prefix_written = encoder.update(prefix, &mut prefix_output).unwrap();

    let before = encoder.clone();
    let mut measuring = before.clone();
    let required = measuring.update(chunk, &mut []).unwrap_err().required();
    assert!(required > 0);

    let mut too_small = vec![0xa5; required - 1];
    let untouched = too_small.clone();
    let error = encoder.update(chunk, &mut too_small).unwrap_err();
    assert_eq!(error.required(), required);
    assert_eq!(encoder, before);
    assert_eq!(too_small, untouched);

    let mut retry = vec![0; required];
    let chunk_written = encoder.update(chunk, &mut retry).unwrap();
    assert!(chunk_written <= required);
    let after_chunk = encoder.clone();
    assert_eq!(encoder.update(b"", &mut []), Ok(0));
    assert_eq!(encoder, after_chunk);

    let (tail, tail_len) = encoder.finish();
    let mut encoded = prefix_output[..prefix_written].to_vec();
    encoded.extend_from_slice(&retry[..chunk_written]);
    encoded.extend_from_slice(&tail[..tail_len]);
    assert_eq!(encoded, reference::encode(b"abcdefghijk"));
}

#[test]
fn update_capacity_success_is_content_independent() {
    let inputs = [[0_u8; 5], [0xff_u8; 5]];

    for input in inputs {
        let mut encoder = Encoder::new();
        let mut too_small = [0xa5_u8; 5];
        let untouched = too_small;
        let error = encoder.update(&input, &mut too_small).unwrap_err();
        assert_eq!(error.required(), 6);
        assert_eq!(encoder, Encoder::new());
        assert_eq!(too_small, untouched);

        let mut sufficient = [0_u8; 6];
        let written = encoder.update(&input, &mut sufficient).unwrap();
        assert!(written <= 6);
    }
}

#[test]
fn encode_into_upper_bound_capacity_and_one_less_contract() {
    let expected = reference::encode(b"hello");
    let required = 2 * ((8 * b"hello".len()) / 13) + 2;
    let mut sufficient = vec![0; required];
    let written = encode_into(b"hello", &mut sufficient).unwrap();
    assert_eq!(written, expected.len());
    assert_eq!(&sufficient[..written], expected);

    let mut too_small = vec![0x5a; required - 1];
    let untouched = too_small.clone();
    let error = encode_into(b"hello", &mut too_small).unwrap_err();
    assert_eq!(error.required(), required);
    assert_eq!(too_small, untouched);

    assert_eq!(encode_into(b"", &mut []), Ok(0));
}

#[test]
fn c_reference_large_encode_and_decode_do_not_deadlock() {
    let input = vec![0_u8; 1024 * 1024];
    let rust_encoded = rust_encode(&input);
    let c_encoded = reference::encode(&input);
    assert_eq!(c_encoded, rust_encoded);
    assert_eq!(reference::decode(&c_encoded), input);
    eprintln!(
        "C oracle 1 MiB round trip: {} input bytes, {} encoded bytes",
        input.len(),
        c_encoded.len()
    );
}

#[test]
fn every_partition_of_a_fixed_message_matches_one_shot_and_c() {
    let input = b"chunk-test";
    let expected = reference::encode(input);
    let boundary_count = input.len() - 1;
    let partition_count = 1_usize << boundary_count;

    for mask in 0..partition_count {
        let mut chunks = Vec::new();
        let mut start = 0;
        for boundary in 0..boundary_count {
            if mask & (1 << boundary) != 0 {
                chunks.push(&input[start..=boundary]);
                chunks.push(&input[0..0]);
                start = boundary + 1;
            }
        }
        chunks.push(&input[start..]);
        assert_eq!(streaming_encode(chunks), expected, "partition mask {mask}");
    }
    assert_eq!(rust_encode(input), expected);
    eprintln!("streaming partitions checked: {partition_count}");
}

#[test]
fn all_two_chunk_splits_match_c_reference() {
    for length in 0..=64 {
        let input = fixed_random(length);
        let expected = reference::encode(&input);
        for split in 0..=length {
            assert_eq!(
                streaming_encode([&input[..split], &input[split..]]),
                expected,
                "length={length}, split={split}"
            );
        }
    }
}
