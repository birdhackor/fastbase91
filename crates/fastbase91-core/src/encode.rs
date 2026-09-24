use core::fmt;

use crate::tables::{ALPHABET, ENCODE_PAIRS};

/// Returns an encoded-length upper bound for `input_len` bytes.
///
/// The bound holds for every reachable encoder state and includes the up-to-two
/// byte tail returned by [`Encoder::finish`]. Mathematically it is
/// `2 * ceil((8 * input_len + 13) / 13)`. The quotient/remainder form below
/// avoids overflowing while forming `8 * input_len`.
///
/// `None` means that the mathematical bound does not fit in `usize`. A `Some`
/// result can still exceed `isize::MAX`, so allocating APIs perform that
/// separate platform-capacity check.
pub const fn max_encoded_len(input_len: usize) -> Option<usize> {
    let quotient = input_len / 13;
    let remainder = input_len % 13;
    let prefix = match quotient.checked_mul(16) {
        Some(value) => value,
        None => return None,
    };
    let remaining_bits = remainder * 8 + 13;
    let remaining_groups = remaining_bits.div_ceil(13);
    prefix.checked_add(remaining_groups * 2)
}

/// Error returned by the fallible encoding APIs.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The caller-provided output storage cannot hold the result.
    OutputTooSmall(OutputTooSmall),
    /// The requested output is not allocatable or allocation failed.
    #[cfg(feature = "alloc")]
    AllocationFailed,
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutputTooSmall(error) => error.fmt(formatter),
            #[cfg(feature = "alloc")]
            Self::AllocationFailed => formatter.write_str("encoding allocation failed"),
        }
    }
}

impl core::error::Error for EncodeError {}

/// Stateful standard basE91 encoder.
///
/// A call to [`finish`](Self::finish) ends the current message and consumes the
/// encoder, so the same stream cannot be continued afterwards. Separately
/// finished messages must not be concatenated and treated as one basE91 stream.
///
/// The consuming contract is enforced by the type system: the encoder is not
/// `Copy`, so reusing it after `finish` fails to compile.
///
/// ```compile_fail
/// use fastbase91_core::Encoder;
/// let mut encoder = Encoder::new();
/// let mut output = [0_u8; 4];
/// let _tail = encoder.finish();
/// // `finish` consumed the encoder; encoding more must not compile.
/// let _ = encoder.update(b"x", &mut output);
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Encoder {
    queue: u32,
    nbits: u8,
}

impl Encoder {
    /// Creates an empty encoder.
    pub const fn new() -> Self {
        Self { queue: 0, nbits: 0 }
    }

    /// Encodes a chunk into caller-provided storage.
    ///
    /// If the output is too small, neither the encoder state nor the output
    /// slice is changed. Capacity is checked using a content-independent upper
    /// bound derived from the current bit count and the input length.
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<usize, OutputTooSmall> {
        let required = self.update_capacity(input.len());
        if output.len() < required {
            return Err(OutputTooSmall::new(required));
        }

        // Invariant whenever bytes are ingested: `nbits <= 13` and `queue`
        // holds exactly those bits (zeros above).
        let mut queue = u64::from(self.queue);
        let mut nbits = u32::from(self.nbits);
        let mut written = 0;

        macro_rules! emit_pair {
            ($out:expr) => {{
                let low13 = (queue & 8191) as usize;
                if low13 > 88 {
                    queue >>= 13;
                    nbits -= 13;
                    $out.copy_from_slice(&ENCODE_PAIRS[low13]);
                } else {
                    let value = (queue & 16383) as usize;
                    queue >>= 14;
                    nbits -= 14;
                    $out.copy_from_slice(&ENCODE_PAIRS[value]);
                }
            }};
        }

        // Word-at-a-time path: OR in as many whole input bytes as fit in the
        // 64-bit queue (6..=8), which leaves 57..=64 bits, always enough for
        // exactly four pairs (at most 56 bits). Each pair value depends only
        // on the lowest 13/14 queued bits, so ingesting later bytes early
        // cannot change it; the emitted symbols and the leftover state are
        // the same as the byte loop's.
        let mut pos = 0;
        while pos + 8 <= input.len() {
            let word =
                u64::from_le_bytes(input[pos..pos + 8].try_into().expect("slice of length 8"));
            let whole_bytes = (64 - nbits) >> 3;
            let whole_bits = whole_bytes << 3;
            queue |= (word & (u64::MAX >> (64 - whole_bits))) << nbits;
            nbits += whole_bits;
            pos += whole_bytes as usize;
            let out = &mut output[written..written + 8];
            emit_pair!(&mut out[0..2]);
            emit_pair!(&mut out[2..4]);
            emit_pair!(&mut out[4..6]);
            emit_pair!(&mut out[6..8]);
            written += 8;
        }

        for &byte in &input[pos..] {
            queue |= u64::from(byte) << nbits;
            nbits += 8;
            if nbits <= 13 {
                continue;
            }
            emit_pair!(&mut output[written..written + 2]);
            written += 2;
        }
        // `nbits <= 13` here, so the queue fits the stored width.
        self.queue = queue as u32;
        self.nbits = nbits as u8;
        Ok(written)
    }

