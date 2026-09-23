use core::fmt;

use crate::tables::ALPHABET;

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
/// A call to [`finish`](Self::finish) ends the current message. Separately
/// finished messages must not be concatenated and treated as one basE91 stream.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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
    /// slice is changed. An empty chunk always succeeds and writes no bytes.
    pub fn update(&mut self, input: &[u8], output: &mut [u8]) -> Result<usize, OutputTooSmall> {
        let (next, required) = self.preflight(input);
        if output.len() < required {
            return Err(OutputTooSmall::new(required));
        }

        let mut working = *self;
        let mut written = 0;
        for &byte in input {
            if let Some(value) = working.push(byte) {
                output[written] = ALPHABET[value % 91];
                output[written + 1] = ALPHABET[value / 91];
                written += 2;
            }
        }
        *self = next;
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

    fn push(&mut self, byte: u8) -> Option<usize> {
        self.queue |= u32::from(byte) << self.nbits;
        self.nbits += 8;
        if self.nbits <= 13 {
            return None;
        }

        let mut value = self.queue & 8191;
        let consumed = if value > 88 {
            13
        } else {
            value = self.queue & 16383;
            14
        };
        self.queue >>= consumed;
        self.nbits -= consumed;
        Some(value as usize)
    }

    fn preflight(&self, input: &[u8]) -> (Self, usize) {
        let mut next = *self;
        let mut required = 0_usize;
        for &byte in input {
            if next.push(byte).is_some() {
                required = required.saturating_add(2);
            }
        }
        (next, required)
    }

    pub(crate) fn encode_into(
        mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, OutputTooSmall> {
        let (after_update, update_len) = self.preflight(input);
        let (_, tail_len) = after_update.finish();
        let required = update_len.saturating_add(tail_len);
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

    /// Returns the exact capacity required by the attempted operation.
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
    fn allocation_limit_is_distinct_from_math_overflow() {
        let bound = max_encoded_len(isize::MAX as usize).expect("bound fits usize");
        assert!(bound > isize::MAX as usize);
        assert_eq!(max_encoded_len(usize::MAX), None);
    }
}
