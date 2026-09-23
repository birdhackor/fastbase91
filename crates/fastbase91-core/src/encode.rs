use core::fmt;

/// Error returned by the future fallible encoding APIs.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EncodeError {
    /// The caller-provided output storage cannot hold the result.
    OutputTooSmall(OutputTooSmall),
    /// The allocator could not reserve the requested storage.
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

#[cfg(feature = "std")]
impl std::error::Error for EncodeError {}

/// Stateful encoder placeholder for the first implementation milestone.
#[derive(Clone, Copy, Debug, Default)]
pub struct Encoder;

impl Encoder {
    /// Creates an empty encoder.
    pub const fn new() -> Self {
        Self
    }

    /// Updates the encoder without consuming any input in the scaffold.
    pub fn update(&mut self, _input: &[u8], _output: &mut [u8]) -> Result<usize, OutputTooSmall> {
        Ok(0)
    }

    /// Returns the final encoded tail. The scaffold has no tail.
    #[must_use]
    pub fn finish(self) -> ([u8; 2], usize) {
        ([0; 2], 0)
    }

    pub(crate) fn encode_into(
        mut self,
        input: &[u8],
        output: &mut [u8],
    ) -> Result<usize, OutputTooSmall> {
        let written = self.update(input, output)?;
        let (tail, tail_len) = self.finish();
        let end = written
            .checked_add(tail_len)
            .ok_or_else(|| OutputTooSmall::new(usize::MAX))?;
        if end > output.len() {
            return Err(OutputTooSmall::new(end));
        }
        output[written..end].copy_from_slice(&tail[..tail_len]);
        Ok(end)
    }
}

#[cfg(feature = "alloc")]
pub fn encode(input: &[u8]) -> Result<alloc::vec::Vec<u8>, EncodeError> {
    let mut output = alloc::vec::Vec::new();
    output
        .try_reserve_exact(0)
        .map_err(|_| EncodeError::AllocationFailed)?;
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

    /// Returns the minimum capacity requested by the operation.
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

#[cfg(feature = "std")]
impl std::error::Error for OutputTooSmall {}
