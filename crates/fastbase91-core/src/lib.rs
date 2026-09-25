#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]

//! A safe encoder and decoder for the standard basE91 binary-to-text encoding.
//!
//! The crate provides allocating one-shot `encode` and `decode` APIs when the
//! `alloc` feature is enabled, caller-provided-buffer APIs through
//! [`encode_into`] and [`decode_into`], and streaming APIs through [`Encoder`]
//! and [`Decoder`]. The default `std` feature includes `alloc`; with both
//! features disabled, the crate supports `no_std` use without an allocator.
//! Lenient or strict decoding is selected with [`DecodeOptions`].

#[cfg(feature = "alloc")]
extern crate alloc;

mod decode;
mod encode;
mod tables;

/// The version of this core crate.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub use decode::{max_decoded_len, DecodeError, DecodeOptions, Decoder};
pub use encode::{max_encoded_len, EncodeError, Encoder, OutputTooSmall};

/// Encodes into caller-provided storage.
///
/// The output is produced by the same state machine as streaming encoding.
pub fn encode_into(input: &[u8], output: &mut [u8]) -> Result<usize, OutputTooSmall> {
    Encoder::new().encode_into(input, output)
}

/// Decodes from caller-provided storage.
///
/// Decodes according to `options`, writes decoded bytes to `output`, and
/// includes the final byte produced from any unpaired input symbol.
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
            Ok(5)
        );
        assert_eq!(&output[..5], b"\xfb\x8d\xca\x1d\xab");
    }

    #[test]
    fn version_matches_package_metadata() {
        assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
        assert!(!VERSION.is_empty());
    }
}
