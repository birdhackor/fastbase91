mod reference;

use fastbase91_core::{
    decode_into, encode_into, max_decoded_len, max_encoded_len, DecodeError, DecodeOptions, Decoder,
};

fn strict_options() -> DecodeOptions {
    let mut options = DecodeOptions::new();
    options.reject_non_alphabet = true;
    options
}

fn rust_encode(input: &[u8]) -> Vec<u8> {
    let mut output = vec![0; max_encoded_len(input.len()).expect("test length has a bound")];
    let written = encode_into(input, &mut output).expect("upper bound is sufficient");
    output.truncate(written);
    output
}

fn rust_decode(input: &[u8], options: DecodeOptions) -> Result<Vec<u8>, DecodeError> {
    let mut output = vec![0; max_decoded_len(input.len()).expect("test length has a bound")];
    let written = decode_into(input, &mut output, options)?;
    output.truncate(written);
    Ok(output)
}

fn streaming_decode<'a>(
    chunks: impl IntoIterator<Item = &'a [u8]>,
    options: DecodeOptions,
) -> Result<Vec<u8>, DecodeError> {
    let mut decoder = Decoder::new(options);
    let mut decoded = Vec::new();
    for chunk in chunks {
        let mut output = vec![0; max_decoded_len(chunk.len()).unwrap()];
        let written = decoder.update(chunk, &mut output)?;
        decoded.extend_from_slice(&output[..written]);
    }
    if let Some(tail) = decoder.finish()? {
        decoded.push(tail);
    }
    Ok(decoded)
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

fn output_required(error: DecodeError) -> usize {
    match error {
        DecodeError::OutputTooSmall(error) => error.required(),
        other => panic!("expected OutputTooSmall, got {other:?}"),
    }
}

#[test]
fn hello_matches_named_vector_and_c_reference() {
    let c_encoded = reference::encode(b"hello");
    assert_eq!(c_encoded, b"TPwJh>A");
    assert_eq!(rust_encode(b"hello"), c_encoded);

    let c_decoded = reference::decode(b"TPwJh>A");
    assert_eq!(c_decoded, b"hello");
    assert_eq!(
        rust_decode(b"TPwJh>A", DecodeOptions::new()).unwrap(),
        c_decoded
    );
}

#[test]
fn round_trip_varied_inputs_in_lenient_and_strict_modes() {
    let mut cases = vec![Vec::new()];
    for length in 1..=256 {
        cases.push((0..length).map(|index| index as u8).collect());
        cases.push(fixed_random(length));
        cases.push(vec![0; length]);
        cases.push(vec![0xff; length]);
    }

    for input in &cases {
        let encoded = rust_encode(input);
        assert_eq!(
            rust_decode(&encoded, DecodeOptions::new()).unwrap(),
            *input,
            "lenient round trip failed at input length {}",
            input.len()
        );
        assert_eq!(
            rust_decode(&encoded, strict_options()).unwrap(),
            *input,
            "strict round trip failed at input length {}",
            input.len()
        );
    }
    eprintln!("decoder round-trip cases per mode: {}", cases.len());
}

#[test]
fn lenient_differential_257_lengths_times_four_contents() {
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
                rust_decode(&input, DecodeOptions::new()).unwrap(),
                reference::decode(&input),
                "C reference mismatch at length {length}"
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 257 * 4);
    eprintln!("C reference decode cases: {cases} (257 lengths x 4 contents)");
}

