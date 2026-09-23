#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod decode;
mod encode;
mod tables;

pub use decode::{DecodeError, DecodeOptions, Decoder};
pub use encode::{max_encoded_len, EncodeError, Encoder, OutputTooSmall};

/// Encodes into caller-provided storage.
///
/// The output is produced by the same state machine as streaming encoding.
pub fn encode_into(input: &[u8], output: &mut [u8]) -> Result<usize, OutputTooSmall> {
    Encoder::new().encode_into(input, output)
}

/// Decodes from caller-provided storage.
///
/// This is a compile-time scaffold only; the basE91 state machine is planned
/// for a later milestone and currently emits no bytes.
pub fn decode_into(
    input: &[u8],
    output: &mut [u8],
    options: DecodeOptions,
) -> Result<usize, DecodeError> {
    Decoder::new(options).decode_into(input, output)
}

#[cfg(feature = "alloc")]
pub use encode::encode;

#[cfg(feature = "alloc")]
pub use decode::decode;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_apis_compile() {
        let mut output = [0_u8; 16];
        assert!(encode_into(b"input", &mut output).is_ok());
        assert_eq!(
            decode_into(b"encoded", &mut output, DecodeOptions::new()),
            Ok(0)
        );
    }
}
