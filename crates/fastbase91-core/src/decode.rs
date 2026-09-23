use core::fmt;

use crate::tables::DECODE_TABLE;
use crate::OutputTooSmall;

/// Decoder policy options.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeOptions {
    /// Whether bytes outside the basE91 alphabet should be rejected.
    pub reject_non_alphabet: bool,
}

impl DecodeOptions {
    /// Creates lenient options that ignore bytes outside the basE91 alphabet.
    pub const fn new() -> Self {
        Self {
            reject_non_alphabet: false,
        }
    }
}

impl Default for DecodeOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Error returned by the decoding APIs.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DecodeError {
    /// The caller-provided output storage cannot hold the result.
    OutputTooSmall(OutputTooSmall),
    /// A strict decode encountered a byte outside the alphabet.
    InvalidByte { byte: u8, offset: usize },
    /// The allocator could not reserve the requested storage.
    #[cfg(feature = "alloc")]
    AllocationFailed,
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputTooSmall(error) => error.fmt(formatter),
            Self::InvalidByte { byte, offset } => {
                write!(formatter, "invalid byte 0x{byte:02x} at offset {offset}")
            }
            #[cfg(feature = "alloc")]
            Self::AllocationFailed => formatter.write_str("decoding allocation failed"),
        }
    }
}

impl core::error::Error for DecodeError {}

/// Returns a decoded-length upper bound for `input_len` bytes.
///
/// The bound holds for every reachable decoder state and every input, and
/// includes the possible one-byte tail returned by [`Decoder::finish`]. In the
/// worst case, an existing pending symbol makes the number of symbols at most
/// `input_len + 1`. The resulting bound is monotone in that symbol count, so it
/// completes `ceil(input_len / 2)` pairs of at most 14 bits and has a tail
/// exactly when `input_len` is even. The quotient and remainder calculation
/// avoids forming `14 * input_len`.
///
/// `None` means that the mathematical bound does not fit in `usize`. A `Some`
/// result can still exceed `isize::MAX`, so allocating APIs perform that
/// separate platform-capacity check.
pub const fn max_decoded_len(input_len: usize) -> Option<usize> {
    let pairs = input_len / 2 + input_len % 2;
    let prefix = match (pairs / 4).checked_mul(7) {
        Some(value) => value,
        None => return None,
    };
    let remaining_bytes = ((pairs % 4) * 14).div_ceil(8);
    let tail = if input_len % 2 == 0 { 1 } else { 0 };
    match prefix.checked_add(remaining_bytes) {
        Some(value) => value.checked_add(tail),
        None => None,
    }
}

/// Stateful standard basE91 decoder.
///
/// A call to [`finish`](Self::finish) ends the current message and consumes the
/// decoder, so the same stream cannot be continued afterwards. The consuming
/// contract is enforced by the type system: the decoder is not `Copy`, so
/// reusing it after `finish` fails to compile.
///
/// ```compile_fail
/// use fastbase91_core::{DecodeOptions, Decoder};
/// let mut decoder = Decoder::new(DecodeOptions::new());
/// let mut output = [0_u8; 4];
/// let _tail = decoder.finish();
/// // `finish` consumed the decoder; decoding more must not compile.
/// let _ = decoder.update(b"AA", &mut output);
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decoder {
    options: DecodeOptions,
    pending: Option<u8>,
    queue: u32,
    nbits: u8,
    consumed: usize,
}

impl Decoder {
    /// Creates a decoder with the requested policy.
    pub const fn new(options: DecodeOptions) -> Self {
        Self {
            options,
            pending: None,
            queue: 0,
            nbits: 0,
            consumed: 0,
        }
    }