#[test]
fn named_finish_states_and_comparison_boundary_match_c_reference() {
    const VECTORS: &[(&str, &[u8], &[u8])] = &[
        ("no pending, empty", b"", b""),
        ("no pending after pair", b"AA", b"\x00"),
        ("pending at nbits 0", b"B", b"\x01"),
        (
            "pending at nbits 1",
            b"AAAA~AB",
            b"\x00\x00\x00\x90\x05\x02",
        ),
        ("pending at nbits 2", b"~A~AB", b"\x59\x20\x0b\x04"),
        ("pending at nbits 3", b"~AAAB", b"\x59\x00\x00\x08"),
        ("pending at nbits 4", b"AAAAB", b"\x00\x00\x00\x10"),
        ("pending at nbits 5", b"~AB", b"\x59\x20"),
        ("pending at nbits 6", b"AAB", b"\x00\x40"),
        (
            "pending at nbits 7",
            b"AAAAAA~AB",
            b"\x00\x00\x00\x00\x00\x64\x81",
        ),
        ("combined 87 uses 14 bits", b"|ABB", b"\x57\x00\x17"),
        ("combined 88 uses 14 bits", b"}ABB", b"\x58\x00\x17"),
        ("combined 89 uses 13 bits", b"~ABB", b"\x59\x80\x0b"),
    ];

    for &(name, input, expected) in VECTORS {
        let c_decoded = reference::decode(input);
        assert_eq!(c_decoded, expected, "C oracle changed for {name}");
        assert_eq!(
            rust_decode(input, DecodeOptions::new()).unwrap(),
            c_decoded,
            "Rust mismatch for {name}"
        );
    }
}

#[test]
fn strict_and_lenient_named_vectors() {
    let spaced = b"TPw Jh>A";
    assert_eq!(reference::decode(spaced), b"hello");
    assert_eq!(rust_decode(spaced, DecodeOptions::new()).unwrap(), b"hello");
    assert_eq!(
        rust_decode(spaced, strict_options()),
        Err(DecodeError::InvalidByte {
            byte: b' ',
            offset: 3,
        })
    );

    for &(byte, offset) in &[(0_u8, 1_usize), (0x80, 2), (0xff, 3)] {
        let mut input = b"AAAB".to_vec();
        input[offset] = byte;
        assert_eq!(
            rust_decode(&input, strict_options()),
            Err(DecodeError::InvalidByte { byte, offset })
        );
        assert_eq!(
            rust_decode(&input, DecodeOptions::new()).unwrap(),
            reference::decode(&input)
        );
    }
}

#[test]
fn strict_offset_is_chunk_local_and_state_is_transactional() {
    let mut decoder = Decoder::new(strict_options());
    let mut decoded = Vec::new();

    let mut prefix_output = vec![0; max_decoded_len(3).unwrap()];
    let prefix_written = decoder.update(b"TPw", &mut prefix_output).unwrap();
    decoded.extend_from_slice(&prefix_output[..prefix_written]);

    let before = decoder.clone();
    let mut invalid_output = vec![0xa5; max_decoded_len(5).unwrap()];
    // The offset is the byte's 0-based index within this `update` input (the
    // space is at index 1 of "J h>A"), not a cumulative stream position.
    assert_eq!(
        decoder.update(b"J h>A", &mut invalid_output),
        Err(DecodeError::InvalidByte {
            byte: b' ',
            offset: 1,
        })
    );
    // State is unchanged after a strict error; `invalid_output` may already
    // hold bytes decoded before the invalid byte, so its contents are
    // intentionally not asserted (the strict contract does not preserve output).
    assert_eq!(decoder, before);

    let mut retry_output = vec![0; max_decoded_len(4).unwrap()];
    let retry_written = decoder.update(b"Jh>A", &mut retry_output).unwrap();
    decoded.extend_from_slice(&retry_output[..retry_written]);
    if let Some(tail) = decoder.finish().unwrap() {
        decoded.push(tail);
    }
    assert_eq!(decoded, b"hello");
}

#[test]
fn every_partition_matches_one_shot_in_both_modes() {
    let input = b"TPwJh>A";
    let expected = rust_decode(input, DecodeOptions::new()).unwrap();
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
        assert_eq!(
            streaming_decode(chunks.clone(), DecodeOptions::new()).unwrap(),
            expected,
            "lenient partition mask {mask}"
        );
        assert_eq!(
            streaming_decode(chunks, strict_options()).unwrap(),
            expected,
            "strict partition mask {mask}"
        );
    }
    assert_eq!(expected, reference::decode(input));
    eprintln!("decoder streaming partitions checked per mode: {partition_count}");
}

