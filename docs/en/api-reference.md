# API reference

## Python

The following Python API is the package's type stub (`fastbase91/__init__.pyi`) as written. The website build embeds this file directly from the source tree instead of copying its declarations by hand. `ReadableBuffer` exists only in the stub, for type checkers: it stands for any object that supports the buffer protocol (such as `bytes`, `bytearray` or `memoryview`) and cannot be imported from `fastbase91` at run time. Type checkers accept any such object, but at run time some of them raise `BufferError` (see the table below). See [Usage](usage.md) for examples.

```python
--8<-- "bindings/python/python/fastbase91/__init__.pyi"
```

### Exceptions

| Exception | When it occurs |
| --- | --- |
| `DecodeError` | A decode encounters a byte outside the basE91 alphabet in strict mode only. It is a `ValueError` subclass; its `offset` and `byte` attributes identify the offending byte. For one-shot `decode`, `offset` is the position within the input. For `Decoder(strict=True).update()`, it includes bytes consumed by earlier updates and is the position in the whole stream. |
| `TypeError` | `encode`, `decode`, or an `update()` call receives an object that does not support the buffer protocol, such as `str`. |
| `BufferError` | A buffer is not one-dimensional and contiguous, or its format is not exactly `B` or `b` (unsigned or signed bytes). For example, ctypes arrays raise it; pass `bytes(obj)` instead. |
| `ValueError` | After `finish()` closes an `Encoder` or `Decoder`, calling that object's `update()` or `finish()` raises `ValueError`. |
| `MemoryError` | An input or output buffer cannot be allocated, or the Rust core reports `AllocationFailed`. |
| `RuntimeError` | The Rust core reports an internal error that should not occur, or a borrow conflict occurs when the same `Encoder` or `Decoder` object is used simultaneously from two threads; the latter may raise `RuntimeError`. See [Free-threading](free-threading.md). |

## Rust

The complete Rust API reference is generated from the source and published on [docs.rs](https://docs.rs/fastbase91-core).

### Errors

| Error or variant | APIs that return it | State on error |
| --- | --- | --- |
| `OutputTooSmall` | `encode_into`, `Encoder::update` | The required size is available from `required()`. Capacity is checked first, so state and output are unchanged. |
| `EncodeError::AllocationFailed` | `encode()` with `alloc` | Reported by the one-shot allocation path. |
| `DecodeError::OutputTooSmall` | `decode_into`, `Decoder::update` | Capacity is checked first, so state and output are unchanged. |
| `DecodeError::InvalidByte { byte, offset }` | `decode` (with `alloc`), `decode_into`, `Decoder::update` in strict mode | The decoder state is unchanged; discard output written by the failed call. In `Decoder::update`, `offset` is within that chunk, while in one-shot `decode` and `decode_into` it is within the complete input. |
| `DecodeError::AllocationFailed` | `decode()` with `alloc` | Reported by the one-shot allocation path. |

The one-shot `encode()` and `decode()` functions and both `AllocationFailed` variants exist only with the `alloc` feature, which the default `std` feature enables. `encode()` returns `EncodeError`; because it reserves enough capacity up front (the `max_encoded_len` bound), only `AllocationFailed` is expected in practice. `Encoder::finish` returns `([u8; 2], usize)` and does not fail. `Decoder::finish` returns `Result<Option<u8>, DecodeError>`.