    /// Decodes a chunk into caller-provided storage.
    ///
    /// If an error is returned, neither the decoder state nor the output slice
    /// is changed. Capacity is checked first using a content-independent upper
    /// bound. Strict decoding then checks the entire chunk before decoding it.
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<usize, DecodeError> {
        let required = self.update_capacity(input.len());
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall(OutputTooSmall::new(required)));
        }

        if self.options.reject_non_alphabet {
            if let Some((offset, &byte)) = input
                .iter()
                .enumerate()
                .find(|&(_, &byte)| DECODE_TABLE[usize::from(byte)] == 91)
            {
                return Err(DecodeError::InvalidByte {
                    byte,
                    offset: self.consumed.saturating_add(offset),
                });
            }
        }

        let mut working = self.clone();
        let mut written = 0;
        for &byte in input {
            let value = DECODE_TABLE[usize::from(byte)];
            if value == 91 {
                continue;
            }
            if let Some(first) = working.pending.take() {
                let combined = u32::from(first) + u32::from(value) * 91;
                working.queue |= combined << working.nbits;
                working.nbits += if combined & 8191 > 88 { 13 } else { 14 };
                while working.nbits > 7 {
                    output[written] = working.queue as u8;
                    written += 1;
                    working.queue >>= 8;
                    working.nbits -= 8;
                }
            } else {
                working.pending = Some(value);
            }
        }
        working.consumed = working.consumed.saturating_add(input.len());
        *self = working;
        Ok(written)
    }

    /// Returns the final decoded byte, if an unpaired symbol remains.
    pub fn finish(self) -> Result<Option<u8>, DecodeError> {
        Ok(self
            .pending
            .map(|first| (self.queue | (u32::from(first) << self.nbits)) as u8))
    }

    pub(crate) fn decode_into(
        mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let required = one_shot_capacity(input.len());
        if output.len() < required {
            return Err(DecodeError::OutputTooSmall(OutputTooSmall::new(required)));
        }

        let written = self.update(input, output)?;
        if let Some(byte) = self.finish()? {
            output[written] = byte;
            Ok(written + 1)
        } else {
            Ok(written)
        }
    }

    fn update_capacity(&self, input_len: usize) -> usize {
        let pairs = input_len / 2
            + if self.pending.is_some() {
                input_len % 2
            } else {
                0
            };
        let prefix = (pairs / 4).saturating_mul(7);
        let remaining_bits = usize::from(self.nbits) + (pairs % 4) * 14;
        prefix.saturating_add(remaining_bits / 8)
    }
}

const fn one_shot_capacity(input_len: usize) -> usize {
    let pairs = input_len / 2;
    let prefix = (pairs / 4).saturating_mul(7);
    let remaining_bytes = ((pairs % 4) * 14) / 8;
    prefix
        .saturating_add(remaining_bytes)
        .saturating_add(input_len % 2)
}

#[cfg(feature = "alloc")]
pub fn decode(input: &[u8], options: DecodeOptions) -> Result<alloc::vec::Vec<u8>, DecodeError> {
    let capacity = max_decoded_len(input.len()).ok_or(DecodeError::AllocationFailed)?;
    if capacity > isize::MAX as usize {
        return Err(DecodeError::AllocationFailed);
    }

    let mut output = alloc::vec::Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| DecodeError::AllocationFailed)?;
    output.resize(capacity, 0);
    let written = Decoder::new(options).decode_into(input, &mut output)?;
    output.truncate(written);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_abstract_state_classes_are_reachable_and_shifts_fit_u32() {
        let mut reachable = [[false; 8]; 2];
        reachable[0][0] = true;

        for _ in 0..32 {
            let mut next = reachable;
            for nbits in 0..=7 {
                if reachable[0][nbits] {
                    next[1][nbits] = true;
                }
                if reachable[1][nbits] {
                    next[0][(nbits + 13) % 8] = true;
                    next[0][(nbits + 14) % 8] = true;
                }
            }
            reachable = next;
        }

        assert!(reachable.into_iter().flatten().all(|state| state));
        for nbits in 0..=7 {
            let shifted = 8280_u32 << nbits;
            assert!(shifted < (1_u32 << 21));
        }
    }

    #[test]
    fn update_capacity_matches_wide_formula() {
        for pending in [None, Some(0)] {
            for nbits in 0..=7 {
                let decoder = Decoder {
                    options: DecodeOptions::new(),
                    pending,
                    queue: 0,
                    nbits,
                    consumed: 0,
                };
                for input_len in [0, 1, 2, 7, 8, 9, usize::MAX] {
                    let pairs = input_len as u128 / 2
                        + u128::from(pending.is_some()) * (input_len as u128 % 2);
                    let wide = (u128::from(nbits) + 14 * pairs) / 8;
                    let expected = usize::try_from(wide).unwrap_or(usize::MAX);
                    assert_eq!(decoder.update_capacity(input_len), expected);
                }
            }
        }
    }
}