#[test]
fn output_too_small_is_transactional_retryable_and_precedes_strict_validation() {
    let mut decoder = Decoder::new(DecodeOptions::new());
    let mut prefix_output = [0_u8; 4];
    let prefix_written = decoder.update(b"~A", &mut prefix_output).unwrap();
    let before = decoder.clone();

    let chunk = b"BBBBBB";
    let required = output_required(decoder.update(chunk, &mut []).unwrap_err());
    assert!(required > 0);
    assert_eq!(decoder, before);

    let mut too_small = vec![0xa5; required - 1];
    let untouched = too_small.clone();
    let error = decoder.update(chunk, &mut too_small).unwrap_err();
    assert_eq!(output_required(error), required);
    assert_eq!(decoder, before);
    assert_eq!(too_small, untouched);

    let mut retry = vec![0; required];
    let retry_written = decoder.update(chunk, &mut retry).unwrap();
    let mut decoded = prefix_output[..prefix_written].to_vec();
    decoded.extend_from_slice(&retry[..retry_written]);
    if let Some(tail) = decoder.finish().unwrap() {
        decoded.push(tail);
    }
    assert_eq!(decoded, reference::decode(b"~ABBBBBB"));

    let mut strict = Decoder::new(strict_options());
    let error = strict.update(b"  ", &mut []).unwrap_err();
    assert!(matches!(error, DecodeError::OutputTooSmall(_)));
    assert_eq!(strict, Decoder::new(strict_options()));
}

#[test]
fn update_capacity_success_is_content_independent() {
    for prefix in [&b""[..], &b"A"[..], &b"~A"[..]] {
        let mut alphabet_decoder = Decoder::new(DecodeOptions::new());
        let mut prefix_output = vec![0; max_decoded_len(prefix.len()).unwrap()];
        alphabet_decoder.update(prefix, &mut prefix_output).unwrap();
        let mut ignored_decoder = alphabet_decoder.clone();

        let alphabet = b"AAAAA";
        let ignored = [0xff; 5];
        let alphabet_required =
            output_required(alphabet_decoder.update(alphabet, &mut []).unwrap_err());
        let ignored_required =
            output_required(ignored_decoder.update(&ignored, &mut []).unwrap_err());
        assert_eq!(alphabet_required, ignored_required);

        let mut alphabet_output = vec![0; alphabet_required];
        let mut ignored_output = vec![0; ignored_required];
        assert!(alphabet_decoder
            .update(alphabet, &mut alphabet_output)
            .is_ok());
        assert!(ignored_decoder
            .update(&ignored, &mut ignored_output)
            .is_ok());
    }
}

#[test]
fn max_decoded_len_matches_wide_formula_and_covers_reachable_states() {
    let samples = [
        0,
        1,
        2,
        7,
        8,
        255,
        256,
        isize::MAX as usize,
        usize::MAX - 1,
        usize::MAX,
    ];
    for input_len in samples {
        let pairs = input_len as u128 / 2 + input_len as u128 % 2;
        let tail = u128::from(input_len % 2 == 0);
        let wide = (14 * pairs).div_ceil(8) + tail;
        assert_eq!(
            max_decoded_len(input_len),
            usize::try_from(wide).ok(),
            "input_len={input_len}"
        );
    }
    assert!(max_decoded_len(usize::MAX).is_some());
    assert!(max_decoded_len(isize::MAX as usize).unwrap() <= isize::MAX as usize);

    const PREFIXES: &[&[u8]] = &[b"", b"A", b"AA", b"~A", b"AAAA~A", b"~A~A", b"AAAAAA~AB"];
    let alphabet = [b'A'; 64];
    for &prefix in PREFIXES {
        let mut decoder = Decoder::new(DecodeOptions::new());
        let mut prefix_output = vec![0; max_decoded_len(prefix.len()).unwrap()];
        decoder.update(prefix, &mut prefix_output).unwrap();
        for chunk_len in 0..=64 {
            let bound = max_decoded_len(chunk_len).unwrap();
            let mut working = decoder.clone();
            let mut output = vec![0; bound];
            let written = working.update(&alphabet[..chunk_len], &mut output).unwrap();
            let tail = usize::from(working.finish().unwrap().is_some());
            assert!(
                written + tail <= bound,
                "prefix={prefix:?}, len={chunk_len}"
            );
        }
    }
}