    /// Returns the final encoded tail and its length.
    #[must_use]
    pub fn finish(self) -> ([u8; 2], usize) {
        let mut tail = [0; 2];
        if self.nbits == 0 {
            return (tail, 0);
        }

        tail[0] = ALPHABET[self.queue as usize % 91];
        if self.nbits > 7 || self.queue > 90 {
            tail[1] = ALPHABET[self.queue as usize / 91];
            (tail, 2)
        } else {
            (tail, 1)
        }
    }

    fn update_capacity(&self, input_len: usize) -> usize {
        // This is 2 * floor((nbits + 8 * input_len) / 13), split at
        // 13-byte boundaries so that 8 * input_len is never formed.
        let quotient = input_len / 13;
        let remainder = input_len % 13;
        let prefix = quotient.saturating_mul(16);
        let remaining_bits = usize::from(self.nbits) + remainder * 8;
        prefix.saturating_add((remaining_bits / 13) * 2)
    }

    pub(crate) fn encode_into(
        mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, OutputTooSmall> {
        let tail_capacity = if self.nbits == 0 && input.is_empty() {
            0
        } else {
            2
        };
        let required = self
            .update_capacity(input.len())
            .saturating_add(tail_capacity);
        if output.len() < required {
            return Err(OutputTooSmall::new(required));
        }

        let written = self.update(input, output)?;
        let (tail, tail_len) = self.finish();
        let end = written + tail_len;
        output[written..end].copy_from_slice(&tail[..tail_len]);
        Ok(end)
    }
}

#[cfg(feature = "alloc")]
pub fn encode(input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
    let capacity = max_encoded_len(input.len()).ok_or(EncodeError::AllocationFailed)?;
    if capacity > isize::MAX as usize {
        return Err(EncodeError::AllocationFailed);
    }

    let mut output = alloc::vec::Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| EncodeError::AllocationFailed)?;
    output.resize(capacity, 0);
    let written = Encoder::new()
        .encode_into(input, &mut output)
        .map_err(EncodeError::OutputTooSmall)?;
    output.truncate(written);
    Ok(output)
}

/// A caller-visible output capacity error.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputTooSmall {
    required: usize,
}

impl OutputTooSmall {
    pub(crate) const fn new(required: usize) -> Self {
        Self { required }
    }

    /// Returns the capacity required by the attempted operation.
    pub const fn required(self) -> usize {
        self.required
    }
}

impl fmt::Display for OutputTooSmall {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "output buffer is too small; need {} bytes",
            self.required
        )
    }
}

impl core::error::Error for OutputTooSmall {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_capacity_matches_wide_formula_and_saturates() {
        for nbits in [0, 7, 13] {
            let encoder = Encoder { queue: 0, nbits };
            for input_len in [0, 1, 12, 13, 14, usize::MAX] {
                let wide = 2 * ((u128::from(nbits) + 8 * input_len as u128) / 13);
                let expected = usize::try_from(wide).unwrap_or(usize::MAX);
                assert_eq!(encoder.update_capacity(input_len), expected);
            }
        }
    }

    #[test]
    fn allocation_limit_is_distinct_from_math_overflow() {
        let bound = max_encoded_len(isize::MAX as usize).expect("bound fits usize");
        assert!(bound > isize::MAX as usize);
        assert_eq!(max_encoded_len(usize::MAX), None);
    }
}
