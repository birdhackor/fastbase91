# Usage

## Python

Install the extension from PyPI:

```console
python -m pip install fastbase91
```

### One-shot encoding and decoding

```python
import fastbase91

encoded = fastbase91.encode(b"hello")
decoded = fastbase91.decode(encoded)

assert encoded == b"TPwJh>A"
assert decoded == b"hello"
```

`encode(data)` and `decode(data, *, strict=False)` return `bytes`. Their input may be `bytes`, `bytearray`, or another compatible contiguous, one-dimensional byte buffer such as `memoryview`.

Lenient decoding is the default. Bytes outside the basE91 alphabet are ignored:

```python
assert fastbase91.decode(b"TPw Jh>A") == b"hello"
```

Set `strict=True` when non-alphabet bytes must be rejected. `DecodeError` is a `ValueError` subclass whose `offset` and `byte` attributes identify the offending input:

```python
try:
    fastbase91.decode(b"TPw Jh>A", strict=True)
except fastbase91.DecodeError as error:
    assert error.offset == 3
    assert error.byte == 0x20
```

### Streaming

Use a fresh encoder or decoder for each independent stream, append every result from `update()`, and append the result from `finish()`:

```python
encoder = fastbase91.Encoder()
encoded = encoder.update(b"hel")
encoded += encoder.update(b"lo")
encoded += encoder.finish()
assert encoded == b"TPwJh>A"

decoder = fastbase91.Decoder(strict=False)
decoded = decoder.update(encoded[:3])
decoded += decoder.update(encoded[3:])
decoded += decoder.finish()
assert decoded == b"hello"
```

`Encoder.update(data)` and `Decoder.update(data)` return the output available for that chunk. `finish()` flushes the tail and closes the Python stream object; another `update()` or `finish()` on that object raises `ValueError`. Pass `strict=True` to `Decoder` to reject the first non-alphabet byte.

The package exposes its binding version as `fastbase91.__version__` and the linked Rust core version as `fastbase91.CORE_VERSION`.

## Rust

Add the core crate:

```console
cargo add fastbase91-core
```

### Allocating APIs

The default `std` feature enables `alloc`. With either `std` or `alloc`, one-shot calls return a `Vec<u8>`:

```rust
use fastbase91_core::{decode, encode, DecodeOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let encoded = encode(b"hello")?;
    assert_eq!(encoded, b"TPwJh>A");

    let decoded = decode(&encoded, DecodeOptions::default())?;
    assert_eq!(decoded, b"hello");
    Ok(())
}
```

The signatures are:

```rust
pub fn encode(input: &[u8]) -> Result<Vec<u8>, EncodeError>;
pub fn decode(
    input: &[u8],
    options: DecodeOptions,
) -> Result<Vec<u8>, DecodeError>;
```

`DecodeOptions::new()` and `DecodeOptions::default()` are lenient. To reject non-alphabet bytes, set the public field before decoding:

```rust
let mut options = fastbase91_core::DecodeOptions::new();
options.reject_non_alphabet = true;
```

### `no_std` without allocation

Disable default features when the target has neither `std` nor an allocator:

```toml
[dependencies]
fastbase91-core = { version = "0.1.1", default-features = false }
```

Use caller-provided buffers with `encode_into` and `decode_into`:

```rust
use fastbase91_core::{decode_into, encode_into, DecodeOptions};

let mut encoded = [0_u8; 10];
let encoded_len = encode_into(b"hello", &mut encoded).unwrap();
assert_eq!(&encoded[..encoded_len], b"TPwJh>A");

let mut decoded = [0_u8; 7];
let decoded_len = decode_into(
    &encoded[..encoded_len],
    &mut decoded,
    DecodeOptions::new(),
)
.unwrap();
assert_eq!(&decoded[..decoded_len], b"hello");
```

`encode_into` returns `Result<usize, OutputTooSmall>`. `decode_into` returns `Result<usize, DecodeError>`, whose variants include `OutputTooSmall` and `InvalidByte`. `max_encoded_len(input_len)` and `max_decoded_len(input_len)` return `Option<usize>` upper bounds for sizing output storage.

To use the allocating APIs in a `no_std` environment that does provide an allocator, enable only `alloc`:

```toml
fastbase91-core = { version = "0.1.1", default-features = false, features = ["alloc"] }
```

### Streaming core API

The core streaming types write into caller-provided slices:

```rust
Encoder::new()
Encoder::update(&mut self, input: &[u8], output: &mut [u8])
    -> Result<usize, OutputTooSmall>
Encoder::finish(self) -> ([u8; 2], usize)

Decoder::new(options: DecodeOptions)
Decoder::update(&mut self, input: &[u8], output: &mut [u8])
    -> Result<usize, DecodeError>
Decoder::finish(self) -> Result<Option<u8>, DecodeError>
```

The Rust `finish` methods consume the stream. For the encoder, copy the first `usize` bytes from the returned two-byte tail. For the decoder, append the returned byte when the result is `Some`.

The crate version is available as `fastbase91_core::VERSION`.
