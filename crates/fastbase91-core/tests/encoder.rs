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

            let bound = max_encoded_len(chunk_len).unwrap();
            let mut chunk_output = vec![0; bound];
            let written = encoder
                .update(&data[64..64 + chunk_len], &mut chunk_output)
                .unwrap();
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

    let before = encoder;
    let mut probe = [0_u8; 32];
    let mut measuring = before;
    let required = measuring.update(chunk, &mut probe).unwrap();
    assert!(required > 0);

    let mut too_small = vec![0xa5; required - 1];
    let untouched = too_small.clone();
    let error = encoder.update(chunk, &mut too_small).unwrap_err();
    assert_eq!(error.required(), required);
    assert_eq!(encoder, before);
    assert_eq!(too_small, untouched);

    let mut retry = vec![0; required];
    assert_eq!(encoder.update(chunk, &mut retry), Ok(required));
    let after_chunk = encoder;
    assert_eq!(encoder.update(b"", &mut []), Ok(0));
    assert_eq!(encoder, after_chunk);

    let (tail, tail_len) = encoder.finish();
    let mut encoded = prefix_output[..prefix_written].to_vec();
    encoded.extend_from_slice(&retry);
    encoded.extend_from_slice(&tail[..tail_len]);
    assert_eq!(encoded, reference::encode(b"abcdefghijk"));
}

#[test]
fn encode_into_exact_capacity_and_one_less_contract() {
    let expected = reference::encode(b"hello");
    let mut exact = vec![0; expected.len()];
    assert_eq!(encode_into(b"hello", &mut exact), Ok(expected.len()));
    assert_eq!(exact, expected);

    let mut too_small = vec![0x5a; expected.len() - 1];
    let untouched = too_small.clone();
    let error = encode_into(b"hello", &mut too_small).unwrap_err();
    assert_eq!(error.required(), expected.len());
    assert_eq!(too_small, untouched);

    assert_eq!(encode_into(b"", &mut []), Ok(0));
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
