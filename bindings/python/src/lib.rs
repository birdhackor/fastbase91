#![forbid(unsafe_code)]

use fastbase91_core::{DecodeOptions, Decoder as CoreDecoder, Encoder as CoreEncoder};
use pyo3::exceptions::{PyMemoryError, PyValueError};
use pyo3::prelude::*;

fn memory_error() -> PyErr {
    PyMemoryError::new_err("fastbase91 allocation failed")
}

#[pyfunction]
fn encode(data: &[u8]) -> PyResult<Vec<u8>> {
    fastbase91_core::encode(data).map_err(|_| memory_error())
}

#[pyfunction]
#[pyo3(signature = (data, *, strict = false))]
fn decode(data: &[u8], strict: bool) -> PyResult<Vec<u8>> {
    let mut options = DecodeOptions::new();
    options.reject_non_alphabet = strict;
    fastbase91_core::decode(data, options).map_err(|error| PyValueError::new_err(error.to_string()))
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

    fn update(&mut self, data: &[u8]) -> PyResult<Vec<u8>> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("encoder is closed"))?;
        let mut output = Vec::new();
        let written = inner
            .update(data, &mut output)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        output.truncate(written);
        Ok(output)
    }

    fn finish(&mut self) -> PyResult<Vec<u8>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("encoder is closed"))?;
        let (tail, length) = inner.finish();
        Ok(tail[..length].to_vec())
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

    fn update(&mut self, data: &[u8]) -> PyResult<Vec<u8>> {
        let inner = self
            .inner
            .as_mut()
            .ok_or_else(|| PyValueError::new_err("decoder is closed"))?;
        let mut output = Vec::new();
        let written = inner
            .update(data, &mut output)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        output.truncate(written);
        Ok(output)
    }

    fn finish(&mut self) -> PyResult<Vec<u8>> {
        let inner = self
            .inner
            .take()
            .ok_or_else(|| PyValueError::new_err("decoder is closed"))?;
        match inner.finish() {
            Ok(Some(byte)) => Ok(vec![byte]),
            Ok(None) => Ok(Vec::new()),
            Err(error) => Err(PyValueError::new_err(error.to_string())),
        }
    }
}

#[pymodule(gil_used = false)]
fn _fastbase91(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(encode, module)?)?;
    module.add_function(wrap_pyfunction!(decode, module)?)?;
    module.add_class::<Encoder>()?;
    module.add_class::<Decoder>()?;
    Ok(())
}
