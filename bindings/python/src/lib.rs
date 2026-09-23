#![forbid(unsafe_code)]

use fastbase91_core::{
    max_decoded_len, max_encoded_len, DecodeError as CoreDecodeError, DecodeOptions,
    Decoder as CoreDecoder, EncodeError as CoreEncodeError, Encoder as CoreEncoder,
};
use pyo3::buffer::PyBuffer;
use pyo3::exceptions::{PyBufferError, PyMemoryError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

pyo3::create_exception!(
    fastbase91._fastbase91,
    DecodeError,
    PyValueError,
    "A byte outside the basE91 alphabet was encountered while decoding."
);

fn memory_error() -> PyErr {
    PyMemoryError::new_err("fastbase91 allocation failed")
}

fn internal_error(message: String) -> PyErr {
    PyRuntimeError::new_err(format!("fastbase91 internal error: {message}"))
}

fn allocate_output(capacity: Option<usize>) -> PyResult<Vec<u8>> {
    let capacity = capacity
        .filter(|&capacity| capacity <= isize::MAX as usize)
        .ok_or_else(memory_error)?;
    let mut output = Vec::new();
    output
        .try_reserve_exact(capacity)
        .map_err(|_| memory_error())?;
    output.resize(capacity, 0);
    Ok(output)
}

fn copy_input(py: Python<'_>, data: &Bound<'_, PyAny>) -> PyResult<Vec<u8>> {
    let buffer = PyBuffer::<u8>::get(data)?;
    if buffer.dimensions() != 1 || !buffer.is_c_contiguous() {
        return Err(PyBufferError::new_err(
            "v1 only accepts contiguous one-dimensional bytes-like objects",
        ));
    }

    let length = buffer.item_count();
    let mut input = Vec::new();
    input
        .try_reserve_exact(length)
        .map_err(|_| memory_error())?;
    input.resize(length, 0);
    buffer.copy_to_slice(py, &mut input)?;
    Ok(input)
}

fn to_py_bytes<'py>(py: Python<'py>, output: &[u8]) -> PyResult<Bound<'py, PyBytes>> {
    PyBytes::new_with(py, output.len(), |bytes| {
        bytes.copy_from_slice(output);
        Ok(())
    })
}

fn encode_error(error: CoreEncodeError) -> PyErr {
    let message = error.to_string();
    match error {
        CoreEncodeError::AllocationFailed => memory_error(),
        CoreEncodeError::OutputTooSmall(_) => internal_error(message),
        _ => internal_error(message),
    }
}

fn decode_error(py: Python<'_>, error: CoreDecodeError) -> PyErr {
    let message = error.to_string();
    match error {
        CoreDecodeError::InvalidByte { byte, offset } => {
            let error = DecodeError::new_err(message);
            if let Err(attribute_error) = error.value(py).setattr("offset", offset) {
                return attribute_error;
            }
            if let Err(attribute_error) = error.value(py).setattr("byte", byte) {
                return attribute_error;
            }
            error
        }
        CoreDecodeError::AllocationFailed => memory_error(),
        CoreDecodeError::OutputTooSmall(_) => internal_error(message),
        _ => internal_error(message),
    }
}

/// Encode a bytes-like object and return `bytes`.
///
/// The caller must not mutate a writable input buffer concurrently during this call.
#[pyfunction]
fn encode<'py>(py: Python<'py>, data: &Bound<'_, PyAny>) -> PyResult<Bound<'py, PyBytes>> {
    let input = copy_input(py, data)?;
    let output = py
        .detach(move || fastbase91_core::encode(&input))
        .map_err(encode_error)?;
    to_py_bytes(py, &output)
}

/// Decode a bytes-like object and return `bytes`.
///
/// The caller must not mutate a writable input buffer concurrently during this call.
#[pyfunction]
#[pyo3(signature = (data, *, strict = false))]
fn decode<'py>(
    py: Python<'py>,
    data: &Bound<'_, PyAny>,
    strict: bool,
) -> PyResult<Bound<'py, PyBytes>> {
    let input = copy_input(py, data)?;
    let mut options = DecodeOptions::new();
    options.reject_non_alphabet = strict;
    let output = py
        .detach(move || fastbase91_core::decode(&input, options))
        .map_err(|error| decode_error(py, error))?;
    to_py_bytes(py, &output)
}

#[pyclass(module = "fastbase91._fastbase91")]
struct Encoder {
    inner: Option<CoreEncoder>,
}

#[pymethods]
impl Encoder {
    #[new]
    fn new() -> Self {
        Self {
            inner: Some(CoreEncoder::new()),
        }
    }

    /// Encode the next bytes-like input chunk and return the available output.
    ///
    /// The caller must not mutate a writable input buffer concurrently during this call.
    fn update<'py>(
        &mut self,
        py: Python<'py>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mut working = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("encoder is closed"))?
            .clone();
        let input = copy_input(py, data)?;
        let mut output = allocate_output(max_encoded_len(input.len()))?;
        let written = working
            .update(&input, &mut output)
            .map_err(|error| internal_error(error.to_string()))?;
        output.truncate(written);
        let bytes = to_py_bytes(py, &output)?;
        self.inner = Some(working);
        Ok(bytes)
    }

    fn finish<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("encoder is closed"))?;
        let (tail, length) = inner.finish();
        to_py_bytes(py, &tail[..length])
    }
}

#[pyclass(module = "fastbase91._fastbase91")]
struct Decoder {
    inner: Option<CoreDecoder>,
}

#[pymethods]
impl Decoder {
    #[new]
    #[pyo3(signature = (*, strict = false))]
    fn new(strict: bool) -> Self {
        let mut options = DecodeOptions::new();
        options.reject_non_alphabet = strict;
        Self {
            inner: Some(CoreDecoder::new(options)),
        }
    }

    /// Decode the next bytes-like input chunk and return the available output.
    ///
    /// The caller must not mutate a writable input buffer concurrently during this call.
    fn update<'py>(
        &mut self,
        py: Python<'py>,
        data: &Bound<'_, PyAny>,
    ) -> PyResult<Bound<'py, PyBytes>> {
        let mut working = self
            .inner
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("decoder is closed"))?
            .clone();
        let input = copy_input(py, data)?;
        let mut output = allocate_output(max_decoded_len(input.len()))?;
        let written = working
            .update(&input, &mut output)
            .map_err(|error| decode_error(py, error))?;
        output.truncate(written);
        let bytes = to_py_bytes(py, &output)?;
        self.inner = Some(working);
        Ok(bytes)
    }

    fn finish<'py>(&mut self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("decoder is closed"))?;
        match inner.finish() {
            Ok(Some(byte)) => to_py_bytes(py, &[byte]),
            Ok(None) => to_py_bytes(py, &[]),
            Err(error) => Err(decode_error(py, error)),
        }
    }
}

#[pymodule(gil_used = false)]
fn _fastbase91(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("DecodeError", module.py().get_type::<DecodeError>())?;
    module.add_function(wrap_pyfunction!(encode, module)?)?;
    module.add_function(wrap_pyfunction!(decode, module)?)?;
    module.add_class::<Encoder>()?;
    module.add_class::<Decoder>()?;
    Ok(())
}
