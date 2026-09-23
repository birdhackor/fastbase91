use core::fmt;

use crate::OutputTooSmall;

/// Decoder policy options reserved for the real implementation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecodeOptions {
    /// Whether bytes outside the basE91 alphabet should be rejected.
    pub reject_non_alphabet: bool,
}

impl DecodeOptions {
    /// Creates lenient options, matching the planned default contract.
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

/// Error returned by the future decoding APIs.
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

#[cfg(feature = "std")]
impl std::error::Error for DecodeError {}

/// Stateful decoder placeholder for the first implementation milestone.
#[derive(Clone, Copy, Debug)]
pub struct Decoder {
    options: DecodeOptions,
}

impl Decoder {
    /// Creates a decoder with the requested policy.
    pub const fn new(options: DecodeOptions) -> Self {
        Self { options }
    }

    /// Updates the decoder without consuming any input in the scaffold.
    pub fn update(&mut self, _input: &[u8], _output: &mut [u8]) -> Result<usize, DecodeError> {
        let _ = self.options;
        Ok(0)
    }

    /// Finishes the decoder. The scaffold has no pending byte.
    pub fn finish(self) -> Result<Option<u8>, DecodeError> {
        Ok(None)
    }

    pub(crate) fn decode_into(
        mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, DecodeError> {
        let written = self.update(input, output)?;
        if let Some(byte) = self.finish()? {
            let slot = output
                .get_mut(written)
                .ok_or_else(|| DecodeError::OutputTooSmall(OutputTooSmall::new(written + 1)))?;
            *slot = byte;
            Ok(written + 1)
        } else {
            Ok(written)
        }
    }
}

#[cfg(feature = "alloc")]
pub fn decode(input: &[u8], options: DecodeOptions) -> Result<alloc::vec::Vec<u8>, DecodeError> {
    let mut output = alloc::vec::Vec::new();
    output
        .try_reserve_exact(0)
        .map_err(|_| DecodeError::AllocationFailed)?;
    let written = Decoder::new(options).decode_into(input, &mut output)?;
    output.truncate(written);
    Ok(output)
}
